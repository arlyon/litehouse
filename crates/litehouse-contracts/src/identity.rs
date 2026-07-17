use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

/// An immutable semantic version used by contract and runtime-profile records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub const V1: Self = Self {
        major: 1,
        minor: 0,
        patch: 0,
    };
}

impl fmt::Display for Version {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A versioned WIT package identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageIdentity {
    pub namespace: String,
    pub name: String,
    pub version: Version,
}

impl PackageIdentity {
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
        version: Version,
    ) -> Result<Self, IdentityError> {
        let identity = Self {
            namespace: namespace.into(),
            name: name.into(),
            version,
        };
        validate_kebab_identifier("namespace", &identity.namespace)?;
        validate_kebab_identifier("name", &identity.name)?;
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate(&self) -> Result<(), IdentityError> {
        validate_kebab_identifier("namespace", &self.namespace)?;
        validate_kebab_identifier("name", &self.name)?;
        Ok(())
    }

    pub fn exact_major(&self) -> ExactMajorIdentity<'_> {
        ExactMajorIdentity {
            namespace: &self.namespace,
            name: &self.name,
            major: self.version.major,
        }
    }
}

/// Runtime routing identity: namespace, package name, and exact major.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactMajorIdentity<'a> {
    pub namespace: &'a str,
    pub name: &'a str,
    pub major: u32,
}

/// A same-major lower-bound requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageRequirement {
    pub namespace: String,
    pub name: String,
    pub minimum: Version,
}

impl PackageRequirement {
    pub fn validate(&self) -> Result<(), IdentityError> {
        validate_kebab_identifier("namespace", &self.namespace)?;
        validate_kebab_identifier("name", &self.name)?;
        if self.minimum.major == 0 {
            return Err(IdentityError::ZeroMajor);
        }
        Ok(())
    }

    pub fn accepts(&self, candidate: &PackageIdentity) -> bool {
        self.namespace == candidate.namespace
            && self.name == candidate.name
            && self.minimum.major == candidate.version.major
            && candidate.version >= self.minimum
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    InvalidIdentifier { field: &'static str },
    ZeroMajor,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentifier { field } => {
                write!(formatter, "{field} must be a bounded kebab identifier")
            }
            Self::ZeroMajor => {
                formatter.write_str("contract requirements must use a non-zero major")
            }
        }
    }
}

impl Error for IdentityError {}

fn validate_kebab_identifier(field: &'static str, value: &str) -> Result<(), IdentityError> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || (byte == b'-'
                    && index > 0
                    && index + 1 < value.len()
                    && value.as_bytes()[index - 1] != b'-')
        })
        && !value.ends_with('-');
    if valid {
        Ok(())
    } else {
        Err(IdentityError::InvalidIdentifier { field })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prerelease_plugin_identity_is_valid_but_contract_requirements_need_a_major() {
        PackageIdentity::new(
            "litehouse",
            "fixture-plugin",
            Version {
                major: 0,
                minor: 1,
                patch: 0,
            },
        )
        .unwrap();
        assert!(
            PackageRequirement {
                namespace: "litehouse".into(),
                name: "fixture".into(),
                minimum: Version {
                    major: 0,
                    minor: 1,
                    patch: 0,
                },
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn exact_major_requirement_accepts_only_same_package_and_major() {
        let requirement = PackageRequirement {
            namespace: "litehouse".into(),
            name: "on-off".into(),
            minimum: Version {
                major: 1,
                minor: 2,
                patch: 3,
            },
        };
        assert!(
            requirement.accepts(
                &PackageIdentity::new(
                    "litehouse",
                    "on-off",
                    Version {
                        major: 1,
                        minor: 9,
                        patch: 0,
                    },
                )
                .unwrap()
            )
        );
        assert!(
            !requirement
                .accepts(&PackageIdentity::new("litehouse", "on-off", Version::V1).unwrap())
        );
        assert!(
            !requirement.accepts(
                &PackageIdentity::new(
                    "litehouse",
                    "on-off",
                    Version {
                        major: 2,
                        minor: 0,
                        patch: 0,
                    },
                )
                .unwrap()
            )
        );
    }
}
