use litehouse_contracts::identity::{PackageIdentity, PackageRequirement, Version};
use std::{error::Error, fmt};

macro_rules! string_id {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdentityError> {
                let value = value.into();
                validate_id($label, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

string_id!(InstanceId, "instance ID");
string_id!(FacetKey, "facet key");
string_id!(FeatureId, "feature ID");
string_id!(OperationId, "operation ID");
string_id!(EventId, "event ID");
string_id!(SourceEventId, "source event ID");

fn validate_id(field: &'static str, value: &str) -> Result<(), IdentityError> {
    if value.is_empty()
        || value.len() > 128
        || !value.is_ascii()
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(IdentityError::InvalidId(field));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Generation(u64);

impl Generation {
    pub fn new(value: u64) -> Result<Self, IdentityError> {
        nonzero("generation", value).map(Self)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Incarnation(u64);

impl Incarnation {
    pub fn new(value: u64) -> Result<Self, IdentityError> {
        nonzero("incarnation", value).map(Self)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GrantRevision(u64);

impl GrantRevision {
    pub fn new(value: u64) -> Result<Self, IdentityError> {
        nonzero("grant revision", value).map(Self)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}

fn nonzero(field: &'static str, value: u64) -> Result<u64, IdentityError> {
    if value == 0 {
        Err(IdentityError::Zero(field))
    } else {
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractId {
    namespace: String,
    name: String,
    major: u32,
}

impl ContractId {
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
        major: u32,
    ) -> Result<Self, IdentityError> {
        let namespace = namespace.into();
        let name = name.into();
        if major == 0 {
            return Err(IdentityError::Zero("contract major"));
        }
        PackageIdentity::new(
            namespace.clone(),
            name.clone(),
            Version {
                major,
                minor: 0,
                patch: 0,
            },
        )
        .map_err(|_| IdentityError::InvalidContract)?;
        Ok(Self {
            namespace,
            name,
            major,
        })
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn major(&self) -> u32 {
        self.major
    }
}

impl fmt::Display for ContractId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}@{}", self.namespace, self.name, self.major)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractRequirement {
    contract: ContractId,
    minimum: Version,
}

impl ContractRequirement {
    pub fn new(contract: ContractId, minimum: Version) -> Result<Self, IdentityError> {
        if contract.major != minimum.major {
            return Err(IdentityError::ContractMajorMismatch);
        }
        Ok(Self { contract, minimum })
    }

    pub fn exact(contract: ContractId) -> Self {
        let minimum = Version {
            major: contract.major,
            minor: 0,
            patch: 0,
        };
        Self { contract, minimum }
    }

    pub fn from_package(requirement: &PackageRequirement) -> Result<Self, IdentityError> {
        requirement
            .validate()
            .map_err(|_| IdentityError::InvalidContract)?;
        Self::new(
            ContractId::new(
                requirement.namespace.clone(),
                requirement.name.clone(),
                requirement.minimum.major,
            )?,
            requirement.minimum,
        )
    }

    pub fn contract(&self) -> &ContractId {
        &self.contract
    }

    pub fn minimum(&self) -> Version {
        self.minimum
    }

    pub fn accepts(&self, version: Version) -> bool {
        version.major == self.contract.major && version >= self.minimum
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EndpointIdentity {
    provider: InstanceId,
    facet: FacetKey,
    contract: ContractId,
}

impl EndpointIdentity {
    pub fn new(provider: InstanceId, facet: FacetKey, contract: ContractId) -> Self {
        Self {
            provider,
            facet,
            contract,
        }
    }

    pub fn provider(&self) -> &InstanceId {
        &self.provider
    }

    pub fn facet(&self) -> &FacetKey {
        &self.facet
    }

    pub fn contract(&self) -> &ContractId {
        &self.contract
    }
}

impl fmt::Display for EndpointIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}/{}/{}",
            self.provider, self.facet, self.contract
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    InvalidId(&'static str),
    Zero(&'static str),
    InvalidContract,
    ContractMajorMismatch,
}

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId(field) => write!(formatter, "invalid bounded {field}"),
            Self::Zero(field) => write!(formatter, "{field} must be non-zero"),
            Self::InvalidContract => formatter.write_str("invalid contract identity"),
            Self::ContractMajorMismatch => {
                formatter.write_str("contract requirement must use the exact contract major")
            }
        }
    }
}

impl Error for IdentityError {}
