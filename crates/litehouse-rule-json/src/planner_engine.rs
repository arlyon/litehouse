use std::collections::BTreeMap;

use cel::Value as CelValue;
use jiff::{Timestamp as JiffTimestamp, civil::DateTime as CivilDateTime, tz::TimeZone};
use litehouse_contracts::quantity::Quantity;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    CompiledNode, CompiledQuantityResult, CompiledRule, CompiledSchedule, CompiledTrigger,
    condition::evaluate_condition,
    model::{
        CapabilityOperation, DesiredPatchOperation, DesiredProjectionPath, DesiredProjectionValue,
        EndOutcome, ReceiptOutcome, ReceiptRequirement, StateFieldRef,
    },
    planner_model::{
        DesiredField, DesiredProjection, EffectBinding, EffectIntent, EvaluationEvent,
        EvaluationIdentity, Frontier, PlannerEvaluationInput, PlannerModelError, PlannerState,
        PredicateOutcome, RunState, RunStatus, RuntimeAssumption, RuntimeColorValue, RuntimeValue,
        SavedFrame, SavedNodeValue, SavedValue, Suspension, TimerIntent, TransitionOutcome,
        TransitionPlan, TriggerInput, ValueState, desired_path_rank, receipt_rank,
        selector_identity,
    },
};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TransitionError {
    #[error(transparent)]
    Model(#[from] PlannerModelError),
    #[error("evaluation identity does not match the compiled rule")]
    IdentityMismatch,
    #[error("initial trigger does not match the compiled trigger")]
    TriggerMismatch,
    #[error("schedule occurrence does not match the compiled schedule")]
    ScheduleMismatch,
    #[error("initial input cannot start while a run is active")]
    RunAlreadyActive,
    #[error("initial trigger does not match the head of the process-local queue")]
    QueuedTriggerMismatch,
    #[error("resume input does not match an active suspension")]
    SuspensionMismatch,
    #[error("missing capability snapshot for node {node:?}")]
    MissingSnapshot { node: String },
    #[error("missing snapshot field for node {node:?}")]
    MissingSnapshotField { node: String },
    #[error("missing or incompatible effect binding for node {node:?}")]
    MissingEffectBinding { node: String },
    #[error("compiled graph invariant failed at {node:?}: {reason}")]
    InvalidCompiledGraph { node: String, reason: String },
    #[error("desired-state test failed at {node:?} path {path:?}")]
    DesiredTestFailed {
        node: String,
        path: DesiredProjectionPath,
    },
    #[error("desired-state operation requires an existing value at {node:?} path {path:?}")]
    DesiredValueMissing {
        node: String,
        path: DesiredProjectionPath,
    },
    #[error("planner phase for {kind} is not implemented yet")]
    PhasePending { kind: &'static str },
    #[error("planner exceeded the compiled node bound")]
    StepLimit,
    #[error("planner has {actual} timers; configured maximum is {maximum}")]
    TimerLimit { actual: usize, maximum: usize },
    #[error("encoded trace is {actual} bytes; maximum is {maximum}")]
    TraceTooLarge { actual: usize, maximum: usize },
    #[error("saved planner state is inconsistent with the compiled graph: {0}")]
    InvalidSavedState(String),
    #[error("timestamp arithmetic overflowed")]
    TimestampOverflow,
}

#[derive(Clone)]
struct WorkFrame {
    branch_id: String,
    current: String,
    trigger: TriggerInput,
    values: BTreeMap<String, SavedNodeValue>,
    desired: BTreeMap<String, DesiredField>,
    assumptions: Vec<RuntimeAssumption>,
    trace: Vec<String>,
}

impl WorkFrame {
    fn initial(current: String, trigger_id: &str, trigger: TriggerInput) -> Self {
        Self {
            branch_id: "root".to_owned(),
            current,
            trigger,
            values: BTreeMap::new(),
            desired: BTreeMap::new(),
            assumptions: Vec::new(),
            trace: vec![trigger_id.to_owned()],
        }
    }

    fn from_saved(saved: SavedFrame) -> Self {
        let desired = saved
            .desired
            .map(|projection| {
                projection
                    .fields
                    .into_iter()
                    .map(|field| (desired_key(&field.selector, field.path()), field))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            branch_id: saved.branch_id,
            current: saved.resume_node_id,
            trigger: saved.trigger,
            values: saved
                .values
                .into_iter()
                .map(|value| (value.node_id, value.value))
                .collect(),
            desired,
            assumptions: saved.assumptions,
            trace: saved.trace_node_ids,
        }
    }

    fn into_saved(self, resume_node_id: String) -> Result<SavedFrame, PlannerModelError> {
        SavedFrame::new(
            self.branch_id,
            resume_node_id,
            self.trigger,
            self.values
                .into_iter()
                .map(|(node_id, value)| SavedValue { node_id, value })
                .collect(),
            desired_projection(self.desired)?,
            self.assumptions,
            self.trace,
        )
    }
}

pub fn plan_microstep(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
) -> Result<TransitionPlan, TransitionError> {
    input.validate(compiled.limits().max_plan_input_bytes as usize)?;
    validate_identity(compiled, input)?;
    validate_compiled_state(compiled, input)?;
    if let EvaluationEvent::Initial { trigger } = &input.event
        && input.state.active_run.is_some()
    {
        validate_trigger(compiled, trigger)?;
        if let Some(plan) = update_active_admission(compiled, input, trigger.clone())? {
            return Ok(plan);
        }
        return queue_active_trigger(compiled, input, trigger.clone());
    }
    let mut next_queue = input.state.queued_triggers.clone();
    if let EvaluationEvent::Initial { trigger } = &input.event
        && let Some(queued) = next_queue.first()
    {
        if queued != trigger {
            return Err(TransitionError::QueuedTriggerMismatch);
        }
        next_queue.remove(0);
    }

    let (frame, retained_suspensions, resume_terminal) = start_frame(compiled, input)?;
    if let Some(outcome) = resume_terminal {
        return finish_or_continue_closed_admission(
            compiled,
            input,
            frame,
            retained_suspensions,
            outcome,
            Vec::new(),
            Vec::new(),
            next_queue,
        );
    }
    evaluate_frame(
        compiled,
        input,
        frame,
        retained_suspensions,
        next_queue,
        Vec::new(),
        Vec::new(),
    )
}

fn evaluate_frame(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
    mut frame: WorkFrame,
    mut retained_suspensions: Vec<Suspension>,
    next_queue: Vec<TriggerInput>,
    mut effects: Vec<EffectIntent>,
    mut timers: Vec<TimerIntent>,
) -> Result<TransitionPlan, TransitionError> {
    let mut terminal = None;

    for _ in 0..crate::model::HARD_MAX_NODES {
        frame.trace.push(frame.current.clone());
        let node = compiled.nodes().get(&frame.current).ok_or_else(|| {
            TransitionError::InvalidCompiledGraph {
                node: frame.current.clone(),
                reason: "control edge targets an unknown node".to_owned(),
            }
        })?;
        match node {
            CompiledNode::Snapshot {
                selector, field, ..
            } => {
                evaluate_snapshot(&mut frame, input, selector, field)?;
                frame.current = required_next(compiled, &frame.current, "next")?;
            }
            CompiledNode::Condition { program } => {
                let outcome = evaluate_cel(compiled, &frame, input, program)?;
                frame
                    .values
                    .insert(frame.current.clone(), SavedNodeValue::Predicate { outcome });
                frame.current = required_next(compiled, &frame.current, "next")?;
            }
            CompiledNode::Quantity {
                input_profile,
                operation,
            } => {
                evaluate_quantity(compiled, &mut frame, input_profile.as_str(), operation)?;
                frame.current = required_next(compiled, &frame.current, "next")?;
            }
            CompiledNode::Branch => {
                let outcome = predicate_input(compiled, &frame, &frame.current, "predicate")?;
                let port = predicate_port(&outcome);
                match optional_next(compiled, &frame.current, port) {
                    Some(next) => frame.current = next.to_owned(),
                    None => {
                        terminal = Some(outcome_transition(&outcome));
                        break;
                    }
                }
            }
            CompiledNode::DesiredPatch {
                snapshot_node_id,
                operations,
            } => {
                apply_desired_patch(compiled, input, &mut frame, snapshot_node_id, operations)?;
                frame.current = required_next(compiled, &frame.current, "next")?;
            }
            CompiledNode::EffectIntent {
                selector,
                operation,
                deadline_ms,
                required_receipt,
                retry,
            } => {
                let continuation =
                    compiled
                        .receipt_continuation(&frame.current)
                        .ok_or_else(|| TransitionError::InvalidCompiledGraph {
                            node: frame.current.clone(),
                            reason: "effect has no typed receipt continuation".to_owned(),
                        })?;
                let binding = binding_for(input, &frame.current, selector, operation)?.clone();
                frame
                    .assumptions
                    .push(RuntimeAssumption::EffectBindingRevision {
                        node_id: frame.current.clone(),
                        provider_generation: binding.provider_generation,
                        facet_revision: binding.facet_revision,
                        grant_revision: binding.grant_revision,
                    });
                let effect = build_effect(
                    compiled,
                    input,
                    &frame,
                    continuation,
                    &binding,
                    operation,
                    *deadline_ms,
                    *required_receipt,
                    retry,
                )?;
                let required_confidence = effect.required_confidence;
                let suspension_id = effect.suspension_id.clone();
                let intent_id = effect.intent_id.clone();
                let saved = frame.clone().into_saved(continuation.to_owned())?;
                retained_suspensions.push(Suspension::Effect {
                    suspension_id,
                    intent_id,
                    effect_node_id: frame.current.clone(),
                    operation: operation.clone(),
                    required_confidence,
                    binding: binding.clone(),
                    idempotency_key: effect.idempotency_key.clone(),
                    issued_microstep: input.identity.microstep,
                    issued_at: effect.issued_at,
                    continuation_node_id: continuation.to_owned(),
                    frame: saved,
                });
                effects.push(effect);
                terminal = Some(TransitionOutcome::Suspended);
                break;
            }
            CompiledNode::Delay { duration_ms } => {
                let continuation = required_next(compiled, &frame.current, "next")?;
                let timer = build_timer(input, &frame, &continuation, *duration_ms)?;
                let saved = frame.clone().into_saved(continuation.clone())?;
                retained_suspensions.push(Suspension::Timer {
                    suspension_id: timer.suspension_id.clone(),
                    timer_id: timer.timer_id.clone(),
                    continuation_node_id: continuation,
                    issued_microstep: input.identity.microstep,
                    issued_at: timer.scheduled_at,
                    scheduled_for: timer.fire_at,
                    frame: saved,
                });
                timers.push(timer);
                terminal = Some(TransitionOutcome::Suspended);
                break;
            }
            CompiledNode::ReceiptBranch { .. } => {
                return Err(TransitionError::InvalidCompiledGraph {
                    node: frame.current,
                    reason: "receipt branch can only be entered by an effect-receipt microstep"
                        .to_owned(),
                });
            }
            CompiledNode::End { outcome } => {
                terminal = Some(end_outcome(*outcome));
                break;
            }
            CompiledNode::DebounceThrottle { key, policy, .. } => {
                let continuation =
                    optional_next(compiled, &frame.current, "admitted").map(str::to_owned);
                let resume_node = continuation
                    .clone()
                    .unwrap_or_else(|| frame.current.clone());
                let timer = build_admission_timer(
                    input,
                    &frame,
                    &resume_node,
                    admission_window_ms(*policy),
                )?;
                let saved = frame.clone().into_saved(resume_node)?;
                let winning_trigger = match policy {
                    crate::AdmissionPolicy::DebounceTrailing { .. } => Some(frame.trigger.clone()),
                    crate::AdmissionPolicy::ThrottleLeading { .. } => None,
                };
                retained_suspensions.push(Suspension::Admission {
                    suspension_id: timer.suspension_id.clone(),
                    timer_id: timer.timer_id.clone(),
                    node_id: frame.current.clone(),
                    key: *key,
                    policy: *policy,
                    issued_microstep: input.identity.microstep,
                    issued_at: timer.scheduled_at,
                    scheduled_for: timer.fire_at,
                    winning_trigger,
                    dropped_count: 0,
                    coalesced_count: 0,
                    window_closed: false,
                    continuation_node_id: continuation.clone(),
                    frame: saved,
                });
                timers.push(timer);
                match policy {
                    crate::AdmissionPolicy::DebounceTrailing { .. } => {
                        terminal = Some(TransitionOutcome::Suspended);
                        break;
                    }
                    crate::AdmissionPolicy::ThrottleLeading { .. } => {
                        if let Some(continuation) = continuation {
                            frame.current = continuation;
                        } else {
                            terminal = Some(TransitionOutcome::Noop);
                            break;
                        }
                    }
                }
            }
            CompiledNode::ForkJoin { .. } => {
                return Err(TransitionError::PhasePending { kind: "fork_join" });
            }
        }
    }

    let outcome = terminal.ok_or(TransitionError::StepLimit)?;
    finish_or_continue_closed_admission(
        compiled,
        input,
        frame,
        retained_suspensions,
        outcome,
        effects,
        timers,
        next_queue,
    )
}

#[allow(clippy::too_many_arguments)]
fn finish_or_continue_closed_admission(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
    frame: WorkFrame,
    mut retained_suspensions: Vec<Suspension>,
    outcome: TransitionOutcome,
    effects: Vec<EffectIntent>,
    timers: Vec<TimerIntent>,
    next_queue: Vec<TriggerInput>,
) -> Result<TransitionPlan, TransitionError> {
    if outcome != TransitionOutcome::Suspended
        && retained_suspensions.len() == 1
        && matches!(
            retained_suspensions.first(),
            Some(Suspension::Admission {
                window_closed: true,
                winning_trigger: Some(_),
                continuation_node_id: Some(_),
                ..
            })
        )
    {
        let Suspension::Admission {
            frame,
            winning_trigger: Some(trigger),
            continuation_node_id: Some(continuation),
            ..
        } = retained_suspensions.remove(0)
        else {
            unreachable!("closed trailing admission was checked above")
        };
        let mut trailing = WorkFrame::from_saved(frame);
        trailing.branch_id = "trailing".to_owned();
        trailing.trigger = trigger;
        trailing.current = continuation;
        return evaluate_frame(
            compiled,
            input,
            trailing,
            retained_suspensions,
            next_queue,
            effects,
            timers,
        );
    }
    finish_plan(
        compiled,
        input,
        frame,
        retained_suspensions,
        outcome,
        effects,
        timers,
        next_queue,
    )
}

fn update_active_admission(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
    trigger: TriggerInput,
) -> Result<Option<TransitionPlan>, TransitionError> {
    let Some(mut run) = input.state.active_run.clone() else {
        return Ok(None);
    };
    let Some(index) = run.frontier.suspended.iter().position(|suspension| {
        matches!(
            suspension,
            Suspension::Admission {
                window_closed: false,
                ..
            }
        )
    }) else {
        return Ok(None);
    };
    if input.identity.run_id != run.run_id || input.identity.microstep != run.next_microstep {
        return Err(TransitionError::IdentityMismatch);
    }
    let mut timers = Vec::new();
    let mut dropped_frame = None;
    let (node_id, outcome) = match &mut run.frontier.suspended[index] {
        Suspension::Admission {
            suspension_id,
            timer_id,
            node_id,
            policy,
            issued_microstep,
            issued_at,
            scheduled_for,
            winning_trigger,
            dropped_count,
            coalesced_count,
            continuation_node_id,
            frame,
            ..
        } => {
            *dropped_count = dropped_count
                .checked_add(1)
                .ok_or(TransitionError::StepLimit)?;
            match policy {
                crate::AdmissionPolicy::DebounceTrailing { window_ms } => {
                    if let Some(dropped_node) = optional_next(compiled, node_id, "dropped") {
                        let mut dropped = WorkFrame::from_saved(frame.clone());
                        dropped.current = dropped_node.to_owned();
                        dropped_frame = Some(dropped);
                    }
                    *coalesced_count = coalesced_count
                        .checked_add(1)
                        .ok_or(TransitionError::StepLimit)?;
                    *winning_trigger = Some(trigger);
                    frame.trigger = winning_trigger
                        .clone()
                        .expect("debounce retained a winning trigger");
                    let seed = transition_seed(
                        &input.identity,
                        input.identity.microstep,
                        node_id,
                        &frame.branch_id,
                    );
                    *timer_id = format!("a_{}", &seed[..32]);
                    *suspension_id = format!("s_{}", &seed[32..64]);
                    *issued_microstep = input.identity.microstep;
                    *issued_at = input.evaluation_time;
                    *scheduled_for = add_milliseconds(input.evaluation_time, *window_ms)?;
                    timers.push(TimerIntent::new(
                        timer_id.clone(),
                        frame.branch_id.clone(),
                        suspension_id.clone(),
                        continuation_node_id
                            .clone()
                            .unwrap_or_else(|| node_id.clone()),
                        *issued_at,
                        *scheduled_for,
                    )?);
                    (node_id.clone(), TransitionOutcome::Suspended)
                }
                crate::AdmissionPolicy::ThrottleLeading { trailing, .. } => {
                    if let Some(dropped_node) = optional_next(compiled, node_id, "dropped") {
                        let mut dropped = WorkFrame::from_saved(frame.clone());
                        dropped.current = dropped_node.to_owned();
                        dropped.trigger = trigger.clone();
                        dropped_frame = Some(dropped);
                    }
                    if *trailing {
                        *coalesced_count = coalesced_count
                            .checked_add(1)
                            .ok_or(TransitionError::StepLimit)?;
                        *winning_trigger = Some(trigger);
                        (node_id.clone(), TransitionOutcome::Suspended)
                    } else {
                        (node_id.clone(), TransitionOutcome::CapacityRejected)
                    }
                }
            }
        }
        _ => unreachable!("admission suspension index was selected"),
    };
    if let Some(dropped) = dropped_frame {
        return evaluate_frame(
            compiled,
            input,
            dropped,
            run.frontier.suspended,
            input.state.queued_triggers.clone(),
            Vec::new(),
            timers,
        )
        .map(Some);
    }
    run.next_microstep = run
        .next_microstep
        .checked_add(1)
        .ok_or(TransitionError::StepLimit)?;
    let next_state = PlannerState::new(
        input
            .state
            .version
            .checked_add(1)
            .ok_or(TransitionError::StepLimit)?,
        input.state.process_epoch.clone(),
        Some(run),
        input.state.queued_triggers.clone(),
    )?;
    let plan = TransitionPlan::new(
        input.identity.clone(),
        outcome,
        Vec::new(),
        timers,
        None,
        next_state,
        vec![RuntimeAssumption::PlannerStateVersion {
            version: input.state.version,
        }],
        vec![compiled.trigger_id().to_owned(), node_id],
    )?;
    Ok(Some(plan))
}

fn queue_active_trigger(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
    trigger: TriggerInput,
) -> Result<TransitionPlan, TransitionError> {
    let mut queued = input.state.queued_triggers.clone();
    let maximum = compiled.limits().max_queued_triggers as usize;
    let outcome = match compiled.policies().trigger_queue {
        crate::TriggerQueuePolicy::DropNewest if queued.len() >= maximum => {
            TransitionOutcome::CapacityRejected
        }
        crate::TriggerQueuePolicy::DropNewest => {
            queued.push(trigger);
            TransitionOutcome::Suspended
        }
        crate::TriggerQueuePolicy::DropOldest => {
            if queued.len() >= maximum {
                queued.remove(0);
            }
            queued.push(trigger);
            TransitionOutcome::Suspended
        }
        crate::TriggerQueuePolicy::Coalesce => {
            queued.clear();
            queued.push(trigger);
            TransitionOutcome::Suspended
        }
    };
    let next_state = PlannerState::new(
        input
            .state
            .version
            .checked_add(1)
            .ok_or(TransitionError::TimestampOverflow)?,
        input.state.process_epoch.clone(),
        input.state.active_run.clone(),
        queued,
    )?;
    TransitionPlan::new(
        input.identity.clone(),
        outcome,
        Vec::new(),
        Vec::new(),
        None,
        next_state,
        vec![RuntimeAssumption::PlannerStateVersion {
            version: input.state.version,
        }],
        vec![compiled.trigger_id().to_owned()],
    )
    .map_err(Into::into)
}

fn validate_identity(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
) -> Result<(), TransitionError> {
    let identity = &input.identity;
    if identity.rule_id != compiled.rule_id()
        || identity.rule_revision != compiled.revision()
        || identity.semantic_sha256 != compiled.semantic_sha256()
        || identity.profile_sha256 != compiled.profile_sha256()
        || &identity.artifacts != compiled.artifacts()
    {
        return Err(TransitionError::IdentityMismatch);
    }
    Ok(())
}

fn validate_compiled_state(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
) -> Result<(), TransitionError> {
    match &input.event {
        EvaluationEvent::Initial { .. } => {
            if input.state.active_run.is_none() && input.identity.microstep != 1 {
                return Err(TransitionError::InvalidSavedState(
                    "a new run must start at microstep 1".to_owned(),
                ));
            }
        }
        EvaluationEvent::Timer { .. } | EvaluationEvent::EffectReceipt { .. } => {
            let run = input
                .state
                .active_run
                .as_ref()
                .ok_or(TransitionError::SuspensionMismatch)?;
            if run.status != RunStatus::Waiting || run.frontier.suspended.is_empty() {
                return Err(TransitionError::InvalidSavedState(
                    "resume events require a waiting suspended run".to_owned(),
                ));
            }
        }
    }
    if let Some(run) = &input.state.active_run {
        let timer_count = run
            .frontier
            .suspended
            .iter()
            .filter(|suspension| {
                matches!(
                    suspension,
                    Suspension::Timer { .. } | Suspension::Admission { .. }
                )
            })
            .count();
        let maximum_timers = compiled.limits().max_timers as usize;
        if timer_count > maximum_timers {
            return Err(TransitionError::TimerLimit {
                actual: timer_count,
                maximum: maximum_timers,
            });
        }
        let mut issued_identity = input.identity.clone();
        issued_identity.run_id = run.run_id.clone();
        for frame in &run.frontier.ready {
            validate_saved_frame_against_compiled(compiled, frame)?;
        }
        for suspension in &run.frontier.suspended {
            match suspension {
                Suspension::Effect {
                    suspension_id,
                    intent_id,
                    effect_node_id,
                    operation,
                    required_confidence,
                    binding,
                    idempotency_key,
                    issued_microstep,
                    continuation_node_id,
                    frame,
                    ..
                } => {
                    let Some(CompiledNode::EffectIntent {
                        operation: compiled_operation,
                        ..
                    }) = compiled.nodes().get(effect_node_id)
                    else {
                        return Err(TransitionError::InvalidSavedState(format!(
                            "effect suspension names invalid node {effect_node_id:?}"
                        )));
                    };
                    if *issued_microstep >= run.next_microstep {
                        return Err(TransitionError::InvalidSavedState(
                            "effect suspension issuance is not before the next microstep"
                                .to_owned(),
                        ));
                    }
                    let seed = transition_seed(
                        &issued_identity,
                        *issued_microstep,
                        effect_node_id,
                        &frame.branch_id,
                    );
                    let Some(CompiledNode::EffectIntent {
                        selector,
                        required_receipt,
                        ..
                    }) = compiled.nodes().get(effect_node_id)
                    else {
                        unreachable!("effect node was checked above")
                    };
                    let expected_confidence = match required_receipt {
                        ReceiptRequirement::AcceptedForDelivery => {
                            crate::ReceiptConfidence::AcceptedForDelivery
                        }
                        ReceiptRequirement::NotificationAdvertisedMaximum => *binding
                            .receipt_support
                            .iter()
                            .max_by_key(|confidence| receipt_rank(**confidence))
                            .ok_or_else(|| {
                                TransitionError::InvalidSavedState(
                                    "saved effect binding has no receipt support".to_owned(),
                                )
                            })?,
                    };
                    if binding.node_id != *effect_node_id
                        || binding.selector != *selector
                        || binding.operation != operation.kind()
                        || *required_confidence != expected_confidence
                        || intent_id != &format!("i_{}", &seed[..32])
                        || suspension_id != &format!("s_{}", &seed[32..64])
                        || idempotency_key
                            != &effect_idempotency_key(
                                &issued_identity,
                                *issued_microstep,
                                effect_node_id,
                                &seed,
                            )
                        || operation != compiled_operation
                        || compiled.receipt_continuation(effect_node_id)
                            != Some(continuation_node_id.as_str())
                    {
                        return Err(TransitionError::InvalidSavedState(
                            "effect suspension does not match its compiled continuation".to_owned(),
                        ));
                    }
                    let Some(CompiledNode::ReceiptBranch {
                        effect_node_id: receipt_effect,
                        ..
                    }) = compiled.nodes().get(continuation_node_id)
                    else {
                        return Err(TransitionError::InvalidSavedState(
                            "effect suspension continuation is not a receipt branch".to_owned(),
                        ));
                    };
                    if receipt_effect != effect_node_id {
                        return Err(TransitionError::InvalidSavedState(
                            "receipt branch belongs to another effect".to_owned(),
                        ));
                    }
                    validate_saved_frame_against_compiled(compiled, frame)?;
                }
                Suspension::Timer {
                    suspension_id,
                    timer_id,
                    continuation_node_id,
                    issued_microstep,
                    issued_at,
                    scheduled_for,
                    frame,
                } => {
                    let delay_node_id = frame.trace_node_ids.last().ok_or_else(|| {
                        TransitionError::InvalidSavedState(
                            "timer frame has no issuing delay node".to_owned(),
                        )
                    })?;
                    if !matches!(
                        compiled.nodes().get(delay_node_id),
                        Some(CompiledNode::Delay { .. })
                    ) {
                        return Err(TransitionError::InvalidSavedState(
                            "timer frame was not issued by a delay node".to_owned(),
                        ));
                    }
                    if *issued_microstep >= run.next_microstep {
                        return Err(TransitionError::InvalidSavedState(
                            "timer suspension issuance is not before the next microstep".to_owned(),
                        ));
                    }
                    let seed = transition_seed(
                        &issued_identity,
                        *issued_microstep,
                        delay_node_id,
                        &frame.branch_id,
                    );
                    let Some(CompiledNode::Delay { duration_ms }) =
                        compiled.nodes().get(delay_node_id)
                    else {
                        unreachable!("delay node was checked above")
                    };
                    let expected_continuation = required_next(compiled, delay_node_id, "next")?;
                    let expected_due = add_milliseconds(*issued_at, *duration_ms)?;
                    if timer_id != &format!("t_{}", &seed[..32])
                        || suspension_id != &format!("s_{}", &seed[32..64])
                        || continuation_node_id != &expected_continuation
                        || scheduled_for != &expected_due
                    {
                        return Err(TransitionError::InvalidSavedState(format!(
                            "timer continuation names invalid node {continuation_node_id:?}"
                        )));
                    }
                    validate_saved_frame_against_compiled(compiled, frame)?;
                }
                Suspension::Admission {
                    suspension_id,
                    timer_id,
                    node_id,
                    key,
                    policy,
                    issued_microstep,
                    issued_at,
                    scheduled_for,
                    continuation_node_id,
                    frame,
                    ..
                } => {
                    let Some(CompiledNode::DebounceThrottle {
                        key: compiled_key,
                        policy: compiled_policy,
                        ..
                    }) = compiled.nodes().get(node_id)
                    else {
                        return Err(TransitionError::InvalidSavedState(
                            "admission suspension names an invalid node".to_owned(),
                        ));
                    };
                    if *issued_microstep >= run.next_microstep {
                        return Err(TransitionError::InvalidSavedState(
                            "admission suspension issuance is not before the next microstep"
                                .to_owned(),
                        ));
                    }
                    let seed = transition_seed(
                        &issued_identity,
                        *issued_microstep,
                        node_id,
                        &frame.branch_id,
                    );
                    let expected_due = add_milliseconds(*issued_at, admission_window_ms(*policy))?;
                    if key != compiled_key
                        || policy != compiled_policy
                        || suspension_id != &format!("s_{}", &seed[32..64])
                        || timer_id != &format!("a_{}", &seed[..32])
                        || scheduled_for != &expected_due
                        || optional_next(compiled, node_id, "admitted")
                            != continuation_node_id.as_deref()
                    {
                        return Err(TransitionError::InvalidSavedState(
                            "admission suspension does not match its compiled node".to_owned(),
                        ));
                    }
                    validate_saved_frame_against_compiled(compiled, frame)?;
                }
            }
        }
    }
    Ok(())
}

fn validate_saved_frame_against_compiled(
    compiled: &CompiledRule,
    frame: &SavedFrame,
) -> Result<(), TransitionError> {
    if !compiled.nodes().contains_key(&frame.resume_node_id) {
        return Err(TransitionError::InvalidSavedState(format!(
            "saved frame resumes at unknown node {:?}",
            frame.resume_node_id
        )));
    }
    for node_id in &frame.trace_node_ids {
        if node_id != compiled.trigger_id() && !compiled.nodes().contains_key(node_id) {
            return Err(TransitionError::InvalidSavedState(format!(
                "saved trace names unknown node {node_id:?}"
            )));
        }
    }
    for saved in &frame.values {
        let Some(node) = compiled.nodes().get(&saved.node_id) else {
            return Err(TransitionError::InvalidSavedState(format!(
                "saved value names unknown node {:?}",
                saved.node_id
            )));
        };
        let compatible = match (node, &saved.value) {
            (CompiledNode::Snapshot { field, .. }, SavedNodeValue::Value { state }) => {
                crate::planner_model::validate_snapshot_value_state(field, state).is_ok()
            }
            (CompiledNode::Condition { .. }, SavedNodeValue::Predicate { .. }) => true,
            (CompiledNode::Quantity { operation, .. }, SavedNodeValue::Predicate { .. }) => {
                matches!(operation, crate::CompiledQuantityOperation::Compare { .. })
            }
            (
                CompiledNode::Quantity {
                    input_profile,
                    operation,
                },
                SavedNodeValue::Value { state },
            ) => {
                let expected = match operation {
                    crate::CompiledQuantityOperation::Convert { .. }
                    | crate::CompiledQuantityOperation::Quantize { .. } => input_profile,
                    crate::CompiledQuantityOperation::Add { result_profile, .. }
                    | crate::CompiledQuantityOperation::Subtract { result_profile, .. }
                    | crate::CompiledQuantityOperation::Multiply { result_profile, .. }
                    | crate::CompiledQuantityOperation::Divide { result_profile, .. } => {
                        result_profile
                    }
                    crate::CompiledQuantityOperation::Compare { .. } => {
                        return Err(TransitionError::InvalidSavedState(format!(
                            "quantity comparison {:?} saved a value",
                            saved.node_id
                        )));
                    }
                };
                match state {
                    ValueState::Known {
                        value: RuntimeValue::Quantity(quantity),
                    } => quantity.profile_id().as_str() == expected.as_str(),
                    ValueState::Known { .. } => false,
                    ValueState::Missing { .. }
                    | ValueState::Unknown { .. }
                    | ValueState::Error { .. } => true,
                }
            }
            _ => false,
        };
        if !compatible {
            return Err(TransitionError::InvalidSavedState(format!(
                "saved value has the wrong type for node {:?}",
                saved.node_id
            )));
        }
    }
    Ok(())
}

fn start_frame(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
) -> Result<(WorkFrame, Vec<Suspension>, Option<TransitionOutcome>), TransitionError> {
    match &input.event {
        EvaluationEvent::Initial { trigger } => {
            if input.state.active_run.is_some() {
                return Err(TransitionError::RunAlreadyActive);
            }
            validate_trigger(compiled, trigger)?;
            let current = required_next(compiled, compiled.trigger_id(), "next")?;
            Ok((
                WorkFrame::initial(current, compiled.trigger_id(), trigger.clone()),
                Vec::new(),
                None,
            ))
        }
        EvaluationEvent::Timer {
            timer_id,
            suspension_id,
            scheduled_for,
            fired_at,
        } => {
            let run = input
                .state
                .active_run
                .as_ref()
                .ok_or(TransitionError::SuspensionMismatch)?;
            let mut retained = run.frontier.suspended.clone();
            let index = retained
                .iter()
                .position(|suspension| match suspension {
                    Suspension::Timer {
                        suspension_id: candidate,
                        timer_id: candidate_timer,
                        scheduled_for: candidate_scheduled,
                        ..
                    }
                    | Suspension::Admission {
                        suspension_id: candidate,
                        timer_id: candidate_timer,
                        scheduled_for: candidate_scheduled,
                        window_closed: false,
                        ..
                    } => {
                        candidate == suspension_id
                            && candidate_timer == timer_id
                            && candidate_scheduled == scheduled_for
                            && fired_at >= candidate_scheduled
                    }
                    Suspension::Effect { .. }
                    | Suspension::Admission {
                        window_closed: true,
                        ..
                    } => false,
                })
                .ok_or(TransitionError::SuspensionMismatch)?;
            match retained.remove(index) {
                Suspension::Timer { frame, .. } => {
                    Ok((WorkFrame::from_saved(frame), retained, None))
                }
                mut admission @ Suspension::Admission { .. } => {
                    let (saved, winning_trigger, continuation) = match &admission {
                        Suspension::Admission {
                            frame,
                            winning_trigger,
                            continuation_node_id,
                            ..
                        } => (
                            frame.clone(),
                            winning_trigger.clone(),
                            continuation_node_id.clone(),
                        ),
                        _ => unreachable!(),
                    };
                    let mut frame = WorkFrame::from_saved(saved);
                    if winning_trigger.is_some() && !retained.is_empty() {
                        let Suspension::Admission { window_closed, .. } = &mut admission else {
                            unreachable!()
                        };
                        *window_closed = true;
                        retained.push(admission);
                        return Ok((frame, retained, Some(TransitionOutcome::Noop)));
                    }
                    if let (Some(trigger), Some(continuation)) = (winning_trigger, continuation) {
                        frame.trigger = trigger;
                        frame.current = continuation;
                        Ok((frame, retained, None))
                    } else {
                        Ok((frame, retained, Some(TransitionOutcome::Noop)))
                    }
                }
                Suspension::Effect { .. } => unreachable!("matched timer suspension"),
            }
        }
        EvaluationEvent::EffectReceipt {
            suspension_id,
            receipt,
        } => {
            let run = input
                .state
                .active_run
                .as_ref()
                .ok_or(TransitionError::SuspensionMismatch)?;
            let mut retained = run.frontier.suspended.clone();
            let index = retained
                .iter()
                .position(|suspension| {
                    matches!(
                        suspension,
                        Suspension::Effect {
                            suspension_id: candidate,
                            intent_id,
                            operation,
                            binding,
                            required_confidence,
                            idempotency_key,
                            issued_at,
                            ..
                        } if candidate == suspension_id
                            && intent_id == &receipt.intent_id
                            && operation == &receipt.operation
                            && binding.provider_generation == receipt.provider_generation
                            && receipt.accepted_at >= *issued_at
                            && receipt.idempotency_key.as_ref().is_none_or(|key| key == idempotency_key)
                            && (receipt_is_failure(receipt.outcome)
                                || receipt_rank(receipt.confidence)
                                    >= receipt_rank(*required_confidence))
                    )
                })
                .ok_or(TransitionError::SuspensionMismatch)?;
            let Suspension::Effect {
                continuation_node_id,
                frame,
                ..
            } = retained.remove(index)
            else {
                unreachable!("matched effect suspension")
            };
            let mut frame = WorkFrame::from_saved(frame);
            let port = receipt_port(receipt.outcome);
            frame.trace.push(continuation_node_id.clone());
            if let Some(next) = optional_next(compiled, &continuation_node_id, port) {
                frame.current = next.to_owned();
                Ok((frame, retained, None))
            } else {
                frame.current = continuation_node_id;
                Ok((
                    frame,
                    retained,
                    Some(receipt_terminal_outcome(receipt.outcome)),
                ))
            }
        }
    }
}

fn validate_trigger(
    compiled: &CompiledRule,
    trigger: &TriggerInput,
) -> Result<(), TransitionError> {
    let schedule_pair = matches!(
        (compiled.trigger(), trigger),
        (
            CompiledTrigger::TimeSchedule { .. },
            TriggerInput::TimeSchedule { .. }
        )
    );
    let matches = match (compiled.trigger(), trigger) {
        (
            CompiledTrigger::StateObservation {
                selector,
                mode,
                initial,
            },
            TriggerInput::StateObservation {
                selector: actual,
                delivery,
                ..
            },
        ) if selector == actual => match delivery {
            crate::planner_model::ObservationDelivery::InitialHydration => {
                matches!(initial, crate::model::InitialObservationPolicy::Run)
            }
            crate::planner_model::ObservationDelivery::AnyObservation
            | crate::planner_model::ObservationDelivery::ValueUnchanged => {
                matches!(mode, crate::model::ObservationMode::AnyObservation)
            }
            crate::planner_model::ObservationDelivery::ValueChanged => true,
        },
        (
            CompiledTrigger::DiscreteEvent {
                selector, event, ..
            },
            TriggerInput::DiscreteEvent {
                selector: actual,
                event: actual_event,
                ..
            },
        ) => selector == actual && event == actual_event,
        (
            CompiledTrigger::TimeSchedule { schedule },
            TriggerInput::TimeSchedule {
                occurrence_id,
                activation_anchor,
                scheduled_for,
            },
        ) => {
            occurrence_id == &schedule_occurrence_id(compiled, *activation_anchor, *scheduled_for)
                && schedule_accepts(schedule, *activation_anchor, *scheduled_for)
        }
        _ => false,
    };
    if matches {
        Ok(())
    } else if schedule_pair {
        Err(TransitionError::ScheduleMismatch)
    } else {
        Err(TransitionError::TriggerMismatch)
    }
}

pub fn schedule_occurrence_id(
    compiled: &CompiledRule,
    activation_anchor: crate::Timestamp,
    scheduled_for: crate::Timestamp,
) -> String {
    let digest = digest_parts(&[
        compiled.rule_id(),
        &compiled.revision().to_string(),
        compiled.semantic_sha256(),
        compiled.profile_sha256(),
        &activation_anchor.unix_seconds().to_string(),
        &activation_anchor.nanosecond().to_string(),
        &scheduled_for.unix_seconds().to_string(),
        &scheduled_for.nanosecond().to_string(),
    ]);
    format!("occ_{}", &digest[..32])
}

fn schedule_accepts(
    schedule: &CompiledSchedule,
    activation_anchor: crate::Timestamp,
    scheduled_for: crate::Timestamp,
) -> bool {
    let Ok(scheduled) = JiffTimestamp::new(
        scheduled_for.unix_seconds(),
        scheduled_for.nanosecond() as i32,
    ) else {
        return false;
    };
    match schedule {
        CompiledSchedule::OneShot { at } => &scheduled == at,
        CompiledSchedule::FixedInterval { interval } => {
            let elapsed = timestamp_nanoseconds(scheduled_for)
                .checked_sub(timestamp_nanoseconds(activation_anchor));
            elapsed.is_some_and(|elapsed| elapsed > 0 && elapsed % interval.as_nanos() == 0)
        }
        CompiledSchedule::WeeklyCivil {
            days,
            time,
            timezone,
            nonexistent_time,
            duplicate_time,
        } => {
            let Ok(zone) = TimeZone::get(timezone) else {
                return false;
            };
            let zoned = scheduled.to_zoned(zone.clone());
            if !days
                .iter()
                .any(|day| authored_weekday(*day) == zoned.weekday())
            {
                return false;
            }
            let authored = CivilDateTime::from_parts(zoned.date(), *time);
            let earlier = zone.to_ambiguous_timestamp(authored).earlier();
            let later = zone.to_ambiguous_timestamp(authored).later();
            let compatible = zone.to_ambiguous_timestamp(authored).compatible();
            let (Ok(earlier), Ok(later), Ok(compatible)) = (earlier, later, compatible) else {
                return false;
            };
            let earlier_local = earlier.to_zoned(zone.clone()).datetime();
            let later_local = later.to_zoned(zone.clone()).datetime();
            let fold = earlier != later && earlier_local == authored && later_local == authored;
            let gap = earlier != later && !fold;
            if gap {
                return match nonexistent_time {
                    crate::NonexistentTimePolicy::Skip => false,
                    crate::NonexistentTimePolicy::ShiftForward => scheduled == compatible,
                };
            }
            if fold {
                return match duplicate_time {
                    crate::DuplicateTimePolicy::First => scheduled == earlier,
                    crate::DuplicateTimePolicy::Second => scheduled == later,
                    crate::DuplicateTimePolicy::Both => scheduled == earlier || scheduled == later,
                };
            }
            scheduled == compatible
        }
    }
}

fn authored_weekday(day: crate::Weekday) -> jiff::civil::Weekday {
    match day {
        crate::Weekday::Monday => jiff::civil::Weekday::Monday,
        crate::Weekday::Tuesday => jiff::civil::Weekday::Tuesday,
        crate::Weekday::Wednesday => jiff::civil::Weekday::Wednesday,
        crate::Weekday::Thursday => jiff::civil::Weekday::Thursday,
        crate::Weekday::Friday => jiff::civil::Weekday::Friday,
        crate::Weekday::Saturday => jiff::civil::Weekday::Saturday,
        crate::Weekday::Sunday => jiff::civil::Weekday::Sunday,
    }
}

fn timestamp_nanoseconds(timestamp: crate::Timestamp) -> i128 {
    i128::from(timestamp.unix_seconds()) * 1_000_000_000 + i128::from(timestamp.nanosecond())
}

fn required_next(
    compiled: &CompiledRule,
    node: &str,
    port: &str,
) -> Result<String, TransitionError> {
    optional_next(compiled, node, port)
        .map(str::to_owned)
        .ok_or_else(|| TransitionError::InvalidCompiledGraph {
            node: node.to_owned(),
            reason: format!("required output port {port:?} is not connected"),
        })
}

fn optional_next<'a>(compiled: &'a CompiledRule, node: &str, port: &str) -> Option<&'a str> {
    compiled
        .outgoing
        .get(&(node.to_owned(), port.to_owned()))
        .map(String::as_str)
}

fn data_source<'a>(
    compiled: &'a CompiledRule,
    node: &str,
    port: &str,
) -> Result<(&'a str, &'a str), TransitionError> {
    compiled
        .data_sources
        .get(&(node.to_owned(), port.to_owned()))
        .map(|(source, source_port)| (source.as_str(), source_port.as_str()))
        .ok_or_else(|| TransitionError::InvalidCompiledGraph {
            node: node.to_owned(),
            reason: format!("required data input {port:?} has no source"),
        })
}

