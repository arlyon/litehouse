use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use litehouse_contracts::quantity::Quantity;
use litehouse_rule_json::{
    CapabilityOperation, CapabilitySnapshot, ColorModel as RuntimeColorModel,
    ColorValue as RuleColor, CompiledNode, CompiledRule, CompiledTrigger,
    CoveringAxis as RuleCoveringAxis, CoveringMovement as RuntimeCoveringMovement,
    DeduplicationGuarantee, EffectBinding, EffectReceipt, EvaluationErrorCode, EvaluationEvent,
    EvaluationIdentity, Freshness, InitialObservationPolicy, MissingReason, ObservationDelivery,
    ObservationMode, OperationKind, PlannerEvaluationInput, PlannerState,
    ReceiptConfidence as PlannerReceiptConfidence, ReceiptOutcome, RuntimeColorValue, RuntimeValue,
    Selector, SelectorCardinality, SnapshotField, StateFieldRef,
    ThermostatAction as RuntimeThermostatAction, ThermostatMode as RuleThermostatMode,
    ThermostatModeValue, ThermostatTargets as RuntimeTargets, Timestamp, TransitionOutcome,
    TriggerInput, UnknownReason, ValueState, next_schedule_occurrence, plan_microstep,
    schedule_occurrence_id,
};
use thiserror::Error;

use crate::{
    Availability, BundledProvider, ColorDirection, ColorModel, ColorValue, CoveringAxis,
    CoveringMovement, FacetQueryRequest, FacetQueryResult, FacetState, InteractionColor,
    InteractionColorSpace, InteractionCoveringAxis, InteractionNotificationPriority,
    InteractionRequest, InteractionTarget, InteractionThermostatMode, LevelDirection,
    ManagementService, OutcomeConfidence, Principal, ReceiptConfidence, ThermostatAction,
    ThermostatMode, ThermostatTargets, WorkflowDirectory, WorkflowRunStatus, WorkflowRuntime,
    canonical_operations,
};

const POLL_INTERVAL: Duration = Duration::from_secs(1);
const LOOP_INTERVAL: Duration = Duration::from_millis(100);
const QUERY_TIMEOUT_MS: u32 = 5_000;
const MAX_EVENTS_PER_WORKFLOW_TICK: usize = 32;
const MAX_PENDING_EVENTS: usize = 256;
const MAX_POLL_BACKOFF: Duration = Duration::from_secs(60);
const GRANT_REVISION: u64 = 1;
static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Error)]
pub enum WorkflowExecutorError {
    #[error("workflow {rule_id} cannot activate: {reason}")]
    Activation { rule_id: String, reason: String },
    #[error("workflow {rule_id} execution failed: {reason}")]
    Execution { rule_id: String, reason: String },
}

#[derive(Clone)]
struct ResolvedFacet {
    selector: Selector,
    facet_id: String,
    generation: u64,
}

struct ScheduledTimer {
    timer_id: String,
    suspension_id: String,
    scheduled_for: Timestamp,
}

struct ActiveWorkflow {
    compiled: CompiledRule,
    facets: BTreeMap<String, ResolvedFacet>,
    snapshot_fields: BTreeMap<String, Vec<StateFieldRef>>,
    effect_bindings: Vec<EffectBinding>,
    state: PlannerState,
    activation_anchor: Timestamp,
    next_schedule: Option<Timestamp>,
    timers: Vec<ScheduledTimer>,
    events: VecDeque<EvaluationEvent>,
    trigger_observation: Option<FacetQueryResult>,
    next_poll: tokio::time::Instant,
    poll_failures: u8,
    failed: bool,
}

pub struct WorkflowExecutor {
    management: ManagementService,
    principal: Principal,
    runtime: Arc<WorkflowRuntime>,
    process_epoch: String,
    workflows: Vec<ActiveWorkflow>,
}

impl WorkflowExecutor {
    pub async fn activate(
        management: ManagementService,
        principal: Principal,
        directory: Arc<WorkflowDirectory>,
        runtime: Arc<WorkflowRuntime>,
    ) -> Result<Self, WorkflowExecutorError> {
        let process_epoch = fresh_hex_id();
        let activation_anchor = now_timestamp();
        let mut workflows = Vec::new();
        for loaded in directory.workflows().values() {
            let compiled = loaded.compiled();
            if !compiled.enabled() {
                continue;
            }
            if matches!(compiled.trigger(), CompiledTrigger::DiscreteEvent { .. }) {
                return Err(activation_error(
                    compiled,
                    "discrete-event workflows require the provider event-drain seam",
                ));
            }
            workflows.push(
                activate_workflow(
                    &management,
                    &principal,
                    compiled,
                    &process_epoch,
                    activation_anchor,
                )
                .await?,
            );
        }
        Ok(Self {
            management,
            principal,
            runtime,
            process_epoch,
            workflows,
        })
    }

    pub async fn run(&mut self) -> Result<(), WorkflowExecutorError> {
        loop {
            let now = now_timestamp();
            let instant = tokio::time::Instant::now();
            for workflow in &mut self.workflows {
                if workflow.failed {
                    continue;
                }
                enqueue_due_events(workflow, now);
                if instant >= workflow.next_poll {
                    match poll_trigger(&self.management, &self.principal, workflow).await {
                        Ok(()) => {
                            workflow.poll_failures = 0;
                            workflow.next_poll = instant + POLL_INTERVAL;
                        }
                        Err(error) => {
                            workflow.poll_failures = workflow.poll_failures.saturating_add(1);
                            let seconds = 1u64
                                .checked_shl(u32::from(workflow.poll_failures.min(6)))
                                .unwrap_or(64);
                            workflow.next_poll =
                                instant + Duration::from_secs(seconds).min(MAX_POLL_BACKOFF);
                            tracing::warn!(
                                rule_id = workflow.compiled.rule_id(),
                                error = %error,
                                "workflow trigger poll failed; retrying with backoff"
                            );
                        }
                    }
                }
                if let Err(error) = drive_workflow(
                    &self.management,
                    &self.principal,
                    &self.runtime,
                    &self.process_epoch,
                    workflow,
                )
                .await
                {
                    workflow.failed = true;
                    tracing::error!(
                        rule_id = workflow.compiled.rule_id(),
                        error = %error,
                        "workflow stopped for this process epoch"
                    );
                }
            }
            tokio::time::sleep(LOOP_INTERVAL).await;
        }
    }
}

