use std::collections::BTreeSet;

use litehouse_contracts::quantity::Quantity;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    CompiledArtifacts,
    model::{
        CapabilityOperation, CapabilityOperationKind, ContractEventRef, DecimalValue,
        DesiredProjectionPath, DesiredProjectionValue, HARD_MAX_EFFECT_INTENTS, HARD_MAX_NODES,
        HARD_MAX_PARALLEL_BRANCHES, HARD_MAX_PATCH_OPERATIONS, HARD_MAX_PLAN_INPUT_BYTES,
        HARD_MAX_QUEUED_TRIGGERS, HARD_MAX_STRING_BYTES, HARD_MAX_TIMER_DURATION_MS,
        HARD_MAX_TIMERS_PER_RULE, RebindPolicy, ReceiptConfidence, ReceiptOutcome,
        ReceiptRequirement, RetryPolicy, Selector, SelectorCardinality, StateFieldRef,
        ThermostatMode, valid_contract_part, valid_decimal_shape, valid_hex_32, valid_hex_64,
        valid_id,
    },
};

const MAX_SNAPSHOT_FIELDS: usize = 64;
const MAX_RECEIPT_CONFIDENCES: usize = 4;
const MAX_STATUS_ITEMS: usize = 64;
const MAX_EFFECT_DEADLINE_MS: u32 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Timestamp {
    unix_seconds: i64,
    nanosecond: u32,
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawTimestamp {
            unix_seconds: i64,
            nanosecond: u32,
        }

        let raw = RawTimestamp::deserialize(deserializer)?;
        Self::new(raw.unix_seconds, raw.nanosecond).map_err(serde::de::Error::custom)
    }
}

impl Timestamp {
    pub fn new(unix_seconds: i64, nanosecond: u32) -> Result<Self, PlannerModelError> {
        if nanosecond >= 1_000_000_000 {
            return Err(PlannerModelError::InvalidTimestamp);
        }
        Ok(Self {
            unix_seconds,
            nanosecond,
        })
    }

    pub const fn unix_seconds(self) -> i64 {
        self.unix_seconds
    }

