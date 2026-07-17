//! Closed v1 management operation inventory and validation.
//!
//! This module describes adapter inputs. It deliberately contains no routing,
//! transport, authorization implementation, or I/O.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashSet},
    error::Error,
    fmt,
    sync::OnceLock,
};

pub const MANAGEMENT_OPERATION_COUNT: usize = 74;
pub const CANONICAL_OPERATION_COUNT: usize = 45;
pub const CAPABILITY_OPERATION_COUNT: usize = 29;
pub const MANAGEMENT_OPERATIONS_V1_JSON: &[u8] =
    include_bytes!("../management/management-operations-v1.json");

pub const FIXED_WIRE_SCOPES: [OAuthScope; 6] = [
    OAuthScope::Read,
    OAuthScope::Operate,
    OAuthScope::Configure,
    OAuthScope::Install,
    OAuthScope::Diagnostics,
    OAuthScope::Admin,
];

pub const CANONICAL_OPERATION_IDS: [&str; CANONICAL_OPERATION_COUNT] = [
    "system.get@1",
    "system.health.get@1",
    "schemas.get@1",
    "instances.list@1",
    "instances.get@1",
    "instances.facets.list@1",
    "instances.validate@1",
    "instances.draft.create@1",
    "instances.draft.update@1",
    "instances.activate@1",
    "instances.disable@1",
    "instances.delete@1",
    "facets.get@1",
    "facets.state.snapshot@1",
    "facets.state.stream@1",
    "facets.events.stream@1",
    "rules.list@1",
    "rules.get@1",
    "rules.revision.get@1",
    "rules.runs.page@1",
    "rules.run.get@1",
    "rules.validate@1",
    "rules.compile@1",
    "rules.simulate@1",
    "rules.draft.create@1",
    "rules.draft.update@1",
    "rules.activate@1",
    "rules.rollback@1",
    "rules.enable@1",
    "rules.disable@1",
    "rules.delete@1",
    "rules.trigger@1",
    "rules.run.cancel@1",
    "registry.list@1",
    "registry.release.get@1",
    "registry.refresh@1",
    "plugins.install.plan@1",
    "plugins.update.plan@1",
    "plugins.rollback.plan@1",
    "plugins.install.apply@1",
    "plugins.update.apply@1",
    "plugins.rollback.apply@1",
    "operations.result.get@1",
    "diagnostics.summary@1",
    "diagnostics.events.page@1",
];

