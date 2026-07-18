//! Authoritative, closed Litehouse Rule JSON V1 model and compatibility planner.
//!
//! The crate owns strict JSON shape parsing, generated Draft 2020-12 artifacts,
//! semantic graph compilation, and the existing pure immutable-input planner.
//! Families not yet executable return a precise planner-subset diagnostic. A
//! supported effect produces an intent and `AwaitingReceipt`; it never claims
//! success before the exact typed receipt microstep. This crate does not persist
//! rules, dispatch providers, mutate observed state, or execute timers.

mod artifacts;
mod catalog;
mod compile;
mod fixtures;
mod model;
mod planner;
pub mod schema;

pub use compile::{CompileError, CompiledRule, RuleJsonError, compile_rule_json};
pub use model::*;
pub use planner::{
    Assumption, AssumptionError, AssumptionKind, EffectIntent, EvaluationErrorCode,
    EvaluationInput, Freshness, InputError, ObservationValue, ObservedRevision, OnOffOperation,
    OnOffSetArguments, PlanError, PlanOutcome, ScalarValue, SnapshotInput, StateObservationInput,
    TargetInput, Timestamp, TransitionPlan, UnknownReason, plan,
};
