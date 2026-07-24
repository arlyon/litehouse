use serde::{
    Deserialize,
    de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use std::{collections::BTreeSet, fmt};
use url::{Host, Url};
use zeroize::{Zeroize, Zeroizing};

pub(crate) const MAX_BODY_BYTES: usize = 4_096;
pub(crate) const MAX_TITLE_BYTES: usize = 256;
pub(crate) const MAX_TITLE_SCALARS: usize = 80;
pub(crate) const MAX_RESPONSE_BYTES: usize = 16 * 1_024;
pub(crate) const MAX_RESPONSE_HEADER_BYTES: usize = 8 * 1_024;
const MAX_REQUEST_HEADER_BYTES: usize = 2 * 1_024;
const MAX_JSON_DEPTH: usize = 4;
const MAX_CONFIG_BYTES: usize = 4_096;
const MAX_HOST_BYTES: usize = 253;
const MAX_TOPIC_BYTES: usize = 64;
const MAX_TOKEN_BYTES: usize = 1_024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProtocolError {
    InvalidConfiguration,
    RequestRejected,
    AuthorizationFailed,
    SizeRejected,
    RateLimited,
    OutcomeUnknown,
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
    origin: String,
    topic: String,
    #[serde(rename = "ACCESS_TOKEN")]
    access_token: Secret,
}

pub(crate) struct Config {
    scheme: Scheme,
    authority: String,
    path: String,
    access_token: Secret,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scheme {
    Http,
    Https,
}

impl Config {
    pub(crate) fn from_json(input: &str) -> Result<Self, ProtocolError> {
        if input.len() > MAX_CONFIG_BYTES {
            return Err(ProtocolError::InvalidConfiguration);
        }
        let raw: RawConfig =
            serde_json::from_str(input).map_err(|_| ProtocolError::InvalidConfiguration)?;
        if !valid_topic(&raw.topic)
            || raw.access_token.expose().is_empty()
            || raw.access_token.expose().len() > MAX_TOKEN_BYTES
            || raw.access_token.expose().contains(['\r', '\n'])
        {
            return Err(ProtocolError::InvalidConfiguration);
        }

        let url = Url::parse(&raw.origin).map_err(|_| ProtocolError::InvalidConfiguration)?;
        let scheme = match url.scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            _ => return Err(ProtocolError::InvalidConfiguration),
        };
        if !url.username().is_empty()
            || url.password().is_some()
            || !matches!(url.path(), "" | "/")
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(ProtocolError::InvalidConfiguration);
        }
        let host = url.host().ok_or(ProtocolError::InvalidConfiguration)?;
        let host_text = url.host_str().ok_or(ProtocolError::InvalidConfiguration)?;
        if host_text.is_empty() || host_text.len() > MAX_HOST_BYTES || !host_text.is_ascii() {
            return Err(ProtocolError::InvalidConfiguration);
        }
        if scheme == Scheme::Http && !private_http_host(&host) {
            return Err(ProtocolError::InvalidConfiguration);
        }
        let port = url
            .port_or_known_default()
            .ok_or(ProtocolError::InvalidConfiguration)?;
        let authority = match host {
            Host::Domain(domain) => format!("{}:{port}", domain.to_ascii_lowercase()),
            Host::Ipv4(address) => format!("{address}:{port}"),
            Host::Ipv6(address) => format!("[{address}]:{port}"),
        };

        Ok(Self {
            scheme,
            authority,
            path: format!("/{}", raw.topic),
            access_token: raw.access_token,
        })
    }

    pub(crate) fn request(
        &self,
        body: String,
        title: Option<String>,
        priority: Option<&'static str>,
    ) -> Result<RequestSpec, ProtocolError> {
        let authorization = Zeroizing::new(format!("Bearer {}", self.access_token.expose()));
        let content_length = body.len().to_string();
        let headers = [
            ("Host", Some(self.authority.as_bytes())),
            ("Authorization", Some(authorization.as_bytes())),
            ("Content-Type", Some("text/plain; charset=utf-8".as_bytes())),
            ("Content-Length", Some(content_length.as_bytes())),
            ("Title", title.as_deref().map(str::as_bytes)),
            ("Priority", priority.map(str::as_bytes)),
        ];
        if serialized_header_bytes(headers) > MAX_REQUEST_HEADER_BYTES {
            return Err(ProtocolError::InvalidConfiguration);
        }
        Ok(RequestSpec {
            scheme: self.scheme,
            authority: self.authority.clone(),
            path_with_query: self.path.clone(),
            authorization,
            content_length,
            title,
            priority,
            body: body.into_bytes(),
        })
    }
}

