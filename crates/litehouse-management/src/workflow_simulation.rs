use litehouse_contracts::quantity::Quantity;
use litehouse_rule_json::{
    BooleanOccurrence, CapabilityOperation, CapabilitySnapshot, ColorModel, CompiledNode,
    CompiledRule, CompiledTrigger, CoveringMovement, DecimalValue, DeduplicationGuarantee,
    DesiredProjectionPath, DesiredProjectionValue, EffectBinding, EvaluationErrorCode,
    EvaluationEvent, EvaluationIdentity, EventPayload, Freshness, MissingReason,
    NativeRepresentation, ObservationDelivery, OperationKind, PlannerEvaluationInput, PlannerState,
    Quantization, ReceiptConfidence, ReceiptRequirement, RetryPolicy, RoundingMode,
    RuntimeColorValue, RuntimeDuration, RuntimeValue, Selector, SnapshotField, StateFieldRef,
    ThermostatAction, ThermostatMode, ThermostatModeValue, ThermostatTargets, Timestamp,
    TransitionOutcome, TriggerInput, UnknownReason, ValueState, plan_microstep,
    schedule_occurrence_id,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

const SIMULATION_PROCESS_EPOCH: &str = "00000000000000000000000000000001";
const MAX_SIMULATION_SNAPSHOTS: usize = 128;
const MAX_SIMULATION_SNAPSHOT_FIELDS: usize = 64;
const MAX_SIMULATION_EFFECT_BINDINGS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationInput {
    pub evaluation_time: WorkflowSimulationTimestamp,
    pub trigger: WorkflowSimulationTrigger,
    #[schemars(length(max = 128))]
    pub snapshots: Vec<WorkflowSimulationSnapshot>,
    #[schemars(length(max = 32))]
    pub effect_bindings: Vec<WorkflowSimulationEffectBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkflowSimulationTrigger {
    StateObservation {
        provider_generation: u64,
        facet_revision: u64,
        observed_at: WorkflowSimulationTimestamp,
        delivery: WorkflowSimulationObservationDelivery,
    },
    BooleanOccurrence {
        #[schemars(length(min = 1, max = 128))]
        event_id: String,
        #[schemars(length(min = 1, max = 128))]
        source_id: String,
        generation: u64,
        observed_at: WorkflowSimulationTimestamp,
        value: Option<bool>,
    },
    TimeSchedule {
        activation_anchor: WorkflowSimulationTimestamp,
        scheduled_for: WorkflowSimulationTimestamp,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationObservationDelivery {
    InitialHydration,
    AnyObservation,
    ValueChanged,
    ValueUnchanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationSnapshot {
    pub selector: Selector,
    pub provider_generation: u64,
    pub facet_revision: u64,
    pub source_time: Option<WorkflowSimulationTimestamp>,
    pub received_at: WorkflowSimulationTimestamp,
    pub freshness: WorkflowSimulationFreshness,
    #[schemars(length(min = 1, max = 64))]
    pub fields: Vec<WorkflowSimulationSnapshotField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationFreshness {
    Fresh,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationSnapshotField {
    pub field: StateFieldRef,
    pub state: WorkflowSimulationValueState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkflowSimulationValueState {
    Known {
        value: WorkflowSimulationValue,
    },
    Missing {
        reason: WorkflowSimulationMissingReason,
    },
    Unknown {
        reason: WorkflowSimulationUnknownReason,
    },
    Error {
        code: WorkflowSimulationErrorCode,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationMissingReason {
    NotReported,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationUnknownReason {
    Unavailable,
    Stale,
    Gap,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationErrorCode {
    SourceError,
    InvalidValue,
    TypeMismatch,
    QuantityError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkflowSimulationValue {
    Bool {
        value: bool,
    },
    String {
        #[schemars(length(max = 128))]
        value: String,
    },
    I64 {
        value: i64,
    },
    U64 {
        value: u64,
    },
    Timestamp {
        value: WorkflowSimulationTimestamp,
    },
    Duration {
        milliseconds: u64,
    },
    Quantity {
        value: WorkflowSimulationQuantity,
    },
    ColorValue {
        value: WorkflowSimulationColorValue,
    },
    ColorModel {
        value: WorkflowSimulationColorModel,
    },
    ThermostatMode {
        value: WorkflowSimulationThermostatMode,
    },
    ThermostatAction {
        value: WorkflowSimulationThermostatAction,
    },
    ThermostatTargets {
        value: WorkflowSimulationThermostatTargets,
    },
    CoveringMovement {
        value: WorkflowSimulationCoveringMovement,
    },
    NativeRepresentation {
        value: WorkflowSimulationNativeRepresentation,
    },
    StringList {
        #[schemars(length(max = 64))]
        value: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationQuantity {
    #[schemars(length(min = 1, max = 128))]
    pub profile: String,
    #[schemars(length(min = 1, max = 128))]
    pub magnitude: String,
    #[schemars(length(min = 1, max = 64))]
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "model", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkflowSimulationColorValue {
    HueSaturation {
        hue_degrees: DecimalValue,
        saturation_ratio: DecimalValue,
    },
    CieXy {
        x: DecimalValue,
        y: DecimalValue,
    },
    ColorTemperature {
        kelvin: DecimalValue,
    },
    CalibratedRgb {
        red: DecimalValue,
        green: DecimalValue,
        blue: DecimalValue,
        #[schemars(length(min = 1, max = 64))]
        color_space: String,
    },
    UncalibratedRgb {
        red: DecimalValue,
        green: DecimalValue,
        blue: DecimalValue,
    },
    NativeWhiteChannels {
        white: Option<DecimalValue>,
        cold_white: Option<DecimalValue>,
        warm_white: Option<DecimalValue>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationColorModel {
    HueSaturation,
    CieXy,
    ColorTemperature,
    CalibratedRgb,
    UncalibratedRgb,
    NativeWhiteChannels,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkflowSimulationThermostatMode {
    Known {
        value: ThermostatMode,
    },
    ProviderQualified {
        #[schemars(length(min = 1, max = 64))]
        value: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationThermostatAction {
    Idle,
    Heating,
    Cooling,
    Drying,
    Fan,
    Defrosting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum WorkflowSimulationThermostatTargets {
    Single {
        temperature: WorkflowSimulationQuantity,
    },
    HeatCool {
        heat: WorkflowSimulationQuantity,
        cool: WorkflowSimulationQuantity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationCoveringMovement {
    Opening,
    Closing,
    Stopped,
    Idle,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationNativeRepresentation {
    #[schemars(length(min = 1, max = 128))]
    pub magnitude: String,
    #[schemars(length(min = 1, max = 64))]
    pub unit: String,
    pub quantization_scale: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationEffectBinding {
    #[schemars(length(min = 1, max = 128))]
    pub node_id: String,
    pub provider_generation: u64,
    pub facet_revision: u64,
    pub grant_revision: u64,
    pub operation_kind: WorkflowSimulationOperationKind,
    pub deduplication: WorkflowSimulationDeduplication,
    #[schemars(length(min = 1, max = 4))]
    pub receipt_support: Vec<ReceiptConfidence>,
    pub default_deadline_ms: u32,
    pub maximum_deadline_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationOperationKind {
    AbsoluteConvergent,
    Relative,
    Continuous,
    Momentary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationDeduplication {
    None,
    ProviderGeneration,
    Durable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationTimestamp {
    pub unix_seconds: i64,
    #[schemars(range(max = 999_999_999))]
    pub nanosecond: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationResult {
    pub simulated: bool,
    pub bindings_hypothetical: bool,
    #[schemars(regex(pattern = "^[0-9a-f]{32}$"))]
    pub loaded_rule_id: String,
    pub loaded_rule_revision: u64,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub loaded_source_sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub loaded_semantic_sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub loaded_profile_sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub plan_sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub next_state_sha256: String,
    pub outcome: WorkflowSimulationOutcome,
    #[schemars(length(max = 32))]
    pub effects: Vec<WorkflowSimulationEffect>,
    #[schemars(length(max = 32))]
    pub timers: Vec<WorkflowSimulationTimer>,
    #[schemars(length(max = 128))]
    pub desired: Vec<WorkflowSimulationDesiredField>,
    pub assumption_count: u32,
    #[schemars(length(max = 128))]
    pub trace_node_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowSimulationOutcome {
    Suspended,
    Success,
    Noop,
    Unknown,
    Error,
    Cancelled,
    CapacityRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationEffect {
    pub intent_id: String,
    pub node_id: String,
    pub selector: Selector,
    pub operation: CapabilityOperation,
    pub required_receipt: ReceiptRequirement,
    pub required_confidence: ReceiptConfidence,
    pub retry: RetryPolicy,
    pub issued_at: WorkflowSimulationTimestamp,
    pub deadline: WorkflowSimulationTimestamp,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationTimer {
    pub timer_id: String,
    pub continuation_node_id: String,
    pub scheduled_at: WorkflowSimulationTimestamp,
    pub fire_at: WorkflowSimulationTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSimulationDesiredField {
    pub selector: Selector,
    pub path: DesiredProjectionPath,
    pub value: Option<DesiredProjectionValue>,
}

#[derive(Debug, Error)]
pub enum WorkflowSimulationError {
    #[error("simulation trigger does not match the compiled workflow trigger")]
    TriggerMismatch,
    #[error("simulation collection exceeds its bound")]
    CollectionBound,
    #[error("simulation references an unknown or non-effect node")]
    InvalidEffectBinding,
    #[error("simulation input is invalid: {0}")]
    InvalidInput(String),
    #[error("simulation planner rejected the input: {0}")]
    Planner(String),
    #[error("simulation result could not be encoded")]
    Encoding,
}

pub fn simulate_initial_microstep(
    compiled: &CompiledRule,
    loaded_source_sha256: String,
    input: &WorkflowSimulationInput,
) -> Result<WorkflowSimulationResult, WorkflowSimulationError> {
    let encoded_input = serde_json::to_vec(input).map_err(|_| WorkflowSimulationError::Encoding)?;
    if encoded_input.len() > compiled.limits().max_plan_input_bytes as usize
        || input.snapshots.len() > MAX_SIMULATION_SNAPSHOTS
        || input.effect_bindings.len() > MAX_SIMULATION_EFFECT_BINDINGS
        || input
            .snapshots
            .iter()
            .any(|snapshot| snapshot.fields.len() > MAX_SIMULATION_SNAPSHOT_FIELDS)
    {
        return Err(WorkflowSimulationError::CollectionBound);
    }
    let evaluation_time = timestamp(input.evaluation_time)?;
    let trigger = simulation_trigger(compiled, &input.trigger)?;
    let snapshots = input
        .snapshots
        .iter()
        .map(simulation_snapshot)
        .collect::<Result<Vec<_>, _>>()?;
    let bindings = input
        .effect_bindings
        .iter()
        .map(|binding| simulation_binding(compiled, binding))
        .collect::<Result<Vec<_>, _>>()?;
    let run_id = hex_prefix(Sha256::digest(&encoded_input).as_slice(), 16);
    let identity = EvaluationIdentity::new(
        SIMULATION_PROCESS_EPOCH,
        compiled.rule_id(),
        compiled.revision(),
        compiled.semantic_sha256(),
        compiled.profile_sha256(),
        compiled.artifacts().clone(),
        run_id,
        1,
    )
    .map_err(invalid_input)?;
    let state =
        PlannerState::initial(1, SIMULATION_PROCESS_EPOCH, Vec::new()).map_err(invalid_input)?;
    let planner_input = PlannerEvaluationInput::new(
        identity,
        EvaluationEvent::initial(trigger),
        evaluation_time,
        snapshots,
        bindings,
        state,
    )
    .map_err(invalid_input)?;
    let plan = plan_microstep(compiled, &planner_input)
        .map_err(|error| WorkflowSimulationError::Planner(error.to_string()))?;
    project_plan(compiled, loaded_source_sha256, &plan)
}

fn simulation_trigger(
    compiled: &CompiledRule,
    trigger: &WorkflowSimulationTrigger,
) -> Result<TriggerInput, WorkflowSimulationError> {
    match (compiled.trigger(), trigger) {
        (
            CompiledTrigger::StateObservation { selector, .. },
            WorkflowSimulationTrigger::StateObservation {
                provider_generation,
                facet_revision,
                observed_at,
                delivery,
            },
        ) => TriggerInput::state_observation(
            selector.clone(),
            *provider_generation,
            *facet_revision,
            timestamp(*observed_at)?,
            match delivery {
                WorkflowSimulationObservationDelivery::InitialHydration => {
                    ObservationDelivery::InitialHydration
                }
                WorkflowSimulationObservationDelivery::AnyObservation => {
                    ObservationDelivery::AnyObservation
                }
                WorkflowSimulationObservationDelivery::ValueChanged => {
                    ObservationDelivery::ValueChanged
                }
                WorkflowSimulationObservationDelivery::ValueUnchanged => {
                    ObservationDelivery::ValueUnchanged
                }
            },
        )
        .map_err(invalid_input),
        (
            CompiledTrigger::DiscreteEvent {
                selector, event, ..
            },
            WorkflowSimulationTrigger::BooleanOccurrence {
                event_id,
                source_id,
                generation,
                observed_at,
                value,
            },
        ) => {
            let observed_at = timestamp(*observed_at)?;
            let occurrence = BooleanOccurrence::new(
                event_id.clone(),
                source_id.clone(),
                *generation,
                observed_at,
                *value,
            )
            .map_err(invalid_input)?;
            TriggerInput::discrete_event(
                selector.clone(),
                event.clone(),
                EventPayload::BooleanOccurrence { occurrence },
                observed_at,
            )
            .map_err(invalid_input)
        }
        (
            CompiledTrigger::TimeSchedule { .. },
            WorkflowSimulationTrigger::TimeSchedule {
                activation_anchor,
                scheduled_for,
            },
        ) => {
            let activation_anchor = timestamp(*activation_anchor)?;
            let scheduled_for = timestamp(*scheduled_for)?;
            TriggerInput::time_schedule(
                schedule_occurrence_id(compiled, activation_anchor, scheduled_for),
                activation_anchor,
                scheduled_for,
            )
            .map_err(invalid_input)
        }
        _ => Err(WorkflowSimulationError::TriggerMismatch),
    }
}

fn simulation_snapshot(
    snapshot: &WorkflowSimulationSnapshot,
) -> Result<CapabilitySnapshot, WorkflowSimulationError> {
    let fields = snapshot
        .fields
        .iter()
        .map(|field| {
            SnapshotField::new(field.field.clone(), value_state(&field.state)?)
                .map_err(invalid_input)
        })
        .collect::<Result<Vec<_>, _>>()?;
    CapabilitySnapshot::new(
        snapshot.selector.clone(),
        snapshot.provider_generation,
        snapshot.facet_revision,
        snapshot.source_time.map(timestamp).transpose()?,
        timestamp(snapshot.received_at)?,
        match snapshot.freshness {
            WorkflowSimulationFreshness::Fresh => Freshness::Fresh,
            WorkflowSimulationFreshness::Stale => Freshness::Stale,
        },
        fields,
    )
    .map_err(invalid_input)
}

fn simulation_binding(
    compiled: &CompiledRule,
    binding: &WorkflowSimulationEffectBinding,
) -> Result<EffectBinding, WorkflowSimulationError> {
    let Some(CompiledNode::EffectIntent {
        selector,
        operation,
        ..
    }) = compiled.nodes().get(&binding.node_id)
    else {
        return Err(WorkflowSimulationError::InvalidEffectBinding);
    };
    EffectBinding::new(
        binding.node_id.clone(),
        selector.clone(),
        operation.kind(),
        binding.provider_generation,
        binding.facet_revision,
        binding.grant_revision,
        match binding.operation_kind {
            WorkflowSimulationOperationKind::AbsoluteConvergent => {
                OperationKind::AbsoluteConvergent
            }
            WorkflowSimulationOperationKind::Relative => OperationKind::Relative,
            WorkflowSimulationOperationKind::Continuous => OperationKind::Continuous,
            WorkflowSimulationOperationKind::Momentary => OperationKind::Momentary,
        },
        match binding.deduplication {
            WorkflowSimulationDeduplication::None => DeduplicationGuarantee::None,
            WorkflowSimulationDeduplication::ProviderGeneration => {
                DeduplicationGuarantee::ProviderGeneration
            }
            WorkflowSimulationDeduplication::Durable => DeduplicationGuarantee::Durable,
        },
        binding.receipt_support.clone(),
        binding.default_deadline_ms,
        binding.maximum_deadline_ms,
    )
    .map_err(invalid_input)
}

fn value_state(
    state: &WorkflowSimulationValueState,
) -> Result<ValueState, WorkflowSimulationError> {
    Ok(match state {
        WorkflowSimulationValueState::Known { value } => ValueState::Known {
            value: runtime_value(value)?,
        },
        WorkflowSimulationValueState::Missing { reason } => ValueState::Missing {
            reason: match reason {
                WorkflowSimulationMissingReason::NotReported => MissingReason::NotReported,
                WorkflowSimulationMissingReason::NotApplicable => MissingReason::NotApplicable,
            },
        },
        WorkflowSimulationValueState::Unknown { reason } => ValueState::Unknown {
            reason: match reason {
                WorkflowSimulationUnknownReason::Unavailable => UnknownReason::Unavailable,
                WorkflowSimulationUnknownReason::Stale => UnknownReason::Stale,
                WorkflowSimulationUnknownReason::Gap => UnknownReason::Gap,
                WorkflowSimulationUnknownReason::Unsupported => UnknownReason::Unsupported,
            },
        },
        WorkflowSimulationValueState::Error { code } => ValueState::Error {
            code: match code {
                WorkflowSimulationErrorCode::SourceError => EvaluationErrorCode::SourceError,
                WorkflowSimulationErrorCode::InvalidValue => EvaluationErrorCode::InvalidValue,
                WorkflowSimulationErrorCode::TypeMismatch => EvaluationErrorCode::TypeMismatch,
                WorkflowSimulationErrorCode::QuantityError => EvaluationErrorCode::QuantityError,
            },
        },
    })
}

fn runtime_value(value: &WorkflowSimulationValue) -> Result<RuntimeValue, WorkflowSimulationError> {
    Ok(match value {
        WorkflowSimulationValue::Bool { value } => RuntimeValue::Bool(*value),
        WorkflowSimulationValue::String { value } => {
            ensure_string_bound(value, 128, "string value")?;
            RuntimeValue::String(value.clone())
        }
        WorkflowSimulationValue::I64 { value } => RuntimeValue::I64(*value),
        WorkflowSimulationValue::U64 { value } => RuntimeValue::U64(*value),
        WorkflowSimulationValue::Timestamp { value } => RuntimeValue::Timestamp(timestamp(*value)?),
        WorkflowSimulationValue::Duration { milliseconds } => {
            RuntimeValue::Duration(RuntimeDuration {
                milliseconds: *milliseconds,
            })
        }
        WorkflowSimulationValue::Quantity { value } => RuntimeValue::Quantity(quantity(value)?),
        WorkflowSimulationValue::ColorValue { value } => {
            RuntimeValue::ColorValue(runtime_color(value)?)
        }
        WorkflowSimulationValue::ColorModel { value } => RuntimeValue::ColorModel(match value {
            WorkflowSimulationColorModel::HueSaturation => ColorModel::HueSaturation,
            WorkflowSimulationColorModel::CieXy => ColorModel::CieXy,
            WorkflowSimulationColorModel::ColorTemperature => ColorModel::ColorTemperature,
            WorkflowSimulationColorModel::CalibratedRgb => ColorModel::CalibratedRgb,
            WorkflowSimulationColorModel::UncalibratedRgb => ColorModel::UncalibratedRgb,
            WorkflowSimulationColorModel::NativeWhiteChannels => ColorModel::NativeWhiteChannels,
        }),
        WorkflowSimulationValue::ThermostatMode { value } => {
            RuntimeValue::ThermostatMode(match value {
                WorkflowSimulationThermostatMode::Known { value } => {
                    ThermostatModeValue::Known(*value)
                }
                WorkflowSimulationThermostatMode::ProviderQualified { value } => {
                    ensure_nonempty_string_bound(value, 64, "provider-qualified thermostat mode")?;
                    ThermostatModeValue::ProviderQualified(value.clone())
                }
            })
        }
        WorkflowSimulationValue::ThermostatAction { value } => {
            RuntimeValue::ThermostatAction(match value {
                WorkflowSimulationThermostatAction::Idle => ThermostatAction::Idle,
                WorkflowSimulationThermostatAction::Heating => ThermostatAction::Heating,
                WorkflowSimulationThermostatAction::Cooling => ThermostatAction::Cooling,
                WorkflowSimulationThermostatAction::Drying => ThermostatAction::Drying,
                WorkflowSimulationThermostatAction::Fan => ThermostatAction::Fan,
                WorkflowSimulationThermostatAction::Defrosting => ThermostatAction::Defrosting,
            })
        }
        WorkflowSimulationValue::ThermostatTargets { value } => {
            RuntimeValue::ThermostatTargets(match value {
                WorkflowSimulationThermostatTargets::Single { temperature } => {
                    ThermostatTargets::Single {
                        temperature: quantity(temperature)?,
                    }
                }
                WorkflowSimulationThermostatTargets::HeatCool { heat, cool } => {
                    ThermostatTargets::HeatCool {
                        heat: quantity(heat)?,
                        cool: quantity(cool)?,
                    }
                }
            })
        }
        WorkflowSimulationValue::CoveringMovement { value } => {
            RuntimeValue::CoveringMovement(match value {
                WorkflowSimulationCoveringMovement::Opening => CoveringMovement::Opening,
                WorkflowSimulationCoveringMovement::Closing => CoveringMovement::Closing,
                WorkflowSimulationCoveringMovement::Stopped => CoveringMovement::Stopped,
                WorkflowSimulationCoveringMovement::Idle => CoveringMovement::Idle,
                WorkflowSimulationCoveringMovement::Unknown => CoveringMovement::Unknown,
            })
        }
        WorkflowSimulationValue::NativeRepresentation { value } => {
            ensure_nonempty_string_bound(&value.magnitude, 128, "native magnitude")?;
            ensure_nonempty_string_bound(&value.unit, 64, "native unit")?;
            RuntimeValue::NativeRepresentation(NativeRepresentation {
                magnitude: value.magnitude.clone(),
                unit: value.unit.clone(),
                quantization: value.quantization_scale.map(|scale| Quantization {
                    scale,
                    mode: RoundingMode::HalfEven,
                }),
            })
        }
        WorkflowSimulationValue::StringList { value } => {
            if value.len() > 64 {
                return Err(WorkflowSimulationError::CollectionBound);
            }
            for item in value {
                ensure_string_bound(item, 128, "string-list item")?;
            }
            RuntimeValue::StringList(value.clone())
        }
    })
}

fn runtime_color(
    value: &WorkflowSimulationColorValue,
) -> Result<RuntimeColorValue, WorkflowSimulationError> {
    Ok(match value {
        WorkflowSimulationColorValue::HueSaturation {
            hue_degrees,
            saturation_ratio,
        } => RuntimeColorValue::HueSaturation {
            hue_degrees: hue_degrees.clone(),
            saturation_ratio: saturation_ratio.clone(),
        },
        WorkflowSimulationColorValue::CieXy { x, y } => RuntimeColorValue::CieXy {
            x: x.clone(),
            y: y.clone(),
        },
        WorkflowSimulationColorValue::ColorTemperature { kelvin } => {
            RuntimeColorValue::ColorTemperature {
                kelvin: kelvin.clone(),
            }
        }
        WorkflowSimulationColorValue::CalibratedRgb {
            red,
            green,
            blue,
            color_space,
        } => {
            ensure_nonempty_string_bound(color_space, 64, "calibrated colour space")?;
            RuntimeColorValue::CalibratedRgb {
                red: red.clone(),
                green: green.clone(),
                blue: blue.clone(),
                color_space: color_space.clone(),
            }
        }
        WorkflowSimulationColorValue::UncalibratedRgb { red, green, blue } => {
            RuntimeColorValue::UncalibratedRgb {
                red: red.clone(),
                green: green.clone(),
                blue: blue.clone(),
            }
        }
        WorkflowSimulationColorValue::NativeWhiteChannels {
            white,
            cold_white,
            warm_white,
        } => RuntimeColorValue::NativeWhiteChannels {
            white: white.clone(),
            cold_white: cold_white.clone(),
            warm_white: warm_white.clone(),
        },
    })
}

fn quantity(value: &WorkflowSimulationQuantity) -> Result<Quantity, WorkflowSimulationError> {
    ensure_nonempty_string_bound(&value.profile, 128, "quantity profile")?;
    ensure_nonempty_string_bound(&value.magnitude, 128, "quantity magnitude")?;
    ensure_nonempty_string_bound(&value.unit, 64, "quantity unit")?;
    Quantity::parse(&value.profile, &value.magnitude, &value.unit)
        .map_err(|error| WorkflowSimulationError::InvalidInput(error.to_string()))
}

fn timestamp(value: WorkflowSimulationTimestamp) -> Result<Timestamp, WorkflowSimulationError> {
    Timestamp::new(value.unix_seconds, value.nanosecond).map_err(invalid_input)
}

fn output_timestamp(value: Timestamp) -> WorkflowSimulationTimestamp {
    WorkflowSimulationTimestamp {
        unix_seconds: value.unix_seconds(),
        nanosecond: value.nanosecond(),
    }
}

fn project_plan(
    compiled: &CompiledRule,
    loaded_source_sha256: String,
    plan: &litehouse_rule_json::TransitionPlan,
) -> Result<WorkflowSimulationResult, WorkflowSimulationError> {
    let plan_bytes = serde_json::to_vec(plan).map_err(|_| WorkflowSimulationError::Encoding)?;
    let state_bytes =
        serde_json::to_vec(plan.next_state()).map_err(|_| WorkflowSimulationError::Encoding)?;
    let effects = plan
        .effects()
        .iter()
        .map(|effect| WorkflowSimulationEffect {
            intent_id: effect.intent_id.clone(),
            node_id: effect.binding.node_id.clone(),
            selector: effect.binding.selector.clone(),
            operation: effect.operation.clone(),
            required_receipt: effect.required_receipt,
            required_confidence: effect.required_confidence,
            retry: effect.retry.clone(),
            issued_at: output_timestamp(effect.issued_at),
            deadline: output_timestamp(effect.deadline),
            idempotency_key: effect.idempotency_key.clone(),
        })
        .collect();
    let timers = plan
        .timers()
        .iter()
        .map(|timer| WorkflowSimulationTimer {
            timer_id: timer.timer_id.clone(),
            continuation_node_id: timer.continuation_node_id.clone(),
            scheduled_at: output_timestamp(timer.scheduled_at),
            fire_at: output_timestamp(timer.fire_at),
        })
        .collect();
    let desired = plan
        .desired()
        .map(|projection| {
            projection
                .fields
                .iter()
                .map(|field| WorkflowSimulationDesiredField {
                    selector: field.selector.clone(),
                    path: field.path,
                    value: field.value.clone(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(WorkflowSimulationResult {
        simulated: true,
        bindings_hypothetical: true,
        loaded_rule_id: compiled.rule_id().to_owned(),
        loaded_rule_revision: compiled.revision(),
        loaded_source_sha256,
        loaded_semantic_sha256: compiled.semantic_sha256().to_owned(),
        loaded_profile_sha256: compiled.profile_sha256().to_owned(),
        plan_sha256: hex_digest(Sha256::digest(plan_bytes)),
        next_state_sha256: hex_digest(Sha256::digest(state_bytes)),
        outcome: match plan.outcome() {
            TransitionOutcome::Suspended => WorkflowSimulationOutcome::Suspended,
            TransitionOutcome::Success => WorkflowSimulationOutcome::Success,
            TransitionOutcome::Noop => WorkflowSimulationOutcome::Noop,
            TransitionOutcome::Unknown => WorkflowSimulationOutcome::Unknown,
            TransitionOutcome::Error => WorkflowSimulationOutcome::Error,
            TransitionOutcome::Cancelled => WorkflowSimulationOutcome::Cancelled,
            TransitionOutcome::CapacityRejected => WorkflowSimulationOutcome::CapacityRejected,
        },
        effects,
        timers,
        desired,
        assumption_count: plan.assumptions().len().try_into().unwrap_or(u32::MAX),
        trace_node_ids: plan.trace_node_ids().to_vec(),
    })
}

fn ensure_string_bound(
    value: &str,
    maximum: usize,
    field: &'static str,
) -> Result<(), WorkflowSimulationError> {
    if value.len() > maximum {
        return Err(WorkflowSimulationError::InvalidInput(format!(
            "{field} exceeds {maximum} bytes"
        )));
    }
    Ok(())
}

fn ensure_nonempty_string_bound(
    value: &str,
    maximum: usize,
    field: &'static str,
) -> Result<(), WorkflowSimulationError> {
    if value.is_empty() {
        return Err(WorkflowSimulationError::InvalidInput(format!(
            "{field} must not be empty"
        )));
    }
    ensure_string_bound(value, maximum, field)
}

fn invalid_input(error: impl ToString) -> WorkflowSimulationError {
    WorkflowSimulationError::InvalidInput(error.to_string())
}

fn hex_prefix(bytes: &[u8], take: usize) -> String {
    bytes
        .iter()
        .take(take)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