async fn activate_workflow(
    management: &ManagementService,
    principal: &Principal,
    compiled: &CompiledRule,
    process_epoch: &str,
    activation_anchor: Timestamp,
) -> Result<ActiveWorkflow, WorkflowExecutorError> {
    let mut selectors = BTreeMap::<String, Selector>::new();
    let mut snapshot_fields = BTreeMap::<String, Vec<StateFieldRef>>::new();
    match compiled.trigger() {
        CompiledTrigger::StateObservation { selector, .. }
        | CompiledTrigger::DiscreteEvent { selector, .. } => {
            selectors.insert(selector_key(selector), selector.clone());
        }
        CompiledTrigger::TimeSchedule { .. } => {}
    }
    for node in compiled.nodes().values() {
        match node {
            CompiledNode::Snapshot {
                selector, field, ..
            } => {
                let key = selector_key(selector);
                selectors.insert(key.clone(), selector.clone());
                snapshot_fields.entry(key).or_default().push(field.clone());
            }
            CompiledNode::EffectIntent { selector, .. } => {
                selectors.insert(selector_key(selector), selector.clone());
            }
            _ => {}
        }
    }
    for fields in snapshot_fields.values_mut() {
        fields.sort_by_key(StateFieldRef::catalog_name);
        fields.dedup();
    }

    let mut facets = BTreeMap::new();
    for (key, selector) in selectors {
        facets.insert(
            key,
            resolve_selector(management, compiled, selector.clone())?,
        );
    }

    let mut effect_bindings = Vec::new();
    for (node_id, node) in compiled.nodes() {
        let CompiledNode::EffectIntent {
            selector,
            operation,
            deadline_ms,
            retry,
            ..
        } = node
        else {
            continue;
        };
        let facet = facets
            .get(&selector_key(selector))
            .expect("every effect selector was resolved");
        let (kind, maximum_deadline) = effect_support(management, selector, operation)?;
        if *deadline_ms > maximum_deadline {
            return Err(activation_error(
                compiled,
                format!(
                    "effect {node_id:?} deadline {deadline_ms}ms exceeds provider maximum {maximum_deadline}ms"
                ),
            ));
        }
        if !matches!(retry, litehouse_rule_json::RetryPolicy::None) {
            return Err(activation_error(
                compiled,
                format!(
                    "effect {node_id:?} requires provider deduplication that is not advertised"
                ),
            ));
        }
        effect_bindings.push(
            EffectBinding::new(
                node_id.clone(),
                selector.clone(),
                operation.kind(),
                facet.generation,
                facet_generation_revision(management, selector)?,
                GRANT_REVISION,
                kind,
                DeduplicationGuarantee::None,
                vec![PlannerReceiptConfidence::AcceptedForDelivery],
                maximum_deadline,
                maximum_deadline,
            )
            .map_err(|error| activation_error(compiled, error.to_string()))?,
        );
    }

    let mut trigger_observation = None;
    for (key, fields) in &snapshot_fields {
        let facet = facets.get(key).expect("snapshot selector was resolved");
        query_snapshot(management, principal, facet, fields, now_timestamp())
            .await
            .map_err(|reason| activation_error(compiled, reason))?;
    }
    let mut events = VecDeque::new();
    if let CompiledTrigger::StateObservation {
        selector, initial, ..
    } = compiled.trigger()
    {
        let facet = facets
            .get(&selector_key(selector))
            .expect("trigger selector was resolved");
        let observation = query_facet(management, principal, facet)
            .await
            .map_err(|reason| activation_error(compiled, reason))?;
        if matches!(initial, InitialObservationPolicy::Run) {
            let observed_at =
                timestamp_from_milliseconds(observation.received_at_unix_milliseconds);
            events.push_back(EvaluationEvent::initial(
                TriggerInput::state_observation(
                    selector.clone(),
                    observation.generation,
                    observation.revision,
                    observed_at,
                    ObservationDelivery::InitialHydration,
                )
                .map_err(|error| activation_error(compiled, error.to_string()))?,
            ));
        }
        trigger_observation = Some(observation);
    }

    let next_schedule = next_schedule_occurrence(compiled, activation_anchor, activation_anchor);
    Ok(ActiveWorkflow {
        compiled: compiled.clone(),
        facets,
        snapshot_fields,
        effect_bindings,
        state: PlannerState::initial(1, process_epoch, Vec::new())
            .map_err(|error| activation_error(compiled, error.to_string()))?,
        activation_anchor,
        next_schedule,
        timers: Vec::new(),
        events,
        trigger_observation,
        next_poll: tokio::time::Instant::now() + POLL_INTERVAL,
        poll_failures: 0,
        failed: false,
    })
}

fn resolve_selector(
    management: &ManagementService,
    compiled: &CompiledRule,
    selector: Selector,
) -> Result<ResolvedFacet, WorkflowExecutorError> {
    if !matches!(selector.cardinality, SelectorCardinality::One) {
        return Err(activation_error(
            compiled,
            "Phase 1 automatic execution requires selector cardinality one",
        ));
    }
    if selector.contract.interface != "capability" {
        return Err(activation_error(
            compiled,
            "selector does not name a capability interface",
        ));
    }
    let mut matches = Vec::new();
    let mut global_facet_count = 0;
    for provider in management.providers.iter() {
        for descriptor in provider.handle.descriptors() {
            let endpoint = descriptor.endpoint();
            let identity = endpoint.identity();
            if identity.facet().as_str() == selector.facet_key {
                global_facet_count += 1;
            }
            if identity.provider().as_str() == selector.provider_instance_id
                && identity.facet().as_str() == selector.facet_key
                && identity.contract().namespace() == selector.contract.namespace
                && identity.contract().name() == selector.contract.package
                && identity.contract().major() == u32::from(selector.contract.major)
                && provider.handle.is_running()
            {
                matches.push(endpoint.generation().get());
            }
        }
    }
    if matches.len() != 1 || global_facet_count != 1 {
        return Err(activation_error(
            compiled,
            format!(
                "selector {}/{} did not resolve to one unambiguous running facet",
                selector.provider_instance_id, selector.facet_key
            ),
        ));
    }
    Ok(ResolvedFacet {
        facet_id: selector.facet_key.clone(),
        selector,
        generation: matches[0],
    })
}

fn facet_generation_revision(
    management: &ManagementService,
    selector: &Selector,
) -> Result<u64, WorkflowExecutorError> {
    management
        .providers
        .iter()
        .flat_map(|provider| provider.handle.descriptors())
        .find(|descriptor| {
            let identity = descriptor.endpoint().identity();
            identity.provider().as_str() == selector.provider_instance_id
                && identity.facet().as_str() == selector.facet_key
                && identity.contract().name() == selector.contract.package
        })
        .map(|descriptor| descriptor.endpoint().incarnation().get())
        .ok_or_else(|| WorkflowExecutorError::Activation {
            rule_id: "unknown".to_owned(),
            reason: "resolved facet disappeared during activation".to_owned(),
        })
}