fn private_http_host(host: &Host<&str>) -> bool {
    match host {
        Host::Domain(_) => false,
        Host::Ipv4(address) => {
            address.is_private() || address.is_loopback() || address.is_link_local()
        }
        Host::Ipv6(address) => {
            address.is_loopback() || address.is_unique_local() || address.is_unicast_link_local()
        }
    }
}

fn valid_topic(topic: &str) -> bool {
    !topic.is_empty()
        && topic.len() <= MAX_TOPIC_BYTES
        && topic
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn serialized_header_bytes<'a>(
    headers: impl IntoIterator<Item = (&'a str, Option<&'a [u8]>)>,
) -> usize {
    headers
        .into_iter()
        .filter_map(|(name, value)| value.map(|value| name.len() + 2 + value.len() + 2))
        .try_fold(2_usize, usize::checked_add)
        .unwrap_or(usize::MAX)
}

pub(crate) struct RequestSpec {
    pub(crate) scheme: Scheme,
    pub(crate) authority: String,
    pub(crate) path_with_query: String,
    authorization: Zeroizing<String>,
    content_length: String,
    title: Option<String>,
    priority: Option<&'static str>,
    pub(crate) body: Vec<u8>,
}

impl RequestSpec {
    pub(crate) fn authorization(&self) -> &[u8] {
        self.authorization.as_bytes()
    }

    pub(crate) fn content_length(&self) -> &[u8] {
        self.content_length.as_bytes()
    }

    pub(crate) fn title(&self) -> Option<&[u8]> {
        self.title.as_deref().map(str::as_bytes)
    }

    pub(crate) fn priority(&self) -> Option<&[u8]> {
        self.priority.map(str::as_bytes)
    }
}

pub(crate) fn valid_body(body: &str) -> bool {
    body.len() <= MAX_BODY_BYTES
}

pub(crate) fn valid_title(title: &str) -> bool {
    title.len() <= MAX_TITLE_BYTES
        && title.chars().count() <= MAX_TITLE_SCALARS
        && !title.contains(['\r', '\n'])
}

pub(crate) fn response_headers_within_bound(headers: &[(String, Vec<u8>)]) -> bool {
    serialized_header_bytes(
        headers
            .iter()
            .map(|(name, value)| (name.as_str(), Some(value.as_slice()))),
    ) <= MAX_RESPONSE_HEADER_BYTES
}

pub(crate) fn parse_response(status: u16, body: &[u8]) -> Result<String, ProtocolError> {
    match status {
        200 => parse_message_id(body),
        400 => Err(ProtocolError::RequestRejected),
        401 | 403 => Err(ProtocolError::AuthorizationFailed),
        413 => Err(ProtocolError::SizeRejected),
        429 => Err(ProtocolError::RateLimited),
        _ => Err(ProtocolError::OutcomeUnknown),
    }
}

fn parse_message_id(body: &[u8]) -> Result<String, ProtocolError> {
    let value = parse_bounded_json(body)?;
    let object = value.as_object().ok_or(ProtocolError::OutcomeUnknown)?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 64 && id.is_ascii())
        .ok_or(ProtocolError::OutcomeUnknown)?;
    Ok(id.into())
}

fn parse_bounded_json(body: &[u8]) -> Result<Value, ProtocolError> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(ProtocolError::OutcomeUnknown);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(body);
    let value = ValueSeed(1)
        .deserialize(&mut deserializer)
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    deserializer
        .end()
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    Ok(value)
}

struct ValueSeed(usize);

impl<'de> DeserializeSeed<'de> for ValueSeed {
    type Value = Value;

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        if self.0 > MAX_JSON_DEPTH {
            return Err(D::Error::custom("JSON nesting limit exceeded"));
        }
        deserializer.deserialize_any(ValueVisitor(self.0))
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

    fn visit_some<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        ValueSeed(self.0).deserialize(deserializer)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn config(origin: &str, topic: &str, token: &str) -> String {
        serde_json::json!({"origin": origin, "topic": topic, "ACCESS_TOKEN": token}).to_string()
    }

    #[test]
    fn origin_and_topic_are_closed_and_normalized() {
        let parsed =
            Config::from_json(&config("HTTPS://Ntfy.Sh/", "home_alerts-1", "token")).unwrap();
        let request = parsed.request("hello".into(), None, None).unwrap();
        assert_eq!(request.scheme, Scheme::Https);
        assert_eq!(request.authority, "ntfy.sh:443");
        assert_eq!(request.path_with_query, "/home_alerts-1");

        for invalid in [
            config("https://ntfy.sh/x", "topic", "token"),
            config("https://user@ntfy.sh", "topic", "token"),
            config("https://ntfy.sh?x=1", "topic", "token"),
            config("http://example.com", "topic", "token"),
            config("https://ntfy.sh", "bad/topic", "token"),
            config("https://ntfy.sh", "topic", ""),
        ] {
            assert!(Config::from_json(&invalid).is_err(), "accepted {invalid}");
        }
        assert!(Config::from_json(&config("http://127.0.0.1:8080", "topic", "x")).is_ok());
    }