const fn admission_window_ms(policy: crate::AdmissionPolicy) -> u64 {
    match policy {
        crate::AdmissionPolicy::DebounceTrailing { window_ms }
        | crate::AdmissionPolicy::ThrottleLeading { window_ms, .. } => window_ms,
    }
}

fn receipt_is_failure(outcome: ReceiptOutcome) -> bool {
    matches!(
        outcome,
        ReceiptOutcome::Rejected
            | ReceiptOutcome::Error
            | ReceiptOutcome::Cancelled
            | ReceiptOutcome::OutcomeUnknown
    )
}

fn receipt_terminal_outcome(outcome: ReceiptOutcome) -> TransitionOutcome {
    match outcome {
        ReceiptOutcome::AcceptedForDelivery
        | ReceiptOutcome::ProtocolOrTargetAcknowledged
        | ReceiptOutcome::Completed
        | ReceiptOutcome::ObservedCompleted => TransitionOutcome::Success,
        ReceiptOutcome::Rejected | ReceiptOutcome::Error => TransitionOutcome::Error,
        ReceiptOutcome::Cancelled => TransitionOutcome::Cancelled,
        ReceiptOutcome::OutcomeUnknown => TransitionOutcome::Unknown,
    }
}

fn receipt_port(outcome: ReceiptOutcome) -> &'static str {
    match outcome {
        ReceiptOutcome::AcceptedForDelivery => "accepted_for_delivery",
        ReceiptOutcome::ProtocolOrTargetAcknowledged => "protocol_or_target_acknowledged",
        ReceiptOutcome::Completed => "completed",
        ReceiptOutcome::ObservedCompleted => "observed_completed",
        ReceiptOutcome::Rejected => "rejected",
        ReceiptOutcome::Error => "error",
        ReceiptOutcome::Cancelled => "cancelled",
        ReceiptOutcome::OutcomeUnknown => "outcome_unknown",
    }
}

