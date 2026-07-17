use litehouse_contracts::management::{OAuthScope, ScopePolicy, management_manifest};
use litehouse_provider_host::{
    Availability as HostAvailability, CallContext, FacetKind, FacetState as HostFacetState,
    FacetValue as HostFacetValue, ProviderHandle, ProviderHostError, Quality as HostQuality,
    ReceiptConfidence as HostReceiptConfidence, UnavailabilityReason as HostUnavailabilityReason,
};
use litehouse_service_fabric as fabric;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::BTreeSet,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

const INVENTORY_OPERATION: &str = "inventory.snapshot@1";
const QUERY_OPERATION: &str = "facets.query@1";
const SET_OPERATION: &str = "capability.on-off.set@1";
const MAX_INVENTORY_FACETS: usize = 200;

static REQUEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct Principal {
    id: fabric::InstanceId,
    scopes: BTreeSet<OAuthScope>,
}

impl Principal {
    pub fn new(
        id: impl Into<String>,
        scopes: impl IntoIterator<Item = OAuthScope>,
    ) -> Result<Self, fabric::IdentityError> {
        Ok(Self {
            id: fabric::InstanceId::new(id)?,
            scopes: scopes.into_iter().collect(),
        })
    }

    pub fn proving(id: impl Into<String>) -> Result<Self, fabric::IdentityError> {
        Self::new(id, [OAuthScope::Read, OAuthScope::Operate])
    }

    pub fn id(&self) -> &fabric::InstanceId {
        &self.id
    }

