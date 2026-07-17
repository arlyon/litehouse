//! Typed, host-owned service routing for isolated Component Model stores.
//!
//! The kernel owns endpoint metadata, binding snapshots, grant checks, and
//! generation/incarnation fences. Generated adapters keep provider resources in
//! provider stores and put only [`ProxyId`] values in consumer-store resources.
//! No guest resource handle is part of this crate's routing model.

mod authorization;
mod error;
mod fabric;
mod identity;
mod registry;
mod state;

pub use authorization::{
    AuthorizationAction, AuthorizationRequest, GrantError, GrantSet, ServiceAuthorizer,
    ServiceGrant,
};
pub use error::{
    AuthorizationError, AvailabilityError, BindingError, CancellationOrigin, CapacityError,
    CoreError, ExecutionError, OutcomeUnknownError, RetryAdvice, ServiceError, TimingError,
    UnknownOutcomeCause,
};
pub use fabric::{
    Binding, BindingId, Cardinality, ContractSpec, Dispatch, InstanceGeneration, LifecycleError,
    ProxyId, RebindPolicy, RequestError, ResolveRequest, ServiceFabric, TypedOperation, TypedProxy,
};
pub use identity::{
    ContractId, ContractRequirement, EndpointIdentity, EventId, FacetKey, FeatureId, Generation,
    GrantRevision, IdentityError, Incarnation, InstanceId, OperationId, SourceEventId,
};
pub use registry::{EndpointDescriptor, EndpointRegistry, RegistryError, Selector};
pub use state::{EventIdentity, StateIdentity};