fn evaluate_snapshot(
    frame: &mut WorkFrame,
    input: &PlannerEvaluationInput,
    selector: &crate::model::Selector,
    field: &StateFieldRef,
) -> Result<(), TransitionError> {
    let snapshot = input
        .snapshots
        .iter()
        .find(|snapshot| snapshot.selector == *selector)
        .ok_or_else(|| TransitionError::MissingSnapshot {
            node: frame.current.clone(),
        })?;
    let snapshot_field = snapshot
        .fields
        .iter()
        .find(|candidate| candidate.field == *field)
        .ok_or_else(|| TransitionError::MissingSnapshotField {
            node: frame.current.clone(),
        })?;
    let state = if snapshot.freshness == crate::Freshness::Stale {
        ValueState::Unknown {
            reason: crate::UnknownReason::Stale,
        }
    } else {
        snapshot_field.state.clone()
    };
    frame
        .values
        .insert(frame.current.clone(), SavedNodeValue::Value { state });
    frame.assumptions.push(RuntimeAssumption::SnapshotRevision {
        selector: selector.clone(),
        provider_generation: snapshot.provider_generation,
        facet_revision: snapshot.facet_revision,
        freshness: snapshot.freshness,
    });
    Ok(())
}

fn evaluate_cel(
    compiled: &CompiledRule,
    frame: &WorkFrame,
    input: &PlannerEvaluationInput,
    program: &crate::CheckedCelProgram,
) -> Result<PredicateOutcome, TransitionError> {
    let mut variables = BTreeMap::new();
    for declared in program.inputs() {
        let (source, source_port) = data_source(compiled, &frame.current, &declared.name)?;
        if source_port != "value" && source_port != "result" {
            return Err(TransitionError::InvalidCompiledGraph {
                node: frame.current.clone(),
                reason: "CEL input source has an unsupported output port".to_owned(),
            });
        }
        let saved =
            frame
                .values
                .get(source)
                .ok_or_else(|| TransitionError::InvalidCompiledGraph {
                    node: frame.current.clone(),
                    reason: format!("CEL input source {source:?} was not evaluated"),
                })?;
        let state = match saved {
            SavedNodeValue::Value { state } => state,
            SavedNodeValue::Predicate { .. } => {
                return Err(TransitionError::InvalidCompiledGraph {
                    node: frame.current.clone(),
                    reason: "CEL input cannot consume a predicate outcome".to_owned(),
                });
            }
        };
        match state {
            ValueState::Known { value } => {
                let value =
                    runtime_to_cel(value).ok_or_else(|| TransitionError::InvalidCompiledGraph {
                        node: frame.current.clone(),
                        reason: "runtime value is not admitted by the CEL profile".to_owned(),
                    })?;
                variables.insert(declared.name.clone(), value);
            }
            ValueState::Missing { .. } => {
                return Ok(PredicateOutcome::Unknown {
                    reason: crate::UnknownReason::Unavailable,
                });
            }
            ValueState::Unknown { reason } => {
                return Ok(PredicateOutcome::Unknown { reason: *reason });
            }
            ValueState::Error { code } => {
                return Ok(PredicateOutcome::Error { code: *code });
            }
        }
    }
    match evaluate_condition(
        program,
        variables,
        input.evaluation_time.unix_seconds(),
        input.evaluation_time.nanosecond(),
    ) {
        Ok(true) => Ok(PredicateOutcome::True),
        Ok(false) => Ok(PredicateOutcome::False),
        Err(_) => Ok(PredicateOutcome::Error {
            code: crate::EvaluationErrorCode::InvalidValue,
        }),
    }
}

