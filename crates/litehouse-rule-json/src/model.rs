use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "1.0.0";
pub const SCHEMA_URL: &str = "https://schemas.litehouse.dev/rule-ir/v1.0.0/schema.json";
pub const COMPILER_PROFILE: &str = "litehouse:rule-compiler@1";
pub const CEL_PROFILE: &str = "litehouse-cel@1";
pub const RUNTIME_PROFILE: &str = "litehouse:rule-runtime@1";
pub const TIMEZONE_DATABASE: &str = "jiff-tzdb@0.1.8+2026c";
pub const CONTRACT_CATALOG_ARTIFACT: &str = "litehouse:contract-catalog@1";
pub const MANAGEMENT_OPERATIONS_ARTIFACT: &str = "litehouse:management-operations@1";
pub const PORT_CATALOG_ARTIFACT: &str =
    "https://schemas.litehouse.dev/rule-ir/v1.0.0/node-port-catalog.json";
pub const HARD_MAX_JSON_BYTES: usize = 128 * 1024;
pub const HARD_MAX_CANONICAL_SEMANTIC_BYTES: usize = 64 * 1024;
pub const HARD_MAX_NODES: usize = 128;
pub const HARD_MAX_EDGES: usize = 256;
pub const HARD_MAX_GRAPH_DEPTH: usize = 32;
pub const HARD_MAX_JSON_DEPTH: usize = 32;
pub const HARD_MAX_STRING_BYTES: usize = 16 * 1024;
pub const HARD_MAX_COLLECTION_ITEMS: usize = 1_024;
pub const HARD_MAX_PARALLEL_BRANCHES: usize = 4;
pub const HARD_MAX_QUEUED_TRIGGERS: usize = 16;
pub const HARD_MAX_CONDITION_NODES: usize = 32;
pub const HARD_MAX_CEL_SOURCE_BYTES: usize = 4 * 1024;
pub const HARD_MAX_CEL_AST_BYTES: usize = 16 * 1024;
pub const HARD_MAX_CEL_STEPS: usize = 10_000;
pub const HARD_MAX_CEL_MEMORY_BYTES: usize = 1024 * 1024;
pub const HARD_MAX_CEL_EVAL_MS: usize = 10;
pub const HARD_MAX_EFFECT_INTENTS: usize = 32;
pub const HARD_MAX_PATCH_OPERATIONS: usize = 128;
pub const HARD_MAX_PATCH_BYTES: usize = 32 * 1024;
pub const HARD_MAX_TIMERS_PER_RULE: usize = 32;
pub const HARD_MAX_TIMER_DURATION_MS: u64 = 24 * 60 * 60 * 1000;
pub const HARD_MAX_PLAN_INPUT_BYTES: usize = 64 * 1024;
pub const HARD_MAX_TRACE_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleDocument {
    pub schema: SchemaLock,
    #[schemars(regex(pattern = "^[0-9a-f]{32}$"))]
    pub rule_id: String,
    #[schemars(range(min = 1))]
    pub revision: u64,
    pub semantic: SemanticRule,
    pub editor: EditorData,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaLock {
    pub url: RuleSchemaUrl,
    pub version: RuleSchemaVersion,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub profile_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum RuleSchemaUrl {
    #[serde(rename = "https://schemas.litehouse.dev/rule-ir/v1.0.0/schema.json")]
    V1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum RuleSchemaVersion {
    #[serde(rename = "1.0.0")]
    V1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticRule {
    #[schemars(length(min = 1, max = 128))]
    pub name: String,
    #[schemars(length(max = 4096))]
    pub description: String,
    pub locks: SemanticLocks,
    pub policies: SemanticPolicies,
    pub limits: RuleLimits,
    #[schemars(length(max = 64))]
    pub annotations: BTreeMap<NamespacedAnnotationKey, AnnotationValue>,
    pub trigger: TriggerNode,
    #[schemars(length(max = 127))]
    pub nodes: Vec<Node>,
    #[schemars(length(max = 256))]
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct NamespacedAnnotationKey(
    #[schemars(regex(pattern = "^[a-z][a-z0-9-]{0,31}:[a-z][a-z0-9_.-]{0,62}$"))] pub String,
);

impl NamespacedAnnotationKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AnnotationValue(#[schemars(length(max = 1024))] pub String);

impl AnnotationValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticLocks {
    pub compiler: CompilerArtifactLock,
    pub cel: CelArtifactLock,
    pub quantity_catalog: QuantityCatalogArtifactLock,
    pub runtime_profile: RuntimeProfileArtifactLock,
    pub timezone_database: TimezoneDatabaseArtifactLock,
    pub contract_catalog: ContractCatalogArtifactLock,
    pub management_operations: ManagementOperationsArtifactLock,
    pub port_catalog: PortCatalogArtifactLock,
}

macro_rules! fixed_artifact_lock {
    ($lock:ident, $id:ident, $wire:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $lock {
            pub id: $id,
            #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
            pub sha256: String,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
        pub enum $id {
            #[serde(rename = $wire)]
            V1,
        }
    };
}

fixed_artifact_lock!(
    CompilerArtifactLock,
    CompilerArtifactId,
    "litehouse:rule-compiler@1"
);
fixed_artifact_lock!(CelArtifactLock, CelArtifactId, "litehouse-cel@1");
fixed_artifact_lock!(
    QuantityCatalogArtifactLock,
    QuantityCatalogArtifactId,
    "litehouse:quantity-catalog@1"
);
fixed_artifact_lock!(
    RuntimeProfileArtifactLock,
    RuntimeProfileArtifactId,
    "litehouse:rule-runtime@1"
);
fixed_artifact_lock!(
    TimezoneDatabaseArtifactLock,
    TimezoneDatabaseArtifactId,
    "jiff-tzdb@0.1.8+2026c"
);
fixed_artifact_lock!(
    ContractCatalogArtifactLock,
    ContractCatalogArtifactId,
    "litehouse:contract-catalog@1"
);
fixed_artifact_lock!(
    ManagementOperationsArtifactLock,
    ManagementOperationsArtifactId,
    "litehouse:management-operations@1"
);
fixed_artifact_lock!(
    PortCatalogArtifactLock,
    PortCatalogArtifactId,
    "https://schemas.litehouse.dev/rule-ir/v1.0.0/node-port-catalog.json"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticPolicies {
    pub activation: ActivationPolicy,
    pub dependency_hydration: DependencyHydrationPolicy,
    pub trigger_queue: TriggerQueuePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActivationPolicy {
    Explicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DependencyHydrationPolicy {
    WaitUntilHydratedOrUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TriggerQueuePolicy {
    DropNewest,
    DropOldest,
    Coalesce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleLimits {
    #[schemars(range(min = 1, max = 128))]
    pub max_nodes: u16,
    #[schemars(range(min = 1, max = 256))]
    pub max_edges: u16,
    #[schemars(range(min = 1, max = 32))]
    pub max_depth: u8,
    #[schemars(range(min = 2, max = 4))]
    pub max_parallel_branches: u8,
    #[schemars(range(min = 1, max = 16))]
    pub max_queued_triggers: u8,
    #[schemars(range(min = 1, max = 32))]
    pub max_condition_nodes: u8,
    #[schemars(range(min = 1, max = 32))]
    pub max_effect_intents: u8,
    #[schemars(range(min = 1, max = 128))]
    pub max_patch_operations: u8,
    #[schemars(range(min = 1, max = 32))]
    pub max_timers: u8,
    #[schemars(range(min = 1, max = 10000))]
    pub max_cel_steps: u16,
    #[schemars(range(min = 1, max = 1048576))]
    pub max_cel_memory_bytes: u32,
    #[schemars(range(min = 1, max = 10))]
    pub max_cel_eval_ms: u8,
    #[schemars(range(min = 1024, max = 65536))]
    pub max_plan_input_bytes: u32,
    #[schemars(range(min = 1024, max = 32768))]
    pub max_trace_bytes: u32,
}

impl RuleLimits {
    pub const fn hard_maxima() -> Self {
        Self {
            max_nodes: HARD_MAX_NODES as u16,
            max_edges: HARD_MAX_EDGES as u16,
            max_depth: HARD_MAX_GRAPH_DEPTH as u8,
            max_parallel_branches: HARD_MAX_PARALLEL_BRANCHES as u8,
            max_queued_triggers: HARD_MAX_QUEUED_TRIGGERS as u8,
            max_condition_nodes: HARD_MAX_CONDITION_NODES as u8,
            max_effect_intents: HARD_MAX_EFFECT_INTENTS as u8,
            max_patch_operations: HARD_MAX_PATCH_OPERATIONS as u8,
            max_timers: HARD_MAX_TIMERS_PER_RULE as u8,
            max_cel_steps: HARD_MAX_CEL_STEPS as u16,
            max_cel_memory_bytes: HARD_MAX_CEL_MEMORY_BYTES as u32,
            max_cel_eval_ms: HARD_MAX_CEL_EVAL_MS as u8,
            max_plan_input_bytes: HARD_MAX_PLAN_INPUT_BYTES as u32,
            max_trace_bytes: HARD_MAX_TRACE_BYTES as u32,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TriggerNode {
    StateObservation {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        selector: Selector,
        mode: ObservationMode,
        initial: InitialObservationPolicy,
    },
    DiscreteEvent {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        selector: Selector,
        event: ContractEventRef,
        deduplication: EventDeduplicationPolicy,
    },
    TimeSchedule {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        schedule: Schedule,
    },
}

impl TriggerNode {
    pub fn id(&self) -> &str {
        match self {
            Self::StateObservation { id, .. }
            | Self::DiscreteEvent { id, .. }
            | Self::TimeSchedule { id, .. } => id,
        }
    }

    pub fn version(&self) -> u8 {
        match self {
            Self::StateObservation { version, .. }
            | Self::DiscreteEvent { version, .. }
            | Self::TimeSchedule { version, .. } => *version,
        }
    }

    pub fn selector(&self) -> Option<&Selector> {
        match self {
            Self::StateObservation { selector, .. } | Self::DiscreteEvent { selector, .. } => {
                Some(selector)
            }
            Self::TimeSchedule { .. } => None,
        }
    }

    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::StateObservation { .. } => "state_observation",
            Self::DiscreteEvent { .. } => "discrete_event",
            Self::TimeSchedule { .. } => "time_schedule",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservationMode {
    AnyObservation,
    ValueChange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InitialObservationPolicy {
    Ignore,
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EventDeduplicationPolicy {
    ProcessEpochAtLeastOnce,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "contract", deny_unknown_fields)]
pub enum ContractEventRef {
    #[serde(rename = "litehouse:on-off@1.0.0")]
    OnOff { event: OnOffEventKind },
    #[serde(rename = "litehouse:boolean-state@1.0.0")]
    BooleanState { event: BooleanStateEventKind },
    #[serde(rename = "litehouse:level-control@1.0.0")]
    LevelControl { event: StateObservationEventKind },
    #[serde(rename = "litehouse:color-control@1.0.0")]
    ColorControl { event: StateObservationEventKind },
    #[serde(rename = "litehouse:measurement@1.0.0")]
    Measurement { event: MeasurementEventKind },
    #[serde(rename = "litehouse:thermostat@1.0.0")]
    Thermostat { event: StateObservationEventKind },
    #[serde(rename = "litehouse:window-covering@1.0.0")]
    WindowCovering { event: StateObservationEventKind },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum OnOffEventKind {
    StateObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum BooleanStateEventKind {
    StateObservation,
    Occurrence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum StateObservationEventKind {
    StateObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum MeasurementEventKind {
    Observation,
}

impl ContractEventRef {
    pub fn selector_contract(&self) -> ContractRef {
        ContractRef::litehouse(self.package(), "capability", 1)
    }

    pub const fn package(&self) -> &'static str {
        match self {
            Self::OnOff { .. } => "on-off",
            Self::BooleanState { .. } => "boolean-state",
            Self::LevelControl { .. } => "level-control",
            Self::ColorControl { .. } => "color-control",
            Self::Measurement { .. } => "measurement",
            Self::Thermostat { .. } => "thermostat",
            Self::WindowCovering { .. } => "window-covering",
        }
    }

    pub const fn event_name(&self) -> &'static str {
        match self {
            Self::BooleanState {
                event: BooleanStateEventKind::Occurrence,
            } => "occurrence",
            Self::Measurement { .. } => "observation",
            _ => "state-observation",
        }
    }

    pub const fn payload_type(&self) -> ContractTypeRef {
        match self {
            Self::OnOff { .. } => ContractTypeRef::OnOffState,
            Self::BooleanState {
                event: BooleanStateEventKind::Occurrence,
            } => ContractTypeRef::BooleanOccurrence,
            Self::BooleanState { .. } => ContractTypeRef::BooleanState,
            Self::LevelControl { .. } => ContractTypeRef::LevelState,
            Self::ColorControl { .. } => ContractTypeRef::ColorState,
            Self::Measurement { .. } => ContractTypeRef::MeasurementState,
            Self::Thermostat { .. } => ContractTypeRef::ThermostatState,
            Self::WindowCovering { .. } => ContractTypeRef::CoveringState,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Schedule {
    OneShot {
        at: TimestampValue,
    },
    FixedInterval {
        #[schemars(range(min = 1, max = 604800000))]
        interval_ms: u64,
    },
    WeeklyCivil {
        #[schemars(length(min = 1, max = 7))]
        days: Vec<Weekday>,
        time: CivilTime,
        #[schemars(length(min = 1, max = 64))]
        timezone: String,
        nonexistent_time: NonexistentTimePolicy,
        duplicate_time: DuplicateTimePolicy,
    },
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CivilTime {
    #[schemars(range(max = 23))]
    pub hour: u8,
    #[schemars(range(max = 59))]
    pub minute: u8,
    #[schemars(range(max = 59))]
    pub second: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NonexistentTimePolicy {
    Skip,
    ShiftForward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DuplicateTimePolicy {
    First,
    Second,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TimestampValue {
    pub unix_seconds: i64,
    #[schemars(range(max = 999999999))]
    pub nanosecond: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Node {
    Snapshot {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        selector: Selector,
        field: StateFieldRef,
    },
    Condition {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        expression: CelCondition,
    },
    Quantity {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        input_profile: QuantityProfileRef,
        operation: QuantityOperation,
    },
    Branch {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
    },
    DesiredPatch {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        snapshot_node_id: String,
        #[schemars(length(min = 1, max = 128))]
        operations: Vec<DesiredPatchOperation>,
    },
    EffectIntent {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        selector: Selector,
        operation: CapabilityOperation,
        #[schemars(range(min = 1, max = 30000))]
        deadline_ms: u32,
        required_receipt: ReceiptRequirement,
        retry: RetryPolicy,
    },
    Delay {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        duration: DurationValue,
    },
    DebounceThrottle {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        trigger: TriggerContextKind,
        key: AdmissionKey,
        policy: AdmissionPolicy,
    },
    ForkJoin {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        phase: ForkJoinPhase,
        conflict_policy: PatchConflictPolicy,
    },
    ReceiptBranch {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        effect_node_id: String,
    },
    End {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        outcome: EndOutcome,
    },
}

impl Node {
    pub fn id(&self) -> &str {
        match self {
            Self::Snapshot { id, .. }
            | Self::Condition { id, .. }
            | Self::Quantity { id, .. }
            | Self::Branch { id, .. }
            | Self::DesiredPatch { id, .. }
            | Self::EffectIntent { id, .. }
            | Self::Delay { id, .. }
            | Self::DebounceThrottle { id, .. }
            | Self::ForkJoin { id, .. }
            | Self::ReceiptBranch { id, .. }
            | Self::End { id, .. } => id,
        }
    }

    pub fn version(&self) -> u8 {
        match self {
            Self::Snapshot { version, .. }
            | Self::Condition { version, .. }
            | Self::Quantity { version, .. }
            | Self::Branch { version, .. }
            | Self::DesiredPatch { version, .. }
            | Self::EffectIntent { version, .. }
            | Self::Delay { version, .. }
            | Self::DebounceThrottle { version, .. }
            | Self::ForkJoin { version, .. }
            | Self::ReceiptBranch { version, .. }
            | Self::End { version, .. } => *version,
        }
    }

    pub const fn kind_name(&self) -> &'static str {
        match self {
            Self::Snapshot { .. } => "snapshot",
            Self::Condition { .. } => "condition",
            Self::Quantity { .. } => "quantity",
            Self::Branch { .. } => "branch",
            Self::DesiredPatch { .. } => "desired_patch",
            Self::EffectIntent { .. } => "effect_intent",
            Self::Delay { .. } => "delay",
            Self::DebounceThrottle { .. } => "debounce_throttle",
            Self::ForkJoin { .. } => "fork_join",
            Self::ReceiptBranch { .. } => "receipt_branch",
            Self::End { .. } => "end",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CelCondition {
    #[schemars(length(min = 1, max = 4096))]
    pub source: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub normalized_source_sha256: String,
    #[schemars(length(min = 1, max = 16))]
    pub inputs: Vec<ConditionInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConditionInput {
    #[schemars(regex(pattern = "^[a-z][a-z0-9_]{0,31}$"))]
    pub name: String,
    pub type_ref: TypeRef,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum QuantityCatalogRef {
    #[serde(rename = "litehouse:quantity-catalog@1")]
    V1,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum QuantityProfileRef {
    #[serde(rename = "litehouse:quantity/temperature-absolute@1")]
    TemperatureAbsolute,
    #[serde(rename = "litehouse:quantity/temperature-difference@1")]
    TemperatureDifference,
    #[serde(rename = "litehouse:quantity/relative-humidity@1")]
    RelativeHumidity,
    #[serde(rename = "litehouse:quantity/ratio@1")]
    Ratio,
    #[serde(rename = "litehouse:quantity/wmo-weather-code@1")]
    WmoWeatherCode,
    #[serde(rename = "litehouse:quantity/pressure@1")]
    Pressure,
    #[serde(rename = "litehouse:quantity/illuminance@1")]
    Illuminance,
    #[serde(rename = "litehouse:quantity/power@1")]
    Power,
    #[serde(rename = "litehouse:quantity/energy@1")]
    Energy,
    #[serde(rename = "litehouse:quantity/voltage@1")]
    Voltage,
    #[serde(rename = "litehouse:quantity/electric-current@1")]
    ElectricCurrent,
    #[serde(rename = "litehouse:quantity/frequency@1")]
    Frequency,
    #[serde(rename = "litehouse:quantity/battery-ratio@1")]
    BatteryRatio,
    #[serde(rename = "litehouse:quantity/signal-strength@1")]
    SignalStrength,
    #[serde(rename = "litehouse:quantity/carbon-dioxide@1")]
    CarbonDioxide,
    #[serde(rename = "litehouse:quantity/carbon-monoxide@1")]
    CarbonMonoxide,
    #[serde(rename = "litehouse:quantity/volatile-organic-compounds@1")]
    VolatileOrganicCompounds,
    #[serde(rename = "litehouse:quantity/particulate-matter-pm1@1")]
    ParticulateMatterPm1,
    #[serde(rename = "litehouse:quantity/particulate-matter-pm2-5@1")]
    ParticulateMatterPm2_5,
    #[serde(rename = "litehouse:quantity/particulate-matter-pm10@1")]
    ParticulateMatterPm10,
    #[serde(rename = "litehouse:quantity/distance@1")]
    Distance,
    #[serde(rename = "litehouse:quantity/speed@1")]
    Speed,
    #[serde(rename = "litehouse:quantity/volume@1")]
    Volume,
    #[serde(rename = "litehouse:quantity/volumetric-flow@1")]
    VolumetricFlow,
}

impl QuantityProfileRef {
    pub const ALL: [Self; 24] = [
        Self::TemperatureAbsolute,
        Self::TemperatureDifference,
        Self::RelativeHumidity,
        Self::Ratio,
        Self::WmoWeatherCode,
        Self::Pressure,
        Self::Illuminance,
        Self::Power,
        Self::Energy,
        Self::Voltage,
        Self::ElectricCurrent,
        Self::Frequency,
        Self::BatteryRatio,
        Self::SignalStrength,
        Self::CarbonDioxide,
        Self::CarbonMonoxide,
        Self::VolatileOrganicCompounds,
        Self::ParticulateMatterPm1,
        Self::ParticulateMatterPm2_5,
        Self::ParticulateMatterPm10,
        Self::Distance,
        Self::Speed,
        Self::Volume,
        Self::VolumetricFlow,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TemperatureAbsolute => "litehouse:quantity/temperature-absolute@1",
            Self::TemperatureDifference => "litehouse:quantity/temperature-difference@1",
            Self::RelativeHumidity => "litehouse:quantity/relative-humidity@1",
            Self::Ratio => "litehouse:quantity/ratio@1",
            Self::WmoWeatherCode => "litehouse:quantity/wmo-weather-code@1",
            Self::Pressure => "litehouse:quantity/pressure@1",
            Self::Illuminance => "litehouse:quantity/illuminance@1",
            Self::Power => "litehouse:quantity/power@1",
            Self::Energy => "litehouse:quantity/energy@1",
            Self::Voltage => "litehouse:quantity/voltage@1",
            Self::ElectricCurrent => "litehouse:quantity/electric-current@1",
            Self::Frequency => "litehouse:quantity/frequency@1",
            Self::BatteryRatio => "litehouse:quantity/battery-ratio@1",
            Self::SignalStrength => "litehouse:quantity/signal-strength@1",
            Self::CarbonDioxide => "litehouse:quantity/carbon-dioxide@1",
            Self::CarbonMonoxide => "litehouse:quantity/carbon-monoxide@1",
            Self::VolatileOrganicCompounds => "litehouse:quantity/volatile-organic-compounds@1",
            Self::ParticulateMatterPm1 => "litehouse:quantity/particulate-matter-pm1@1",
            Self::ParticulateMatterPm2_5 => "litehouse:quantity/particulate-matter-pm2-5@1",
            Self::ParticulateMatterPm10 => "litehouse:quantity/particulate-matter-pm10@1",
            Self::Distance => "litehouse:quantity/distance@1",
            Self::Speed => "litehouse:quantity/speed@1",
            Self::Volume => "litehouse:quantity/volume@1",
            Self::VolumetricFlow => "litehouse:quantity/volumetric-flow@1",
        }
    }

    pub fn snapshot_projection_suffix(self) -> String {
        self.as_str()
            .strip_prefix("litehouse:quantity/")
            .and_then(|value| value.strip_suffix("@1"))
            .expect("closed quantity profile ID")
            .replace('-', "_")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum QuantityOperation {
    Convert {
        #[schemars(length(min = 1, max = 256))]
        target_unit: String,
    },
    Compare {
        operator: CompareOperator,
        right: QuantityWire,
    },
    Add {
        right: QuantityWire,
    },
    Subtract {
        right: QuantityWire,
    },
    Multiply {
        right: QuantityWire,
        result_profile: QuantityProfileRef,
    },
    Divide {
        right: QuantityWire,
        result_profile: QuantityProfileRef,
    },
    Quantize {
        #[schemars(range(max = 18))]
        scale: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompareOperator {
    Equal,
    NotEqual,
    Less,
    LessOrEqual,
    Greater,
    GreaterOrEqual,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum ContractTypeRef {
    #[serde(rename = "litehouse:quantity/types@1/canonical-decimal")]
    CanonicalDecimal,
    #[serde(rename = "litehouse:quantity/types@1/quantity")]
    Quantity,
    #[serde(rename = "litehouse:quantity/types@1/native-representation")]
    NativeRepresentation,
    #[serde(rename = "litehouse:on-off/types@1/state")]
    OnOffState,
    #[serde(rename = "litehouse:boolean-state/types@1/state")]
    BooleanState,
    #[serde(rename = "litehouse:boolean-state/types@1/occurrence")]
    BooleanOccurrence,
    #[serde(rename = "litehouse:level-control/types@1/state")]
    LevelState,
    #[serde(rename = "litehouse:color-control/types@1/state")]
    ColorState,
    #[serde(rename = "litehouse:color-control/types@1/color-value")]
    ColorValue,
    #[serde(rename = "litehouse:color-control/types@1/color-model")]
    ColorModel,
    #[serde(rename = "litehouse:measurement/types@1/state")]
    MeasurementState,
    #[serde(rename = "litehouse:thermostat/types@1/state")]
    ThermostatState,
    #[serde(rename = "litehouse:thermostat/types@1/mode")]
    ThermostatMode,
    #[serde(rename = "litehouse:thermostat/types@1/action")]
    ThermostatAction,
    #[serde(rename = "litehouse:thermostat/types@1/targets")]
    ThermostatTargets,
    #[serde(rename = "litehouse:window-covering/types@1/state")]
    CoveringState,
    #[serde(rename = "litehouse:window-covering/types@1/movement")]
    CoveringMovement,
}

impl ContractTypeRef {
    pub const fn contract(self) -> (&'static str, &'static str, u16, &'static str) {
        match self {
            Self::CanonicalDecimal => ("quantity", "types", 1, "canonical-decimal"),
            Self::Quantity => ("quantity", "types", 1, "quantity"),
            Self::NativeRepresentation => ("quantity", "types", 1, "native-representation"),
            Self::OnOffState => ("on-off", "types", 1, "state"),
            Self::BooleanState => ("boolean-state", "types", 1, "state"),
            Self::BooleanOccurrence => ("boolean-state", "types", 1, "occurrence"),
            Self::LevelState => ("level-control", "types", 1, "state"),
            Self::ColorState => ("color-control", "types", 1, "state"),
            Self::ColorValue => ("color-control", "types", 1, "color-value"),
            Self::ColorModel => ("color-control", "types", 1, "color-model"),
            Self::MeasurementState => ("measurement", "types", 1, "state"),
            Self::ThermostatState => ("thermostat", "types", 1, "state"),
            Self::ThermostatMode => ("thermostat", "types", 1, "mode"),
            Self::ThermostatAction => ("thermostat", "types", 1, "action"),
            Self::ThermostatTargets => ("thermostat", "types", 1, "targets"),
            Self::CoveringState => ("window-covering", "types", 1, "state"),
            Self::CoveringMovement => ("window-covering", "types", 1, "movement"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypeRef {
    Bool,
    String,
    I64,
    U64,
    Timestamp,
    Duration,
    Optional {
        item: Box<TypeRef>,
    },
    List {
        item: Box<TypeRef>,
        #[schemars(range(min = 1, max = 1024))]
        max_items: u16,
    },
    Quantity {
        catalog: QuantityCatalogRef,
        profile: QuantityProfileRef,
    },
    Contract {
        type_ref: ContractTypeRef,
    },
    TriggerContext {
        trigger: TriggerContextKind,
    },
    Receipt {
        operation: CapabilityOperationKind,
    },
    Control,
    PredicateOutcome,
    DesiredPatch,
    EffectIntent,
    ForkToken,
    RunOutcome,
}

impl TypeRef {
    pub const fn scalar_name(&self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::String => "string",
            Self::I64 => "i64",
            Self::U64 => "u64",
            Self::Timestamp => "timestamp",
            Self::Duration => "duration",
            Self::Optional { .. } => "optional",
            Self::List { .. } => "list",
            Self::Quantity { .. } => "quantity",
            Self::Contract { .. } => "contract",
            Self::TriggerContext { .. } => "trigger_context",
            Self::Receipt { .. } => "receipt",
            Self::Control => "control",
            Self::PredicateOutcome => "predicate_outcome",
            Self::DesiredPatch => "desired_patch",
            Self::EffectIntent => "effect_intent",
            Self::ForkToken => "fork_token",
            Self::RunOutcome => "run_outcome",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TriggerContextKind {
    StateObservation,
    DiscreteEvent,
    TimeSchedule,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum StatefulContractKind {
    OnOff,
    BooleanState,
    LevelControl,
    ColorControl,
    Measurement,
    Thermostat,
    WindowCovering,
}

impl StatefulContractKind {
    pub const ALL: [Self; 7] = [
        Self::OnOff,
        Self::BooleanState,
        Self::LevelControl,
        Self::ColorControl,
        Self::Measurement,
        Self::Thermostat,
        Self::WindowCovering,
    ];

    pub const fn package(self) -> &'static str {
        match self {
            Self::OnOff => "on-off",
            Self::BooleanState => "boolean-state",
            Self::LevelControl => "level-control",
            Self::ColorControl => "color-control",
            Self::Measurement => "measurement",
            Self::Thermostat => "thermostat",
            Self::WindowCovering => "window-covering",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StateFieldRef {
    OnOffCurrent,
    BooleanCurrent,
    LevelCurrentLevel,
    LevelTargetLevel,
    LevelTransitionRemaining,
    ColorCurrentColor,
    ColorTargetColor,
    ColorActiveModel,
    MeasurementCurrent { profile: QuantityProfileRef },
    MeasurementNativeRepresentation,
    ThermostatCurrentMode,
    ThermostatTargetMode,
    ThermostatAction,
    ThermostatRunning,
    ThermostatTargets,
    ThermostatFan,
    ThermostatSwing,
    ThermostatPreset,
    CoveringCurrentPosition { axis: CoveringAxis },
    CoveringTargetPosition { axis: CoveringAxis },
    CoveringMovement { axis: CoveringAxis },
    CoveringSafetyStatus,
    CoveringFaultStatus,
    ObservationRevision { contract: StatefulContractKind },
}

impl StateFieldRef {
    pub fn catalog_name(&self) -> String {
        match self {
            Self::OnOffCurrent => "on_off_current".to_owned(),
            Self::BooleanCurrent => "boolean_current".to_owned(),
            Self::LevelCurrentLevel => "level_current_level".to_owned(),
            Self::LevelTargetLevel => "level_target_level".to_owned(),
            Self::LevelTransitionRemaining => "level_transition_remaining".to_owned(),
            Self::ColorCurrentColor => "color_current_color".to_owned(),
            Self::ColorTargetColor => "color_target_color".to_owned(),
            Self::ColorActiveModel => "color_active_model".to_owned(),
            Self::MeasurementCurrent { profile } => {
                format!(
                    "measurement_current_{}",
                    profile.snapshot_projection_suffix()
                )
            }
            Self::MeasurementNativeRepresentation => "measurement_native_representation".to_owned(),
            Self::ThermostatCurrentMode => "thermostat_current_mode".to_owned(),
            Self::ThermostatTargetMode => "thermostat_target_mode".to_owned(),
            Self::ThermostatAction => "thermostat_action".to_owned(),
            Self::ThermostatRunning => "thermostat_running".to_owned(),
            Self::ThermostatTargets => "thermostat_targets".to_owned(),
            Self::ThermostatFan => "thermostat_fan".to_owned(),
            Self::ThermostatSwing => "thermostat_swing".to_owned(),
            Self::ThermostatPreset => "thermostat_preset".to_owned(),
            Self::CoveringCurrentPosition { axis } => {
                format!("covering_{axis:?}_current_position").to_lowercase()
            }
            Self::CoveringTargetPosition { axis } => {
                format!("covering_{axis:?}_target_position").to_lowercase()
            }
            Self::CoveringMovement { axis } => format!("covering_{axis:?}_movement").to_lowercase(),
            Self::CoveringSafetyStatus => "covering_safety_status".to_owned(),
            Self::CoveringFaultStatus => "covering_fault_status".to_owned(),
            Self::ObservationRevision { contract } => {
                format!(
                    "{}_observation_revision",
                    contract.package().replace('-', "_")
                )
            }
        }
    }

    pub fn selector_contract(&self) -> ContractRef {
        let package = match self {
            Self::OnOffCurrent => "on-off",
            Self::BooleanCurrent => "boolean-state",
            Self::LevelCurrentLevel | Self::LevelTargetLevel | Self::LevelTransitionRemaining => {
                "level-control"
            }
            Self::ColorCurrentColor | Self::ColorTargetColor | Self::ColorActiveModel => {
                "color-control"
            }
            Self::MeasurementCurrent { .. } | Self::MeasurementNativeRepresentation => {
                "measurement"
            }
            Self::ThermostatCurrentMode
            | Self::ThermostatTargetMode
            | Self::ThermostatAction
            | Self::ThermostatRunning
            | Self::ThermostatTargets
            | Self::ThermostatFan
            | Self::ThermostatSwing
            | Self::ThermostatPreset => "thermostat",
            Self::CoveringCurrentPosition { .. }
            | Self::CoveringTargetPosition { .. }
            | Self::CoveringMovement { .. }
            | Self::CoveringSafetyStatus
            | Self::CoveringFaultStatus => "window-covering",
            Self::ObservationRevision { contract } => contract.package(),
        };
        ContractRef::litehouse(package, "capability", 1)
    }

    pub fn type_ref(&self) -> TypeRef {
        let optional = |item| TypeRef::Optional {
            item: Box::new(item),
        };
        match self {
            Self::OnOffCurrent | Self::BooleanCurrent | Self::ThermostatRunning => {
                optional(TypeRef::Bool)
            }
            Self::LevelCurrentLevel
            | Self::LevelTargetLevel
            | Self::CoveringCurrentPosition { .. }
            | Self::CoveringTargetPosition { .. } => optional(TypeRef::Contract {
                type_ref: ContractTypeRef::Quantity,
            }),
            Self::LevelTransitionRemaining => optional(TypeRef::U64),
            Self::ColorCurrentColor | Self::ColorTargetColor => optional(TypeRef::Contract {
                type_ref: ContractTypeRef::ColorValue,
            }),
            Self::ColorActiveModel => optional(TypeRef::Contract {
                type_ref: ContractTypeRef::ColorModel,
            }),
            Self::MeasurementCurrent { profile } => optional(TypeRef::Quantity {
                catalog: QuantityCatalogRef::V1,
                profile: *profile,
            }),
            Self::MeasurementNativeRepresentation => optional(TypeRef::Contract {
                type_ref: ContractTypeRef::NativeRepresentation,
            }),
            Self::ThermostatCurrentMode | Self::ThermostatTargetMode => {
                optional(TypeRef::Contract {
                    type_ref: ContractTypeRef::ThermostatMode,
                })
            }
            Self::ThermostatAction => optional(TypeRef::Contract {
                type_ref: ContractTypeRef::ThermostatAction,
            }),
            Self::ThermostatTargets => optional(TypeRef::Contract {
                type_ref: ContractTypeRef::ThermostatTargets,
            }),
            Self::ThermostatFan | Self::ThermostatSwing | Self::ThermostatPreset => {
                optional(TypeRef::String)
            }
            Self::CoveringMovement { .. } => optional(TypeRef::Contract {
                type_ref: ContractTypeRef::CoveringMovement,
            }),
            Self::CoveringSafetyStatus | Self::CoveringFaultStatus => TypeRef::List {
                item: Box::new(TypeRef::String),
                max_items: 64,
            },
            Self::ObservationRevision { .. } => TypeRef::U64,
        }
    }

    pub(crate) fn planner_value_type(&self) -> Option<ValueType> {
        match self {
            Self::OnOffCurrent | Self::BooleanCurrent | Self::ThermostatRunning => {
                Some(ValueType::Bool)
            }
            Self::ThermostatFan | Self::ThermostatSwing | Self::ThermostatPreset => {
                Some(ValueType::String)
            }
            Self::ObservationRevision { .. } | Self::LevelTransitionRemaining => {
                Some(ValueType::U64)
            }
            Self::MeasurementCurrent { profile } => Some(ValueType::Quantity {
                profile: profile.as_str().to_owned(),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum DesiredPatchOperation {
    Test { expected: DesiredProjectionValue },
    Add { value: DesiredProjectionValue },
    Replace { value: DesiredProjectionValue },
    Remove { path: OptionalDesiredProjectionPath },
}

impl DesiredPatchOperation {
    pub const fn path(&self) -> DesiredProjectionPath {
        match self {
            Self::Test { expected } => expected.path(),
            Self::Add { value } | Self::Replace { value } => value.path(),
            Self::Remove { path } => path.required_path(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "path", content = "value", deny_unknown_fields)]
pub enum DesiredProjectionValue {
    #[serde(rename = "/on")]
    On(bool),
    #[serde(rename = "/level")]
    Level(DecimalValue),
    #[serde(rename = "/color")]
    Color(ColorValue),
    #[serde(rename = "/thermostat/mode")]
    ThermostatMode(ThermostatMode),
    #[serde(rename = "/thermostat/single_target")]
    ThermostatSingleTarget(DecimalValue),
    #[serde(rename = "/thermostat/heat_target")]
    ThermostatHeatTarget(DecimalValue),
    #[serde(rename = "/thermostat/cool_target")]
    ThermostatCoolTarget(DecimalValue),
    #[serde(rename = "/thermostat/fan")]
    ThermostatFan(ChoiceValue),
    #[serde(rename = "/thermostat/swing")]
    ThermostatSwing(ChoiceValue),
    #[serde(rename = "/thermostat/preset")]
    ThermostatPreset(ChoiceValue),
    #[serde(rename = "/covering/lift")]
    CoveringLift(DecimalValue),
    #[serde(rename = "/covering/tilt")]
    CoveringTilt(DecimalValue),
}

impl DesiredProjectionValue {
    pub const fn path(&self) -> DesiredProjectionPath {
        match self {
            Self::On(_) => DesiredProjectionPath::On,
            Self::Level(_) => DesiredProjectionPath::Level,
            Self::Color(_) => DesiredProjectionPath::Color,
            Self::ThermostatMode(_) => DesiredProjectionPath::ThermostatMode,
            Self::ThermostatSingleTarget(_) => DesiredProjectionPath::ThermostatSingleTarget,
            Self::ThermostatHeatTarget(_) => DesiredProjectionPath::ThermostatHeatTarget,
            Self::ThermostatCoolTarget(_) => DesiredProjectionPath::ThermostatCoolTarget,
            Self::ThermostatFan(_) => DesiredProjectionPath::ThermostatFan,
            Self::ThermostatSwing(_) => DesiredProjectionPath::ThermostatSwing,
            Self::ThermostatPreset(_) => DesiredProjectionPath::ThermostatPreset,
            Self::CoveringLift(_) => DesiredProjectionPath::CoveringLift,
            Self::CoveringTilt(_) => DesiredProjectionPath::CoveringTilt,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum DesiredProjectionPath {
    #[serde(rename = "/on")]
    On,
    #[serde(rename = "/level")]
    Level,
    #[serde(rename = "/color")]
    Color,
    #[serde(rename = "/thermostat/mode")]
    ThermostatMode,
    #[serde(rename = "/thermostat/single_target")]
    ThermostatSingleTarget,
    #[serde(rename = "/thermostat/heat_target")]
    ThermostatHeatTarget,
    #[serde(rename = "/thermostat/cool_target")]
    ThermostatCoolTarget,
    #[serde(rename = "/thermostat/fan")]
    ThermostatFan,
    #[serde(rename = "/thermostat/swing")]
    ThermostatSwing,
    #[serde(rename = "/thermostat/preset")]
    ThermostatPreset,
    #[serde(rename = "/covering/lift")]
    CoveringLift,
    #[serde(rename = "/covering/tilt")]
    CoveringTilt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum OptionalDesiredProjectionPath {
    #[serde(rename = "/level")]
    Level,
    #[serde(rename = "/color")]
    Color,
    #[serde(rename = "/thermostat/single_target")]
    ThermostatSingleTarget,
    #[serde(rename = "/thermostat/heat_target")]
    ThermostatHeatTarget,
    #[serde(rename = "/thermostat/cool_target")]
    ThermostatCoolTarget,
    #[serde(rename = "/thermostat/fan")]
    ThermostatFan,
    #[serde(rename = "/thermostat/swing")]
    ThermostatSwing,
    #[serde(rename = "/thermostat/preset")]
    ThermostatPreset,
    #[serde(rename = "/covering/lift")]
    CoveringLift,
    #[serde(rename = "/covering/tilt")]
    CoveringTilt,
}

impl DesiredProjectionPath {
    pub const ALL: [Self; 12] = [
        Self::On,
        Self::Level,
        Self::Color,
        Self::ThermostatMode,
        Self::ThermostatSingleTarget,
        Self::ThermostatHeatTarget,
        Self::ThermostatCoolTarget,
        Self::ThermostatFan,
        Self::ThermostatSwing,
        Self::ThermostatPreset,
        Self::CoveringLift,
        Self::CoveringTilt,
    ];

    pub const fn package(self) -> &'static str {
        match self {
            Self::On => "on-off",
            Self::Level => "level-control",
            Self::Color => "color-control",
            Self::ThermostatMode
            | Self::ThermostatSingleTarget
            | Self::ThermostatHeatTarget
            | Self::ThermostatCoolTarget
            | Self::ThermostatFan
            | Self::ThermostatSwing
            | Self::ThermostatPreset => "thermostat",
            Self::CoveringLift | Self::CoveringTilt => "window-covering",
        }
    }
}

impl OptionalDesiredProjectionPath {
    pub const fn required_path(self) -> DesiredProjectionPath {
        match self {
            Self::Level => DesiredProjectionPath::Level,
            Self::Color => DesiredProjectionPath::Color,
            Self::ThermostatSingleTarget => DesiredProjectionPath::ThermostatSingleTarget,
            Self::ThermostatHeatTarget => DesiredProjectionPath::ThermostatHeatTarget,
            Self::ThermostatCoolTarget => DesiredProjectionPath::ThermostatCoolTarget,
            Self::ThermostatFan => DesiredProjectionPath::ThermostatFan,
            Self::ThermostatSwing => DesiredProjectionPath::ThermostatSwing,
            Self::ThermostatPreset => DesiredProjectionPath::ThermostatPreset,
            Self::CoveringLift => DesiredProjectionPath::CoveringLift,
            Self::CoveringTilt => DesiredProjectionPath::CoveringTilt,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum CapabilityOperationKind {
    #[serde(rename = "capability.on-off.set@1")]
    OnOffSet,
    #[serde(rename = "capability.on-off.toggle@1")]
    OnOffToggle,
    #[serde(rename = "capability.level.set@1")]
    LevelSet,
    #[serde(rename = "capability.level.step@1")]
    LevelStep,
    #[serde(rename = "capability.level.move@1")]
    LevelMove,
    #[serde(rename = "capability.level.stop@1")]
    LevelStop,
    #[serde(rename = "capability.level.transition@1")]
    LevelTransition,
    #[serde(rename = "capability.level.set-coupled@1")]
    LevelSetCoupled,
    #[serde(rename = "capability.level.step-coupled@1")]
    LevelStepCoupled,
    #[serde(rename = "capability.level.move-coupled@1")]
    LevelMoveCoupled,
    #[serde(rename = "capability.level.transition-coupled@1")]
    LevelTransitionCoupled,
    #[serde(rename = "capability.color.set@1")]
    ColorSet,
    #[serde(rename = "capability.color.step@1")]
    ColorStep,
    #[serde(rename = "capability.color.move@1")]
    ColorMove,
    #[serde(rename = "capability.color.stop@1")]
    ColorStop,
    #[serde(rename = "capability.color.transition@1")]
    ColorTransition,
    #[serde(rename = "capability.thermostat.set-mode@1")]
    ThermostatSetMode,
    #[serde(rename = "capability.thermostat.set-single-target@1")]
    ThermostatSetSingleTarget,
    #[serde(rename = "capability.thermostat.set-dual-target@1")]
    ThermostatSetDualTarget,
    #[serde(rename = "capability.thermostat.set-fan@1")]
    ThermostatSetFan,
    #[serde(rename = "capability.thermostat.set-swing@1")]
    ThermostatSetSwing,
    #[serde(rename = "capability.thermostat.set-preset@1")]
    ThermostatSetPreset,
    #[serde(rename = "capability.covering.open@1")]
    CoveringOpen,
    #[serde(rename = "capability.covering.close@1")]
    CoveringClose,
    #[serde(rename = "capability.covering.stop@1")]
    CoveringStop,
    #[serde(rename = "capability.covering.set-lift@1")]
    CoveringSetLift,
    #[serde(rename = "capability.covering.set-tilt@1")]
    CoveringSetTilt,
    #[serde(rename = "capability.momentary.invoke@1")]
    MomentaryInvoke,
    #[serde(rename = "capability.notification.send@1")]
    NotificationSend,
}

impl CapabilityOperationKind {
    pub const ALL: [Self; 29] = [
        Self::OnOffSet,
        Self::OnOffToggle,
        Self::LevelSet,
        Self::LevelStep,
        Self::LevelMove,
        Self::LevelStop,
        Self::LevelTransition,
        Self::LevelSetCoupled,
        Self::LevelStepCoupled,
        Self::LevelMoveCoupled,
        Self::LevelTransitionCoupled,
        Self::ColorSet,
        Self::ColorStep,
        Self::ColorMove,
        Self::ColorStop,
        Self::ColorTransition,
        Self::ThermostatSetMode,
        Self::ThermostatSetSingleTarget,
        Self::ThermostatSetDualTarget,
        Self::ThermostatSetFan,
        Self::ThermostatSetSwing,
        Self::ThermostatSetPreset,
        Self::CoveringOpen,
        Self::CoveringClose,
        Self::CoveringStop,
        Self::CoveringSetLift,
        Self::CoveringSetTilt,
        Self::MomentaryInvoke,
        Self::NotificationSend,
    ];

    pub const fn operation_id(self) -> &'static str {
        match self {
            Self::OnOffSet => "capability.on-off.set@1",
            Self::OnOffToggle => "capability.on-off.toggle@1",
            Self::LevelSet => "capability.level.set@1",
            Self::LevelStep => "capability.level.step@1",
            Self::LevelMove => "capability.level.move@1",
            Self::LevelStop => "capability.level.stop@1",
            Self::LevelTransition => "capability.level.transition@1",
            Self::LevelSetCoupled => "capability.level.set-coupled@1",
            Self::LevelStepCoupled => "capability.level.step-coupled@1",
            Self::LevelMoveCoupled => "capability.level.move-coupled@1",
            Self::LevelTransitionCoupled => "capability.level.transition-coupled@1",
            Self::ColorSet => "capability.color.set@1",
            Self::ColorStep => "capability.color.step@1",
            Self::ColorMove => "capability.color.move@1",
            Self::ColorStop => "capability.color.stop@1",
            Self::ColorTransition => "capability.color.transition@1",
            Self::ThermostatSetMode => "capability.thermostat.set-mode@1",
            Self::ThermostatSetSingleTarget => "capability.thermostat.set-single-target@1",
            Self::ThermostatSetDualTarget => "capability.thermostat.set-dual-target@1",
            Self::ThermostatSetFan => "capability.thermostat.set-fan@1",
            Self::ThermostatSetSwing => "capability.thermostat.set-swing@1",
            Self::ThermostatSetPreset => "capability.thermostat.set-preset@1",
            Self::CoveringOpen => "capability.covering.open@1",
            Self::CoveringClose => "capability.covering.close@1",
            Self::CoveringStop => "capability.covering.stop@1",
            Self::CoveringSetLift => "capability.covering.set-lift@1",
            Self::CoveringSetTilt => "capability.covering.set-tilt@1",
            Self::MomentaryInvoke => "capability.momentary.invoke@1",
            Self::NotificationSend => "capability.notification.send@1",
        }
    }

    pub const fn package(self) -> &'static str {
        match self {
            Self::OnOffSet | Self::OnOffToggle => "on-off",
            Self::LevelSet
            | Self::LevelStep
            | Self::LevelMove
            | Self::LevelStop
            | Self::LevelTransition
            | Self::LevelSetCoupled
            | Self::LevelStepCoupled
            | Self::LevelMoveCoupled
            | Self::LevelTransitionCoupled => "level-control",
            Self::ColorSet
            | Self::ColorStep
            | Self::ColorMove
            | Self::ColorStop
            | Self::ColorTransition => "color-control",
            Self::ThermostatSetMode
            | Self::ThermostatSetSingleTarget
            | Self::ThermostatSetDualTarget
            | Self::ThermostatSetFan
            | Self::ThermostatSetSwing
            | Self::ThermostatSetPreset => "thermostat",
            Self::CoveringOpen
            | Self::CoveringClose
            | Self::CoveringStop
            | Self::CoveringSetLift
            | Self::CoveringSetTilt => "window-covering",
            Self::MomentaryInvoke => "momentary-action",
            Self::NotificationSend => "notification-sink",
        }
    }

    pub const fn retry_eligible(self) -> bool {
        matches!(
            self,
            Self::OnOffSet
                | Self::LevelSet
                | Self::LevelTransition
                | Self::LevelSetCoupled
                | Self::LevelTransitionCoupled
                | Self::ColorSet
                | Self::ColorTransition
                | Self::ThermostatSetMode
                | Self::ThermostatSetSingleTarget
                | Self::ThermostatSetDualTarget
                | Self::ThermostatSetFan
                | Self::ThermostatSetSwing
                | Self::ThermostatSetPreset
                | Self::CoveringSetLift
                | Self::CoveringSetTilt
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", deny_unknown_fields)]
pub enum CapabilityOperation {
    #[serde(rename = "capability.on-off.set@1")]
    OnOffSet { on: bool },
    #[serde(rename = "capability.on-off.toggle@1")]
    OnOffToggle,
    #[serde(rename = "capability.level.set@1")]
    LevelSet { level: DecimalValue },
    #[serde(rename = "capability.level.step@1")]
    LevelStep { step: DecimalValue },
    #[serde(rename = "capability.level.move@1")]
    LevelMove {
        direction: LevelDirection,
        rate_per_second: DecimalValue,
    },
    #[serde(rename = "capability.level.stop@1")]
    LevelStop,
    #[serde(rename = "capability.level.transition@1")]
    LevelTransition {
        level: DecimalValue,
        #[schemars(range(min = 1, max = 30000))]
        duration_ms: u32,
    },
    #[serde(rename = "capability.level.set-coupled@1")]
    LevelSetCoupled {
        level: DecimalValue,
        on_when_nonzero: bool,
    },
    #[serde(rename = "capability.level.step-coupled@1")]
    LevelStepCoupled {
        step: DecimalValue,
        on_when_nonzero: bool,
    },
    #[serde(rename = "capability.level.move-coupled@1")]
    LevelMoveCoupled {
        direction: LevelDirection,
        rate_per_second: DecimalValue,
        on_when_nonzero: bool,
    },
    #[serde(rename = "capability.level.transition-coupled@1")]
    LevelTransitionCoupled {
        level: DecimalValue,
        #[schemars(range(min = 1, max = 30000))]
        duration_ms: u32,
        on_when_nonzero: bool,
    },
    #[serde(rename = "capability.color.set@1")]
    ColorSet { color: ColorValue },
    #[serde(rename = "capability.color.step@1")]
    ColorStep {
        color: ColorValue,
        direction: ColorDirection,
    },
    #[serde(rename = "capability.color.move@1")]
    ColorMove {
        color: ColorValue,
        direction: ColorDirection,
    },
    #[serde(rename = "capability.color.stop@1")]
    ColorStop,
    #[serde(rename = "capability.color.transition@1")]
    ColorTransition {
        color: ColorValue,
        #[schemars(range(min = 1, max = 30000))]
        duration_ms: u32,
    },
    #[serde(rename = "capability.thermostat.set-mode@1")]
    ThermostatSetMode { mode: ThermostatMode },
    #[serde(rename = "capability.thermostat.set-single-target@1")]
    ThermostatSetSingleTarget { target_celsius: DecimalValue },
    #[serde(rename = "capability.thermostat.set-dual-target@1")]
    ThermostatSetDualTarget {
        heat_target_celsius: DecimalValue,
        cool_target_celsius: DecimalValue,
    },
    #[serde(rename = "capability.thermostat.set-fan@1")]
    ThermostatSetFan { fan_choice: ChoiceValue },
    #[serde(rename = "capability.thermostat.set-swing@1")]
    ThermostatSetSwing { swing_choice: ChoiceValue },
    #[serde(rename = "capability.thermostat.set-preset@1")]
    ThermostatSetPreset { preset_choice: ChoiceValue },
    #[serde(rename = "capability.covering.open@1")]
    CoveringOpen { axis: CoveringAxis },
    #[serde(rename = "capability.covering.close@1")]
    CoveringClose { axis: CoveringAxis },
    #[serde(rename = "capability.covering.stop@1")]
    CoveringStop { axis: CoveringAxis },
    #[serde(rename = "capability.covering.set-lift@1")]
    CoveringSetLift { position: DecimalValue },
    #[serde(rename = "capability.covering.set-tilt@1")]
    CoveringSetTilt { position: DecimalValue },
    #[serde(rename = "capability.momentary.invoke@1")]
    MomentaryInvoke,
    #[serde(rename = "capability.notification.send@1")]
    NotificationSend {
        #[schemars(regex(pattern = "^[A-Za-z0-9][A-Za-z0-9_-]{0,127}$"))]
        notification_template_id: String,
        priority: NotificationPriority,
        #[schemars(range(min = 1, max = 86400000))]
        expires_in_ms: Option<u32>,
    },
}

impl CapabilityOperation {
    pub const fn kind(&self) -> CapabilityOperationKind {
        match self {
            Self::OnOffSet { .. } => CapabilityOperationKind::OnOffSet,
            Self::OnOffToggle => CapabilityOperationKind::OnOffToggle,
            Self::LevelSet { .. } => CapabilityOperationKind::LevelSet,
            Self::LevelStep { .. } => CapabilityOperationKind::LevelStep,
            Self::LevelMove { .. } => CapabilityOperationKind::LevelMove,
            Self::LevelStop => CapabilityOperationKind::LevelStop,
            Self::LevelTransition { .. } => CapabilityOperationKind::LevelTransition,
            Self::LevelSetCoupled { .. } => CapabilityOperationKind::LevelSetCoupled,
            Self::LevelStepCoupled { .. } => CapabilityOperationKind::LevelStepCoupled,
            Self::LevelMoveCoupled { .. } => CapabilityOperationKind::LevelMoveCoupled,
            Self::LevelTransitionCoupled { .. } => CapabilityOperationKind::LevelTransitionCoupled,
            Self::ColorSet { .. } => CapabilityOperationKind::ColorSet,
            Self::ColorStep { .. } => CapabilityOperationKind::ColorStep,
            Self::ColorMove { .. } => CapabilityOperationKind::ColorMove,
            Self::ColorStop => CapabilityOperationKind::ColorStop,
            Self::ColorTransition { .. } => CapabilityOperationKind::ColorTransition,
            Self::ThermostatSetMode { .. } => CapabilityOperationKind::ThermostatSetMode,
            Self::ThermostatSetSingleTarget { .. } => {
                CapabilityOperationKind::ThermostatSetSingleTarget
            }
            Self::ThermostatSetDualTarget { .. } => {
                CapabilityOperationKind::ThermostatSetDualTarget
            }
            Self::ThermostatSetFan { .. } => CapabilityOperationKind::ThermostatSetFan,
            Self::ThermostatSetSwing { .. } => CapabilityOperationKind::ThermostatSetSwing,
            Self::ThermostatSetPreset { .. } => CapabilityOperationKind::ThermostatSetPreset,
            Self::CoveringOpen { .. } => CapabilityOperationKind::CoveringOpen,
            Self::CoveringClose { .. } => CapabilityOperationKind::CoveringClose,
            Self::CoveringStop { .. } => CapabilityOperationKind::CoveringStop,
            Self::CoveringSetLift { .. } => CapabilityOperationKind::CoveringSetLift,
            Self::CoveringSetTilt { .. } => CapabilityOperationKind::CoveringSetTilt,
            Self::MomentaryInvoke => CapabilityOperationKind::MomentaryInvoke,
            Self::NotificationSend { .. } => CapabilityOperationKind::NotificationSend,
        }
    }

    pub const fn operation_id(&self) -> &'static str {
        match self {
            Self::OnOffSet { .. } => "capability.on-off.set@1",
            Self::OnOffToggle => "capability.on-off.toggle@1",
            Self::LevelSet { .. } => "capability.level.set@1",
            Self::LevelStep { .. } => "capability.level.step@1",
            Self::LevelMove { .. } => "capability.level.move@1",
            Self::LevelStop => "capability.level.stop@1",
            Self::LevelTransition { .. } => "capability.level.transition@1",
            Self::LevelSetCoupled { .. } => "capability.level.set-coupled@1",
            Self::LevelStepCoupled { .. } => "capability.level.step-coupled@1",
            Self::LevelMoveCoupled { .. } => "capability.level.move-coupled@1",
            Self::LevelTransitionCoupled { .. } => "capability.level.transition-coupled@1",
            Self::ColorSet { .. } => "capability.color.set@1",
            Self::ColorStep { .. } => "capability.color.step@1",
            Self::ColorMove { .. } => "capability.color.move@1",
            Self::ColorStop => "capability.color.stop@1",
            Self::ColorTransition { .. } => "capability.color.transition@1",
            Self::ThermostatSetMode { .. } => "capability.thermostat.set-mode@1",
            Self::ThermostatSetSingleTarget { .. } => "capability.thermostat.set-single-target@1",
            Self::ThermostatSetDualTarget { .. } => "capability.thermostat.set-dual-target@1",
            Self::ThermostatSetFan { .. } => "capability.thermostat.set-fan@1",
            Self::ThermostatSetSwing { .. } => "capability.thermostat.set-swing@1",
            Self::ThermostatSetPreset { .. } => "capability.thermostat.set-preset@1",
            Self::CoveringOpen { .. } => "capability.covering.open@1",
            Self::CoveringClose { .. } => "capability.covering.close@1",
            Self::CoveringStop { .. } => "capability.covering.stop@1",
            Self::CoveringSetLift { .. } => "capability.covering.set-lift@1",
            Self::CoveringSetTilt { .. } => "capability.covering.set-tilt@1",
            Self::MomentaryInvoke => "capability.momentary.invoke@1",
            Self::NotificationSend { .. } => "capability.notification.send@1",
        }
    }

    pub const fn is_notification(&self) -> bool {
        matches!(self, Self::NotificationSend { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct DecimalValue(
    #[schemars(
        regex(pattern = "^-?(0|[1-9][0-9]*)(\\.[0-9]+)?$"),
        length(min = 1, max = 32)
    )]
    pub String,
);

impl DecimalValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ChoiceValue(#[schemars(length(min = 1, max = 64))] pub String);

impl ChoiceValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "model", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ColorValue {
    HueSaturation {
        hue_degrees: DecimalValue,
        saturation: DecimalValue,
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
        color_space: ColorSpace,
    },
    UncalibratedRgb {
        red: DecimalValue,
        green: DecimalValue,
        blue: DecimalValue,
    },
    NativeWhiteChannels {
        white: DecimalValue,
        cold_white: Option<DecimalValue>,
        warm_white: Option<DecimalValue>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ColorSpace {
    Srgb,
    DisplayP3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LevelDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ColorDirection {
    Increase,
    Decrease,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ThermostatMode {
    Off,
    Heat,
    Cool,
    Auto,
    Dry,
    FanOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoveringAxis {
    Lift,
    Tilt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptRequirement {
    AcceptedForDelivery,
    NotificationAdvertisedMaximum,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptConfidence {
    AcceptedForDelivery,
    ProtocolOrTargetAcknowledged,
    Completed,
    ObservedCompleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptOutcome {
    AcceptedForDelivery,
    ProtocolOrTargetAcknowledged,
    Completed,
    ObservedCompleted,
    Rejected,
    Error,
    Cancelled,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RetryPolicy {
    None,
    DefinitelyPreDispatch {
        #[schemars(range(min = 2, max = 3))]
        max_attempts: u8,
        #[schemars(range(min = 1, max = 30000))]
        backoff_ms: u32,
        provider_deduplication: ProviderDeduplicationRequirement,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDeduplicationRequirement {
    MatchingProviderGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DurationValue {
    #[schemars(range(min = 1, max = 86400000))]
    pub milliseconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionKey {
    RuleTrigger,
    RuleTriggerAndEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AdmissionPolicy {
    DebounceTrailing {
        #[schemars(range(min = 1, max = 86400000))]
        window_ms: u64,
    },
    ThrottleLeading {
        #[schemars(range(min = 1, max = 86400000))]
        window_ms: u64,
        trailing: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ForkJoinPhase {
    Fork {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        join_node_id: String,
        #[schemars(length(min = 2, max = 4))]
        branches: Vec<ForkBranch>,
    },
    Join {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        fork_node_id: String,
        #[schemars(length(min = 2, max = 4))]
        branches: Vec<ForkBranch>,
    },
}

impl ForkJoinPhase {
    pub fn branches(&self) -> &[ForkBranch] {
        match self {
            Self::Fork { branches, .. } | Self::Join { branches, .. } => branches,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ForkBranch {
    #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,31}$"))]
    pub id: String,
    #[schemars(range(min = 0, max = 3))]
    pub order: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PatchConflictPolicy {
    RejectConflict,
    OrderedLastWins,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuantityWire {
    pub profile: QuantityProfileRef,
    #[schemars(
        regex(pattern = "^-?(0|[1-9][0-9]*)(\\.[0-9]*[1-9])?$"),
        length(min = 1, max = 96)
    )]
    pub magnitude: String,
    #[schemars(length(min = 1, max = 256))]
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    #[schemars(regex(pattern = "^[0-9a-f]{32}$"))]
    pub provider_instance_id: String,
    #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
    pub facet_key: String,
    pub contract: ContractRef,
    pub cardinality: SelectorCardinality,
    pub rebind: RebindPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SelectorCardinality {
    One,
    Optional,
    Many,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RebindPolicy {
    Never,
    SameIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractRef {
    #[schemars(regex(pattern = "^[a-z][a-z0-9-]{0,63}$"))]
    pub namespace: String,
    #[schemars(regex(pattern = "^[a-z][a-z0-9-]{0,63}$"))]
    pub package: String,
    #[schemars(regex(pattern = "^[a-z][a-z0-9-]{0,63}$"))]
    pub interface: String,
    #[schemars(range(min = 1))]
    pub major: u16,
}

impl ContractRef {
    pub fn litehouse(package: &str, interface: &str, major: u16) -> Self {
        Self {
            namespace: "litehouse".to_owned(),
            package: package.to_owned(),
            interface: interface.to_owned(),
            major,
        }
    }

    pub fn is_on_off_v1(&self) -> bool {
        self.namespace == "litehouse"
            && self.package == "on-off"
            && self.interface == "capability"
            && self.major == 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
    pub id: String,
    pub from: EdgeEndpoint,
    pub to: EdgeEndpoint,
    #[schemars(range(max = 255))]
    pub order: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeEndpoint {
    #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
    pub node: String,
    #[schemars(regex(pattern = "^[a-z][a-z0-9_:-]{0,63}$"))]
    pub port: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndOutcome {
    Success,
    Noop,
    Unknown,
    Error,
    Cancelled,
    CapacityRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditorData {
    #[schemars(range(min = 1))]
    pub revision: u64,
    #[schemars(length(max = 128))]
    pub nodes: BTreeMap<EditorNodeKey, EditorNodeData>,
    #[schemars(length(max = 32))]
    pub groups: BTreeMap<EditorGroupKey, EditorGroupData>,
    pub viewport: EditorViewport,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct EditorNodeKey(#[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))] pub String);

impl EditorNodeKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct EditorGroupKey(#[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))] pub String);

impl EditorGroupKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditorNodeData {
    pub x: i32,
    pub y: i32,
    pub collapsed: bool,
    pub group: Option<EditorGroupKey>,
    pub presentation: EditorPresentation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditorGroupData {
    #[schemars(length(min = 1, max = 128))]
    pub label: String,
    pub x: i32,
    pub y: i32,
    #[schemars(range(min = 1, max = 100000))]
    pub width: u32,
    #[schemars(range(min = 1, max = 100000))]
    pub height: u32,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EditorPresentation {
    Standard,
    Compact,
    Emphasized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditorViewport {
    pub x: i32,
    pub y: i32,
    #[schemars(range(min = 100, max = 4000))]
    pub zoom_milli: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ValueType {
    Bool,
    String,
    I64,
    U64,
    Quantity { profile: String },
}

impl ValueType {
    pub(crate) const fn name(&self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::String => "string",
            Self::I64 => "i64",
            Self::U64 => "u64",
            Self::Quantity { .. } => "quantity",
        }
    }
}

pub(crate) fn valid_id(value: &str) -> bool {
    value.len() <= 64
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

pub(crate) fn valid_hex_32(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn valid_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn valid_contract_part(value: &str) -> bool {
    value.len() <= 64
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(crate) fn valid_decimal_shape(value: &str) -> bool {
    if value.is_empty() || value.len() > 96 || value == "-0" {
        return false;
    }
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let mut pieces = unsigned.split('.');
    let integer = pieces.next().unwrap_or_default();
    let fraction = pieces.next();
    if pieces.next().is_some()
        || integer.is_empty()
        || !integer.bytes().all(|byte| byte.is_ascii_digit())
        || (integer.len() > 1 && integer.starts_with('0'))
    {
        return false;
    }
    match fraction {
        None => true,
        Some(fraction) => {
            !fraction.is_empty()
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
                && !fraction.ends_with('0')
        }
    }
}