pub const CAPABILITY_OPERATION_IDS: [&str; CAPABILITY_OPERATION_COUNT] = [
    "capability.on-off.set@1",
    "capability.on-off.toggle@1",
    "capability.level.set@1",
    "capability.level.step@1",
    "capability.level.move@1",
    "capability.level.stop@1",
    "capability.level.transition@1",
    "capability.level.set-coupled@1",
    "capability.level.step-coupled@1",
    "capability.level.move-coupled@1",
    "capability.level.transition-coupled@1",
    "capability.color.set@1",
    "capability.color.step@1",
    "capability.color.move@1",
    "capability.color.stop@1",
    "capability.color.transition@1",
    "capability.thermostat.set-mode@1",
    "capability.thermostat.set-single-target@1",
    "capability.thermostat.set-dual-target@1",
    "capability.thermostat.set-fan@1",
    "capability.thermostat.set-swing@1",
    "capability.thermostat.set-preset@1",
    "capability.covering.open@1",
    "capability.covering.close@1",
    "capability.covering.stop@1",
    "capability.covering.set-lift@1",
    "capability.covering.set-tilt@1",
    "capability.momentary.invoke@1",
    "capability.notification.send@1",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementManifest {
    pub manifest_version: u32,
    pub catalog: String,
    pub fixed_scopes: Vec<OAuthScope>,
    pub operations: Vec<ManagementOperation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementOperation {
    pub id: String,
    pub kind: OperationKind,
    pub service_binding: Option<CapabilityServiceBinding>,
    pub scope_policy: ScopePolicy,
    pub authorization: AuthorizationMetadata,
    pub class: OperationClass,
    pub idempotency: IdempotencyRule,
    pub preconditions: Preconditions,
    pub deadline: Deadline,
    pub progress: ProgressShape,
    pub stream: StreamShape,
    pub http: HttpBinding,
    pub webrtc: WebRtcBinding,
    pub mcp: McpBinding,
    pub web: WebMetadata,
    pub projection: ProjectionPolicy,
    pub redaction: RedactionPolicy,
    pub schemas: OperationSchemas,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityServiceBinding {
    pub package: String,
    pub interface: String,
    pub operation: String,
    pub projection: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationKind {
    Canonical,
    Capability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OAuthScope {
    #[serde(rename = "litehouse:read")]
    Read,
    #[serde(rename = "litehouse:operate")]
    Operate,
    #[serde(rename = "litehouse:configure")]
    Configure,
    #[serde(rename = "litehouse:install")]
    Install,
    #[serde(rename = "litehouse:diagnostics")]
    Diagnostics,
    #[serde(rename = "litehouse:admin")]
    Admin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ScopePolicy {
    Fixed { scope: OAuthScope },
    ReferencedOperation { operation_id_field: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationMetadata {
    pub action: AuthorizationAction,
    pub resource: AuthorizationResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorizationAction {
    Read,
    Validate,
    Create,
    Update,
    Activate,
    Rollback,
    Enable,
    Disable,
    Delete,
    Stream,
    Compile,
    Simulate,
    Trigger,
    Cancel,
    Refresh,
    Plan,
    Install,
    Lookup,
    Operate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuthorizationResource {
    System,
    SchemaCatalog,
    Instance,
    InstanceFacet,
    Facet,
    FacetState,
    FacetEvent,
    Rule,
    RuleRevision,
    RuleRun,
    Registry,
    PluginInstallation,
    OperationResult,
    Diagnostics,
    Capability,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OperationClass {
    Read,
    Pure,
    Mutation,
    Destructive,
}

impl OperationClass {
    pub const fn has_side_effect(self) -> bool {
        matches!(self, Self::Mutation | Self::Destructive)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IdempotencyRule {
    NotRequired,
    Required,
    HostRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preconditions {
    pub revision: RevisionPrecondition,
    pub generation: GenerationPrecondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RevisionPrecondition {
    None,
    ConfigRevision,
    DesiredOrActiveRevision,
    ExactRevisionAndCascade,
    OptionalDraftRevision,
    BaseRevision,
    RuleOrActiveRevision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GenerationPrecondition {
    None,
    FacetGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deadline {
    pub default_ms: u32,
    pub maximum_ms: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProgressShape {
    None,
    BoundedJob {
        phases: Vec<JobPhase>,
        maximum_updates: u16,
        current_total: bool,
        correlation_id: bool,
        truncation: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobPhase {
    Validate,
    Compile,
    Simulate,
    Refresh,
    Stage,
    Apply,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum StreamShape {
    None,
    BoundedPullBatch {
        records: StreamRecords,
        default_items: u16,
        maximum_items: u16,
        maximum_bytes: u32,
        cursor: CursorScope,
        gap: StreamFailureShape,
        stale: StreamFailureShape,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StreamRecords {
    State,
    Event,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CursorScope {
    GenerationScoped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StreamFailureShape {
    Typed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpBinding {
    pub method: HttpMethod,
    pub path: String,
    pub cache: CachePolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    #[serde(rename = "POST")]
    Post,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CachePolicy {
    NoStore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebRtcBinding {
    pub operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpBinding {
    pub tool: String,
    pub resource_template: Option<String>,
    pub annotations: McpAnnotations,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpAnnotations {
    pub read_only: bool,
    pub destructive: bool,
    pub idempotent: bool,
    pub open_world: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebMetadata {
    pub group: WebGroup,
    pub label: String,
    pub cache: CachePolicy,
    pub destructive_confirmation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WebGroup {
    System,
    Schemas,
    Instances,
    Facets,
    Rules,
    Registry,
    Plugins,
    Operations,
    Diagnostics,
    Capabilities,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectionPolicy {
    pub input: ProjectionMode,
    pub output: ProjectionMode,
    pub unknown_fields: UnknownInputPolicy,
    pub protected_values: ProtectedValuePolicy,
    pub private_addresses: PrivateAddressPolicy,
    pub errors: ErrorProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectionMode {
    SchemaAllowlist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnknownInputPolicy {
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProtectedValuePolicy {
    Omit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PrivateAddressPolicy {
    Omit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorProjection {
    SafeEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RedactionPolicy {
    pub unknown_fields: UnknownOutputPolicy,
    pub protected_values: ProtectedValuePolicy,
    pub private_addresses: PrivateAddressPolicy,
    pub untrusted_text: UntrustedTextPolicy,
    pub diagnostics: DiagnosticProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UnknownOutputPolicy {
    Hide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UntrustedTextPolicy {
    PreserveMarked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticProjection {
    Allowlisted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationSchemas {
    pub input: SchemaDocument,
    pub output: SchemaDocument,
    pub error: SchemaDocument,
    pub mcp_projection: SchemaDocument,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaDocument {
    pub sha256: String,
    pub schema: Value,
}

/// Stable, sorted metadata needed by future HTTP/WebRTC/MCP/web generators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectionMetadata<'a> {
    pub operation_id: &'a str,
    pub scope_policy: &'a ScopePolicy,
    pub authorization: AuthorizationMetadata,
    pub class: OperationClass,
    pub idempotency: IdempotencyRule,
    pub preconditions: Preconditions,
    pub deadline: Deadline,
    pub service_binding: Option<&'a CapabilityServiceBinding>,
    pub http_method: HttpMethod,
    pub http_path: &'a str,
    pub webrtc_operation_id: &'a str,
    pub mcp_tool: &'a str,
    pub mcp_resource_template: Option<&'a str>,
    pub canonical_input_sha256: &'a str,
    pub canonical_output_sha256: &'a str,
    pub canonical_error_sha256: &'a str,
    pub mcp_projection_sha256: &'a str,
    pub projection: ProjectionPolicy,
    pub redaction: RedactionPolicy,
}

impl ManagementManifest {
    pub fn parse(bytes: &[u8]) -> Result<Self, ManifestError> {
        parse_management_manifest(bytes)
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        validate_management_manifest(self)
    }

    pub fn projection_metadata(&self) -> Vec<ProjectionMetadata<'_>> {
        let mut metadata = self
            .operations
            .iter()
            .map(|operation| ProjectionMetadata {
                operation_id: &operation.id,
                scope_policy: &operation.scope_policy,
                authorization: operation.authorization,
                class: operation.class,
                idempotency: operation.idempotency,
                preconditions: operation.preconditions,
                deadline: operation.deadline,
                service_binding: operation.service_binding.as_ref(),
                http_method: operation.http.method,
                http_path: &operation.http.path,
                webrtc_operation_id: &operation.webrtc.operation_id,
                mcp_tool: &operation.mcp.tool,
                mcp_resource_template: operation.mcp.resource_template.as_deref(),
                canonical_input_sha256: &operation.schemas.input.sha256,
                canonical_output_sha256: &operation.schemas.output.sha256,
                canonical_error_sha256: &operation.schemas.error.sha256,
                mcp_projection_sha256: &operation.schemas.mcp_projection.sha256,
                projection: operation.projection,
                redaction: operation.redaction,
            })
            .collect::<Vec<_>>();
        metadata.sort_by(|left, right| left.operation_id.cmp(right.operation_id));
        metadata
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    Json(String),
    Invalid(String),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message) => write!(formatter, "invalid management manifest JSON: {message}"),
            Self::Invalid(message) => write!(formatter, "invalid management manifest: {message}"),
        }
    }
}

impl Error for ManifestError {}

static MANAGEMENT_MANIFEST: OnceLock<Result<ManagementManifest, ManifestError>> = OnceLock::new();

pub fn management_manifest() -> Result<&'static ManagementManifest, ManifestError> {
    match MANAGEMENT_MANIFEST
        .get_or_init(|| parse_management_manifest(MANAGEMENT_OPERATIONS_V1_JSON))
    {
        Ok(manifest) => Ok(manifest),
        Err(error) => Err(error.clone()),
    }
}

pub fn parse_management_manifest(bytes: &[u8]) -> Result<ManagementManifest, ManifestError> {
    let manifest = serde_json::from_slice::<ManagementManifest>(bytes)
        .map_err(|error| ManifestError::Json(error.to_string()))?;
    validate_management_manifest(&manifest)?;
    Ok(manifest)
}

pub fn schema_sha256(schema: &Value) -> Result<String, ManifestError> {
    let canonical = serde_json::to_vec(schema)
        .map_err(|error| ManifestError::Invalid(format!("cannot serialize schema: {error}")))?;
    Ok(hex(Sha256::digest(canonical)))
}

pub fn validate_management_manifest(manifest: &ManagementManifest) -> Result<(), ManifestError> {
    require(manifest.manifest_version == 1, "manifest_version must be 1")?;
    require(
        manifest.catalog == "litehouse-management-operations",
        "catalog must be litehouse-management-operations",
    )?;
    require(
        manifest.fixed_scopes.as_slice() == FIXED_WIRE_SCOPES,
        "fixed_scopes must contain the six v1 wire scopes in canonical order",
    )?;
    require(
        manifest.operations.len() == MANAGEMENT_OPERATION_COUNT,
        "manifest must contain exactly 74 operations",
    )?;

    let mut expected_ids = CANONICAL_OPERATION_IDS
        .iter()
        .chain(CAPABILITY_OPERATION_IDS.iter())
        .copied()
        .collect::<Vec<_>>();
    expected_ids.sort_unstable();

    let actual_ids = manifest
        .operations
        .iter()
        .map(|operation| operation.id.as_str())
        .collect::<Vec<_>>();
    require(
        actual_ids.windows(2).all(|pair| pair[0] < pair[1]),
        "operations must be unique and sorted by exact operation ID",
    )?;
    require(
        actual_ids == expected_ids,
        "operation ID inventory is not the closed v1 catalog",
    )?;

    let canonical_count = manifest
        .operations
        .iter()
        .filter(|operation| operation.kind == OperationKind::Canonical)
        .count();
    let capability_count = manifest
        .operations
        .iter()
        .filter(|operation| operation.kind == OperationKind::Capability)
        .count();
    require(
        canonical_count == CANONICAL_OPERATION_COUNT
            && capability_count == CAPABILITY_OPERATION_COUNT,
        "operation kind counts must be 45 canonical and 29 capability",
    )?;

    let mut paths = HashSet::with_capacity(MANAGEMENT_OPERATION_COUNT);
    let mut tools = HashSet::with_capacity(MANAGEMENT_OPERATION_COUNT);
    let mut webrtc_ids = HashSet::with_capacity(MANAGEMENT_OPERATION_COUNT);
    for operation in &manifest.operations {
        validate_operation(operation)?;
        require(
            paths.insert(&operation.http.path),
            operation_error(operation, "duplicate HTTP path"),
        )?;
        require(
            tools.insert(&operation.mcp.tool),
            operation_error(operation, "duplicate MCP tool"),
        )?;
        require(
            webrtc_ids.insert(&operation.webrtc.operation_id),
            operation_error(operation, "duplicate WebRTC operation ID"),
        )?;
    }
    Ok(())
}

fn validate_operation(operation: &ManagementOperation) -> Result<(), ManifestError> {
    let expected_kind = if CAPABILITY_OPERATION_IDS.contains(&operation.id.as_str()) {
        OperationKind::Capability
    } else {
        OperationKind::Canonical
    };
    require(
        operation.kind == expected_kind,
        operation_error(
            operation,
            "operation kind does not match its closed catalog ID",
        ),
    )?;
    require(
        operation.service_binding == expected_service_binding(&operation.id),
        operation_error(
            operation,
            "capability service binding does not match the closed projection map",
        ),
    )?;

    let (name, major) = operation.id.rsplit_once('@').ok_or_else(|| {
        ManifestError::Invalid(operation_error(operation, "ID lacks exact major"))
    })?;
    require(
        major == "1",
        operation_error(operation, "only exact major 1 is allowed"),
    )?;
    require(
        valid_operation_name(name),
        operation_error(operation, "invalid operation ID syntax"),
    )?;
    require(
        !contains_escape_hatch(name),
        operation_error(
            operation,
            "generic or administrative escape hatch is forbidden",
        ),
    )?;

    validate_scope(operation)?;
    require(
        operation.authorization == expected_authorization(&operation.id),
        operation_error(
            operation,
            "authorization action/resource does not match the operation",
        ),
    )?;
    let expected_class = expected_class(&operation.id);
    require(
        operation.class == expected_class,
        operation_error(
            operation,
            "operation class does not match the closed catalog",
        ),
    )?;
    let expected_idempotency = if operation.kind == OperationKind::Capability {
        IdempotencyRule::HostRequired
    } else if operation.class.has_side_effect() {
        IdempotencyRule::Required
    } else {
        IdempotencyRule::NotRequired
    };
    require(
        operation.idempotency == expected_idempotency,
        operation_error(operation, "idempotency rule does not match operation class"),
    )?;
    require(
        !operation.class.has_side_effect() || operation.idempotency != IdempotencyRule::NotRequired,
        operation_error(operation, "every mutation must require idempotency"),
    )?;

    require(
        operation.preconditions == expected_preconditions(&operation.id),
        operation_error(
            operation,
            "revision/generation preconditions do not match the operation",
        ),
    )?;
    require(
        operation.deadline == expected_deadline(&operation.id),
        operation_error(
            operation,
            "deadline does not match the bounded operation policy",
        ),
    )?;
    require(
        operation.deadline.default_ms > 0
            && operation.deadline.default_ms <= operation.deadline.maximum_ms
            && operation.deadline.maximum_ms <= 300_000,
        operation_error(
            operation,
            "deadline must be nonzero, ordered, and at most 300 seconds",
        ),
    )?;
    validate_progress_and_stream(operation)?;

    let expected_path = format!("/api/operations/{name}/v{major}");
    require(
        operation.http.method == HttpMethod::Post
            && operation.http.path == expected_path
            && operation.http.cache == CachePolicy::NoStore,
        operation_error(
            operation,
            "HTTP must be the exact derived POST no-store binding",
        ),
    )?;
    require(
        operation.webrtc.operation_id == operation.id,
        operation_error(
            operation,
            "WebRTC binding must carry the exact operation ID",
        ),
    )?;
    let expected_tool = format!("litehouse_{}_v{major}", name.replace(['.', '-'], "_"));
    require(
        operation.mcp.tool == expected_tool,
        operation_error(
            operation,
            "MCP tool is not exactly derived from the operation ID",
        ),
    )?;
    require(
        operation.mcp.resource_template.as_deref() == expected_resource_template(&operation.id),
        operation_error(
            operation,
            "MCP resource mapping does not match the closed resource list",
        ),
    )?;
    let expected_annotations = McpAnnotations {
        read_only: matches!(operation.class, OperationClass::Read | OperationClass::Pure),
        destructive: operation.class == OperationClass::Destructive,
        idempotent: true,
        open_world: false,
    };
    require(
        operation.mcp.annotations == expected_annotations,
        operation_error(
            operation,
            "MCP annotations do not reflect canonical metadata",
        ),
    )?;
    require(
        operation.web.group == expected_web_group(&operation.id)
            && !operation.web.label.is_empty()
            && operation.web.label.len() <= 96
            && operation.web.cache == CachePolicy::NoStore
            && operation.web.destructive_confirmation
                == (operation.class == OperationClass::Destructive),
        operation_error(operation, "web metadata is not deterministic or no-store"),
    )?;
    require(
        operation.projection == canonical_projection_policy()
            && operation.redaction == canonical_redaction_policy(),
        operation_error(
            operation,
            "projection/redaction policy must be the closed safe policy",
        ),
    )?;

    validate_schema_document(operation, "canonical input", &operation.schemas.input)?;
    validate_schema_document(operation, "canonical output", &operation.schemas.output)?;
    validate_schema_document(operation, "canonical error", &operation.schemas.error)?;
    validate_schema_document(
        operation,
        "MCP projection",
        &operation.schemas.mcp_projection,
    )?;
    validate_schema_semantics(operation)?;
    Ok(())
}

fn validate_scope(operation: &ManagementOperation) -> Result<(), ManifestError> {
    if operation.id == "operations.result.get@1" {
        require(
            operation.scope_policy
                == ScopePolicy::ReferencedOperation {
                    operation_id_field: "operation_id".into(),
                },
            operation_error(
                operation,
                "result lookup must use the referenced operation's scope",
            ),
        )
    } else {
        let expected = expected_fixed_scope(&operation.id);
        require(
            operation.scope_policy == ScopePolicy::Fixed { scope: expected }
                && expected != OAuthScope::Admin,
            operation_error(
                operation,
                "fixed scope does not match the exact v1 operation scope",
            ),
        )
    }
}

fn validate_progress_and_stream(operation: &ManagementOperation) -> Result<(), ManifestError> {
    let expected_records = match operation.id.as_str() {
        "facets.state.stream@1" => Some(StreamRecords::State),
        "facets.events.stream@1" => Some(StreamRecords::Event),
        _ => None,
    };
    match (&operation.stream, expected_records) {
        (
            StreamShape::BoundedPullBatch {
                records,
                default_items,
                maximum_items,
                maximum_bytes,
                cursor,
                gap,
                stale,
            },
            Some(expected),
        ) => require(
            *records == expected
                && *default_items == 50
                && *maximum_items == 200
                && *maximum_bytes == 65_536
                && *cursor == CursorScope::GenerationScoped
                && *gap == StreamFailureShape::Typed
                && *stale == StreamFailureShape::Typed,
            operation_error(
                operation,
                "stream batch shape is not the bounded generation-scoped v1 shape",
            ),
        )?,
        (StreamShape::None, None) => {}
        _ => {
            return Err(ManifestError::Invalid(operation_error(
                operation,
                "stream shape does not match operation",
            )));
        }
    }

    let expected_phases: Option<&[JobPhase]> = match operation.id.as_str() {
        "rules.compile@1" => Some(&[JobPhase::Validate, JobPhase::Compile]),
        "rules.simulate@1" => Some(&[JobPhase::Validate, JobPhase::Simulate]),
        "registry.refresh@1" => Some(&[JobPhase::Refresh, JobPhase::Verify]),
        "plugins.install.apply@1" | "plugins.update.apply@1" | "plugins.rollback.apply@1" => {
            Some(&[
                JobPhase::Validate,
                JobPhase::Stage,
                JobPhase::Apply,
                JobPhase::Verify,
            ])
        }
        _ => None,
    };
    match (&operation.progress, expected_phases) {
        (
            ProgressShape::BoundedJob {
                phases,
                maximum_updates,
                current_total,
                correlation_id,
                truncation,
            },
            Some(expected),
        ) => require(
            phases == expected
                && *maximum_updates == 64
                && *current_total
                && *correlation_id
                && *truncation,
            operation_error(operation, "bounded progress shape does not match operation"),
        )?,
        (ProgressShape::None, None) => {}
        _ => {
            return Err(ManifestError::Invalid(operation_error(
                operation,
                "progress shape does not match operation",
            )));
        }
    }
    Ok(())
}

fn validate_schema_document(
    operation: &ManagementOperation,
    role: &str,
    document: &SchemaDocument,
) -> Result<(), ManifestError> {
    require(
        document.sha256.len() == 64
            && document
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        operation_error(
            operation,
            &format!("{role} digest must be lowercase SHA-256"),
        ),
    )?;
    let actual = schema_sha256(&document.schema)?;
    require(
        actual == document.sha256,
        operation_error(operation, &format!("{role} schema SHA-256 mismatch")),
    )?;

    let root = document.schema.as_object().ok_or_else(|| {
        ManifestError::Invalid(operation_error(
            operation,
            &format!("{role} schema root must be an object"),
        ))
    })?;
    let expected_title = format!("{} {role}", operation.id);
    require(
        root.get("$schema").and_then(Value::as_str)
            == Some("https://json-schema.org/draft/2020-12/schema")
            && root.get("title").and_then(Value::as_str) == Some(expected_title.as_str()),
        operation_error(
            operation,
            &format!("{role} schema identity is not operation-specific"),
        ),
    )?;
    validate_closed_schema_node(operation, role, &document.schema, "$")
}

fn validate_closed_schema_node(
    operation: &ManagementOperation,
    role: &str,
    node: &Value,
    path: &str,
) -> Result<(), ManifestError> {
    match node {
        Value::Object(object) => {
            require(
                !object.contains_key("$ref"),
                operation_error(
                    operation,
                    &format!("{role} schema uses unresolved reference at {path}"),
                ),
            )?;
            if object.get("type").and_then(Value::as_str) == Some("object") {
                require(
                    object.get("additionalProperties") == Some(&Value::Bool(false)),
                    operation_error(operation, &format!("{role} object is not closed at {path}")),
                )?;
                let properties = object
                    .get("properties")
                    .and_then(Value::as_object)
                    .ok_or_else(|| {
                        ManifestError::Invalid(operation_error(
                            operation,
                            &format!("{role} object has no property allowlist at {path}"),
                        ))
                    })?;
                let required = object
                    .get("required")
                    .and_then(Value::as_array)
                    .ok_or_else(|| {
                        ManifestError::Invalid(operation_error(
                            operation,
                            &format!("{role} object has no required list at {path}"),
                        ))
                    })?;
                let mut names = BTreeSet::new();
                for name in required {
                    let name = name.as_str().ok_or_else(|| {
                        ManifestError::Invalid(operation_error(
                            operation,
                            &format!("{role} required name is not a string at {path}"),
                        ))
                    })?;
                    require(
                        properties.contains_key(name) && names.insert(name),
                        operation_error(
                            operation,
                            &format!("{role} required list is invalid at {path}"),
                        ),
                    )?;
                }
                for name in properties.keys() {
                    require(
                        !protected_or_escape_field(name),
                        operation_error(
                            operation,
                            &format!(
                                "{role} exposes protected/address/generic field {name:?} at {path}"
                            ),
                        ),
                    )?;
                }
            }
            for (key, child) in object {
                validate_closed_schema_node(operation, role, child, &format!("{path}/{key}"))?;
            }
        }
        Value::Array(values) => {
            for (index, child) in values.iter().enumerate() {
                validate_closed_schema_node(operation, role, child, &format!("{path}/{index}"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_schema_semantics(operation: &ManagementOperation) -> Result<(), ManifestError> {
    let input = schema_properties(&operation.schemas.input.schema)?;
    let input_required = schema_required(&operation.schemas.input.schema)?;
    let output = schema_properties(&operation.schemas.output.schema)?;
    require(
        !output.is_empty(),
        operation_error(
            operation,
            "canonical output schema must be operation-specific and nonempty",
        ),
    )?;

    if operation.preconditions.generation == GenerationPrecondition::FacetGeneration {
        require(
            input.contains_key("facet_generation") && input_required.contains("facet_generation"),
            operation_error(
                operation,
                "facet generation precondition must be a required canonical field",
            ),
        )?;
    }
    validate_revision_schema(operation, input, &input_required)?;

    if operation.id == "operations.result.get@1" {
        require(
            input_required.contains("operation_id") && input_required.contains("idempotency_key"),
            operation_error(
                operation,
                "result lookup requires operation_id and idempotency_key",
            ),
        )?;
    }

    let error_properties = schema_properties(&operation.schemas.error.schema)?;
    require(
        error_properties
            .get("operation_id")
            .and_then(Value::as_object)
            .and_then(|schema| schema.get("const"))
            .and_then(Value::as_str)
            == Some(operation.id.as_str()),
        operation_error(
            operation,
            "canonical error schema must be pinned to the exact operation ID",
        ),
    )?;

    let projected = schema_properties(&operation.schemas.mcp_projection.schema)?;
    let projected_required = schema_required(&operation.schemas.mcp_projection.schema)?;
    for (name, schema) in input {
        require(
            projected.get(name) == Some(schema),
            operation_error(
                operation,
                "MCP projection must preserve every allowed canonical input field",
            ),
        )?;
    }
    require(
        projected.contains_key("timeout_ms"),
        operation_error(
            operation,
            "MCP projection must expose the bounded relative timeout",
        ),
    )?;
    let expected_extra =
        if operation.class.has_side_effect() && !input.contains_key("idempotency_key") {
            2
        } else {
            1
        };
    require(
        projected.len() == input.len() + expected_extra,
        operation_error(
            operation,
            "MCP projection may add only timeout and required idempotency metadata",
        ),
    )?;
    if operation.class.has_side_effect() {
        require(
            projected.contains_key("idempotency_key")
                && projected_required.contains("idempotency_key"),
            operation_error(
                operation,
                "MCP mutation projection must require idempotency_key",
            ),
        )?;
    }
    for name in &input_required {
        require(
            projected_required.contains(name),
            operation_error(
                operation,
                "MCP projection weakened canonical required fields",
            ),
        )?;
    }
    Ok(())
}

fn validate_revision_schema(
    operation: &ManagementOperation,
    input: &serde_json::Map<String, Value>,
    required: &BTreeSet<&str>,
) -> Result<(), ManifestError> {
    let required_field = match (operation.id.as_str(), operation.preconditions.revision) {
        (_, RevisionPrecondition::None) => return Ok(()),
        (_, RevisionPrecondition::OptionalDraftRevision) => {
            return require(
                input.contains_key("draft_revision") && !required.contains("draft_revision"),
                operation_error(
                    operation,
                    "optional draft revision must be declared but optional",
                ),
            );
        }
        (_, RevisionPrecondition::ConfigRevision) => "config_revision",
        ("instances.activate@1", RevisionPrecondition::DesiredOrActiveRevision) => {
            "desired_revision"
        }
        ("instances.disable@1", RevisionPrecondition::DesiredOrActiveRevision) => "active_revision",
        (id, RevisionPrecondition::DesiredOrActiveRevision) if id.starts_with("plugins.") => {
            "desired_or_active_revision"
        }
        (_, RevisionPrecondition::ExactRevisionAndCascade) => "exact_revision",
        (_, RevisionPrecondition::BaseRevision) => "base_revision",
        ("rules.activate@1", RevisionPrecondition::RuleOrActiveRevision) => "rule_revision",
        ("rules.rollback@1", RevisionPrecondition::RuleOrActiveRevision) => "active_revision",
        (_, RevisionPrecondition::RuleOrActiveRevision) => "active_revision",
        _ => {
            return Err(ManifestError::Invalid(operation_error(
                operation,
                "unsupported revision precondition mapping",
            )));
        }
    };
    require(
        input.contains_key(required_field) && required.contains(required_field),
        operation_error(
            operation,
            "required revision precondition is absent from canonical input",
        ),
    )?;
    if operation.preconditions.revision == RevisionPrecondition::ExactRevisionAndCascade {
        require(
            required.contains("cascade") && required.contains("dependencies"),
            operation_error(
                operation,
                "destructive revision precondition requires an explicit cascade set",
            ),
        )?;
    }
    Ok(())
}

fn schema_properties(schema: &Value) -> Result<&serde_json::Map<String, Value>, ManifestError> {
    schema
        .as_object()
        .and_then(|root| root.get("properties"))
        .and_then(Value::as_object)
        .ok_or_else(|| ManifestError::Invalid("schema root has no properties".into()))
}

fn schema_required(schema: &Value) -> Result<BTreeSet<&str>, ManifestError> {
    schema
        .as_object()
        .and_then(|root| root.get("required"))
        .and_then(Value::as_array)
        .ok_or_else(|| ManifestError::Invalid("schema root has no required list".into()))?
        .iter()
        .map(|value| {
            value.as_str().ok_or_else(|| {
                ManifestError::Invalid("schema required item is not a string".into())
            })
        })
        .collect()
}

fn expected_service_binding(id: &str) -> Option<CapabilityServiceBinding> {
    let (package, operation, projection) = match id {
        "capability.on-off.set@1" => ("on-off", "set", "boolean-value"),
        "capability.on-off.toggle@1" => ("on-off", "toggle", "direct"),
        "capability.level.set@1" => ("level-control", "set-level", "quantity-level"),
        "capability.level.step@1" => ("level-control", "step", "signed-step"),
        "capability.level.move@1" => ("level-control", "move", "directional-rate"),
        "capability.level.stop@1" => ("level-control", "stop", "direct"),
        "capability.level.transition@1" => {
            ("level-control", "set-level", "quantity-level-transition")
        }
        "capability.level.set-coupled@1" => (
            "level-control",
            "set-level-with-on-off",
            "quantity-level-coupled",
        ),
        "capability.level.step-coupled@1" => {
            ("level-control", "step-with-on-off", "signed-step-coupled")
        }
        "capability.level.move-coupled@1" => (
            "level-control",
            "move-with-on-off",
            "directional-rate-coupled",
        ),
        "capability.level.transition-coupled@1" => (
            "level-control",
            "set-level-with-on-off",
            "quantity-level-transition-coupled",
        ),
        "capability.color.set@1" => ("color-control", "set-color", "color-value"),
        "capability.color.step@1" => ("color-control", "step-color", "directional-color-delta"),
        "capability.color.move@1" => ("color-control", "move-color", "directional-color-rate"),
        "capability.color.stop@1" => ("color-control", "stop", "direct"),
        "capability.color.transition@1" => ("color-control", "set-color", "color-value-transition"),
        "capability.thermostat.set-mode@1" => ("thermostat", "set-mode", "thermostat-mode"),
        "capability.thermostat.set-single-target@1" => {
            ("thermostat", "set-single-target", "celsius-single-target")
        }
        "capability.thermostat.set-dual-target@1" => {
            ("thermostat", "set-dual-target", "celsius-dual-target")
        }
        "capability.thermostat.set-fan@1" => ("thermostat", "set-fan", "choice-value"),
        "capability.thermostat.set-swing@1" => ("thermostat", "set-swing", "choice-value"),
        "capability.thermostat.set-preset@1" => ("thermostat", "set-preset", "choice-value"),
        "capability.covering.open@1" => ("window-covering", "open", "axis-value"),
        "capability.covering.close@1" => ("window-covering", "close", "axis-value"),
        "capability.covering.stop@1" => ("window-covering", "stop", "axis-value"),
        "capability.covering.set-lift@1" => {
            ("window-covering", "set-position", "fixed-lift-position")
        }
        "capability.covering.set-tilt@1" => {
            ("window-covering", "set-position", "fixed-tilt-position")
        }
        "capability.momentary.invoke@1" => ("momentary-action", "invoke", "direct"),
        "capability.notification.send@1" => (
            "notification-sink",
            "send",
            "notification-template-expansion",
        ),
        _ => return None,
    };
    Some(CapabilityServiceBinding {
        package: format!("litehouse:{package}@1.0.0"),
        interface: "capability".into(),
        operation: operation.into(),
        projection: projection.into(),
    })
}

fn expected_fixed_scope(id: &str) -> OAuthScope {
    if id.starts_with("capability.") || matches!(id, "rules.trigger@1" | "rules.run.cancel@1") {
        OAuthScope::Operate
    } else if id.starts_with("diagnostics.") {
        OAuthScope::Diagnostics
    } else if id.starts_with("plugins.") {
        OAuthScope::Install
    } else if (id.starts_with("instances.")
        && !matches!(
            id,
            "instances.list@1" | "instances.get@1" | "instances.facets.list@1"
        ))
        || (id.starts_with("rules.")
            && !matches!(
                id,
                "rules.list@1"
                    | "rules.get@1"
                    | "rules.revision.get@1"
                    | "rules.runs.page@1"
                    | "rules.run.get@1"
            ))
    {
        OAuthScope::Configure
    } else {
        OAuthScope::Read
    }
}

fn expected_class(id: &str) -> OperationClass {
    if matches!(id, "instances.delete@1" | "rules.delete@1") {
        OperationClass::Destructive
    } else if matches!(
        id,
        "instances.validate@1"
            | "rules.validate@1"
            | "rules.compile@1"
            | "rules.simulate@1"
            | "plugins.install.plan@1"
            | "plugins.update.plan@1"
            | "plugins.rollback.plan@1"
    ) {
        OperationClass::Pure
    } else if id.starts_with("capability.")
        || matches!(
            id,
            "instances.draft.create@1"
                | "instances.draft.update@1"
                | "instances.activate@1"
                | "instances.disable@1"
                | "rules.draft.create@1"
                | "rules.draft.update@1"
                | "rules.activate@1"
                | "rules.rollback@1"
                | "rules.enable@1"
                | "rules.disable@1"
                | "rules.trigger@1"
                | "rules.run.cancel@1"
                | "registry.refresh@1"
                | "plugins.install.apply@1"
                | "plugins.update.apply@1"
                | "plugins.rollback.apply@1"
        )
    {
        OperationClass::Mutation
    } else {
        OperationClass::Read
    }
}

fn expected_preconditions(id: &str) -> Preconditions {
    let revision = match id {
        "instances.draft.update@1" => RevisionPrecondition::ConfigRevision,
        "instances.activate@1"
        | "instances.disable@1"
        | "plugins.install.apply@1"
        | "plugins.update.apply@1"
        | "plugins.rollback.apply@1" => RevisionPrecondition::DesiredOrActiveRevision,
        "instances.delete@1" => RevisionPrecondition::ExactRevisionAndCascade,
        "rules.validate@1" | "rules.compile@1" | "rules.simulate@1" => {
            RevisionPrecondition::OptionalDraftRevision
        }
        "rules.draft.update@1" => RevisionPrecondition::BaseRevision,
        "rules.activate@1" | "rules.rollback@1" | "rules.enable@1" | "rules.disable@1"
        | "rules.delete@1" => RevisionPrecondition::RuleOrActiveRevision,
        _ => RevisionPrecondition::None,
    };
    let generation = if id.starts_with("facets.") || id.starts_with("capability.") {
        GenerationPrecondition::FacetGeneration
    } else {
        GenerationPrecondition::None
    };
    Preconditions {
        revision,
        generation,
    }
}

fn expected_deadline(id: &str) -> Deadline {
    let default_ms = match expected_class(id) {
        OperationClass::Read => 10_000,
        OperationClass::Pure | OperationClass::Mutation | OperationClass::Destructive => 30_000,
    };
    let maximum_ms = if id == "registry.refresh@1"
        || matches!(
            id,
            "plugins.install.apply@1" | "plugins.update.apply@1" | "plugins.rollback.apply@1"
        ) {
        300_000
    } else {
        30_000
    };
    Deadline {
        default_ms,
        maximum_ms,
    }
}

fn expected_authorization(id: &str) -> AuthorizationMetadata {
    if id.starts_with("capability.") {
        return AuthorizationMetadata {
            action: AuthorizationAction::Operate,
            resource: AuthorizationResource::Capability,
        };
    }
    if id.starts_with("diagnostics.") {
        return AuthorizationMetadata {
            action: AuthorizationAction::Read,
            resource: AuthorizationResource::Diagnostics,
        };
    }
    if id == "operations.result.get@1" {
        return AuthorizationMetadata {
            action: AuthorizationAction::Lookup,
            resource: AuthorizationResource::OperationResult,
        };
    }
    if id.starts_with("plugins.") {
        return AuthorizationMetadata {
            action: if id.contains(".plan@") {
                AuthorizationAction::Plan
            } else {
                AuthorizationAction::Install
            },
            resource: AuthorizationResource::PluginInstallation,
        };
    }
    if id.starts_with("registry.") {
        return AuthorizationMetadata {
            action: if id == "registry.refresh@1" {
                AuthorizationAction::Refresh
            } else {
                AuthorizationAction::Read
            },
            resource: AuthorizationResource::Registry,
        };
    }
    if id.starts_with("system.") {
        return AuthorizationMetadata {
            action: AuthorizationAction::Read,
            resource: AuthorizationResource::System,
        };
    }
    if id.starts_with("schemas.") {
        return AuthorizationMetadata {
            action: AuthorizationAction::Read,
            resource: AuthorizationResource::SchemaCatalog,
        };
    }
    if id.starts_with("facets.state.") {
        return AuthorizationMetadata {
            action: if id.contains(".stream@") {
                AuthorizationAction::Stream
            } else {
                AuthorizationAction::Read
            },
            resource: AuthorizationResource::FacetState,
        };
    }
    if id.starts_with("facets.events.") {
        return AuthorizationMetadata {
            action: AuthorizationAction::Stream,
            resource: AuthorizationResource::FacetEvent,
        };
    }
    if id.starts_with("facets.") {
        return AuthorizationMetadata {
            action: AuthorizationAction::Read,
            resource: AuthorizationResource::Facet,
        };
    }

    let resource = if id.starts_with("instances.") {
        if id.contains(".facets.") {
            AuthorizationResource::InstanceFacet
        } else {
            AuthorizationResource::Instance
        }
    } else if id.contains(".run") || id.contains(".runs.") {
        AuthorizationResource::RuleRun
    } else if id.contains(".revision.") {
        AuthorizationResource::RuleRevision
    } else {
        AuthorizationResource::Rule
    };
    let action = if [".list@", ".get@", ".page@"]
        .iter()
        .any(|part| id.contains(part))
    {
        AuthorizationAction::Read
    } else if id.contains(".validate@") {
        AuthorizationAction::Validate
    } else if id.contains(".compile@") {
        AuthorizationAction::Compile
    } else if id.contains(".simulate@") {
        AuthorizationAction::Simulate
    } else if id.contains(".create@") {
        AuthorizationAction::Create
    } else if id.contains(".update@") {
        AuthorizationAction::Update
    } else if id.contains(".activate@") {
        AuthorizationAction::Activate
    } else if id.contains(".rollback@") {
        AuthorizationAction::Rollback
    } else if id.contains(".enable@") {
        AuthorizationAction::Enable
    } else if id.contains(".disable@") {
        AuthorizationAction::Disable
    } else if id.contains(".delete@") {
        AuthorizationAction::Delete
    } else if id.contains(".trigger@") {
        AuthorizationAction::Trigger
    } else if id.contains(".cancel@") {
        AuthorizationAction::Cancel
    } else {
        unreachable!("closed operation catalog was validated before metadata")
    };
    AuthorizationMetadata { action, resource }
}

fn expected_resource_template(id: &str) -> Option<&'static str> {
    match id {
        "system.get@1" => Some("litehouse://server"),
        "instances.get@1" => Some("litehouse://instances/{instance-id}"),
        "facets.get@1" => Some("litehouse://facets/{facet-id}"),
        "facets.state.snapshot@1" => Some("litehouse://facets/{facet-id}/state"),
        "rules.revision.get@1" => Some("litehouse://rules/{rule-id}/revisions/{revision}"),
        "rules.run.get@1" => Some("litehouse://rules/{rule-id}/runs/{run-id}"),
        "operations.result.get@1" => {
            Some("litehouse://operations/{operation-id}/{idempotency-key}")
        }
        "registry.release.get@1" => Some("litehouse://registry/{package}/{version}"),
        "schemas.get@1" => Some("litehouse://schemas/{schema-id}/{version}"),
        _ => None,
    }
}

fn expected_web_group(id: &str) -> WebGroup {
    if id.starts_with("capability.") {
        WebGroup::Capabilities
    } else if id.starts_with("system.") {
        WebGroup::System
    } else if id.starts_with("schemas.") {
        WebGroup::Schemas
    } else if id.starts_with("instances.") {
        WebGroup::Instances
    } else if id.starts_with("facets.") {
        WebGroup::Facets
    } else if id.starts_with("rules.") {
        WebGroup::Rules
    } else if id.starts_with("registry.") {
        WebGroup::Registry
    } else if id.starts_with("plugins.") {
        WebGroup::Plugins
    } else if id.starts_with("operations.") {
        WebGroup::Operations
    } else {
        WebGroup::Diagnostics
    }
}

const fn canonical_projection_policy() -> ProjectionPolicy {
    ProjectionPolicy {
        input: ProjectionMode::SchemaAllowlist,
        output: ProjectionMode::SchemaAllowlist,
        unknown_fields: UnknownInputPolicy::Deny,
        protected_values: ProtectedValuePolicy::Omit,
        private_addresses: PrivateAddressPolicy::Omit,
        errors: ErrorProjection::SafeEnvelope,
    }
}

const fn canonical_redaction_policy() -> RedactionPolicy {
    RedactionPolicy {
        unknown_fields: UnknownOutputPolicy::Hide,
        protected_values: ProtectedValuePolicy::Omit,
        private_addresses: PrivateAddressPolicy::Omit,
        untrusted_text: UntrustedTextPolicy::PreserveMarked,
        diagnostics: DiagnosticProjection::Allowlisted,
    }
}

fn valid_operation_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 160
        && name.split('.').all(|part| {
            !part.is_empty()
                && !part.starts_with('-')
                && !part.ends_with('-')
                && !part.contains("--")
                && part
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        })
}

fn contains_escape_hatch(name: &str) -> bool {
    name.split(['.', '-']).any(|part| {
        matches!(
            part,
            "admin"
                | "generic"
                | "shell"
                | "filesystem"
                | "database"
                | "container"
                | "network"
                | "http"
                | "request"
                | "method"
        )
    })
}

fn protected_or_escape_field(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.contains("password")
        || lower.contains("secret")
        || lower.contains("credential")
        || lower.contains("address")
        || lower.contains("endpoint")
        || lower.contains("private_key")
        || lower.contains("access_token")
        || lower.contains("refresh_token")
        || lower.contains("authorization")
        || lower.contains("cookie")
        || matches!(
            lower.as_str(),
            "url" | "uri" | "host" | "port" | "path" | "method" | "headers" | "body" | "sql"
        )
}

fn operation_error(operation: &ManagementOperation, message: &str) -> String {
    format!("{}: {message}", operation.id)
}

fn require(condition: bool, message: impl Into<String>) -> Result<(), ManifestError> {
    if condition {
        Ok(())
    } else {
        Err(ManifestError::Invalid(message.into()))
    }
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