fn runtime_to_cel(value: &RuntimeValue) -> Option<CelValue> {
    match value {
        RuntimeValue::Bool(value) => Some(CelValue::Bool(*value)),
        RuntimeValue::String(value) => Some(CelValue::String(value.clone().into())),
        RuntimeValue::I64(value) => Some(CelValue::Int(*value)),
        RuntimeValue::U64(value) => Some(CelValue::UInt(*value)),
        RuntimeValue::Timestamp(value) => chrono::DateTime::<chrono::Utc>::from_timestamp(
            value.unix_seconds(),
            value.nanosecond(),
        )
        .map(|value| CelValue::Timestamp(value.fixed_offset())),
        RuntimeValue::Duration(value) => Some(CelValue::Duration(chrono::Duration::milliseconds(
            value.milliseconds as i64,
        ))),
        RuntimeValue::ColorModel(value) => enum_string(value).map(CelValue::String),
        RuntimeValue::ThermostatMode(value) => match value {
            crate::planner_model::ThermostatModeValue::Known(value) => {
                enum_string(value).map(CelValue::String)
            }
            crate::planner_model::ThermostatModeValue::ProviderQualified(value) => {
                Some(CelValue::String(value.clone().into()))
            }
        },
        RuntimeValue::ThermostatAction(value) => enum_string(value).map(CelValue::String),
        RuntimeValue::CoveringMovement(value) => enum_string(value).map(CelValue::String),
        RuntimeValue::Quantity(_)
        | RuntimeValue::ColorValue(_)
        | RuntimeValue::ThermostatTargets(_)
        | RuntimeValue::NativeRepresentation(_)
        | RuntimeValue::StringList(_) => None,
    }
}

