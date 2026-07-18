use litehouse_contracts::management::{
    CANONICAL_OPERATION_COUNT, CAPABILITY_OPERATION_COUNT, FIXED_WIRE_SCOPES,
    MANAGEMENT_OPERATION_COUNT, MANAGEMENT_OPERATIONS_V1_JSON, MCP_PROVING_TOOL_BINDINGS,
    ManagementManifest, OAuthScope, OperationClass, OperationKind, ScopePolicy,
    management_manifest, schema_sha256,
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::PathBuf};
use wit_parser::{Resolve, UnresolvedPackageGroup};

fn manifest_value() -> Value {
    serde_json::from_slice(MANAGEMENT_OPERATIONS_V1_JSON).unwrap()
}

fn parse_value(
    value: &Value,
) -> Result<ManagementManifest, litehouse_contracts::management::ManifestError> {
    ManagementManifest::parse(&serde_json::to_vec(value).unwrap())
}

fn operation_mut<'a>(manifest: &'a mut Value, id: &str) -> &'a mut Value {
    manifest["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|operation| operation["id"] == id)
        .unwrap()
}

fn property_names(schema: &Value, names: &mut Vec<String>) {
    match schema {
        Value::Object(object) => {
            if object.get("type").and_then(Value::as_str) == Some("object") {
                names.extend(
                    object
                        .get("properties")
                        .and_then(Value::as_object)
                        .unwrap()
                        .keys()
                        .cloned(),
                );
            }
            for child in object.values() {
                property_names(child, names);
            }
        }
        Value::Array(values) => {
            for child in values {
                property_names(child, names);
            }
        }
        _ => {}
    }
}

#[test]
fn embedded_manifest_is_authoritative_closed_inventory() {
    let manifest = management_manifest().unwrap();
    assert_eq!(manifest.operations.len(), MANAGEMENT_OPERATION_COUNT);
    assert_eq!(
        manifest
            .operations
            .iter()
            .filter(|operation| operation.kind == OperationKind::Canonical)
            .count(),
        CANONICAL_OPERATION_COUNT
    );
    assert_eq!(
        manifest
            .operations
            .iter()
            .filter(|operation| operation.kind == OperationKind::Capability)
            .count(),
        CAPABILITY_OPERATION_COUNT
    );
    assert!(
        manifest
            .operations
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
    );
}

#[test]
fn every_capability_projection_targets_a_real_wit_operation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packages");
    let mut package_roots = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path().join("1.0.0/wit"))
        .collect::<Vec<_>>();
    package_roots.sort();
    let groups = package_roots
        .iter()
        .map(|path| UnresolvedPackageGroup::parse_dir(path).unwrap())
        .collect::<Vec<_>>();

    for operation in management_manifest()
        .unwrap()
        .operations
        .iter()
        .filter(|operation| operation.kind == OperationKind::Capability)
    {
        let binding = operation.service_binding.as_ref().unwrap();
        let source_index = groups
            .iter()
            .position(|group| group.main.name.to_string() == binding.package)
            .unwrap_or_else(|| panic!("{} targets missing {}", operation.id, binding.package));
        let mut resolve = Resolve::default();
        let package_id = resolve
            .push_groups(
                groups[source_index].clone(),
                groups
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| *index != source_index)
                    .map(|(_, group)| group.clone())
                    .collect(),
            )
            .unwrap();
        let interface_id = resolve.packages[package_id]
            .interfaces
            .get(&binding.interface)
            .unwrap_or_else(|| {
                panic!(
                    "{} targets missing {}/{}",
                    operation.id, binding.package, binding.interface
                )
            });
        assert!(
            resolve.interfaces[*interface_id]
                .functions
                .contains_key(&binding.operation),
            "{} targets missing {}/{}/{}",
            operation.id,
            binding.package,
            binding.interface,
            binding.operation
        );
    }
}

