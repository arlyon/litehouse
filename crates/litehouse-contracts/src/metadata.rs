use crate::{
    config::{AnnotatedConfigField, ConfigFieldAnnotation, validate_field_token},
    identity::{PackageIdentity, PackageRequirement},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, error::Error, fmt};

pub const METADATA_SECTION_NAME: &str = "litehouse:metadata-v1";
pub const METADATA_MAGIC: &[u8; 8] = b"LHMETA01";
pub const MAX_METADATA_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    Plugin,
    Contract,
    RuntimeProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvidedContract {
    pub identity: PackageIdentity,
    pub operations: Vec<String>,
    pub events: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequiredContract {
    pub requirement: PackageRequirement,
    pub operations: Vec<String>,
    pub selector: SelectorCardinality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectorCardinality {
    One,
    Optional,
    Many,
}

/// Maximum host facilities requested by a signed artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct HostFacilityRequests {
    pub exact_http_origins: Vec<String>,
    pub exact_tcp_origins: Vec<String>,
    pub clocks: bool,
    pub random: bool,
    pub tracing: bool,
}

/// Bounded metadata exported synchronously by a plugin component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeMetadataV1 {
    pub package: PackageIdentity,
    pub runtime_profile: PackageRequirement,
    pub description: Option<String>,
    pub provides: Vec<ProvidedContract>,
    pub requires: Vec<RequiredContract>,
    pub host_facilities: HostFacilityRequests,
    pub config_schema: Option<String>,
    pub config_annotations: Vec<AnnotatedConfigField>,
}

/// Unsigned, deterministic input that T5/T5b later binds into a signed release.
///
/// This type intentionally has no signature, key, publication target, or claim
/// that a component digest has been verified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnsignedReleaseInputV1 {
    pub kind: ArtifactKind,
    pub package: PackageIdentity,
    pub runtime_metadata_sha256: String,
    pub abi_sha256: String,
    pub source_sha256: String,
    pub license: String,
    pub source_revision: String,
    pub build_profile: String,
}

impl RuntimeMetadataV1 {
    pub fn encode_section(&self) -> Result<Vec<u8>, MetadataError> {
        self.validate()?;
        let json = serde_json::to_vec(self).map_err(|_| MetadataError::Malformed)?;
        if json.len() > MAX_METADATA_BYTES - METADATA_MAGIC.len() {
            return Err(MetadataError::TooLarge);
        }
        let mut encoded = Vec::with_capacity(METADATA_MAGIC.len() + json.len());
        encoded.extend_from_slice(METADATA_MAGIC);
        encoded.extend_from_slice(&json);
        Ok(encoded)
    }

    pub fn decode_section(bytes: &[u8]) -> Result<Self, MetadataError> {
        if bytes.len() > MAX_METADATA_BYTES {
            return Err(MetadataError::TooLarge);
        }
        let json = bytes
            .strip_prefix(METADATA_MAGIC)
            .ok_or(MetadataError::WrongVersion)?;
        let metadata: Self = serde_json::from_slice(json).map_err(|_| MetadataError::Malformed)?;
        metadata.validate()?;
        Ok(metadata)
    }

    pub fn validate(&self) -> Result<(), MetadataError> {
        if self
            .description
            .as_ref()
            .is_some_and(|value| value.len() > 1024)
            || self.provides.len() > 64
            || self.requires.len() > 64
            || self.host_facilities.exact_http_origins.len() > 16
            || self.host_facilities.exact_tcp_origins.len() > 16
            || self.config_annotations.len() > 128
            || self
                .config_schema
                .as_ref()
                .is_some_and(|schema| schema.len() > 48 * 1024)
        {
            return Err(MetadataError::BoundExceeded);
        }
        self.package
            .validate()
            .map_err(|_| MetadataError::Malformed)?;
        self.runtime_profile
            .validate()
            .map_err(|_| MetadataError::Malformed)?;
        for provided in &self.provides {
            provided
                .identity
                .validate()
                .map_err(|_| MetadataError::Malformed)?;
            validate_names(&provided.operations)?;
            validate_names(&provided.events)?;
        }
        for required in &self.requires {
            required
                .requirement
                .validate()
                .map_err(|_| MetadataError::Malformed)?;
            validate_names(&required.operations)?;
        }
        if !self
            .host_facilities
            .exact_http_origins
            .iter()
            .all(|origin| valid_http_origin(origin))
            || !self
                .host_facilities
                .exact_tcp_origins
                .iter()
                .all(|origin| valid_tcp_origin(origin))
        {
            return Err(MetadataError::Malformed);
        }
        validate_config_metadata(self.config_schema.as_deref(), &self.config_annotations)
    }
}