fn enum_string<T: serde::Serialize>(value: &T) -> Option<std::sync::Arc<String>> {
    serde_json::to_value(value)
        .ok()?
        .as_str()
        .map(|value| std::sync::Arc::new(value.to_owned()))
}

fn evaluate_quantity(
    compiled: &CompiledRule,
    frame: &mut WorkFrame,
    input_profile: &str,
    operation: &crate::CompiledQuantityOperation,
) -> Result<(), TransitionError> {
    let (source, source_port) = data_source(compiled, &frame.current, "value")?;
    if source_port != "value" && source_port != "result" {
        return Err(TransitionError::InvalidCompiledGraph {
            node: frame.current.clone(),
            reason: "quantity input source has an unsupported output port".to_owned(),
        });
    }
    let saved = frame
        .values
        .get(source)
        .ok_or_else(|| TransitionError::InvalidCompiledGraph {
            node: frame.current.clone(),
            reason: format!("quantity input source {source:?} was not evaluated"),
        })?;
    let state = match saved {
        SavedNodeValue::Value { state } => state,
        SavedNodeValue::Predicate { .. } => {
            return Err(TransitionError::InvalidCompiledGraph {
                node: frame.current.clone(),
                reason: "quantity input cannot consume a predicate outcome".to_owned(),
            });
        }
    };
    let result = match state {
        ValueState::Known {
            value: RuntimeValue::Quantity(quantity),
        } if quantity.profile_id().as_str() == input_profile => {
            match operation.evaluate(quantity) {
                Ok(CompiledQuantityResult::Quantity(value)) => SavedNodeValue::Value {
                    state: ValueState::Known {
                        value: RuntimeValue::Quantity(value),
                    },
                },
                Ok(CompiledQuantityResult::Predicate(value)) => SavedNodeValue::Predicate {
                    outcome: if value {
                        PredicateOutcome::True
                    } else {
                        PredicateOutcome::False
                    },
                },
                Err(_) => match operation {
                    crate::CompiledQuantityOperation::Compare { .. } => SavedNodeValue::Predicate {
                        outcome: PredicateOutcome::Error {
                            code: crate::EvaluationErrorCode::QuantityError,
                        },
                    },
                    _ => SavedNodeValue::Value {
                        state: ValueState::Error {
                            code: crate::EvaluationErrorCode::QuantityError,
                        },
                    },
                },
            }
        }
        ValueState::Known { .. } => quantity_non_value(
            operation,
            PredicateOutcome::Error {
                code: crate::EvaluationErrorCode::TypeMismatch,
            },
        ),
        ValueState::Missing { .. } => quantity_non_value(
            operation,
            PredicateOutcome::Unknown {
                reason: crate::UnknownReason::Unavailable,
            },
        ),
        ValueState::Unknown { reason } => {
            quantity_non_value(operation, PredicateOutcome::Unknown { reason: *reason })
        }
        ValueState::Error { code } => {
            quantity_non_value(operation, PredicateOutcome::Error { code: *code })
        }
    };
    frame.values.insert(frame.current.clone(), result);
    Ok(())
}

