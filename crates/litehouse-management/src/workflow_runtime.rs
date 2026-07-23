use std::{collections::VecDeque, sync::Mutex};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const TERMINAL_PER_WORKFLOW: usize = 16;
const TERMINAL_SERVER_WIDE: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowRunStatus {
    Running,
    Succeeded,
    Failed,
    Skipped,
    TimedOut,
    Cancelled,
}

impl WorkflowRunStatus {
    fn is_terminal(self) -> bool {
        self != Self::Running
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowRunOutcome {
    pub rule_id: String,
    pub run_id: String,
    pub status: WorkflowRunStatus,
    pub started_at_unix_milliseconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_unix_milliseconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowOutcomeSnapshot {
    pub active: Vec<WorkflowRunOutcome>,
    pub terminal: Vec<WorkflowRunOutcome>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WorkflowRuntimeError {
    #[error("workflow already has an active run")]
    AlreadyActive,
    #[error("workflow run is not active")]
    NotActive,
    #[error("workflow terminal status is required")]
    NonTerminalStatus,
    #[error("workflow runtime state is unavailable")]
    StateUnavailable,
}

#[derive(Default)]
struct RuntimeState {
    active: Vec<WorkflowRunOutcome>,
    terminal: VecDeque<WorkflowRunOutcome>,
}

#[derive(Default)]
pub struct WorkflowRuntime {
    state: Mutex<RuntimeState>,
}

impl WorkflowRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn begin(
        &self,
        rule_id: impl Into<String>,
        run_id: impl Into<String>,
        started_at_unix_milliseconds: u64,
    ) -> Result<(), WorkflowRuntimeError> {
        let rule_id = rule_id.into();
        let mut state = self
            .state
            .lock()
            .map_err(|_| WorkflowRuntimeError::StateUnavailable)?;
        if state.active.iter().any(|run| run.rule_id == rule_id) {
            return Err(WorkflowRuntimeError::AlreadyActive);
        }
        state.active.push(WorkflowRunOutcome {
            rule_id,
            run_id: run_id.into(),
            status: WorkflowRunStatus::Running,
            started_at_unix_milliseconds,
            finished_at_unix_milliseconds: None,
        });
        state.active.sort_by(|left, right| {
            right
                .started_at_unix_milliseconds
                .cmp(&left.started_at_unix_milliseconds)
                .then_with(|| left.rule_id.cmp(&right.rule_id))
        });
        Ok(())
    }

    pub fn finish(
        &self,
        rule_id: &str,
        run_id: &str,
        status: WorkflowRunStatus,
        finished_at_unix_milliseconds: u64,
    ) -> Result<(), WorkflowRuntimeError> {
        if !status.is_terminal() {
            return Err(WorkflowRuntimeError::NonTerminalStatus);
        }
        let mut state = self
            .state
            .lock()
            .map_err(|_| WorkflowRuntimeError::StateUnavailable)?;
        let index = state
            .active
            .iter()
            .position(|run| run.rule_id == rule_id && run.run_id == run_id)
            .ok_or(WorkflowRuntimeError::NotActive)?;
        let mut outcome = state.active.remove(index);
        outcome.status = status;
        outcome.finished_at_unix_milliseconds = Some(finished_at_unix_milliseconds);
        state.terminal.push_front(outcome);

        let mut seen_for_rule = 0;
        state.terminal.retain(|run| {
            if run.rule_id != rule_id {
                return true;
            }
            seen_for_rule += 1;
            seen_for_rule <= TERMINAL_PER_WORKFLOW
        });
        state.terminal.truncate(TERMINAL_SERVER_WIDE);
        Ok(())
    }

    pub fn outcomes(
        &self,
        rule_id: Option<&str>,
    ) -> Result<WorkflowOutcomeSnapshot, WorkflowRuntimeError> {
        let state = self
            .state
            .lock()
            .map_err(|_| WorkflowRuntimeError::StateUnavailable)?;
        Ok(WorkflowOutcomeSnapshot {
            active: state
                .active
                .iter()
                .filter(|run| rule_id.is_none_or(|rule_id| run.rule_id == rule_id))
                .cloned()
                .collect(),
            terminal: state
                .terminal
                .iter()
                .filter(|run| rule_id.is_none_or(|rule_id| run.rule_id == rule_id))
                .cloned()
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finish(runtime: &WorkflowRuntime, rule: &str, index: u64) {
        let run = format!("{index:032x}");
        runtime.begin(rule, &run, index).unwrap();
        runtime
            .finish(rule, &run, WorkflowRunStatus::Succeeded, index + 1)
            .unwrap();
    }

    #[test]
    fn outcomes_are_newest_first_and_bounded_per_workflow() {
        let runtime = WorkflowRuntime::new();
        for index in 0..20 {
            finish(&runtime, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", index);
        }
        let outcomes = runtime
            .outcomes(Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))
            .unwrap();
        assert_eq!(outcomes.terminal.len(), 16);
        assert_eq!(outcomes.terminal[0].run_id, format!("{:032x}", 19));
        assert_eq!(outcomes.terminal[15].run_id, format!("{:032x}", 4));
    }

    #[test]
    fn outcomes_are_bounded_server_wide() {
        let runtime = WorkflowRuntime::new();
        for index in 0..300 {
            finish(&runtime, &format!("{index:032x}"), index);
        }
        assert_eq!(runtime.outcomes(None).unwrap().terminal.len(), 256);
    }

    #[test]
    fn only_one_active_run_is_allowed_per_workflow() {
        let runtime = WorkflowRuntime::new();
        runtime
            .begin(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                1,
            )
            .unwrap();
        assert_eq!(
            runtime.begin(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "cccccccccccccccccccccccccccccccc",
                2,
            ),
            Err(WorkflowRuntimeError::AlreadyActive)
        );
    }
}
