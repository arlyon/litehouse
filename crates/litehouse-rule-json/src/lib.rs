//! Authoritative, closed Litehouse Rule JSON V1 model and pure microstep planner.
//!
//! The crate owns strict JSON shape parsing, generated Draft 2020-12 artifacts,
//! semantic graph compilation, and immutable `initial`, `timer`, and
//! `effect_receipt` transitions. Plans contain typed desired projections, effect
//! intents, timers, assumptions, and process-local next state; they never persist
//! rules, dispatch providers, mutate observed state, or execute timers.

mod artifacts;
mod canonical;
mod catalog;
mod compile;
mod condition;
mod fixtures;
mod model;
mod planner_engine;
mod planner_model;
pub mod schema;

pub use compile::{
    CompileError, CompiledArtifacts, CompiledNode, CompiledQuantityOperation,
    CompiledQuantityResult, CompiledRule, CompiledSchedule, CompiledTrigger, RuleJsonError,
    compile_rule_json,
};
pub use condition::CheckedCelProgram;
pub use model::*;
pub use planner_engine::{TransitionError, plan_microstep, schedule_occurrence_id};
pub use planner_model::*;
