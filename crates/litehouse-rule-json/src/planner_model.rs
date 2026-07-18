use std::collections::BTreeSet;

use litehouse_contracts::quantity::Quantity;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use crate::planner::Timestamp;

use crate::{
    EvaluationErrorCode, Freshness, UnknownReason,
    model::{
        CapabilityOperation, CapabilityOperationKind, ColorValue, ContractEventRef,
        DesiredProjectionPath, DesiredProjectionValue, HARD_MAX_EFFECT_INTENTS, HARD_MAX_NODES,
        HARD_MAX_PARALLEL_BRANCHES, HARD_MAX_PATCH_OPERATIONS, HARD_MAX_PLAN_INPUT_BYTES,
        HARD_MAX_QUEUED_TRIGGERS, HARD_MAX_STRING_BYTES, HARD_MAX_TIMER_DURATION_MS,
        HARD_MAX_TIMERS_PER_RULE, RebindPolicy, ReceiptConfidence, ReceiptOutcome,
        ReceiptRequirement, RetryPolicy, Selector, SelectorCardinality, StateFieldRef,
        ThermostatMode, valid_contract_part, valid_hex_32, valid_hex_64, valid_id,
    },
};

const MAX_SNAPSHOT_FIELDS: usize = 64;
const MAX_RECEIPT_CONFIDENCES: usize = 4;
const MAX_STATUS_ITEMS: usize = 64;
const MAX_EFFECT_DEADLINE_MS: u32 = 30_000;

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
    ColorValue(ColorValue),
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
pub enum TriggerInput {
    StateObservation {
        selector: Selector,
        provider_generation: u64,
        facet_revision: u64,
        observed_at: Timestamp,
    },
    DiscreteEvent {
        selector: Selector,
        event: ContractEventRef,
        occurrence: Option<BooleanOccurrence>,
        observed_at: Timestamp,
    },
    TimeSchedule {
        occurrence_id: String,
        scheduled_for: Timestamp,
    },
}

impl TriggerInput {
    pub fn state_observation(
        selector: Selector,
        provider_generation: u64,
        facet_revision: u64,
        observed_at: Timestamp,
    ) -> Result<Self, PlannerModelError> {
        validate_selector(&selector)?;
        require_nonzero_revision(provider_generation, "provider_generation")?;
        require_nonzero_revision(facet_revision, "facet_revision")?;
        Ok(Self::StateObservation {
            selector,
            provider_generation,
            facet_revision,
            observed_at,
        })
    }

    pub fn discrete_event(
        selector: Selector,
        event: ContractEventRef,
        occurrence: Option<BooleanOccurrence>,
        observed_at: Timestamp,
    ) -> Result<Self, PlannerModelError> {
        validate_selector(&selector)?;
        if selector.contract != event.selector_contract() {
            return Err(PlannerModelError::TriggerContractMismatch);
        }
        let is_occurrence = matches!(
            event,
            ContractEventRef::BooleanState {
                event: crate::BooleanStateEventKind::Occurrence
            }
        );
        if is_occurrence != occurrence.is_some() {
            return Err(PlannerModelError::DiscreteEventShapeMismatch);
        }
        if occurrence
            .as_ref()
            .is_some_and(|value| value.received_at > observed_at)
        {
            return Err(PlannerModelError::InvalidTimestampOrder);
        }
        Ok(Self::DiscreteEvent {
            selector,
            event,
            occurrence,
            observed_at,
        })
    }