fn validate_names(names: &[String]) -> Result<(), MetadataError> {
    if names.len() > 128
        || names
            .iter()
            .any(|name| !valid_kebab_name(name) || name.len() > 64)
    {
        return Err(MetadataError::BoundExceeded);
    }
    Ok(())
}

fn valid_kebab_name(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || (byte == b'-'
                    && index > 0
                    && index + 1 < value.len()
                    && value.as_bytes()[index - 1] != b'-')
        })
}

fn valid_http_origin(value: &str) -> bool {
    if value.len() > 512 || !value.is_ascii() {
        return false;
    }
    let Ok(parsed) = url::Url::parse(value) else {
        return false;
    };
    matches!(parsed.scheme(), "http" | "https")
        && parsed.host().is_some()
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.path() == "/"
        && parsed.query().is_none()
        && parsed.fragment().is_none()
        && parsed.origin().ascii_serialization() == value
}

fn valid_tcp_origin(value: &str) -> bool {
    if value.len() > 512 || !value.is_ascii() {
        return false;
    }
    let Ok(parsed) = url::Url::parse(&format!("tcp://{value}")) else {
        return false;
    };
    let (Some(host), Some(port)) = (parsed.host(), parsed.port()) else {
        return false;
    };
    if port == 0
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || !matches!(parsed.path(), "" | "/")
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return false;
    }
    let host = match host {
        url::Host::Domain(host) => host.to_string(),
        url::Host::Ipv4(host) => host.to_string(),
        url::Host::Ipv6(host) => format!("[{host}]"),
    };
    value == format!("{host}:{port}")
}

fn validate_config_metadata(
    schema: Option<&str>,
    annotations: &[AnnotatedConfigField],
) -> Result<(), MetadataError> {
    let schema = match schema {
        Some(schema) => {
            let parsed: Value =
                serde_json::from_str(schema).map_err(|_| MetadataError::Malformed)?;
            if !parsed.is_object() {
                return Err(MetadataError::Malformed);
            }
            Some(parsed)
        }
        None if annotations.is_empty() => None,
        None => return Err(MetadataError::Malformed),
    };
    if annotations.is_empty() {
        return Ok(());
    }
    let properties = schema
        .as_ref()
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)
        .ok_or(MetadataError::Malformed)?;
    let mut pointers = HashSet::new();
    for field in annotations {
        if field.json_pointer.len() > 256 || !pointers.insert(field.json_pointer.as_str()) {
            return Err(MetadataError::BoundExceeded);
        }
        let property = decode_top_level_pointer(&field.json_pointer)?;
        if !properties.contains_key(&property) {
            return Err(MetadataError::Malformed);
        }
        if let ConfigFieldAnnotation::EnvironmentSecret { field_token, .. } = &field.annotation {
            validate_field_token(field_token).map_err(|_| MetadataError::Malformed)?;
        }
    }
    Ok(())
}

fn decode_top_level_pointer(pointer: &str) -> Result<String, MetadataError> {
    let encoded = pointer
        .strip_prefix('/')
        .filter(|property| !property.is_empty() && !property.contains('/'))
        .ok_or(MetadataError::Malformed)?;
    let mut decoded = String::with_capacity(encoded.len());
    let mut characters = encoded.chars();
    while let Some(character) = characters.next() {
        if character != '~' {
            decoded.push(character);
            continue;
        }
        match characters.next() {
            Some('0') => decoded.push('~'),
            Some('1') => decoded.push('/'),
            _ => return Err(MetadataError::Malformed),
        }
    }
    Ok(decoded)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataError {
    WrongVersion,
    TooLarge,
    BoundExceeded,
    Malformed,
}

impl fmt::Display for MetadataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongVersion => formatter.write_str("unsupported metadata envelope"),
            Self::TooLarge => formatter.write_str("metadata exceeds 64 KiB"),
            Self::BoundExceeded => formatter.write_str("metadata field bound exceeded"),
            Self::Malformed => formatter.write_str("malformed metadata"),
        }
    }
}