fn effect_support(
    management: &ManagementService,
    selector: &Selector,
    operation: &CapabilityOperation,
) -> Result<(OperationKind, u32), WorkflowExecutorError> {
    let managed = management
        .providers
        .iter()
        .find(|provider| {
            provider.handle.provider().instance().as_str() == selector.provider_instance_id
        })
        .ok_or_else(|| WorkflowExecutorError::Activation {
            rule_id: "unknown".to_owned(),
            reason: "resolved provider disappeared during activation".to_owned(),
        })?;
    let descriptor = managed
        .handle
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.endpoint().identity().facet().as_str() == selector.facet_key)
        .ok_or_else(|| WorkflowExecutorError::Activation {
            rule_id: "unknown".to_owned(),
            reason: "resolved facet disappeared during activation".to_owned(),
        })?;
    if !canonical_operations(descriptor)
        .iter()
        .any(|candidate| candidate == operation.operation_id())
    {
        return Err(WorkflowExecutorError::Activation {
            rule_id: "unknown".to_owned(),
            reason: format!("facet does not advertise {}", operation.operation_id()),
        });
    }
    let support = match managed.kind {
        BundledProvider::Esphome => (
            if matches!(operation, CapabilityOperation::MomentaryInvoke) {
                OperationKind::Momentary
            } else {
                OperationKind::AbsoluteConvergent
            },
            30_000,
        ),
        BundledProvider::Tasmota => (OperationKind::AbsoluteConvergent, 5_000),
        BundledProvider::Ntfy => (OperationKind::Momentary, 8_000),
        BundledProvider::OpenMeteo => {
            return Err(WorkflowExecutorError::Activation {
                rule_id: "unknown".to_owned(),
                reason: "Open-Meteo does not advertise capability effects".to_owned(),
            });
        }
    };
    Ok(support)
}

fn enqueue_due_events(workflow: &mut ActiveWorkflow, now: Timestamp) {
    let mut retained = Vec::new();
    let mut due_events = Vec::new();
    for timer in workflow.timers.drain(..) {
        if timer.scheduled_for <= now {
            if let Ok(event) = EvaluationEvent::timer(
                timer.timer_id,
                timer.suspension_id,
                timer.scheduled_for,
                now,
            ) {
                due_events.push(event);
            }
        } else {
            retained.push(timer);
        }
    }
    workflow.timers = retained;
    for event in due_events {
        enqueue_internal_event(workflow, event);
    }

    if let Some(due) = workflow.next_schedule.filter(|due| *due <= now) {
        let occurrence =
            schedule_occurrence_id(&workflow.compiled, workflow.activation_anchor, due);
        if let Ok(trigger) =
            TriggerInput::time_schedule(occurrence, workflow.activation_anchor, due)
        {
            enqueue_trigger_event(workflow, EvaluationEvent::initial(trigger));
        }
        workflow.next_schedule =
            next_schedule_occurrence(&workflow.compiled, workflow.activation_anchor, now);
    }
}

fn enqueue_trigger_event(workflow: &mut ActiveWorkflow, event: EvaluationEvent) {
    if workflow.events.len() < MAX_PENDING_EVENTS {
        workflow.events.push_back(event);
    } else {
        tracing::warn!(
            rule_id = workflow.compiled.rule_id(),
            "workflow trigger dropped because its process-local queue is full"
        );
    }
}

fn enqueue_internal_event(workflow: &mut ActiveWorkflow, event: EvaluationEvent) {
    if workflow.events.len() >= MAX_PENDING_EVENTS
        && let Some(index) = workflow
            .events
            .iter()
            .rposition(|queued| matches!(queued, EvaluationEvent::Initial { .. }))
    {
        workflow.events.remove(index);
    }
    if workflow.events.len() < MAX_PENDING_EVENTS {
        workflow.events.push_back(event);
    } else {
        workflow.failed = true;
        tracing::error!(
            rule_id = workflow.compiled.rule_id(),
            "workflow internal event queue exceeded its bound"
        );
    }
}

async fn poll_trigger(
    management: &ManagementService,
    principal: &Principal,
    workflow: &mut ActiveWorkflow,
) -> Result<(), WorkflowExecutorError> {
    let CompiledTrigger::StateObservation { selector, mode, .. } = workflow.compiled.trigger()
    else {
        return Ok(());
    };
    let facet = workflow
        .facets
        .get(&selector_key(selector))
        .expect("trigger selector was resolved");
    let observation = query_facet(management, principal, facet)
        .await
        .map_err(|reason| execution_error(&workflow.compiled, reason))?;
    let previous = workflow
        .trigger_observation
        .replace(observation.clone())
        .expect("state trigger was hydrated at activation");
    if observation.revision == previous.revision {
        return Ok(());
    }
    let delivery = if observation.state != previous.state {
        ObservationDelivery::ValueChanged
    } else if matches!(mode, ObservationMode::AnyObservation) {
        ObservationDelivery::ValueUnchanged
    } else {
        return Ok(());
    };
    let observed_at = timestamp_from_milliseconds(observation.received_at_unix_milliseconds);
    let trigger = TriggerInput::state_observation(
        selector.clone(),
        observation.generation,
        observation.revision,
        observed_at,
        delivery,
    )
    .map_err(|error| execution_error(&workflow.compiled, error.to_string()))?;
    enqueue_trigger_event(workflow, EvaluationEvent::initial(trigger));
    Ok(())
}

async fn drive_workflow(
    management: &ManagementService,
    principal: &Principal,
    runtime: &WorkflowRuntime,
    process_epoch: &str,
    workflow: &mut ActiveWorkflow,
) -> Result<(), WorkflowExecutorError> {
    for _ in 0..MAX_EVENTS_PER_WORKFLOW_TICK {
        let Some(event) = next_workflow_event(workflow) else {
            break;
        };
        let starting = !workflow.state.has_active_run();
        let (run_id, microstep) = workflow
            .state
            .active_cursor()
            .map(|cursor| (cursor.run_id, cursor.next_microstep))
            .unwrap_or_else(|| (fresh_hex_id(), 1));
        if starting {
            runtime
                .begin(
                    workflow.compiled.rule_id(),
                    &run_id,
                    now_unix_milliseconds(),
                )
                .map_err(|error| execution_error(&workflow.compiled, error.to_string()))?;
        }
        let result = evaluate_event(
            management,
            principal,
            process_epoch,
            workflow,
            &run_id,
            microstep,
            event,
        )
        .await;
        let plan = match result {
            Ok(plan) => plan,
            Err(error) => {
                let _ = runtime.finish(
                    workflow.compiled.rule_id(),
                    &run_id,
                    WorkflowRunStatus::Failed,
                    now_unix_milliseconds(),
                );
                return Err(error);
            }
        };
        let was_active = workflow.state.has_active_run() || starting;
        workflow.state = plan.next_state().clone();
        let pending_timers = workflow
            .state
            .pending_timer_ids()
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        discard_stale_timer_events(&mut workflow.events, &pending_timers);
        reconcile_armed_timers(&mut workflow.timers, &pending_timers, plan.timers());
        for effect in plan.effects() {
            let receipt = dispatch_effect(management, principal, effect).await;
            let event = EvaluationEvent::effect_receipt(effect.suspension_id.clone(), receipt)
                .map_err(|error| execution_error(&workflow.compiled, error.to_string()))?;
            enqueue_internal_event(workflow, event);
        }
        if was_active && !workflow.state.has_active_run() {
            runtime
                .finish(
                    workflow.compiled.rule_id(),
                    &run_id,
                    terminal_status(plan.outcome()),
                    now_unix_milliseconds(),
                )
                .map_err(|error| execution_error(&workflow.compiled, error.to_string()))?;
        }
    }
    Ok(())
}

