mod interaction;
mod workflow_files;

pub use interaction::{
    ColorDirection, InteractionColor, InteractionColorSpace, InteractionCoveringAxis,
    InteractionNotificationPriority, InteractionRequest, InteractionTarget,
    InteractionThermostatMode, LevelDirection,
};
pub use workflow_files::{
    LoadedWorkflow, WorkflowDirectory, WorkflowFileFormat, WorkflowFilesError,
    WorkflowMutationError, WorkflowRepository, WorkflowValidation,
};

use litehouse_contracts::management::{
    CAPABILITY_OPERATION_IDS, OAuthScope, ScopePolicy, management_manifest,
};
use litehouse_provider_host::{
    Availability as HostAvailability, CallContext, ColorModel as HostColorModel, ColorMoveRequest,
    ColorSetRequest, ColorStepRequest, ColorValue as HostColorValue, CoupledLevelMoveRequest,
    CoupledLevelRequest, CoupledLevelStepRequest, CoveringAxis as HostCoveringAxis,
    CoveringMovement as HostCoveringMovement, FacetKind, FacetState as HostFacetState,
    FacetValue as HostFacetValue, KnownThermostatMode as HostKnownThermostatMode, LevelMoveRequest,
    LevelSetRequest, LevelStepRequest, ProviderHandle, ProviderHostError, Quality as HostQuality,
    Quantity as HostQuantity, ReceiptConfidence as HostReceiptConfidence,
    StepDirection as HostStepDirection, ThermostatAction as HostThermostatAction,
    ThermostatMode as HostThermostatMode, ThermostatTargets as HostThermostatTargets,
    Transition as HostTransition, UnavailabilityReason as HostUnavailabilityReason,
};
use litehouse_service_fabric as fabric;
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
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
const WORKFLOW_OPERATION: &str = "workflows.manage@1";
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkflowRequest {
    List,
    Get {
        #[schemars(regex(pattern = "^[0-9a-f]{32}$"))]
        rule_id: String,
    },
    Validate {
        #[schemars(length(min = 1, max = 512))]
        relative_path: String,
        #[schemars(length(max = 131_072))]
        source: String,
    },
    Put {
        #[schemars(length(min = 1, max = 512))]
        relative_path: String,
        #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
        expected_sha256: Option<String>,
        #[schemars(length(max = 131_072))]
        source: String,
    },
    SetEnabled {
        #[schemars(length(min = 1, max = 512))]
        relative_path: String,
        #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
        expected_sha256: String,
        enabled: bool,
    },
    Delete {
        #[schemars(length(min = 1, max = 512))]
        relative_path: String,
        #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
        expected_sha256: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSummary {
    pub rule_id: String,
    pub relative_path: String,
    pub revision: u64,
    pub enabled: bool,
    pub source_sha256: String,
    pub semantic_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowToolResult {
    pub authored_sha256: String,
    pub loaded_sha256: String,
    pub restart_required: bool,
    pub workflows: Vec<WorkflowSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<WorkflowValidationResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowValidationResult {
    pub rule_id: String,
    pub revision: u64,
    pub enabled: bool,
    pub semantic_sha256: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FacetState {
    OnOff {
        current: Option<bool>,
    },
    BooleanState {
        current: Option<bool>,
    },
    Measurement {
        current: Option<MeasurementStateValue>,
    },
    Level {
        current: Option<QuantityValue>,
        target: Option<QuantityValue>,
        transition_remaining_milliseconds: Option<u64>,
    },
    Color {
        current: Option<ColorValue>,
        target: Option<ColorValue>,
        active_model: Option<ColorModel>,
    },
    Thermostat {
        current_mode: Option<ThermostatMode>,
        target_mode: Option<ThermostatMode>,
        current_action: Option<ThermostatAction>,
        running: Option<bool>,
        targets: Option<ThermostatTargets>,
        fan: Option<String>,
        swing: Option<String>,
        preset: Option<String>,
    },
    WindowCovering {
        axes: Vec<CoveringAxisState>,
        safety_status: Vec<String>,
        fault_status: Vec<String>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuantityValue {
    pub profile: String,
    pub magnitude: String,
    pub unit: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "kebab-case")]
pub enum ColorValue {
    HueSaturation {
        hue_degrees: String,
        saturation_ratio: String,
    },
    CieXy {
        x: String,
        y: String,
    },
    ColorTemperature {
        kelvin: String,
    },
    CalibratedRgb {
        red: String,
        green: String,
        blue: String,
        color_space: String,
    },
    UncalibratedRgb {
        red: String,
        green: String,
        blue: String,
    },
    NativeWhiteChannels {
        white: Option<String>,
        cold_white: Option<String>,
        warm_white: Option<String>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorModel {
    HueSaturation,
    CieXy,
    ColorTemperature,
    CalibratedRgb,
    UncalibratedRgb,
    NativeWhiteChannels,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum ThermostatMode {
    Off,
    Auto,
    Heat,
    Cool,
    Dry,
    FanOnly,
    ProviderQualified(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThermostatAction {
    Idle,
    Heating,
    Cooling,
    Drying,
    Fan,
    Defrosting,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ThermostatTargets {
    Single {
        temperature: QuantityValue,
    },
    HeatCool {
        heat: QuantityValue,
        cool: QuantityValue,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoveringAxisState {
    pub axis: CoveringAxis,
    pub current: Option<QuantityValue>,
    pub target: Option<QuantityValue>,
    pub movement: CoveringMovement,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoveringAxis {
    Lift,
    Tilt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoveringMovement {
    Opening,
    Closing,
    Stopped,
    Idle,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MeasurementStateValue {
    pub profile: String,
    pub magnitude: String,
    pub unit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_magnitude: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractionResult {
    pub facet_id: String,
    pub generation: u64,
    pub revision: u64,
    pub receipt_id: String,
    pub confidence: ReceiptConfidence,
    pub deduplicated: bool,
}

pub type OnOffSetResult = InteractionResult;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BundledProvider {
    Esphome,
    Tasmota,
    Ntfy,
    OpenMeteo,
}

impl BundledProvider {
    pub const fn package(self) -> &'static str {
        match self {
            Self::Esphome => "esphome",
            Self::Tasmota => "tasmota",
            Self::Ntfy => "ntfy",
            Self::OpenMeteo => "open-meteo",
        }
    }

    pub const fn version(self) -> &'static str {
        match self {
            Self::Tasmota => "0.1.1",
            Self::Esphome | Self::Ntfy | Self::OpenMeteo => "0.1.0",
        }
    }

    pub const fn component_file_name(self) -> &'static str {
        match self {
            Self::Esphome => "esphome.wasm",
            Self::Tasmota => "tasmota.wasm",
            Self::Ntfy => "ntfy.wasm",
            Self::OpenMeteo => "open_meteo.wasm",
        }
    }
}

pub const BUNDLED_PROVIDERS: [BundledProvider; 4] = [
    BundledProvider::Esphome,
    BundledProvider::Tasmota,
    BundledProvider::Ntfy,
    BundledProvider::OpenMeteo,
];

#[derive(Clone)]
pub struct ManagedProvider {
    kind: BundledProvider,
    handle: ProviderHandle,
}

impl ManagedProvider {
    pub fn new(kind: BundledProvider, handle: ProviderHandle) -> Self {
        Self { kind, handle }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManagementServiceBuildError {
    #[error("provider instance {instance_id:?} appears more than once")]
    DuplicateInstance { instance_id: String },
}

#[derive(Clone)]
pub struct ManagementService {
    providers: Arc<[ManagedProvider]>,
}

struct PreparedInteraction {
    provider: ProviderHandle,
    endpoint: fabric::EndpointIdentity,
    generation: fabric::Generation,
    context: CallContext,
}

impl ManagementService {
    pub fn from_bundled(
        providers: impl IntoIterator<Item = ManagedProvider>,
    ) -> Result<Self, ManagementServiceBuildError> {
        let mut providers: Vec<_> = providers.into_iter().collect();
        providers.sort_by(|left, right| {
            left.handle
                .provider()
                .instance()
                .as_str()
                .cmp(right.handle.provider().instance().as_str())
        });
        for adjacent in providers.windows(2) {
            let left = adjacent[0].handle.provider().instance().as_str();
            let right = adjacent[1].handle.provider().instance().as_str();
            if left == right {
                return Err(ManagementServiceBuildError::DuplicateInstance {
                    instance_id: left.to_owned(),
                });
            }
        }
        Ok(Self {
            providers: providers.into(),
        })
    }

    pub fn for_esphome(provider: ProviderHandle) -> Self {
        Self::from_bundled([ManagedProvider::new(BundledProvider::Esphome, provider)])
            .expect("one provider cannot contain a duplicate instance")
    }

    pub async fn inventory_snapshot(
        &self,
        principal: &Principal,
        request: InventoryRequest,
    ) -> Result<InventorySnapshot, ManagementError> {
        self.authorize(principal, INVENTORY_OPERATION)?;
        operation_timeout(INVENTORY_OPERATION, request.timeout_ms)?;

        let total_facets = self
            .providers
            .iter()
            .map(|provider| provider.handle.descriptors().len())
            .sum::<usize>();
        let truncated = total_facets > MAX_INVENTORY_FACETS;
        let mut remaining = MAX_INVENTORY_FACETS;
        let mut instances = Vec::with_capacity(self.providers.len());
        for managed in self.providers.iter() {
            let running = managed.handle.is_running();
            let descriptors = managed.handle.descriptors();
            let facets = descriptors
                .iter()
                .take(remaining)
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
                        operations: canonical_operations(descriptor),
                    }
                })
                .collect::<Vec<_>>();
            remaining = remaining.saturating_sub(facets.len());
            let provider = managed.handle.provider();
            let instance_id = provider.instance().as_str().to_owned();
            instances.push(InventoryInstance {
                instance_id: instance_id.clone(),
                package: managed.kind.package().to_owned(),
                version: managed.kind.version().to_owned(),
                display_name: instance_id,
                status: if running {
                    InstanceStatus::Active
                } else {
                    InstanceStatus::Unavailable
                },
                generation: provider.generation().get(),
                facets,
            });
        }
        tracing::info!(
            operation_id = INVENTORY_OPERATION,
            principal = %principal.id(),
            instance_count = instances.len(),
            facet_count = total_facets.min(MAX_INVENTORY_FACETS),
            truncated,
            "management operation completed"
        );
        Ok(InventorySnapshot {
            instances,
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
        let (provider, endpoint) = self.resolve(QUERY_OPERATION, &request.facet_id)?;
        let generation = generation(QUERY_OPERATION, request.facet_generation)?;
        let context = call_context(timeout, None);
        let state = provider
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
        self.interact(
            principal,
            InteractionRequest::OnOffSet {
                target: InteractionTarget {
                    facet_id: request.facet_id,
                    facet_generation: request.facet_generation,
                    timeout_ms: request.timeout_ms,
                    idempotency_key: request.idempotency_key,
                },
                on: request.on,
            },
        )
        .await
    }

    pub async fn interact(
        &self,
        principal: &Principal,
        request: InteractionRequest,
    ) -> Result<InteractionResult, ManagementError> {
        let operation_id = request.operation_id();
        let result = match request {
            InteractionRequest::OnOffSet { target, on } => {
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::OnOff,
                    "set",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .set_on_off(&call.endpoint, call.generation, on, call.context)
                    .await
                    .map_err(|error| provider_error(operation_id, error))?;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::OnOffToggle { target } => {
                let _ = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::OnOff,
                    "query",
                    &[],
                )?;
                return Err(unsupported_projection(
                    operation_id,
                    "provider component ABI does not expose On/Off toggle",
                ));
            }
            InteractionRequest::LevelSet { target, level } => {
                validate_decimal(operation_id, "level", &level)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::LevelControl,
                    "set-level",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .set_level(
                        &call.endpoint,
                        call.generation,
                        LevelSetRequest {
                            level: ratio(level),
                            transition: None,
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::LevelStep { target, step } => {
                let (direction, amount) = signed_level(operation_id, "step", step)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::LevelControl,
                    "step",
                    &["relative-step"],
                )?;
                let receipt = call
                    .provider
                    .step_level(
                        &call.endpoint,
                        call.generation,
                        LevelStepRequest {
                            direction,
                            amount: ratio(amount),
                            transition: None,
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::LevelMove {
                target,
                direction,
                rate_per_second,
            } => {
                validate_decimal(operation_id, "rate_per_second", &rate_per_second)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::LevelControl,
                    "move",
                    &["continuous-move"],
                )?;
                let receipt = call
                    .provider
                    .move_level(
                        &call.endpoint,
                        call.generation,
                        LevelMoveRequest {
                            direction: level_direction(direction),
                            rate_per_second: ratio(rate_per_second),
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::LevelStop { target } => {
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::LevelControl,
                    "stop",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .stop_level(&call.endpoint, call.generation, call.context)
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::LevelTransition {
                target,
                level,
                duration_ms,
            } => {
                validate_decimal(operation_id, "level", &level)?;
                validate_duration(operation_id, duration_ms)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::LevelControl,
                    "set-level",
                    &["transition"],
                )?;
                let receipt = call
                    .provider
                    .set_level(
                        &call.endpoint,
                        call.generation,
                        LevelSetRequest {
                            level: ratio(level),
                            transition: Some(transition(duration_ms)),
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::LevelSetCoupled {
                target,
                level,
                on_when_nonzero,
            } => {
                validate_decimal(operation_id, "level", &level)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::LevelControl,
                    "set-level-with-on-off",
                    &["coupled-on-off"],
                )?;
                let receipt = call
                    .provider
                    .set_level_with_on_off(
                        &call.endpoint,
                        call.generation,
                        CoupledLevelRequest {
                            level: ratio(level),
                            on_when_nonzero,
                            transition: None,
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::LevelStepCoupled {
                target,
                step,
                on_when_nonzero,
            } => {
                let (direction, amount) = signed_level(operation_id, "step", step)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::LevelControl,
                    "step-with-on-off",
                    &["relative-step", "coupled-on-off"],
                )?;
                let receipt = call
                    .provider
                    .step_level_with_on_off(
                        &call.endpoint,
                        call.generation,
                        CoupledLevelStepRequest {
                            direction,
                            amount: ratio(amount),
                            on_when_nonzero,
                            transition: None,
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::LevelMoveCoupled {
                target,
                direction,
                rate_per_second,
                on_when_nonzero,
            } => {
                validate_decimal(operation_id, "rate_per_second", &rate_per_second)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::LevelControl,
                    "move-with-on-off",
                    &["continuous-move", "coupled-on-off"],
                )?;
                let receipt = call
                    .provider
                    .move_level_with_on_off(
                        &call.endpoint,
                        call.generation,
                        CoupledLevelMoveRequest {
                            direction: level_direction(direction),
                            rate_per_second: ratio(rate_per_second),
                            on_when_nonzero,
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::LevelTransitionCoupled {
                target,
                level,
                duration_ms,
                on_when_nonzero,
            } => {
                validate_decimal(operation_id, "level", &level)?;
                validate_duration(operation_id, duration_ms)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::LevelControl,
                    "set-level-with-on-off",
                    &["transition", "coupled-on-off"],
                )?;
                let receipt = call
                    .provider
                    .set_level_with_on_off(
                        &call.endpoint,
                        call.generation,
                        CoupledLevelRequest {
                            level: ratio(level),
                            on_when_nonzero,
                            transition: Some(transition(duration_ms)),
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ColorSet { target, color } => {
                let color = interaction_color(operation_id, color, None)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::ColorControl,
                    "set-color",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .set_color(
                        &call.endpoint,
                        call.generation,
                        ColorSetRequest {
                            color,
                            transition: None,
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ColorStep {
                target,
                color,
                direction,
            } => {
                let color = interaction_color(operation_id, color, Some(direction))?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::ColorControl,
                    "step-color",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .step_color(
                        &call.endpoint,
                        call.generation,
                        ColorStepRequest {
                            delta: color,
                            transition: None,
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ColorMove {
                target,
                color,
                direction,
            } => {
                let color = interaction_color(operation_id, color, Some(direction))?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::ColorControl,
                    "move-color",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .move_color(
                        &call.endpoint,
                        call.generation,
                        ColorMoveRequest {
                            rate_per_second: color,
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ColorStop { target } => {
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::ColorControl,
                    "stop",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .stop_color(&call.endpoint, call.generation, call.context)
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ColorTransition {
                target,
                color,
                duration_ms,
            } => {
                validate_duration(operation_id, duration_ms)?;
                let color = interaction_color(operation_id, color, None)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::ColorControl,
                    "set-color",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .set_color(
                        &call.endpoint,
                        call.generation,
                        ColorSetRequest {
                            color,
                            transition: Some(transition(duration_ms)),
                        },
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ThermostatSetMode { target, mode } => {
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::Thermostat,
                    "set-mode",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .set_thermostat_mode(
                        &call.endpoint,
                        call.generation,
                        thermostat_set_mode(mode),
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ThermostatSetSingleTarget {
                target,
                target_celsius,
            } => {
                validate_decimal(operation_id, "target_celsius", &target_celsius)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::Thermostat,
                    "set-single-target",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .set_thermostat_single_target(
                        &call.endpoint,
                        call.generation,
                        celsius(target_celsius),
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ThermostatSetDualTarget {
                target,
                heat_target_celsius,
                cool_target_celsius,
            } => {
                validate_decimal(operation_id, "heat_target_celsius", &heat_target_celsius)?;
                validate_decimal(operation_id, "cool_target_celsius", &cool_target_celsius)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::Thermostat,
                    "set-dual-target",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .set_thermostat_dual_target(
                        &call.endpoint,
                        call.generation,
                        celsius(heat_target_celsius),
                        celsius(cool_target_celsius),
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ThermostatSetFan { target, fan_choice } => {
                validate_choice(operation_id, "fan_choice", &fan_choice)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::Thermostat,
                    "set-fan",
                    &["fan-choice"],
                )?;
                let receipt = call
                    .provider
                    .set_thermostat_fan(&call.endpoint, call.generation, fan_choice, call.context)
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ThermostatSetSwing {
                target,
                swing_choice,
            } => {
                validate_choice(operation_id, "swing_choice", &swing_choice)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::Thermostat,
                    "set-swing",
                    &["swing-choice"],
                )?;
                let receipt = call
                    .provider
                    .set_thermostat_swing(
                        &call.endpoint,
                        call.generation,
                        swing_choice,
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::ThermostatSetPreset {
                target,
                preset_choice,
            } => {
                validate_choice(operation_id, "preset_choice", &preset_choice)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::Thermostat,
                    "set-preset",
                    &["preset-choice"],
                )?;
                let receipt = call
                    .provider
                    .set_thermostat_preset(
                        &call.endpoint,
                        call.generation,
                        preset_choice,
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::CoveringOpen { target, axis } => {
                let (axis, feature) = covering_axis(axis);
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::WindowCovering,
                    "open",
                    &[feature],
                )?;
                let receipt = call
                    .provider
                    .open_covering(&call.endpoint, call.generation, axis, call.context)
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::CoveringClose { target, axis } => {
                let (axis, feature) = covering_axis(axis);
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::WindowCovering,
                    "close",
                    &[feature],
                )?;
                let receipt = call
                    .provider
                    .close_covering(&call.endpoint, call.generation, axis, call.context)
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::CoveringStop { target, axis } => {
                let (axis, feature) = covering_axis(axis);
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::WindowCovering,
                    "stop",
                    &[feature, "stop"],
                )?;
                let receipt = call
                    .provider
                    .stop_covering(&call.endpoint, call.generation, axis, call.context)
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::CoveringSetLift { target, position } => {
                validate_decimal(operation_id, "position", &position)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::WindowCovering,
                    "set-position",
                    &["lift", "target-position"],
                )?;
                let receipt = call
                    .provider
                    .set_covering_position(
                        &call.endpoint,
                        call.generation,
                        HostCoveringAxis::Lift,
                        ratio(position),
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::CoveringSetTilt { target, position } => {
                validate_decimal(operation_id, "position", &position)?;
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::WindowCovering,
                    "set-position",
                    &["tilt", "target-position"],
                )?;
                let receipt = call
                    .provider
                    .set_covering_position(
                        &call.endpoint,
                        call.generation,
                        HostCoveringAxis::Tilt,
                        ratio(position),
                        call.context,
                    )
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::MomentaryInvoke { target } => {
                let call = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::MomentaryAction,
                    "invoke",
                    &[],
                )?;
                let receipt = call
                    .provider
                    .invoke_momentary_action(&call.endpoint, call.generation, call.context)
                    .await
                    .map_err(|error| provider_error(operation_id, error))?
                    .receipt;
                interaction_result(target.facet_id, receipt)
            }
            InteractionRequest::NotificationSend {
                target,
                notification_template_id,
                expires_in_ms,
                ..
            } => {
                if !valid_public_id(&notification_template_id)
                    || expires_in_ms.is_some_and(|value| value == 0 || value > 86_400_000)
                {
                    return Err(caller_error(
                        operation_id,
                        ErrorCode::ContractFailed,
                        "notification template request is invalid",
                    ));
                }
                let _ = self.prepare_interaction(
                    principal,
                    operation_id,
                    &target,
                    FacetKind::NotificationSink,
                    "send",
                    &[],
                )?;
                return Err(management_error(
                    operation_id,
                    ErrorGroup::Availability,
                    ErrorCode::Unsupported,
                    Some(ErrorOrigin::Host),
                    RetryKind::Never,
                    "notification template expansion is unavailable",
                    OutcomeConfidence::Definite,
                ));
            }
        };
        tracing::info!(
            operation_id,
            principal = %principal.id(),
            facet_id = %result.facet_id,
            confidence = ?result.confidence,
            deduplicated = result.deduplicated,
            "management operation completed"
        );
        Ok(result)
    }

    fn prepare_interaction(
        &self,
        principal: &Principal,
        operation_id: &'static str,
        target: &InteractionTarget,
        expected_kind: FacetKind,
        provider_operation: &str,
        required_features: &[&str],
    ) -> Result<PreparedInteraction, ManagementError> {
        self.authorize(principal, operation_id)?;
        validate_facet_input(operation_id, &target.facet_id, target.facet_generation)?;
        if !valid_idempotency_key(&target.idempotency_key) {
            return Err(caller_error(
                operation_id,
                ErrorCode::ContractFailed,
                "idempotency_key must be 32 lowercase hexadecimal characters",
            ));
        }
        let timeout = operation_timeout(operation_id, target.timeout_ms)?;
        let (provider, descriptor) = self.resolve_descriptor(operation_id, &target.facet_id)?;
        let endpoint = descriptor.endpoint();
        let supported = descriptor.kind() == expected_kind
            && endpoint
                .operations()
                .iter()
                .any(|operation| operation.as_str() == provider_operation)
            && required_features.iter().all(|required| {
                endpoint
                    .features()
                    .iter()
                    .any(|feature| feature.as_str() == *required)
            });
        if !supported {
            return Err(caller_error(
                operation_id,
                ErrorCode::Unsupported,
                "facet does not support the requested typed capability operation",
            ));
        }
        Ok(PreparedInteraction {
            provider,
            endpoint: endpoint.identity().clone(),
            generation: generation(operation_id, target.facet_generation)?,
            context: call_context(timeout, Some(target.idempotency_key.clone())),
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
    ) -> Result<(ProviderHandle, fabric::EndpointIdentity), ManagementError> {
        self.resolve_descriptor(operation_id, facet_id)
            .map(|(provider, descriptor)| (provider, descriptor.endpoint().identity().clone()))
    }

    fn resolve_descriptor(
        &self,
        operation_id: &'static str,
        facet_id: &str,
    ) -> Result<(ProviderHandle, litehouse_provider_host::HostFacetDescriptor), ManagementError>
    {
        let mut matches = self.providers.iter().filter_map(|provider| {
            provider
                .handle
                .descriptors()
                .iter()
                .find(|descriptor| descriptor.endpoint().identity().facet().as_str() == facet_id)
                .map(|descriptor| (provider.handle.clone(), descriptor.clone()))
        });
        let Some(found) = matches.next() else {
            return Err(management_error(
                operation_id,
                ErrorGroup::Binding,
                ErrorCode::NotFound,
                Some(ErrorOrigin::Caller),
                RetryKind::Never,
                "facet was not found",
                OutcomeConfidence::Definite,
            ));
        };
        if matches.next().is_some() {
            return Err(management_error(
                operation_id,
                ErrorGroup::Binding,
                ErrorCode::Ambiguous,
                Some(ErrorOrigin::Caller),
                RetryKind::Never,
                "facet identity is ambiguous across provider instances",
                OutcomeConfidence::Definite,
            ));
        }
        Ok(found)
    }
}

#[derive(Clone)]
pub struct McpAdapter {
    management: ManagementService,
    principal: Principal,
    workflows: Option<Arc<WorkflowRepository>>,
    tool_router: ToolRouter<Self>,
}

impl McpAdapter {
    pub fn new(management: ManagementService, principal: Principal) -> Self {
        Self {
            management,
            principal,
            workflows: None,
            tool_router: Self::tool_router(),
        }
    }

    pub fn with_workflows(mut self, workflows: Arc<WorkflowRepository>) -> Self {
        self.workflows = Some(workflows);
        self
    }

    fn workflow_operation(
        &self,
        request: WorkflowRequest,
    ) -> Result<WorkflowToolResult, ManagementError> {
        let repository = self.workflows.as_ref().ok_or_else(|| {
            management_error(
                WORKFLOW_OPERATION,
                ErrorGroup::Availability,
                ErrorCode::Unsupported,
                Some(ErrorOrigin::Host),
                RetryKind::Never,
                "workflow authoring is unavailable",
                OutcomeConfidence::Definite,
            )
        })?;
        match request {
            WorkflowRequest::List => {
                let authored = repository.authored_snapshot().map_err(workflow_error)?;
                workflow_tool_result(repository, authored, None, None)
            }
            WorkflowRequest::Get { rule_id } => {
                if !valid_idempotency_key(&rule_id) {
                    return Err(caller_error(
                        WORKFLOW_OPERATION,
                        ErrorCode::ContractFailed,
                        "rule_id must be 32 lowercase hexadecimal characters",
                    ));
                }
                let (authored, source) = repository
                    .authored_workflow_source(&rule_id)
                    .map_err(workflow_error)?;
                let source = String::from_utf8(source).map_err(|_| {
                    caller_error(
                        WORKFLOW_OPERATION,
                        ErrorCode::ContractFailed,
                        "workflow source is not UTF-8",
                    )
                })?;
                let mut result = workflow_tool_result(repository, authored, Some(source), None)?;
                result
                    .workflows
                    .retain(|workflow| workflow.rule_id == rule_id);
                Ok(result)
            }
            WorkflowRequest::Validate {
                relative_path,
                source,
            } => {
                let validation =
                    WorkflowRepository::validate_source(&relative_path, source.as_bytes())
                        .map_err(workflow_error)?;
                let authored = repository.authored_snapshot().map_err(workflow_error)?;
                workflow_tool_result(
                    repository,
                    authored,
                    None,
                    Some(workflow_validation_result(validation)),
                )
            }
            WorkflowRequest::Put {
                relative_path,
                expected_sha256,
                source,
            } => {
                let expected = expected_sha256.as_deref().map(parse_sha256).transpose()?;
                let authored = repository
                    .put(&relative_path, expected, source.as_bytes())
                    .map_err(workflow_error)?;
                workflow_tool_result(repository, authored, None, None)
            }
            WorkflowRequest::SetEnabled {
                relative_path,
                expected_sha256,
                enabled,
            } => {
                let expected = parse_sha256(&expected_sha256)?;
                let authored = repository
                    .set_enabled(&relative_path, expected, enabled)
                    .map_err(workflow_error)?;
                workflow_tool_result(repository, authored, None, None)
            }
            WorkflowRequest::Delete {
                relative_path,
                expected_sha256,
            } => {
                let expected = parse_sha256(&expected_sha256)?;
                let authored = repository
                    .remove(&relative_path, expected)
                    .map_err(workflow_error)?;
                workflow_tool_result(repository, authored, None, None)
            }
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
        description = "Invoke one of the 29 closed typed Litehouse capability operations.",
        input_schema = interaction_input_schema(),
        output_schema = canonical_output_schema(CAPABILITY_OPERATION_IDS[0])
    )]
    async fn interact(
        &self,
        Parameters(request): Parameters<InteractionRequest>,
    ) -> CallToolResult {
        tool_result(self.management.interact(&self.principal, request).await)
    }

    #[tool(
        name = "litehouse_workflows",
        description = "Inspect, validate, and atomically author Rule JSON workflows. Authored changes require a Litehouse restart.",
        input_schema = workflow_input_schema(),
        output_schema = rmcp::handler::server::tool::schema_for_type::<WorkflowToolResult>()
    )]
    async fn workflows(&self, Parameters(request): Parameters<WorkflowRequest>) -> CallToolResult {
        tool_result(self.workflow_operation(request))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for McpAdapter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Use only the four closed Litehouse Phase 1 tools.")
    }
}

fn interaction_input_schema() -> Arc<Map<String, Value>> {
    let manifest = management_manifest().expect("embedded management manifest is validated");
    let variants = CAPABILITY_OPERATION_IDS
        .iter()
        .map(|operation_id| {
            let operation = manifest
                .operations
                .iter()
                .find(|operation| operation.id == *operation_id)
                .expect("capability operation is present in the embedded manifest");
            let mut schema = operation.schemas.mcp_projection.schema.clone();
            let object = schema
                .as_object_mut()
                .expect("capability MCP projection is an object schema");
            let properties = object
                .get_mut("properties")
                .and_then(Value::as_object_mut)
                .expect("capability MCP projection has properties");
            properties.insert(
                "operation".to_owned(),
                serde_json::json!({ "const": operation_id, "type": "string" }),
            );
            object
                .get_mut("required")
                .and_then(Value::as_array_mut)
                .expect("capability MCP projection has required fields")
                .push(Value::String("operation".to_owned()));
            schema
        })
        .collect::<Vec<_>>();
    Arc::new(Map::from_iter([
        ("type".to_owned(), Value::String("object".to_owned())),
        ("oneOf".to_owned(), Value::Array(variants)),
    ]))
}

fn workflow_input_schema() -> Arc<Map<String, Value>> {
    let generated = rmcp::handler::server::tool::schema_for_type::<WorkflowRequest>();
    let mut schema = generated.as_ref().clone();
    schema.insert("type".to_owned(), Value::String("object".to_owned()));
    Arc::new(schema)
}

fn workflow_tool_result(
    repository: &WorkflowRepository,
    authored: Arc<WorkflowDirectory>,
    source: Option<String>,
    validation: Option<WorkflowValidationResult>,
) -> Result<WorkflowToolResult, ManagementError> {
    let loaded = repository.snapshot().map_err(workflow_error)?;
    let authored_sha256 = hex_sha256(authored.authored_sha256());
    let loaded_sha256 = hex_sha256(loaded.authored_sha256());
    let workflows = authored
        .workflows()
        .values()
        .map(workflow_summary)
        .collect();
    Ok(WorkflowToolResult {
        restart_required: authored_sha256 != loaded_sha256,
        authored_sha256,
        loaded_sha256,
        workflows,
        source,
        validation,
    })
}

fn workflow_summary(workflow: &LoadedWorkflow) -> WorkflowSummary {
    WorkflowSummary {
        rule_id: workflow.compiled().rule_id().to_owned(),
        relative_path: workflow.relative_path().to_owned(),
        revision: workflow.compiled().revision(),
        enabled: workflow.compiled().enabled(),
        source_sha256: hex_sha256(workflow.source_sha256()),
        semantic_sha256: workflow.compiled().semantic_sha256().to_owned(),
    }
}

fn workflow_validation_result(validation: WorkflowValidation) -> WorkflowValidationResult {
    WorkflowValidationResult {
        rule_id: validation.rule_id,
        revision: validation.revision,
        enabled: validation.enabled,
        semantic_sha256: validation.semantic_sha256,
    }
}

fn parse_sha256(value: &str) -> Result<[u8; 32], ManagementError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(caller_error(
            WORKFLOW_OPERATION,
            ErrorCode::ContractFailed,
            "expected_sha256 must be 64 lowercase hexadecimal characters",
        ));
    }
    let mut digest = [0_u8; 32];
    for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(pair).expect("validated hexadecimal is UTF-8");
        digest[index] = u8::from_str_radix(text, 16).expect("validated hexadecimal decodes");
    }
    Ok(digest)
}

fn hex_sha256(value: [u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn workflow_error(error: WorkflowMutationError) -> ManagementError {
    match error {
        WorkflowMutationError::Conflict => management_error(
            WORKFLOW_OPERATION,
            ErrorGroup::Contract,
            ErrorCode::StaleHandle,
            Some(ErrorOrigin::Caller),
            RetryKind::Never,
            "workflow source changed since it was read",
            OutcomeConfidence::Definite,
        ),
        WorkflowMutationError::Busy => management_error(
            WORKFLOW_OPERATION,
            ErrorGroup::Capacity,
            ErrorCode::Overloaded,
            Some(ErrorOrigin::Host),
            RetryKind::Retry,
            "another authored-file mutation is in progress",
            OutcomeConfidence::Definite,
        ),
        WorkflowMutationError::CommitIndeterminate(_)
        | WorkflowMutationError::AuthoredButNotLoaded { .. }
        | WorkflowMutationError::AuthoredChangedAfterCommit { .. } => management_error(
            WORKFLOW_OPERATION,
            ErrorGroup::OutcomeUnknown,
            ErrorCode::OutcomeUnknown,
            Some(ErrorOrigin::Host),
            RetryKind::Never,
            "workflow mutation outcome requires a fresh authored read",
            OutcomeConfidence::OutcomeUnknown,
        ),
        WorkflowMutationError::StatePoisoned | WorkflowMutationError::Io { .. } => {
            management_error(
                WORKFLOW_OPERATION,
                ErrorGroup::Execution,
                ErrorCode::ProviderFailed,
                Some(ErrorOrigin::Host),
                RetryKind::Never,
                "workflow authoring failed",
                OutcomeConfidence::Definite,
            )
        }
        WorkflowMutationError::WorkflowNotFound { .. } => management_error(
            WORKFLOW_OPERATION,
            ErrorGroup::Binding,
            ErrorCode::NotFound,
            Some(ErrorOrigin::Caller),
            RetryKind::Never,
            "workflow was not found",
            OutcomeConfidence::Definite,
        ),
        WorkflowMutationError::Files(_)
        | WorkflowMutationError::InvalidPath { .. }
        | WorkflowMutationError::UnsupportedExtension { .. }
        | WorkflowMutationError::DuplicateRuleId { .. } => caller_error(
            WORKFLOW_OPERATION,
            ErrorCode::ContractFailed,
            "workflow source failed validation",
        ),
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

fn canonical_operations(descriptor: &litehouse_provider_host::HostFacetDescriptor) -> Vec<String> {
    let endpoint = descriptor.endpoint();
    let supports_operation = |name: &str| {
        endpoint
            .operations()
            .iter()
            .any(|operation| operation.as_str() == name)
    };
    let supports_features = |required: &[&str]| {
        required.iter().all(|required| {
            if *required == "lift-or-tilt" {
                endpoint
                    .features()
                    .iter()
                    .any(|feature| matches!(feature.as_str(), "lift" | "tilt"))
            } else {
                endpoint
                    .features()
                    .iter()
                    .any(|feature| feature.as_str() == *required)
            }
        })
    };
    let mappings: &[(&str, &str, &[&str])] = match descriptor.kind() {
        FacetKind::OnOff => &[
            (QUERY_OPERATION, "query", &[]),
            ("capability.on-off.set@1", "set", &[]),
            ("capability.on-off.toggle@1", "toggle", &[]),
        ],
        FacetKind::BooleanState | FacetKind::Measurement => &[(QUERY_OPERATION, "query", &[])],
        FacetKind::NotificationSink => &[("capability.notification.send@1", "send", &[])],
        FacetKind::LevelControl => &[
            (QUERY_OPERATION, "query", &[]),
            ("capability.level.set@1", "set-level", &[]),
            ("capability.level.step@1", "step", &["relative-step"]),
            ("capability.level.move@1", "move", &["continuous-move"]),
            ("capability.level.stop@1", "stop", &[]),
            (
                "capability.level.transition@1",
                "set-level",
                &["transition"],
            ),
            (
                "capability.level.set-coupled@1",
                "set-level-with-on-off",
                &["coupled-on-off"],
            ),
            (
                "capability.level.step-coupled@1",
                "step-with-on-off",
                &["relative-step", "coupled-on-off"],
            ),
            (
                "capability.level.move-coupled@1",
                "move-with-on-off",
                &["continuous-move", "coupled-on-off"],
            ),
            (
                "capability.level.transition-coupled@1",
                "set-level-with-on-off",
                &["transition", "coupled-on-off"],
            ),
        ],
        FacetKind::ColorControl => &[
            (QUERY_OPERATION, "query", &[]),
            ("capability.color.set@1", "set-color", &[]),
            ("capability.color.step@1", "step-color", &[]),
            ("capability.color.move@1", "move-color", &[]),
            ("capability.color.stop@1", "stop", &[]),
            ("capability.color.transition@1", "set-color", &[]),
        ],
        FacetKind::Thermostat => &[
            (QUERY_OPERATION, "query", &[]),
            ("capability.thermostat.set-mode@1", "set-mode", &[]),
            (
                "capability.thermostat.set-single-target@1",
                "set-single-target",
                &[],
            ),
            (
                "capability.thermostat.set-dual-target@1",
                "set-dual-target",
                &[],
            ),
            (
                "capability.thermostat.set-fan@1",
                "set-fan",
                &["fan-choice"],
            ),
            (
                "capability.thermostat.set-swing@1",
                "set-swing",
                &["swing-choice"],
            ),
            (
                "capability.thermostat.set-preset@1",
                "set-preset",
                &["preset-choice"],
            ),
        ],
        FacetKind::WindowCovering => &[
            (QUERY_OPERATION, "query", &[]),
            ("capability.covering.open@1", "open", &["lift-or-tilt"]),
            ("capability.covering.close@1", "close", &["lift-or-tilt"]),
            (
                "capability.covering.stop@1",
                "stop",
                &["lift-or-tilt", "stop"],
            ),
            (
                "capability.covering.set-lift@1",
                "set-position",
                &["lift", "target-position"],
            ),
            (
                "capability.covering.set-tilt@1",
                "set-position",
                &["tilt", "target-position"],
            ),
        ],
        FacetKind::MomentaryAction => &[("capability.momentary.invoke@1", "invoke", &[])],
    };
    mappings
        .iter()
        .filter(|(_, operation, features)| {
            supports_operation(operation) && supports_features(features)
        })
        .map(|(canonical, _, _)| (*canonical).to_owned())
        .collect()
}

fn interaction_result(
    facet_id: String,
    receipt: litehouse_provider_host::CommandReceipt,
) -> InteractionResult {
    InteractionResult {
        facet_id,
        generation: receipt.provider_generation.get(),
        revision: 0,
        receipt_id: receipt.request_id,
        confidence: match receipt.confidence {
            HostReceiptConfidence::AcceptedForDelivery => ReceiptConfidence::AcceptedForDelivery,
            HostReceiptConfidence::ProtocolOrTargetAcknowledged => ReceiptConfidence::Acknowledged,
            HostReceiptConfidence::Completed => ReceiptConfidence::Completed,
            HostReceiptConfidence::ObservedCompleted => ReceiptConfidence::ObservedCompleted,
        },
        deduplicated: receipt.deduplicated,
    }
}

fn ratio(magnitude: String) -> HostQuantity {
    HostQuantity {
        profile: "ratio".to_owned(),
        magnitude,
        unit: "1".to_owned(),
    }
}

fn celsius(magnitude: String) -> HostQuantity {
    HostQuantity {
        profile: "temperature".to_owned(),
        magnitude,
        unit: "Cel".to_owned(),
    }
}

fn transition(duration_ms: u32) -> HostTransition {
    HostTransition {
        duration_milliseconds: u64::from(duration_ms),
    }
}

fn level_direction(direction: LevelDirection) -> HostStepDirection {
    match direction {
        LevelDirection::Up => HostStepDirection::Increase,
        LevelDirection::Down => HostStepDirection::Decrease,
    }
}

fn signed_level(
    operation_id: &'static str,
    field: &str,
    value: String,
) -> Result<(HostStepDirection, String), ManagementError> {
    validate_decimal(operation_id, field, &value)?;
    Ok(if let Some(unsigned) = value.strip_prefix('-') {
        (HostStepDirection::Decrease, unsigned.to_owned())
    } else {
        (HostStepDirection::Increase, value)
    })
}

fn validate_decimal(
    operation_id: &'static str,
    _field: &str,
    value: &str,
) -> Result<(), ManagementError> {
    let valid = value.len() <= 32 && !value.is_empty() && {
        let value = value.strip_prefix('-').unwrap_or(value);
        let (integer, fraction) = value
            .split_once('.')
            .map_or((value, None), |(i, f)| (i, Some(f)));
        !integer.is_empty()
            && integer.bytes().all(|byte| byte.is_ascii_digit())
            && (integer == "0" || !integer.starts_with('0'))
            && fraction.is_none_or(|fraction| {
                !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
            })
    };
    if valid {
        Ok(())
    } else {
        Err(caller_error(
            operation_id,
            ErrorCode::ContractFailed,
            "value must be a canonical bounded decimal",
        ))
    }
}

fn validate_duration(operation_id: &'static str, duration_ms: u32) -> Result<(), ManagementError> {
    if (1..=30_000).contains(&duration_ms) {
        Ok(())
    } else {
        Err(caller_error(
            operation_id,
            ErrorCode::ContractFailed,
            "duration_ms must be between 1 and 30000",
        ))
    }
}

fn validate_choice(
    operation_id: &'static str,
    _field: &str,
    value: &str,
) -> Result<(), ManagementError> {
    if !value.is_empty() && value.len() <= 64 && !value.bytes().any(|byte| byte.is_ascii_control())
    {
        Ok(())
    } else {
        Err(caller_error(
            operation_id,
            ErrorCode::ContractFailed,
            "choice must be non-empty and bounded",
        ))
    }
}

fn signed_color_decimal(value: String, direction: ColorDirection) -> String {
    match direction {
        ColorDirection::Increase => value,
        ColorDirection::Decrease => value
            .strip_prefix('-')
            .map_or_else(|| format!("-{value}"), ToOwned::to_owned),
    }
}

fn interaction_color(
    operation_id: &'static str,
    color: InteractionColor,
    direction: Option<ColorDirection>,
) -> Result<HostColorValue, ManagementError> {
    let project = |field: &str, value: String| -> Result<String, ManagementError> {
        validate_decimal(operation_id, field, &value)?;
        Ok(direction.map_or(value.clone(), |direction| {
            signed_color_decimal(value, direction)
        }))
    };
    Ok(match color {
        InteractionColor::HueSaturation {
            hue_degrees,
            saturation,
        } => HostColorValue::HueSaturation {
            hue_degrees: project("hue_degrees", hue_degrees)?,
            saturation_ratio: project("saturation", saturation)?,
        },
        InteractionColor::CieXy { x, y } => HostColorValue::CieXy {
            x: project("x", x)?,
            y: project("y", y)?,
        },
        InteractionColor::ColorTemperature { kelvin } => HostColorValue::ColorTemperature {
            kelvin: project("kelvin", kelvin)?,
        },
        InteractionColor::CalibratedRgb {
            red,
            green,
            blue,
            color_space,
        } => HostColorValue::CalibratedRgb {
            red: project("red", red)?,
            green: project("green", green)?,
            blue: project("blue", blue)?,
            color_space: match color_space {
                InteractionColorSpace::Srgb => "srgb",
                InteractionColorSpace::DisplayP3 => "display-p3",
            }
            .to_owned(),
        },
        InteractionColor::UncalibratedRgb { red, green, blue } => HostColorValue::UncalibratedRgb {
            red: project("red", red)?,
            green: project("green", green)?,
            blue: project("blue", blue)?,
        },
        InteractionColor::NativeWhiteChannels {
            white,
            cold_white,
            warm_white,
        } => HostColorValue::NativeWhiteChannels {
            white: Some(project("white", white)?),
            cold_white: cold_white
                .map(|value| project("cold_white", value))
                .transpose()?,
            warm_white: warm_white
                .map(|value| project("warm_white", value))
                .transpose()?,
        },
    })
}

fn thermostat_set_mode(mode: InteractionThermostatMode) -> HostThermostatMode {
    HostThermostatMode::Known(match mode {
        InteractionThermostatMode::Off => HostKnownThermostatMode::Off,
        InteractionThermostatMode::Heat => HostKnownThermostatMode::Heat,
        InteractionThermostatMode::Cool => HostKnownThermostatMode::Cool,
        InteractionThermostatMode::Auto => HostKnownThermostatMode::Auto,
        InteractionThermostatMode::Dry => HostKnownThermostatMode::Dry,
        InteractionThermostatMode::FanOnly => HostKnownThermostatMode::FanOnly,
    })
}

fn covering_axis(axis: InteractionCoveringAxis) -> (HostCoveringAxis, &'static str) {
    match axis {
        InteractionCoveringAxis::Lift => (HostCoveringAxis::Lift, "lift"),
        InteractionCoveringAxis::Tilt => (HostCoveringAxis::Tilt, "tilt"),
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
            HostFacetValue::Measurement(current) => FacetState::Measurement {
                current: current.map(|value| MeasurementStateValue {
                    profile: value.profile,
                    magnitude: value.magnitude,
                    unit: value.unit,
                    native_magnitude: value.native_magnitude,
                    native_unit: value.native_unit,
                }),
            },
            HostFacetValue::Level(v) => FacetState::Level {
                current: v.current.map(quantity_value),
                target: v.target.map(quantity_value),
                transition_remaining_milliseconds: v.transition_remaining_milliseconds,
            },
            HostFacetValue::Color(v) => FacetState::Color {
                current: v.current.map(color_value),
                target: v.target.map(color_value),
                active_model: v.active_model.map(color_model),
            },
            HostFacetValue::Thermostat(v) => FacetState::Thermostat {
                current_mode: v.current_mode.map(thermostat_mode),
                target_mode: v.target_mode.map(thermostat_mode),
                current_action: v.current_action.map(|v| match v {
                    HostThermostatAction::Idle => ThermostatAction::Idle,
                    HostThermostatAction::Heating => ThermostatAction::Heating,
                    HostThermostatAction::Cooling => ThermostatAction::Cooling,
                    HostThermostatAction::Drying => ThermostatAction::Drying,
                    HostThermostatAction::Fan => ThermostatAction::Fan,
                    HostThermostatAction::Defrosting => ThermostatAction::Defrosting,
                }),
                running: v.running,
                targets: v.targets.map(|v| match v {
                    HostThermostatTargets::Single(v) => ThermostatTargets::Single {
                        temperature: quantity_value(v),
                    },
                    HostThermostatTargets::HeatCool { heat, cool } => ThermostatTargets::HeatCool {
                        heat: quantity_value(heat),
                        cool: quantity_value(cool),
                    },
                }),
                fan: v.fan,
                swing: v.swing,
                preset: v.preset,
            },
            HostFacetValue::WindowCovering(v) => FacetState::WindowCovering {
                axes: v
                    .axes
                    .into_iter()
                    .map(|v| CoveringAxisState {
                        axis: match v.axis {
                            HostCoveringAxis::Lift => CoveringAxis::Lift,
                            HostCoveringAxis::Tilt => CoveringAxis::Tilt,
                        },
                        current: v.current.map(quantity_value),
                        target: v.target.map(quantity_value),
                        movement: match v.movement {
                            HostCoveringMovement::Opening => CoveringMovement::Opening,
                            HostCoveringMovement::Closing => CoveringMovement::Closing,
                            HostCoveringMovement::Stopped => CoveringMovement::Stopped,
                            HostCoveringMovement::Idle => CoveringMovement::Idle,
                            HostCoveringMovement::Unknown => CoveringMovement::Unknown,
                        },
                    })
                    .collect(),
                safety_status: v.safety_status,
                fault_status: v.fault_status,
            },
        },
    }
}
fn quantity_value(v: litehouse_provider_host::Quantity) -> QuantityValue {
    QuantityValue {
        profile: v.profile,
        magnitude: v.magnitude,
        unit: v.unit,
    }
}
fn color_model(v: HostColorModel) -> ColorModel {
    match v {
        HostColorModel::HueSaturation => ColorModel::HueSaturation,
        HostColorModel::CieXy => ColorModel::CieXy,
        HostColorModel::ColorTemperature => ColorModel::ColorTemperature,
        HostColorModel::CalibratedRgb => ColorModel::CalibratedRgb,
        HostColorModel::UncalibratedRgb => ColorModel::UncalibratedRgb,
        HostColorModel::NativeWhiteChannels => ColorModel::NativeWhiteChannels,
    }
}
fn color_value(v: HostColorValue) -> ColorValue {
    match v {
        HostColorValue::HueSaturation {
            hue_degrees,
            saturation_ratio,
        } => ColorValue::HueSaturation {
            hue_degrees,
            saturation_ratio,
        },
        HostColorValue::CieXy { x, y } => ColorValue::CieXy { x, y },
        HostColorValue::ColorTemperature { kelvin } => ColorValue::ColorTemperature { kelvin },
        HostColorValue::CalibratedRgb {
            red,
            green,
            blue,
            color_space,
        } => ColorValue::CalibratedRgb {
            red,
            green,
            blue,
            color_space,
        },
        HostColorValue::UncalibratedRgb { red, green, blue } => {
            ColorValue::UncalibratedRgb { red, green, blue }
        }
        HostColorValue::NativeWhiteChannels {
            white,
            cold_white,
            warm_white,
        } => ColorValue::NativeWhiteChannels {
            white,
            cold_white,
            warm_white,
        },
    }
}
fn thermostat_mode(v: HostThermostatMode) -> ThermostatMode {
    match v {
        HostThermostatMode::Known(v) => match v {
            HostKnownThermostatMode::Off => ThermostatMode::Off,
            HostKnownThermostatMode::Auto => ThermostatMode::Auto,
            HostKnownThermostatMode::Heat => ThermostatMode::Heat,
            HostKnownThermostatMode::Cool => ThermostatMode::Cool,
            HostKnownThermostatMode::Dry => ThermostatMode::Dry,
            HostKnownThermostatMode::FanOnly => ThermostatMode::FanOnly,
        },
        HostThermostatMode::ProviderQualified(v) => ThermostatMode::ProviderQualified(v),
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

fn unsupported_projection(operation_id: &'static str, message: &'static str) -> ManagementError {
    management_error(
        operation_id,
        ErrorGroup::Availability,
        ErrorCode::Unsupported,
        Some(ErrorOrigin::Host),
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
    fn bundled_provider_catalog_is_exact_and_stable() {
        assert_eq!(
            BUNDLED_PROVIDERS.map(|provider| (
                provider.package(),
                provider.version(),
                provider.component_file_name()
            )),
            [
                ("esphome", "0.1.0", "esphome.wasm"),
                ("tasmota", "0.1.1", "tasmota.wasm"),
                ("ntfy", "0.1.0", "ntfy.wasm"),
                ("open-meteo", "0.1.0", "open_meteo.wasm"),
            ]
        );
    }

    #[tokio::test]
    async fn empty_bundled_inventory_is_valid() {
        let service = ManagementService::from_bundled([]).unwrap();
        let principal = Principal::proving("phase1-test").unwrap();
        assert_eq!(
            service
                .inventory_snapshot(&principal, InventoryRequest::default())
                .await
                .unwrap(),
            InventorySnapshot {
                instances: Vec::new(),
                truncated: false,
            }
        );
    }

    #[test]
    fn phase1_mcp_surface_has_exactly_four_tools() {
        let tools = McpAdapter::tool_router().list_all();
        let names = tools
            .iter()
            .map(|tool| tool.name.to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            names,
            BTreeSet::from([
                "litehouse_interact".to_owned(),
                "litehouse_inventory".to_owned(),
                "litehouse_query".to_owned(),
                "litehouse_workflows".to_owned(),
            ])
        );
        let workflow = tools
            .iter()
            .find(|tool| tool.name == "litehouse_workflows")
            .unwrap();
        let schema = serde_json::to_string(&workflow.input_schema).unwrap();
        assert!(schema.contains("\"maxLength\":131072"));
        assert!(schema.contains("\"maxLength\":512"));
        assert!(schema.contains("^[0-9a-f]{64}$"));
    }

    #[test]
    fn workflow_tool_authors_without_replacing_the_loaded_snapshot() {
        let root = std::env::temp_dir().join(format!(
            "litehouse-workflow-tool-{}-{}",
            std::process::id(),
            REQUEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source = include_str!("../../litehouse-rule-json/fixtures/valid-minimal.json");
        std::fs::write(root.join("rule.json"), source).unwrap();
        let repository = Arc::new(WorkflowRepository::open(&root).unwrap());
        let loaded = repository.snapshot().unwrap();
        let workflow = loaded.workflows().values().next().unwrap();
        let expected_sha256 = hex_sha256(workflow.source_sha256());
        let mut updated: serde_json::Value = serde_json::from_str(source).unwrap();
        updated["revision"] = serde_json::json!(2);
        let updated = serde_json::to_string_pretty(&updated).unwrap();
        let adapter = McpAdapter::new(
            ManagementService::from_bundled([]).unwrap(),
            Principal::proving("phase1-workflow-test").unwrap(),
        )
        .with_workflows(repository.clone());

        let result = adapter
            .workflow_operation(WorkflowRequest::Put {
                relative_path: "rule.json".to_owned(),
                expected_sha256: Some(expected_sha256),
                source: updated,
            })
            .unwrap();
        assert!(result.restart_required);
        assert_eq!(result.workflows[0].revision, 2);
        assert_eq!(
            repository
                .snapshot()
                .unwrap()
                .workflows()
                .values()
                .next()
                .unwrap()
                .compiled()
                .revision(),
            1
        );
        let disabled = adapter
            .workflow_operation(WorkflowRequest::SetEnabled {
                relative_path: "rule.json".to_owned(),
                expected_sha256: result.workflows[0].source_sha256.clone(),
                enabled: false,
            })
            .unwrap();
        assert!(!disabled.workflows[0].enabled);
        assert!(disabled.restart_required);
        let get = adapter
            .workflow_operation(WorkflowRequest::Get {
                rule_id: result.workflows[0].rule_id.clone(),
            })
            .unwrap();
        let source = get.source.unwrap();
        assert!(source.contains("\"revision\": 2"));
        assert!(source.contains("\"enabled\": false"));

        drop(adapter);
        drop(repository);
        std::fs::remove_dir_all(root).unwrap();
    }

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
            operation_timeout(CAPABILITY_OPERATION_IDS[0], None).unwrap(),
            Duration::from_secs(30)
        );
        assert!(operation_timeout(QUERY_OPERATION, Some(30_000)).is_ok());
        assert!(operation_timeout(QUERY_OPERATION, Some(0)).is_err());
        assert!(operation_timeout(QUERY_OPERATION, Some(30_001)).is_err());
    }

    #[test]
    fn rmcp_metadata_uses_the_frozen_projection_and_output_schemas() {
        let manifest = management_manifest().unwrap();
        for operation_id in [
            INVENTORY_OPERATION,
            QUERY_OPERATION,
            CAPABILITY_OPERATION_IDS[0],
        ] {
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
    fn interaction_schema_is_manifest_driven_and_exactly_29_variants() {
        let schema = interaction_input_schema();
        let variants = schema["oneOf"].as_array().unwrap();
        let actual = variants
            .iter()
            .map(|variant| {
                variant["properties"]["operation"]["const"]
                    .as_str()
                    .unwrap()
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(variants.len(), CAPABILITY_OPERATION_IDS.len());
        assert_eq!(actual.len(), CAPABILITY_OPERATION_IDS.len());
        assert_eq!(actual, CAPABILITY_OPERATION_IDS.into_iter().collect());
        assert!(
            serde_json::from_value::<InteractionRequest>(serde_json::json!({
                "operation": "capability.generic.dispatch@1",
                "facet_id": "relay-1",
                "facet_generation": 1,
                "timeout_ms": 1000,
                "idempotency_key": "0123456789abcdef0123456789abcdef"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<InteractionRequest>(serde_json::json!({
                "operation": "capability.on-off.toggle@1",
                "facet_id": "relay-1",
                "facet_generation": 1,
                "timeout_ms": 1000,
                "idempotency_key": "0123456789abcdef0123456789abcdef",
                "unexpected": true
            }))
            .is_err()
        );
    }

    #[test]
    fn schemars_interaction_union_has_only_authoritative_operation_tags() {
        fn collect_operation_consts<'a>(value: &'a Value, found: &mut BTreeSet<&'a str>) {
            if let Some(operation) = value
                .get("properties")
                .and_then(|properties| properties.get("operation"))
                .and_then(|operation| operation.get("const"))
                .and_then(Value::as_str)
            {
                found.insert(operation);
            }
            match value {
                Value::Array(values) => {
                    for value in values {
                        collect_operation_consts(value, found);
                    }
                }
                Value::Object(values) => {
                    for value in values.values() {
                        collect_operation_consts(value, found);
                    }
                }
                _ => {}
            }
        }

        let generated = serde_json::to_value(schemars::schema_for!(InteractionRequest)).unwrap();
        let mut actual = BTreeSet::new();
        collect_operation_consts(&generated, &mut actual);
        assert_eq!(actual, CAPABILITY_OPERATION_IDS.into_iter().collect());
    }

    #[test]
    fn unsupported_host_projection_is_safe_and_never_retried() {
        let error = unsupported_projection(
            "capability.on-off.toggle@1",
            "provider component ABI does not expose On/Off toggle",
        );
        assert_eq!(error.inner.code, ErrorCode::Unsupported);
        assert_eq!(error.inner.origin, Some(ErrorOrigin::Host));
        assert_eq!(error.retry.kind, RetryKind::Never);
        assert_eq!(error.outcome.confidence, OutcomeConfidence::Definite);
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