impl Error for MetadataError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Version;

    fn fixture() -> RuntimeMetadataV1 {
        RuntimeMetadataV1 {
            package: PackageIdentity::new("litehouse", "fixture", Version::V1).unwrap(),
            runtime_profile: PackageRequirement {
                namespace: "litehouse".into(),
                name: "runtime-profile".into(),
                minimum: Version::V1,
            },
            description: Some("fixture".into()),
            provides: Vec::new(),
            requires: Vec::new(),
            host_facilities: HostFacilityRequests::default(),
            config_schema: None,
            config_annotations: Vec::new(),
        }
    }

    #[test]
    fn metadata_envelope_is_typed_and_versioned() {
        let metadata = fixture();
        let encoded = metadata.encode_section().unwrap();
        assert_eq!(
            RuntimeMetadataV1::decode_section(&encoded).unwrap(),
            metadata
        );
    }

    #[test]
    fn metadata_rejects_unversioned_json() {
        assert_eq!(
            RuntimeMetadataV1::decode_section(br#"{}"#),
            Err(MetadataError::WrongVersion)
        );
        assert_eq!(
            RuntimeMetadataV1::decode_section(&vec![0; MAX_METADATA_BYTES + 1]),
            Err(MetadataError::TooLarge)
        );
    }

    #[test]
    fn metadata_validates_nested_authority_and_config_annotations() {
        let mut metadata = fixture();
        metadata.host_facilities.exact_http_origins = vec!["https://example.test/path".into()];
        assert_eq!(metadata.validate(), Err(MetadataError::Malformed));

        metadata.host_facilities.exact_http_origins = vec!["http://:".into()];
        assert_eq!(metadata.validate(), Err(MetadataError::Malformed));
        metadata.host_facilities.exact_http_origins =
            vec!["https://example.test:not-a-port".into()];
        assert_eq!(metadata.validate(), Err(MetadataError::Malformed));
        metadata.host_facilities.exact_http_origins = vec!["https://example.test".into()];
        metadata.host_facilities.exact_tcp_origins = vec!["[::1]:6053".into()];
        assert_eq!(metadata.validate(), Ok(()));

        metadata.config_schema = Some("{".into());
        assert_eq!(metadata.validate(), Err(MetadataError::Malformed));
        metadata.config_schema = Some(
            r#"{"type":"object","properties":{"token":{"type":"string"},"foo~2bar":{"type":"string"}}}"#
                .into(),
        );
        metadata.config_annotations = vec![AnnotatedConfigField {
            json_pointer: "/missing".into(),
            annotation: ConfigFieldAnnotation::EnvironmentSecret {
                field_token: "bad-token".into(),
                encoding: crate::config::SecretEncoding::Utf8,
            },
        }];
        assert_eq!(metadata.validate(), Err(MetadataError::Malformed));

        metadata.config_annotations[0].json_pointer = "/foo~2bar".into();
        metadata.config_annotations[0].annotation = ConfigFieldAnnotation::EnvironmentSecret {
            field_token: "API_TOKEN".into(),
            encoding: crate::config::SecretEncoding::Utf8,
        };
        assert_eq!(metadata.validate(), Err(MetadataError::Malformed));

        metadata.config_annotations[0].json_pointer = "/token".into();
        metadata.config_annotations[0].annotation = ConfigFieldAnnotation::EnvironmentSecret {
            field_token: "API_TOKEN".into(),
            encoding: crate::config::SecretEncoding::Utf8,
        };
        assert_eq!(metadata.validate(), Ok(()));
    }
}