fn quantity_non_value(
    operation: &crate::CompiledQuantityOperation,
    outcome: PredicateOutcome,
) -> SavedNodeValue {
    if matches!(operation, crate::CompiledQuantityOperation::Compare { .. }) {
        SavedNodeValue::Predicate { outcome }
    } else {
        let state = match outcome {
            PredicateOutcome::Unknown { reason } => ValueState::Unknown { reason },
            PredicateOutcome::Error { code } => ValueState::Error { code },
            PredicateOutcome::True | PredicateOutcome::False => unreachable!(),
        };
        SavedNodeValue::Value { state }
    }
}

fn predicate_input(
    compiled: &CompiledRule,
    frame: &WorkFrame,
    node: &str,
    port: &str,
) -> Result<PredicateOutcome, TransitionError> {
    let (source, source_port) = data_source(compiled, node, port)?;
    if source_port != "result" {
        return Err(TransitionError::InvalidCompiledGraph {
            node: node.to_owned(),
            reason: "predicate source does not use a result output".to_owned(),
        });
    }
    match frame.values.get(source) {
        Some(SavedNodeValue::Predicate { outcome }) => Ok(outcome.clone()),
        _ => Err(TransitionError::InvalidCompiledGraph {
            node: node.to_owned(),
            reason: "predicate source was not evaluated".to_owned(),
        }),
    }
}

const fn predicate_port(outcome: &PredicateOutcome) -> &'static str {
    match outcome {
        PredicateOutcome::True => "true",
        PredicateOutcome::False => "false",
        PredicateOutcome::Unknown { .. } => "unknown",
        PredicateOutcome::Error { .. } => "error",
    }
}

const fn outcome_transition(outcome: &PredicateOutcome) -> TransitionOutcome {
    match outcome {
        PredicateOutcome::True | PredicateOutcome::False => TransitionOutcome::Noop,
        PredicateOutcome::Unknown { .. } => TransitionOutcome::Unknown,
        PredicateOutcome::Error { .. } => TransitionOutcome::Error,
    }
}