    pub const fn nanosecond(self) -> u32 {
        self.nanosecond
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Freshness {
    Fresh,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum UnknownReason {
    Unavailable,
    Stale,
    Gap,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum EvaluationErrorCode {
    SourceError,
    InvalidValue,
    TypeMismatch,
    QuantityError,
}

/// A closed runtime value. Its variants cover every value produced by
/// [`StateFieldRef`] without admitting an untyped JSON escape hatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeValue {
    Bool(bool),
    String(String),
    I64(i64),
    U64(u64),
    Timestamp(Timestamp),
    Duration(RuntimeDuration),
    Quantity(Quantity),
    ColorValue(RuntimeColorValue),
    ColorModel(ColorModel),
    ThermostatMode(ThermostatModeValue),
    ThermostatAction(ThermostatAction),
    ThermostatTargets(ThermostatTargets),
    CoveringMovement(CoveringMovement),
    NativeRepresentation(NativeRepresentation),
    StringList(Vec<String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeDuration {
    pub milliseconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "kebab-case", deny_unknown_fields)]
pub enum RuntimeColorValue {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum ColorModel {
    HueSaturation,
    CieXy,
    ColorTemperature,
    CalibratedRgb,
    UncalibratedRgb,
    NativeWhiteChannels,
}

/// The thermostat WIT mode is wider than the authored operation enum because a
/// provider may expose a provider-qualified mode in observed state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ThermostatModeValue {
    Known(ThermostatMode),
    ProviderQualified(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ThermostatAction {
    Idle,
    Heating,
    Cooling,
    Drying,
    Fan,
    Defrosting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ThermostatTargets {
    Single { temperature: Quantity },
    HeatCool { heat: Quantity, cool: Quantity },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CoveringMovement {
    Opening,
    Closing,
    Stopped,
    Idle,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RoundingMode {
    HalfEven,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quantization {
    pub scale: u32,
    pub mode: RoundingMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeRepresentation {
    pub magnitude: String,
    pub unit: String,
    pub quantization: Option<Quantization>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MissingReason {
    NotReported,
    NotApplicable,
}

/// Known, absent, indeterminate, and failed values are deliberately distinct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValueState {
    Known { value: RuntimeValue },
    Missing { reason: MissingReason },
    Unknown { reason: UnknownReason },
    Error { code: EvaluationErrorCode },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PredicateOutcome {
    True,
    False,
    Unknown { reason: UnknownReason },
    Error { code: EvaluationErrorCode },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotField {
    pub field: StateFieldRef,
    pub state: ValueState,
}

impl SnapshotField {
    pub fn new(field: StateFieldRef, state: ValueState) -> Result<Self, PlannerModelError> {
        validate_value_state(&field, &state)?;
        Ok(Self { field, state })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilitySnapshot {
    pub selector: Selector,
    pub provider_generation: u64,
    pub facet_revision: u64,
    pub source_time: Option<Timestamp>,
    pub received_at: Timestamp,
    pub freshness: Freshness,
    pub fields: Vec<SnapshotField>,
}

impl CapabilitySnapshot {
    pub fn new(
        selector: Selector,
        provider_generation: u64,
        facet_revision: u64,
        source_time: Option<Timestamp>,
        received_at: Timestamp,
        freshness: Freshness,
        fields: Vec<SnapshotField>,
    ) -> Result<Self, PlannerModelError> {
        validate_selector(&selector)?;
        require_nonzero_revision(provider_generation, "provider_generation")?;
        require_nonzero_revision(facet_revision, "facet_revision")?;
        if source_time.is_some_and(|time| time > received_at) {
            return Err(PlannerModelError::InvalidTimestampOrder);
        }
        if fields.is_empty() || fields.len() > MAX_SNAPSHOT_FIELDS {
            return Err(PlannerModelError::CollectionBound {
                collection: "snapshot fields",
                actual: fields.len(),
                maximum: MAX_SNAPSHOT_FIELDS,
            });
        }
        let mut seen = BTreeSet::new();
        for entry in &fields {
            validate_value_state(&entry.field, &entry.state)?;
            if entry.field.selector_contract() != selector.contract {
                return Err(PlannerModelError::SnapshotContractMismatch);
            }
            let key = field_identity(&entry.field);
            if !seen.insert(key.clone()) {
                return Err(PlannerModelError::DuplicateSnapshotField(key));
            }
        }
        Ok(Self {
            selector,
            provider_generation,
            facet_revision,
            source_time,
            received_at,
            freshness,
            fields,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanOccurrence {
    pub event_id: String,
    pub source_id: String,
    pub generation: u64,
    pub received_at: Timestamp,
    pub value: Option<bool>,
}

impl BooleanOccurrence {
    pub fn new(
        event_id: impl Into<String>,
        source_id: impl Into<String>,
        generation: u64,
        received_at: Timestamp,
        value: Option<bool>,
    ) -> Result<Self, PlannerModelError> {
        let event_id = event_id.into();
        let source_id = source_id.into();
        validate_id("event_id", &event_id)?;
        validate_id("source_id", &source_id)?;
        require_nonzero_revision(generation, "event generation")?;
        Ok(Self {
            event_id,
            source_id,
            generation,
            received_at,
            value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventPayload {
    StateObservation { snapshot: CapabilitySnapshot },
    BooleanOccurrence { occurrence: BooleanOccurrence },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationDelivery {
    InitialHydration,
    AnyObservation,
    ValueChanged,
    ValueUnchanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TriggerInput {
    StateObservation {
        selector: Selector,
        provider_generation: u64,
        facet_revision: u64,
        observed_at: Timestamp,
        delivery: ObservationDelivery,
    },
    DiscreteEvent {
        selector: Selector,
        event: ContractEventRef,
        payload: EventPayload,
        observed_at: Timestamp,
    },
    TimeSchedule {
        occurrence_id: String,
        activation_anchor: Timestamp,
        scheduled_for: Timestamp,
    },
}

impl TriggerInput {
    pub fn state_observation(
        selector: Selector,
        provider_generation: u64,
        facet_revision: u64,
        observed_at: Timestamp,
        delivery: ObservationDelivery,
    ) -> Result<Self, PlannerModelError> {
        validate_selector(&selector)?;
        require_nonzero_revision(provider_generation, "provider_generation")?;
        require_nonzero_revision(facet_revision, "facet_revision")?;
        Ok(Self::StateObservation {
            selector,
            provider_generation,
            facet_revision,
            observed_at,
            delivery,
        })
    }

    pub fn discrete_event(
        selector: Selector,
        event: ContractEventRef,
        payload: EventPayload,
        observed_at: Timestamp,
    ) -> Result<Self, PlannerModelError> {
        validate_selector(&selector)?;
        match &payload {
            EventPayload::StateObservation { snapshot } => {
                CapabilitySnapshot::new(
                    snapshot.selector.clone(),
                    snapshot.provider_generation,
                    snapshot.facet_revision,
                    snapshot.source_time,
                    snapshot.received_at,
                    snapshot.freshness,
                    snapshot.fields.clone(),
                )?;
            }
            EventPayload::BooleanOccurrence { occurrence } => {
                BooleanOccurrence::new(
                    occurrence.event_id.clone(),
                    occurrence.source_id.clone(),
                    occurrence.generation,
                    occurrence.received_at,
                    occurrence.value,
                )?;
            }
        }
        if selector.contract != event.selector_contract() {
            return Err(PlannerModelError::TriggerContractMismatch);
        }
        let shape_matches = match (&event, &payload) {
            (
                ContractEventRef::BooleanState {
                    event: crate::BooleanStateEventKind::Occurrence,
                },
                EventPayload::BooleanOccurrence { occurrence },
            ) => occurrence.received_at <= observed_at,
            (
                ContractEventRef::BooleanState {
                    event: crate::BooleanStateEventKind::StateObservation,
                }
                | ContractEventRef::OnOff { .. }
                | ContractEventRef::LevelControl { .. }
                | ContractEventRef::ColorControl { .. }
                | ContractEventRef::Measurement { .. }
                | ContractEventRef::Thermostat { .. }
                | ContractEventRef::WindowCovering { .. },
                EventPayload::StateObservation { snapshot },
            ) => snapshot.selector == selector && snapshot.received_at <= observed_at,
            _ => false,
        };
        if !shape_matches {
            return Err(PlannerModelError::DiscreteEventShapeMismatch);
        }
        Ok(Self::DiscreteEvent {
            selector,
            event,
            payload,
            observed_at,
        })
    }

    pub fn time_schedule(
        occurrence_id: impl Into<String>,
        activation_anchor: Timestamp,
        scheduled_for: Timestamp,
    ) -> Result<Self, PlannerModelError> {
        let occurrence_id = occurrence_id.into();
        validate_id("occurrence_id", &occurrence_id)?;
        if scheduled_for <= activation_anchor {
            return Err(PlannerModelError::InvalidTimestampOrder);
        }
        Ok(Self::TimeSchedule {
            occurrence_id,
            activation_anchor,
            scheduled_for,
        })
    }

    fn event_time(&self) -> Timestamp {
        match self {
            Self::StateObservation { observed_at, .. }
            | Self::DiscreteEvent { observed_at, .. } => *observed_at,
            Self::TimeSchedule { scheduled_for, .. } => *scheduled_for,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectReceipt {
    pub intent_id: String,
    pub request_id: String,
    pub operation: CapabilityOperation,
    pub attempt: u32,
    pub provider_generation: u64,
    pub confidence: ReceiptConfidence,
    pub outcome: ReceiptOutcome,
    pub accepted_at: Timestamp,
    pub completed_at: Option<Timestamp>,
    pub idempotency_key: Option<String>,
}

impl EffectReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        intent_id: impl Into<String>,
        request_id: impl Into<String>,
        operation: CapabilityOperation,
        attempt: u32,
        provider_generation: u64,
        confidence: ReceiptConfidence,
        outcome: ReceiptOutcome,
        accepted_at: Timestamp,
        completed_at: Option<Timestamp>,
        idempotency_key: Option<String>,
    ) -> Result<Self, PlannerModelError> {
        let intent_id = intent_id.into();
        let request_id = request_id.into();
        validate_id("intent_id", &intent_id)?;
        validate_id("request_id", &request_id)?;
        if attempt == 0 {
            return Err(PlannerModelError::ZeroRevision("attempt"));
        }
        require_nonzero_revision(provider_generation, "provider_generation")?;
        if completed_at.is_some_and(|time| time < accepted_at) {
            return Err(PlannerModelError::InvalidTimestampOrder);
        }
        if let Some(key) = &idempotency_key
            && !valid_hex_64(key)
        {
            return Err(PlannerModelError::InvalidDigest("idempotency_key"));
        }
        Ok(Self {
            intent_id,
            request_id,
            operation,
            attempt,
            provider_generation,
            confidence,
            outcome,
            accepted_at,
            completed_at,
            idempotency_key,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EvaluationEvent {
    Initial {
        trigger: TriggerInput,
    },
    Timer {
        timer_id: String,
        suspension_id: String,
        scheduled_for: Timestamp,
        fired_at: Timestamp,
    },
    EffectReceipt {
        suspension_id: String,
        receipt: EffectReceipt,
    },
}

impl EvaluationEvent {
    pub fn initial(trigger: TriggerInput) -> Self {
        Self::Initial { trigger }
    }

    pub fn timer(
        timer_id: impl Into<String>,
        suspension_id: impl Into<String>,
        scheduled_for: Timestamp,
        fired_at: Timestamp,
    ) -> Result<Self, PlannerModelError> {
        let timer_id = timer_id.into();
        let suspension_id = suspension_id.into();
        validate_id("timer_id", &timer_id)?;
        validate_id("suspension_id", &suspension_id)?;
        if fired_at < scheduled_for {
            return Err(PlannerModelError::InvalidTimestampOrder);
        }
        Ok(Self::Timer {
            timer_id,
            suspension_id,
            scheduled_for,
            fired_at,
        })
    }

    pub fn effect_receipt(
        suspension_id: impl Into<String>,
        receipt: EffectReceipt,
    ) -> Result<Self, PlannerModelError> {
        let suspension_id = suspension_id.into();
        validate_id("suspension_id", &suspension_id)?;
        Ok(Self::EffectReceipt {
            suspension_id,
            receipt,
        })
    }

    fn event_time(&self) -> Timestamp {
        match self {
            Self::Initial { trigger } => trigger.event_time(),
            Self::Timer { fired_at, .. } => *fired_at,
            Self::EffectReceipt { receipt, .. } => {
                receipt.completed_at.unwrap_or(receipt.accepted_at)
            }
        }
    }
}

/// Exact WIT `operation-kind` support advertised by a bound capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum OperationKind {
    AbsoluteConvergent,
    Relative,
    Continuous,
    Momentary,
}

/// Exact WIT `deduplication-guarantee` support advertised by a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum DeduplicationGuarantee {
    None,
    ProviderGeneration,
    Durable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectBinding {
    pub node_id: String,
    pub selector: Selector,
    pub operation: CapabilityOperationKind,
    pub provider_generation: u64,
    pub facet_revision: u64,
    pub grant_revision: u64,
    pub operation_kind: OperationKind,
    pub deduplication: DeduplicationGuarantee,
    pub receipt_support: Vec<ReceiptConfidence>,
    pub default_deadline_ms: u32,
    pub maximum_deadline_ms: u32,
}

impl EffectBinding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_id: impl Into<String>,
        selector: Selector,
        operation: CapabilityOperationKind,
        provider_generation: u64,
        facet_revision: u64,
        grant_revision: u64,
        operation_kind: OperationKind,
        deduplication: DeduplicationGuarantee,
        receipt_support: Vec<ReceiptConfidence>,
        default_deadline_ms: u32,
        maximum_deadline_ms: u32,
    ) -> Result<Self, PlannerModelError> {
        let node_id = node_id.into();
        validate_id("node_id", &node_id)?;
        validate_selector(&selector)?;
        if selector.contract != crate::ContractRef::litehouse(operation.package(), "capability", 1)
        {
            return Err(PlannerModelError::EffectContractMismatch);
        }
        require_nonzero_revision(provider_generation, "provider_generation")?;
        require_nonzero_revision(facet_revision, "facet_revision")?;
        require_nonzero_revision(grant_revision, "grant_revision")?;
        if receipt_support.is_empty() || receipt_support.len() > MAX_RECEIPT_CONFIDENCES {
            return Err(PlannerModelError::CollectionBound {
                collection: "receipt support",
                actual: receipt_support.len(),
                maximum: MAX_RECEIPT_CONFIDENCES,
            });
        }
        let mut unique_receipts = BTreeSet::new();
        for receipt in &receipt_support {
            if !unique_receipts.insert(receipt_rank(*receipt)) {
                return Err(PlannerModelError::DuplicateReceiptSupport);
            }
        }
        if default_deadline_ms == 0
            || default_deadline_ms > maximum_deadline_ms
            || maximum_deadline_ms > MAX_EFFECT_DEADLINE_MS
        {
            return Err(PlannerModelError::InvalidDeadline);
        }
        Ok(Self {
            node_id,
            selector,
            operation,
            provider_generation,
            facet_revision,
            grant_revision,
            operation_kind,
            deduplication,
            receipt_support,
            default_deadline_ms,
            maximum_deadline_ms,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationIdentity {
    pub process_epoch: String,
    pub rule_id: String,
    pub rule_revision: u64,
    pub semantic_sha256: String,
    pub profile_sha256: String,
    pub artifacts: CompiledArtifacts,
    pub run_id: String,
    pub microstep: u32,
}

impl EvaluationIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        process_epoch: impl Into<String>,
        rule_id: impl Into<String>,
        rule_revision: u64,
        semantic_sha256: impl Into<String>,
        profile_sha256: impl Into<String>,
        artifacts: CompiledArtifacts,
        run_id: impl Into<String>,
        microstep: u32,
    ) -> Result<Self, PlannerModelError> {
        let process_epoch = process_epoch.into();
        let rule_id = rule_id.into();
        let semantic_sha256 = semantic_sha256.into();
        let profile_sha256 = profile_sha256.into();
        let run_id = run_id.into();
        if !valid_hex_32(&process_epoch) {
            return Err(PlannerModelError::InvalidIdentifier("process_epoch"));
        }
        if !valid_hex_32(&rule_id) {
            return Err(PlannerModelError::InvalidIdentifier("rule_id"));
        }
        if !valid_hex_32(&run_id) {
            return Err(PlannerModelError::InvalidIdentifier("run_id"));
        }
        require_nonzero_revision(rule_revision, "rule_revision")?;
        if !valid_hex_64(&semantic_sha256) {
            return Err(PlannerModelError::InvalidDigest("semantic_sha256"));
        }
        if !valid_hex_64(&profile_sha256) {
            return Err(PlannerModelError::InvalidDigest("profile_sha256"));
        }
        for (name, digest) in [
            ("schema_sha256", artifacts.schema_sha256.as_str()),
            ("compiler_sha256", artifacts.compiler_sha256.as_str()),
            ("cel_sha256", artifacts.cel_sha256.as_str()),
            (
                "quantity_catalog_sha256",
                artifacts.quantity_catalog_sha256.as_str(),
            ),
            (
                "runtime_profile_sha256",
                artifacts.runtime_profile_sha256.as_str(),
            ),
            (
                "timezone_database_sha256",
                artifacts.timezone_database_sha256.as_str(),
            ),
            (
                "contract_catalog_sha256",
                artifacts.contract_catalog_sha256.as_str(),
            ),
            (
                "management_operations_sha256",
                artifacts.management_operations_sha256.as_str(),
            ),
            (
                "port_catalog_sha256",
                artifacts.port_catalog_sha256.as_str(),
            ),
        ] {
            if !valid_hex_64(digest) {
                return Err(PlannerModelError::InvalidDigest(name));
            }
        }
        if microstep == 0 {
            return Err(PlannerModelError::ZeroRevision("microstep"));
        }
        Ok(Self {
            process_epoch,
            rule_id,
            rule_revision,
            semantic_sha256,
            profile_sha256,
            artifacts,
            run_id,
            microstep,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedValue {
    pub(crate) node_id: String,
    pub(crate) value: SavedNodeValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum SavedNodeValue {
    Value { state: ValueState },
    Predicate { outcome: PredicateOutcome },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedFrame {
    pub(crate) branch_id: String,
    pub(crate) resume_node_id: String,
    pub(crate) trigger: TriggerInput,
    pub(crate) values: Vec<SavedValue>,
    pub(crate) desired: Option<DesiredProjection>,
    pub(crate) assumptions: Vec<RuntimeAssumption>,
    pub(crate) trace_node_ids: Vec<String>,
}

impl SavedFrame {
    pub(crate) fn new(
        branch_id: impl Into<String>,
        resume_node_id: impl Into<String>,
        trigger: TriggerInput,
        values: Vec<SavedValue>,
        desired: Option<DesiredProjection>,
        assumptions: Vec<RuntimeAssumption>,
        trace_node_ids: Vec<String>,
    ) -> Result<Self, PlannerModelError> {
        let branch_id = branch_id.into();
        let resume_node_id = resume_node_id.into();
        validate_id("branch_id", &branch_id)?;
        validate_id("resume_node_id", &resume_node_id)?;
        if values.len() > HARD_MAX_NODES {
            return Err(PlannerModelError::CollectionBound {
                collection: "saved values",
                actual: values.len(),
                maximum: HARD_MAX_NODES,
            });
        }
        let mut seen = BTreeSet::new();
        for value in &values {
            validate_id("saved value node_id", &value.node_id)?;
            if !seen.insert(value.node_id.clone()) {
                return Err(PlannerModelError::DuplicateIdentifier(
                    value.node_id.clone(),
                ));
            }
        }
        if trace_node_ids.len() > HARD_MAX_NODES {
            return Err(PlannerModelError::CollectionBound {
                collection: "saved trace node IDs",
                actual: trace_node_ids.len(),
                maximum: HARD_MAX_NODES,
            });
        }
        for node_id in &trace_node_ids {
            validate_id("saved trace node_id", node_id)?;
        }
        validate_trigger_input(&trigger)?;
        Ok(Self {
            branch_id,
            resume_node_id,
            trigger,
            values,
            desired,
            assumptions,
            trace_node_ids,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Suspension {
    Effect {
        suspension_id: String,
        intent_id: String,
        effect_node_id: String,
        operation: CapabilityOperation,
        required_confidence: ReceiptConfidence,
        binding: EffectBinding,
        idempotency_key: String,
        issued_microstep: u32,
        issued_at: Timestamp,
        continuation_node_id: String,
        frame: SavedFrame,
    },
    Timer {
        suspension_id: String,
        timer_id: String,
        continuation_node_id: String,
        issued_microstep: u32,
        issued_at: Timestamp,
        scheduled_for: Timestamp,
        frame: SavedFrame,
    },
    Admission {
        suspension_id: String,
        timer_id: String,
        node_id: String,
        key: crate::AdmissionKey,
        policy: crate::AdmissionPolicy,
        issued_microstep: u32,
        issued_at: Timestamp,
        scheduled_for: Timestamp,
        winning_trigger: Option<TriggerInput>,
        dropped_count: u64,
        coalesced_count: u64,
        window_closed: bool,
        continuation_node_id: Option<String>,
        frame: SavedFrame,
    },
}

impl Suspension {
    fn suspension_id(&self) -> &str {
        match self {
            Self::Effect { suspension_id, .. }
            | Self::Timer { suspension_id, .. }
            | Self::Admission { suspension_id, .. } => suspension_id,
        }
    }

    fn validate(&self) -> Result<(), PlannerModelError> {
        validate_id("suspension_id", self.suspension_id())?;
        match self {
            Self::Effect {
                intent_id,
                effect_node_id,
                binding,
                idempotency_key,
                issued_microstep,
                continuation_node_id,
                frame,
                ..
            } => {
                validate_id("intent_id", intent_id)?;
                validate_id("effect_node_id", effect_node_id)?;
                validate_effect_binding(binding)?;
                if *issued_microstep == 0 {
                    return Err(PlannerModelError::ZeroRevision("issued_microstep"));
                }
                if !valid_hex_64(idempotency_key) {
                    return Err(PlannerModelError::InvalidDigest("idempotency_key"));
                }
                validate_id("continuation_node_id", continuation_node_id)?;
                validate_saved_frame(frame)?;
                if frame.resume_node_id != *continuation_node_id {
                    return Err(PlannerModelError::ContinuationMismatch);
                }
                Ok(())
            }
            Self::Timer {
                timer_id,
                continuation_node_id,
                issued_microstep,
                issued_at,
                scheduled_for,
                frame,
                ..
            } => {
                validate_id("timer_id", timer_id)?;
                validate_id("continuation_node_id", continuation_node_id)?;
                if *issued_microstep == 0 {
                    return Err(PlannerModelError::ZeroRevision("issued_microstep"));
                }
                if scheduled_for <= issued_at {
                    return Err(PlannerModelError::InvalidTimestampOrder);
                }
                validate_saved_frame(frame)?;
                if frame.resume_node_id != *continuation_node_id {
                    return Err(PlannerModelError::ContinuationMismatch);
                }
                Ok(())
            }
            Self::Admission {
                timer_id,
                node_id,
                issued_microstep,
                issued_at,
                scheduled_for,
                winning_trigger,
                continuation_node_id,
                frame,
                ..
            } => {
                validate_id("timer_id", timer_id)?;
                validate_id("node_id", node_id)?;
                if let Some(continuation_node_id) = continuation_node_id {
                    validate_id("continuation_node_id", continuation_node_id)?;
                }
                if *issued_microstep == 0 {
                    return Err(PlannerModelError::ZeroRevision("issued_microstep"));
                }
                if scheduled_for <= issued_at {
                    return Err(PlannerModelError::InvalidTimestampOrder);
                }
                if let Some(trigger) = winning_trigger {
                    validate_trigger_input(trigger)?;
                }
                validate_saved_frame(frame)?;
                if continuation_node_id
                    .as_deref()
                    .is_some_and(|continuation| frame.resume_node_id != continuation)
                {
                    return Err(PlannerModelError::ContinuationMismatch);
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JoinArrival {
    pub(crate) join_node_id: String,
    pub(crate) branch_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Frontier {
    pub(crate) ready: Vec<SavedFrame>,
    pub(crate) suspended: Vec<Suspension>,
    pub(crate) join_arrivals: Vec<JoinArrival>,
}

impl Frontier {
    pub(crate) fn new(
        ready: Vec<SavedFrame>,
        suspended: Vec<Suspension>,
        join_arrivals: Vec<JoinArrival>,
    ) -> Result<Self, PlannerModelError> {
        if ready.len() + suspended.len() > HARD_MAX_PARALLEL_BRANCHES {
            return Err(PlannerModelError::CollectionBound {
                collection: "frontier branches",
                actual: ready.len() + suspended.len(),
                maximum: HARD_MAX_PARALLEL_BRANCHES,
            });
        }
        let mut branches = BTreeSet::new();
        for frame in &ready {
            if !branches.insert(frame.branch_id.clone()) {
                return Err(PlannerModelError::DuplicateIdentifier(
                    frame.branch_id.clone(),
                ));
            }
        }
        let mut suspension_ids = BTreeSet::new();
        for suspension in &suspended {
            suspension.validate()?;
            if !suspension_ids.insert(suspension.suspension_id().to_owned()) {
                return Err(PlannerModelError::DuplicateIdentifier(
                    suspension.suspension_id().to_owned(),
                ));
            }
        }
        if join_arrivals.len() > HARD_MAX_PARALLEL_BRANCHES {
            return Err(PlannerModelError::CollectionBound {
                collection: "join arrivals",
                actual: join_arrivals.len(),
                maximum: HARD_MAX_PARALLEL_BRANCHES,
            });
        }
        let mut arrivals = BTreeSet::new();
        for arrival in &join_arrivals {
            validate_id("join_node_id", &arrival.join_node_id)?;
            validate_id("branch_id", &arrival.branch_id)?;
            if !arrivals.insert((arrival.join_node_id.clone(), arrival.branch_id.clone())) {
                return Err(PlannerModelError::DuplicateJoinArrival);
            }
        }
        Ok(Self {
            ready,
            suspended,
            join_arrivals,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum RunStatus {
    Ready,
    Waiting,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RunState {
    pub(crate) run_id: String,
    pub(crate) next_microstep: u32,
    pub(crate) status: RunStatus,
    pub(crate) frontier: Frontier,
}

impl RunState {
    pub(crate) fn new(
        run_id: impl Into<String>,
        next_microstep: u32,
        status: RunStatus,
        frontier: Frontier,
    ) -> Result<Self, PlannerModelError> {
        let run_id = run_id.into();
        if !valid_hex_32(&run_id) {
            return Err(PlannerModelError::InvalidIdentifier("run_id"));
        }
        if next_microstep == 0 {
            return Err(PlannerModelError::ZeroRevision("next_microstep"));
        }
        let state = Self {
            run_id,
            next_microstep,
            status,
            frontier,
        };
        validate_run_state(&state)?;
        Ok(state)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveRunCursor {
    pub run_id: String,
    pub next_microstep: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AdmissionStatus {
    pub node_id: String,
    pub window_ends_at: Timestamp,
    pub has_winning_trigger: bool,
    pub dropped_count: u64,
    pub coalesced_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerState {
    pub(crate) version: u64,
    pub(crate) process_epoch: String,
    pub(crate) active_run: Option<RunState>,
    pub(crate) queued_triggers: Vec<TriggerInput>,
}

impl PlannerState {
    pub fn initial(
        version: u64,
        process_epoch: impl Into<String>,
        queued_triggers: Vec<TriggerInput>,
    ) -> Result<Self, PlannerModelError> {
        Self::new(version, process_epoch, None, queued_triggers)
    }

    pub(crate) fn new(
        version: u64,
        process_epoch: impl Into<String>,
        active_run: Option<RunState>,
        queued_triggers: Vec<TriggerInput>,
    ) -> Result<Self, PlannerModelError> {
        require_nonzero_revision(version, "planner state version")?;
        let process_epoch = process_epoch.into();
        if !valid_hex_32(&process_epoch) {
            return Err(PlannerModelError::InvalidIdentifier("process_epoch"));
        }
        if queued_triggers.len() > HARD_MAX_QUEUED_TRIGGERS {
            return Err(PlannerModelError::CollectionBound {
                collection: "queued triggers",
                actual: queued_triggers.len(),
                maximum: HARD_MAX_QUEUED_TRIGGERS,
            });
        }
        let state = Self {
            version,
            process_epoch,
            active_run,
            queued_triggers,
        };
        validate_planner_state(&state)?;
        Ok(state)
    }

    pub const fn version(&self) -> u64 {
        self.version
    }

    pub fn process_epoch(&self) -> &str {
        &self.process_epoch
    }

    pub const fn has_active_run(&self) -> bool {
        self.active_run.is_some()
    }

    pub fn active_cursor(&self) -> Option<ActiveRunCursor> {
        self.active_run.as_ref().map(|run| ActiveRunCursor {
            run_id: run.run_id.clone(),
            next_microstep: run.next_microstep,
        })
    }

    pub fn admission_statuses(&self) -> Vec<AdmissionStatus> {
        self.active_run
            .iter()
            .flat_map(|run| &run.frontier.suspended)
            .filter_map(|suspension| match suspension {
                Suspension::Admission {
                    node_id,
                    scheduled_for,
                    winning_trigger,
                    dropped_count,
                    coalesced_count,
                    ..
                } => Some(AdmissionStatus {
                    node_id: node_id.clone(),
                    window_ends_at: *scheduled_for,
                    has_winning_trigger: winning_trigger.is_some(),
                    dropped_count: *dropped_count,
                    coalesced_count: *coalesced_count,
                }),
                Suspension::Effect { .. } | Suspension::Timer { .. } => None,
            })
            .collect()
    }

    pub fn queued_triggers(&self) -> &[TriggerInput] {
        &self.queued_triggers
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectIntent {
    pub intent_id: String,
    pub branch_id: String,
    pub suspension_id: String,
    pub continuation_node_id: String,
    pub binding: EffectBinding,
    pub operation: CapabilityOperation,
    pub required_receipt: ReceiptRequirement,
    pub required_confidence: ReceiptConfidence,
    pub retry: RetryPolicy,
    pub issued_at: Timestamp,
    pub deadline: Timestamp,
    pub idempotency_key: String,
}

impl EffectIntent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        intent_id: impl Into<String>,
        branch_id: impl Into<String>,
        suspension_id: impl Into<String>,
        continuation_node_id: impl Into<String>,
        binding: EffectBinding,
        operation: CapabilityOperation,
        required_receipt: ReceiptRequirement,
        retry: RetryPolicy,
        issued_at: Timestamp,
        deadline: Timestamp,
        idempotency_key: impl Into<String>,
    ) -> Result<Self, PlannerModelError> {
        let intent_id = intent_id.into();
        let branch_id = branch_id.into();
        let suspension_id = suspension_id.into();
        let continuation_node_id = continuation_node_id.into();
        let idempotency_key = idempotency_key.into();
        validate_id("intent_id", &intent_id)?;
        validate_id("branch_id", &branch_id)?;
        validate_id("suspension_id", &suspension_id)?;
        validate_id("continuation_node_id", &continuation_node_id)?;
        if operation.kind() != binding.operation {
            return Err(PlannerModelError::EffectOperationMismatch);
        }
        let required_confidence = match required_receipt {
            ReceiptRequirement::AcceptedForDelivery => ReceiptConfidence::AcceptedForDelivery,
            ReceiptRequirement::NotificationAdvertisedMaximum => *binding
                .receipt_support
                .iter()
                .max_by_key(|confidence| receipt_rank(**confidence))
                .expect("validated binding has receipt support"),
        };
        if !binding.receipt_support.contains(&required_confidence) {
            return Err(PlannerModelError::UnsupportedReceipt);
        }
        if matches!(retry, RetryPolicy::DefinitelyPreDispatch { .. })
            && (binding.deduplication != DeduplicationGuarantee::ProviderGeneration
                || !operation.kind().retry_eligible())
        {
            return Err(PlannerModelError::InvalidRetry);
        }
        if deadline <= issued_at {
            return Err(PlannerModelError::InvalidTimestampOrder);
        }
        if !valid_hex_64(&idempotency_key) {
            return Err(PlannerModelError::InvalidDigest("idempotency_key"));
        }
        Ok(Self {
            intent_id,
            branch_id,
            suspension_id,
            continuation_node_id,
            binding,
            operation,
            required_receipt,
            required_confidence,
            retry,
            issued_at,
            deadline,
            idempotency_key,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimerIntent {
    pub timer_id: String,
    pub branch_id: String,
    pub suspension_id: String,
    pub continuation_node_id: String,
    pub scheduled_at: Timestamp,
    pub fire_at: Timestamp,
}

impl TimerIntent {
    pub fn new(
        timer_id: impl Into<String>,
        branch_id: impl Into<String>,
        suspension_id: impl Into<String>,
        continuation_node_id: impl Into<String>,
        scheduled_at: Timestamp,
        fire_at: Timestamp,
    ) -> Result<Self, PlannerModelError> {
        let timer_id = timer_id.into();
        let branch_id = branch_id.into();
        let suspension_id = suspension_id.into();
        let continuation_node_id = continuation_node_id.into();
        validate_id("timer_id", &timer_id)?;
        validate_id("branch_id", &branch_id)?;
        validate_id("suspension_id", &suspension_id)?;
        validate_id("continuation_node_id", &continuation_node_id)?;
        if fire_at <= scheduled_at {
            return Err(PlannerModelError::InvalidTimestampOrder);
        }
        let elapsed_nanoseconds = timestamp_nanoseconds(fire_at)
            .checked_sub(timestamp_nanoseconds(scheduled_at))
            .ok_or(PlannerModelError::InvalidTimerDuration)?;
        if elapsed_nanoseconds > i128::from(HARD_MAX_TIMER_DURATION_MS) * 1_000_000 {
            return Err(PlannerModelError::InvalidTimerDuration);
        }
        Ok(Self {
            timer_id,
            branch_id,
            suspension_id,
            continuation_node_id,
            scheduled_at,
            fire_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredField {
    pub selector: Selector,
    pub path: DesiredProjectionPath,
    pub value: Option<DesiredProjectionValue>,
}

impl DesiredField {
    pub fn new(
        selector: Selector,
        path: DesiredProjectionPath,
        value: Option<DesiredProjectionValue>,
    ) -> Result<Self, PlannerModelError> {
        validate_selector(&selector)?;
        if selector.contract.package != path.package()
            || value.as_ref().is_some_and(|value| value.path() != path)
        {
            return Err(PlannerModelError::DesiredContractMismatch);
        }
        Ok(Self {
            selector,
            path,
            value,
        })
    }

    pub const fn path(&self) -> DesiredProjectionPath {
        self.path
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DesiredProjection {
    pub fields: Vec<DesiredField>,
}

impl DesiredProjection {
    pub fn new(fields: Vec<DesiredField>) -> Result<Self, PlannerModelError> {
        if fields.len() > HARD_MAX_PATCH_OPERATIONS {
            return Err(PlannerModelError::CollectionBound {
                collection: "desired fields",
                actual: fields.len(),
                maximum: HARD_MAX_PATCH_OPERATIONS,
            });
        }
        let mut seen = BTreeSet::new();
        for field in &fields {
            DesiredField::new(field.selector.clone(), field.path, field.value.clone())?;
            let key = (
                selector_identity(&field.selector),
                desired_path_rank(field.path()),
            );
            if !seen.insert(key) {
                return Err(PlannerModelError::DuplicateDesiredField);
            }
        }
        Ok(Self { fields })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeAssumption {
    SnapshotRevision {
        selector: Selector,
        provider_generation: u64,
        facet_revision: u64,
        freshness: Freshness,
    },
    EffectBindingRevision {
        node_id: String,
        provider_generation: u64,
        facet_revision: u64,
        grant_revision: u64,
    },
    PlannerStateVersion {
        version: u64,
    },
    ExpectedValue {
        selector: Selector,
        field: StateFieldRef,
        state: ValueState,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum TransitionOutcome {
    Suspended,
    Success,
    Noop,
    Unknown,
    Error,
    Cancelled,
    CapacityRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransitionPlan {
    pub(crate) identity: EvaluationIdentity,
    pub(crate) outcome: TransitionOutcome,
    pub(crate) effects: Vec<EffectIntent>,
    pub(crate) timers: Vec<TimerIntent>,
    pub(crate) desired: Option<DesiredProjection>,
    pub(crate) next_state: PlannerState,
    pub(crate) assumptions: Vec<RuntimeAssumption>,
    pub(crate) trace_node_ids: Vec<String>,
}

impl TransitionPlan {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        identity: EvaluationIdentity,
        outcome: TransitionOutcome,
        effects: Vec<EffectIntent>,
        timers: Vec<TimerIntent>,
        desired: Option<DesiredProjection>,
        next_state: PlannerState,
        assumptions: Vec<RuntimeAssumption>,
        trace_node_ids: Vec<String>,
    ) -> Result<Self, PlannerModelError> {
        validate_identity_value(&identity)?;
        validate_planner_state(&next_state)?;
        if let Some(projection) = &desired {
            DesiredProjection::new(projection.fields.clone())?;
        }
        for assumption in &assumptions {
            validate_assumption(assumption)?;
        }
        for effect in &effects {
            let rebuilt = EffectIntent::new(
                effect.intent_id.clone(),
                effect.branch_id.clone(),
                effect.suspension_id.clone(),
                effect.continuation_node_id.clone(),
                effect.binding.clone(),
                effect.operation.clone(),
                effect.required_receipt,
                effect.retry.clone(),
                effect.issued_at,
                effect.deadline,
                effect.idempotency_key.clone(),
            )?;
            if rebuilt != *effect {
                return Err(PlannerModelError::InvalidEffectIntent);
            }
        }
        for timer in &timers {
            TimerIntent::new(
                timer.timer_id.clone(),
                timer.branch_id.clone(),
                timer.suspension_id.clone(),
                timer.continuation_node_id.clone(),
                timer.scheduled_at,
                timer.fire_at,
            )?;
        }
        if effects.len() > HARD_MAX_EFFECT_INTENTS {
            return Err(PlannerModelError::CollectionBound {
                collection: "effect intents",
                actual: effects.len(),
                maximum: HARD_MAX_EFFECT_INTENTS,
            });
        }
        if timers.len() > HARD_MAX_TIMERS_PER_RULE {
            return Err(PlannerModelError::CollectionBound {
                collection: "timer intents",
                actual: timers.len(),
                maximum: HARD_MAX_TIMERS_PER_RULE,
            });
        }
        let mut intent_ids = BTreeSet::new();
        for effect in &effects {
            if !intent_ids.insert(effect.intent_id.clone()) {
                return Err(PlannerModelError::DuplicateIdentifier(
                    effect.intent_id.clone(),
                ));
            }
        }
        let mut timer_ids = BTreeSet::new();
        for timer in &timers {
            if !timer_ids.insert(timer.timer_id.clone()) {
                return Err(PlannerModelError::DuplicateIdentifier(
                    timer.timer_id.clone(),
                ));
            }
        }
        if trace_node_ids.len() > HARD_MAX_NODES {
            return Err(PlannerModelError::CollectionBound {
                collection: "trace node IDs",
                actual: trace_node_ids.len(),
                maximum: HARD_MAX_NODES,
            });
        }
        for node_id in &trace_node_ids {
            validate_id("trace node_id", node_id)?;
        }
        let plan = Self {
            identity,
            outcome,
            effects,
            timers,
            desired,
            next_state,
            assumptions,
            trace_node_ids,
        };
        validate_encoded_size("transition plan", &plan)?;
        Ok(plan)
    }

    pub const fn identity(&self) -> &EvaluationIdentity {
        &self.identity
    }

    pub const fn outcome(&self) -> TransitionOutcome {
        self.outcome
    }

    pub fn effects(&self) -> &[EffectIntent] {
        &self.effects
    }

    pub fn timers(&self) -> &[TimerIntent] {
        &self.timers
    }

    pub const fn desired(&self) -> Option<&DesiredProjection> {
        self.desired.as_ref()
    }

    pub const fn next_state(&self) -> &PlannerState {
        &self.next_state
    }

    pub fn assumptions(&self) -> &[RuntimeAssumption] {
        &self.assumptions
    }

    pub fn trace_node_ids(&self) -> &[String] {
        &self.trace_node_ids
    }
}

/// Immutable input to one pure planner microstep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerEvaluationInput {
    pub(crate) identity: EvaluationIdentity,
    pub(crate) event: EvaluationEvent,
    pub(crate) evaluation_time: Timestamp,
    pub(crate) snapshots: Vec<CapabilitySnapshot>,
    pub(crate) effect_bindings: Vec<EffectBinding>,
    pub(crate) state: PlannerState,
}

impl PlannerEvaluationInput {
    pub fn new(
        identity: EvaluationIdentity,
        event: EvaluationEvent,
        evaluation_time: Timestamp,
        snapshots: Vec<CapabilitySnapshot>,
        effect_bindings: Vec<EffectBinding>,
        state: PlannerState,
    ) -> Result<Self, PlannerModelError> {
        if state.process_epoch != identity.process_epoch {
            return Err(PlannerModelError::StateIdentityMismatch);
        }
        if let Some(run) = &state.active_run
            && !matches!(event, EvaluationEvent::Initial { .. })
            && (run.run_id != identity.run_id || run.next_microstep != identity.microstep)
        {
            return Err(PlannerModelError::StateIdentityMismatch);
        }
        if event.event_time() > evaluation_time
            || snapshots
                .iter()
                .any(|snapshot| snapshot.received_at > evaluation_time)
        {
            return Err(PlannerModelError::InvalidTimestampOrder);
        }
        if snapshots.len() > HARD_MAX_NODES {
            return Err(PlannerModelError::CollectionBound {
                collection: "capability snapshots",
                actual: snapshots.len(),
                maximum: HARD_MAX_NODES,
            });
        }
        if effect_bindings.len() > HARD_MAX_EFFECT_INTENTS {
            return Err(PlannerModelError::CollectionBound {
                collection: "effect bindings",
                actual: effect_bindings.len(),
                maximum: HARD_MAX_EFFECT_INTENTS,
            });
        }
        let mut snapshot_selectors = BTreeSet::new();
        for snapshot in &snapshots {
            let identity = selector_identity(&snapshot.selector);
            if !snapshot_selectors.insert(identity) {
                return Err(PlannerModelError::DuplicateSnapshot);
            }
        }
        let mut binding_nodes = BTreeSet::new();
        for binding in &effect_bindings {
            if !binding_nodes.insert(binding.node_id.clone()) {
                return Err(PlannerModelError::DuplicateEffectBinding(
                    binding.node_id.clone(),
                ));
            }
        }
        let input = Self {
            identity,
            event,
            evaluation_time,
            snapshots,
            effect_bindings,
            state,
        };
        input.validate(HARD_MAX_PLAN_INPUT_BYTES)?;
        Ok(input)
    }

    pub const fn identity(&self) -> &EvaluationIdentity {
        &self.identity
    }

    pub const fn event(&self) -> &EvaluationEvent {
        &self.event
    }

    pub const fn evaluation_time(&self) -> Timestamp {
        self.evaluation_time
    }

    pub fn snapshots(&self) -> &[CapabilitySnapshot] {
        &self.snapshots
    }

    pub fn effect_bindings(&self) -> &[EffectBinding] {
        &self.effect_bindings
    }

    pub const fn state(&self) -> &PlannerState {
        &self.state
    }

    pub fn encoded_bytes(&self) -> usize {
        encoded_size(self)
    }

    pub(crate) fn validate(&self, maximum_bytes: usize) -> Result<(), PlannerModelError> {
        validate_evaluation_input(self)?;
        let actual = self.encoded_bytes();
        if actual > maximum_bytes {
            return Err(PlannerModelError::EncodedInputTooLarge {
                actual,
                maximum: maximum_bytes,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlannerModelError {
    #[error("timestamp nanosecond must be below one billion")]
    InvalidTimestamp,
    #[error("invalid {0} identifier")]
    InvalidIdentifier(&'static str),
    #[error("invalid {0} digest")]
    InvalidDigest(&'static str),
    #[error("{0} must be non-zero")]
    ZeroRevision(&'static str),
    #[error("selector shape is invalid")]
    InvalidSelector,
    #[error("timestamps are not in causal order")]
    InvalidTimestampOrder,
    #[error("planner state identity does not match the evaluation identity")]
    StateIdentityMismatch,
    #[error("planner run status and frontier are inconsistent")]
    InvalidRunState,
    #[error("suspension continuation does not match its saved frame")]
    ContinuationMismatch,
    #[error("{collection} has {actual} items; maximum is {maximum}")]
    CollectionBound {
        collection: &'static str,
        actual: usize,
        maximum: usize,
    },
    #[error("duplicate snapshot field {0}")]
    DuplicateSnapshotField(String),
    #[error("snapshot field does not belong to the selector contract")]
    SnapshotContractMismatch,
    #[error("snapshot field value has the wrong closed runtime type")]
    SnapshotValueTypeMismatch,
    #[error("runtime string or native representation exceeds its bound")]
    RuntimeValueTooLarge,
    #[error("trigger event does not belong to the selector contract")]
    TriggerContractMismatch,
    #[error("discrete event payload does not match its exact event type")]
    DiscreteEventShapeMismatch,
    #[error("effect binding does not belong to the operation contract")]
    EffectContractMismatch,
    #[error("duplicate receipt support")]
    DuplicateReceiptSupport,
    #[error("effect deadline bounds are invalid")]
    InvalidDeadline,
    #[error("effect operation does not match its binding")]
    EffectOperationMismatch,
    #[error("effect intent derived fields are inconsistent")]
    InvalidEffectIntent,
    #[error("binding does not support the required receipt")]
    UnsupportedReceipt,
    #[error("retry requires an eligible operation and provider-generation deduplication")]
    InvalidRetry,
    #[error("timer duration is outside the process-local bound")]
    InvalidTimerDuration,
    #[error("desired field does not belong to the selector contract")]
    DesiredContractMismatch,
    #[error("duplicate desired field")]
    DuplicateDesiredField,
    #[error("duplicate identifier {0}")]
    DuplicateIdentifier(String),
    #[error("duplicate join arrival")]
    DuplicateJoinArrival,
    #[error("duplicate capability snapshot")]
    DuplicateSnapshot,
    #[error("duplicate effect binding for node {0}")]
    DuplicateEffectBinding(String),
    #[error("encoded planner input is {actual} bytes; maximum is {maximum}")]
    EncodedInputTooLarge { actual: usize, maximum: usize },
    #[error("encoded {subject} is {actual} bytes; maximum is {maximum}")]
    EncodedModelTooLarge {
        subject: &'static str,
        actual: usize,
        maximum: usize,
    },
}

fn validate_evaluation_input(input: &PlannerEvaluationInput) -> Result<(), PlannerModelError> {
    validate_identity_value(&input.identity)?;
    validate_event(&input.event)?;
    validate_planner_state(&input.state)?;
    if input.state.process_epoch != input.identity.process_epoch {
        return Err(PlannerModelError::StateIdentityMismatch);
    }
    if let Some(run) = &input.state.active_run
        && !matches!(input.event, EvaluationEvent::Initial { .. })
        && (run.run_id != input.identity.run_id || run.next_microstep != input.identity.microstep)
    {
        return Err(PlannerModelError::StateIdentityMismatch);
    }
    if input.event.event_time() > input.evaluation_time
        || input
            .snapshots
            .iter()
            .any(|snapshot| snapshot.received_at > input.evaluation_time)
    {
        return Err(PlannerModelError::InvalidTimestampOrder);
    }
    if input.snapshots.len() > HARD_MAX_NODES {
        return Err(PlannerModelError::CollectionBound {
            collection: "capability snapshots",
            actual: input.snapshots.len(),
            maximum: HARD_MAX_NODES,
        });
    }
    if input.effect_bindings.len() > HARD_MAX_EFFECT_INTENTS {
        return Err(PlannerModelError::CollectionBound {
            collection: "effect bindings",
            actual: input.effect_bindings.len(),
            maximum: HARD_MAX_EFFECT_INTENTS,
        });
    }
    let mut selectors = BTreeSet::new();
    for snapshot in &input.snapshots {
        CapabilitySnapshot::new(
            snapshot.selector.clone(),
            snapshot.provider_generation,
            snapshot.facet_revision,
            snapshot.source_time,
            snapshot.received_at,
            snapshot.freshness,
            snapshot.fields.clone(),
        )?;
        if !selectors.insert(selector_identity(&snapshot.selector)) {
            return Err(PlannerModelError::DuplicateSnapshot);
        }
    }
    let mut nodes = BTreeSet::new();
    for binding in &input.effect_bindings {
        validate_effect_binding(binding)?;
        if !nodes.insert(binding.node_id.clone()) {
            return Err(PlannerModelError::DuplicateEffectBinding(
                binding.node_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_identity_value(identity: &EvaluationIdentity) -> Result<(), PlannerModelError> {
    EvaluationIdentity::new(
        identity.process_epoch.clone(),
        identity.rule_id.clone(),
        identity.rule_revision,
        identity.semantic_sha256.clone(),
        identity.profile_sha256.clone(),
        identity.artifacts.clone(),
        identity.run_id.clone(),
        identity.microstep,
    )?;
    Ok(())
}

fn validate_trigger_input(trigger: &TriggerInput) -> Result<(), PlannerModelError> {
    match trigger {
        TriggerInput::StateObservation {
            selector,
            provider_generation,
            facet_revision,
            observed_at,
            delivery,
        } => {
            TriggerInput::state_observation(
                selector.clone(),
                *provider_generation,
                *facet_revision,
                *observed_at,
                *delivery,
            )?;
        }
        TriggerInput::DiscreteEvent {
            selector,
            event,
            payload,
            observed_at,
        } => {
            TriggerInput::discrete_event(
                selector.clone(),
                event.clone(),
                payload.clone(),
                *observed_at,
            )?;
        }
        TriggerInput::TimeSchedule {
            occurrence_id,
            activation_anchor,
            scheduled_for,
        } => {
            TriggerInput::time_schedule(occurrence_id.clone(), *activation_anchor, *scheduled_for)?;
        }
    }
    Ok(())
}

fn validate_receipt(receipt: &EffectReceipt) -> Result<(), PlannerModelError> {
    EffectReceipt::new(
        receipt.intent_id.clone(),
        receipt.request_id.clone(),
        receipt.operation.clone(),
        receipt.attempt,
        receipt.provider_generation,
        receipt.confidence,
        receipt.outcome,
        receipt.accepted_at,
        receipt.completed_at,
        receipt.idempotency_key.clone(),
    )?;
    Ok(())
}

fn validate_event(event: &EvaluationEvent) -> Result<(), PlannerModelError> {
    match event {
        EvaluationEvent::Initial { trigger } => validate_trigger_input(trigger),
        EvaluationEvent::Timer {
            timer_id,
            suspension_id,
            scheduled_for,
            fired_at,
        } => {
            EvaluationEvent::timer(
                timer_id.clone(),
                suspension_id.clone(),
                *scheduled_for,
                *fired_at,
            )?;
            Ok(())
        }
        EvaluationEvent::EffectReceipt {
            suspension_id,
            receipt,
        } => {
            validate_id("suspension_id", suspension_id)?;
            validate_receipt(receipt)
        }
    }
}

fn validate_effect_binding(binding: &EffectBinding) -> Result<(), PlannerModelError> {
    EffectBinding::new(
        binding.node_id.clone(),
        binding.selector.clone(),
        binding.operation,
        binding.provider_generation,
        binding.facet_revision,
        binding.grant_revision,
        binding.operation_kind,
        binding.deduplication,
        binding.receipt_support.clone(),
        binding.default_deadline_ms,
        binding.maximum_deadline_ms,
    )?;
    Ok(())
}

fn validate_saved_frame(frame: &SavedFrame) -> Result<(), PlannerModelError> {
    validate_id("branch_id", &frame.branch_id)?;
    validate_id("resume_node_id", &frame.resume_node_id)?;
    validate_trigger_input(&frame.trigger)?;
    if frame.values.len() > HARD_MAX_NODES || frame.trace_node_ids.len() > HARD_MAX_NODES {
        return Err(PlannerModelError::CollectionBound {
            collection: "saved frame items",
            actual: frame.values.len().max(frame.trace_node_ids.len()),
            maximum: HARD_MAX_NODES,
        });
    }
    let mut values = BTreeSet::new();
    for value in &frame.values {
        validate_id("saved value node_id", &value.node_id)?;
        if !values.insert(value.node_id.clone()) {
            return Err(PlannerModelError::DuplicateIdentifier(
                value.node_id.clone(),
            ));
        }
        if let SavedNodeValue::Value {
            state: ValueState::Known { value },
        } = &value.value
        {
            validate_runtime_value(value)?;
        }
    }
    for node_id in &frame.trace_node_ids {
        validate_id("saved trace node_id", node_id)?;
    }
    if let Some(desired) = &frame.desired {
        DesiredProjection::new(desired.fields.clone())?;
    }
    for assumption in &frame.assumptions {
        validate_assumption(assumption)?;
    }
    Ok(())
}

fn validate_assumption(assumption: &RuntimeAssumption) -> Result<(), PlannerModelError> {
    match assumption {
        RuntimeAssumption::SnapshotRevision {
            selector,
            provider_generation,
            facet_revision,
            ..
        } => {
            validate_selector(selector)?;
            require_nonzero_revision(*provider_generation, "provider_generation")?;
            require_nonzero_revision(*facet_revision, "facet_revision")
        }
        RuntimeAssumption::EffectBindingRevision {
            node_id,
            provider_generation,
            facet_revision,
            grant_revision,
        } => {
            validate_id("node_id", node_id)?;
            require_nonzero_revision(*provider_generation, "provider_generation")?;
            require_nonzero_revision(*facet_revision, "facet_revision")?;
            require_nonzero_revision(*grant_revision, "grant_revision")
        }
        RuntimeAssumption::PlannerStateVersion { version } => {
            require_nonzero_revision(*version, "planner state version")
        }
        RuntimeAssumption::ExpectedValue {
            selector, state, ..
        } => {
            validate_selector(selector)?;
            if let ValueState::Known { value } = state {
                validate_runtime_value(value)?;
            }
            Ok(())
        }
    }
}

fn validate_frontier(frontier: &Frontier) -> Result<(), PlannerModelError> {
    Frontier::new(
        frontier.ready.clone(),
        frontier.suspended.clone(),
        frontier.join_arrivals.clone(),
    )?;
    for frame in &frontier.ready {
        validate_saved_frame(frame)?;
    }
    Ok(())
}

fn validate_run_state(run: &RunState) -> Result<(), PlannerModelError> {
    if !valid_hex_32(&run.run_id) {
        return Err(PlannerModelError::InvalidIdentifier("run_id"));
    }
    require_nonzero_revision(run.next_microstep as u64, "next_microstep")?;
    validate_frontier(&run.frontier)?;
    let coherent = match run.status {
        RunStatus::Ready => !run.frontier.ready.is_empty(),
        RunStatus::Waiting => run.frontier.ready.is_empty() && !run.frontier.suspended.is_empty(),
        RunStatus::Completed | RunStatus::Cancelled => {
            run.frontier.ready.is_empty()
                && run.frontier.suspended.is_empty()
                && run.frontier.join_arrivals.is_empty()
        }
    };
    if coherent {
        Ok(())
    } else {
        Err(PlannerModelError::InvalidRunState)
    }
}

fn validate_planner_state(state: &PlannerState) -> Result<(), PlannerModelError> {
    require_nonzero_revision(state.version, "planner state version")?;
    if !valid_hex_32(&state.process_epoch) {
        return Err(PlannerModelError::InvalidIdentifier("process_epoch"));
    }
    if state.queued_triggers.len() > HARD_MAX_QUEUED_TRIGGERS {
        return Err(PlannerModelError::CollectionBound {
            collection: "queued triggers",
            actual: state.queued_triggers.len(),
            maximum: HARD_MAX_QUEUED_TRIGGERS,
        });
    }
    if let Some(run) = &state.active_run {
        validate_run_state(run)?;
    }
    for trigger in &state.queued_triggers {
        validate_trigger_input(trigger)?;
    }
    Ok(())
}

fn validate_selector(selector: &Selector) -> Result<(), PlannerModelError> {
    if valid_hex_32(&selector.provider_instance_id)
        && valid_id(&selector.facet_key)
        && valid_contract_part(&selector.contract.namespace)
        && valid_contract_part(&selector.contract.package)
        && valid_contract_part(&selector.contract.interface)
        && selector.contract.major != 0
        && selector.cardinality == SelectorCardinality::One
        && matches!(
            selector.rebind,
            RebindPolicy::Never | RebindPolicy::SameIdentity
        )
    {
        Ok(())
    } else {
        Err(PlannerModelError::InvalidSelector)
    }
}

fn validate_id(name: &'static str, value: &str) -> Result<(), PlannerModelError> {
    if valid_id(value) {
        Ok(())
    } else {
        Err(PlannerModelError::InvalidIdentifier(name))
    }
}

fn require_nonzero_revision(value: u64, name: &'static str) -> Result<(), PlannerModelError> {
    if value == 0 {
        Err(PlannerModelError::ZeroRevision(name))
    } else {
        Ok(())
    }
}

fn validate_value_state(
    field: &StateFieldRef,
    state: &ValueState,
) -> Result<(), PlannerModelError> {
    let ValueState::Known { value } = state else {
        return Ok(());
    };
    validate_runtime_value(value)?;
    let matches = match field {
        StateFieldRef::OnOffCurrent
        | StateFieldRef::BooleanCurrent
        | StateFieldRef::ThermostatRunning => matches!(value, RuntimeValue::Bool(_)),
        StateFieldRef::LevelTransitionRemaining => matches!(value, RuntimeValue::U64(_)),
        StateFieldRef::ObservationRevision { .. } => matches!(value, RuntimeValue::U64(_)),
        StateFieldRef::LevelCurrentLevel
        | StateFieldRef::LevelTargetLevel
        | StateFieldRef::CoveringCurrentPosition { .. }
        | StateFieldRef::CoveringTargetPosition { .. } => matches!(
            value,
            RuntimeValue::Quantity(quantity)
                if quantity.profile_id().as_str() == "litehouse:quantity/ratio@1"
        ),
        StateFieldRef::ColorCurrentColor | StateFieldRef::ColorTargetColor => {
            matches!(value, RuntimeValue::ColorValue(_))
        }
        StateFieldRef::ColorActiveModel => matches!(value, RuntimeValue::ColorModel(_)),
        StateFieldRef::MeasurementCurrent { profile } => match value {
            RuntimeValue::Quantity(quantity) => quantity.profile_id().as_str() == profile.as_str(),
            _ => false,
        },
        StateFieldRef::MeasurementNativeRepresentation => {
            matches!(value, RuntimeValue::NativeRepresentation(_))
        }
        StateFieldRef::ThermostatCurrentMode | StateFieldRef::ThermostatTargetMode => {
            matches!(value, RuntimeValue::ThermostatMode(_))
        }
        StateFieldRef::ThermostatAction => {
            matches!(value, RuntimeValue::ThermostatAction(_))
        }
        StateFieldRef::ThermostatTargets => {
            matches!(
                value,
                RuntimeValue::ThermostatTargets(ThermostatTargets::Single { temperature })
                    if temperature.profile_id().as_str()
                        == "litehouse:quantity/temperature-absolute@1"
            ) || matches!(
                value,
                RuntimeValue::ThermostatTargets(ThermostatTargets::HeatCool { heat, cool })
                    if heat.profile_id().as_str()
                        == "litehouse:quantity/temperature-absolute@1"
                        && cool.profile_id().as_str()
                            == "litehouse:quantity/temperature-absolute@1"
                        && heat.checked_cmp(cool).is_ok_and(|ordering| !ordering.is_gt())
            )
        }
        StateFieldRef::ThermostatFan
        | StateFieldRef::ThermostatSwing
        | StateFieldRef::ThermostatPreset => matches!(value, RuntimeValue::String(_)),
        StateFieldRef::CoveringMovement { .. } => {
            matches!(value, RuntimeValue::CoveringMovement(_))
        }
        StateFieldRef::CoveringSafetyStatus | StateFieldRef::CoveringFaultStatus => {
            matches!(value, RuntimeValue::StringList(_))
        }
    };
    if matches {
        Ok(())
    } else {
        Err(PlannerModelError::SnapshotValueTypeMismatch)
    }
}

pub(crate) fn validate_snapshot_value_state(
    field: &StateFieldRef,
    state: &ValueState,
) -> Result<(), PlannerModelError> {
    validate_value_state(field, state)
}

fn validate_runtime_value(value: &RuntimeValue) -> Result<(), PlannerModelError> {
    let valid = match value {
        RuntimeValue::String(value) => value.len() <= HARD_MAX_STRING_BYTES,
        RuntimeValue::ThermostatMode(ThermostatModeValue::ProviderQualified(value)) => {
            !value.is_empty() && value.len() <= 64
        }
        RuntimeValue::ColorValue(value) => match value {
            RuntimeColorValue::HueSaturation {
                hue_degrees,
                saturation_ratio,
            } => valid_decimal_shape(&hue_degrees.0) && valid_decimal_shape(&saturation_ratio.0),
            RuntimeColorValue::CieXy { x, y } => {
                valid_decimal_shape(&x.0) && valid_decimal_shape(&y.0)
            }
            RuntimeColorValue::ColorTemperature { kelvin } => valid_decimal_shape(&kelvin.0),
            RuntimeColorValue::CalibratedRgb {
                red,
                green,
                blue,
                color_space,
            } => {
                valid_decimal_shape(&red.0)
                    && valid_decimal_shape(&green.0)
                    && valid_decimal_shape(&blue.0)
                    && !color_space.is_empty()
                    && color_space.len() <= 64
            }
            RuntimeColorValue::UncalibratedRgb { red, green, blue } => {
                valid_decimal_shape(&red.0)
                    && valid_decimal_shape(&green.0)
                    && valid_decimal_shape(&blue.0)
            }
            RuntimeColorValue::NativeWhiteChannels {
                white,
                cold_white,
                warm_white,
            } => [white, cold_white, warm_white]
                .into_iter()
                .flatten()
                .all(|value| valid_decimal_shape(&value.0)),
        },
        RuntimeValue::NativeRepresentation(value) => {
            !value.magnitude.is_empty()
                && value.magnitude.len() <= HARD_MAX_STRING_BYTES
                && !value.unit.is_empty()
                && value.unit.len() <= 256
        }
        RuntimeValue::StringList(values) => {
            values.len() <= MAX_STATUS_ITEMS
                && values
                    .iter()
                    .all(|value| value.len() <= HARD_MAX_STRING_BYTES)
        }
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(PlannerModelError::RuntimeValueTooLarge)
    }
}

fn field_identity(field: &StateFieldRef) -> String {
    serde_json::to_string(field).expect("closed state field must serialize")
}

pub(crate) fn selector_identity(selector: &Selector) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}",
        selector.provider_instance_id,
        selector.facet_key,
        selector.contract.namespace,
        selector.contract.package,
        selector.contract.interface,
        selector.contract.major,
    )
}

pub(crate) const fn receipt_rank(confidence: ReceiptConfidence) -> u8 {
    match confidence {
        ReceiptConfidence::AcceptedForDelivery => 0,
        ReceiptConfidence::ProtocolOrTargetAcknowledged => 1,
        ReceiptConfidence::Completed => 2,
        ReceiptConfidence::ObservedCompleted => 3,
    }
}

pub(crate) const fn desired_path_rank(path: DesiredProjectionPath) -> u8 {
    match path {
        DesiredProjectionPath::On => 0,
        DesiredProjectionPath::Level => 1,
        DesiredProjectionPath::Color => 2,
        DesiredProjectionPath::ThermostatMode => 3,
        DesiredProjectionPath::ThermostatSingleTarget => 4,
        DesiredProjectionPath::ThermostatHeatTarget => 5,
        DesiredProjectionPath::ThermostatCoolTarget => 6,
        DesiredProjectionPath::ThermostatFan => 7,
        DesiredProjectionPath::ThermostatSwing => 8,
        DesiredProjectionPath::ThermostatPreset => 9,
        DesiredProjectionPath::CoveringLift => 10,
        DesiredProjectionPath::CoveringTilt => 11,
    }
}

fn timestamp_nanoseconds(timestamp: Timestamp) -> i128 {
    i128::from(timestamp.unix_seconds()) * 1_000_000_000 + i128::from(timestamp.nanosecond())
}

fn encoded_size<T: Serialize>(value: &T) -> usize {
    serde_json::to_vec(value)
        .expect("closed planner model must serialize")
        .len()
}

fn validate_encoded_size<T: Serialize>(
    subject: &'static str,
    value: &T,
) -> Result<(), PlannerModelError> {
    let actual = encoded_size(value);
    if actual > HARD_MAX_PLAN_INPUT_BYTES {
        Err(PlannerModelError::EncodedModelTooLarge {
            subject,
            actual,
            maximum: HARD_MAX_PLAN_INPUT_BYTES,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BooleanStateEventKind, ContractRef, RebindPolicy, SelectorCardinality, StatefulContractKind,
    };

    const HEX_32: &str = "11111111111111111111111111111111";
    const HEX_64: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    fn timestamp(seconds: i64) -> Timestamp {
        Timestamp::new(seconds, 0).unwrap()
    }

    fn selector(provider: usize, package: &str) -> Selector {
        Selector {
            provider_instance_id: format!("{provider:032x}"),
            facet_key: "main".to_owned(),
            contract: ContractRef::litehouse(package, "capability", 1),
            cardinality: SelectorCardinality::One,
            rebind: RebindPolicy::Never,
        }
    }

    fn artifacts() -> CompiledArtifacts {
        CompiledArtifacts {
            schema_sha256: HEX_64.to_owned(),
            compiler_sha256: HEX_64.to_owned(),
            cel_sha256: HEX_64.to_owned(),
            quantity_catalog_sha256: HEX_64.to_owned(),
            runtime_profile_sha256: HEX_64.to_owned(),
            timezone_database_sha256: HEX_64.to_owned(),
            contract_catalog_sha256: HEX_64.to_owned(),
            management_operations_sha256: HEX_64.to_owned(),
            port_catalog_sha256: HEX_64.to_owned(),
        }
    }

    fn identity() -> EvaluationIdentity {
        EvaluationIdentity::new(HEX_32, HEX_32, 1, HEX_64, HEX_64, artifacts(), HEX_32, 1).unwrap()
    }

    fn state() -> PlannerState {
        PlannerState::new(1, HEX_32, None, Vec::new()).unwrap()
    }

    fn binding(node_id: &str) -> EffectBinding {
        EffectBinding::new(
            node_id,
            selector(1, "on-off"),
            CapabilityOperationKind::OnOffSet,
            1,
            1,
            1,
            OperationKind::AbsoluteConvergent,
            DeduplicationGuarantee::ProviderGeneration,
            vec![ReceiptConfidence::AcceptedForDelivery],
            1_000,
            30_000,
        )
        .unwrap()
    }

    #[test]
    fn timestamp_deserialization_reuses_the_normalized_constructor() {
        assert!(
            serde_json::from_str::<Timestamp>(
                r#"{"unix_seconds":1721234567,"nanosecond":1000000000}"#,
            )
            .is_err()
        );
        assert_eq!(
            serde_json::from_str::<Timestamp>(
                r#"{"unix_seconds":1721234567,"nanosecond":999999999}"#,
            )
            .unwrap(),
            Timestamp::new(1_721_234_567, 999_999_999).unwrap(),
        );
    }

    #[test]
    fn rejects_invalid_identity_ids_and_digests() {
        assert_eq!(
            EvaluationIdentity::new("not-hex", HEX_32, 1, HEX_64, HEX_64, artifacts(), HEX_32, 1,),
            Err(PlannerModelError::InvalidIdentifier("process_epoch"))
        );
        assert_eq!(
            EvaluationIdentity::new(HEX_32, HEX_32, 1, "bad", HEX_64, artifacts(), HEX_32, 1,),
            Err(PlannerModelError::InvalidDigest("semantic_sha256"))
        );
    }

    #[test]
    fn every_state_field_accepts_only_its_closed_runtime_value() {
        let ratio = || Quantity::parse("litehouse:quantity/ratio@1", "0.5", "1").unwrap();
        let temperature =
            || Quantity::parse("litehouse:quantity/temperature-absolute@1", "293.15", "K").unwrap();
        let cases = vec![
            (StateFieldRef::OnOffCurrent, RuntimeValue::Bool(true)),
            (StateFieldRef::BooleanCurrent, RuntimeValue::Bool(false)),
            (
                StateFieldRef::LevelCurrentLevel,
                RuntimeValue::Quantity(ratio()),
            ),
            (
                StateFieldRef::LevelTargetLevel,
                RuntimeValue::Quantity(ratio()),
            ),
            (
                StateFieldRef::LevelTransitionRemaining,
                RuntimeValue::U64(100),
            ),
            (
                StateFieldRef::ColorCurrentColor,
                RuntimeValue::ColorValue(RuntimeColorValue::ColorTemperature {
                    kelvin: DecimalValue::new("4000"),
                }),
            ),
            (
                StateFieldRef::ColorTargetColor,
                RuntimeValue::ColorValue(RuntimeColorValue::NativeWhiteChannels {
                    white: Some(DecimalValue::new("0.5")),
                    cold_white: None,
                    warm_white: None,
                }),
            ),
            (
                StateFieldRef::ColorActiveModel,
                RuntimeValue::ColorModel(ColorModel::ColorTemperature),
            ),
            (
                StateFieldRef::MeasurementCurrent {
                    profile: crate::QuantityProfileRef::Power,
                },
                RuntimeValue::Quantity(
                    Quantity::parse("litehouse:quantity/power@1", "40", "W").unwrap(),
                ),
            ),
            (
                StateFieldRef::MeasurementNativeRepresentation,
                RuntimeValue::NativeRepresentation(NativeRepresentation {
                    magnitude: "40".to_owned(),
                    unit: "W".to_owned(),
                    quantization: None,
                }),
            ),
            (
                StateFieldRef::ThermostatCurrentMode,
                RuntimeValue::ThermostatMode(ThermostatModeValue::Known(ThermostatMode::Heat)),
            ),
            (
                StateFieldRef::ThermostatTargetMode,
                RuntimeValue::ThermostatMode(ThermostatModeValue::Known(ThermostatMode::Auto)),
            ),
            (
                StateFieldRef::ThermostatAction,
                RuntimeValue::ThermostatAction(ThermostatAction::Heating),
            ),
            (StateFieldRef::ThermostatRunning, RuntimeValue::Bool(true)),
            (
                StateFieldRef::ThermostatTargets,
                RuntimeValue::ThermostatTargets(ThermostatTargets::Single {
                    temperature: temperature(),
                }),
            ),
            (
                StateFieldRef::ThermostatFan,
                RuntimeValue::String("auto".to_owned()),
            ),
            (
                StateFieldRef::ThermostatSwing,
                RuntimeValue::String("vertical".to_owned()),
            ),
            (
                StateFieldRef::ThermostatPreset,
                RuntimeValue::String("eco".to_owned()),
            ),
            (
                StateFieldRef::CoveringCurrentPosition {
                    axis: crate::CoveringAxis::Lift,
                },
                RuntimeValue::Quantity(ratio()),
            ),
            (
                StateFieldRef::CoveringTargetPosition {
                    axis: crate::CoveringAxis::Tilt,
                },
                RuntimeValue::Quantity(ratio()),
            ),
            (
                StateFieldRef::CoveringMovement {
                    axis: crate::CoveringAxis::Lift,
                },
                RuntimeValue::CoveringMovement(CoveringMovement::Opening),
            ),
            (
                StateFieldRef::CoveringSafetyStatus,
                RuntimeValue::StringList(vec!["clear".to_owned()]),
            ),
            (
                StateFieldRef::CoveringFaultStatus,
                RuntimeValue::StringList(vec!["none".to_owned()]),
            ),
            (
                StateFieldRef::ObservationRevision {
                    contract: StatefulContractKind::OnOff,
                },
                RuntimeValue::U64(1),
            ),
        ];
        for (field, value) in cases {
            SnapshotField::new(field, ValueState::Known { value }).unwrap();
        }
        assert!(matches!(
            SnapshotField::new(
                StateFieldRef::LevelTransitionRemaining,
                ValueState::Known {
                    value: RuntimeValue::Duration(RuntimeDuration { milliseconds: 100 })
                },
            ),
            Err(PlannerModelError::SnapshotValueTypeMismatch)
        ));
    }

    #[test]
    fn rejects_duplicate_snapshot_fields() {
        let field = SnapshotField::new(
            StateFieldRef::ObservationRevision {
                contract: StatefulContractKind::OnOff,
            },
            ValueState::Known {
                value: RuntimeValue::U64(1),
            },
        )
        .unwrap();
        assert!(matches!(
            CapabilitySnapshot::new(
                selector(1, "on-off"),
                1,
                1,
                None,
                timestamp(1),
                Freshness::Fresh,
                vec![field.clone(), field],
            ),
            Err(PlannerModelError::DuplicateSnapshotField(_))
        ));
    }

    #[test]
    fn rejects_duplicate_effect_bindings() {
        let trigger = TriggerInput::discrete_event(
            selector(2, "boolean-state"),
            ContractEventRef::BooleanState {
                event: BooleanStateEventKind::Occurrence,
            },
            EventPayload::BooleanOccurrence {
                occurrence: BooleanOccurrence::new("event", "source", 1, timestamp(1), Some(true))
                    .unwrap(),
            },
            timestamp(1),
        )
        .unwrap();
        assert!(matches!(
            PlannerEvaluationInput::new(
                identity(),
                EvaluationEvent::initial(trigger),
                timestamp(1),
                Vec::new(),
                vec![binding("effect"), binding("effect")],
                state(),
            ),
            Err(PlannerModelError::DuplicateEffectBinding(node)) if node == "effect"
        ));
    }

    #[test]
    fn rejects_encoded_input_over_64_kib() {
        let trigger =
            TriggerInput::time_schedule("occurrence", timestamp(0), timestamp(1)).unwrap();
        let snapshots = (1..=5)
            .map(|provider| {
                CapabilitySnapshot::new(
                    selector(provider, "window-covering"),
                    1,
                    1,
                    None,
                    timestamp(1),
                    Freshness::Fresh,
                    vec![
                        SnapshotField::new(
                            StateFieldRef::CoveringSafetyStatus,
                            ValueState::Known {
                                value: RuntimeValue::StringList(vec!["x".repeat(15_000)]),
                            },
                        )
                        .unwrap(),
                    ],
                )
                .unwrap()
            })
            .collect();
        assert!(matches!(
            PlannerEvaluationInput::new(
                identity(),
                EvaluationEvent::initial(trigger),
                timestamp(1),
                snapshots,
                Vec::new(),
                state(),
            ),
            Err(PlannerModelError::EncodedInputTooLarge { maximum, .. })
                if maximum == 64 * 1024
        ));
    }
}
