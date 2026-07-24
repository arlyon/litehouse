use crate::identity::{
    ContractRequirement, EndpointIdentity, FeatureId, Generation, Incarnation, InstanceId,
    OperationId,
};
use litehouse_contracts::identity::Version;
use std::{collections::BTreeMap, collections::BTreeSet, error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointDescriptor {
    identity: EndpointIdentity,
    version: Version,
    generation: Generation,
    incarnation: Incarnation,
    operations: BTreeSet<OperationId>,
    features: BTreeSet<FeatureId>,
}

impl EndpointDescriptor {
    pub fn new(
        identity: EndpointIdentity,
        version: Version,
        generation: Generation,
        incarnation: Incarnation,
        operations: impl IntoIterator<Item = OperationId>,
        features: impl IntoIterator<Item = FeatureId>,
    ) -> Result<Self, RegistryError> {
        if version.major != identity.contract().major() {
            return Err(RegistryError::ContractMajorMismatch);
        }
        let operations = operations.into_iter().collect::<BTreeSet<_>>();
        if operations.is_empty() {
            return Err(RegistryError::EmptyOperations);
        }
        Ok(Self {
            identity,
            version,
            generation,
            incarnation,
            operations,
            features: features.into_iter().collect(),
        })
    }

    pub fn identity(&self) -> &EndpointIdentity {
        &self.identity
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn generation(&self) -> Generation {
        self.generation
    }

    pub fn incarnation(&self) -> Incarnation {
        self.incarnation
    }

    pub fn operations(&self) -> &BTreeSet<OperationId> {
        &self.operations
    }

    pub fn features(&self) -> &BTreeSet<FeatureId> {
        &self.features
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    requirement: ContractRequirement,
    provider: Option<InstanceId>,
    facet: Option<crate::identity::FacetKey>,
    required_features: BTreeSet<FeatureId>,
}

impl Selector {
    pub fn new(requirement: ContractRequirement) -> Self {
        Self {
            requirement,
            provider: None,
            facet: None,
            required_features: BTreeSet::new(),
        }
    }

    pub fn provider(mut self, provider: InstanceId) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn facet(mut self, facet: crate::identity::FacetKey) -> Self {
        self.facet = Some(facet);
        self
    }

    pub fn require_feature(mut self, feature: FeatureId) -> Self {
        self.required_features.insert(feature);
        self
    }

    pub fn requirement(&self) -> &ContractRequirement {
        &self.requirement
    }

    pub fn provider_filter(&self) -> Option<&InstanceId> {
        self.provider.as_ref()
    }

    pub fn facet_filter(&self) -> Option<&crate::identity::FacetKey> {
        self.facet.as_ref()
    }

    pub fn matches(&self, endpoint: &EndpointDescriptor) -> bool {
        endpoint.identity.contract() == self.requirement.contract()
            && self.requirement.accepts(endpoint.version)
            && self
                .provider
                .as_ref()
                .is_none_or(|provider| provider == endpoint.identity.provider())
            && self
                .facet
                .as_ref()
                .is_none_or(|facet| facet == endpoint.identity.facet())
            && self
                .required_features
                .iter()
                .all(|feature| endpoint.features.contains(feature))
    }
}

/// Host-owned routing metadata. It deliberately contains no Wasmtime instance,
/// guest resource, `ResourceAny`, table index, or provider implementation handle.
#[derive(Debug, Default)]
pub struct EndpointRegistry {
    endpoints: BTreeMap<EndpointIdentity, EndpointDescriptor>,
}

impl EndpointRegistry {
    pub fn publish(&mut self, endpoint: EndpointDescriptor) -> Result<(), RegistryError> {
        if let Some(current) = self.endpoints.get(endpoint.identity()) {
            if endpoint.generation < current.generation
                || (endpoint.generation == current.generation
                    && endpoint.incarnation < current.incarnation)
            {
                return Err(RegistryError::LifecycleWentBackwards);
            }
            if endpoint.generation == current.generation
                && endpoint.incarnation == current.incarnation
                && endpoint != *current
            {
                return Err(RegistryError::ChangedWithoutIncarnation);
            }
        }
        self.endpoints.insert(endpoint.identity.clone(), endpoint);
        Ok(())
    }

    pub fn get(&self, identity: &EndpointIdentity) -> Option<&EndpointDescriptor> {
        self.endpoints.get(identity)
    }

    pub fn remove(&mut self, identity: &EndpointIdentity) -> Option<EndpointDescriptor> {
        self.endpoints.remove(identity)
    }

    pub fn remove_provider(&mut self, provider: &InstanceId) -> Vec<EndpointDescriptor> {
        let identities = self
            .endpoints
            .keys()
            .filter(|identity| identity.provider() == provider)
            .cloned()
            .collect::<Vec<_>>();
        identities
            .into_iter()
            .filter_map(|identity| self.endpoints.remove(&identity))
            .collect()
    }

    /// Results follow the canonical endpoint identity order: provider instance,
    /// facet key, then exact contract identity. Selector fields never affect it.
    pub fn resolve(&self, selector: &Selector) -> Vec<&EndpointDescriptor> {
        self.endpoints
            .values()
            .filter(|endpoint| selector.matches(endpoint))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.endpoints.len()
    }

    pub fn is_empty(&self) -> bool {
        self.endpoints.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryError {
    ContractMajorMismatch,
    EmptyOperations,
    LifecycleWentBackwards,
    ChangedWithoutIncarnation,
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContractMajorMismatch => {
                formatter.write_str("endpoint version does not match its exact contract major")
            }
            Self::EmptyOperations => formatter.write_str("endpoint must support an operation"),
            Self::LifecycleWentBackwards => {
                formatter.write_str("endpoint generation/incarnation moved backwards")
            }
            Self::ChangedWithoutIncarnation => {
                formatter.write_str("endpoint metadata changed without a new facet incarnation")
            }
        }
    }
}

impl Error for RegistryError {}