#[test]
fn fixed_scope_vocabulary_is_complete_but_admin_grants_no_operation() {
    let manifest = management_manifest().unwrap();
    assert_eq!(
        manifest.fixed_scopes.as_slice(),
        FIXED_WIRE_SCOPES.as_slice()
    );

    let used = manifest
        .operations
        .iter()
        .filter_map(|operation| match &operation.scope_policy {
            ScopePolicy::Fixed { scope } => Some(*scope),
            ScopePolicy::ReferencedOperation { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        used,
        BTreeSet::from([
            OAuthScope::Read,
            OAuthScope::Operate,
            OAuthScope::Configure,
            OAuthScope::Install,
            OAuthScope::Diagnostics,
        ])
    );
    assert!(!used.contains(&OAuthScope::Admin));
    assert_eq!(
        manifest
            .operations
            .iter()
            .filter(|operation| matches!(
                &operation.scope_policy,
                ScopePolicy::ReferencedOperation { .. }
            ))
            .map(|operation| operation.id.as_str())
            .collect::<Vec<_>>(),
        ["operations.result.get@1"]
    );
}

#[test]
fn bindings_are_exact_derived_and_reads_are_no_store() {
    let manifest = management_manifest().unwrap();
    for operation in &manifest.operations {
        let (name, major) = operation.id.rsplit_once('@').unwrap();
        assert_eq!(
            operation.http.path,
            format!("/api/operations/{name}/v{major}")
        );
        assert_eq!(
            operation.webrtc.operation_id.as_str(),
            operation.id.as_str()
        );
        let expected_tool = MCP_PROVING_TOOL_BINDINGS
            .iter()
            .find(|binding| binding.operation_id == operation.id)
            .map_or_else(
                || format!("litehouse_{}_v{major}", name.replace(['.', '-'], "_")),
                |binding| binding.tool.to_owned(),
            );
        assert_eq!(operation.mcp.tool, expected_tool);
        if operation.class == OperationClass::Read {
            assert_eq!(
                serde_json::to_value(operation.http.cache).unwrap(),
                "no-store"
            );
            assert_eq!(
                serde_json::to_value(operation.web.cache).unwrap(),
                "no-store"
            );
        }
    }
}

#[test]
fn proving_mcp_surface_is_exactly_three_closed_tools() {
    assert_eq!(
        MCP_PROVING_TOOL_BINDINGS
            .iter()
            .map(|binding| (binding.tool, binding.operation_id))
            .collect::<Vec<_>>(),
        [
            ("litehouse_inventory", "inventory.snapshot@1"),
            ("litehouse_query", "facets.query@1"),
            ("litehouse_interact", "capability.on-off.set@1"),
        ]
    );

    let manifest = management_manifest().unwrap();
    for binding in MCP_PROVING_TOOL_BINDINGS {
        let operation = manifest
            .operations
            .iter()
            .find(|operation| operation.id == binding.operation_id)
            .unwrap();
        assert_eq!(operation.mcp.tool, binding.tool);
        assert!(!operation.mcp.annotations.open_world);
    }
}

#[test]
fn proving_mcp_schemas_are_closed_and_typed() {
    let manifest = management_manifest().unwrap();
    let operation = |id: &str| {
        manifest
            .operations
            .iter()
            .find(|operation| operation.id == id)
            .unwrap()
    };

    let inventory = operation("inventory.snapshot@1");
    assert!(
        inventory.schemas.input.schema["properties"]
            .as_object()
            .unwrap()
            .is_empty()
    );
    let facet_properties = &inventory.schemas.output.schema["properties"]["instances"]["items"]["properties"]
        ["facets"]["items"]["properties"];
    assert_eq!(
        facet_properties
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            "available",
            "contract",
            "facet_id",
            "features",
            "generation",
            "label",
            "operations",
        ])
    );

    let query = operation("facets.query@1");
    assert_eq!(
        query.schemas.output.schema["properties"]["state"]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|schema| schema["properties"]["kind"]["const"].as_str().unwrap())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["boolean-state", "measurement", "on-off"])
    );

    let interact = operation("capability.on-off.set@1");
    assert_eq!(
        interact.schemas.input.schema["required"],
        json!(["facet_id", "facet_generation", "on"])
    );
    assert_eq!(
        interact.schemas.mcp_projection.schema["required"],
        json!(["facet_generation", "facet_id", "idempotency_key", "on"])
    );
}

#[test]
fn inventory_has_no_admin_or_generic_escape_hatch() {
    let manifest = management_manifest().unwrap();
    for operation in &manifest.operations {
        let segments = operation.id.split(['.', '-', '@']).collect::<Vec<_>>();
        for forbidden in [
            "admin",
            "generic",
            "shell",
            "filesystem",
            "database",
            "container",
            "network",
            "http",
            "request",
            "method",
        ] {
            assert!(
                !segments.contains(&forbidden),
                "{} contains {forbidden}",
                operation.id
            );
        }
        assert!(!operation.mcp.annotations.open_world);
    }
}

#[test]
fn mcp_projection_schemas_omit_secret_and_address_fields() {
    let manifest = management_manifest().unwrap();
    for operation in &manifest.operations {
        let mut names = Vec::new();
        property_names(&operation.schemas.mcp_projection.schema, &mut names);
        for name in names {
            let name = name.to_ascii_lowercase();
            assert!(!name.contains("secret"), "{} exposes {name}", operation.id);
            assert!(
                !name.contains("password"),
                "{} exposes {name}",
                operation.id
            );
            assert!(
                !name.contains("credential"),
                "{} exposes {name}",
                operation.id
            );
            assert!(!name.contains("address"), "{} exposes {name}", operation.id);
            assert!(
                !name.contains("endpoint"),
                "{} exposes {name}",
                operation.id
            );
            assert!(
                !name.contains("access_token"),
                "{} exposes {name}",
                operation.id
            );
            assert!(
                !name.contains("refresh_token"),
                "{} exposes {name}",
                operation.id
            );
        }
    }
}

