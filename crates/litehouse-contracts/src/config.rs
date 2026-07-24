use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

/// Closed metadata attached to one typed plugin-config field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ConfigFieldAnnotation {
    EnvironmentSecret {
        field_token: String,
        encoding: SecretEncoding,
    },
    PrivateLocal {
        projection: PrivateProjection,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SecretEncoding {
    Utf8,
    Base64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrivateProjection {
    ConfiguredStatus,
    Omitted,
}

/// A path and its closed annotation, emitted by `Config` implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotatedConfigField {
    pub json_pointer: String,
    pub annotation: ConfigFieldAnnotation,
}

/// Typed metadata for a plugin config schema.
pub trait ConfigMetadata {
    fn schema_json() -> String;
    fn annotations() -> Vec<AnnotatedConfigField>;
}

/// Safe projection of an environment requirement. It never contains the value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentRequirementStatus {
    pub field_token: String,
    pub variable_name: String,
    pub configured: bool,
}

pub fn validate_field_token(token: &str) -> Result<(), ConfigAnnotationError> {
    let mut bytes = token.bytes();
    let first = bytes
        .next()
        .ok_or(ConfigAnnotationError::InvalidFieldToken)?;
    if token.len() > 64
        || !first.is_ascii_uppercase()
        || bytes.any(|byte| !(byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'))
    {
        return Err(ConfigAnnotationError::InvalidFieldToken);
    }
    Ok(())
}

/// Derives the only environment-variable name accepted for one protected field.
pub fn environment_variable_name(
    instance_id: &str,
    field_token: &str,
) -> Result<String, ConfigAnnotationError> {
    validate_field_token(field_token)?;
    if !valid_lower_uuid(instance_id) {
        return Err(ConfigAnnotationError::InvalidInstanceId);
    }
    Ok(format!(
        "LITEHOUSE_INSTANCE_{}_{}",
        instance_id.replace('-', "_").to_ascii_uppercase(),
        field_token
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigAnnotationError {
    InvalidFieldToken,
    InvalidInstanceId,
}

impl fmt::Display for ConfigAnnotationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFieldToken => {
                formatter.write_str("field token must match [A-Z][A-Z0-9_]{0,63}")
            }
            Self::InvalidInstanceId => {
                formatter.write_str("instance ID must be a canonical lowercase UUID")
            }
        }
    }
}

impl Error for ConfigAnnotationError {}

fn valid_lower_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    value.bytes().enumerate().all(|(index, byte)| match index {
        8 | 13 | 18 | 23 => byte == b'-',
        _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_environment_name_without_accepting_a_caller_reference() {
        assert_eq!(
            environment_variable_name("018f0d9f-60f7-7c2b-8e7b-7be63f80fb88", "WEB_PASSWORD")
                .unwrap(),
            "LITEHOUSE_INSTANCE_018F0D9F_60F7_7C2B_8E7B_7BE63F80FB88_WEB_PASSWORD"
        );
    }

    #[test]
    fn rejects_noncanonical_tokens_and_instances() {
        assert!(validate_field_token("password").is_err());
        assert!(validate_field_token("A-B").is_err());
        assert!(environment_variable_name("owner-selected", "TOKEN").is_err());
    }
}