fn apply_desired_patch(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
    frame: &mut WorkFrame,
    snapshot_node_id: &str,
    operations: &[DesiredPatchOperation],
) -> Result<(), TransitionError> {
    let CompiledNode::Snapshot { selector, .. } = compiled
        .nodes()
        .get(snapshot_node_id)
        .ok_or_else(|| TransitionError::InvalidCompiledGraph {
            node: frame.current.clone(),
            reason: "desired patch references a missing snapshot".to_owned(),
        })?
    else {
        return Err(TransitionError::InvalidCompiledGraph {
            node: frame.current.clone(),
            reason: "desired patch target is not a snapshot".to_owned(),
        });
    };
    let snapshot = input
        .snapshots
        .iter()
        .find(|snapshot| snapshot.selector == *selector)
        .ok_or_else(|| TransitionError::MissingSnapshot {
            node: snapshot_node_id.to_owned(),
        })?;
    frame.assumptions.push(RuntimeAssumption::SnapshotRevision {
        selector: selector.clone(),
        provider_generation: snapshot.provider_generation,
        facet_revision: snapshot.facet_revision,
        freshness: snapshot.freshness,
    });
    for operation in operations {
        let path = operation.path();
        let key = desired_key(selector, path);
        match operation {
            DesiredPatchOperation::Test { expected } => {
                let matches = match frame.desired.get(&key) {
                    Some(field) => field.value.as_ref() == Some(expected),
                    None => baseline_matches(input, selector, path, expected),
                };
                if !matches {
                    return Err(TransitionError::DesiredTestFailed {
                        node: frame.current.clone(),
                        path,
                    });
                }
                frame.assumptions.push(RuntimeAssumption::ExpectedValue {
                    selector: selector.clone(),
                    field: state_field_for_path(path),
                    state: baseline_state(input, selector, path).unwrap_or(ValueState::Missing {
                        reason: crate::planner_model::MissingReason::NotReported,
                    }),
                });
            }
            DesiredPatchOperation::Add { value } => {
                frame.desired.insert(
                    key,
                    DesiredField::new(selector.clone(), path, Some(value.clone()))?,
                );
            }
            DesiredPatchOperation::Replace { value } => {
                if !desired_exists(input, frame, selector, path) {
                    return Err(TransitionError::DesiredValueMissing {
                        node: frame.current.clone(),
                        path,
                    });
                }
                frame.desired.insert(
                    key,
                    DesiredField::new(selector.clone(), path, Some(value.clone()))?,
                );
            }
            DesiredPatchOperation::Remove { .. } => {
                if !desired_exists(input, frame, selector, path) {
                    return Err(TransitionError::DesiredValueMissing {
                        node: frame.current.clone(),
                        path,
                    });
                }
                frame
                    .desired
                    .insert(key, DesiredField::new(selector.clone(), path, None)?);
            }
        }
    }
    Ok(())
}

fn desired_exists(
    input: &PlannerEvaluationInput,
    frame: &WorkFrame,
    selector: &crate::model::Selector,
    path: DesiredProjectionPath,
) -> bool {
    match frame.desired.get(&desired_key(selector, path)) {
        Some(field) => field.value.is_some(),
        None => baseline_state(input, selector, path)
            .is_some_and(|state| matches!(state, ValueState::Known { .. })),
    }
}

fn baseline_matches(
    input: &PlannerEvaluationInput,
    selector: &crate::model::Selector,
    path: DesiredProjectionPath,
    expected: &DesiredProjectionValue,
) -> bool {
    let Some(ValueState::Known { value }) = baseline_state(input, selector, path) else {
        return false;
    };
    desired_value_matches_runtime(expected, &value)
}

fn baseline_state(
    input: &PlannerEvaluationInput,
    selector: &crate::model::Selector,
    path: DesiredProjectionPath,
) -> Option<ValueState> {
    let snapshot = input
        .snapshots
        .iter()
        .find(|snapshot| snapshot.selector == *selector)?;
    if snapshot.freshness == crate::Freshness::Stale {
        return Some(ValueState::Unknown {
            reason: crate::UnknownReason::Stale,
        });
    }
    let field = snapshot
        .fields
        .iter()
        .find(|field| field.field == state_field_for_path(path))?;
    match (path, &field.state) {
        (
            DesiredProjectionPath::ThermostatSingleTarget,
            ValueState::Known {
                value:
                    RuntimeValue::ThermostatTargets(crate::planner_model::ThermostatTargets::Single {
                        temperature,
                    }),
            },
        ) => Some(ValueState::Known {
            value: RuntimeValue::Quantity(temperature.clone()),
        }),
        (
            DesiredProjectionPath::ThermostatHeatTarget,
            ValueState::Known {
                value:
                    RuntimeValue::ThermostatTargets(crate::planner_model::ThermostatTargets::HeatCool {
                        heat,
                        ..
                    }),
            },
        ) => Some(ValueState::Known {
            value: RuntimeValue::Quantity(heat.clone()),
        }),
        (
            DesiredProjectionPath::ThermostatCoolTarget,
            ValueState::Known {
                value:
                    RuntimeValue::ThermostatTargets(crate::planner_model::ThermostatTargets::HeatCool {
                        cool,
                        ..
                    }),
            },
        ) => Some(ValueState::Known {
            value: RuntimeValue::Quantity(cool.clone()),
        }),
        (_, state) => Some(state.clone()),
    }
}

fn state_field_for_path(path: DesiredProjectionPath) -> StateFieldRef {
    match path {
        DesiredProjectionPath::On => StateFieldRef::OnOffCurrent,
        DesiredProjectionPath::Level => StateFieldRef::LevelCurrentLevel,
        DesiredProjectionPath::Color => StateFieldRef::ColorCurrentColor,
        DesiredProjectionPath::ThermostatMode => StateFieldRef::ThermostatCurrentMode,
        DesiredProjectionPath::ThermostatSingleTarget
        | DesiredProjectionPath::ThermostatHeatTarget
        | DesiredProjectionPath::ThermostatCoolTarget => StateFieldRef::ThermostatTargets,
        DesiredProjectionPath::ThermostatFan => StateFieldRef::ThermostatFan,
        DesiredProjectionPath::ThermostatSwing => StateFieldRef::ThermostatSwing,
        DesiredProjectionPath::ThermostatPreset => StateFieldRef::ThermostatPreset,
        DesiredProjectionPath::CoveringLift => StateFieldRef::CoveringCurrentPosition {
            axis: crate::model::CoveringAxis::Lift,
        },
        DesiredProjectionPath::CoveringTilt => StateFieldRef::CoveringCurrentPosition {
            axis: crate::model::CoveringAxis::Tilt,
        },
    }
}

fn desired_value_matches_runtime(expected: &DesiredProjectionValue, actual: &RuntimeValue) -> bool {
    match (expected, actual) {
        (DesiredProjectionValue::On(expected), RuntimeValue::Bool(actual)) => expected == actual,
        (DesiredProjectionValue::Level(expected), RuntimeValue::Quantity(actual))
        | (DesiredProjectionValue::CoveringLift(expected), RuntimeValue::Quantity(actual))
        | (DesiredProjectionValue::CoveringTilt(expected), RuntimeValue::Quantity(actual)) => {
            quantity_matches(actual, "litehouse:quantity/ratio@1", &expected.0, "1")
        }
        (DesiredProjectionValue::Color(expected), RuntimeValue::ColorValue(actual)) => {
            authored_color_matches_runtime(expected, actual)
        }
        (
            DesiredProjectionValue::ThermostatMode(expected),
            RuntimeValue::ThermostatMode(crate::planner_model::ThermostatModeValue::Known(actual)),
        ) => expected == actual,
        (
            DesiredProjectionValue::ThermostatSingleTarget(expected),
            RuntimeValue::Quantity(actual),
        )
        | (
            DesiredProjectionValue::ThermostatHeatTarget(expected),
            RuntimeValue::Quantity(actual),
        )
        | (
            DesiredProjectionValue::ThermostatCoolTarget(expected),
            RuntimeValue::Quantity(actual),
        ) => quantity_matches(
            actual,
            "litehouse:quantity/temperature-absolute@1",
            &expected.0,
            "Cel",
        ),
        (DesiredProjectionValue::ThermostatFan(expected), RuntimeValue::String(actual))
        | (DesiredProjectionValue::ThermostatSwing(expected), RuntimeValue::String(actual))
        | (DesiredProjectionValue::ThermostatPreset(expected), RuntimeValue::String(actual)) => {
            expected.0 == *actual
        }
        _ => false,
    }
}

fn quantity_matches(actual: &Quantity, profile: &str, magnitude: &str, unit: &str) -> bool {
    Quantity::parse(profile, magnitude, unit)
        .ok()
        .and_then(|expected| actual.checked_cmp(&expected).ok())
        .is_some_and(|ordering| ordering.is_eq())
}

fn authored_color_matches_runtime(
    expected: &crate::model::ColorValue,
    actual: &RuntimeColorValue,
) -> bool {
    use crate::model::ColorValue as A;
    use RuntimeColorValue as R;
    match (expected, actual) {
        (
            A::HueSaturation {
                hue_degrees: eh,
                saturation: es,
            },
            R::HueSaturation {
                hue_degrees: ah,
                saturation_ratio: as_,
            },
        ) => eh == ah && es == as_,
        (A::CieXy { x: ex, y: ey }, R::CieXy { x: ax, y: ay }) => ex == ax && ey == ay,
        (A::ColorTemperature { kelvin: e }, R::ColorTemperature { kelvin: a }) => e == a,
        (
            A::CalibratedRgb {
                red: er,
                green: eg,
                blue: eb,
                color_space,
            },
            R::CalibratedRgb {
                red: ar,
                green: ag,
                blue: ab,
                color_space: actual_space,
            },
        ) => {
            er == ar
                && eg == ag
                && eb == ab
                && serde_json::to_value(color_space)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .as_deref()
                    == Some(actual_space.as_str())
        }
        (
            A::UncalibratedRgb {
                red: er,
                green: eg,
                blue: eb,
            },
            R::UncalibratedRgb {
                red: ar,
                green: ag,
                blue: ab,
            },
        ) => er == ar && eg == ag && eb == ab,
        (
            A::NativeWhiteChannels {
                white: ew,
                cold_white: ec,
                warm_white: ewa,
            },
            R::NativeWhiteChannels {
                white: aw,
                cold_white: ac,
                warm_white: awa,
            },
        ) => Some(ew) == aw.as_ref() && ec == ac && ewa == awa,
        _ => false,
    }
}