    pub fn time_schedule(
        occurrence_id: impl Into<String>,
        scheduled_for: Timestamp,
    ) -> Result<Self, PlannerModelError> {
        let occurrence_id = occurrence_id.into();
        validate_id("occurrence_id", &occurrence_id)?;
        Ok(Self::TimeSchedule {
            occurrence_id,
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
        if microstep == 0 {
            return Err(PlannerModelError::ZeroRevision("microstep"));
        }
        Ok(Self {
            process_epoch,
            rule_id,
            rule_revision,
            semantic_sha256,
            profile_sha256,
            run_id,
            microstep,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedValue {
    pub node_id: String,
    pub value: ValueState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedFrame {
    pub branch_id: String,
    pub resume_node_id: String,
    pub values: Vec<SavedValue>,
}

impl SavedFrame {
    pub fn new(
        branch_id: impl Into<String>,
        resume_node_id: impl Into<String>,
        values: Vec<SavedValue>,
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
        Ok(Self {
            branch_id,
            resume_node_id,
            values,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Suspension {
    Effect {
        suspension_id: String,
        intent_id: String,
        continuation_node_id: String,
        frame: SavedFrame,
    },
    Timer {
        suspension_id: String,
        timer_id: String,
        continuation_node_id: String,
        frame: SavedFrame,
    },
}

impl Suspension {
    fn suspension_id(&self) -> &str {
        match self {
            Self::Effect { suspension_id, .. } | Self::Timer { suspension_id, .. } => suspension_id,
        }
    }

    fn validate(&self) -> Result<(), PlannerModelError> {
        validate_id("suspension_id", self.suspension_id())?;
        match self {
            Self::Effect {
                intent_id,
                continuation_node_id,
                ..
            } => {
                validate_id("intent_id", intent_id)?;
                validate_id("continuation_node_id", continuation_node_id)
            }
            Self::Timer {
                timer_id,
                continuation_node_id,
                ..
            } => {
                validate_id("timer_id", timer_id)?;
                validate_id("continuation_node_id", continuation_node_id)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinArrival {
    pub join_node_id: String,
    pub branch_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frontier {
    pub ready: Vec<SavedFrame>,
    pub suspended: Vec<Suspension>,
    pub join_arrivals: Vec<JoinArrival>,
}

impl Frontier {
    pub fn new(
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunState {
    pub run_id: String,
    pub next_microstep: u32,
    pub status: RunStatus,
    pub frontier: Frontier,
}

impl RunState {
    pub fn new(
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
        Ok(Self {
            run_id,
            next_microstep,
            status,
            frontier,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerState {
    pub version: u64,
    pub active_run: Option<RunState>,
    pub queued_triggers: Vec<TriggerInput>,
}

impl PlannerState {
    pub fn new(
        version: u64,
        active_run: Option<RunState>,
        queued_triggers: Vec<TriggerInput>,
    ) -> Result<Self, PlannerModelError> {
        require_nonzero_revision(version, "planner state version")?;
        if queued_triggers.len() > HARD_MAX_QUEUED_TRIGGERS {
            return Err(PlannerModelError::CollectionBound {
                collection: "queued triggers",
                actual: queued_triggers.len(),
                maximum: HARD_MAX_QUEUED_TRIGGERS,
            });
        }
        Ok(Self {
            version,
            active_run,
            queued_triggers,
        })
    }
}

/// The operation remains generic for typed internal lowering; the public runtime
/// boundary uses the closed authored [`CapabilityOperation`] instantiation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EffectIntent<O = CapabilityOperation> {
    pub intent_id: String,
    pub branch_id: String,
    pub suspension_id: String,
    pub continuation_node_id: String,
    pub binding: EffectBinding,
    pub operation: O,
    pub required_receipt: ReceiptRequirement,
    pub retry: RetryPolicy,
    pub issued_at: Timestamp,
    pub deadline: Timestamp,
    pub idempotency_key: String,
}

impl EffectIntent<CapabilityOperation> {
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
    pub value: DesiredProjectionValue,
}

impl DesiredField {
    pub fn new(
        selector: Selector,
        value: DesiredProjectionValue,
    ) -> Result<Self, PlannerModelError> {
        validate_selector(&selector)?;
        if selector.contract.package != value.path().package() {
            return Err(PlannerModelError::DesiredContractMismatch);
        }
        Ok(Self { selector, value })
    }

    pub const fn path(&self) -> DesiredProjectionPath {
        self.value.path()
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransitionPlan {
    pub identity: EvaluationIdentity,
    pub outcome: TransitionOutcome,
    pub effects: Vec<EffectIntent>,
    pub timers: Vec<TimerIntent>,
    pub desired: Option<DesiredProjection>,
    pub next_state: PlannerState,
    pub assumptions: Vec<RuntimeAssumption>,
    pub trace_node_ids: Vec<String>,
}

impl TransitionPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: EvaluationIdentity,
        outcome: TransitionOutcome,
        effects: Vec<EffectIntent>,
        timers: Vec<TimerIntent>,
        desired: Option<DesiredProjection>,
        next_state: PlannerState,
        assumptions: Vec<RuntimeAssumption>,
        trace_node_ids: Vec<String>,
    ) -> Result<Self, PlannerModelError> {
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
}

/// Immutable input to one pure planner microstep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerEvaluationInput {
    pub identity: EvaluationIdentity,
    pub event: EvaluationEvent,
    pub evaluation_time: Timestamp,
    pub snapshots: Vec<CapabilitySnapshot>,
    pub effect_bindings: Vec<EffectBinding>,
    pub state: PlannerState,
    #[serde(skip)]
    encoded_bytes: usize,
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
        let mut input = Self {
            identity,
            event,
            evaluation_time,
            snapshots,
            effect_bindings,
            state,
            encoded_bytes: 0,
        };
        input.encoded_bytes = encoded_size(&input);
        if input.encoded_bytes > HARD_MAX_PLAN_INPUT_BYTES {
            return Err(PlannerModelError::EncodedInputTooLarge {
                actual: input.encoded_bytes,
                maximum: HARD_MAX_PLAN_INPUT_BYTES,
            });
        }
        Ok(input)
    }

    pub const fn encoded_bytes(&self) -> usize {
        self.encoded_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PlannerModelError {
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
        StateFieldRef::LevelTransitionRemaining => matches!(value, RuntimeValue::Duration(_)),
        StateFieldRef::ObservationRevision { .. } => matches!(value, RuntimeValue::U64(_)),
        StateFieldRef::LevelCurrentLevel
        | StateFieldRef::LevelTargetLevel
        | StateFieldRef::CoveringCurrentPosition { .. }
        | StateFieldRef::CoveringTargetPosition { .. } => {
            matches!(value, RuntimeValue::Quantity(_))
        }
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
            matches!(value, RuntimeValue::ThermostatTargets(_))
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

fn validate_runtime_value(value: &RuntimeValue) -> Result<(), PlannerModelError> {
    let valid = match value {
        RuntimeValue::String(value) => value.len() <= HARD_MAX_STRING_BYTES,
        RuntimeValue::ThermostatMode(ThermostatModeValue::ProviderQualified(value)) => {
            !value.is_empty() && value.len() <= 64
        }
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

fn selector_identity(selector: &Selector) -> String {
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

const fn receipt_rank(confidence: ReceiptConfidence) -> u8 {
    match confidence {
        ReceiptConfidence::AcceptedForDelivery => 0,
        ReceiptConfidence::ProtocolOrTargetAcknowledged => 1,
        ReceiptConfidence::Completed => 2,
        ReceiptConfidence::ObservedCompleted => 3,
    }
}

const fn desired_path_rank(path: DesiredProjectionPath) -> u8 {
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

    fn identity() -> EvaluationIdentity {
        EvaluationIdentity::new(HEX_32, HEX_32, 1, HEX_64, HEX_64, HEX_32, 1).unwrap()
    }

    fn state() -> PlannerState {
        PlannerState::new(1, None, Vec::new()).unwrap()
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
    fn rejects_invalid_identity_ids_and_digests() {
        assert_eq!(
            EvaluationIdentity::new("not-hex", HEX_32, 1, HEX_64, HEX_64, HEX_32, 1),
            Err(PlannerModelError::InvalidIdentifier("process_epoch"))
        );
        assert_eq!(
            EvaluationIdentity::new(HEX_32, HEX_32, 1, "bad", HEX_64, HEX_32, 1),
            Err(PlannerModelError::InvalidDigest("semantic_sha256"))
        );
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
            Some(BooleanOccurrence::new("event", "source", 1, timestamp(1), Some(true)).unwrap()),
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
        let trigger = TriggerInput::time_schedule("occurrence", timestamp(1)).unwrap();
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
