use std::{cmp::Ordering, collections::BTreeMap};

use cel::Value as CelValue;
use litehouse_contracts::quantity::Quantity;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    compile::{CompiledNode, CompiledQuantityOperation, CompiledRule},
    condition::evaluate_condition,
    model::{
        CompareOperator, EndOutcome, HARD_MAX_EFFECT_INTENTS, HARD_MAX_NODES,
        HARD_MAX_PLAN_INPUT_BYTES, HARD_MAX_STRING_BYTES, RebindPolicy, Selector,
        SelectorCardinality, ValueType, valid_contract_part, valid_hex_32, valid_hex_64, valid_id,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timestamp {
    unix_seconds: i64,
    nanosecond: u32,
}

impl Timestamp {
    pub fn new(unix_seconds: i64, nanosecond: u32) -> Result<Self, InputError> {
        if nanosecond >= 1_000_000_000 {
            return Err(InputError::InvalidTimestamp);
        }
        Ok(Self {
            unix_seconds,
            nanosecond,
        })
    }

    pub fn unix_seconds(self) -> i64 {
        self.unix_seconds
    }

    pub fn nanosecond(self) -> u32 {
        self.nanosecond
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScalarValue {
    Bool { value: bool },
    String { value: String },
    I64 { value: i64 },
    U64 { value: u64 },
    Quantity { value: Quantity },
}

impl ScalarValue {
    fn value_type(&self) -> ValueType {
        match self {
            Self::Bool { .. } => ValueType::Bool,
            Self::String { .. } => ValueType::String,
            Self::I64 { .. } => ValueType::I64,
            Self::U64 { .. } => ValueType::U64,
            Self::Quantity { value } => ValueType::Quantity {
                profile: value.profile_id().as_str().to_owned(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownReason {
    Unavailable,
    Stale,
    Gap,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationErrorCode {
    SourceError,
    InvalidValue,
    TypeMismatch,
    QuantityError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationValue {
    Known { value: ScalarValue },
    Unknown { reason: UnknownReason },
    Error { code: EvaluationErrorCode },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Fresh,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StateObservationInput {
    selector: Selector,
    provider_generation: u64,
    facet_revision: u64,
    observed_at: Timestamp,
}

impl StateObservationInput {
    pub fn new(
        selector: Selector,
        provider_generation: u64,
        facet_revision: u64,
        observed_at: Timestamp,
    ) -> Result<Self, InputError> {
        if provider_generation == 0 || facet_revision == 0 {
            return Err(InputError::ZeroGenerationOrRevision);
        }
        validate_input_selector(&selector)?;
        Ok(Self {
            selector,
            provider_generation,
            facet_revision,
            observed_at,
        })
    }

    pub fn selector(&self) -> &Selector {
        &self.selector
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SnapshotInput {
    node_id: String,
    selector: Selector,
    field: String,
    provider_generation: u64,
    facet_revision: u64,
    observed_at: Timestamp,
    freshness: Freshness,
    value: ObservationValue,
}

impl SnapshotInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        node_id: impl Into<String>,
        selector: Selector,
        field: impl Into<String>,
        provider_generation: u64,
        facet_revision: u64,
        observed_at: Timestamp,
        freshness: Freshness,
        value: ObservationValue,
    ) -> Result<Self, InputError> {
        let node_id = node_id.into();
        if !valid_id(&node_id) {
            return Err(InputError::InvalidNodeId(node_id));
        }
        let field = field.into();
        if !valid_id(&field) {
            return Err(InputError::InvalidSnapshotField(field));
        }
        if provider_generation == 0 || facet_revision == 0 {
            return Err(InputError::ZeroGenerationOrRevision);
        }
        validate_input_selector(&selector)?;
        validate_observation_value(&value)?;
        Ok(Self {
            node_id,
            selector,
            field,
            provider_generation,
            facet_revision,
            observed_at,
            freshness,
            value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetInput {
    selector: Selector,
    provider_generation: u64,
    facet_revision: u64,
    grant_revision: u64,
    absolute_set: bool,
}

impl TargetInput {
    pub fn new(
        selector: Selector,
        provider_generation: u64,
        facet_revision: u64,
        grant_revision: u64,
        absolute_set: bool,
    ) -> Result<Self, InputError> {
        if provider_generation == 0 || facet_revision == 0 || grant_revision == 0 {
            return Err(InputError::ZeroGenerationOrRevision);
        }
        validate_input_selector(&selector)?;
        Ok(Self {
            selector,
            provider_generation,
            facet_revision,
            grant_revision,
            absolute_set,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvaluationInput {
    rule_id: String,
    rule_revision: u64,
    profile_sha256: String,
    evaluation_time: Timestamp,
    trigger: StateObservationInput,
    snapshots: Vec<SnapshotInput>,
    targets: Vec<TargetInput>,
    #[serde(skip)]
    encoded_bytes: usize,
}

impl EvaluationInput {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        rule_id: impl Into<String>,
        rule_revision: u64,
        profile_sha256: impl Into<String>,
        evaluation_time: Timestamp,
        trigger: StateObservationInput,
        snapshots: Vec<SnapshotInput>,
        targets: Vec<TargetInput>,
    ) -> Result<Self, InputError> {
        let rule_id = rule_id.into();
        let profile_sha256 = profile_sha256.into();
        if !valid_hex_32(&rule_id) {
            return Err(InputError::InvalidRuleId);
        }
        if rule_revision == 0 {
            return Err(InputError::InvalidRuleRevision);
        }
        if !valid_hex_64(&profile_sha256) {
            return Err(InputError::InvalidProfileDigest);
        }
        if snapshots.len() > HARD_MAX_NODES {
            return Err(InputError::TooManySnapshots {
                actual: snapshots.len(),
                maximum: HARD_MAX_NODES,
            });
        }
        if targets.len() > HARD_MAX_EFFECT_INTENTS {
            return Err(InputError::TooManyTargets {
                actual: targets.len(),
                maximum: HARD_MAX_EFFECT_INTENTS,
            });
        }
        if trigger.observed_at > evaluation_time
            || snapshots
                .iter()
                .any(|snapshot| snapshot.observed_at > evaluation_time)
        {
            return Err(InputError::ObservationAfterEvaluationTime);
        }
        let mut snapshot_ids = std::collections::BTreeSet::new();
        for snapshot in &snapshots {
            if !snapshot_ids.insert(snapshot.node_id.clone()) {
                return Err(InputError::DuplicateSnapshot(snapshot.node_id.clone()));
            }
        }
        let mut target_selectors = std::collections::BTreeSet::new();
        for target in &targets {
            if !target_selectors.insert(selector_identity(&target.selector)) {
                return Err(InputError::DuplicateTarget);
            }
        }
        let mut input = Self {
            rule_id,
            rule_revision,
            profile_sha256,
            evaluation_time,
            trigger,
            snapshots,
            targets,
            encoded_bytes: 0,
        };
        input.encoded_bytes = serde_json::to_vec(&input)
            .expect("closed evaluation input must serialize")
            .len();
        if input.encoded_bytes > HARD_MAX_PLAN_INPUT_BYTES {
            return Err(InputError::InputTooLarge {
                actual: input.encoded_bytes,
                maximum: HARD_MAX_PLAN_INPUT_BYTES,
            });
        }
        Ok(input)
    }

    pub fn evaluation_time(&self) -> Timestamp {
        self.evaluation_time
    }

    pub fn trigger(&self) -> &StateObservationInput {
        &self.trigger
    }

    pub fn snapshots(&self) -> &[SnapshotInput] {
        &self.snapshots
    }

    pub fn targets(&self) -> &[TargetInput] {
        &self.targets
    }

    pub(crate) fn encoded_bytes(&self) -> usize {
        self.encoded_bytes
    }
}

fn validate_input_selector(selector: &Selector) -> Result<(), InputError> {
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
        Err(InputError::InvalidSelector)
    }
}

fn validate_observation_value(value: &ObservationValue) -> Result<(), InputError> {
    if let ObservationValue::Known {
        value: ScalarValue::String { value },
    } = value
        && value.len() > HARD_MAX_STRING_BYTES
    {
        return Err(InputError::ValueTooLarge {
            actual: value.len(),
            maximum: HARD_MAX_STRING_BYTES,
        });
    }
    let encoded = serde_json::to_vec(value)
        .expect("closed observation value must serialize")
        .len();
    if encoded > HARD_MAX_STRING_BYTES {
        Err(InputError::ValueTooLarge {
            actual: encoded,
            maximum: HARD_MAX_STRING_BYTES,
        })
    } else {
        Ok(())
    }
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum InputError {
    #[error("timestamp nanosecond must be below one billion")]
    InvalidTimestamp,
    #[error("rule ID must be 32 lowercase hexadecimal characters")]
    InvalidRuleId,
    #[error("rule revision must be non-zero")]
    InvalidRuleRevision,
    #[error("profile digest must be 64 lowercase hexadecimal characters")]
    InvalidProfileDigest,
    #[error("generation, facet revision, and grant revision values must be non-zero")]
    ZeroGenerationOrRevision,
    #[error("invalid snapshot node ID {0:?}")]
    InvalidNodeId(String),
    #[error("invalid snapshot field {0:?}")]
    InvalidSnapshotField(String),
    #[error("an observation is later than evaluation_time")]
    ObservationAfterEvaluationTime,
    #[error("duplicate snapshot input for node {0:?}")]
    DuplicateSnapshot(String),
    #[error("duplicate resolved target selector")]
    DuplicateTarget,
    #[error("evaluation selector is invalid")]
    InvalidSelector,
    #[error("evaluation scalar value is {actual} bytes; the maximum is {maximum}")]
    ValueTooLarge { actual: usize, maximum: usize },
    #[error("evaluation has {actual} snapshots; the maximum is {maximum}")]
    TooManySnapshots { actual: usize, maximum: usize },
    #[error("evaluation has {actual} targets; the maximum is {maximum}")]
    TooManyTargets { actual: usize, maximum: usize },
    #[error("encoded evaluation input is {actual} bytes; the maximum is {maximum}")]
    InputTooLarge { actual: usize, maximum: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionPlan {
    rule_id: String,
    rule_revision: u64,
    evaluation_time: Timestamp,
    outcome: PlanOutcome,
    intents: Vec<EffectIntent>,
    assumptions: Vec<Assumption>,
    visited_nodes: Vec<String>,
}

impl TransitionPlan {
    pub fn outcome(&self) -> PlanOutcome {
        self.outcome
    }

    pub fn intents(&self) -> &[EffectIntent] {
        &self.intents
    }

    pub fn assumptions(&self) -> &[Assumption] {
        &self.assumptions
    }

    pub fn visited_nodes(&self) -> &[String] {
        &self.visited_nodes
    }

    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }

    pub fn rule_revision(&self) -> u64 {
        self.rule_revision
    }

    pub fn evaluation_time(&self) -> Timestamp {
        self.evaluation_time
    }

    pub fn check_assumptions(&self, current: &[ObservedRevision]) -> Result<(), AssumptionError> {
        for assumption in &self.assumptions {
            let observed = current
                .iter()
                .find(|observed| observed.selector == assumption.selector)
                .ok_or_else(|| AssumptionError::Missing {
                    selector: selector_label(&assumption.selector),
                })?;
            if observed.provider_generation != assumption.provider_generation
                || observed.facet_revision != assumption.facet_revision
                || assumption
                    .grant_revision
                    .is_some_and(|revision| observed.grant_revision != Some(revision))
                || (assumption.requires_absolute_set && !observed.absolute_set)
            {
                return Err(AssumptionError::Stale {
                    selector: selector_label(&assumption.selector),
                    expected_generation: assumption.provider_generation,
                    actual_generation: observed.provider_generation,
                    expected_revision: assumption.facet_revision,
                    actual_revision: observed.facet_revision,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanOutcome {
    AwaitingReceipt,
    Success,
    Noop,
    Unknown,
    Error,
}

impl From<EndOutcome> for PlanOutcome {
    fn from(value: EndOutcome) -> Self {
        match value {
            EndOutcome::Success => Self::Success,
            EndOutcome::Noop => Self::Noop,
            EndOutcome::Unknown => Self::Unknown,
            EndOutcome::Error => Self::Error,
            EndOutcome::Cancelled | EndOutcome::CapacityRejected => Self::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectIntent {
    pub intent_id: String,
    pub node_id: String,
    pub selector: Selector,
    pub operation: OnOffOperation,
    pub arguments: OnOffSetArguments,
    pub expected_provider_generation: u64,
    pub expected_facet_revision: u64,
    pub grant_revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnOffOperation {
    SetV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnOffSetArguments {
    pub on: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assumption {
    pub kind: AssumptionKind,
    pub selector: Selector,
    pub provider_generation: u64,
    pub facet_revision: u64,
    pub grant_revision: Option<u64>,
    pub requires_absolute_set: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssumptionKind {
    Snapshot,
    EffectTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedRevision {
    pub selector: Selector,
    pub provider_generation: u64,
    pub facet_revision: u64,
    pub grant_revision: Option<u64>,
    pub absolute_set: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AssumptionError {
    #[error("assumption target {selector:?} is no longer present")]
    Missing { selector: String },
    #[error("assumption for {selector:?} is stale")]
    Stale {
        selector: String,
        expected_generation: u64,
        actual_generation: u64,
        expected_revision: u64,
        actual_revision: u64,
    },
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PlanError {
    #[error("evaluation input rule identity does not match the compiled rule")]
    RuleIdentityMismatch,
    #[error("evaluation input rule revision does not match the compiled rule")]
    RuleRevisionMismatch,
    #[error("evaluation input profile digest does not match the compiled rule")]
    ProfileDigestMismatch,
    #[error("trigger selector does not match the compiled trigger")]
    TriggerSelectorMismatch,
    #[error("missing snapshot input for node {node:?}")]
    MissingSnapshot { node: String },
    #[error("snapshot selector does not match node {node:?}")]
    SnapshotSelectorMismatch { node: String },
    #[error("missing resolved effect target for node {node:?}")]
    MissingTarget { node: String },
    #[error("effect target for node {node:?} does not support absolute set")]
    MissingAbsoluteSet { node: String },
    #[error("encoded evaluation input is {actual} bytes; compiled maximum is {maximum}")]
    InputTooLarge { actual: usize, maximum: usize },
    #[error("encoded planner trace is {actual} bytes; compiled maximum is {maximum}")]
    TraceTooLarge { actual: usize, maximum: usize },
    #[error("compiled graph invariant failed at {node:?}: {reason}")]
    InvalidCompiledGraph { node: String, reason: String },
    #[error("planner exceeded the compiled node bound")]
    StepLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ComparisonOutcome {
    True,
    False,
    Unknown(UnknownReason),
    Error(EvaluationErrorCode),
}

pub fn plan(compiled: &CompiledRule, input: &EvaluationInput) -> Result<TransitionPlan, PlanError> {
    if input.encoded_bytes() > compiled.max_plan_input_bytes {
        return Err(PlanError::InputTooLarge {
            actual: input.encoded_bytes(),
            maximum: compiled.max_plan_input_bytes,
        });
    }
    validate_input_identity(compiled, input)?;

    let mut visited_nodes = vec![compiled.trigger_id.clone()];
    validate_trace_size(compiled, &visited_nodes)?;
    let mut intents = Vec::new();
    let mut assumptions = Vec::new();
    let mut snapshots: BTreeMap<String, ObservationValue> = BTreeMap::new();
    let mut comparisons: BTreeMap<String, ComparisonOutcome> = BTreeMap::new();
    let mut current = next(compiled, &compiled.trigger_id, "next")?;

    for _ in 0..HARD_MAX_NODES {
        visited_nodes.push(current.clone());
        validate_trace_size(compiled, &visited_nodes)?;
        let node = compiled
            .nodes
            .get(&current)
            .ok_or_else(|| PlanError::InvalidCompiledGraph {
                node: current.clone(),
                reason: "control edge targets the trigger or an unknown node".to_owned(),
            })?;
        match node {
            CompiledNode::Snapshot {
                selector,
                field,
                value_type,
            } => {
                let snapshot = input
                    .snapshots
                    .iter()
                    .find(|snapshot| snapshot.node_id == current)
                    .ok_or_else(|| PlanError::MissingSnapshot {
                        node: current.clone(),
                    })?;
                if snapshot.selector != *selector || snapshot.field != *field {
                    return Err(PlanError::SnapshotSelectorMismatch {
                        node: current.clone(),
                    });
                }
                let value = if snapshot.freshness == Freshness::Stale {
                    ObservationValue::Unknown {
                        reason: UnknownReason::Stale,
                    }
                } else if matches!(
                    &snapshot.value,
                    ObservationValue::Known { value } if value.value_type() != *value_type
                ) {
                    ObservationValue::Error {
                        code: EvaluationErrorCode::TypeMismatch,
                    }
                } else {
                    snapshot.value.clone()
                };
                snapshots.insert(current.clone(), value);
                assumptions.push(Assumption {
                    kind: AssumptionKind::Snapshot,
                    selector: selector.clone(),
                    provider_generation: snapshot.provider_generation,
                    facet_revision: snapshot.facet_revision,
                    grant_revision: None,
                    requires_absolute_set: false,
                });
                current = next(compiled, &current, "next")?;
            }
            CompiledNode::Condition { program } => {
                let _static_steps = program.static_steps();
                let mut variables = BTreeMap::new();
                let mut outcome = None;
                for declared in program.inputs() {
                    let (source, source_port) = data_source(compiled, &current, &declared.name)?;
                    if source_port != "value" {
                        return Err(PlanError::InvalidCompiledGraph {
                            node: current,
                            reason: "condition input is not a snapshot value".to_owned(),
                        });
                    }
                    let observation =
                        snapshots
                            .get(&source)
                            .ok_or_else(|| PlanError::InvalidCompiledGraph {
                                node: current.clone(),
                                reason: "condition input was not evaluated before CEL".to_owned(),
                            })?;
                    match observation {
                        ObservationValue::Known { value } => match scalar_to_cel(value) {
                            Some(value) => {
                                variables.insert(declared.name.clone(), value);
                            }
                            None => {
                                outcome = Some(ComparisonOutcome::Error(
                                    EvaluationErrorCode::TypeMismatch,
                                ));
                                break;
                            }
                        },
                        ObservationValue::Unknown { reason } => {
                            outcome = Some(ComparisonOutcome::Unknown(*reason));
                            break;
                        }
                        ObservationValue::Error { code } => {
                            outcome = Some(ComparisonOutcome::Error(*code));
                            break;
                        }
                    }
                }
                let result = outcome.unwrap_or_else(|| {
                    match evaluate_condition(
                        program,
                        variables,
                        input.evaluation_time.unix_seconds(),
                        input.evaluation_time.nanosecond(),
                    ) {
                        Ok(true) => ComparisonOutcome::True,
                        Ok(false) => ComparisonOutcome::False,
                        Err(_) => ComparisonOutcome::Error(EvaluationErrorCode::InvalidValue),
                    }
                });
                comparisons.insert(current.clone(), result);
                current = next(compiled, &current, "next")?;
            }
            CompiledNode::Quantity {
                input_port,
                input_profile,
                operation,
            } => {
                let (source, source_port) = data_source(compiled, &current, input_port)?;
                if source_port != "value" {
                    return Err(PlanError::InvalidCompiledGraph {
                        node: current,
                        reason: "quantity input is not a snapshot value".to_owned(),
                    });
                }
                let observation =
                    snapshots
                        .get(&source)
                        .ok_or_else(|| PlanError::InvalidCompiledGraph {
                            node: current.clone(),
                            reason: "snapshot value was not evaluated before quantity operation"
                                .to_owned(),
                        })?;
                match operation {
                    CompiledQuantityOperation::Compare { operator, right } => {
                        comparisons.insert(
                            current.clone(),
                            compare_quantity_observation(
                                observation,
                                input_profile.as_str(),
                                *operator,
                                right,
                            ),
                        );
                    }
                    _ => {
                        return Err(PlanError::InvalidCompiledGraph {
                            node: current,
                            reason: "non-comparison quantity execution belongs to T8.1c".to_owned(),
                        });
                    }
                }
                current = next(compiled, &current, "next")?;
            }
            CompiledNode::Branch => {
                let (source, source_port) = data_source(compiled, &current, "predicate")?;
                if source_port != "result" {
                    return Err(PlanError::InvalidCompiledGraph {
                        node: current,
                        reason: "branch condition is not a comparison result".to_owned(),
                    });
                }
                let result =
                    comparisons
                        .get(&source)
                        .ok_or_else(|| PlanError::InvalidCompiledGraph {
                            node: current.clone(),
                            reason: "comparison was not evaluated before branch".to_owned(),
                        })?;
                let port = match result {
                    ComparisonOutcome::True => "true",
                    ComparisonOutcome::False => "false",
                    ComparisonOutcome::Unknown(reason) => {
                        let _ = reason;
                        "unknown"
                    }
                    ComparisonOutcome::Error(code) => {
                        let _ = code;
                        "error"
                    }
                };
                current = next(compiled, &current, port)?;
            }
            CompiledNode::OnOffSet { selector, value } => {
                let target = input
                    .targets
                    .iter()
                    .find(|target| target.selector == *selector)
                    .ok_or_else(|| PlanError::MissingTarget {
                        node: current.clone(),
                    })?;
                if !target.absolute_set {
                    return Err(PlanError::MissingAbsoluteSet {
                        node: current.clone(),
                    });
                }
                assumptions.push(Assumption {
                    kind: AssumptionKind::EffectTarget,
                    selector: selector.clone(),
                    provider_generation: target.provider_generation,
                    facet_revision: target.facet_revision,
                    grant_revision: Some(target.grant_revision),
                    requires_absolute_set: true,
                });
                intents.push(EffectIntent {
                    intent_id: format!("{}:{}:{}", compiled.rule_id, compiled.revision, current),
                    node_id: current.clone(),
                    selector: selector.clone(),
                    operation: OnOffOperation::SetV1,
                    arguments: OnOffSetArguments { on: *value },
                    expected_provider_generation: target.provider_generation,
                    expected_facet_revision: target.facet_revision,
                    grant_revision: target.grant_revision,
                });
                return Ok(TransitionPlan {
                    rule_id: compiled.rule_id.clone(),
                    rule_revision: compiled.revision,
                    evaluation_time: input.evaluation_time,
                    outcome: PlanOutcome::AwaitingReceipt,
                    intents,
                    assumptions,
                    visited_nodes,
                });
            }
            CompiledNode::ReceiptBranch => {
                return Err(PlanError::InvalidCompiledGraph {
                    node: current,
                    reason:
                        "receipt branches require a later typed receipt microstep owned by T8.1c"
                            .to_owned(),
                });
            }
            CompiledNode::End { outcome } => {
                return Ok(TransitionPlan {
                    rule_id: compiled.rule_id.clone(),
                    rule_revision: compiled.revision,
                    evaluation_time: input.evaluation_time,
                    outcome: (*outcome).into(),
                    intents,
                    assumptions,
                    visited_nodes,
                });
            }
        }
    }
    Err(PlanError::StepLimit)
}

fn validate_trace_size(compiled: &CompiledRule, visited_nodes: &[String]) -> Result<(), PlanError> {
    let actual = serde_json::to_vec(visited_nodes)
        .expect("closed trace must serialize")
        .len();
    if actual > compiled.max_trace_bytes {
        Err(PlanError::TraceTooLarge {
            actual,
            maximum: compiled.max_trace_bytes,
        })
    } else {
        Ok(())
    }
}

fn validate_input_identity(
    compiled: &CompiledRule,
    input: &EvaluationInput,
) -> Result<(), PlanError> {
    if input.rule_id != compiled.rule_id {
        return Err(PlanError::RuleIdentityMismatch);
    }
    if input.rule_revision != compiled.revision {
        return Err(PlanError::RuleRevisionMismatch);
    }
    if input.profile_sha256 != compiled.profile_sha256 {
        return Err(PlanError::ProfileDigestMismatch);
    }
    let trigger_selector =
        compiled
            .trigger_selector
            .as_ref()
            .ok_or_else(|| PlanError::InvalidCompiledGraph {
                node: compiled.trigger_id.clone(),
                reason: "scheduled trigger execution belongs to T8.1c".to_owned(),
            })?;
    if input.trigger.selector != *trigger_selector {
        return Err(PlanError::TriggerSelectorMismatch);
    }
    Ok(())
}

fn next(compiled: &CompiledRule, node: &str, port: &str) -> Result<String, PlanError> {
    compiled
        .outgoing
        .get(&(node.to_owned(), port.to_owned()))
        .cloned()
        .ok_or_else(|| PlanError::InvalidCompiledGraph {
            node: node.to_owned(),
            reason: format!("missing control output {port:?}"),
        })
}

fn data_source(
    compiled: &CompiledRule,
    node: &str,
    port: &str,
) -> Result<(String, String), PlanError> {
    compiled
        .data_sources
        .get(&(node.to_owned(), port.to_owned()))
        .cloned()
        .ok_or_else(|| PlanError::InvalidCompiledGraph {
            node: node.to_owned(),
            reason: format!("missing data input {port:?}"),
        })
}

fn selector_label(selector: &Selector) -> String {
    format!("{}:{}", selector.provider_instance_id, selector.facet_key)
}

fn scalar_to_cel(value: &ScalarValue) -> Option<CelValue> {
    match value {
        ScalarValue::Bool { value } => Some(CelValue::Bool(*value)),
        ScalarValue::String { value } => Some(CelValue::String(value.clone().into())),
        ScalarValue::I64 { value } => Some(CelValue::Int(*value)),
        ScalarValue::U64 { value } => Some(CelValue::UInt(*value)),
        ScalarValue::Quantity { .. } => None,
    }
}

fn compare_quantity_observation(
    observation: &ObservationValue,
    profile: &str,
    operator: CompareOperator,
    right: &Quantity,
) -> ComparisonOutcome {
    let left = match observation {
        ObservationValue::Known { value } => value,
        ObservationValue::Unknown { reason } => return ComparisonOutcome::Unknown(*reason),
        ObservationValue::Error { code } => return ComparisonOutcome::Error(*code),
    };
    let ScalarValue::Quantity { value: left } = left else {
        return ComparisonOutcome::Error(EvaluationErrorCode::TypeMismatch);
    };
    if left.profile_id().as_str() != profile {
        return ComparisonOutcome::Error(EvaluationErrorCode::TypeMismatch);
    }
    let ordering = match left.checked_cmp(right) {
        Ok(ordering) => ordering,
        Err(_) => return ComparisonOutcome::Error(EvaluationErrorCode::QuantityError),
    };
    let result = match operator {
        CompareOperator::Equal => ordering == Ordering::Equal,
        CompareOperator::NotEqual => ordering != Ordering::Equal,
        CompareOperator::Less => ordering == Ordering::Less,
        CompareOperator::LessOrEqual => ordering != Ordering::Greater,
        CompareOperator::Greater => ordering == Ordering::Greater,
        CompareOperator::GreaterOrEqual => ordering != Ordering::Less,
    };
    if result {
        ComparisonOutcome::True
    } else {
        ComparisonOutcome::False
    }
}
