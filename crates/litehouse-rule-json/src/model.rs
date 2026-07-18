use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "1.0.0";
pub const SCHEMA_URL: &str = "https://schemas.litehouse.dev/rule-ir/v1.0.0/schema.json";
pub const COMPILER_PROFILE: &str = "litehouse:rule-compiler@1";
pub const HARD_MAX_JSON_BYTES: usize = 128 * 1024;
pub const HARD_MAX_NODES: usize = 128;
pub const HARD_MAX_EDGES: usize = 256;
pub const HARD_MAX_GRAPH_DEPTH: usize = 32;
pub const HARD_MAX_JSON_DEPTH: usize = 32;
pub const HARD_MAX_STRING_BYTES: usize = 16 * 1024;
pub const HARD_MAX_COLLECTION_ITEMS: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleDocument {
    pub schema: SchemaLock,
    #[schemars(regex(pattern = "^[0-9a-f]{32}$"))]
    pub rule_id: String,
    pub revision: u64,
    pub semantic: SemanticRule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaLock {
    #[schemars(length(min = 1, max = 256))]
    pub url: String,
    #[schemars(length(min = 1, max = 16))]
    pub version: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub sha256: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub profile_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticRule {
    #[schemars(length(min = 1, max = 128))]
    pub name: String,
    #[schemars(length(max = 4096))]
    pub description: String,
    pub locks: SemanticLocks,
    pub limits: RuleLimits,
    pub trigger: TriggerNode,
    #[schemars(length(max = 127))]
    pub nodes: Vec<Node>,
    #[schemars(length(max = 256))]
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SemanticLocks {
    #[schemars(length(min = 1, max = 64))]
    pub compiler: String,
    #[schemars(length(min = 1, max = 64))]
    pub quantity_catalog: String,
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
}

impl TriggerNode {
    pub fn id(&self) -> &str {
        match self {
            Self::StateObservation { id, .. } => id,
        }
    }

    pub fn version(&self) -> u8 {
        match self {
            Self::StateObservation { version, .. } => *version,
        }
    }

    pub fn selector(&self) -> &Selector {
        match self {
            Self::StateObservation { selector, .. } => selector,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Node {
    Snapshot {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        selector: Selector,
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        field: String,
        value_type: ValueType,
    },
    Compare {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        value_type: ValueType,
        expression: CompareExpression,
    },
    Branch {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
    },
    OnOffSet {
        #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
        id: String,
        #[schemars(range(min = 1, max = 1))]
        version: u8,
        selector: Selector,
        value: bool,
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
            | Self::Compare { id, .. }
            | Self::Branch { id, .. }
            | Self::OnOffSet { id, .. }
            | Self::End { id, .. } => id,
        }
    }

    pub fn version(&self) -> u8 {
        match self {
            Self::Snapshot { version, .. }
            | Self::Compare { version, .. }
            | Self::Branch { version, .. }
            | Self::OnOffSet { version, .. }
            | Self::End { version, .. } => *version,
        }
    }

    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Snapshot { .. } => "snapshot",
            Self::Compare { .. } => "compare",
            Self::Branch { .. } => "branch",
            Self::OnOffSet { .. } => "on_off_set",
            Self::End { .. } => "end",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareExpression {
    pub operator: CompareOperator,
    pub right: ScalarLiteral,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ValueType {
    Bool,
    String,
    I64,
    Quantity {
        #[schemars(length(min = 1, max = 128))]
        profile: String,
    },
}

impl ValueType {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::String => "string",
            Self::I64 => "i64",
            Self::Quantity { .. } => "quantity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScalarLiteral {
    Bool {
        value: bool,
    },
    String {
        #[schemars(length(max = 16384))]
        value: String,
    },
    I64 {
        value: i64,
    },
    Quantity {
        value: QuantityWire,
    },
}

impl ScalarLiteral {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Bool { .. } => "bool",
            Self::String { .. } => "string",
            Self::I64 { .. } => "i64",
            Self::Quantity { .. } => "quantity",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuantityWire {
    #[schemars(length(min = 1, max = 128))]
    pub profile: String,
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
    pub major: u16,
}

impl ContractRef {
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
    pub order: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EdgeEndpoint {
    #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
    pub node: String,
    #[schemars(regex(pattern = "^[a-z][a-z0-9_-]{0,63}$"))]
    pub port: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EndOutcome {
    Success,
    Noop,
    Unknown,
    Error,
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

pub(crate) fn valid_field(value: &str) -> bool {
    valid_id(value)
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
