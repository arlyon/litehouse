use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{
    Deserialize,
    de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    net::Ipv4Addr,
    str::FromStr,
};
use zeroize::{Zeroize, Zeroizing};

pub(crate) const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_JSON_DEPTH: usize = 8;
const MAX_POWER_CHANNELS: usize = 32;
const MAX_ARRAY_ELEMENTS: usize = 8;
const MAX_MEASUREMENTS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProtocolError {
    InvalidConfiguration,
    AuthenticationFailed,
    RefererBlocked,
    UpstreamHttpStatus(u16),
    Protocol,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct Secret(String);
impl Secret {
    fn expose(&self) -> &str {
        &self.0
    }
}
impl Drop for Secret {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    address: String,
    #[serde(default = "default_poll_seconds")]
    poll_seconds: u16,
    #[serde(rename = "WEB_PASSWORD")]
    web_password: Secret,
}
fn default_poll_seconds() -> u16 {
    30
}

pub(crate) struct Config {
    pub(crate) address: Ipv4Addr,
    pub(crate) poll_seconds: u16,
    password: Secret,
}
impl Config {
    pub(crate) fn from_json(input: &str) -> Result<Self, ProtocolError> {
        if input.len() > 4096 {
            return Err(ProtocolError::InvalidConfiguration);
        }
        let raw: RawConfig =
            serde_json::from_str(input).map_err(|_| ProtocolError::InvalidConfiguration)?;
        let address =
            Ipv4Addr::from_str(&raw.address).map_err(|_| ProtocolError::InvalidConfiguration)?;
        if !address.is_private()
            || address.is_unspecified()
            || address.is_loopback()
            || address.is_link_local()
            || address.is_multicast()
            || address.is_broadcast()
            || raw.address != address.to_string()
            || !(10..=300).contains(&raw.poll_seconds)
            || raw.web_password.expose().is_empty()
            || raw.web_password.expose().len() > 32
        {
            return Err(ProtocolError::InvalidConfiguration);
        }
        Ok(Self {
            address,
            poll_seconds: raw.poll_seconds,
            password: raw.web_password,
        })
    }
    pub(crate) fn request(&self, command: FixedCommand) -> RequestSpec {
        let mut credentials = Zeroizing::new(format!("admin:{}", self.password.expose()));
        let authorization =
            Zeroizing::new(format!("Basic {}", STANDARD.encode(credentials.as_bytes())));
        credentials.zeroize();
        RequestSpec {
            authority: format!("{}:80", self.address),
            path_with_query: command.path(),
            authorization,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FixedCommand {
    Status2,
    Status,
    Template,
    Status10,
    SetPower { channel: u8, value: bool },
}
impl FixedCommand {
    fn path(self) -> String {
        match self {
            Self::Status2 => "/cm?cmnd=Status%202".into(),
            Self::Status => "/cm?cmnd=Status".into(),
            Self::Template => "/cm?cmnd=Template".into(),
            Self::Status10 => "/cm?cmnd=Status%2010".into(),
            Self::SetPower { channel, value } => {
                debug_assert!((1..=32).contains(&channel));
                format!("/cm?cmnd=Power{channel}%20{}", u8::from(value))
            }
        }
    }
}
pub(crate) struct RequestSpec {
    pub(crate) authority: String,
    pub(crate) path_with_query: String,
    authorization: Zeroizing<String>,
}
impl RequestSpec {
    pub(crate) fn authorization(&self) -> &[u8] {
        self.authorization.as_bytes()
    }
}

pub(crate) fn parse_response(status: u16, body: &[u8]) -> Result<Value, ProtocolError> {
    if status == 401 {
        return Err(ProtocolError::AuthenticationFailed);
    }
    if status != 200 {
        return Err(ProtocolError::UpstreamHttpStatus(status));
    }
    let first = body.iter().copied().find(|b| !b.is_ascii_whitespace());
    if first.is_none() || first == Some(b'<') {
        return Err(ProtocolError::RefererBlocked);
    }
    let value = parse_bounded_json(body)?;
    if value
        .get("Command")
        .and_then(Value::as_str)
        .is_some_and(|v| v.eq_ignore_ascii_case("error") || v.eq_ignore_ascii_case("unknown"))
    {
        return Err(ProtocolError::Protocol);
    }
    Ok(value)
}

pub(crate) fn parse_bounded_json(body: &[u8]) -> Result<Value, ProtocolError> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(ProtocolError::Protocol);
    }
    let mut de = serde_json::Deserializer::from_slice(body);
    let value = ValueSeed(1)
        .deserialize(&mut de)
        .map_err(|_| ProtocolError::Protocol)?;
    de.end().map_err(|_| ProtocolError::Protocol)?;
    Ok(value)
}
struct ValueSeed(usize);
impl<'de> DeserializeSeed<'de> for ValueSeed {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(self, de: D) -> Result<Value, D::Error> {
        if self.0 > MAX_JSON_DEPTH {
            return Err(D::Error::custom("JSON nesting limit exceeded"));
        }
        de.deserialize_any(ValueVisitor(self.0))
    }
}
struct ValueVisitor(usize);
impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("bounded duplicate-free JSON")
    }
    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::Number(v.into()))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::Number(v.into()))
    }
    fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Value, E> {
        Number::from_f64(v)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite"))
    }
    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::String(v.into()))
    }
    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(Value::String(v))
    }
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_some<D: serde::Deserializer<'de>>(self, de: D) -> Result<Value, D::Error> {
        ValueSeed(self.0).deserialize(de)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut out = Vec::new();
        while let Some(v) = seq.next_element_seed(ValueSeed(self.0 + 1))? {
            out.push(v);
        }
        Ok(Value::Array(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut out = Map::new();
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(A::Error::custom("duplicate key"));
            }
            out.insert(key, map.next_value_seed(ValueSeed(self.0 + 1))?);
        }
        Ok(Value::Object(out))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StatusSnapshot {
    pub(crate) module: Value,
    pub(crate) power: Vec<bool>,
    pub(crate) labels: Vec<Option<String>>,
}
pub(crate) fn parse_status2(v: &Value) -> Result<[u8; 32], ProtocolError> {
    let x = v
        .get("StatusFWR")
        .and_then(Value::as_object)
        .ok_or(ProtocolError::Protocol)?;
    digest(&Value::Object(x.clone()))
}
pub(crate) fn parse_template(v: &Value) -> Result<[u8; 32], ProtocolError> {
    if !v.is_object() {
        Err(ProtocolError::Protocol)
    } else {
        digest(v)
    }
}
fn digest(v: &Value) -> Result<[u8; 32], ProtocolError> {
    Ok(Sha256::digest(serde_json::to_vec(v).map_err(|_| ProtocolError::Protocol)?).into())
}
pub(crate) fn routing_digest(
    module: &Value,
    template: &[u8; 32],
    channels: usize,
) -> Result<[u8; 32], ProtocolError> {
    let mut d = Sha256::new();
    d.update(serde_json::to_vec(module).map_err(|_| ProtocolError::Protocol)?);
    d.update([0]);
    d.update(template);
    d.update((channels as u64).to_be_bytes());
    Ok(d.finalize().into())
}
pub(crate) fn parse_status(v: &Value) -> Result<StatusSnapshot, ProtocolError> {
    let s = v
        .get("Status")
        .and_then(Value::as_object)
        .ok_or(ProtocolError::Protocol)?;
    let module = s
        .get("Module")
        .cloned()
        .filter(|v| v.is_string() || v.is_number())
        .ok_or(ProtocolError::Protocol)?;
    let bits = s
        .get("Power")
        .and_then(Value::as_str)
        .ok_or(ProtocolError::Protocol)?;
    if bits.is_empty()
        || bits.len() > MAX_POWER_CHANNELS
        || !bits.bytes().all(|b| matches!(b, b'0' | b'1'))
    {
        return Err(ProtocolError::Protocol);
    }
    let power: Vec<_> = bits.bytes().rev().map(|b| b == b'1').collect();
    let names = s.get("FriendlyName").and_then(Value::as_array);
    let labels = (0..power.len())
        .map(|i| {
            names
                .and_then(|n| n.get(i))
                .and_then(Value::as_str)
                .filter(|x| !x.is_empty() && x.len() <= 256 && x.chars().count() <= 80)
                .map(str::to_owned)
        })
        .collect();
    Ok(StatusSnapshot {
        module,
        power,
        labels,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MeasurementKind {
    Instantaneous,
    Total,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MeasurementSample {
    pub(crate) key: String,
    pub(crate) label: String,
    pub(crate) profile: &'static str,
    pub(crate) kind: MeasurementKind,
    pub(crate) magnitude: String,
    pub(crate) unit: &'static str,
}
pub(crate) fn parse_measurements(v: &Value) -> Result<Vec<MeasurementSample>, ProtocolError> {
    let sensors = v
        .get("StatusSNS")
        .and_then(Value::as_object)
        .ok_or(ProtocolError::Protocol)?;
    let temp_unit = match sensors.get("TempUnit").and_then(Value::as_str) {
        Some("C") => Some("Cel"),
        Some("F") => Some("[degF]"),
        _ => None,
    };
    let mut out = BTreeMap::new();
    for (sensor, value) in sensors {
        let Some(fields) = value.as_object() else {
            continue;
        };
        if sensor == "ENERGY" {
            for (field, profile, kind, unit) in [
                ("Power", "power", MeasurementKind::Instantaneous, "W"),
                ("Voltage", "voltage", MeasurementKind::Instantaneous, "V"),
                (
                    "Current",
                    "electric-current",
                    MeasurementKind::Instantaneous,
                    "A",
                ),
                (
                    "Frequency",
                    "frequency",
                    MeasurementKind::Instantaneous,
                    "Hz",
                ),
                ("Total", "energy", MeasurementKind::Total, "kW.h"),
            ] {
                if let Some(source) = fields.get(field) {
                    insert_samples(
                        &mut out,
                        &format!("ENERGY.{field}"),
                        source,
                        profile,
                        kind,
                        unit,
                    );
                }
            }
        } else {
            if let Some(unit) = temp_unit
                && let Some(source) = fields.get("Temperature")
            {
                insert_samples(
                    &mut out,
                    &format!("{sensor}.Temperature"),
                    source,
                    "temperature",
                    MeasurementKind::Instantaneous,
                    unit,
                );
            }
            if let Some(source) = fields.get("Humidity") {
                insert_samples(
                    &mut out,
                    &format!("{sensor}.Humidity"),
                    source,
                    "relative-humidity",
                    MeasurementKind::Instantaneous,
                    "%",
                );
            }
        }
    }
    Ok(out.into_values().take(MAX_MEASUREMENTS).collect())
}
fn insert_samples(
    out: &mut BTreeMap<String, MeasurementSample>,
    path: &str,
    v: &Value,
    profile: &'static str,
    kind: MeasurementKind,
    unit: &'static str,
) {
    match v {
        Value::Number(n) => insert_sample(out, path, None, n, profile, kind, unit),
        Value::Array(a) if a.len() <= MAX_ARRAY_ELEMENTS => {
            for (i, v) in a.iter().enumerate() {
                if let Value::Number(n) = v {
                    insert_sample(out, path, Some(i + 1), n, profile, kind, unit);
                }
            }
        }
        _ => {}
    }
}
fn insert_sample(
    out: &mut BTreeMap<String, MeasurementSample>,
    path: &str,
    index: Option<usize>,
    n: &Number,
    profile: &'static str,
    kind: MeasurementKind,
    unit: &'static str,
) {
    let magnitude = n.to_string();
    if magnitude.contains(['e', 'E']) || path.len() > 256 {
        return;
    }
    let encoded = percent_encode(path);
    let key = index.map_or_else(
        || format!("measurement/{encoded}"),
        |i| format!("measurement/{encoded}/{i}"),
    );
    let label = index.map_or_else(|| path.into(), |i| format!("{path} {i}"));
    out.insert(
        key.clone(),
        MeasurementSample {
            key,
            label,
            profile,
            kind,
            magnitude,
            unit,
        },
    );
}
fn percent_encode(v: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::new();
    for b in v.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(b));
        } else {
            out.push('%');
            out.push(char::from(HEX[(b >> 4) as usize]));
            out.push(char::from(HEX[(b & 15) as usize]));
        }
    }
    out
}
pub(crate) fn validate_power_ack(
    v: &Value,
    channel: u8,
    count: usize,
) -> Result<(), ProtocolError> {
    let o = v.as_object().ok_or(ProtocolError::Protocol)?;
    let powers: Vec<_> = o
        .iter()
        .filter(|(k, _)| {
            k.as_str() == "POWER"
                || k.strip_prefix("POWER")
                    .is_some_and(|x| !x.is_empty() && x.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    let allowed = if count == 1 {
        powers
            .first()
            .is_some_and(|(k, v)| matches!(k.as_str(), "POWER" | "POWER1") && v.is_string())
    } else {
        powers
            .first()
            .is_some_and(|(k, v)| *k == &format!("POWER{channel}") && v.is_string())
    };
    if powers.len() == 1 && allowed {
        Ok(())
    } else {
        Err(ProtocolError::Protocol)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config(address: &str, poll: u16) -> String {
        format!(r#"{{"address":"{address}","poll_seconds":{poll},"WEB_PASSWORD":"x"}}"#)
    }
    #[test]
    fn config_is_closed_private_ipv4_only() {
        for a in ["10.1.2.3", "172.16.0.1", "192.168.4.1"] {
            assert!(Config::from_json(&config(a, 30)).is_ok())
        }
        for a in [
            "8.8.8.8",
            "127.0.0.1",
            "169.254.1.1",
            "node.local",
            "10.0.0.1:80",
        ] {
            assert!(Config::from_json(&config(a, 30)).is_err())
        }
        assert!(
            Config::from_json(r#"{"address":"10.0.0.1","WEB_PASSWORD":"x","extra":1}"#).is_err()
        );
    }
    #[test]
    fn fixed_requests_are_exact() {
        let c = Config::from_json(&config("192.168.1.7", 30)).unwrap();
        let paths = [
            FixedCommand::Status2,
            FixedCommand::Status,
            FixedCommand::Template,
            FixedCommand::Status10,
            FixedCommand::SetPower {
                channel: 1,
                value: false,
            },
            FixedCommand::SetPower {
                channel: 32,
                value: true,
            },
        ]
        .map(|x| c.request(x).path_with_query);
        assert_eq!(
            paths,
            [
                "/cm?cmnd=Status%202",
                "/cm?cmnd=Status",
                "/cm?cmnd=Template",
                "/cm?cmnd=Status%2010",
                "/cm?cmnd=Power1%200",
                "/cm?cmnd=Power32%201"
            ]
        );
        let r = c.request(FixedCommand::Status);
        assert_eq!(r.authority, "192.168.1.7:80");
        assert!(r.authorization().starts_with(b"Basic "));
        assert!(!r.path_with_query.contains("password"));
    }
    #[test]
    fn json_is_bounded_duplicate_safe() {
        assert!(parse_bounded_json(br#"{"a":1,"a":2}"#).is_err());
        assert!(parse_bounded_json(br#"{"a":NaN}"#).is_err());
        assert!(parse_bounded_json(br#"[[[[[[[[[0]]]]]]]]]"#).is_err());
        let exact = format!(r#"{{"x":"{}"}}"#, "a".repeat(MAX_RESPONSE_BYTES - 8));
        assert_eq!(exact.len(), MAX_RESPONSE_BYTES);
        assert!(parse_bounded_json(exact.as_bytes()).is_ok());
        assert!(parse_bounded_json(format!("{exact} ").as_bytes()).is_err());
    }
    #[test]
    fn status_bits_are_lsb_first() {
        for bits in ["1", "10", "1010", "10000000000000000000000000000001"] {
            let s = parse_status(&serde_json::json!({"Status":{"Module":1,"Power":bits}})).unwrap();
            assert_eq!(s.power.len(), bits.len());
            assert_eq!(s.power[0], bits.ends_with('1'));
        }
    }
    #[test]
    fn whitelist_handles_repeated_sensors_and_arrays() {
        let v = serde_json::json!({"StatusSNS":{"TempUnit":"C","A":{"Temperature":21.5,"Humidity":40,"Pressure":1000},"B":{"Temperature":[22,23]},"ENERGY":{"Power":[10,11],"Voltage":230,"Current":0.2,"Frequency":50,"Total":1.5,"Today":0.1}}});
        let s = parse_measurements(&v).unwrap();
        let keys: BTreeSet<_> = s.iter().map(|x| x.key.as_str()).collect();
        for k in [
            "measurement/A.Temperature",
            "measurement/B.Temperature/1",
            "measurement/ENERGY.Power/2",
            "measurement/ENERGY.Total",
        ] {
            assert!(keys.contains(k))
        }
        assert!(
            !keys
                .iter()
                .any(|k| k.contains("Today") || k.contains("Pressure"))
        );
    }
    #[test]
    fn units_and_unsupported_shapes_are_closed() {
        let f = parse_measurements(
            &serde_json::json!({"StatusSNS":{"TempUnit":"F","A":{"Temperature":70}}}),
        )
        .unwrap();
        assert_eq!(f[0].unit, "[degF]");
        let none=parse_measurements(&serde_json::json!({"StatusSNS":{"TempUnit":"K","A":{"Temperature":1},"ENERGY":{"Power":[1,2,3,4,5,6,7,8,9]}}})).unwrap();
        assert!(none.is_empty());
    }
    #[test]
    fn acknowledgements_do_not_interpret_state_text() {
        assert!(validate_power_ack(&serde_json::json!({"POWER":"ACTIVE"}), 1, 1).is_ok());
        assert!(validate_power_ack(&serde_json::json!({"POWER1":"OFFLINE"}), 1, 1).is_ok());
        assert!(validate_power_ack(&serde_json::json!({"POWER4":"LOCKED"}), 4, 4).is_ok());
        assert!(validate_power_ack(&serde_json::json!({"POWER3":"LOCKED"}), 4, 4).is_err());
        assert!(parse_response(200, br#"{"Command":"Error"}"#).is_err());
    }
    #[test]
    fn response_errors_are_closed_and_redirects_rejected() {
        assert_eq!(
            parse_response(401, b"").unwrap_err(),
            ProtocolError::AuthenticationFailed
        );
        assert_eq!(
            parse_response(200, b"").unwrap_err(),
            ProtocolError::RefererBlocked
        );
        assert_eq!(
            parse_response(302, b"x").unwrap_err(),
            ProtocolError::UpstreamHttpStatus(302)
        );
        assert_eq!(
            parse_response(200, b"bad").unwrap_err(),
            ProtocolError::Protocol
        );
    }
    #[test]
    fn object_order_does_not_change_digest() {
        let a = parse_bounded_json(br#"{"GPIO":[1],"NAME":"G"}"#).unwrap();
        let b = parse_bounded_json(br#"{"NAME":"G","GPIO":[1]}"#).unwrap();
        assert_eq!(parse_template(&a).unwrap(), parse_template(&b).unwrap());
    }
}