fn discard_stale_timer_events(events: &mut VecDeque<EvaluationEvent>, pending: &BTreeSet<String>) {
    events.retain(|event| match event {
        EvaluationEvent::Timer { timer_id, .. } => pending.contains(timer_id),
        EvaluationEvent::Initial { .. } | EvaluationEvent::EffectReceipt { .. } => true,
    });
}

fn next_workflow_event(workflow: &mut ActiveWorkflow) -> Option<EvaluationEvent> {
    if !workflow.state.has_active_run()
        && let Some(trigger) = workflow.state.queued_triggers().first().cloned()
    {
        return Some(EvaluationEvent::initial(trigger));
    }
    dequeue_raw_event(&mut workflow.events, workflow.state.has_active_run())
}

fn dequeue_raw_event(
    events: &mut VecDeque<EvaluationEvent>,
    active_run: bool,
) -> Option<EvaluationEvent> {
    if active_run
        && let Some(index) = events
            .iter()
            .position(|event| !matches!(event, EvaluationEvent::Initial { .. }))
    {
        return events.remove(index);
    }
    events.pop_front()
}

fn reconcile_armed_timers(
    armed: &mut Vec<ScheduledTimer>,
    pending: &BTreeSet<String>,
    emitted: &[litehouse_rule_json::TimerIntent],
) {
    armed.retain(|timer| pending.contains(&timer.timer_id));
    for timer in emitted {
        armed.retain(|current| current.timer_id != timer.timer_id);
        armed.push(ScheduledTimer {
            timer_id: timer.timer_id.clone(),
            suspension_id: timer.suspension_id.clone(),
            scheduled_for: timer.fire_at,
        });
    }
}

async fn evaluate_event(
    management: &ManagementService,
    principal: &Principal,
    process_epoch: &str,
    workflow: &ActiveWorkflow,
    run_id: &str,
    microstep: u32,
    event: EvaluationEvent,
) -> Result<litehouse_rule_json::TransitionPlan, WorkflowExecutorError> {
    let evaluation_time = now_timestamp();
    let mut snapshots = Vec::new();
    for (key, fields) in &workflow.snapshot_fields {
        let facet = workflow
            .facets
            .get(key)
            .expect("snapshot selector was resolved");
        snapshots.push(
            query_snapshot(management, principal, facet, fields, evaluation_time)
                .await
                .map_err(|reason| execution_error(&workflow.compiled, reason))?,
        );
    }
    let identity = EvaluationIdentity::new(
        process_epoch,
        workflow.compiled.rule_id(),
        workflow.compiled.revision(),
        workflow.compiled.semantic_sha256(),
        workflow.compiled.profile_sha256(),
        workflow.compiled.artifacts().clone(),
        run_id,
        microstep,
    )
    .map_err(|error| execution_error(&workflow.compiled, error.to_string()))?;
    let input = PlannerEvaluationInput::new(
        identity,
        event,
        evaluation_time,
        snapshots,
        workflow.effect_bindings.clone(),
        workflow.state.clone(),
    )
    .map_err(|error| execution_error(&workflow.compiled, error.to_string()))?;
    plan_microstep(&workflow.compiled, &input)
        .map_err(|error| execution_error(&workflow.compiled, error.to_string()))
}

async fn dispatch_effect(
    management: &ManagementService,
    principal: &Principal,
    effect: &litehouse_rule_json::EffectIntent,
) -> EffectReceipt {
    let target = InteractionTarget {
        facet_id: effect.binding.selector.facet_key.clone(),
        facet_generation: effect.binding.provider_generation,
        timeout_ms: Some(deadline_remaining_ms(effect.deadline)),
        idempotency_key: effect.idempotency_key.clone(),
    };
    let operation = effect.operation.clone();
    let accepted_at = now_timestamp();
    match management
        .interact(principal, interaction_request(target, &operation))
        .await
    {
        Ok(result) => {
            let completed_at = now_timestamp();
            let (confidence, outcome) = receipt_projection(result.confidence);
            EffectReceipt::new(
                effect.intent_id.clone(),
                result.receipt_id,
                operation,
                1,
                result.generation,
                confidence,
                outcome,
                accepted_at,
                Some(completed_at),
                Some(effect.idempotency_key.clone()),
            )
            .expect("typed management receipts satisfy the planner model")
        }
        Err(error) => EffectReceipt::new(
            effect.intent_id.clone(),
            error.error_id.clone(),
            operation,
            1,
            effect.binding.provider_generation,
            PlannerReceiptConfidence::AcceptedForDelivery,
            if error.outcome.confidence == OutcomeConfidence::OutcomeUnknown {
                ReceiptOutcome::OutcomeUnknown
            } else {
                ReceiptOutcome::Error
            },
            accepted_at,
            Some(now_timestamp()),
            Some(effect.idempotency_key.clone()),
        )
        .expect("management errors satisfy the planner model"),
    }
}