    pub fn scopes(&self) -> &BTreeSet<OAuthScope> {
        &self.scopes
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryRequest {
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacetQueryRequest {
    pub facet_id: String,
    pub facet_generation: u64,
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnOffSetRequest {
    pub facet_id: String,
    pub facet_generation: u64,
    pub on: bool,
    pub timeout_ms: Option<u32>,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventorySnapshot {
    pub instances: Vec<InventoryInstance>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryInstance {
    pub instance_id: String,
    pub package: String,
    pub version: String,
    pub display_name: String,
    pub status: InstanceStatus,
    pub generation: u64,
    pub facets: Vec<InventoryFacet>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstanceStatus {
    Active,
    Disabled,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryFacet {
    pub facet_id: String,
    pub generation: u64,
    pub contract: String,
    pub label: String,
    pub available: bool,
    pub features: Vec<String>,
    pub operations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacetQueryResult {
    pub facet_id: String,
    pub generation: u64,
    pub revision: u64,
    pub availability: Availability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailability_reason: Option<UnavailabilityReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,
    pub received_at_unix_milliseconds: u64,
    pub state: FacetState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Availability {
    Available,
    Unavailable,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnavailabilityReason {
    Disconnected,
    Reconnecting,
    ProviderFailed,
    EndpointRemoved,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Quality {
    SourceReported,
    SourceAssumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FacetState {
    OnOff { current: Option<bool> },
    BooleanState { current: Option<bool> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnOffSetResult {
    pub facet_id: String,
    pub generation: u64,
    pub revision: u64,
    pub receipt_id: String,
    pub confidence: ReceiptConfidence,
    pub deduplicated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReceiptConfidence {
    AcceptedForDelivery,
    Acknowledged,
    Completed,
    ObservedCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ManagementError(Box<ManagementErrorEnvelope>);

impl std::ops::Deref for ManagementError {
    type Target = ManagementErrorEnvelope;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ManagementError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementErrorEnvelope {
    pub operation_id: String,
    pub inner: ErrorCore,
    pub retry: Retry,
    pub safe_message: String,
    pub error_id: String,
    pub correlation_id: String,
    pub outcome: ErrorOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorCore {
    pub group: ErrorGroup,
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<ErrorOrigin>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorGroup {
    Binding,
    Authorization,
    Availability,
    Timing,
    Capacity,
    Execution,
    OutcomeUnknown,
    Contract,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    NotFound,
    Ambiguous,
    StaleHandle,
    EndpointRemoved,
    Denied,
    ExpiredGrant,
    InsufficientScope,
    Unavailable,
    Reconnecting,
    ShuttingDown,
    DeadlineExceeded,
    Cancelled,
    Overloaded,
    ProviderFailed,
    ProtocolFailed,
    Unsupported,
    OutcomeUnknown,
    ContractFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorOrigin {
    Caller,
    Provider,
    Host,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retry {
    pub kind: RetryKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RetryKind {
    Never,
    Retry,
    Reconnect,
    Rebind,
    LookupResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorOutcome {
    pub confidence: OutcomeConfidence,
    pub lookup_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutcomeConfidence {
    Definite,
    OutcomeUnknown,
}

#[derive(Clone)]
pub struct ManagementService {
    provider: ProviderHandle,
}

impl ManagementService {
    pub fn for_esphome(provider: ProviderHandle) -> Self {
        Self { provider }
    }

    pub async fn inventory_snapshot(
        &self,
        principal: &Principal,
        request: InventoryRequest,
    ) -> Result<InventorySnapshot, ManagementError> {
        self.authorize(principal, INVENTORY_OPERATION)?;
        operation_timeout(INVENTORY_OPERATION, request.timeout_ms)?;

        let running = self.provider.is_running();
        let descriptors = self.provider.descriptors();
        let truncated = descriptors.len() > MAX_INVENTORY_FACETS;
        let facets: Vec<InventoryFacet> = descriptors
            .iter()
            .take(MAX_INVENTORY_FACETS)
            .map(|descriptor| {
                let endpoint = descriptor.endpoint();
                let identity = endpoint.identity();
                InventoryFacet {
                    facet_id: identity.facet().as_str().to_owned(),
                    generation: endpoint.generation().get(),
                    contract: format!(
                        "{}@{}",
                        identity.contract().name(),
                        identity.contract().major()
                    ),
                    label: descriptor
                        .label()
                        .unwrap_or_else(|| identity.facet().as_str())
                        .to_owned(),
                    available: running,
                    features: endpoint
                        .features()
                        .iter()
                        .map(|feature| feature.as_str().to_owned())
                        .collect(),
                    operations: canonical_operations(descriptor.kind()),
                }
            })
            .collect();
        let provider = self.provider.provider();
        let instance_id = provider.instance().as_str().to_owned();
        tracing::info!(
            operation_id = INVENTORY_OPERATION,
            principal = %principal.id(),
            facet_count = facets.len(),
            truncated,
            "management operation completed"
        );
        Ok(InventorySnapshot {
            instances: vec![InventoryInstance {
                instance_id: instance_id.clone(),
                package: "esphome".into(),
                version: "0.1.0".into(),
                display_name: instance_id,
                status: if running {
                    InstanceStatus::Active
                } else {
                    InstanceStatus::Unavailable
                },
                generation: provider.generation().get(),
                facets,
            }],
            truncated,
        })
    }

    pub async fn facet_query(
        &self,
        principal: &Principal,
        request: FacetQueryRequest,
    ) -> Result<FacetQueryResult, ManagementError> {
        self.authorize(principal, QUERY_OPERATION)?;
        validate_facet_input(QUERY_OPERATION, &request.facet_id, request.facet_generation)?;
        let timeout = operation_timeout(QUERY_OPERATION, request.timeout_ms)?;
        let endpoint = self.resolve(QUERY_OPERATION, &request.facet_id)?;
        let generation = generation(QUERY_OPERATION, request.facet_generation)?;
        let context = call_context(timeout, None);
        let state = self
            .provider
            .query(&endpoint, generation, context)
            .await
            .map_err(|error| provider_error(QUERY_OPERATION, error))?;
        tracing::info!(
            operation_id = QUERY_OPERATION,
            principal = %principal.id(),
            facet_id = %request.facet_id,
            "management operation completed"
        );
        Ok(query_result(request.facet_id, state))
    }

    pub async fn on_off_set(
        &self,
        principal: &Principal,
        request: OnOffSetRequest,
    ) -> Result<OnOffSetResult, ManagementError> {
        self.authorize(principal, SET_OPERATION)?;
        validate_facet_input(SET_OPERATION, &request.facet_id, request.facet_generation)?;
        if !valid_idempotency_key(&request.idempotency_key) {
            return Err(caller_error(
                SET_OPERATION,
                ErrorCode::ContractFailed,
                "idempotency_key must be 32 lowercase hexadecimal characters",
            ));
        }
        let timeout = operation_timeout(SET_OPERATION, request.timeout_ms)?;
        let endpoint = self.resolve(SET_OPERATION, &request.facet_id)?;
        if endpoint.contract().name() != "on-off" {
            return Err(caller_error(
                SET_OPERATION,
                ErrorCode::Unsupported,
                "facet does not support absolute On/Off set",
            ));
        }
        let generation = generation(SET_OPERATION, request.facet_generation)?;
        let context = call_context(timeout, Some(request.idempotency_key));
        let receipt = self
            .provider
            .set_on_off(&endpoint, generation, request.on, context)
            .await
            .map_err(|error| provider_error(SET_OPERATION, error))?;
        tracing::info!(
            operation_id = SET_OPERATION,
            principal = %principal.id(),
            facet_id = %request.facet_id,
            confidence = ?receipt.confidence,
            deduplicated = receipt.deduplicated,
            "management operation completed"
        );
        Ok(OnOffSetResult {
            facet_id: request.facet_id,
            generation: receipt.provider_generation.get(),
            revision: 0,
            receipt_id: receipt.request_id,
            confidence: match receipt.confidence {
                HostReceiptConfidence::AcceptedForDelivery => {
                    ReceiptConfidence::AcceptedForDelivery
                }
                HostReceiptConfidence::ProtocolOrTargetAcknowledged => {
                    ReceiptConfidence::Acknowledged
                }
                HostReceiptConfidence::Completed => ReceiptConfidence::Completed,
                HostReceiptConfidence::ObservedCompleted => ReceiptConfidence::ObservedCompleted,
            },
            deduplicated: receipt.deduplicated,
        })
    }

    fn authorize(
        &self,
        principal: &Principal,
        operation_id: &'static str,
    ) -> Result<(), ManagementError> {
        let operation = management_manifest()
            .ok()
            .and_then(|manifest| {
                manifest
                    .operations
                    .iter()
                    .find(|item| item.id == operation_id)
            })
            .ok_or_else(|| {
                caller_error(
                    operation_id,
                    ErrorCode::ContractFailed,
                    "management operation contract is unavailable",
                )
            })?;
        let ScopePolicy::Fixed { scope } = operation.scope_policy else {
            return Err(caller_error(
                operation_id,
                ErrorCode::ContractFailed,
                "management operation does not use a fixed scope",
            ));
        };
        if principal.scopes.contains(&scope) {
            tracing::debug!(
                operation_id,
                principal = %principal.id(),
                "management authorization allowed"
            );
            Ok(())
        } else {
            tracing::warn!(
                operation_id,
                principal = %principal.id(),
                "management authorization denied"
            );
            Err(management_error(
                operation_id,
                ErrorGroup::Authorization,
                ErrorCode::InsufficientScope,
                Some(ErrorOrigin::Caller),
                RetryKind::Never,
                "principal lacks the required fixed scope",
                OutcomeConfidence::Definite,
            ))
        }
    }

    fn resolve(
        &self,
        operation_id: &'static str,
        facet_id: &str,
    ) -> Result<fabric::EndpointIdentity, ManagementError> {
        self.provider
            .descriptors()
            .iter()
            .find(|descriptor| descriptor.endpoint().identity().facet().as_str() == facet_id)
            .map(|descriptor| descriptor.endpoint().identity().clone())
            .ok_or_else(|| {
                management_error(
                    operation_id,
                    ErrorGroup::Binding,
                    ErrorCode::NotFound,
                    Some(ErrorOrigin::Caller),
                    RetryKind::Never,
                    "facet was not found",
                    OutcomeConfidence::Definite,
                )
            })
    }
}

#[derive(Clone)]
pub struct McpAdapter {
    management: ManagementService,
    principal: Principal,
    tool_router: ToolRouter<Self>,
}

impl McpAdapter {
    pub fn new(management: ManagementService, principal: Principal) -> Self {
        Self {
            management,
            principal,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl McpAdapter {
    #[tool(
        name = "litehouse_inventory",
        description = "List the bounded Litehouse provider and facet inventory.",
        input_schema = mcp_input_schema(INVENTORY_OPERATION),
        output_schema = canonical_output_schema(INVENTORY_OPERATION)
    )]
    async fn inventory(&self, Parameters(request): Parameters<InventoryRequest>) -> CallToolResult {
        tool_result(
            self.management
                .inventory_snapshot(&self.principal, request)
                .await,
        )
    }

    #[tool(
        name = "litehouse_query",
        description = "Query one typed Litehouse facet by ID and generation.",
        input_schema = mcp_input_schema(QUERY_OPERATION),
        output_schema = canonical_output_schema(QUERY_OPERATION)
    )]
    async fn query(&self, Parameters(request): Parameters<FacetQueryRequest>) -> CallToolResult {
        tool_result(self.management.facet_query(&self.principal, request).await)
    }

    #[tool(
        name = "litehouse_interact",
        description = "Set one On/Off facet to an absolute state.",
        input_schema = mcp_input_schema(SET_OPERATION),
        output_schema = canonical_output_schema(SET_OPERATION)
    )]
    async fn interact(&self, Parameters(request): Parameters<OnOffSetRequest>) -> CallToolResult {
        tool_result(self.management.on_off_set(&self.principal, request).await)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpAdapter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Use only the three closed Litehouse proving tools.")
    }
}

fn mcp_input_schema(operation_id: &str) -> Arc<Map<String, Value>> {
    operation_schema(operation_id, true)
}

fn canonical_output_schema(operation_id: &str) -> Arc<Map<String, Value>> {
    operation_schema(operation_id, false)
}

fn operation_schema(operation_id: &str, mcp_input: bool) -> Arc<Map<String, Value>> {
    let manifest = management_manifest().expect("embedded management manifest is validated");
    let operation = manifest
        .operations
        .iter()
        .find(|operation| operation.id == operation_id)
        .expect("proving operation is present in the embedded manifest");
    let value = if mcp_input {
        operation.schemas.mcp_projection.schema.clone()
    } else {
        operation.schemas.output.schema.clone()
    };
    let Value::Object(object) = value else {
        panic!("proving operation schema must be an object")
    };
    Arc::new(object)
}

fn tool_result<T: Serialize>(result: Result<T, ManagementError>) -> CallToolResult {
    match result {
        Ok(value) => CallToolResult::structured(
            serde_json::to_value(value).expect("management results are serializable"),
        ),
        Err(error) => CallToolResult::structured_error(
            serde_json::to_value(error).expect("management errors are serializable"),
        ),
    }
}

fn canonical_operations(kind: FacetKind) -> Vec<String> {
    match kind {
        FacetKind::OnOff => vec![QUERY_OPERATION.into(), SET_OPERATION.into()],
        FacetKind::BooleanState => vec![QUERY_OPERATION.into()],
    }
}

fn query_result(facet_id: String, state: HostFacetState) -> FacetQueryResult {
    let (availability, unavailability_reason) = match state.availability {
        HostAvailability::Available => (Availability::Available, None),
        HostAvailability::Unavailable(reason) => (
            Availability::Unavailable,
            Some(match reason {
                HostUnavailabilityReason::Disconnected => UnavailabilityReason::Disconnected,
                HostUnavailabilityReason::Reconnecting => UnavailabilityReason::Reconnecting,
                HostUnavailabilityReason::ProviderFailed => UnavailabilityReason::ProviderFailed,
                HostUnavailabilityReason::EndpointRemoved => UnavailabilityReason::EndpointRemoved,
                HostUnavailabilityReason::Unknown => UnavailabilityReason::Unknown,
            }),
        ),
        HostAvailability::Stale { .. } => (Availability::Stale, None),
    };
    FacetQueryResult {
        facet_id,
        generation: state.generation.get(),
        revision: state.revision,
        availability,
        unavailability_reason,
        quality: state.quality.map(|quality| match quality {
            HostQuality::SourceReported => Quality::SourceReported,
            HostQuality::SourceAssumed => Quality::SourceAssumed,
        }),
        received_at_unix_milliseconds: state.received_at_unix_milliseconds,
        state: match state.value {
            HostFacetValue::OnOff(current) => FacetState::OnOff { current },
            HostFacetValue::BooleanState(current) => FacetState::BooleanState { current },
        },
    }
}

fn validate_facet_input(
    operation_id: &'static str,
    facet_id: &str,
    facet_generation: u64,
) -> Result<(), ManagementError> {
    if !valid_public_id(facet_id) {
        return Err(caller_error(
            operation_id,
            ErrorCode::ContractFailed,
            "facet_id is not a canonical bounded identifier",
        ));
    }
    if facet_generation == 0 {
        return Err(caller_error(
            operation_id,
            ErrorCode::ContractFailed,
            "facet_generation must be non-zero",
        ));
    }
    Ok(())
}

fn valid_public_id(value: &str) -> bool {
    value.len() <= 128
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn valid_idempotency_key(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn operation_timeout(
    operation_id: &'static str,
    requested: Option<u32>,
) -> Result<Duration, ManagementError> {
    let deadline = management_manifest()
        .ok()
        .and_then(|manifest| {
            manifest
                .operations
                .iter()
                .find(|operation| operation.id == operation_id)
        })
        .map(|operation| operation.deadline)
        .ok_or_else(|| {
            caller_error(
                operation_id,
                ErrorCode::ContractFailed,
                "management operation contract is unavailable",
            )
        })?;
    let milliseconds = requested.unwrap_or(deadline.default_ms);
    if milliseconds == 0 || milliseconds > deadline.maximum_ms {
        return Err(caller_error(
            operation_id,
            ErrorCode::ContractFailed,
            "timeout_ms is outside the canonical operation deadline",
        ));
    }
    Ok(Duration::from_millis(milliseconds.into()))
}

fn generation(
    operation_id: &'static str,
    value: u64,
) -> Result<fabric::Generation, ManagementError> {
    fabric::Generation::new(value).map_err(|_| {
        caller_error(
            operation_id,
            ErrorCode::ContractFailed,
            "facet_generation must be non-zero",
        )
    })
}

fn call_context(timeout: Duration, idempotency_key: Option<String>) -> CallContext {
    call_context_at(tokio::time::Instant::now() + timeout, idempotency_key)
}

fn call_context_at(deadline: tokio::time::Instant, idempotency_key: Option<String>) -> CallContext {
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let request_id = format!("management-{sequence:016x}");
    CallContext {
        correlation_id: Some(request_id.clone()),
        request_id,
        deadline,
        idempotency_key,
        attempt: 1,
    }
}

fn caller_error(
    operation_id: &'static str,
    code: ErrorCode,
    message: &'static str,
) -> ManagementError {
    let group = if code == ErrorCode::Unsupported {
        ErrorGroup::Execution
    } else {
        ErrorGroup::Contract
    };
    management_error(
        operation_id,
        group,
        code,
        Some(ErrorOrigin::Caller),
        RetryKind::Never,
        message,
        OutcomeConfidence::Definite,
    )
}

fn provider_error(operation_id: &'static str, error: ProviderHostError) -> ManagementError {
    match error {
        ProviderHostError::Service(error) => service_error(operation_id, error),
        ProviderHostError::ActorStopped | ProviderHostError::FabricUnavailable => management_error(
            operation_id,
            ErrorGroup::Availability,
            ErrorCode::Unavailable,
            Some(ErrorOrigin::Host),
            RetryKind::Reconnect,
            "provider is unavailable",
            OutcomeConfidence::Definite,
        ),
        ProviderHostError::TooManyFacets => management_error(
            operation_id,
            ErrorGroup::Capacity,
            ErrorCode::Overloaded,
            Some(ErrorOrigin::Provider),
            RetryKind::Never,
            "provider exceeded a bounded host limit",
            OutcomeConfidence::Definite,
        ),
        ProviderHostError::ComponentLinkFailed
        | ProviderHostError::ComponentInstantiationFailed
        | ProviderHostError::ProviderStartFailed
        | ProviderHostError::InvalidConfiguration
        | ProviderHostError::InvalidDescriptor
        | ProviderHostError::DuplicateFacet
        | ProviderHostError::FabricRejected => management_error(
            operation_id,
            ErrorGroup::Execution,
            ErrorCode::ProviderFailed,
            Some(ErrorOrigin::Host),
            RetryKind::Never,
            "provider execution failed",
            OutcomeConfidence::Definite,
        ),
    }
}

fn service_error(operation_id: &'static str, error: fabric::ServiceError) -> ManagementError {
    use fabric::{
        AuthorizationError, AvailabilityError, BindingError, CapacityError, CoreError,
        ExecutionError, RetryAdvice, TimingError, UnknownOutcomeCause,
    };
    let (group, code, origin, confidence) = match error.inner {
        CoreError::Binding(value) => (
            ErrorGroup::Binding,
            match value {
                BindingError::NotFound => ErrorCode::NotFound,
                BindingError::Ambiguous => ErrorCode::Ambiguous,
                BindingError::StaleHandle => ErrorCode::StaleHandle,
                BindingError::EndpointRemoved => ErrorCode::EndpointRemoved,
            },
            Some(ErrorOrigin::Caller),
            OutcomeConfidence::Definite,
        ),
        CoreError::Authorization(value) => (
            ErrorGroup::Authorization,
            match value {
                AuthorizationError::Denied => ErrorCode::Denied,
                AuthorizationError::ExpiredGrant => ErrorCode::ExpiredGrant,
                AuthorizationError::InsufficientScope => ErrorCode::InsufficientScope,
            },
            Some(ErrorOrigin::Caller),
            OutcomeConfidence::Definite,
        ),
        CoreError::Availability(value) => (
            ErrorGroup::Availability,
            match value {
                AvailabilityError::Unavailable => ErrorCode::Unavailable,
                AvailabilityError::Reconnecting => ErrorCode::Reconnecting,
                AvailabilityError::ShuttingDown => ErrorCode::ShuttingDown,
            },
            Some(ErrorOrigin::Provider),
            OutcomeConfidence::Definite,
        ),
        CoreError::Timing(value) => (
            ErrorGroup::Timing,
            match value {
                TimingError::DeadlineExceeded { .. } => ErrorCode::DeadlineExceeded,
                TimingError::Cancelled(_) => ErrorCode::Cancelled,
            },
            match value {
                TimingError::Cancelled(origin) => Some(error_origin(origin)),
                TimingError::DeadlineExceeded { .. } => Some(ErrorOrigin::Host),
            },
            OutcomeConfidence::Definite,
        ),
        CoreError::Capacity(CapacityError::Overloaded { .. }) => (
            ErrorGroup::Capacity,
            ErrorCode::Overloaded,
            Some(ErrorOrigin::Host),
            OutcomeConfidence::Definite,
        ),
        CoreError::Execution(value) => (
            ErrorGroup::Execution,
            match value {
                ExecutionError::ProviderTrapped => ErrorCode::ProviderFailed,
                ExecutionError::ProtocolFailed => ErrorCode::ProtocolFailed,
                ExecutionError::Unsupported => ErrorCode::Unsupported,
            },
            Some(ErrorOrigin::Provider),
            OutcomeConfidence::Definite,
        ),
        CoreError::OutcomeUnknown(value) => (
            ErrorGroup::OutcomeUnknown,
            ErrorCode::OutcomeUnknown,
            Some(match value.cause {
                UnknownOutcomeCause::Cancelled(origin) => error_origin(origin),
                UnknownOutcomeCause::DeadlineExceeded => ErrorOrigin::Host,
                UnknownOutcomeCause::Disconnected | UnknownOutcomeCause::ProviderFailed => {
                    ErrorOrigin::Provider
                }
            }),
            OutcomeConfidence::OutcomeUnknown,
        ),
        CoreError::ContractError => (
            ErrorGroup::Contract,
            ErrorCode::ContractFailed,
            Some(ErrorOrigin::Provider),
            OutcomeConfidence::Definite,
        ),
    };
    let (retry, retry_after_ms) = match error.retry {
        RetryAdvice::Never => (RetryKind::Never, None),
        RetryAdvice::Reconnect => (RetryKind::Reconnect, None),
        RetryAdvice::Rebind => (RetryKind::Rebind, None),
        RetryAdvice::RetryAfter(value) => (RetryKind::Retry, value),
    };
    let mut projected = management_error(
        operation_id,
        group,
        code,
        origin,
        retry,
        "provider operation failed",
        confidence,
    );
    projected.retry.retry_after_ms = retry_after_ms;
    projected
}

fn error_origin(origin: fabric::CancellationOrigin) -> ErrorOrigin {
    match origin {
        fabric::CancellationOrigin::Caller => ErrorOrigin::Caller,
        fabric::CancellationOrigin::Provider => ErrorOrigin::Provider,
        fabric::CancellationOrigin::Host => ErrorOrigin::Host,
        fabric::CancellationOrigin::Shutdown => ErrorOrigin::Shutdown,
    }
}

fn management_error(
    operation_id: &'static str,
    group: ErrorGroup,
    code: ErrorCode,
    origin: Option<ErrorOrigin>,
    retry: RetryKind,
    safe_message: &'static str,
    confidence: OutcomeConfidence,
) -> ManagementError {
    let sequence = REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    ManagementError(Box::new(ManagementErrorEnvelope {
        operation_id: operation_id.into(),
        inner: ErrorCore {
            group,
            code,
            origin,
        },
        retry: Retry {
            kind: retry,
            retry_after_ms: None,
        },
        safe_message: safe_message.into(),
        error_id: format!("management-error-{sequence:016x}"),
        correlation_id: format!("management-{sequence:016x}"),
        outcome: ErrorOutcome {
            confidence,
            lookup_available: false,
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proving_principal_has_only_read_and_operate() {
        let principal = Principal::proving("mcp-proof").unwrap();
        assert_eq!(
            principal.scopes,
            BTreeSet::from([OAuthScope::Read, OAuthScope::Operate])
        );
    }

    #[test]
    fn adapter_inputs_are_closed_to_canonical_bounds() {
        assert!(valid_public_id("switch-1-42"));
        assert!(!valid_public_id("switch/1"));
        assert!(!valid_public_id(""));
        assert!(valid_idempotency_key("0123456789abcdef0123456789abcdef"));
        assert!(!valid_idempotency_key("0123456789ABCDEF0123456789ABCDEF"));
        assert_eq!(
            operation_timeout(QUERY_OPERATION, None).unwrap(),
            Duration::from_secs(10)
        );
        assert_eq!(
            operation_timeout(INVENTORY_OPERATION, None).unwrap(),
            Duration::from_secs(10)
        );
        assert_eq!(
            operation_timeout(SET_OPERATION, None).unwrap(),
            Duration::from_secs(30)
        );
        assert!(operation_timeout(QUERY_OPERATION, Some(30_000)).is_ok());
        assert!(operation_timeout(QUERY_OPERATION, Some(0)).is_err());
        assert!(operation_timeout(QUERY_OPERATION, Some(30_001)).is_err());
    }

    #[test]
    fn rmcp_metadata_uses_the_frozen_projection_and_output_schemas() {
        let manifest = management_manifest().unwrap();
        for operation_id in [INVENTORY_OPERATION, QUERY_OPERATION, SET_OPERATION] {
            let operation = manifest
                .operations
                .iter()
                .find(|operation| operation.id == operation_id)
                .unwrap();
            assert_eq!(
                mcp_input_schema(operation_id).as_ref(),
                operation.schemas.mcp_projection.schema.as_object().unwrap()
            );
            assert_eq!(
                canonical_output_schema(operation_id).as_ref(),
                operation.schemas.output.schema.as_object().unwrap()
            );
        }
    }

    #[test]
    fn structured_errors_are_safe_closed_envelopes() {
        let error = caller_error(
            QUERY_OPERATION,
            ErrorCode::ContractFailed,
            "invalid caller input",
        );
        let value = serde_json::to_value(error).unwrap();
        let object = value.as_object().unwrap();
        assert_eq!(object.len(), 7);
        assert_eq!(object["operation_id"], QUERY_OPERATION);
        assert_eq!(object["inner"]["origin"], "caller");
        assert!(object.get("detail").is_none());
    }
}
