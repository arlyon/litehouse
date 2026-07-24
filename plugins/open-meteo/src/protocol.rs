use serde::{
    Deserialize,
    de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::{collections::BTreeSet, fmt};

pub(crate) const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_JSON_DEPTH: usize = 8;
const MAX_CONFIG_BYTES: usize = 4096;
pub(crate) const AUTHORITY: &str = "api.open-meteo.com:443";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProtocolError {
    InvalidConfiguration,
    RequestRejected,
    RateLimited,
    UnsupportedAccessTier,
    UpstreamUnavailable,
    Protocol,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    latitude: f64,
    longitude: f64,
    usage: String,
    #[serde(default = "default_poll_seconds")]
    poll_seconds: u16,
    label: Option<String>,
}

fn default_poll_seconds() -> u16 {
    900
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    pub(crate) latitude: String,
    pub(crate) longitude: String,
    pub(crate) poll_seconds: u16,
    pub(crate) label: Option<String>,
}

impl Config {
    pub(crate) fn from_json(input: &str) -> Result<Self, ProtocolError> {
        if input.len() > MAX_CONFIG_BYTES {
            return Err(ProtocolError::InvalidConfiguration);
        }
        let raw: RawConfig =
            serde_json::from_str(input).map_err(|_| ProtocolError::InvalidConfiguration)?;
        if !raw.latitude.is_finite()
            || !(-90.0..=90.0).contains(&raw.latitude)
            || !raw.longitude.is_finite()
            || !(-180.0..=180.0).contains(&raw.longitude)
            || raw.usage != "free-noncommercial"
            || !(900..=3600).contains(&raw.poll_seconds)
            || raw.label.as_ref().is_some_and(|label| {
                label.is_empty() || label.len() > 256 || label.chars().count() > 80
            })
        {
            return Err(ProtocolError::InvalidConfiguration);
        }
        Ok(Self {
            latitude: canonical_coordinate(raw.latitude),
            longitude: canonical_coordinate(raw.longitude),
            poll_seconds: raw.poll_seconds,
            label: raw.label,
        })
    }

    pub(crate) fn request(&self) -> RequestSpec {
        RequestSpec {
            authority: AUTHORITY,
            path_with_query: format!(
                "/v1/forecast?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code&temperature_unit=celsius&wind_speed_unit=ms&timeformat=unixtime&timezone=GMT",
                self.latitude, self.longitude
            ),
        }
    }
}

fn canonical_coordinate(value: f64) -> String {
    let mut out = format!("{value:.6}");
    while out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    if out == "-0" {
        out = "0".into();
    }
    out
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RequestSpec {
    pub(crate) authority: &'static str,
    pub(crate) path_with_query: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WeatherField {
    Temperature,
    RelativeHumidity,
    WindSpeed,
    WeatherCode,
}

impl WeatherField {
    pub(crate) const ALL: [Self; 4] = [
        Self::Temperature,
        Self::RelativeHumidity,
        Self::WindSpeed,
        Self::WeatherCode,
    ];

    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Temperature => 0,
            Self::RelativeHumidity => 1,
            Self::WindSpeed => 2,
            Self::WeatherCode => 3,
        }
    }

    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::Temperature => "current/temperature-2m",
            Self::RelativeHumidity => "current/relative-humidity-2m",
            Self::WindSpeed => "current/wind-speed-10m",
            Self::WeatherCode => "current/weather-code",
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Temperature => "Temperature at 2 m",
            Self::RelativeHumidity => "Relative humidity at 2 m",
            Self::WindSpeed => "Wind speed at 10 m",
            Self::WeatherCode => "WMO weather code",
        }
    }

    pub(crate) const fn api_name(self) -> &'static str {
        match self {
            Self::Temperature => "temperature_2m",
            Self::RelativeHumidity => "relative_humidity_2m",
            Self::WindSpeed => "wind_speed_10m",
            Self::WeatherCode => "weather_code",
        }
    }

    pub(crate) const fn api_unit(self) -> &'static str {
        match self {
            Self::Temperature => "°C",
            Self::RelativeHumidity => "%",
            Self::WindSpeed => "m/s",
            Self::WeatherCode => "wmo code",
        }
    }

    pub(crate) const fn unit(self) -> &'static str {
        match self {
            Self::Temperature => "Cel",
            Self::RelativeHumidity | Self::WeatherCode => "1",
            Self::WindSpeed => "m/s",
        }
    }

    pub(crate) const fn profile(self) -> &'static str {
        match self {
            Self::Temperature => "litehouse:quantity/temperature-absolute@1",
            Self::RelativeHumidity => "litehouse:quantity/relative-humidity@1",
            Self::WindSpeed => "litehouse:quantity/speed@1",
            Self::WeatherCode => "litehouse:quantity/wmo-weather-code@1",
        }
    }

    pub(crate) const fn semantic_name(self) -> &'static str {
        match self {
            Self::Temperature => "temperature-2m",
            Self::RelativeHumidity => "relative-humidity-2m",
            Self::WindSpeed => "wind-speed-10m",
            Self::WeatherCode => "weather-code-wmo",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WeatherSnapshot {
    pub(crate) grid_latitude: f64,
    pub(crate) grid_longitude: f64,
    pub(crate) source_time_ms: u64,
    pub(crate) values: [Option<String>; 4],
}

pub(crate) fn parse_response(status: u16, body: &[u8]) -> Result<WeatherSnapshot, ProtocolError> {
    match status {
        200 => parse_weather(&parse_bounded_json(body)?),
        400 => Err(ProtocolError::RequestRejected),
        401 | 403 => Err(ProtocolError::UnsupportedAccessTier),
        429 => Err(ProtocolError::RateLimited),
        500..=599 => Err(ProtocolError::UpstreamUnavailable),
        _ => Err(ProtocolError::Protocol),
    }
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

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded duplicate-free JSON")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }

    fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("non-finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.into()))
    }

    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
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

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut out = Vec::new();
        while let Some(value) = sequence.next_element_seed(ValueSeed(self.0 + 1))? {
            out.push(value);
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

fn parse_weather(value: &Value) -> Result<WeatherSnapshot, ProtocolError> {
    let root = value.as_object().ok_or(ProtocolError::Protocol)?;
    let grid_latitude =
        bounded_f64(root.get("latitude"), -90.0, 90.0).ok_or(ProtocolError::Protocol)?;
    let grid_longitude =
        bounded_f64(root.get("longitude"), -180.0, 180.0).ok_or(ProtocolError::Protocol)?;
    let current = root
        .get("current")
        .and_then(Value::as_object)
        .ok_or(ProtocolError::Protocol)?;
    let source_time_ms = current
        .get("time")
        .and_then(Value::as_u64)
        .and_then(|seconds| seconds.checked_mul(1000))
        .ok_or(ProtocolError::Protocol)?;

    if let Some(units) = root.get("current_units") {
        let units = units.as_object().ok_or(ProtocolError::Protocol)?;
        for field in WeatherField::ALL {
            if units.get(field.api_name()).and_then(Value::as_str) != Some(field.api_unit()) {
                return Err(ProtocolError::Protocol);
            }
        }
    }

    let values = std::array::from_fn(|index| {
        let field = WeatherField::ALL[index];
        parse_field(field, current.get(field.api_name()))
    });
    Ok(WeatherSnapshot {
        grid_latitude,
        grid_longitude,
        source_time_ms,
        values,
    })
}

fn parse_field(field: WeatherField, value: Option<&Value>) -> Option<String> {
    let number = value?.as_f64().filter(|value| value.is_finite())?;
    let valid = match field {
        WeatherField::Temperature => (-150.0..=100.0).contains(&number),
        WeatherField::RelativeHumidity => (0.0..=100.0).contains(&number),
        WeatherField::WindSpeed => (0.0..=500.0).contains(&number),
        WeatherField::WeatherCode => (0.0..=99.0).contains(&number) && number.fract() == 0.0,
    };
    if !valid {
        return None;
    }
    canonical_magnitude(value?.as_number()?)
}

fn bounded_f64(value: Option<&Value>, minimum: f64, maximum: f64) -> Option<f64> {
    value?
        .as_f64()
        .filter(|value| value.is_finite() && (minimum..=maximum).contains(value))
}

fn canonical_magnitude(number: &Number) -> Option<String> {
    if let Some(value) = number.as_i64() {
        return Some(value.to_string());
    }
    if let Some(value) = number.as_u64() {
        return Some(value.to_string());
    }
    let value = number.as_f64()?;
    let mut out = format!("{value:.6}");
    while out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    (out != "-0").then_some(out).or_else(|| Some("0".into()))
}

#[derive(Clone, Copy, Debug)]
struct Window {
    duration_seconds: u64,
    limit: u32,
    started_at: u64,
    used: u32,
}

impl Window {
    const fn new(duration_seconds: u64, limit: u32, now_seconds: u64) -> Self {
        Self {
            duration_seconds,
            limit,
            started_at: now_seconds,
            used: 0,
        }
    }

    fn refresh(&mut self, now_seconds: u64) -> bool {
        if now_seconds < self.started_at {
            return false;
        }
        if now_seconds - self.started_at >= self.duration_seconds {
            self.started_at = now_seconds;
            self.used = 0;
        }
        true
    }
}

#[derive(Debug)]
pub(crate) struct RateBudget {
    windows: [Window; 4],
}

impl RateBudget {
    pub(crate) fn new(now_seconds: u64) -> Self {
        Self {
            windows: [
                Window::new(60, 500, now_seconds),
                Window::new(3_600, 4_000, now_seconds),
                Window::new(86_400, 9_000, now_seconds),
                Window::new(2_592_000, 270_000, now_seconds),
            ],
        }
    }

    pub(crate) fn take(&mut self, now_seconds: u64) -> bool {
        if !self
            .windows
            .iter_mut()
            .all(|window| window.refresh(now_seconds))
            || self
                .windows
                .iter()
                .any(|window| window.used >= window.limit)
        {
            return false;
        }
        for window in &mut self.windows {
            window.used += 1;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn success() -> Vec<u8> {
        r#"{
          "latitude":52.52,"longitude":13.419998,
          "current_units":{"time":"unixtime","temperature_2m":"°C","relative_humidity_2m":"%","wind_speed_10m":"m/s","weather_code":"wmo code"},
          "current":{"time":1710000000,"temperature_2m":12.5,"relative_humidity_2m":71,"wind_speed_10m":4.25,"weather_code":3}
        }"#
            .as_bytes()
            .to_vec()
    }

    #[test]
    fn config_is_closed_bounded_and_acknowledged() {
        let valid = r#"{"latitude":52.520008,"longitude":13.404954,"usage":"free-noncommercial"}"#;
        let config = Config::from_json(valid).unwrap();
        assert_eq!(config.poll_seconds, 900);
        for invalid in [
            r#"{"latitude":91,"longitude":0,"usage":"free-noncommercial"}"#,
            r#"{"latitude":0,"longitude":181,"usage":"free-noncommercial"}"#,
            r#"{"latitude":0,"longitude":0,"usage":"commercial"}"#,
            r#"{"latitude":0,"longitude":0,"usage":"free-noncommercial","poll_seconds":899}"#,
            r#"{"latitude":0,"longitude":0,"usage":"free-noncommercial","url":"https://example.com"}"#,
        ] {
            assert!(Config::from_json(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn request_is_an_exact_https_origin_and_fixed_query() {
        let config = Config::from_json(
            r#"{"latitude":52.5200084,"longitude":-0.0000001,"usage":"free-noncommercial"}"#,
        )
        .unwrap();
        let request = config.request();
        assert_eq!(request.authority, "api.open-meteo.com:443");
        assert_eq!(
            request.path_with_query,
            "/v1/forecast?latitude=52.520008&longitude=0&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code&temperature_unit=celsius&wind_speed_unit=ms&timeformat=unixtime&timezone=GMT"
        );
        assert!(!request.path_with_query.contains("apikey"));
    }

    #[test]
    fn parser_preserves_source_time_grid_and_four_values() {
        let snapshot = parse_response(200, &success()).unwrap();
        assert_eq!(snapshot.grid_latitude, 52.52);
        assert_eq!(snapshot.grid_longitude, 13.419998);
        assert_eq!(snapshot.source_time_ms, 1_710_000_000_000);
        assert_eq!(
            snapshot.values,
            [
                Some("12.5".into()),
                Some("71".into()),
                Some("4.25".into()),
                Some("3".into())
            ]
        );
    }

    #[test]
    fn each_missing_or_invalid_field_is_independently_unknown() {
        for field in WeatherField::ALL {
            let mut value = parse_bounded_json(&success()).unwrap();
            value["current"]
                .as_object_mut()
                .unwrap()
                .remove(field.api_name());
            let body = serde_json::to_vec(&value).unwrap();
            let snapshot = parse_response(200, &body).unwrap();
            assert!(snapshot.values[field.index()].is_none());
            assert_eq!(
                snapshot
                    .values
                    .iter()
                    .filter(|value| value.is_some())
                    .count(),
                3
            );
        }
        let mut value = parse_bounded_json(&success()).unwrap();
        value["current"]["relative_humidity_2m"] = Value::Null;
        value["current"]["weather_code"] = serde_json::json!(3.5);
        let snapshot = parse_response(200, &serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(snapshot.values[1].is_none());
        assert!(snapshot.values[3].is_none());
    }

    #[test]
    fn units_duplicate_depth_size_and_redirect_are_rejected() {
        let mut wrong = parse_bounded_json(&success()).unwrap();
        wrong["current_units"]["wind_speed_10m"] = Value::String("km/h".into());
        assert!(parse_response(200, &serde_json::to_vec(&wrong).unwrap()).is_err());
        assert!(parse_bounded_json(br#"{"x":1,"x":2}"#).is_err());
        assert!(parse_bounded_json(br#"[[[[[[[[[0]]]]]]]]]"#).is_err());
        assert!(parse_bounded_json(&vec![b' '; MAX_RESPONSE_BYTES + 1]).is_err());
        assert_eq!(parse_response(302, b""), Err(ProtocolError::Protocol));
    }

    #[test]
    fn provider_errors_are_closed() {
        assert_eq!(
            parse_response(400, b"upstream text"),
            Err(ProtocolError::RequestRejected)
        );
        assert_eq!(parse_response(429, b""), Err(ProtocolError::RateLimited));
        assert_eq!(
            parse_response(403, b""),
            Err(ProtocolError::UnsupportedAccessTier)
        );
        assert_eq!(
            parse_response(503, b""),
            Err(ProtocolError::UpstreamUnavailable)
        );
    }

    #[test]
    fn rate_budget_enforces_safety_margin_without_unbounded_history() {
        let mut budget = RateBudget::new(10);
        for _ in 0..500 {
            assert!(budget.take(10));
        }
        assert!(!budget.take(10));
        assert!(budget.take(70));
        assert!(!budget.take(69));
        assert_eq!(budget.windows.len(), 4);
    }
}