fn interaction_request(
    target: InteractionTarget,
    operation: &CapabilityOperation,
) -> InteractionRequest {
    match operation {
        CapabilityOperation::OnOffSet { on } => InteractionRequest::OnOffSet { target, on: *on },
        CapabilityOperation::OnOffToggle => InteractionRequest::OnOffToggle { target },
        CapabilityOperation::LevelSet { level } => InteractionRequest::LevelSet {
            target,
            level: level.0.clone(),
        },
        CapabilityOperation::LevelStep { step } => InteractionRequest::LevelStep {
            target,
            step: step.0.clone(),
        },
        CapabilityOperation::LevelMove {
            direction,
            rate_per_second,
        } => InteractionRequest::LevelMove {
            target,
            direction: match direction {
                litehouse_rule_json::LevelDirection::Up => LevelDirection::Up,
                litehouse_rule_json::LevelDirection::Down => LevelDirection::Down,
            },
            rate_per_second: rate_per_second.0.clone(),
        },
        CapabilityOperation::LevelStop => InteractionRequest::LevelStop { target },
        CapabilityOperation::LevelTransition { level, duration_ms } => {
            InteractionRequest::LevelTransition {
                target,
                level: level.0.clone(),
                duration_ms: *duration_ms,
            }
        }
        CapabilityOperation::LevelSetCoupled {
            level,
            on_when_nonzero,
        } => InteractionRequest::LevelSetCoupled {
            target,
            level: level.0.clone(),
            on_when_nonzero: *on_when_nonzero,
        },
        CapabilityOperation::LevelStepCoupled {
            step,
            on_when_nonzero,
        } => InteractionRequest::LevelStepCoupled {
            target,
            step: step.0.clone(),
            on_when_nonzero: *on_when_nonzero,
        },
        CapabilityOperation::LevelMoveCoupled {
            direction,
            rate_per_second,
            on_when_nonzero,
        } => InteractionRequest::LevelMoveCoupled {
            target,
            direction: match direction {
                litehouse_rule_json::LevelDirection::Up => LevelDirection::Up,
                litehouse_rule_json::LevelDirection::Down => LevelDirection::Down,
            },
            rate_per_second: rate_per_second.0.clone(),
            on_when_nonzero: *on_when_nonzero,
        },
        CapabilityOperation::LevelTransitionCoupled {
            level,
            duration_ms,
            on_when_nonzero,
        } => InteractionRequest::LevelTransitionCoupled {
            target,
            level: level.0.clone(),
            duration_ms: *duration_ms,
            on_when_nonzero: *on_when_nonzero,
        },
        CapabilityOperation::ColorSet { color } => InteractionRequest::ColorSet {
            target,
            color: interaction_color(color),
        },
        CapabilityOperation::ColorStep { color, direction } => InteractionRequest::ColorStep {
            target,
            color: interaction_color(color),
            direction: match direction {
                litehouse_rule_json::ColorDirection::Increase => ColorDirection::Increase,
                litehouse_rule_json::ColorDirection::Decrease => ColorDirection::Decrease,
            },
        },
        CapabilityOperation::ColorMove { color, direction } => InteractionRequest::ColorMove {
            target,
            color: interaction_color(color),
            direction: match direction {
                litehouse_rule_json::ColorDirection::Increase => ColorDirection::Increase,
                litehouse_rule_json::ColorDirection::Decrease => ColorDirection::Decrease,
            },
        },
        CapabilityOperation::ColorStop => InteractionRequest::ColorStop { target },
        CapabilityOperation::ColorTransition { color, duration_ms } => {
            InteractionRequest::ColorTransition {
                target,
                color: interaction_color(color),
                duration_ms: *duration_ms,
            }
        }
        CapabilityOperation::ThermostatSetMode { mode } => InteractionRequest::ThermostatSetMode {
            target,
            mode: interaction_mode(*mode),
        },
        CapabilityOperation::ThermostatSetSingleTarget { target_celsius } => {
            InteractionRequest::ThermostatSetSingleTarget {
                target,
                target_celsius: target_celsius.0.clone(),
            }
        }
        CapabilityOperation::ThermostatSetDualTarget {
            heat_target_celsius,
            cool_target_celsius,
        } => InteractionRequest::ThermostatSetDualTarget {
            target,
            heat_target_celsius: heat_target_celsius.0.clone(),
            cool_target_celsius: cool_target_celsius.0.clone(),
        },
        CapabilityOperation::ThermostatSetFan { fan_choice } => {
            InteractionRequest::ThermostatSetFan {
                target,
                fan_choice: fan_choice.0.clone(),
            }
        }
        CapabilityOperation::ThermostatSetSwing { swing_choice } => {
            InteractionRequest::ThermostatSetSwing {
                target,
                swing_choice: swing_choice.0.clone(),
            }
        }
        CapabilityOperation::ThermostatSetPreset { preset_choice } => {
            InteractionRequest::ThermostatSetPreset {
                target,
                preset_choice: preset_choice.0.clone(),
            }
        }
        CapabilityOperation::CoveringOpen { axis } => InteractionRequest::CoveringOpen {
            target,
            axis: interaction_axis(*axis),
        },
        CapabilityOperation::CoveringClose { axis } => InteractionRequest::CoveringClose {
            target,
            axis: interaction_axis(*axis),
        },
        CapabilityOperation::CoveringStop { axis } => InteractionRequest::CoveringStop {
            target,
            axis: interaction_axis(*axis),
        },
        CapabilityOperation::CoveringSetLift { position } => InteractionRequest::CoveringSetLift {
            target,
            position: position.0.clone(),
        },
        CapabilityOperation::CoveringSetTilt { position } => InteractionRequest::CoveringSetTilt {
            target,
            position: position.0.clone(),
        },
        CapabilityOperation::MomentaryInvoke => InteractionRequest::MomentaryInvoke { target },
        CapabilityOperation::NotificationSend {
            notification_template_id,
            priority,
            expires_in_ms,
        } => InteractionRequest::NotificationSend {
            target,
            notification_template_id: notification_template_id.clone(),
            priority: match priority {
                litehouse_rule_json::NotificationPriority::Low => {
                    InteractionNotificationPriority::Low
                }
                litehouse_rule_json::NotificationPriority::Normal => {
                    InteractionNotificationPriority::Normal
                }
                litehouse_rule_json::NotificationPriority::High => {
                    InteractionNotificationPriority::High
                }
                litehouse_rule_json::NotificationPriority::Urgent => {
                    InteractionNotificationPriority::Urgent
                }
            },
            expires_in_ms: *expires_in_ms,
        },
    }
}

fn interaction_color(color: &RuleColor) -> InteractionColor {
    match color {
        RuleColor::HueSaturation {
            hue_degrees,
            saturation,
        } => InteractionColor::HueSaturation {
            hue_degrees: hue_degrees.0.clone(),
            saturation: saturation.0.clone(),
        },
        RuleColor::CieXy { x, y } => InteractionColor::CieXy {
            x: x.0.clone(),
            y: y.0.clone(),
        },
        RuleColor::ColorTemperature { kelvin } => InteractionColor::ColorTemperature {
            kelvin: kelvin.0.clone(),
        },
        RuleColor::CalibratedRgb {
            red,
            green,
            blue,
            color_space,
        } => InteractionColor::CalibratedRgb {
            red: red.0.clone(),
            green: green.0.clone(),
            blue: blue.0.clone(),
            color_space: match color_space {
                litehouse_rule_json::ColorSpace::Srgb => InteractionColorSpace::Srgb,
                litehouse_rule_json::ColorSpace::DisplayP3 => InteractionColorSpace::DisplayP3,
            },
        },
        RuleColor::UncalibratedRgb { red, green, blue } => InteractionColor::UncalibratedRgb {
            red: red.0.clone(),
            green: green.0.clone(),
            blue: blue.0.clone(),
        },
        RuleColor::NativeWhiteChannels {
            white,
            cold_white,
            warm_white,
        } => InteractionColor::NativeWhiteChannels {
            white: white.0.clone(),
            cold_white: cold_white.as_ref().map(|v| v.0.clone()),
            warm_white: warm_white.as_ref().map(|v| v.0.clone()),
        },
    }
}

fn interaction_mode(mode: RuleThermostatMode) -> InteractionThermostatMode {
    match mode {
        RuleThermostatMode::Off => InteractionThermostatMode::Off,
        RuleThermostatMode::Heat => InteractionThermostatMode::Heat,
        RuleThermostatMode::Cool => InteractionThermostatMode::Cool,
        RuleThermostatMode::Auto => InteractionThermostatMode::Auto,
        RuleThermostatMode::Dry => InteractionThermostatMode::Dry,
        RuleThermostatMode::FanOnly => InteractionThermostatMode::FanOnly,
    }
}

