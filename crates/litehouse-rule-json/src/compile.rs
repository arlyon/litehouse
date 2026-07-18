use std::collections::{BTreeMap, BTreeSet, VecDeque};

use litehouse_contracts::quantity::Quantity;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    catalog::{
        PortCardinality as CatalogCardinality, PortDirection as CatalogDirection, ResolvedPort,
        resolved_node_ports, resolved_trigger_ports,
    },
    model::{
        AdmissionPolicy, CapabilityOperation, CelCondition, CompareOperator, ContractRef,
        DesiredPatchOperation, EditorGroupKey, EditorNodeKey, EndOutcome, ForkJoinPhase,
        HARD_MAX_CEL_EVAL_MS, HARD_MAX_CEL_MEMORY_BYTES, HARD_MAX_CEL_SOURCE_BYTES,
        HARD_MAX_CEL_STEPS, HARD_MAX_COLLECTION_ITEMS, HARD_MAX_CONDITION_NODES, HARD_MAX_EDGES,
        HARD_MAX_EFFECT_INTENTS, HARD_MAX_GRAPH_DEPTH, HARD_MAX_JSON_BYTES, HARD_MAX_JSON_DEPTH,
        HARD_MAX_NODES, HARD_MAX_PARALLEL_BRANCHES, HARD_MAX_PATCH_BYTES,
        HARD_MAX_PATCH_OPERATIONS, HARD_MAX_PLAN_INPUT_BYTES, HARD_MAX_QUEUED_TRIGGERS,
        HARD_MAX_STRING_BYTES, HARD_MAX_TIMER_DURATION_MS, HARD_MAX_TIMERS_PER_RULE,
        HARD_MAX_TRACE_BYTES, Node, QuantityCatalogRef, QuantityOperation, QuantityProfileRef,
        RebindPolicy, ReceiptRequirement, RetryPolicy, RuleDocument, Selector, SelectorCardinality,
        TriggerContextKind, TriggerNode, TypeRef, ValueType, valid_contract_part,
        valid_decimal_shape, valid_hex_32, valid_hex_64, valid_id,
    },
    schema::artifact_digests,
};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuleJsonError {
    #[error("rule JSON is {actual} bytes; the maximum is {maximum}")]
    JsonTooLarge { actual: usize, maximum: usize },
    #[error("rule JSON is not valid JSON: {0}")]
    InvalidJson(String),
    #[error("rule JSON shape is invalid: {0}")]
    InvalidShape(String),
    #[error("JSON nesting exceeds {maximum} levels")]
    JsonDepthExceeded { maximum: usize },
    #[error("JSON collection has {actual} items; the maximum is {maximum}")]
    CollectionTooLarge { actual: usize, maximum: usize },
    #[error("JSON string is {actual} bytes; the maximum is {maximum}")]
    StringTooLarge { actual: usize, maximum: usize },
    #[error("floating-point JSON numbers are prohibited")]
    FloatingPointNumber,
    #[error(transparent)]
    Compile(#[from] CompileError),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CompileError {
    #[error("schema digest does not match the shipped schema")]
    SchemaDigestMismatch,
    #[error("profile or artifact digest must be 64 lowercase hexadecimal characters")]
    InvalidProfileDigest,
    #[error("evaluation profile digest does not match the shipped artifact manifest")]
    EvaluationProfileDigestMismatch,
    #[error("{artifact} digest does not match its authoritative shipped artifact")]
    ArtifactDigestMismatch { artifact: &'static str },
    #[error("rule ID must be 32 lowercase hexadecimal characters")]
    InvalidRuleId,
    #[error("rule revision must be non-zero")]
    InvalidRevision,
    #[error("rule limit {name}={actual} is outside {minimum}..={maximum}")]
    InvalidLimit {
        name: &'static str,
        actual: usize,
        minimum: usize,
        maximum: usize,
    },
    #[error("{path} has {actual} items/bytes; the maximum is {maximum}")]
    BoundExceeded {
        path: String,
        actual: usize,
        maximum: usize,
    },
    #[error("{path} must not be empty")]
    EmptyValue { path: String },
    #[error("rule has {actual} nodes; configured maximum is {maximum}")]
    TooManyNodes { actual: usize, maximum: usize },
    #[error("rule has {actual} edges; configured maximum is {maximum}")]
    TooManyEdges { actual: usize, maximum: usize },
    #[error("invalid {kind} ID {id:?}")]
    InvalidId { kind: &'static str, id: String },
    #[error("duplicate node ID {id:?}")]
    DuplicateNodeId { id: String },
    #[error("duplicate edge ID {id:?}")]
    DuplicateEdgeId { id: String },
    #[error("node {node:?} has unsupported version {version}")]
    UnsupportedNodeVersion { node: String, version: u8 },
    #[error("selector on {owner:?} is invalid: {reason}")]
    InvalidSelector { owner: String, reason: String },
    #[error("discrete event selector contract does not match its closed event inventory")]
    EventContractMismatch,
    #[error("snapshot {node:?} field does not belong to selector contract")]
    SnapshotContractMismatch { node: String },
    #[error("effect {node:?} operation {operation} does not belong to selector contract")]
    OperationContractMismatch { node: String, operation: String },
    #[error("desired patch {node:?} references missing or non-snapshot target {snapshot:?}")]
    InvalidDesiredPatchTarget { node: String, snapshot: String },
    #[error("desired patch {node:?} path does not belong to snapshot selector contract")]
    DesiredPatchContractMismatch { node: String },
    #[error("effect {node:?} has an invalid receipt requirement: {reason}")]
    InvalidReceiptRequirement { node: String, reason: String },
    #[error("effect {node:?} has an invalid retry policy: {reason}")]
    InvalidRetryPolicy { node: String, reason: String },
    #[error("node {node:?} uses incompatible comparison types {left} and {right}")]
    ComparisonTypeMismatch {
        node: String,
        left: String,
        right: String,
    },
    #[error("operator {operator:?} is not supported for {value_type}")]
    UnsupportedComparison {
        operator: CompareOperator,
        value_type: String,
    },
    #[error("quantity literal on {node:?} is invalid: {reason}")]
    InvalidQuantity { node: String, reason: String },
    #[error("node {node:?} has invalid authored data: {reason}")]
    InvalidNodeData { node: String, reason: String },
    #[error("edge {edge:?} references unknown node {node:?}")]
    DanglingNode { edge: String, node: String },
    #[error("edge {edge:?} references unknown {direction} port {node:?}.{port}")]
    UnknownPort {
        edge: String,
        direction: &'static str,
        node: String,
        port: String,
    },
    #[error("edge {edge:?} connects incompatible ports: {from} -> {to}")]
    IncompatiblePorts {
        edge: String,
        from: String,
        to: String,
    },
    #[error("receipt branch {node:?} must be wired from its declared effect {effect:?}")]
    ReceiptEffectMismatch { node: String, effect: String },
    #[error("duplicate order {order} at {node:?}.{port}")]
    DuplicateEdgeOrder {
        node: String,
        port: String,
        order: u16,
    },
    #[error("edge order at {node:?}.{port} must start at zero without gaps")]
    NonContiguousEdgeOrder { node: String, port: String },
    #[error("input {node:?}.{port} requires {expected}; found {actual}")]
    InputCardinality {
        node: String,
        port: String,
        expected: &'static str,
        actual: usize,
    },
    #[error("output {node:?}.{port} permits at most one edge; found {actual}")]
    OutputCardinality {
        node: String,
        port: String,
        actual: usize,
    },
    #[error("trigger cannot have incoming edges")]
    TriggerHasIncomingEdge,
    #[error("graph contains a cycle")]
    Cycle,
    #[error("node {node:?} is not reachable from the trigger")]
    UnreachableNode { node: String },
    #[error("graph depth {actual} exceeds configured maximum {maximum}")]
    GraphTooDeep { actual: usize, maximum: usize },
    #[error("planner subset does not yet support {kind} at {node:?}")]
    PlannerSubsetUnsupported { node: String, kind: String },
    #[error("condition on {node:?} is invalid for compatibility lowering: {reason}")]
    InvalidCondition { node: String, reason: String },
}

#[derive(Debug, Clone)]
pub struct CompiledRule {
    pub(crate) rule_id: String,
    pub(crate) revision: u64,
    pub(crate) profile_sha256: String,
    pub(crate) trigger_id: String,
    pub(crate) trigger_selector: Selector,
    pub(crate) nodes: BTreeMap<String, CompiledNode>,
    pub(crate) outgoing: BTreeMap<(String, String), String>,
    pub(crate) data_sources: BTreeMap<(String, String), (String, String)>,
    pub(crate) topological_order: Vec<String>,
    pub(crate) max_plan_input_bytes: usize,
    pub(crate) max_trace_bytes: usize,
}

impl CompiledRule {
    pub fn rule_id(&self) -> &str {
        &self.rule_id
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn profile_sha256(&self) -> &str {
        &self.profile_sha256
    }

    pub fn trigger_id(&self) -> &str {
        &self.trigger_id
    }

    pub fn topological_order(&self) -> &[String] {
        &self.topological_order
    }
}

#[derive(Debug, Clone)]
pub(crate) enum CompiledNode {
    Snapshot {
        selector: Selector,
        field: String,
        value_type: ValueType,
    },
    Compare {
        input_port: &'static str,
        value_type: ValueType,
        operator: CompareOperator,
        right: CompiledScalar,
    },
    Branch,
    OnOffSet {
        selector: Selector,
        value: bool,
    },
    ReceiptBranch,
    End {
        outcome: EndOutcome,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompiledScalar {
    Bool(bool),
    String(String),
    I64(i64),
    U64(u64),
    Quantity(Quantity),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Direction {
    Input,
    Output,
}

#[derive(Debug, Clone)]
struct Port {
    direction: Direction,
    type_ref: TypeRef,
    minimum: usize,
    maximum: Option<usize>,
}

pub fn compile_rule_json(bytes: &[u8]) -> Result<CompiledRule, RuleJsonError> {
    if bytes.len() > HARD_MAX_JSON_BYTES {
        return Err(RuleJsonError::JsonTooLarge {
            actual: bytes.len(),
            maximum: HARD_MAX_JSON_BYTES,
        });
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| RuleJsonError::InvalidJson(error.to_string()))?;
    validate_json_value(&value, 1)?;
    let document: RuleDocument = serde_json::from_slice(bytes)
        .map_err(|error| RuleJsonError::InvalidShape(error.to_string()))?;
    compile(document).map_err(Into::into)
}

pub(crate) fn compile(document: RuleDocument) -> Result<CompiledRule, CompileError> {
    validate_document_bounds(&document)?;

    let trigger_id = document.semantic.trigger.id().to_owned();
    validate_node_id("trigger", &trigger_id)?;
    validate_version(&trigger_id, document.semantic.trigger.version())?;
    validate_trigger(&document.semantic.trigger)?;

    let mut raw_nodes = BTreeMap::new();
    raw_nodes.insert(
        trigger_id.clone(),
        RawNode::Trigger(&document.semantic.trigger),
    );
    for node in &document.semantic.nodes {
        validate_node_id("node", node.id())?;
        validate_version(node.id(), node.version())?;
        if raw_nodes
            .insert(node.id().to_owned(), RawNode::Node(node))
            .is_some()
        {
            return Err(CompileError::DuplicateNodeId {
                id: node.id().to_owned(),
            });
        }
    }

    validate_semantic_nodes(&document, &raw_nodes)?;
    let graph = validate_edges(&raw_nodes, &document.semantic.edges, &trigger_id)?;
    let topological_order = topological_order(&raw_nodes, &document.semantic.edges)?;
    validate_reachability(&raw_nodes, &document.semantic.edges, &trigger_id)?;
    let depth = graph_depth(&topological_order, &document.semantic.edges);
    if depth > usize::from(document.semantic.limits.max_depth) {
        return Err(CompileError::GraphTooDeep {
            actual: depth,
            maximum: usize::from(document.semantic.limits.max_depth),
        });
    }

    let trigger_selector = match &document.semantic.trigger {
        TriggerNode::StateObservation { selector, .. } => selector.clone(),
        trigger => {
            return Err(CompileError::PlannerSubsetUnsupported {
                node: trigger_id,
                kind: trigger.kind_name().to_owned(),
            });
        }
    };

    let mut nodes = BTreeMap::new();
    for node in &document.semantic.nodes {
        nodes.insert(node.id().to_owned(), lower_compatibility_node(node)?);
    }

    Ok(CompiledRule {
        rule_id: document.rule_id,
        revision: document.revision,
        profile_sha256: document.schema.profile_sha256,
        trigger_id: document.semantic.trigger.id().to_owned(),
        trigger_selector,
        nodes,
        outgoing: graph.outgoing,
        data_sources: graph.data_sources,
        topological_order,
        max_plan_input_bytes: document.semantic.limits.max_plan_input_bytes as usize,
        max_trace_bytes: document.semantic.limits.max_trace_bytes as usize,
    })
}

fn validate_json_value(value: &Value, depth: usize) -> Result<(), RuleJsonError> {
    if depth > HARD_MAX_JSON_DEPTH {
        return Err(RuleJsonError::JsonDepthExceeded {
            maximum: HARD_MAX_JSON_DEPTH,
        });
    }
    match value {
        Value::String(value) if value.len() > HARD_MAX_STRING_BYTES => {
            Err(RuleJsonError::StringTooLarge {
                actual: value.len(),
                maximum: HARD_MAX_STRING_BYTES,
            })
        }
        Value::Number(value) if !value.is_i64() && !value.is_u64() => {
            Err(RuleJsonError::FloatingPointNumber)
        }
        Value::Array(values) => {
            if values.len() > HARD_MAX_COLLECTION_ITEMS {
                return Err(RuleJsonError::CollectionTooLarge {
                    actual: values.len(),
                    maximum: HARD_MAX_COLLECTION_ITEMS,
                });
            }
            for value in values {
                validate_json_value(value, depth + 1)?;
            }
            Ok(())
        }
        Value::Object(values) => {
            if values.len() > HARD_MAX_COLLECTION_ITEMS {
                return Err(RuleJsonError::CollectionTooLarge {
                    actual: values.len(),
                    maximum: HARD_MAX_COLLECTION_ITEMS,
                });
            }
            for (key, value) in values {
                if key.len() > HARD_MAX_STRING_BYTES {
                    return Err(RuleJsonError::StringTooLarge {
                        actual: key.len(),
                        maximum: HARD_MAX_STRING_BYTES,
                    });
                }
                validate_json_value(value, depth + 1)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_document_bounds(document: &RuleDocument) -> Result<(), CompileError> {
    let artifacts = artifact_digests();
    if document.schema.sha256 != artifacts.schema {
        return Err(CompileError::SchemaDigestMismatch);
    }
    if !valid_hex_64(&document.schema.profile_sha256) {
        return Err(CompileError::InvalidProfileDigest);
    }
    if document.schema.profile_sha256 != artifacts.profile {
        return Err(CompileError::EvaluationProfileDigestMismatch);
    }
    if !valid_hex_32(&document.rule_id) {
        return Err(CompileError::InvalidRuleId);
    }
    if document.revision == 0 || document.editor.revision == 0 {
        return Err(CompileError::InvalidRevision);
    }
    if document.semantic.name.is_empty() {
        return Err(CompileError::EmptyValue {
            path: "semantic.name".to_owned(),
        });
    }
    bound("semantic.name", document.semantic.name.len(), 128)?;
    bound(
        "semantic.description",
        document.semantic.description.len(),
        4_096,
    )?;
    let locked_artifacts = [
        (
            "compiler",
            &document.semantic.locks.compiler.sha256,
            &artifacts.compiler,
        ),
        ("cel", &document.semantic.locks.cel.sha256, &artifacts.cel),
        (
            "quantity_catalog",
            &document.semantic.locks.quantity_catalog.sha256,
            &artifacts.quantity_catalog,
        ),
        (
            "runtime_profile",
            &document.semantic.locks.runtime_profile.sha256,
            &artifacts.runtime_profile,
        ),
        (
            "timezone_database",
            &document.semantic.locks.timezone_database.sha256,
            &artifacts.timezone_database,
        ),
        (
            "contract_catalog",
            &document.semantic.locks.contract_catalog.sha256,
            &artifacts.contract_catalog,
        ),
        (
            "management_operations",
            &document.semantic.locks.management_operations.sha256,
            &artifacts.management_operations,
        ),
        (
            "port_catalog",
            &document.semantic.locks.port_catalog.sha256,
            &artifacts.port_catalog,
        ),
    ];
    for (artifact, actual, expected) in locked_artifacts {
        if !valid_hex_64(actual) {
            return Err(CompileError::InvalidProfileDigest);
        }
        if actual != expected {
            return Err(CompileError::ArtifactDigestMismatch { artifact });
        }
    }

    validate_limits(document)?;
    let node_count = document.semantic.nodes.len() + 1;
    if node_count > usize::from(document.semantic.limits.max_nodes) {
        return Err(CompileError::TooManyNodes {
            actual: node_count,
            maximum: usize::from(document.semantic.limits.max_nodes),
        });
    }
    if document.semantic.edges.len() > usize::from(document.semantic.limits.max_edges) {
        return Err(CompileError::TooManyEdges {
            actual: document.semantic.edges.len(),
            maximum: usize::from(document.semantic.limits.max_edges),
        });
    }

    bound(
        "semantic.annotations",
        document.semantic.annotations.len(),
        64,
    )?;
    for (key, value) in &document.semantic.annotations {
        if !valid_annotation_key(&key.0) {
            return Err(CompileError::InvalidNodeData {
                node: "semantic".to_owned(),
                reason: format!("invalid annotation key {:?}", key.0),
            });
        }
        bound("semantic.annotations value", value.0.len(), 1_024)?;
    }
    bound("editor.nodes", document.editor.nodes.len(), 128)?;
    bound("editor.groups", document.editor.groups.len(), 32)?;
    for (EditorNodeKey(id), node) in &document.editor.nodes {
        validate_node_id("editor node", id)?;
        if let Some(group) = &node.group
            && !document.editor.groups.contains_key(group)
        {
            return Err(CompileError::InvalidNodeData {
                node: id.clone(),
                reason: "editor node references an unknown group".to_owned(),
            });
        }
    }
    for (EditorGroupKey(id), group) in &document.editor.groups {
        validate_node_id("editor group", id)?;
        if group.label.is_empty() {
            return Err(CompileError::EmptyValue {
                path: format!("editor.groups.{id}.label"),
            });
        }
        bound(&format!("editor.groups.{id}.label"), group.label.len(), 128)?;
        if group.width == 0 || group.width > 100_000 || group.height == 0 || group.height > 100_000
        {
            return Err(CompileError::InvalidNodeData {
                node: id.clone(),
                reason: "editor group dimensions must be in 1..=100000".to_owned(),
            });
        }
    }
    if !(100..=4_000).contains(&document.editor.viewport.zoom_milli) {
        return Err(CompileError::InvalidNodeData {
            node: "editor.viewport".to_owned(),
            reason: "zoom_milli must be in 100..=4000".to_owned(),
        });
    }
    Ok(())
}

fn validate_limits(document: &RuleDocument) -> Result<(), CompileError> {
    let limits = &document.semantic.limits;
    limit("max_nodes", limits.max_nodes.into(), 1, HARD_MAX_NODES)?;
    limit("max_edges", limits.max_edges.into(), 1, HARD_MAX_EDGES)?;
    limit(
        "max_depth",
        limits.max_depth.into(),
        1,
        HARD_MAX_GRAPH_DEPTH,
    )?;
    limit(
        "max_parallel_branches",
        limits.max_parallel_branches.into(),
        2,
        HARD_MAX_PARALLEL_BRANCHES,
    )?;
    limit(
        "max_queued_triggers",
        limits.max_queued_triggers.into(),
        1,
        HARD_MAX_QUEUED_TRIGGERS,
    )?;
    limit(
        "max_condition_nodes",
        limits.max_condition_nodes.into(),
        1,
        HARD_MAX_CONDITION_NODES,
    )?;
    limit(
        "max_effect_intents",
        limits.max_effect_intents.into(),
        1,
        HARD_MAX_EFFECT_INTENTS,
    )?;
    limit(
        "max_patch_operations",
        limits.max_patch_operations.into(),
        1,
        HARD_MAX_PATCH_OPERATIONS,
    )?;
    limit(
        "max_timers",
        limits.max_timers.into(),
        1,
        HARD_MAX_TIMERS_PER_RULE,
    )?;
    limit(
        "max_cel_steps",
        limits.max_cel_steps.into(),
        1,
        HARD_MAX_CEL_STEPS,
    )?;
    limit(
        "max_cel_memory_bytes",
        limits.max_cel_memory_bytes as usize,
        1,
        HARD_MAX_CEL_MEMORY_BYTES,
    )?;
    limit(
        "max_cel_eval_ms",
        limits.max_cel_eval_ms.into(),
        1,
        HARD_MAX_CEL_EVAL_MS,
    )?;
    limit(
        "max_plan_input_bytes",
        limits.max_plan_input_bytes as usize,
        1_024,
        HARD_MAX_PLAN_INPUT_BYTES,
    )?;
    limit(
        "max_trace_bytes",
        limits.max_trace_bytes as usize,
        1_024,
        HARD_MAX_TRACE_BYTES,
    )
}

fn validate_trigger(trigger: &TriggerNode) -> Result<(), CompileError> {
    match trigger {
        TriggerNode::StateObservation { selector, .. } => {
            validate_selector(trigger.id(), selector)?;
            if !is_stateful_contract(&selector.contract) {
                return Err(CompileError::InvalidSelector {
                    owner: trigger.id().to_owned(),
                    reason: "state observation requires a current stateful capability contract"
                        .to_owned(),
                });
            }
        }
        TriggerNode::DiscreteEvent {
            selector, event, ..
        } => {
            validate_selector(trigger.id(), selector)?;
            if selector.contract != event.selector_contract() {
                return Err(CompileError::EventContractMismatch);
            }
        }
        TriggerNode::TimeSchedule { schedule, .. } => validate_schedule(trigger.id(), schedule)?,
    }
    Ok(())
}

fn validate_schedule(node: &str, schedule: &crate::model::Schedule) -> Result<(), CompileError> {
    use crate::model::Schedule;
    match schedule {
        Schedule::OneShot { at } => {
            if at.nanosecond >= 1_000_000_000 {
                return invalid_node(node, "one-shot nanosecond must be below one billion");
            }
        }
        Schedule::FixedInterval { interval_ms } => {
            if !(1..=604_800_000).contains(interval_ms) {
                return invalid_node(node, "fixed interval must be in 1..=604800000 ms");
            }
        }
        Schedule::WeeklyCivil {
            days,
            time,
            timezone,
            ..
        } => {
            if days.is_empty() || days.len() > 7 {
                return invalid_node(node, "weekly schedule must contain 1..=7 days");
            }
            let unique = days.iter().copied().collect::<BTreeSet<_>>();
            if unique.len() != days.len() {
                return invalid_node(node, "weekly schedule days must be unique");
            }
            if time.hour > 23 || time.minute > 59 || time.second > 59 {
                return invalid_node(node, "weekly civil time is invalid");
            }
            if timezone.is_empty() || timezone.len() > 64 {
                return invalid_node(node, "timezone must contain 1..=64 bytes");
            }
        }
    }
    Ok(())
}

fn validate_semantic_nodes(
    document: &RuleDocument,
    nodes: &BTreeMap<String, RawNode<'_>>,
) -> Result<(), CompileError> {
    let mut condition_count = 0;
    let mut effect_count = 0;
    let mut timer_count = 0;
    let mut patch_count = 0;
    for node in &document.semantic.nodes {
        match node {
            Node::Snapshot {
                id,
                selector,
                field,
                ..
            } => {
                validate_selector(id, selector)?;
                if selector.contract != field.selector_contract() {
                    return Err(CompileError::SnapshotContractMismatch { node: id.clone() });
                }
            }
            Node::Condition { id, expression, .. } => {
                condition_count += 1;
                validate_condition_shape(id, expression)?;
            }
            Node::Quantity {
                id,
                input_profile,
                operation,
                ..
            } => validate_quantity_operation(id, *input_profile, operation)?,
            Node::Branch { .. } => {}
            Node::DesiredPatch {
                id,
                snapshot_node_id,
                operations,
                ..
            } => {
                patch_count += operations.len();
                if operations.is_empty() || operations.len() > HARD_MAX_PATCH_OPERATIONS {
                    return invalid_node(id, "desired patch must contain 1..=128 operations");
                }
                let encoded_bytes = serde_json::to_vec(operations)
                    .expect("closed desired patch operations must serialize")
                    .len();
                if encoded_bytes > HARD_MAX_PATCH_BYTES {
                    return Err(CompileError::BoundExceeded {
                        path: format!("semantic.nodes.{id}.operations"),
                        actual: encoded_bytes,
                        maximum: HARD_MAX_PATCH_BYTES,
                    });
                }
                let Some(RawNode::Node(Node::Snapshot {
                    selector: target_selector,
                    ..
                })) = nodes.get(snapshot_node_id)
                else {
                    return Err(CompileError::InvalidDesiredPatchTarget {
                        node: id.clone(),
                        snapshot: snapshot_node_id.clone(),
                    });
                };
                for operation in operations {
                    validate_desired_operation(id, operation)?;
                    if operation.path().package() != target_selector.contract.package {
                        return Err(CompileError::DesiredPatchContractMismatch {
                            node: id.clone(),
                        });
                    }
                }
            }
            Node::EffectIntent {
                id,
                selector,
                operation,
                deadline_ms,
                required_receipt,
                retry,
                ..
            } => {
                effect_count += 1;
                validate_selector(id, selector)?;
                if selector.contract
                    != ContractRef::litehouse(operation.kind().package(), "capability", 1)
                {
                    return Err(CompileError::OperationContractMismatch {
                        node: id.clone(),
                        operation: operation.operation_id().to_owned(),
                    });
                }
                if !(1..=30_000).contains(deadline_ms) {
                    return invalid_node(id, "deadline_ms must be in 1..=30000");
                }
                validate_operation_arguments(id, operation)?;
                validate_receipt_requirement(id, operation, *required_receipt)?;
                validate_retry(id, operation, *deadline_ms, retry)?;
            }
            Node::Delay { id, duration, .. } => {
                timer_count += 1;
                if !(1..=HARD_MAX_TIMER_DURATION_MS).contains(&duration.milliseconds) {
                    return invalid_node(id, "delay must be in 1..=86400000 ms");
                }
            }
            Node::DebounceThrottle {
                id,
                trigger,
                policy,
                ..
            } => {
                if *trigger != trigger_kind(&document.semantic.trigger) {
                    return invalid_node(
                        id,
                        "debounce/throttle trigger context must equal the rule trigger kind",
                    );
                }
                let window = match policy {
                    AdmissionPolicy::DebounceTrailing { window_ms }
                    | AdmissionPolicy::ThrottleLeading { window_ms, .. } => *window_ms,
                };
                if !(1..=HARD_MAX_TIMER_DURATION_MS).contains(&window) {
                    return invalid_node(id, "admission window must be in 1..=86400000 ms");
                }
            }
            Node::ForkJoin { id, phase, .. } => validate_fork_join(
                id,
                phase,
                nodes,
                usize::from(document.semantic.limits.max_parallel_branches),
            )?,
            Node::ReceiptBranch {
                id, effect_node_id, ..
            } => {
                if !matches!(
                    nodes.get(effect_node_id),
                    Some(RawNode::Node(Node::EffectIntent { .. }))
                ) {
                    return invalid_node(id, "receipt branch references a missing effect node");
                }
            }
            Node::End { .. } => {}
        }
    }
    configured_count(
        "condition_nodes",
        condition_count,
        usize::from(document.semantic.limits.max_condition_nodes),
    )?;
    configured_count(
        "effect_intents",
        effect_count,
        usize::from(document.semantic.limits.max_effect_intents),
    )?;
    configured_count(
        "timers",
        timer_count,
        usize::from(document.semantic.limits.max_timers),
    )?;
    configured_count(
        "patch_operations",
        patch_count,
        usize::from(document.semantic.limits.max_patch_operations),
    )
}

fn validate_condition_shape(id: &str, condition: &CelCondition) -> Result<(), CompileError> {
    if condition.source.is_empty() {
        return invalid_node(id, "CEL source must not be empty");
    }
    if condition.source.len() > HARD_MAX_CEL_SOURCE_BYTES {
        return invalid_node(id, "CEL source exceeds 4096 bytes");
    }
    if condition.inputs.is_empty() || condition.inputs.len() > 16 {
        return invalid_node(id, "condition inputs must contain 1..=16 entries");
    }
    let mut names = BTreeSet::new();
    for input in &condition.inputs {
        if !valid_condition_input_name(&input.name) || !names.insert(&input.name) {
            return invalid_node(id, "condition input names must be valid and unique");
        }
        validate_type_ref(&input.type_ref, 1)?;
    }
    if !valid_hex_64(&condition.normalized_source_sha256) {
        return invalid_node(id, "normalized source digest must be lowercase SHA-256");
    }
    Ok(())
}

fn validate_quantity_operation(
    id: &str,
    input_profile: QuantityProfileRef,
    operation: &QuantityOperation,
) -> Result<(), CompileError> {
    validate_profile(id, input_profile)?;
    match operation {
        QuantityOperation::Convert { target_unit } => {
            if target_unit.is_empty() || target_unit.len() > 256 {
                return invalid_node(id, "target unit must contain 1..=256 bytes");
            }
        }
        QuantityOperation::Compare { right, .. }
        | QuantityOperation::Add { right }
        | QuantityOperation::Subtract { right } => validate_quantity_wire(id, right)?,
        QuantityOperation::Multiply {
            right,
            result_profile,
        }
        | QuantityOperation::Divide {
            right,
            result_profile,
        } => {
            validate_quantity_wire(id, right)?;
            validate_profile(id, *result_profile)?;
        }
        QuantityOperation::Quantize { scale } => {
            if *scale > 18 {
                return invalid_node(id, "quantization scale must be at most 18");
            }
        }
    }
    Ok(())
}

fn validate_quantity_wire(
    id: &str,
    value: &crate::model::QuantityWire,
) -> Result<(), CompileError> {
    validate_profile(id, value.profile)?;
    if !valid_decimal_shape(&value.magnitude) || value.unit.is_empty() || value.unit.len() > 256 {
        return Err(CompileError::InvalidQuantity {
            node: id.to_owned(),
            reason: "quantity must use bounded canonical decimal and unit shapes".to_owned(),
        });
    }
    Ok(())
}

fn validate_profile(id: &str, profile: QuantityProfileRef) -> Result<(), CompileError> {
    Quantity::parse(profile.as_str(), "0", quantity_unit(profile.as_str()))
        .map(|_| ())
        .map_err(|error| CompileError::InvalidQuantity {
            node: id.to_owned(),
            reason: error.to_string(),
        })
}

fn validate_desired_operation(
    id: &str,
    operation: &DesiredPatchOperation,
) -> Result<(), CompileError> {
    use crate::model::DesiredProjectionValue as V;
    let value = match operation {
        DesiredPatchOperation::Test { expected } => Some(expected),
        DesiredPatchOperation::Add { value } | DesiredPatchOperation::Replace { value } => {
            Some(value)
        }
        DesiredPatchOperation::Remove { .. } => None,
    };
    if let Some(value) = value {
        match value {
            V::Level(value)
            | V::ThermostatSingleTarget(value)
            | V::ThermostatHeatTarget(value)
            | V::ThermostatCoolTarget(value)
            | V::CoveringLift(value)
            | V::CoveringTilt(value) => validate_decimal(id, &value.0)?,
            V::Color(value) => validate_color(id, value)?,
            V::ThermostatFan(value) | V::ThermostatSwing(value) | V::ThermostatPreset(value) => {
                validate_choice(id, &value.0)?
            }
            V::On(_) | V::ThermostatMode(_) => {}
        }
    }
    Ok(())
}

fn validate_operation_arguments(
    id: &str,
    operation: &CapabilityOperation,
) -> Result<(), CompileError> {
    use CapabilityOperation as O;
    match operation {
        O::LevelSet { level }
        | O::LevelStep { step: level }
        | O::LevelTransition { level, .. }
        | O::LevelSetCoupled { level, .. }
        | O::LevelStepCoupled { step: level, .. }
        | O::LevelMove {
            rate_per_second: level,
            ..
        }
        | O::LevelMoveCoupled {
            rate_per_second: level,
            ..
        }
        | O::LevelTransitionCoupled { level, .. }
        | O::ThermostatSetSingleTarget {
            target_celsius: level,
        }
        | O::CoveringSetLift { position: level }
        | O::CoveringSetTilt { position: level } => validate_decimal(id, &level.0)?,
        O::ThermostatSetDualTarget {
            heat_target_celsius,
            cool_target_celsius,
        } => {
            validate_decimal(id, &heat_target_celsius.0)?;
            validate_decimal(id, &cool_target_celsius.0)?;
        }
        O::ColorSet { color }
        | O::ColorStep { color, .. }
        | O::ColorMove { color, .. }
        | O::ColorTransition { color, .. } => validate_color(id, color)?,
        O::ThermostatSetFan { fan_choice }
        | O::ThermostatSetSwing {
            swing_choice: fan_choice,
        }
        | O::ThermostatSetPreset {
            preset_choice: fan_choice,
        } => validate_choice(id, &fan_choice.0)?,
        O::NotificationSend {
            notification_template_id,
            expires_in_ms,
            ..
        } => {
            if !valid_management_id(notification_template_id) {
                return invalid_node(id, "notification template ID is invalid");
            }
            if expires_in_ms.is_some_and(|value| !(1..=86_400_000).contains(&value)) {
                return invalid_node(id, "notification expiry must be in 1..=86400000 ms");
            }
        }
        O::OnOffSet { .. }
        | O::OnOffToggle
        | O::LevelStop
        | O::ColorStop
        | O::ThermostatSetMode { .. }
        | O::CoveringOpen { .. }
        | O::CoveringClose { .. }
        | O::CoveringStop { .. }
        | O::MomentaryInvoke => {}
    }
    match operation {
        O::LevelTransition { duration_ms, .. }
        | O::LevelTransitionCoupled { duration_ms, .. }
        | O::ColorTransition { duration_ms, .. }
            if !(1..=30_000).contains(duration_ms) =>
        {
            invalid_node(id, "transition duration must be in 1..=30000 ms")
        }
        _ => Ok(()),
    }
}

fn validate_decimal(id: &str, value: &str) -> Result<(), CompileError> {
    if value.is_empty() || value.len() > 32 || !valid_manifest_decimal(value) {
        invalid_node(
            id,
            "decimal argument does not match the management manifest",
        )
    } else {
        Ok(())
    }
}

fn validate_color(id: &str, color: &crate::model::ColorValue) -> Result<(), CompileError> {
    use crate::model::ColorValue as C;
    match color {
        C::HueSaturation {
            hue_degrees,
            saturation,
        } => {
            validate_decimal(id, &hue_degrees.0)?;
            validate_decimal(id, &saturation.0)
        }
        C::CieXy { x, y } => {
            validate_decimal(id, &x.0)?;
            validate_decimal(id, &y.0)
        }
        C::ColorTemperature { kelvin } => validate_decimal(id, &kelvin.0),
        C::CalibratedRgb {
            red, green, blue, ..
        }
        | C::UncalibratedRgb { red, green, blue } => {
            validate_decimal(id, &red.0)?;
            validate_decimal(id, &green.0)?;
            validate_decimal(id, &blue.0)
        }
        C::NativeWhiteChannels {
            white,
            cold_white,
            warm_white,
        } => {
            validate_decimal(id, &white.0)?;
            if let Some(value) = cold_white {
                validate_decimal(id, &value.0)?;
            }
            if let Some(value) = warm_white {
                validate_decimal(id, &value.0)?;
            }
            Ok(())
        }
    }
}

fn validate_choice(id: &str, value: &str) -> Result<(), CompileError> {
    if value.is_empty() || value.len() > 64 {
        invalid_node(id, "choice must contain 1..=64 bytes")
    } else {
        Ok(())
    }
}

fn validate_receipt_requirement(
    id: &str,
    operation: &CapabilityOperation,
    requirement: ReceiptRequirement,
) -> Result<(), CompileError> {
    let valid = if operation.is_notification() {
        matches!(
            requirement,
            ReceiptRequirement::AcceptedForDelivery
                | ReceiptRequirement::NotificationAdvertisedMaximum
        )
    } else {
        requirement == ReceiptRequirement::AcceptedForDelivery
    };
    if valid {
        Ok(())
    } else {
        Err(CompileError::InvalidReceiptRequirement {
            node: id.to_owned(),
            reason: "non-notification operations may require only accepted-for-delivery".to_owned(),
        })
    }
}

fn validate_retry(
    id: &str,
    operation: &CapabilityOperation,
    deadline_ms: u32,
    retry: &RetryPolicy,
) -> Result<(), CompileError> {
    match retry {
        RetryPolicy::None => Ok(()),
        RetryPolicy::DefinitelyPreDispatch {
            max_attempts,
            backoff_ms,
            ..
        } => {
            if !operation.kind().retry_eligible() {
                return Err(CompileError::InvalidRetryPolicy {
                    node: id.to_owned(),
                    reason: format!("{} is fail-closed never-retry", operation.operation_id()),
                });
            }
            if !(2..=3).contains(max_attempts) || !(1..=30_000).contains(backoff_ms) {
                return Err(CompileError::InvalidRetryPolicy {
                    node: id.to_owned(),
                    reason: "attempts must be 2..=3 and backoff 1..=30000 ms".to_owned(),
                });
            }
            let total_backoff = u32::from(*max_attempts - 1).saturating_mul(*backoff_ms);
            if total_backoff >= deadline_ms {
                return Err(CompileError::InvalidRetryPolicy {
                    node: id.to_owned(),
                    reason: "bounded backoff must fit strictly inside the one absolute deadline"
                        .to_owned(),
                });
            }
            Ok(())
        }
    }
}

fn validate_fork_join(
    id: &str,
    phase: &ForkJoinPhase,
    nodes: &BTreeMap<String, RawNode<'_>>,
    configured_maximum: usize,
) -> Result<(), CompileError> {
    let branches = phase.branches();
    if branches.len() < 2 || branches.len() > configured_maximum {
        return invalid_node(
            id,
            "fork/join branch count is outside the configured 2..=4 bound",
        );
    }
    let mut ids = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for branch in branches {
        if !valid_short_id(&branch.id) || !ids.insert(&branch.id) || !orders.insert(branch.order) {
            return invalid_node(
                id,
                "fork/join branch IDs and orders must be valid and unique",
            );
        }
    }
    if orders.iter().copied().ne(0_u8..branches.len() as u8) {
        return invalid_node(id, "fork/join branch orders must be contiguous from zero");
    }
    let counterpart = match phase {
        ForkJoinPhase::Fork { join_node_id, .. } => nodes.get(join_node_id),
        ForkJoinPhase::Join { fork_node_id, .. } => nodes.get(fork_node_id),
    };
    let Some(RawNode::Node(Node::ForkJoin {
        phase: counterpart_phase,
        ..
    })) = counterpart
    else {
        return invalid_node(id, "fork/join counterpart is missing");
    };
    let reciprocal = match (phase, counterpart_phase) {
        (ForkJoinPhase::Fork { join_node_id, .. }, ForkJoinPhase::Join { fork_node_id, .. }) => {
            join_node_id != id && fork_node_id == id
        }
        (ForkJoinPhase::Join { fork_node_id, .. }, ForkJoinPhase::Fork { join_node_id, .. }) => {
            fork_node_id != id && join_node_id == id
        }
        _ => false,
    };
    if !reciprocal || counterpart_phase.branches() != branches {
        return invalid_node(
            id,
            "fork/join counterpart and branch inventory must be reciprocal",
        );
    }
    Ok(())
}

fn validate_type_ref(type_ref: &TypeRef, depth: usize) -> Result<(), CompileError> {
    if depth > HARD_MAX_JSON_DEPTH {
        return Err(CompileError::BoundExceeded {
            path: "type_ref nesting".to_owned(),
            actual: depth,
            maximum: HARD_MAX_JSON_DEPTH,
        });
    }
    match type_ref {
        TypeRef::Optional { item } => validate_type_ref(item, depth + 1),
        TypeRef::List { item, max_items } => {
            if *max_items == 0 || usize::from(*max_items) > HARD_MAX_COLLECTION_ITEMS {
                return Err(CompileError::InvalidNodeData {
                    node: "type_ref".to_owned(),
                    reason: "list max_items must be in 1..=1024".to_owned(),
                });
            }
            validate_type_ref(item, depth + 1)
        }
        TypeRef::Quantity { catalog, profile } => {
            if *catalog != QuantityCatalogRef::V1 {
                return Err(CompileError::InvalidNodeData {
                    node: "type_ref".to_owned(),
                    reason: "unknown quantity catalog".to_owned(),
                });
            }
            validate_profile("type_ref", *profile)
        }
        _ => Ok(()),
    }
}

fn lower_compatibility_node(node: &Node) -> Result<CompiledNode, CompileError> {
    match node {
        Node::Snapshot {
            id,
            selector,
            field,
            ..
        } => {
            let value_type = field.planner_value_type().ok_or_else(|| {
                CompileError::PlannerSubsetUnsupported {
                    node: id.clone(),
                    kind: format!("snapshot field {}", field.catalog_name()),
                }
            })?;
            Ok(CompiledNode::Snapshot {
                selector: selector.clone(),
                field: field.catalog_name(),
                value_type,
            })
        }
        Node::Condition { id, expression, .. } => lower_condition(id, expression),
        Node::Quantity {
            id,
            input_profile,
            operation,
            ..
        } => lower_quantity(id, *input_profile, operation),
        Node::Branch { .. } => Ok(CompiledNode::Branch),
        Node::EffectIntent {
            id,
            selector,
            operation,
            required_receipt,
            retry,
            ..
        } => {
            if *required_receipt != ReceiptRequirement::AcceptedForDelivery {
                return Err(CompileError::PlannerSubsetUnsupported {
                    node: id.clone(),
                    kind: "receipt requirement above accepted-for-delivery".to_owned(),
                });
            }
            if !matches!(retry, RetryPolicy::None) {
                return Err(CompileError::PlannerSubsetUnsupported {
                    node: id.clone(),
                    kind: "effect retry policy".to_owned(),
                });
            }
            match operation {
                CapabilityOperation::OnOffSet { on } => Ok(CompiledNode::OnOffSet {
                    selector: selector.clone(),
                    value: *on,
                }),
                operation => Err(CompileError::PlannerSubsetUnsupported {
                    node: id.clone(),
                    kind: operation.operation_id().to_owned(),
                }),
            }
        }
        Node::ReceiptBranch { .. } => Ok(CompiledNode::ReceiptBranch),
        Node::End { id, outcome, .. } => match outcome {
            EndOutcome::Success | EndOutcome::Noop | EndOutcome::Unknown | EndOutcome::Error => {
                Ok(CompiledNode::End { outcome: *outcome })
            }
            _ => Err(CompileError::PlannerSubsetUnsupported {
                node: id.clone(),
                kind: format!("end outcome {outcome:?}"),
            }),
        },
        node => Err(CompileError::PlannerSubsetUnsupported {
            node: node.id().to_owned(),
            kind: node.kind_name().to_owned(),
        }),
    }
}

fn lower_condition(id: &str, expression: &CelCondition) -> Result<CompiledNode, CompileError> {
    let actual_digest = format!("{:x}", Sha256::digest(expression.source.as_bytes()));
    if expression.normalized_source_sha256 != actual_digest {
        return Err(CompileError::InvalidCondition {
            node: id.to_owned(),
            reason: "normalized source digest does not match source".to_owned(),
        });
    }
    if expression.inputs.len() != 1 || expression.inputs[0].name != "left" {
        return Err(CompileError::PlannerSubsetUnsupported {
            node: id.to_owned(),
            kind: "condition outside the one-input compatibility subset".to_owned(),
        });
    }
    let input = &expression.inputs[0];
    let value_type = type_ref_to_value_type(&input.type_ref).ok_or_else(|| {
        CompileError::PlannerSubsetUnsupported {
            node: id.to_owned(),
            kind: format!("condition input type {}", input.type_ref.scalar_name()),
        }
    })?;
    if value_type == ValueType::U64 {
        return Err(CompileError::PlannerSubsetUnsupported {
            node: id.to_owned(),
            kind: "u64 CEL comparison requires type-faithful CEL uint token parsing".to_owned(),
        });
    }
    let (operator, literal) =
        parse_comparison(&expression.source, &input.name).ok_or_else(|| {
            CompileError::PlannerSubsetUnsupported {
                node: id.to_owned(),
                kind: "CEL expression outside the one-input comparison compatibility subset"
                    .to_owned(),
            }
        })?;
    validate_operator(operator, &value_type)?;
    let right = compile_cel_literal(id, &value_type, literal)?;
    Ok(CompiledNode::Compare {
        input_port: "left",
        value_type,
        operator,
        right,
    })
}

fn lower_quantity(
    id: &str,
    input_profile: QuantityProfileRef,
    operation: &QuantityOperation,
) -> Result<CompiledNode, CompileError> {
    let value_type = ValueType::Quantity {
        profile: input_profile.as_str().to_owned(),
    };
    match operation {
        QuantityOperation::Compare { operator, right } => {
            validate_operator(*operator, &value_type)?;
            let right = compile_quantity_literal(id, input_profile, right)?;
            Ok(CompiledNode::Compare {
                input_port: "value",
                value_type,
                operator: *operator,
                right,
            })
        }
        operation => Err(CompileError::PlannerSubsetUnsupported {
            node: id.to_owned(),
            kind: format!("quantity operation {operation:?}"),
        }),
    }
}

fn type_ref_to_value_type(type_ref: &TypeRef) -> Option<ValueType> {
    match type_ref {
        TypeRef::Bool => Some(ValueType::Bool),
        TypeRef::String => Some(ValueType::String),
        TypeRef::I64 => Some(ValueType::I64),
        TypeRef::U64 => Some(ValueType::U64),
        TypeRef::Quantity { profile, .. } => Some(ValueType::Quantity {
            profile: profile.as_str().to_owned(),
        }),
        TypeRef::Optional { item } => type_ref_to_value_type(item),
        _ => None,
    }
}

fn parse_comparison<'a>(source: &'a str, input_name: &str) -> Option<(CompareOperator, &'a str)> {
    let source = source.trim();
    let rest = source.strip_prefix(input_name)?;
    if rest
        .as_bytes()
        .first()
        .is_some_and(|byte| !byte.is_ascii_whitespace())
    {
        return None;
    }
    let rest = rest.trim_start();
    [
        ("==", CompareOperator::Equal),
        ("!=", CompareOperator::NotEqual),
        ("<=", CompareOperator::LessOrEqual),
        (">=", CompareOperator::GreaterOrEqual),
        ("<", CompareOperator::Less),
        (">", CompareOperator::Greater),
    ]
    .into_iter()
    .find_map(|(token, operator)| {
        let literal = rest.strip_prefix(token)?.trim();
        (!literal.is_empty()).then_some((operator, literal))
    })
}

fn compile_cel_literal(
    node: &str,
    value_type: &ValueType,
    literal: &str,
) -> Result<CompiledScalar, CompileError> {
    let invalid = |reason: &str| CompileError::InvalidCondition {
        node: node.to_owned(),
        reason: reason.to_owned(),
    };
    match value_type {
        ValueType::Bool => match literal {
            "true" => Ok(CompiledScalar::Bool(true)),
            "false" => Ok(CompiledScalar::Bool(false)),
            _ => Err(invalid("Boolean literal must be true or false")),
        },
        ValueType::String => serde_json::from_str::<String>(literal)
            .map(CompiledScalar::String)
            .map_err(|_| invalid("string literal must be a JSON string")),
        ValueType::I64 => literal
            .parse::<i64>()
            .map(CompiledScalar::I64)
            .map_err(|_| invalid("integer literal is outside i64 or not canonical")),
        ValueType::U64 => literal
            .parse::<u64>()
            .map(CompiledScalar::U64)
            .map_err(|_| invalid("integer literal is outside u64 or not canonical")),
        ValueType::Quantity { .. } => Err(CompileError::PlannerSubsetUnsupported {
            node: node.to_owned(),
            kind: "quantity inside CEL".to_owned(),
        }),
    }
}

fn compile_quantity_literal(
    node: &str,
    profile: QuantityProfileRef,
    value: &crate::model::QuantityWire,
) -> Result<CompiledScalar, CompileError> {
    if value.profile != profile || !valid_decimal_shape(&value.magnitude) {
        return Err(CompileError::ComparisonTypeMismatch {
            node: node.to_owned(),
            left: profile.as_str().to_owned(),
            right: value.profile.as_str().to_owned(),
        });
    }
    let quantity =
        Quantity::parse(profile.as_str(), &value.magnitude, &value.unit).map_err(|error| {
            CompileError::InvalidQuantity {
                node: node.to_owned(),
                reason: error.to_string(),
            }
        })?;
    if quantity.profile_id().as_str() != profile.as_str()
        || quantity.magnitude().to_string() != value.magnitude
        || quantity.unit() != value.unit
    {
        return Err(CompileError::InvalidQuantity {
            node: node.to_owned(),
            reason: "quantity literal must already be canonical".to_owned(),
        });
    }
    Ok(CompiledScalar::Quantity(quantity))
}

fn validate_operator(
    operator: CompareOperator,
    value_type: &ValueType,
) -> Result<(), CompileError> {
    let equality = matches!(operator, CompareOperator::Equal | CompareOperator::NotEqual);
    if equality
        || matches!(
            value_type,
            ValueType::I64 | ValueType::U64 | ValueType::Quantity { .. }
        )
    {
        Ok(())
    } else {
        Err(CompileError::UnsupportedComparison {
            operator,
            value_type: value_type.name().to_owned(),
        })
    }
}

struct Graph {
    outgoing: BTreeMap<(String, String), String>,
    data_sources: BTreeMap<(String, String), (String, String)>,
}

fn validate_edges(
    nodes: &BTreeMap<String, RawNode<'_>>,
    edges: &[crate::model::Edge],
    trigger_id: &str,
) -> Result<Graph, CompileError> {
    let mut edge_ids = BTreeSet::new();
    let mut order_keys = BTreeSet::new();
    let mut counts: BTreeMap<(String, String, Direction), usize> = BTreeMap::new();
    let mut outgoing = BTreeMap::new();
    let mut data_sources = BTreeMap::new();

    for edge in edges {
        validate_node_id("edge", &edge.id)?;
        if edge.order > 255 {
            return Err(CompileError::InvalidNodeData {
                node: edge.id.clone(),
                reason: "edge order must be at most 255".to_owned(),
            });
        }
        if !edge_ids.insert(edge.id.clone()) {
            return Err(CompileError::DuplicateEdgeId {
                id: edge.id.clone(),
            });
        }
        let from_node = nodes
            .get(&edge.from.node)
            .ok_or_else(|| CompileError::DanglingNode {
                edge: edge.id.clone(),
                node: edge.from.node.clone(),
            })?;
        let to_node = nodes
            .get(&edge.to.node)
            .ok_or_else(|| CompileError::DanglingNode {
                edge: edge.id.clone(),
                node: edge.to.node.clone(),
            })?;
        let from = port(from_node, nodes, &edge.from.port)
            .filter(|port| port.direction == Direction::Output)
            .ok_or_else(|| CompileError::UnknownPort {
                edge: edge.id.clone(),
                direction: "output",
                node: edge.from.node.clone(),
                port: edge.from.port.clone(),
            })?;
        let to = port(to_node, nodes, &edge.to.port)
            .filter(|port| port.direction == Direction::Input)
            .ok_or_else(|| CompileError::UnknownPort {
                edge: edge.id.clone(),
                direction: "input",
                node: edge.to.node.clone(),
                port: edge.to.port.clone(),
            })?;
        if from.type_ref != to.type_ref {
            return Err(CompileError::IncompatiblePorts {
                edge: edge.id.clone(),
                from: type_ref_name(&from.type_ref),
                to: type_ref_name(&to.type_ref),
            });
        }
        if let RawNode::Node(Node::ReceiptBranch {
            effect_node_id, id, ..
        }) = to_node
            && edge.to.port == "receipt"
            && edge.from.node != *effect_node_id
        {
            return Err(CompileError::ReceiptEffectMismatch {
                node: id.clone(),
                effect: effect_node_id.clone(),
            });
        }
        let order_key = (edge.from.node.clone(), edge.from.port.clone(), edge.order);
        if !order_keys.insert(order_key) {
            return Err(CompileError::DuplicateEdgeOrder {
                node: edge.from.node.clone(),
                port: edge.from.port.clone(),
                order: edge.order,
            });
        }
        *counts
            .entry((
                edge.from.node.clone(),
                edge.from.port.clone(),
                Direction::Output,
            ))
            .or_default() += 1;
        *counts
            .entry((edge.to.node.clone(), edge.to.port.clone(), Direction::Input))
            .or_default() += 1;
        if from.type_ref == TypeRef::Control {
            outgoing.insert(
                (edge.from.node.clone(), edge.from.port.clone()),
                edge.to.node.clone(),
            );
        } else {
            data_sources.insert(
                (edge.to.node.clone(), edge.to.port.clone()),
                (edge.from.node.clone(), edge.from.port.clone()),
            );
        }
    }

    for (node, port, direction) in counts.keys() {
        if *direction == Direction::Output {
            let mut orders = edges
                .iter()
                .filter(|edge| edge.from.node == *node && edge.from.port == *port)
                .map(|edge| edge.order)
                .collect::<Vec<_>>();
            orders.sort_unstable();
            if orders.iter().copied().ne(0_u16..orders.len() as u16) {
                return Err(CompileError::NonContiguousEdgeOrder {
                    node: node.clone(),
                    port: port.clone(),
                });
            }
        }
    }

    for (node_id, node) in nodes {
        for (name, definition) in ports(node, nodes) {
            let count = counts
                .get(&(node_id.clone(), name.clone(), definition.direction))
                .copied()
                .unwrap_or(0);
            match definition.direction {
                Direction::Input if node_id == trigger_id && count != 0 => {
                    return Err(CompileError::TriggerHasIncomingEdge);
                }
                Direction::Input
                    if count < definition.minimum
                        || definition.maximum.is_some_and(|maximum| count > maximum) =>
                {
                    let expected = if definition.maximum == Some(1) {
                        "exactly one edge"
                    } else {
                        "one or more edges"
                    };
                    return Err(CompileError::InputCardinality {
                        node: node_id.clone(),
                        port: name,
                        expected,
                        actual: count,
                    });
                }
                Direction::Output if definition.maximum.is_some_and(|maximum| count > maximum) => {
                    return Err(CompileError::OutputCardinality {
                        node: node_id.clone(),
                        port: name,
                        actual: count,
                    });
                }
                _ => {}
            }
        }
    }

    Ok(Graph {
        outgoing,
        data_sources,
    })
}

fn topological_order(
    nodes: &BTreeMap<String, RawNode<'_>>,
    edges: &[crate::model::Edge],
) -> Result<Vec<String>, CompileError> {
    let mut indegree = nodes
        .keys()
        .map(|id| (id.clone(), 0))
        .collect::<BTreeMap<_, _>>();
    let mut successors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for edge in edges {
        *indegree
            .get_mut(&edge.to.node)
            .expect("validated edge node") += 1;
        successors
            .entry(edge.from.node.clone())
            .or_default()
            .push(edge.to.node.clone());
    }
    for values in successors.values_mut() {
        values.sort();
    }
    let mut ready = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(nodes.len());
    while let Some(id) = ready.pop_first() {
        order.push(id.clone());
        if let Some(next) = successors.get(&id) {
            for successor in next {
                let degree = indegree.get_mut(successor).expect("known successor");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(successor.clone());
                }
            }
        }
    }
    if order.len() != nodes.len() {
        return Err(CompileError::Cycle);
    }
    Ok(order)
}

fn validate_reachability(
    nodes: &BTreeMap<String, RawNode<'_>>,
    edges: &[crate::model::Edge],
    trigger_id: &str,
) -> Result<(), CompileError> {
    let mut successors: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in edges {
        successors
            .entry(&edge.from.node)
            .or_default()
            .push(&edge.to.node);
    }
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([trigger_id]);
    while let Some(node) = queue.pop_front() {
        if !seen.insert(node) {
            continue;
        }
        if let Some(next) = successors.get(node) {
            queue.extend(next.iter().copied());
        }
    }
    for node in nodes.keys() {
        if !seen.contains(node.as_str()) {
            return Err(CompileError::UnreachableNode { node: node.clone() });
        }
    }
    Ok(())
}

fn graph_depth(order: &[String], edges: &[crate::model::Edge]) -> usize {
    let mut predecessors: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for edge in edges {
        predecessors
            .entry(&edge.to.node)
            .or_default()
            .push(&edge.from.node);
    }
    let mut depths: BTreeMap<&str, usize> = BTreeMap::new();
    for node in order {
        let depth = predecessors.get(node.as_str()).map_or(1, |values| {
            1 + values
                .iter()
                .map(|predecessor| depths[predecessor])
                .max()
                .unwrap_or(0)
        });
        depths.insert(node, depth);
    }
    depths.values().copied().max().unwrap_or(0)
}

#[derive(Clone, Copy)]
enum RawNode<'a> {
    Trigger(&'a TriggerNode),
    Node(&'a Node),
}

fn ports(node: &RawNode<'_>, nodes: &BTreeMap<String, RawNode<'_>>) -> Vec<(String, Port)> {
    let model_nodes = nodes
        .values()
        .filter_map(|node| match node {
            RawNode::Node(node) => Some((*node).clone()),
            RawNode::Trigger(_) => None,
        })
        .collect::<Vec<_>>();
    let resolved = match node {
        RawNode::Trigger(trigger) => resolved_trigger_ports(trigger),
        RawNode::Node(node) => resolved_node_ports(node, &model_nodes),
    };
    resolved.into_iter().map(compiler_port).collect()
}

fn compiler_port(port: ResolvedPort) -> (String, Port) {
    let direction = match port.direction {
        CatalogDirection::Input => Direction::Input,
        CatalogDirection::Output => Direction::Output,
    };
    let (minimum, maximum) = match port.cardinality {
        CatalogCardinality::ZeroOrOne => (0, Some(1)),
        CatalogCardinality::ExactlyOne => (1, Some(1)),
        CatalogCardinality::OneOrMore => (1, None),
    };
    (
        port.name,
        Port {
            direction,
            type_ref: port.type_ref,
            minimum,
            maximum,
        },
    )
}

#[allow(dead_code)]
fn legacy_ports(node: &RawNode<'_>, nodes: &BTreeMap<String, RawNode<'_>>) -> Vec<(String, Port)> {
    match node {
        RawNode::Trigger(trigger) => vec![
            ("next".to_owned(), optional_output(TypeRef::Control)),
            (
                "context".to_owned(),
                optional_output(TypeRef::TriggerContext {
                    trigger: trigger_kind(trigger),
                }),
            ),
        ],
        RawNode::Node(Node::Snapshot { field, .. }) => vec![
            ("in".to_owned(), control_input()),
            ("next".to_owned(), optional_output(TypeRef::Control)),
            ("value".to_owned(), optional_output(field.type_ref())),
        ],
        RawNode::Node(Node::Condition { expression, .. }) => {
            let mut result = vec![
                ("in".to_owned(), control_input()),
                ("next".to_owned(), optional_output(TypeRef::Control)),
                (
                    "result".to_owned(),
                    optional_output(TypeRef::PredicateOutcome),
                ),
            ];
            result.extend(
                expression
                    .inputs
                    .iter()
                    .map(|input| (input.name.clone(), data_input(input.type_ref.clone()))),
            );
            result
        }
        RawNode::Node(Node::Quantity {
            input_profile,
            operation,
            ..
        }) => {
            let output = match operation {
                QuantityOperation::Compare { .. } => TypeRef::PredicateOutcome,
                QuantityOperation::Multiply { result_profile, .. }
                | QuantityOperation::Divide { result_profile, .. } => TypeRef::Quantity {
                    catalog: QuantityCatalogRef::V1,
                    profile: *result_profile,
                },
                _ => TypeRef::Quantity {
                    catalog: QuantityCatalogRef::V1,
                    profile: *input_profile,
                },
            };
            vec![
                ("in".to_owned(), control_input()),
                (
                    "value".to_owned(),
                    data_input(TypeRef::Optional {
                        item: Box::new(TypeRef::Quantity {
                            catalog: QuantityCatalogRef::V1,
                            profile: *input_profile,
                        }),
                    }),
                ),
                ("next".to_owned(), optional_output(TypeRef::Control)),
                ("result".to_owned(), optional_output(output)),
            ]
        }
        RawNode::Node(Node::Branch { .. }) => vec![
            ("in".to_owned(), control_input()),
            (
                "predicate".to_owned(),
                data_input(TypeRef::PredicateOutcome),
            ),
            ("true".to_owned(), optional_output(TypeRef::Control)),
            ("false".to_owned(), optional_output(TypeRef::Control)),
            ("unknown".to_owned(), optional_output(TypeRef::Control)),
            ("error".to_owned(), optional_output(TypeRef::Control)),
        ],
        RawNode::Node(Node::DesiredPatch { .. }) => vec![
            ("in".to_owned(), control_input()),
            ("next".to_owned(), optional_output(TypeRef::Control)),
            ("patch".to_owned(), optional_output(TypeRef::DesiredPatch)),
        ],
        RawNode::Node(Node::EffectIntent { operation, .. }) => vec![
            ("in".to_owned(), control_input()),
            ("intent".to_owned(), optional_output(TypeRef::EffectIntent)),
            (
                "receipt".to_owned(),
                optional_output(TypeRef::Receipt {
                    operation: operation.kind(),
                }),
            ),
        ],
        RawNode::Node(Node::Delay { .. }) => vec![
            ("in".to_owned(), control_input()),
            ("next".to_owned(), optional_output(TypeRef::Control)),
        ],
        RawNode::Node(Node::DebounceThrottle { .. }) => vec![
            ("in".to_owned(), control_input()),
            ("admitted".to_owned(), optional_output(TypeRef::Control)),
            ("dropped".to_owned(), optional_output(TypeRef::Control)),
        ],
        RawNode::Node(Node::ForkJoin { phase, .. }) => match phase {
            ForkJoinPhase::Fork { branches, .. } => {
                let mut result = vec![("in".to_owned(), control_input())];
                result.extend(branches.iter().map(|branch| {
                    (
                        format!("branch:{}", branch.id),
                        optional_output(TypeRef::Control),
                    )
                }));
                result
            }
            ForkJoinPhase::Join { branches, .. } => {
                let mut result = vec![("next".to_owned(), optional_output(TypeRef::Control))];
                result.extend(branches.iter().map(|branch| {
                    (
                        format!("arrival:{}", branch.id),
                        data_input(TypeRef::Control),
                    )
                }));
                result
            }
        },
        RawNode::Node(Node::ReceiptBranch { effect_node_id, .. }) => {
            let receipt = match nodes.get(effect_node_id) {
                Some(RawNode::Node(Node::EffectIntent { operation, .. })) => TypeRef::Receipt {
                    operation: operation.kind(),
                },
                _ => TypeRef::Receipt {
                    operation: crate::model::CapabilityOperationKind::OnOffSet,
                },
            };
            vec![
                ("receipt".to_owned(), data_input(receipt)),
                (
                    "accepted_for_delivery".to_owned(),
                    optional_output(TypeRef::Control),
                ),
                (
                    "protocol_or_target_acknowledged".to_owned(),
                    optional_output(TypeRef::Control),
                ),
                ("completed".to_owned(), optional_output(TypeRef::Control)),
                (
                    "observed_completed".to_owned(),
                    optional_output(TypeRef::Control),
                ),
                ("rejected".to_owned(), optional_output(TypeRef::Control)),
                ("error".to_owned(), optional_output(TypeRef::Control)),
                ("cancelled".to_owned(), optional_output(TypeRef::Control)),
                (
                    "outcome_unknown".to_owned(),
                    optional_output(TypeRef::Control),
                ),
            ]
        }
        RawNode::Node(Node::End { .. }) => vec![("in".to_owned(), control_input())],
    }
}

fn port(node: &RawNode<'_>, nodes: &BTreeMap<String, RawNode<'_>>, name: &str) -> Option<Port> {
    ports(node, nodes)
        .into_iter()
        .find_map(|(candidate, port)| (candidate == name).then_some(port))
}

fn optional_output(type_ref: TypeRef) -> Port {
    Port {
        direction: Direction::Output,
        type_ref,
        minimum: 0,
        maximum: Some(1),
    }
}

fn control_input() -> Port {
    Port {
        direction: Direction::Input,
        type_ref: TypeRef::Control,
        minimum: 1,
        maximum: None,
    }
}

fn data_input(type_ref: TypeRef) -> Port {
    Port {
        direction: Direction::Input,
        type_ref,
        minimum: 1,
        maximum: Some(1),
    }
}

fn type_ref_name(type_ref: &TypeRef) -> String {
    serde_json::to_string(type_ref).unwrap_or_else(|_| type_ref.scalar_name().to_owned())
}

fn trigger_kind(trigger: &TriggerNode) -> TriggerContextKind {
    match trigger {
        TriggerNode::StateObservation { .. } => TriggerContextKind::StateObservation,
        TriggerNode::DiscreteEvent { .. } => TriggerContextKind::DiscreteEvent,
        TriggerNode::TimeSchedule { .. } => TriggerContextKind::TimeSchedule,
    }
}

fn validate_selector(owner: &str, selector: &Selector) -> Result<(), CompileError> {
    let reason = if !valid_hex_32(&selector.provider_instance_id) {
        Some("provider_instance_id must be 32 lowercase hexadecimal characters")
    } else if !valid_id(&selector.facet_key) {
        Some("facet_key is not a stable ID")
    } else if !valid_contract_part(&selector.contract.namespace)
        || !valid_contract_part(&selector.contract.package)
        || !valid_contract_part(&selector.contract.interface)
    {
        Some("contract namespace/package/interface is invalid")
    } else if selector.contract.major == 0 {
        Some("contract major must be non-zero")
    } else if selector.cardinality != SelectorCardinality::One {
        Some("V1 executable selectors require cardinality one")
    } else if !matches!(
        selector.rebind,
        RebindPolicy::Never | RebindPolicy::SameIdentity
    ) {
        Some("invalid rebind policy")
    } else {
        None
    };
    reason.map_or(Ok(()), |reason| {
        Err(CompileError::InvalidSelector {
            owner: owner.to_owned(),
            reason: reason.to_owned(),
        })
    })
}

fn is_stateful_contract(contract: &ContractRef) -> bool {
    contract.namespace == "litehouse"
        && contract.interface == "capability"
        && contract.major == 1
        && matches!(
            contract.package.as_str(),
            "on-off"
                | "boolean-state"
                | "level-control"
                | "color-control"
                | "measurement"
                | "thermostat"
                | "window-covering"
        )
}

fn validate_version(node: &str, version: u8) -> Result<(), CompileError> {
    if version == 1 {
        Ok(())
    } else {
        Err(CompileError::UnsupportedNodeVersion {
            node: node.to_owned(),
            version,
        })
    }
}

fn validate_node_id(kind: &'static str, id: &str) -> Result<(), CompileError> {
    if valid_id(id) {
        Ok(())
    } else {
        Err(CompileError::InvalidId {
            kind,
            id: id.to_owned(),
        })
    }
}

fn bound(path: &str, actual: usize, maximum: usize) -> Result<(), CompileError> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(CompileError::BoundExceeded {
            path: path.to_owned(),
            actual,
            maximum,
        })
    }
}

fn limit(
    name: &'static str,
    actual: usize,
    minimum: usize,
    maximum: usize,
) -> Result<(), CompileError> {
    if (minimum..=maximum).contains(&actual) {
        Ok(())
    } else {
        Err(CompileError::InvalidLimit {
            name,
            actual,
            minimum,
            maximum,
        })
    }
}

fn configured_count(name: &'static str, actual: usize, maximum: usize) -> Result<(), CompileError> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(CompileError::InvalidLimit {
            name,
            actual,
            minimum: 0,
            maximum,
        })
    }
}

fn invalid_node<T>(node: &str, reason: &str) -> Result<T, CompileError> {
    Err(CompileError::InvalidNodeData {
        node: node.to_owned(),
        reason: reason.to_owned(),
    })
}

fn quantity_unit(profile: &str) -> &str {
    litehouse_contracts::quantity::lookup_profile(profile)
        .map_or("1", |profile| profile.canonical_unit)
}

fn valid_annotation_key(value: &str) -> bool {
    let Some((namespace, name)) = value.split_once(':') else {
        return false;
    };
    valid_bounded_token(namespace, 32, false) && valid_bounded_token(name, 63, true)
}

fn valid_bounded_token(value: &str, maximum: usize, allow_dot: bool) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b'-'
                || (allow_dot && matches!(byte, b'_' | b'.'))
        })
}

fn valid_condition_input_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_short_id(value: &str) -> bool {
    value.len() <= 32 && valid_id(value)
}

fn valid_management_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn valid_manifest_decimal(value: &str) -> bool {
    if value.is_empty() || value.len() > 32 {
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
    fraction.is_none_or(|fraction| {
        !fraction.is_empty() && fraction.bytes().all(|byte| byte.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{fixtures::fixture_values, schema::artifact_digests};

    #[test]
    fn resolved_catalog_ports_match_the_independent_compiler_inventory() {
        for fixture in fixture_values(&artifact_digests())
            .into_iter()
            .filter(|fixture| fixture.semantic_valid)
        {
            let document: RuleDocument = serde_json::from_value(fixture.value).unwrap();
            let trigger_id = document.semantic.trigger.id().to_owned();
            let mut raw_nodes = BTreeMap::new();
            raw_nodes.insert(trigger_id, RawNode::Trigger(&document.semantic.trigger));
            for node in &document.semantic.nodes {
                raw_nodes.insert(node.id().to_owned(), RawNode::Node(node));
            }
            for (id, node) in &raw_nodes {
                assert_eq!(
                    normalized_ports(ports(node, &raw_nodes)),
                    normalized_ports(legacy_ports(node, &raw_nodes)),
                    "{} {id}",
                    fixture.path,
                );
            }
        }
    }

    fn normalized_ports(
        mut ports: Vec<(String, Port)>,
    ) -> Vec<(String, Direction, String, usize, Option<usize>)> {
        ports.sort_by(|left, right| left.0.cmp(&right.0));
        ports
            .into_iter()
            .map(|(name, port)| {
                (
                    name,
                    port.direction,
                    serde_json::to_string(&port.type_ref).unwrap(),
                    port.minimum,
                    port.maximum,
                )
            })
            .collect()
    }
}
