//! Authoritative, closed Litehouse Rule JSON V1 subset for server-side planning.
//!
//! The crate owns strict JSON shape parsing, semantic graph compilation, and a
//! pure immutable-input-to-transition-plan evaluator. It does not persist rules,
//! dispatch providers, mutate observed state, execute timers, or depend on UI
//! graph libraries.

mod compile;
mod model;
mod planner;
pub mod schema;

pub use compile::{CompileError, CompiledRule, RuleJsonError, compile, compile_rule_json};
pub use model::{
    COMPILER_PROFILE, CompareExpression, CompareOperator, ContractRef, Edge, EdgeEndpoint,
    EndOutcome, HARD_MAX_COLLECTION_ITEMS, HARD_MAX_EDGES, HARD_MAX_GRAPH_DEPTH,
    HARD_MAX_JSON_BYTES, HARD_MAX_JSON_DEPTH, HARD_MAX_NODES, HARD_MAX_STRING_BYTES,
    InitialObservationPolicy, Node, ObservationMode, QuantityWire, RuleDocument, RuleLimits,
    SCHEMA_URL, SCHEMA_VERSION, ScalarLiteral, SchemaLock, Selector, SemanticLocks, SemanticRule,
    TriggerNode, ValueType,
};
pub use planner::{
    Assumption, AssumptionError, AssumptionKind, EffectIntent, EvaluationErrorCode,
    EvaluationInput, Freshness, InputError, ObservationValue, ObservedRevision, OnOffOperation,
    OnOffSetArguments, PlanError, PlanOutcome, ScalarValue, SnapshotInput, StateObservationInput,
    TargetInput, Timestamp, TransitionPlan, UnknownReason, plan,
};