fn interaction_axis(axis: RuleCoveringAxis) -> InteractionCoveringAxis {
    match axis {
        RuleCoveringAxis::Lift => InteractionCoveringAxis::Lift,
        RuleCoveringAxis::Tilt => InteractionCoveringAxis::Tilt,
    }
}

async fn query_facet(
    management: &ManagementService,
    principal: &Principal,
    facet: &ResolvedFacet,
) -> Result<FacetQueryResult, String> {
    management
        .facet_query(
            principal,
            FacetQueryRequest {
                facet_id: facet.facet_id.clone(),
                facet_generation: facet.generation,
                timeout_ms: Some(QUERY_TIMEOUT_MS),
            },
        )
        .await
        .map_err(|error| error.safe_message.clone())
}

async fn query_snapshot(
    management: &ManagementService,
    principal: &Principal,
    facet: &ResolvedFacet,
    fields: &[StateFieldRef],
    evaluation_time: Timestamp,
) -> Result<CapabilitySnapshot, String> {
    let query = query_facet(management, principal, facet).await?;
    let received_at = timestamp_from_milliseconds(query.received_at_unix_milliseconds);
    if received_at > evaluation_time {
        return Err("provider observation timestamp is later than evaluation time".to_owned());
    }
    let freshness = if matches!(query.availability, Availability::Available) {
        Freshness::Fresh
    } else {
        Freshness::Stale
    };
    let fields = fields
        .iter()
        .cloned()
        .map(|field| {
            let state = snapshot_value(&field, &query);
            SnapshotField::new(field, state).map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    CapabilitySnapshot::new(
        facet.selector.clone(),
        query.generation,
        query.revision,
        None,
        received_at,
        freshness,
        fields,
    )
    .map_err(|error| error.to_string())
}

fn snapshot_value(field: &StateFieldRef, query: &FacetQueryResult) -> ValueState {
    if matches!(query.availability, Availability::Unavailable) {
        return ValueState::Unknown {
            reason: UnknownReason::Unavailable,
        };
    }
    let optional = |value: Option<RuntimeValue>| match value {
        Some(value) => ValueState::Known { value },
        None => ValueState::Missing {
            reason: MissingReason::NotReported,
        },
    };
    match (field, &query.state) {
        (StateFieldRef::OnOffCurrent, FacetState::OnOff { current })
        | (StateFieldRef::BooleanCurrent, FacetState::BooleanState { current }) => {
            optional(current.map(RuntimeValue::Bool))
        }
        (StateFieldRef::LevelCurrentLevel, FacetState::Level { current, .. }) => optional(
            current
                .as_ref()
                .and_then(runtime_quantity)
                .map(RuntimeValue::Quantity),
        ),
        (StateFieldRef::LevelTargetLevel, FacetState::Level { target, .. }) => optional(
            target
                .as_ref()
                .and_then(runtime_quantity)
                .map(RuntimeValue::Quantity),
        ),
        (
            StateFieldRef::LevelTransitionRemaining,
            FacetState::Level {
                transition_remaining_milliseconds,
                ..
            },
        ) => optional(transition_remaining_milliseconds.map(RuntimeValue::U64)),
        (StateFieldRef::ColorCurrentColor, FacetState::Color { current, .. }) => optional(
            current
                .as_ref()
                .map(runtime_color)
                .map(RuntimeValue::ColorValue),
        ),
        (StateFieldRef::ColorTargetColor, FacetState::Color { target, .. }) => optional(
            target
                .as_ref()
                .map(runtime_color)
                .map(RuntimeValue::ColorValue),
        ),
        (StateFieldRef::ColorActiveModel, FacetState::Color { active_model, .. }) => optional(
            active_model
                .map(runtime_color_model)
                .map(RuntimeValue::ColorModel),
        ),
        (StateFieldRef::MeasurementCurrent { profile }, FacetState::Measurement { current }) => {
            if current
                .as_ref()
                .is_some_and(|value| value.profile != profile.as_str())
            {
                ValueState::Error {
                    code: EvaluationErrorCode::TypeMismatch,
                }
            } else {
                optional(
                    current
                        .as_ref()
                        .and_then(|value| {
                            Quantity::parse(profile.as_str(), &value.magnitude, &value.unit).ok()
                        })
                        .map(RuntimeValue::Quantity),
                )
            }
        }
        (StateFieldRef::MeasurementNativeRepresentation, FacetState::Measurement { current }) => {
            optional(
                current
                    .as_ref()
                    .and_then(|value| {
                        value
                            .native_magnitude
                            .as_ref()
                            .zip(value.native_unit.as_ref())
                    })
                    .map(|(magnitude, unit)| {
                        RuntimeValue::NativeRepresentation(
                            litehouse_rule_json::NativeRepresentation {
                                magnitude: magnitude.clone(),
                                unit: unit.clone(),
                                quantization: None,
                            },
                        )
                    }),
            )
        }
        (StateFieldRef::ThermostatCurrentMode, FacetState::Thermostat { current_mode, .. }) => {
            optional(
                current_mode
                    .as_ref()
                    .map(runtime_thermostat_mode)
                    .map(RuntimeValue::ThermostatMode),
            )
        }
        (StateFieldRef::ThermostatTargetMode, FacetState::Thermostat { target_mode, .. }) => {
            optional(
                target_mode
                    .as_ref()
                    .map(runtime_thermostat_mode)
                    .map(RuntimeValue::ThermostatMode),
            )
        }
        (StateFieldRef::ThermostatAction, FacetState::Thermostat { current_action, .. }) => {
            optional(
                current_action
                    .map(runtime_thermostat_action)
                    .map(RuntimeValue::ThermostatAction),
            )
        }
        (StateFieldRef::ThermostatRunning, FacetState::Thermostat { running, .. }) => {
            optional(running.map(RuntimeValue::Bool))
        }
        (StateFieldRef::ThermostatTargets, FacetState::Thermostat { targets, .. }) => optional(
            targets
                .as_ref()
                .and_then(runtime_targets)
                .map(RuntimeValue::ThermostatTargets),
        ),
        (StateFieldRef::ThermostatFan, FacetState::Thermostat { fan, .. }) => {
            optional(fan.clone().map(RuntimeValue::String))
        }
        (StateFieldRef::ThermostatSwing, FacetState::Thermostat { swing, .. }) => {
            optional(swing.clone().map(RuntimeValue::String))
        }
        (StateFieldRef::ThermostatPreset, FacetState::Thermostat { preset, .. }) => {
            optional(preset.clone().map(RuntimeValue::String))
        }
        (
            StateFieldRef::CoveringCurrentPosition { axis },
            FacetState::WindowCovering { axes, .. },
        ) => optional(
            covering_axis(axes, *axis)
                .and_then(|axis| axis.current.as_ref())
                .and_then(runtime_quantity)
                .map(RuntimeValue::Quantity),
        ),
        (
            StateFieldRef::CoveringTargetPosition { axis },
            FacetState::WindowCovering { axes, .. },
        ) => optional(
            covering_axis(axes, *axis)
                .and_then(|axis| axis.target.as_ref())
                .and_then(runtime_quantity)
                .map(RuntimeValue::Quantity),
        ),
        (StateFieldRef::CoveringMovement { axis }, FacetState::WindowCovering { axes, .. }) => {
            optional(
                covering_axis(axes, *axis)
                    .map(|axis| RuntimeValue::CoveringMovement(runtime_movement(axis.movement))),
            )
        }
        (StateFieldRef::CoveringSafetyStatus, FacetState::WindowCovering { safety_status, .. }) => {
            ValueState::Known {
                value: RuntimeValue::StringList(safety_status.clone()),
            }
        }
        (StateFieldRef::CoveringFaultStatus, FacetState::WindowCovering { fault_status, .. }) => {
            ValueState::Known {
                value: RuntimeValue::StringList(fault_status.clone()),
            }
        }
        (StateFieldRef::ObservationRevision { .. }, _) => ValueState::Known {
            value: RuntimeValue::U64(query.revision),
        },
        _ => ValueState::Error {
            code: EvaluationErrorCode::TypeMismatch,
        },
    }
}

fn runtime_quantity(value: &crate::QuantityValue) -> Option<Quantity> {
    Quantity::parse(&value.profile, &value.magnitude, &value.unit).ok()
}

fn runtime_color(value: &ColorValue) -> RuntimeColorValue {
    match value {
        ColorValue::HueSaturation {
            hue_degrees,
            saturation_ratio,
        } => RuntimeColorValue::HueSaturation {
            hue_degrees: litehouse_rule_json::DecimalValue::new(hue_degrees),
            saturation_ratio: litehouse_rule_json::DecimalValue::new(saturation_ratio),
        },
        ColorValue::CieXy { x, y } => RuntimeColorValue::CieXy {
            x: litehouse_rule_json::DecimalValue::new(x),
            y: litehouse_rule_json::DecimalValue::new(y),
        },
        ColorValue::ColorTemperature { kelvin } => RuntimeColorValue::ColorTemperature {
            kelvin: litehouse_rule_json::DecimalValue::new(kelvin),
        },
        ColorValue::CalibratedRgb {
            red,
            green,
            blue,
            color_space,
        } => RuntimeColorValue::CalibratedRgb {
            red: litehouse_rule_json::DecimalValue::new(red),
            green: litehouse_rule_json::DecimalValue::new(green),
            blue: litehouse_rule_json::DecimalValue::new(blue),
            color_space: color_space.clone(),
        },
        ColorValue::UncalibratedRgb { red, green, blue } => RuntimeColorValue::UncalibratedRgb {
            red: litehouse_rule_json::DecimalValue::new(red),
            green: litehouse_rule_json::DecimalValue::new(green),
            blue: litehouse_rule_json::DecimalValue::new(blue),
        },
        ColorValue::NativeWhiteChannels {
            white,
            cold_white,
            warm_white,
        } => RuntimeColorValue::NativeWhiteChannels {
            white: white.as_ref().map(litehouse_rule_json::DecimalValue::new),
            cold_white: cold_white
                .as_ref()
                .map(litehouse_rule_json::DecimalValue::new),
            warm_white: warm_white
                .as_ref()
                .map(litehouse_rule_json::DecimalValue::new),
        },
    }
}

fn runtime_color_model(value: ColorModel) -> RuntimeColorModel {
    match value {
        ColorModel::HueSaturation => RuntimeColorModel::HueSaturation,
        ColorModel::CieXy => RuntimeColorModel::CieXy,
        ColorModel::ColorTemperature => RuntimeColorModel::ColorTemperature,
        ColorModel::CalibratedRgb => RuntimeColorModel::CalibratedRgb,
        ColorModel::UncalibratedRgb => RuntimeColorModel::UncalibratedRgb,
        ColorModel::NativeWhiteChannels => RuntimeColorModel::NativeWhiteChannels,
    }
}

fn runtime_thermostat_mode(value: &ThermostatMode) -> ThermostatModeValue {
    ThermostatModeValue::Known(match value {
        ThermostatMode::Off => RuleThermostatMode::Off,
        ThermostatMode::Auto => RuleThermostatMode::Auto,
        ThermostatMode::Heat => RuleThermostatMode::Heat,
        ThermostatMode::Cool => RuleThermostatMode::Cool,
        ThermostatMode::Dry => RuleThermostatMode::Dry,
        ThermostatMode::FanOnly => RuleThermostatMode::FanOnly,
        ThermostatMode::ProviderQualified(value) => {
            return ThermostatModeValue::ProviderQualified(value.clone());
        }
    })
}

fn runtime_thermostat_action(value: ThermostatAction) -> RuntimeThermostatAction {
    match value {
        ThermostatAction::Idle => RuntimeThermostatAction::Idle,
        ThermostatAction::Heating => RuntimeThermostatAction::Heating,
        ThermostatAction::Cooling => RuntimeThermostatAction::Cooling,
        ThermostatAction::Drying => RuntimeThermostatAction::Drying,
        ThermostatAction::Fan => RuntimeThermostatAction::Fan,
        ThermostatAction::Defrosting => RuntimeThermostatAction::Defrosting,
    }
}

fn runtime_targets(value: &ThermostatTargets) -> Option<RuntimeTargets> {
    match value {
        ThermostatTargets::Single { temperature } => Some(RuntimeTargets::Single {
            temperature: runtime_quantity(temperature)?,
        }),
        ThermostatTargets::HeatCool { heat, cool } => Some(RuntimeTargets::HeatCool {
            heat: runtime_quantity(heat)?,
            cool: runtime_quantity(cool)?,
        }),
    }
}

fn covering_axis(
    axes: &[crate::CoveringAxisState],
    wanted: RuleCoveringAxis,
) -> Option<&crate::CoveringAxisState> {
    axes.iter().find(|axis| {
        matches!(
            (axis.axis, wanted),
            (CoveringAxis::Lift, RuleCoveringAxis::Lift)
                | (CoveringAxis::Tilt, RuleCoveringAxis::Tilt)
        )
    })
}

fn runtime_movement(value: CoveringMovement) -> RuntimeCoveringMovement {
    match value {
        CoveringMovement::Opening => RuntimeCoveringMovement::Opening,
        CoveringMovement::Closing => RuntimeCoveringMovement::Closing,
        CoveringMovement::Stopped => RuntimeCoveringMovement::Stopped,
        CoveringMovement::Idle => RuntimeCoveringMovement::Idle,
        CoveringMovement::Unknown => RuntimeCoveringMovement::Unknown,
    }
}

fn receipt_projection(confidence: ReceiptConfidence) -> (PlannerReceiptConfidence, ReceiptOutcome) {
    match confidence {
        ReceiptConfidence::AcceptedForDelivery => (
            PlannerReceiptConfidence::AcceptedForDelivery,
            ReceiptOutcome::AcceptedForDelivery,
        ),
        ReceiptConfidence::Acknowledged => (
            PlannerReceiptConfidence::ProtocolOrTargetAcknowledged,
            ReceiptOutcome::ProtocolOrTargetAcknowledged,
        ),
        ReceiptConfidence::Completed => (
            PlannerReceiptConfidence::Completed,
            ReceiptOutcome::Completed,
        ),
        ReceiptConfidence::ObservedCompleted => (
            PlannerReceiptConfidence::ObservedCompleted,
            ReceiptOutcome::ObservedCompleted,
        ),
    }
}

fn terminal_status(outcome: TransitionOutcome) -> WorkflowRunStatus {
    match outcome {
        TransitionOutcome::Success | TransitionOutcome::Noop => WorkflowRunStatus::Succeeded,
        TransitionOutcome::Cancelled => WorkflowRunStatus::Cancelled,
        TransitionOutcome::CapacityRejected => WorkflowRunStatus::Skipped,
        TransitionOutcome::Suspended => WorkflowRunStatus::TimedOut,
        TransitionOutcome::Unknown | TransitionOutcome::Error => WorkflowRunStatus::Failed,
    }
}

fn selector_key(selector: &Selector) -> String {
    format!(
        "{}/{}/{}:{}@{}",
        selector.provider_instance_id,
        selector.facet_key,
        selector.contract.namespace,
        selector.contract.package,
        selector.contract.major
    )
}

fn deadline_remaining_ms(deadline: Timestamp) -> u32 {
    let now = now_timestamp();
    let deadline_ms =
        i128::from(deadline.unix_seconds()) * 1_000 + i128::from(deadline.nanosecond() / 1_000_000);
    let now_ms = i128::from(now.unix_seconds()) * 1_000 + i128::from(now.nanosecond() / 1_000_000);
    deadline_ms.saturating_sub(now_ms).clamp(1, 30_000) as u32
}

fn now_timestamp() -> Timestamp {
    timestamp_from_milliseconds(now_unix_milliseconds())
}

fn timestamp_from_milliseconds(milliseconds: u64) -> Timestamp {
    Timestamp::new(
        (milliseconds / 1_000) as i64,
        ((milliseconds % 1_000) * 1_000_000) as u32,
    )
    .expect("system timestamps are normalized")
}

fn now_unix_milliseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn fresh_hex_id() -> String {
    let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{:016x}{sequence:016x}", now_unix_milliseconds())
}

fn activation_error(compiled: &CompiledRule, reason: impl Into<String>) -> WorkflowExecutorError {
    WorkflowExecutorError::Activation {
        rule_id: compiled.rule_id().to_owned(),
        reason: reason.into(),
    }
}

fn execution_error(compiled: &CompiledRule, reason: impl Into<String>) -> WorkflowExecutorError {
    WorkflowExecutorError::Execution {
        rule_id: compiled.rule_id().to_owned(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use litehouse_rule_json::compile_rule_json;
    use std::collections::BTreeSet;

    fn target() -> InteractionTarget {
        InteractionTarget {
            facet_id: "target".to_owned(),
            facet_generation: 1,
            timeout_ms: Some(1),
            idempotency_key: "00000000000000000000000000000000".to_owned(),
        }
    }

    #[test]
    fn all_compiled_operations_have_a_typed_management_projection() {
        let compiled = compile_rule_json(include_bytes!(
            "../../litehouse-rule-json/fixtures/valid-all-operations.json"
        ))
        .unwrap();
        let mut projected = BTreeSet::new();
        for node in compiled.nodes().values() {
            if let CompiledNode::EffectIntent { operation, .. } = node {
                projected.insert(interaction_request(target(), operation).operation_id());
            }
        }
        assert_eq!(
            projected.len(),
            litehouse_rule_json::CapabilityOperationKind::ALL.len()
        );
    }

    #[test]
    fn outcome_mapping_never_reports_a_suspension_as_success() {
        assert_eq!(
            terminal_status(TransitionOutcome::Suspended),
            WorkflowRunStatus::TimedOut
        );
        assert_eq!(
            terminal_status(TransitionOutcome::Unknown),
            WorkflowRunStatus::Failed
        );
        assert_eq!(
            terminal_status(TransitionOutcome::Success),
            WorkflowRunStatus::Succeeded
        );
    }

    #[test]
    fn transition_remaining_projects_as_unsigned_milliseconds() {
        let query = FacetQueryResult {
            facet_id: "dimmer".to_owned(),
            generation: 1,
            revision: 2,
            availability: Availability::Available,
            unavailability_reason: None,
            quality: None,
            received_at_unix_milliseconds: 1,
            state: FacetState::Level {
                current: None,
                target: None,
                transition_remaining_milliseconds: Some(250),
            },
        };
        assert_eq!(
            snapshot_value(&StateFieldRef::LevelTransitionRemaining, &query),
            ValueState::Known {
                value: RuntimeValue::U64(250)
            }
        );
    }

    #[test]
    fn active_runs_process_continuations_before_new_triggers() {
        let anchor = Timestamp::new(0, 0).unwrap();
        let scheduled = Timestamp::new(1, 0).unwrap();
        let fired = Timestamp::new(2, 0).unwrap();
        let trigger = TriggerInput::time_schedule("occurrence", anchor, scheduled).unwrap();
        let mut events = VecDeque::from([
            EvaluationEvent::initial(trigger),
            EvaluationEvent::timer("timer", "suspension", scheduled, fired).unwrap(),
        ]);
        assert!(matches!(
            dequeue_raw_event(&mut events, true),
            Some(EvaluationEvent::Timer { timer_id, .. }) if timer_id == "timer"
        ));
        assert!(matches!(
            events.front(),
            Some(EvaluationEvent::Initial { .. })
        ));
    }

    #[test]
    fn superseded_timers_are_removed_before_they_can_fire() {
        let timestamp = Timestamp::new(1, 0).unwrap();
        let mut armed = vec![
            ScheduledTimer {
                timer_id: "stale".to_owned(),
                suspension_id: "old".to_owned(),
                scheduled_for: timestamp,
            },
            ScheduledTimer {
                timer_id: "current".to_owned(),
                suspension_id: "new".to_owned(),
                scheduled_for: timestamp,
            },
        ];
        reconcile_armed_timers(&mut armed, &BTreeSet::from(["current".to_owned()]), &[]);
        assert_eq!(armed.len(), 1);
        assert_eq!(armed[0].timer_id, "current");

        let fired_at = Timestamp::new(2, 0).unwrap();
        let mut events = VecDeque::from([
            EvaluationEvent::timer("stale", "old", timestamp, fired_at).unwrap(),
            EvaluationEvent::timer("current", "new", timestamp, fired_at).unwrap(),
        ]);
        discard_stale_timer_events(&mut events, &BTreeSet::from(["current".to_owned()]));
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events.front(),
            Some(EvaluationEvent::Timer { timer_id, .. }) if timer_id == "current"
        ));
    }
}