fn desired_key(selector: &crate::model::Selector, path: DesiredProjectionPath) -> String {
    format!(
        "{}:{}",
        selector_identity(selector),
        desired_path_rank(path)
    )
}

fn desired_projection(
    fields: BTreeMap<String, DesiredField>,
) -> Result<Option<DesiredProjection>, PlannerModelError> {
    if fields.is_empty() {
        Ok(None)
    } else {
        DesiredProjection::new(fields.into_values().collect()).map(Some)
    }
}

fn binding_for<'a>(
    input: &'a PlannerEvaluationInput,
    node_id: &str,
    selector: &crate::model::Selector,
    operation: &CapabilityOperation,
) -> Result<&'a EffectBinding, TransitionError> {
    input
        .effect_bindings
        .iter()
        .find(|binding| {
            binding.node_id == node_id
                && binding.selector == *selector
                && binding.operation == operation.kind()
        })
        .ok_or_else(|| TransitionError::MissingEffectBinding {
            node: node_id.to_owned(),
        })
}

#[allow(clippy::too_many_arguments)]
fn build_effect(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
    frame: &WorkFrame,
    continuation: &str,
    binding: &EffectBinding,
    operation: &CapabilityOperation,
    deadline_ms: u32,
    required_receipt: ReceiptRequirement,
    retry: &crate::model::RetryPolicy,
) -> Result<EffectIntent, TransitionError> {
    if deadline_ms > binding.maximum_deadline_ms {
        return Err(TransitionError::MissingEffectBinding {
            node: frame.current.clone(),
        });
    }
    let seed = transition_seed(
        &input.identity,
        input.identity.microstep,
        &frame.current,
        &frame.branch_id,
    );
    let intent_id = format!("i_{}", &seed[..32]);
    let suspension_id = format!("s_{}", &seed[32..64]);
    let idempotency_key = effect_idempotency_key(
        &input.identity,
        input.identity.microstep,
        &frame.current,
        &seed,
    );
    let deadline = add_milliseconds(input.evaluation_time, u64::from(deadline_ms))?;
    let effect = EffectIntent::new(
        intent_id,
        frame.branch_id.clone(),
        suspension_id,
        continuation.to_owned(),
        binding.clone(),
        operation.clone(),
        required_receipt,
        retry.clone(),
        input.evaluation_time,
        deadline,
        idempotency_key,
    )?;
    if effect.binding.operation != operation.kind()
        || compiled.nodes().get(&frame.current).is_none()
    {
        return Err(TransitionError::MissingEffectBinding {
            node: frame.current.clone(),
        });
    }
    Ok(effect)
}

fn build_timer(
    input: &PlannerEvaluationInput,
    frame: &WorkFrame,
    continuation: &str,
    duration_ms: u64,
) -> Result<TimerIntent, TransitionError> {
    let seed = transition_seed(
        &input.identity,
        input.identity.microstep,
        &frame.current,
        &frame.branch_id,
    );
    let timer_id = format!("t_{}", &seed[..32]);
    let suspension_id = format!("s_{}", &seed[32..64]);
    TimerIntent::new(
        timer_id,
        frame.branch_id.clone(),
        suspension_id,
        continuation.to_owned(),
        input.evaluation_time,
        add_milliseconds(input.evaluation_time, duration_ms)?,
    )
    .map_err(Into::into)
}

fn effect_idempotency_key(
    identity: &EvaluationIdentity,
    microstep: u32,
    node_id: &str,
    seed: &str,
) -> String {
    digest_parts(&[
        &identity.process_epoch,
        &identity.rule_id,
        &identity.rule_revision.to_string(),
        &identity.semantic_sha256,
        &identity.profile_sha256,
        &identity.run_id,
        &microstep.to_string(),
        node_id,
        seed,
    ])
}

fn build_admission_timer(
    input: &PlannerEvaluationInput,
    frame: &WorkFrame,
    continuation: &str,
    duration_ms: u64,
) -> Result<TimerIntent, TransitionError> {
    let seed = transition_seed(
        &input.identity,
        input.identity.microstep,
        &frame.current,
        &frame.branch_id,
    );
    TimerIntent::new(
        format!("a_{}", &seed[..32]),
        frame.branch_id.clone(),
        format!("s_{}", &seed[32..64]),
        continuation.to_owned(),
        input.evaluation_time,
        add_milliseconds(input.evaluation_time, duration_ms)?,
    )
    .map_err(Into::into)
}

fn transition_seed(
    identity: &EvaluationIdentity,
    microstep: u32,
    node_id: &str,
    branch_id: &str,
) -> String {
    digest_parts(&[
        &identity.process_epoch,
        &identity.rule_id,
        &identity.rule_revision.to_string(),
        &identity.semantic_sha256,
        &identity.profile_sha256,
        &identity.run_id,
        &microstep.to_string(),
        node_id,
        branch_id,
    ])
}

fn digest_parts(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn add_milliseconds(
    timestamp: crate::Timestamp,
    milliseconds: u64,
) -> Result<crate::Timestamp, TransitionError> {
    let added_seconds = milliseconds / 1_000;
    let added_nanos = (milliseconds % 1_000) * 1_000_000;
    let nanos = u64::from(timestamp.nanosecond()) + added_nanos;
    let carry = nanos / 1_000_000_000;
    let seconds = timestamp
        .unix_seconds()
        .checked_add(i64::try_from(added_seconds).map_err(|_| TransitionError::TimestampOverflow)?)
        .and_then(|seconds| seconds.checked_add(carry as i64))
        .ok_or(TransitionError::TimestampOverflow)?;
    crate::Timestamp::new(seconds, (nanos % 1_000_000_000) as u32)
        .map_err(|_| TransitionError::TimestampOverflow)
}

#[allow(clippy::too_many_arguments)]
fn finish_plan(
    compiled: &CompiledRule,
    input: &PlannerEvaluationInput,
    frame: WorkFrame,
    suspensions: Vec<Suspension>,
    outcome: TransitionOutcome,
    effects: Vec<EffectIntent>,
    timers: Vec<TimerIntent>,
    queued_triggers: Vec<TriggerInput>,
) -> Result<TransitionPlan, TransitionError> {
    let maximum_timers = compiled.limits().max_timers as usize;
    let retained_timer_count = suspensions
        .iter()
        .filter(|suspension| {
            matches!(
                suspension,
                Suspension::Timer { .. } | Suspension::Admission { .. }
            )
        })
        .count();
    if retained_timer_count > maximum_timers || timers.len() > maximum_timers {
        return Err(TransitionError::TimerLimit {
            actual: retained_timer_count.max(timers.len()),
            maximum: maximum_timers,
        });
    }
    let trace_bytes = serde_json::to_vec(&frame.trace)
        .map_err(|error| TransitionError::InvalidCompiledGraph {
            node: frame.current.clone(),
            reason: format!("trace serialization failed: {error}"),
        })?
        .len();
    let maximum_trace_bytes = compiled.limits().max_trace_bytes as usize;
    if trace_bytes > maximum_trace_bytes {
        return Err(TransitionError::TraceTooLarge {
            actual: trace_bytes,
            maximum: maximum_trace_bytes,
        });
    }
    let desired = desired_projection(frame.desired.clone())?;
    let mut assumptions = frame.assumptions.clone();
    assumptions.push(RuntimeAssumption::PlannerStateVersion {
        version: input.state.version,
    });
    let next_version = input
        .state
        .version
        .checked_add(1)
        .ok_or(TransitionError::TimestampOverflow)?;
    let active_run = if suspensions.is_empty() {
        None
    } else {
        Some(RunState::new(
            input.identity.run_id.clone(),
            input
                .identity
                .microstep
                .checked_add(1)
                .ok_or(TransitionError::TimestampOverflow)?,
            RunStatus::Waiting,
            Frontier::new(Vec::new(), suspensions, Vec::new())?,
        )?)
    };
    let next_state = PlannerState::new(
        next_version,
        input.identity.process_epoch.clone(),
        active_run,
        queued_triggers,
    )?;
    let outcome = if next_state.active_run.is_some() {
        TransitionOutcome::Suspended
    } else {
        outcome
    };
    TransitionPlan::new(
        input.identity.clone(),
        outcome,
        effects,
        timers,
        desired,
        next_state,
        assumptions,
        frame.trace,
    )
    .map_err(Into::into)
}

const fn end_outcome(outcome: EndOutcome) -> TransitionOutcome {
    match outcome {
        EndOutcome::Success => TransitionOutcome::Success,
        EndOutcome::Noop => TransitionOutcome::Noop,
        EndOutcome::Unknown => TransitionOutcome::Unknown,
        EndOutcome::Error => TransitionOutcome::Error,
        EndOutcome::Cancelled => TransitionOutcome::Cancelled,
        EndOutcome::CapacityRejected => TransitionOutcome::CapacityRejected,
    }
}