    #[test]
    fn request_is_exact_bearer_post_material() {
        let config =
            Config::from_json(&config("https://ntfy.sh", "alerts", "secret-token")).unwrap();
        let request = config
            .request("héllo".into(), Some("Laundry".into()), Some("high"))
            .unwrap();
        assert_eq!(request.path_with_query, "/alerts");
        assert_eq!(request.authorization(), b"Bearer secret-token");
        assert_eq!(request.content_length(), b"6");
        assert_eq!(request.title(), Some("Laundry".as_bytes()));
        assert_eq!(request.priority(), Some(b"high".as_slice()));
        assert_eq!(request.body, "héllo".as_bytes());
    }

    #[test]
    fn request_content_bounds_are_utf8_exact() {
        assert!(valid_body(&"a".repeat(MAX_BODY_BYTES)));
        assert!(!valid_body(&"é".repeat(MAX_BODY_BYTES / 2 + 1)));
        assert!(valid_title(&"é".repeat(80)));
        assert!(!valid_title(&"a".repeat(81)));
        assert!(!valid_title(&"é".repeat(129)));
        assert!(!valid_title("header\ninjection"));
    }

    #[test]
    fn only_exact_bounded_duplicate_safe_200_id_is_accepted() {
        assert_eq!(
            parse_response(200, br#"{"id":"abc123","extra":{"ok":true}}"#).unwrap(),
            "abc123"
        );
        for body in [
            r#"{}"#.as_bytes(),
            r#"[]"#.as_bytes(),
            r#"{"id":""}"#.as_bytes(),
            r#"{"id":1}"#.as_bytes(),
            r#"{"id":"a","id":"b"}"#.as_bytes(),
            r#"{"id":"é"}"#.as_bytes(),
            r#"{"id":"a","x":{"y":{"z":{"too":1}}}}"#.as_bytes(),
        ] {
            assert_eq!(
                parse_response(200, body),
                Err(ProtocolError::OutcomeUnknown)
            );
        }
        assert_eq!(
            parse_response(200, format!(r#"{{"id":"{}"}}"#, "a".repeat(65)).as_bytes()),
            Err(ProtocolError::OutcomeUnknown)
        );
        assert_eq!(
            parse_response(200, &vec![b' '; MAX_RESPONSE_BYTES + 1]),
            Err(ProtocolError::OutcomeUnknown)
        );
    }

    #[test]
    fn statuses_are_definite_only_when_documented() {
        for (status, expected) in [
            (400, ProtocolError::RequestRejected),
            (401, ProtocolError::AuthorizationFailed),
            (403, ProtocolError::AuthorizationFailed),
            (413, ProtocolError::SizeRejected),
            (429, ProtocolError::RateLimited),
            (201, ProtocolError::OutcomeUnknown),
            (302, ProtocolError::OutcomeUnknown),
            (404, ProtocolError::OutcomeUnknown),
            (500, ProtocolError::OutcomeUnknown),
        ] {
            assert_eq!(parse_response(status, b"ignored"), Err(expected));
        }
    }

    #[test]
    fn response_header_bound_counts_wire_framing() {
        let exact = vec![("x".into(), vec![b'a'; MAX_RESPONSE_HEADER_BYTES - 7])];
        assert!(response_headers_within_bound(&exact));
        let over = vec![("x".into(), vec![b'a'; MAX_RESPONSE_HEADER_BYTES - 6])];
        assert!(!response_headers_within_bound(&over));
    }

    #[test]
    fn token_and_generated_headers_are_bounded() {
        let max_token = "x".repeat(MAX_TOKEN_BYTES);
        let parsed = Config::from_json(&config("https://ntfy.sh", "topic", &max_token)).unwrap();
        assert!(
            parsed
                .request(
                    "x".repeat(MAX_BODY_BYTES),
                    Some("é".repeat(80)),
                    Some("max")
                )
                .is_ok()
        );
        assert!(
            Config::from_json(&config(
                "https://ntfy.sh",
                "topic",
                &"x".repeat(MAX_TOKEN_BYTES + 1)
            ))
            .is_err()
        );
    }

    #[test]
    fn private_http_classification_is_ip_only() {
        use std::net::IpAddr;

        for address in [
            IpAddr::from([10, 0, 0, 1]),
            IpAddr::from([172, 16, 0, 1]),
            IpAddr::from([192, 168, 0, 1]),
            IpAddr::from([127, 0, 0, 1]),
        ] {
            let origin = format!("http://{address}:8080");
            assert!(Config::from_json(&config(&origin, "topic", "x")).is_ok());
        }
        assert!(Config::from_json(&config("http://8.8.8.8", "topic", "x")).is_err());
    }
}
