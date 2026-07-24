use crate::{
    authorization::{
        AuthorizationAction, AuthorizationRequest, GrantSet, ServiceAuthorizer, ServiceGrant,
    },
    error::{
        AuthorizationError, BindingError, CoreError, ExecutionError, RetryAdvice, ServiceError,
    },
    identity::{
        ContractRequirement, EndpointIdentity, EventId, Generation, GrantRevision, InstanceId,
        OperationId, SourceEventId,
    },
    registry::{EndpointDescriptor, EndpointRegistry, RegistryError, Selector},
    state::{EventIdentity, StateIdentity},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
    marker::PhantomData,
};

pub trait TypedOperation: Copy + fmt::Debug + Send + Sync + 'static {
    fn name(self) -> &'static str;
}

/// Implemented by generated host adapters. The guest ABI remains fixed and
/// typed; this trait is not a reflection or string-dispatch interface exposed to
/// components.
pub trait ContractSpec: Send + Sync + 'static {
    type Operation: TypedOperation;

    fn requirement() -> ContractRequirement;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    One,
    Optional,
    Many,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RebindPolicy {
    Never,
    SameIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceGeneration {
    instance: InstanceId,
    generation: Generation,
}

impl InstanceGeneration {
    pub fn new(instance: InstanceId, generation: Generation) -> Self {
        Self {
            instance,
            generation,
        }
    }

    pub fn instance(&self) -> &InstanceId {
        &self.instance
    }

    pub fn generation(&self) -> Generation {
        self.generation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProxyId(u64);

impl ProxyId {
    pub fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BindingId(u64);

impl BindingId {
    pub fn get(self) -> u64 {
        self.0
    }
}

/// A consumer-side proxy value. Its only runtime payload is a host-owned ID;
/// exact-major typing exists solely in `PhantomData`.
#[derive(Debug, PartialEq, Eq)]
pub struct TypedProxy<C: ContractSpec> {
    id: ProxyId,
    marker: PhantomData<fn() -> C>,
}

impl<C: ContractSpec> Copy for TypedProxy<C> {}

impl<C: ContractSpec> Clone for TypedProxy<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: ContractSpec> TypedProxy<C> {
    pub fn id(self) -> ProxyId {
        self.id
    }

    fn new(id: ProxyId) -> Self {
        Self {
            id,
            marker: PhantomData,
        }
    }
}

#[derive(Debug)]
pub struct Binding<C: ContractSpec> {
    id: Option<BindingId>,
    cardinality: Cardinality,
    proxies: Vec<TypedProxy<C>>,
}

impl<C: ContractSpec> Binding<C> {
    /// Returns the retained binding ID. Empty optional/many snapshots have no
    /// retained graph state and therefore no binding ID.
    pub fn id(&self) -> Option<BindingId> {
        self.id
    }

    pub fn cardinality(&self) -> Cardinality {
        self.cardinality
    }

    pub fn proxies(&self) -> &[TypedProxy<C>] {
        &self.proxies
    }

    pub fn into_proxies(self) -> Vec<TypedProxy<C>> {
        self.proxies
    }
}

#[derive(Debug)]
pub struct ResolveRequest<C: ContractSpec> {
    consumer: InstanceGeneration,
    selector: Selector,
    operations: BTreeSet<OperationId>,
    cardinality: Cardinality,
    rebind: RebindPolicy,
    marker: PhantomData<fn() -> C>,
}

impl<C: ContractSpec> ResolveRequest<C> {
    pub fn new(
        consumer: InstanceGeneration,
        selector: Selector,
        operations: impl IntoIterator<Item = C::Operation>,
        cardinality: Cardinality,
        rebind: RebindPolicy,
    ) -> Result<Self, RequestError> {
        if selector.requirement().contract() != C::requirement().contract() {
            return Err(RequestError::ContractMismatch);
        }
        let mut operation_ids = BTreeSet::new();
        for operation in operations {
            operation_ids.insert(
                OperationId::new(operation.name()).map_err(|_| RequestError::InvalidOperation)?,
            );
        }
        if operation_ids.is_empty() {
            return Err(RequestError::EmptyOperations);
        }
        Ok(Self {
            consumer,
            selector,
            operations: operation_ids,
            cardinality,
            rebind,
            marker: PhantomData,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatch<C: ContractSpec> {
    proxy: ProxyId,
    endpoint: EndpointIdentity,
    provider_generation: Generation,
    facet_incarnation: crate::identity::Incarnation,
    operation: C::Operation,
}

impl<C: ContractSpec> Dispatch<C> {
    pub fn proxy(&self) -> ProxyId {
        self.proxy
    }

    pub fn endpoint(&self) -> &EndpointIdentity {
        &self.endpoint
    }

    pub fn provider_generation(&self) -> Generation {
        self.provider_generation
    }

    pub fn facet_incarnation(&self) -> crate::identity::Incarnation {
        self.facet_incarnation
    }

    pub fn operation(&self) -> C::Operation {
        self.operation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderStatus {
    Active,
    Trapped,
}

#[derive(Debug, Clone, Copy)]
struct ProviderSession {
    generation: Generation,
    status: ProviderStatus,
}

#[derive(Debug, Clone)]
struct ProxyRecord {
    binding: BindingId,
    consumer: InstanceGeneration,
    endpoint: EndpointIdentity,
    selector: Selector,
    provider_generation: Generation,
    facet_incarnation: crate::identity::Incarnation,
    grant_revision: GrantRevision,
    operations: BTreeSet<OperationId>,
    rebind: RebindPolicy,
}

#[derive(Debug, Clone)]
struct BindingRecord {
    consumer: InstanceId,
    provider_refcounts: BTreeMap<InstanceId, usize>,
    proxies: BTreeSet<ProxyId>,
}

/// The production service-fabric kernel. Provider implementation resources and
/// consumer resource tables live in their respective Wasmtime stores; this type
/// retains only stable metadata and host-owned IDs.
#[derive(Debug, Default)]
pub struct ServiceFabric {
    registry: EndpointRegistry,
    providers: BTreeMap<InstanceId, ProviderSession>,
    consumers: BTreeMap<InstanceId, Generation>,
    grants: BTreeMap<InstanceId, GrantSet>,
    bindings: BTreeMap<BindingId, BindingRecord>,
    proxies: BTreeMap<ProxyId, ProxyRecord>,
    authorizer: ServiceAuthorizer,
    next_binding: u64,
    next_proxy: u64,
    next_error: u64,
}

impl ServiceFabric {
    pub fn registry(&self) -> &EndpointRegistry {
        &self.registry
    }

    pub fn active_binding_count(&self) -> usize {
        self.bindings.len()
    }

    pub fn activate_consumer(
        &mut self,
        consumer: InstanceGeneration,
    ) -> Result<(), LifecycleError> {
        if self
            .consumers
            .get(consumer.instance())
            .is_some_and(|current| *current >= consumer.generation())
        {
            return Err(LifecycleError::GenerationNotIncreasing);
        }
        self.release_consumer_bindings(consumer.instance());
        self.consumers
            .insert(consumer.instance.clone(), consumer.generation);
        Ok(())
    }

    pub fn activate_provider(
        &mut self,
        provider: InstanceGeneration,
    ) -> Result<(), LifecycleError> {
        if self
            .providers
            .get(provider.instance())
            .is_some_and(|current| current.generation >= provider.generation())
        {
            return Err(LifecycleError::GenerationNotIncreasing);
        }
        self.registry.remove_provider(provider.instance());
        self.providers.insert(
            provider.instance,
            ProviderSession {
                generation: provider.generation,
                status: ProviderStatus::Active,
            },
        );
        Ok(())
    }

    pub fn publish_endpoint(&mut self, endpoint: EndpointDescriptor) -> Result<(), LifecycleError> {
        let session = self
            .providers
            .get(endpoint.identity().provider())
            .ok_or(LifecycleError::ProviderNotActive)?;
        if session.status != ProviderStatus::Active || session.generation != endpoint.generation() {
            return Err(LifecycleError::ProviderNotActive);
        }
        self.registry
            .publish(endpoint)
            .map_err(LifecycleError::Registry)
    }

    pub fn withdraw_endpoint(
        &mut self,
        withdrawing: &InstanceGeneration,
        endpoint: &EndpointIdentity,
        expected_incarnation: crate::identity::Incarnation,
    ) -> Result<bool, LifecycleError> {
        if endpoint.provider() != withdrawing.instance() {
            return Err(LifecycleError::ProviderNotActive);
        }
        let session = self
            .providers
            .get(withdrawing.instance())
            .ok_or(LifecycleError::ProviderNotActive)?;
        if session.status != ProviderStatus::Active
            || session.generation != withdrawing.generation()
        {
            return Err(LifecycleError::ProviderNotActive);
        }
        let Some(current) = self.registry.get(endpoint) else {
            return Ok(false);
        };
        if current.generation() != withdrawing.generation()
            || current.incarnation() != expected_incarnation
        {
            return Err(LifecycleError::EndpointNotCurrent);
        }
        Ok(self.registry.remove(endpoint).is_some())
    }

    pub fn trap_provider(&mut self, provider: &InstanceGeneration) -> Result<(), LifecycleError> {
        let session = self
            .providers
            .get_mut(provider.instance())
            .ok_or(LifecycleError::ProviderNotActive)?;
        if session.generation != provider.generation || session.status != ProviderStatus::Active {
            return Err(LifecycleError::ProviderNotActive);
        }
        session.status = ProviderStatus::Trapped;
        self.registry.remove_provider(provider.instance());
        Ok(())
    }

    pub fn set_grants(
        &mut self,
        consumer: &InstanceId,
        revision: GrantRevision,
        grants: Vec<ServiceGrant>,
    ) -> Result<(), LifecycleError> {
        if !self.consumers.contains_key(consumer) {
            return Err(LifecycleError::ConsumerNotActive);
        }
        if self
            .grants
            .get(consumer)
            .is_some_and(|current| current.revision() >= revision)
        {
            return Err(LifecycleError::GrantRevisionNotIncreasing);
        }
        self.grants.insert(
            consumer.clone(),
            GrantSet::new(consumer.clone(), revision, grants),
        );
        Ok(())
    }

    pub fn resolve<C: ContractSpec>(
        &mut self,
        request: ResolveRequest<C>,
    ) -> Result<Binding<C>, ServiceError> {
        self.validate_consumer(&request.consumer)?;
        let current_grant_revision = self
            .grants
            .get(request.consumer.instance())
            .map(GrantSet::revision)
            .ok_or_else(|| {
                self.fail(
                    CoreError::Authorization(AuthorizationError::Denied),
                    RetryAdvice::Never,
                )
            })?;
        let candidates = self
            .registry
            .resolve(&request.selector)
            .into_iter()
            .cloned()
            .collect::<Vec<_>>();
        let mut eligible = Vec::new();
        let mut saw_supported = false;
        let mut last_denial = AuthorizationError::Denied;
        for endpoint in &candidates {
            if !request
                .operations
                .iter()
                .all(|operation| endpoint.operations().contains(operation))
            {
                continue;
            }
            saw_supported = true;
            let mut authorized = true;
            for operation in &request.operations {
                match self.authorizer.authorize(
                    self.grants.get(request.consumer.instance()),
                    &AuthorizationRequest {
                        principal: request.consumer.instance(),
                        action: AuthorizationAction::Bind,
                        endpoint: endpoint.identity(),
                        operation,
                        grant_revision: current_grant_revision,
                    },
                ) {
                    Ok(()) => {}
                    Err(error) => {
                        authorized = false;
                        last_denial = error;
                        break;
                    }
                }
            }
            if authorized {
                eligible.push(endpoint.clone());
            }
        }

        if eligible.is_empty() && !candidates.is_empty() {
            return if saw_supported {
                Err(self.fail(CoreError::Authorization(last_denial), RetryAdvice::Never))
            } else {
                Err(self.fail(
                    CoreError::Execution(ExecutionError::Unsupported),
                    RetryAdvice::Never,
                ))
            };
        }

        match request.cardinality {
            Cardinality::One if eligible.is_empty() => {
                return Err(self.fail(
                    CoreError::Binding(BindingError::NotFound),
                    RetryAdvice::Rebind,
                ));
            }
            Cardinality::One | Cardinality::Optional if eligible.len() > 1 => {
                return Err(self.fail(
                    CoreError::Binding(BindingError::Ambiguous),
                    RetryAdvice::Never,
                ));
            }
            _ => {}
        }

        let selected = match request.cardinality {
            Cardinality::One | Cardinality::Optional => eligible.into_iter().take(1).collect(),
            Cardinality::Many => eligible,
        };
        let mut provider_refcounts = BTreeMap::new();
        for endpoint in &selected {
            *provider_refcounts
                .entry(endpoint.identity().provider().clone())
                .or_insert(0) += 1;
        }
        let providers = provider_refcounts.keys().cloned().collect::<BTreeSet<_>>();
        if self.would_create_cycle(request.consumer.instance(), &providers) {
            return Err(self.fail(
                CoreError::Execution(ExecutionError::Unsupported),
                RetryAdvice::Never,
            ));
        }

        if selected.is_empty() {
            return Ok(Binding {
                id: None,
                cardinality: request.cardinality,
                proxies: Vec::new(),
            });
        }

        let binding_id = self.allocate_binding();
        let mut proxy_ids = BTreeSet::new();
        let mut proxies = Vec::with_capacity(selected.len());
        for endpoint in selected {
            let proxy_id = self.allocate_proxy();
            self.proxies.insert(
                proxy_id,
                ProxyRecord {
                    binding: binding_id,
                    consumer: request.consumer.clone(),
                    endpoint: endpoint.identity().clone(),
                    selector: request.selector.clone(),
                    provider_generation: endpoint.generation(),
                    facet_incarnation: endpoint.incarnation(),
                    grant_revision: current_grant_revision,
                    operations: request.operations.clone(),
                    rebind: request.rebind,
                },
            );
            proxy_ids.insert(proxy_id);
            proxies.push(TypedProxy::new(proxy_id));
        }
        self.bindings.insert(
            binding_id,
            BindingRecord {
                consumer: request.consumer.instance().clone(),
                provider_refcounts,
                proxies: proxy_ids,
            },
        );
        Ok(Binding {
            id: Some(binding_id),
            cardinality: request.cardinality,
            proxies,
        })
    }

    pub fn proxy<C: ContractSpec>(&mut self, id: ProxyId) -> Result<TypedProxy<C>, ServiceError> {
        let record = self.proxies.get(&id).cloned().ok_or_else(|| {
            self.fail(
                CoreError::Binding(BindingError::StaleHandle),
                RetryAdvice::Rebind,
            )
        })?;
        if record.endpoint.contract() != C::requirement().contract() {
            return Err(self.fail(
                CoreError::Binding(BindingError::StaleHandle),
                RetryAdvice::Never,
            ));
        }
        Ok(TypedProxy::new(id))
    }

    pub fn endpoint_for_proxy<C: ContractSpec>(
        &mut self,
        proxy: TypedProxy<C>,
    ) -> Result<EndpointIdentity, ServiceError> {
        self.proxy_record::<C>(proxy)
            .map(|record| record.endpoint.clone())
    }

    pub fn prepare_call<C: ContractSpec>(
        &mut self,
        proxy: TypedProxy<C>,
        operation: C::Operation,
    ) -> Result<Dispatch<C>, ServiceError> {
        let record = self.proxy_record::<C>(proxy)?.clone();
        self.validate_consumer(&record.consumer)?;
        let operation_id = OperationId::new(operation.name()).map_err(|_| {
            self.fail(
                CoreError::Execution(ExecutionError::Unsupported),
                RetryAdvice::Never,
            )
        })?;
        if !record.operations.contains(&operation_id) {
            return Err(self.fail(
                CoreError::Authorization(AuthorizationError::InsufficientScope),
                RetryAdvice::Never,
            ));
        }

        let session = self.providers.get(record.endpoint.provider()).copied();
        match session {
            Some(session) if session.generation != record.provider_generation => {
                return Err(self.fail(
                    CoreError::Binding(BindingError::StaleHandle),
                    RetryAdvice::Rebind,
                ));
            }
            Some(session) if session.status == ProviderStatus::Trapped => {
                return Err(self.fail(
                    CoreError::Execution(ExecutionError::ProviderTrapped),
                    RetryAdvice::Reconnect,
                ));
            }
            None => {
                return Err(self.fail(
                    CoreError::Binding(BindingError::EndpointRemoved),
                    RetryAdvice::Rebind,
                ));
            }
            Some(_) => {}
        }

        let endpoint = self
            .registry
            .get(&record.endpoint)
            .cloned()
            .ok_or_else(|| {
                self.fail(
                    CoreError::Binding(BindingError::EndpointRemoved),
                    RetryAdvice::Rebind,
                )
            })?;
        if endpoint.generation() != record.provider_generation
            || endpoint.incarnation() != record.facet_incarnation
        {
            return Err(self.fail(
                CoreError::Binding(BindingError::StaleHandle),
                RetryAdvice::Rebind,
            ));
        }
        if !endpoint.operations().contains(&operation_id) {
            return Err(self.fail(
                CoreError::Execution(ExecutionError::Unsupported),
                RetryAdvice::Never,
            ));
        }
        let grants = self.grants.get(record.consumer.instance());
        if grants.map(GrantSet::revision) != Some(record.grant_revision) {
            return Err(self.fail(
                CoreError::Authorization(AuthorizationError::ExpiredGrant),
                RetryAdvice::Never,
            ));
        }
        self.authorizer
            .authorize(
                grants,
                &AuthorizationRequest {
                    principal: record.consumer.instance(),
                    action: AuthorizationAction::Invoke,
                    endpoint: &record.endpoint,
                    operation: &operation_id,
                    grant_revision: record.grant_revision,
                },
            )
            .map_err(|error| self.fail(CoreError::Authorization(error), RetryAdvice::Never))?;

        Ok(Dispatch {
            proxy: proxy.id,
            endpoint: record.endpoint,
            provider_generation: record.provider_generation,
            facet_incarnation: record.facet_incarnation,
            operation,
        })
    }

    pub fn rebind<C: ContractSpec>(
        &mut self,
        proxy: TypedProxy<C>,
    ) -> Result<TypedProxy<C>, ServiceError> {
        let old = self.proxy_record::<C>(proxy)?.clone();
        if old.rebind == RebindPolicy::Never {
            return Err(self.fail(
                CoreError::Binding(BindingError::StaleHandle),
                RetryAdvice::Never,
            ));
        }
        self.validate_consumer(&old.consumer)?;
        let current = self.registry.get(&old.endpoint).cloned().ok_or_else(|| {
            self.fail(
                CoreError::Binding(BindingError::EndpointRemoved),
                RetryAdvice::Rebind,
            )
        })?;
        if !old.selector.matches(&current) {
            return Err(self.fail(
                CoreError::Binding(BindingError::EndpointRemoved),
                RetryAdvice::Never,
            ));
        }
        if !old
            .operations
            .iter()
            .all(|operation| current.operations().contains(operation))
        {
            return Err(self.fail(
                CoreError::Execution(ExecutionError::Unsupported),
                RetryAdvice::Never,
            ));
        }
        let grant_revision = self
            .grants
            .get(old.consumer.instance())
            .map(GrantSet::revision)
            .ok_or_else(|| {
                self.fail(
                    CoreError::Authorization(AuthorizationError::Denied),
                    RetryAdvice::Never,
                )
            })?;
        for operation in &old.operations {
            self.authorizer
                .authorize(
                    self.grants.get(old.consumer.instance()),
                    &AuthorizationRequest {
                        principal: old.consumer.instance(),
                        action: AuthorizationAction::Rebind,
                        endpoint: &old.endpoint,
                        operation,
                        grant_revision,
                    },
                )
                .map_err(|error| self.fail(CoreError::Authorization(error), RetryAdvice::Never))?;
        }

        let id = self.allocate_proxy();
        self.proxies.insert(
            id,
            ProxyRecord {
                provider_generation: current.generation(),
                facet_incarnation: current.incarnation(),
                grant_revision,
                ..old.clone()
            },
        );
        if let Some(binding) = self.bindings.get_mut(&old.binding) {
            binding.proxies.insert(id);
            *binding
                .provider_refcounts
                .entry(old.endpoint.provider().clone())
                .or_insert(0) += 1;
        }
        Ok(TypedProxy::new(id))
    }

    pub fn drop_proxy(&mut self, proxy: ProxyId) -> bool {
        let Some(record) = self.proxies.remove(&proxy) else {
            return false;
        };
        let remove_binding = if let Some(binding) = self.bindings.get_mut(&record.binding) {
            binding.proxies.remove(&proxy);
            if let Some(refcount) = binding
                .provider_refcounts
                .get_mut(record.endpoint.provider())
            {
                *refcount -= 1;
                if *refcount == 0 {
                    binding
                        .provider_refcounts
                        .remove(record.endpoint.provider());
                }
            }
            binding.proxies.is_empty()
        } else {
            false
        };
        if remove_binding {
            self.bindings.remove(&record.binding);
        }
        true
    }

    pub fn release_binding(&mut self, binding: BindingId) -> bool {
        let Some(binding) = self.bindings.remove(&binding) else {
            return false;
        };
        for proxy in binding.proxies {
            self.proxies.remove(&proxy);
        }
        true
    }

    pub fn state_identity(
        &mut self,
        endpoint: &EndpointIdentity,
        revision: u64,
    ) -> Result<StateIdentity, ServiceError> {
        let descriptor = self.registry.get(endpoint).cloned().ok_or_else(|| {
            self.fail(
                CoreError::Binding(BindingError::EndpointRemoved),
                RetryAdvice::Rebind,
            )
        })?;
        Ok(StateIdentity {
            endpoint: endpoint.clone(),
            generation: descriptor.generation(),
            revision,
        })
    }

    pub fn event_identity(
        &mut self,
        endpoint: &EndpointIdentity,
        event_id: EventId,
        source_event_id: Option<SourceEventId>,
    ) -> Result<EventIdentity, ServiceError> {
        let descriptor = self.registry.get(endpoint).cloned().ok_or_else(|| {
            self.fail(
                CoreError::Binding(BindingError::EndpointRemoved),
                RetryAdvice::Rebind,
            )
        })?;
        Ok(EventIdentity {
            endpoint: endpoint.clone(),
            generation: descriptor.generation(),
            event_id,
            source_event_id,
        })
    }

    fn validate_consumer(&mut self, consumer: &InstanceGeneration) -> Result<(), ServiceError> {
        if self.consumers.get(consumer.instance()) == Some(&consumer.generation()) {
            Ok(())
        } else {
            Err(self.fail(
                CoreError::Binding(BindingError::StaleHandle),
                RetryAdvice::Never,
            ))
        }
    }

    fn proxy_record<C: ContractSpec>(
        &mut self,
        proxy: TypedProxy<C>,
    ) -> Result<&ProxyRecord, ServiceError> {
        let record = self.proxies.get(&proxy.id).cloned().ok_or_else(|| {
            self.fail(
                CoreError::Binding(BindingError::StaleHandle),
                RetryAdvice::Rebind,
            )
        })?;
        if record.endpoint.contract() != C::requirement().contract() {
            return Err(self.fail(
                CoreError::Binding(BindingError::StaleHandle),
                RetryAdvice::Never,
            ));
        }
        Ok(self
            .proxies
            .get(&proxy.id)
            .expect("proxy existence was checked"))
    }

    fn release_consumer_bindings(&mut self, consumer: &InstanceId) {
        let bindings = self
            .bindings
            .iter()
            .filter_map(|(id, binding)| (binding.consumer == *consumer).then_some(*id))
            .collect::<Vec<_>>();
        for binding in bindings {
            self.release_binding(binding);
        }
    }

    fn would_create_cycle(&self, consumer: &InstanceId, providers: &BTreeSet<InstanceId>) -> bool {
        providers
            .iter()
            .any(|provider| provider == consumer || self.path_exists(provider, consumer))
    }

    fn path_exists(&self, from: &InstanceId, target: &InstanceId) -> bool {
        let mut pending = vec![from.clone()];
        let mut visited = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if current == *target {
                return true;
            }
            if !visited.insert(current.clone()) {
                continue;
            }
            for provider in self
                .bindings
                .values()
                .filter(|binding| binding.consumer == current)
                .flat_map(|binding| binding.provider_refcounts.keys())
            {
                if !visited.contains(provider) {
                    pending.push(provider.clone());
                }
            }
        }
        false
    }

    fn allocate_binding(&mut self) -> BindingId {
        self.next_binding = self
            .next_binding
            .checked_add(1)
            .expect("binding ID exhausted");
        BindingId(self.next_binding)
    }

    fn allocate_proxy(&mut self) -> ProxyId {
        self.next_proxy = self.next_proxy.checked_add(1).expect("proxy ID exhausted");
        ProxyId(self.next_proxy)
    }

    fn fail(&mut self, inner: CoreError, retry: RetryAdvice) -> ServiceError {
        self.next_error = self.next_error.checked_add(1).expect("error ID exhausted");
        ServiceError::new(inner, retry, self.next_error)
    }
}

#[derive(Debug)]
pub enum LifecycleError {
    GenerationNotIncreasing,
    GrantRevisionNotIncreasing,
    ProviderNotActive,
    ConsumerNotActive,
    EndpointNotCurrent,
    Registry(RegistryError),
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GenerationNotIncreasing => {
                formatter.write_str("instance generation must increase")
            }
            Self::GrantRevisionNotIncreasing => formatter.write_str("grant revision must increase"),
            Self::ProviderNotActive => formatter.write_str("provider generation is not active"),
            Self::ConsumerNotActive => formatter.write_str("consumer generation is not active"),
            Self::EndpointNotCurrent => {
                formatter.write_str("endpoint generation/incarnation is not current")
            }
            Self::Registry(error) => error.fmt(formatter),
        }
    }
}

impl Error for LifecycleError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestError {
    ContractMismatch,
    InvalidOperation,
    EmptyOperations,
}

impl fmt::Display for RequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContractMismatch => {
                formatter.write_str("typed contract and selector contract do not match")
            }
            Self::InvalidOperation => formatter.write_str("invalid typed operation name"),
            Self::EmptyOperations => formatter.write_str("binding must request an operation"),
        }
    }
}

impl Error for RequestError {}
