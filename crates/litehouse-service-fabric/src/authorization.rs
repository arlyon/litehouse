use crate::{
    error::AuthorizationError,
    identity::{ContractId, EndpointIdentity, FacetKey, GrantRevision, InstanceId, OperationId},
};
use std::{collections::BTreeSet, error::Error, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceGrant {
    contract: ContractId,
    provider: Option<InstanceId>,
    facet: Option<FacetKey>,
    actions: BTreeSet<AuthorizationAction>,
    operations: BTreeSet<OperationId>,
}

impl ServiceGrant {
    pub fn new(
        contract: ContractId,
        provider: Option<InstanceId>,
        facet: Option<FacetKey>,
        actions: impl IntoIterator<Item = AuthorizationAction>,
        operations: impl IntoIterator<Item = OperationId>,
    ) -> Result<Self, GrantError> {
        let actions = actions.into_iter().collect::<BTreeSet<_>>();
        if actions.is_empty() {
            return Err(GrantError::EmptyActions);
        }
        let operations = operations.into_iter().collect::<BTreeSet<_>>();
        if operations.is_empty() {
            return Err(GrantError::EmptyOperations);
        }
        Ok(Self {
            contract,
            provider,
            facet,
            actions,
            operations,
        })
    }

    pub fn contract(&self) -> &ContractId {
        &self.contract
    }

    pub fn provider(&self) -> Option<&InstanceId> {
        self.provider.as_ref()
    }

    pub fn facet(&self) -> Option<&FacetKey> {
        self.facet.as_ref()
    }

    pub fn actions(&self) -> &BTreeSet<AuthorizationAction> {
        &self.actions
    }

    pub fn operations(&self) -> &BTreeSet<OperationId> {
        &self.operations
    }

    pub fn is_wildcard(&self) -> bool {
        self.provider.is_none() || self.facet.is_none()
    }

    fn matches_resource(&self, endpoint: &EndpointIdentity) -> bool {
        self.contract == *endpoint.contract()
            && self
                .provider
                .as_ref()
                .is_none_or(|provider| provider == endpoint.provider())
            && self
                .facet
                .as_ref()
                .is_none_or(|facet| facet == endpoint.facet())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantSet {
    principal: InstanceId,
    revision: GrantRevision,
    grants: Vec<ServiceGrant>,
}

impl GrantSet {
    pub fn new(principal: InstanceId, revision: GrantRevision, grants: Vec<ServiceGrant>) -> Self {
        Self {
            principal,
            revision,
            grants,
        }
    }

    pub fn principal(&self) -> &InstanceId {
        &self.principal
    }

    pub fn revision(&self) -> GrantRevision {
        self.revision
    }

    pub fn grants(&self) -> &[ServiceGrant] {
        &self.grants
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorizationAction {
    Bind,
    Rebind,
    Invoke,
}

impl AuthorizationAction {
    pub const ALL: [Self; 3] = [Self::Bind, Self::Rebind, Self::Invoke];
}

#[derive(Debug, Clone, Copy)]
pub struct AuthorizationRequest<'a> {
    pub principal: &'a InstanceId,
    pub action: AuthorizationAction,
    pub endpoint: &'a EndpointIdentity,
    pub operation: &'a OperationId,
    pub grant_revision: GrantRevision,
}

/// The single fixed service-authorization seam. It is deliberately a concrete,
/// default-deny matcher rather than a policy language or caller-supplied hook.
#[derive(Debug, Default)]
pub struct ServiceAuthorizer;

impl ServiceAuthorizer {
    pub fn authorize(
        &self,
        grants: Option<&GrantSet>,
        request: &AuthorizationRequest<'_>,
    ) -> Result<(), AuthorizationError> {
        let grants = grants.ok_or(AuthorizationError::Denied)?;
        if grants.principal != *request.principal {
            return Err(AuthorizationError::Denied);
        }
        if grants.revision != request.grant_revision {
            return Err(AuthorizationError::ExpiredGrant);
        }

        let mut resource_match = false;
        for grant in &grants.grants {
            if !grant.actions.contains(&request.action) || !grant.matches_resource(request.endpoint)
            {
                continue;
            }
            resource_match = true;
            if grant.operations.contains(request.operation) {
                return Ok(());
            }
        }
        if resource_match {
            Err(AuthorizationError::InsufficientScope)
        } else {
            Err(AuthorizationError::Denied)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantError {
    EmptyActions,
    EmptyOperations,
}

impl fmt::Display for GrantError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyActions => formatter.write_str("service grant must name an action"),
            Self::EmptyOperations => formatter.write_str("service grant must name an operation"),
        }
    }
}

impl Error for GrantError {}