#[test]
fn unknown_manifest_and_row_fields_are_denied() {
    let mut manifest = manifest_value();
    manifest["escape"] = json!(true);
    assert!(parse_value(&manifest).is_err());

    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "system.get@1")["alias"] = json!("system");
    assert!(parse_value(&manifest).is_err());
}

#[test]
fn changed_id_scope_and_class_are_denied() {
    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "system.get@1")["id"] = json!("admin.call@1");
    assert!(parse_value(&manifest).is_err());

    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "instances.validate@1")["scope_policy"]["scope"] =
        json!("litehouse:admin");
    assert!(parse_value(&manifest).is_err());

    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "instances.draft.create@1")["class"] = json!("pure");
    assert!(parse_value(&manifest).is_err());
}

#[test]
fn changed_transport_derivations_are_denied() {
    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "facets.get@1")["http"]["path"] = json!("/api/facets");
    assert!(parse_value(&manifest).is_err());

    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "facets.get@1")["webrtc"]["operation_id"] = json!("facets.get@2");
    assert!(parse_value(&manifest).is_err());

    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "facets.get@1")["mcp"]["tool"] = json!("litehouse_call_v1");
    assert!(parse_value(&manifest).is_err());
}

#[test]
fn mutation_idempotency_and_preconditions_are_denied_when_weakened() {
    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "capability.on-off.set@1")["idempotency"] = json!("not-required");
    assert!(parse_value(&manifest).is_err());

    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "instances.draft.update@1")["preconditions"]["revision"] =
        json!("none");
    assert!(parse_value(&manifest).is_err());

    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "facets.state.snapshot@1")["preconditions"]["generation"] =
        json!("none");
    assert!(parse_value(&manifest).is_err());
}

#[test]
fn deadlines_match_the_frozen_read_mutation_and_job_policy() {
    let manifest = management_manifest().unwrap();
    let deadline = |id: &str| {
        manifest
            .operations
            .iter()
            .find(|operation| operation.id == id)
            .unwrap()
            .deadline
    };
    assert_eq!(
        (
            deadline("system.get@1").default_ms,
            deadline("system.get@1").maximum_ms
        ),
        (10_000, 30_000)
    );
    assert_eq!(
        (
            deadline("rules.validate@1").default_ms,
            deadline("rules.validate@1").maximum_ms
        ),
        (30_000, 30_000)
    );
    assert_eq!(
        (
            deadline("capability.on-off.set@1").default_ms,
            deadline("capability.on-off.set@1").maximum_ms
        ),
        (30_000, 30_000)
    );
    assert_eq!(
        (
            deadline("plugins.install.apply@1").default_ms,
            deadline("plugins.install.apply@1").maximum_ms
        ),
        (30_000, 300_000)
    );
}

#[test]
fn invalid_deadline_and_cache_policy_are_denied() {
    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "rules.simulate@1")["deadline"]["default_ms"] = json!(120001);
    assert!(parse_value(&manifest).is_err());

    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "system.health.get@1")["http"]["cache"] = json!("shared");
    assert!(parse_value(&manifest).is_err());
}

#[test]
fn schema_digest_changes_and_open_schemas_are_denied() {
    let mut manifest = manifest_value();
    operation_mut(&mut manifest, "rules.validate@1")["schemas"]["input"]["schema"]["properties"]
        ["source"]["maxLength"] = json!(1);
    assert!(parse_value(&manifest).is_err());

    let mut manifest = manifest_value();
    let operation = operation_mut(&mut manifest, "system.get@1");
    operation["schemas"]["input"]["schema"]["additionalProperties"] = json!(true);
    let digest = schema_sha256(&operation["schemas"]["input"]["schema"]).unwrap();
    operation["schemas"]["input"]["sha256"] = json!(digest);
    assert!(parse_value(&manifest).is_err());
}

#[test]
fn protected_field_is_denied_even_with_a_recomputed_digest() {
    let mut manifest = manifest_value();
    let operation = operation_mut(&mut manifest, "instances.draft.update@1");
    let schema = &mut operation["schemas"]["mcp_projection"]["schema"];
    schema["properties"]["private_address"] = json!({"type": "string", "maxLength": 256});
    let digest = schema_sha256(schema).unwrap();
    operation["schemas"]["mcp_projection"]["sha256"] = json!(digest);
    assert!(parse_value(&manifest).is_err());
}
