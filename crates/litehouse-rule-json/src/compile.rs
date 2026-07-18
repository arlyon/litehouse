use std::collections::{BTreeMap, BTreeSet, VecDeque};

use litehouse_contracts::quantity::{CATALOG_ID, Quantity};
use serde_json::Value;
use thiserror::Error;

use crate::{
    model::{
        COMPILER_PROFILE, CompareOperator, ContractRef, Edge, EndOutcome,
        HARD_MAX_COLLECTION_ITEMS, HARD_MAX_EDGES, HARD_MAX_GRAPH_DEPTH, HARD_MAX_JSON_BYTES,
        HARD_MAX_JSON_DEPTH, HARD_MAX_NODES, HARD_MAX_STRING_BYTES, Node, RuleDocument, SCHEMA_URL,
        SCHEMA_VERSION, ScalarLiteral, Selector, ValueType, valid_contract_part,
        valid_decimal_shape, valid_field, valid_hex_32, valid_hex_64, valid_id,
    },
    schema::schema_sha256,
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
    #[error("unsupported schema URL {found:?}")]
    UnsupportedSchemaUrl { found: String },
    #[error("unsupported schema version {found:?}")]
    UnsupportedSchemaVersion { found: String },
    #[error("schema digest does not match the shipped schema")]
    SchemaDigestMismatch,
    #[error("profile_sha256 must be 64 lowercase hexadecimal characters")]
    InvalidProfileDigest,
    #[error("rule ID must be 32 lowercase hexadecimal characters")]
    InvalidRuleId,
    #[error("rule revision must be non-zero")]
    InvalidRevision,
    #[error("unsupported compiler profile {found:?}")]
    UnsupportedCompilerProfile { found: String },
    #[error("unsupported quantity catalog {found:?}")]
    UnsupportedQuantityCatalog { found: String },
    #[error("rule limit {name}={actual} is outside 1..={maximum}")]
    InvalidLimit {
        name: &'static str,
        actual: usize,
        maximum: usize,
    },
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
    #[error("effect {node:?} must target litehouse:on-off/capability@1")]
    InvalidOnOffContract { node: String },
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
    #[error("output {node:?}.{port} requires exactly one edge; found {actual}")]
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
    #[error("rule name or description exceeds its bound")]
    InvalidTextBound,
    #[error("snapshot field on {node:?} is invalid")]
    InvalidSnapshotField { node: String },
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
        value_type: ValueType,
        operator: CompareOperator,
        right: CompiledScalar,
    },
    Branch,
    OnOffSet {
        selector: Selector,
        value: bool,
    },
    End {
        outcome: EndOutcome,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CompiledScalar {
    Bool(bool),
    String(String),
    I64(i64),
    Quantity(Quantity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PortType {
    Control,
    ComparisonOutcome,
    Value(ValueType),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Direction {
    Input,
    Output,
}

#[derive(Debug, Clone)]
struct Port {
    direction: Direction,
    port_type: PortType,
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

pub fn compile(document: RuleDocument) -> Result<CompiledRule, CompileError> {
    validate_header(&document)?;

    let node_limit = usize::from(document.semantic.limits.max_nodes);
    let edge_limit = usize::from(document.semantic.limits.max_edges);
    let depth_limit = usize::from(document.semantic.limits.max_depth);
    let node_count = document.semantic.nodes.len() + 1;
    if node_count > node_limit {
        return Err(CompileError::TooManyNodes {
            actual: node_count,
            maximum: node_limit,
        });
    }
    if document.semantic.edges.len() > edge_limit {
        return Err(CompileError::TooManyEdges {
            actual: document.semantic.edges.len(),
            maximum: edge_limit,
        });
    }

    let trigger_id = document.semantic.trigger.id().to_owned();
    validate_node_id("trigger", &trigger_id)?;
    if document.semantic.trigger.version() != 1 {
        return Err(CompileError::UnsupportedNodeVersion {
            node: trigger_id,
            version: document.semantic.trigger.version(),
        });
    }
    validate_selector("trigger", document.semantic.trigger.selector())?;

    let mut raw_nodes = BTreeMap::new();
    raw_nodes.insert(trigger_id.clone(), RawNode::Trigger);
    for node in &document.semantic.nodes {
        validate_node_id("node", node.id())?;
        if raw_nodes
            .insert(node.id().to_owned(), RawNode::Node(node))
            .is_some()
        {
            return Err(CompileError::DuplicateNodeId {
                id: node.id().to_owned(),
            });
        }
        if node.version() != 1 {
            return Err(CompileError::UnsupportedNodeVersion {
                node: node.id().to_owned(),
                version: node.version(),
            });
        }
    }

    let mut nodes = BTreeMap::new();
    for node in &document.semantic.nodes {
        let compiled = compile_node(node)?;
        nodes.insert(node.id().to_owned(), compiled);
    }

    let graph = validate_edges(&raw_nodes, &document.semantic.edges, &trigger_id)?;
    let topological_order = topological_order(&raw_nodes, &document.semantic.edges)?;
    validate_reachability(&raw_nodes, &document.semantic.edges, &trigger_id)?;
    let depth = graph_depth(&topological_order, &document.semantic.edges);
    if depth > depth_limit {
        return Err(CompileError::GraphTooDeep {
            actual: depth,
            maximum: depth_limit,
        });
    }

    Ok(CompiledRule {
        rule_id: document.rule_id,
        revision: document.revision,
        profile_sha256: document.schema.profile_sha256,
        trigger_id,
        trigger_selector: document.semantic.trigger.selector().clone(),
        nodes,
        outgoing: graph.outgoing,
        data_sources: graph.data_sources,
        topological_order,
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

fn validate_header(document: &RuleDocument) -> Result<(), CompileError> {
    if document.schema.url != SCHEMA_URL {
        return Err(CompileError::UnsupportedSchemaUrl {
            found: document.schema.url.clone(),
        });
    }
    if document.schema.version != SCHEMA_VERSION {
        return Err(CompileError::UnsupportedSchemaVersion {
            found: document.schema.version.clone(),
        });
    }
    if document.schema.sha256 != schema_sha256() {
        return Err(CompileError::SchemaDigestMismatch);
    }
    if !valid_hex_64(&document.schema.profile_sha256) {
        return Err(CompileError::InvalidProfileDigest);
    }
    if !valid_hex_32(&document.rule_id) {
        return Err(CompileError::InvalidRuleId);
    }
    if document.revision == 0 {
        return Err(CompileError::InvalidRevision);
    }
    if document.semantic.name.is_empty()
        || document.semantic.name.len() > 128
        || document.semantic.description.len() > 4096
    {
        return Err(CompileError::InvalidTextBound);
    }
    if document.semantic.locks.compiler != COMPILER_PROFILE {
        return Err(CompileError::UnsupportedCompilerProfile {
            found: document.semantic.locks.compiler.clone(),
        });
    }
    if document.semantic.locks.quantity_catalog != CATALOG_ID {
        return Err(CompileError::UnsupportedQuantityCatalog {
            found: document.semantic.locks.quantity_catalog.clone(),
        });
    }
    validate_limit(
        "max_nodes",
        usize::from(document.semantic.limits.max_nodes),
        HARD_MAX_NODES,
    )?;
    validate_limit(
        "max_edges",
        usize::from(document.semantic.limits.max_edges),
        HARD_MAX_EDGES,
    )?;
    validate_limit(
        "max_depth",
        usize::from(document.semantic.limits.max_depth),
        HARD_MAX_GRAPH_DEPTH,
    )?;
    Ok(())
}

fn validate_limit(name: &'static str, actual: usize, maximum: usize) -> Result<(), CompileError> {
    if actual == 0 || actual > maximum {
        return Err(CompileError::InvalidLimit {
            name,
            actual,
            maximum,
        });
    }
    Ok(())
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

fn compile_node(node: &Node) -> Result<CompiledNode, CompileError> {
    match node {
        Node::Snapshot {
            id,
            selector,
            field,
            value_type,
            ..
        } => {
            validate_selector(id, selector)?;
            if !valid_field(field) {
                return Err(CompileError::InvalidSnapshotField { node: id.clone() });
            }
            validate_value_type(id, value_type)?;
            Ok(CompiledNode::Snapshot {
                selector: selector.clone(),
                field: field.clone(),
                value_type: value_type.clone(),
            })
        }
        Node::Compare {
            id,
            value_type,
            expression,
            ..
        } => {
            validate_value_type(id, value_type)?;
            let right = compile_literal(id, value_type, &expression.right)?;
            validate_operator(expression.operator, value_type)?;
            Ok(CompiledNode::Compare {
                value_type: value_type.clone(),
                operator: expression.operator,
                right,
            })
        }
        Node::Branch { .. } => Ok(CompiledNode::Branch),
        Node::OnOffSet {
            id,
            selector,
            value,
            ..
        } => {
            validate_selector(id, selector)?;
            if !selector.contract.is_on_off_v1() {
                return Err(CompileError::InvalidOnOffContract { node: id.clone() });
            }
            Ok(CompiledNode::OnOffSet {
                selector: selector.clone(),
                value: *value,
            })
        }
        Node::End { outcome, .. } => Ok(CompiledNode::End { outcome: *outcome }),
    }
}

fn validate_value_type(node: &str, value_type: &ValueType) -> Result<(), CompileError> {
    if let ValueType::Quantity { profile } = value_type {
        Quantity::parse(profile, "0", quantity_unit(profile)).map_err(|error| {
            CompileError::InvalidQuantity {
                node: node.to_owned(),
                reason: error.to_string(),
            }
        })?;
    }
    Ok(())
}

fn quantity_unit(profile: &str) -> &str {
    litehouse_contracts::quantity::lookup_profile(profile)
        .map_or("1", |profile| profile.canonical_unit)
}

fn compile_literal(
    node: &str,
    value_type: &ValueType,
    literal: &ScalarLiteral,
) -> Result<CompiledScalar, CompileError> {
    let mismatch = || CompileError::ComparisonTypeMismatch {
        node: node.to_owned(),
        left: value_type.name().to_owned(),
        right: literal.type_name().to_owned(),
    };
    match (value_type, literal) {
        (ValueType::Bool, ScalarLiteral::Bool { value }) => Ok(CompiledScalar::Bool(*value)),
        (ValueType::String, ScalarLiteral::String { value }) => {
            Ok(CompiledScalar::String(value.clone()))
        }
        (ValueType::I64, ScalarLiteral::I64 { value }) => Ok(CompiledScalar::I64(*value)),
        (ValueType::Quantity { profile }, ScalarLiteral::Quantity { value }) => {
            if &value.profile != profile || !valid_decimal_shape(&value.magnitude) {
                return Err(mismatch());
            }
            let quantity = Quantity::parse(&value.profile, &value.magnitude, &value.unit).map_err(
                |error| CompileError::InvalidQuantity {
                    node: node.to_owned(),
                    reason: error.to_string(),
                },
            )?;
            if quantity.profile_id().as_str() != value.profile
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
        _ => Err(mismatch()),
    }
}

fn validate_operator(
    operator: CompareOperator,
    value_type: &ValueType,
) -> Result<(), CompileError> {
    let equality = matches!(operator, CompareOperator::Equal | CompareOperator::NotEqual);
    if equality || matches!(value_type, ValueType::I64 | ValueType::Quantity { .. }) {
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
    edges: &[Edge],
    trigger_id: &str,
) -> Result<Graph, CompileError> {
    let mut edge_ids = BTreeSet::new();
    let mut order_keys = BTreeSet::new();
    let mut counts: BTreeMap<(String, String, Direction), usize> = BTreeMap::new();
    let mut outgoing = BTreeMap::new();
    let mut data_sources = BTreeMap::new();

    for edge in edges {
        validate_node_id("edge", &edge.id)?;
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
        let from = port(from_node, &edge.from.port)
            .filter(|port| port.direction == Direction::Output)
            .ok_or_else(|| CompileError::UnknownPort {
                edge: edge.id.clone(),
                direction: "output",
                node: edge.from.node.clone(),
                port: edge.from.port.clone(),
            })?;
        let to = port(to_node, &edge.to.port)
            .filter(|port| port.direction == Direction::Input)
            .ok_or_else(|| CompileError::UnknownPort {
                edge: edge.id.clone(),
                direction: "input",
                node: edge.to.node.clone(),
                port: edge.to.port.clone(),
            })?;
        if from.port_type != to.port_type {
            return Err(CompileError::IncompatiblePorts {
                edge: edge.id.clone(),
                from: port_type_name(&from.port_type),
                to: port_type_name(&to.port_type),
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

        if from.port_type == PortType::Control {
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

    for ((node, port, direction), count) in &counts {
        if *direction == Direction::Output {
            let mut orders: Vec<_> = edges
                .iter()
                .filter(|edge| edge.from.node == *node && edge.from.port == *port)
                .map(|edge| edge.order)
                .collect();
            orders.sort_unstable();
            if orders.iter().copied().ne(0_u16..orders.len() as u16) {
                return Err(CompileError::NonContiguousEdgeOrder {
                    node: node.clone(),
                    port: port.clone(),
                });
            }
        }
        let _ = count;
    }

    for (node_id, node) in nodes {
        for (name, definition) in ports(node) {
            let count = counts
                .get(&(node_id.clone(), name.to_owned(), definition.direction))
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
                        port: name.to_owned(),
                        expected,
                        actual: count,
                    });
                }
                Direction::Output if count != 1 => {
                    return Err(CompileError::OutputCardinality {
                        node: node_id.clone(),
                        port: name.to_owned(),
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
    edges: &[Edge],
) -> Result<Vec<String>, CompileError> {
    let mut indegree: BTreeMap<String, usize> = nodes.keys().map(|id| (id.clone(), 0)).collect();
    let mut successors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for edge in edges {
        *indegree
            .get_mut(&edge.to.node)
            .expect("edge nodes validated") += 1;
        successors
            .entry(edge.from.node.clone())
            .or_default()
            .push(edge.to.node.clone());
    }
    for values in successors.values_mut() {
        values.sort();
    }

    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
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
    edges: &[Edge],
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

fn graph_depth(order: &[String], edges: &[Edge]) -> usize {
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
    Trigger,
    Node(&'a Node),
}

fn ports(node: &RawNode<'_>) -> Vec<(&'static str, Port)> {
    match node {
        RawNode::Trigger => vec![("next", output(PortType::Control))],
        RawNode::Node(Node::Snapshot { value_type, .. }) => vec![
            ("in", control_input()),
            ("next", output(PortType::Control)),
            ("value", output(PortType::Value(value_type.clone()))),
        ],
        RawNode::Node(Node::Compare { value_type, .. }) => vec![
            ("in", control_input()),
            ("left", data_input(PortType::Value(value_type.clone()))),
            ("next", output(PortType::Control)),
            ("result", output(PortType::ComparisonOutcome)),
        ],
        RawNode::Node(Node::Branch { .. }) => vec![
            ("in", control_input()),
            ("condition", data_input(PortType::ComparisonOutcome)),
            ("true", output(PortType::Control)),
            ("false", output(PortType::Control)),
            ("unknown", output(PortType::Control)),
            ("error", output(PortType::Control)),
        ],
        RawNode::Node(Node::OnOffSet { .. }) => {
            vec![("in", control_input()), ("next", output(PortType::Control))]
        }
        RawNode::Node(Node::End { .. }) => vec![("in", control_input())],
    }
}

fn port(node: &RawNode<'_>, name: &str) -> Option<Port> {
    ports(node)
        .into_iter()
        .find_map(|(candidate, port)| (candidate == name).then_some(port))
}

fn output(port_type: PortType) -> Port {
    Port {
        direction: Direction::Output,
        port_type,
        minimum: 1,
        maximum: Some(1),
    }
}

fn control_input() -> Port {
    Port {
        direction: Direction::Input,
        port_type: PortType::Control,
        minimum: 1,
        maximum: None,
    }
}

fn data_input(port_type: PortType) -> Port {
    Port {
        direction: Direction::Input,
        port_type,
        minimum: 1,
        maximum: Some(1),
    }
}

fn port_type_name(port_type: &PortType) -> String {
    match port_type {
        PortType::Control => "control".to_owned(),
        PortType::ComparisonOutcome => "comparison-outcome".to_owned(),
        PortType::Value(value_type) => value_type.name().to_owned(),
    }
}

#[allow(dead_code)]
fn _assert_contract_ref_closed(_: &ContractRef) {}
