use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use jsonschema::{Draft, JSONSchema};
use litehouse_contracts::{
    CONTRACT_ARTIFACT_MANIFEST,
    management::{CAPABILITY_OPERATION_IDS, MANAGEMENT_OPERATIONS_V1_JSON, management_manifest},
    quantity::{Quantity, profile_ids},
};
use litehouse_rule_json::{
    AssumptionError, CompileError, ContractRef, EvaluationErrorCode, EvaluationInput, Freshness,
    InputError, ObservationValue, ObservedRevision, PlanError, PlanOutcome, QuantityCatalogRef,
    QuantityProfileRef, RebindPolicy, RuleDocument, RuleJsonError, ScalarValue, Selector,
    SelectorCardinality, SnapshotInput, StateObservationInput, StatefulContractKind, TargetInput,
    Timestamp, TypeRef, UnknownReason, compile_rule_json, plan,
    schema::{
        CHECKED_EVALUATION_ARTIFACTS, CHECKED_FIXTURE_INVENTORY, CHECKED_FIXTURES,
        CHECKED_PORT_CATALOG, CHECKED_RULE_SCHEMA, evaluation_profile_sha256,
        generated_evaluation_artifacts_bytes, generated_fixture_artifacts,
        generated_fixture_inventory_bytes, generated_port_catalog_bytes,
        generated_rule_schema_bytes, schema_sha256,
    },
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

const RULE_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn planner_bytes() -> &'static [u8] {
    include_bytes!("../fixtures/valid-planner-compat.json")
}

fn fixture(path: &str) -> Value {
    let fixture = CHECKED_FIXTURES
        .iter()
        .find(|fixture| fixture.path == path)
        .unwrap_or_else(|| panic!("missing fixture {path}"));
    serde_json::from_slice(fixture.bytes).expect("fixture JSON")
}

fn planner_fixture() -> Value {
    serde_json::from_slice(planner_bytes()).expect("planner fixture JSON")
}

fn trigger_selector() -> Selector {
    selector(
        "11111111111111111111111111111111",
        "source_switch",
        "on-off",
    )
}

fn target_selector() -> Selector {
    selector("22222222222222222222222222222222", "target_light", "on-off")
}

fn selector(instance: &str, facet: &str, package: &str) -> Selector {
    Selector {
        provider_instance_id: instance.to_owned(),
        facet_key: facet.to_owned(),
        contract: ContractRef::litehouse(package, "capability", 1),
        cardinality: SelectorCardinality::One,
        rebind: RebindPolicy::SameIdentity,
    }
}

fn input(value: ObservationValue) -> EvaluationInput {
    input_for_snapshot(value, trigger_selector(), "on_off_current")
}

fn input_for_snapshot(
    value: ObservationValue,
    snapshot_selector: Selector,
    field: &str,
) -> EvaluationInput {
    let time = Timestamp::new(1_721_234_567, 123_000_000).unwrap();
    EvaluationInput::new(
        RULE_ID,
        7,
        evaluation_profile_sha256(),
        time,
        StateObservationInput::new(trigger_selector(), 3, 40, time).unwrap(),
        vec![
            SnapshotInput::new(
                "snapshot",
                snapshot_selector,
                field,
                3,
                40,
                time,
                Freshness::Fresh,
                value,
            )
            .unwrap(),
        ],
        vec![TargetInput::new(target_selector(), 8, 12, 5, true).unwrap()],
    )
    .unwrap()
}

fn digest(source: &str) -> String {
    digest_bytes(source.as_bytes())
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn lock_name(artifact_id: &str) -> &'static str {
    match artifact_id {
        "litehouse:rule-compiler@1" => "compiler",
        "litehouse-cel@1" => "cel",
        "litehouse:quantity-catalog@1" => "quantity_catalog",
        "litehouse:rule-runtime@1" => "runtime_profile",
        "jiff-tzdb@0.1.8+2026c" => "timezone_database",
        "litehouse:contract-catalog@1" => "contract_catalog",
        "litehouse:management-operations@1" => "management_operations",
        "https://schemas.litehouse.dev/rule-ir/v1.0.0/node-port-catalog.json" => "port_catalog",
        _ => panic!("unknown artifact {artifact_id}"),
    }
}

fn validator() -> JSONSchema {
    let schema: Value = serde_json::from_slice(CHECKED_RULE_SCHEMA).unwrap();
    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .unwrap()
}

fn compile_value(value: &Value) -> Result<litehouse_rule_json::CompiledRule, RuleJsonError> {
    compile_rule_json(&serde_json::to_vec(value).unwrap())
}

fn assert_semantically_valid(value: &Value) {
    match compile_value(value) {
        Ok(_) | Err(RuleJsonError::Compile(CompileError::PlannerSubsetUnsupported { .. })) => {}
        result => panic!("expected semantic success or bounded subset diagnostic, got {result:?}"),
    }
}

fn compile_error(value: &Value) -> CompileError {
    match compile_value(value) {
        Err(RuleJsonError::Compile(error)) => error,
        result => panic!("expected compile error, got {result:?}"),
    }
}

#[test]
fn generated_schema_catalog_and_release_fixtures_are_byte_deterministic() {
    assert_eq!(generated_rule_schema_bytes(), CHECKED_RULE_SCHEMA);
    assert_eq!(generated_port_catalog_bytes(), CHECKED_PORT_CATALOG);
    assert_eq!(
        generated_evaluation_artifacts_bytes(),
        CHECKED_EVALUATION_ARTIFACTS
    );
    assert_eq!(
        generated_fixture_inventory_bytes(),
        CHECKED_FIXTURE_INVENTORY
    );
    let generated = generated_fixture_artifacts();
    assert!(generated.len() > CHECKED_FIXTURES.len());
    for (generated, checked) in generated
        .iter()
        .take(CHECKED_FIXTURES.len())
        .zip(CHECKED_FIXTURES)
    {
        assert_eq!(generated.path, checked.path);
        assert_eq!(generated.schema_valid, checked.schema_valid);
        assert_eq!(generated.semantic_valid, checked.semantic_valid);
        assert_eq!(generated.bytes, checked.bytes, "{}", generated.path);
    }
    let inventory: Value = serde_json::from_slice(CHECKED_FIXTURE_INVENTORY).unwrap();
    assert_eq!(inventory["version"], 1);
    assert_eq!(
        inventory["fixtures"].as_array().unwrap().len(),
        generated.len()
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let validator = validator();
    for (generated, indexed) in generated
        .iter()
        .zip(inventory["fixtures"].as_array().unwrap())
    {
        let checked = fs::read(root.join(&generated.path)).unwrap();
        assert_eq!(generated.bytes, checked, "{}", generated.path);
        assert_eq!(indexed["path"], generated.path);
        assert_eq!(indexed["schema_valid"], generated.schema_valid);
        assert_eq!(indexed["semantic_valid"], generated.semantic_valid);
        assert_eq!(indexed["sha256"], digest_bytes(&checked));
        let value: Value = serde_json::from_slice(&checked).unwrap();
        if generated.schema_valid && !generated.semantic_valid {
            // Recursive semantic-limit fixtures intentionally pass the wire shape. Running them
            // through jsonschema 0.17's Draft 2020-12 unevaluated-properties traversal expands
            // recursive unions pathologically; strict deserialization proves shape parity while
            // compile_rule_json below proves the semantic rejection they were authored to cover.
            assert!(
                serde_json::from_value::<RuleDocument>(value.clone()).is_ok(),
                "{}",
                generated.path
            );
            assert!(compile_value(&value).is_err(), "{}", generated.path);
        } else {
            assert_eq!(
                validator.is_valid(&value),
                generated.schema_valid,
                "{}",
                generated.path
            );
            if generated.semantic_valid {
                assert_semantically_valid(&value);
            }
        }
    }

    let schema: Value = serde_json::from_slice(CHECKED_RULE_SCHEMA).unwrap();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(
        schema["$id"],
        "https://schemas.litehouse.dev/rule-ir/v1.0.0/schema.json"
    );
    assert_eq!(planner_fixture()["schema"]["sha256"], schema_sha256());

    for fixture in CHECKED_FIXTURES {
        let value: Value = serde_json::from_slice(fixture.bytes).unwrap();
        assert_eq!(
            validator.is_valid(&value),
            fixture.schema_valid,
            "{}",
            fixture.path
        );
        if fixture.semantic_valid {
            assert_semantically_valid(&value);
            assert!(!value["semantic"]["nodes"].as_array().unwrap().is_empty());
            assert!(!value["semantic"]["edges"].as_array().unwrap().is_empty());
        }
    }
}

#[test]
fn ordinary_schema_objects_are_closed_and_only_bounded_maps_are_open() {
    fn inspect(value: &Value, path: &str) {
        match value {
            Value::Object(object) => {
                if object.get("type").and_then(Value::as_str) == Some("object") {
                    let map = object.contains_key("patternProperties");
                    assert_eq!(
                        object.get("unevaluatedProperties"),
                        Some(&json!(false)),
                        "{path}"
                    );
                    if map {
                        assert!(
                            object.get("maxProperties").is_some(),
                            "unbounded map {path}"
                        );
                    } else {
                        assert_eq!(
                            object.get("additionalProperties"),
                            Some(&json!(false)),
                            "{path}"
                        );
                    }
                }
                for (key, child) in object {
                    inspect(child, &format!("{path}/{key}"));
                }
            }
            Value::Array(values) => {
                for (index, child) in values.iter().enumerate() {
                    inspect(child, &format!("{path}/{index}"));
                }
            }
            _ => {}
        }
    }
    inspect(&serde_json::from_slice(CHECKED_RULE_SCHEMA).unwrap(), "$");
}

#[test]
fn published_vocabulary_is_exactly_three_triggers_and_eleven_node_families() {
    let schema: Value = serde_json::from_slice(CHECKED_RULE_SCHEMA).unwrap();
    let node_kinds = schema["$defs"]["Node"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| variant["properties"]["kind"]["const"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        node_kinds,
        BTreeSet::from([
            "snapshot",
            "condition",
            "quantity",
            "branch",
            "desired_patch",
            "effect_intent",
            "delay",
            "debounce_throttle",
            "fork_join",
            "receipt_branch",
            "end",
        ])
    );
    assert!(!node_kinds.contains("compare"));
    assert!(!node_kinds.contains("on_off_set"));

    let trigger_kinds = schema["$defs"]["TriggerNode"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| variant["properties"]["kind"]["const"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        trigger_kinds,
        BTreeSet::from(["state_observation", "discrete_event", "time_schedule"])
    );

    let operation_ids = schema["$defs"]["CapabilityOperation"]["oneOf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|variant| {
            variant["properties"]["operation"]["const"]
                .as_str()
                .unwrap()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        operation_ids,
        CAPABILITY_OPERATION_IDS
            .into_iter()
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn ports_are_closed_connectable_and_receipts_are_exactly_parameterized() {
    let catalog: Value = serde_json::from_slice(CHECKED_PORT_CATALOG).unwrap();
    assert_eq!(catalog["entries"].as_array().unwrap().len(), 14);
    let entry = |kind: &str| {
        catalog["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["kind"] == kind)
            .unwrap()
    };
    let effect = entry("effect_intent");
    let effect_receipt = effect["ports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|port| port["name"] == "receipt")
        .unwrap();
    assert_eq!(effect_receipt["type_ref"]["kind"], "capability_receipt");
    assert_eq!(
        effect_receipt["type_ref"]["operation_property"],
        "operation"
    );
    assert_eq!(effect_receipt["cardinality"], "zero_or_one");
    assert!(
        effect["ports"]
            .as_array()
            .unwrap()
            .iter()
            .all(|port| port["name"] != "next")
    );

    let receipt_branch = entry("receipt_branch");
    let receipt_input = receipt_branch["ports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|port| port["name"] == "receipt")
        .unwrap();
    assert_eq!(
        receipt_input["type_ref"]["kind"],
        "referenced_effect_receipt"
    );
    assert_eq!(
        receipt_input["type_ref"]["effect_node_property"],
        "effect_node_id"
    );
    assert_eq!(receipt_input["cardinality"], "exactly_one");
    assert!(
        receipt_branch["ports"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|port| port["direction"] == "output")
            .all(|port| port["cardinality"] == "zero_or_one")
    );

    let condition = entry("condition");
    assert_eq!(condition["dynamic_ports"][0]["kind"], "condition_inputs");
    assert!(condition["ports"].as_array().unwrap().iter().any(|port| {
        port["name"] == "result" && port["type_ref"]["type_ref"]["kind"] == "predicate_outcome"
    }));
    let branch = entry("branch");
    assert!(branch["ports"].as_array().unwrap().iter().any(|port| {
        port["name"] == "predicate" && port["type_ref"]["type_ref"]["kind"] == "predicate_outcome"
    }));
    let fork_join = entry("fork_join");
    assert!(fork_join["ports"].as_array().unwrap().is_empty());
    assert_eq!(fork_join["dynamic_ports"][0]["kind"], "fork_join_phases");
    assert_eq!(fork_join["dynamic_ports"][0]["fork_input"], "in");
    assert_eq!(fork_join["dynamic_ports"][0]["join_output"], "next");

    let text = String::from_utf8(CHECKED_PORT_CATALOG.to_vec()).unwrap();
    assert!(!text.contains("operation_id"));
    assert!(!text.contains("receipt_outcome"));
    assert!(!text.contains("\"changed\""));
}

#[test]
fn every_catalog_property_path_resolves_and_dynamic_roles_are_explicit() {
    fn expand<'a>(root: &'a Value, value: &'a Value) -> Vec<&'a Value> {
        if let Some(reference) = value.get("$ref").and_then(Value::as_str) {
            return expand(
                root,
                &root["$defs"][reference.strip_prefix("#/$defs/").unwrap()],
            );
        }
        if let Some(variants) = value.get("oneOf").and_then(Value::as_array) {
            return variants
                .iter()
                .flat_map(|variant| expand(root, variant))
                .collect();
        }
        vec![value]
    }

    fn resolve_path<'a>(root: &'a Value, start: &'a Value, path: &str) -> Option<Vec<&'a Value>> {
        let mut current = vec![start];
        for segment in path.split('.') {
            let mut next = Vec::new();
            for schema in current {
                for schema in expand(root, schema) {
                    next.push(schema.get("properties")?.get(segment)?);
                }
            }
            current = next;
        }
        Some(
            current
                .into_iter()
                .flat_map(|schema| expand(root, schema))
                .collect(),
        )
    }

    fn assert_item_path(root: &Value, variant: &Value, source_path: &str, item_path: &str) {
        let arrays = resolve_path(root, variant, source_path)
            .unwrap_or_else(|| panic!("unresolved catalog source path {source_path}"));
        for array in arrays {
            let items = array
                .get("items")
                .unwrap_or_else(|| panic!("catalog source path {source_path} is not an array"));
            assert!(
                resolve_path(root, items, item_path).is_some(),
                "unresolved item path {source_path}[].{item_path}",
            );
        }
    }

    fn assert_type_paths(root: &Value, variant: &Value, expression: &Value) {
        for property in [
            "field_property",
            "profile_property",
            "operation_property",
            "effect_node_property",
            "trigger_property",
        ] {
            if let Some(path) = expression.get(property).and_then(Value::as_str) {
                assert!(
                    resolve_path(root, variant, path).is_some(),
                    "unresolved {property}={path}",
                );
            }
        }
    }

    let schema: Value = serde_json::from_slice(CHECKED_RULE_SCHEMA).unwrap();
    let catalog: Value = serde_json::from_slice(CHECKED_PORT_CATALOG).unwrap();
    for entry in catalog["entries"].as_array().unwrap() {
        let definition = if entry["class"] == "trigger" {
            "TriggerNode"
        } else {
            "Node"
        };
        let variant = schema["$defs"][definition]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|variant| variant["properties"]["kind"]["const"] == entry["kind"])
            .unwrap();
        for port in entry["ports"].as_array().unwrap() {
            assert_type_paths(&schema, variant, &port["type_ref"]);
        }
        for field in entry["runtime_state"]["fields"].as_array().unwrap() {
            assert_type_paths(&schema, variant, &field["type_ref"]);
        }
        for dynamic in entry["dynamic_ports"].as_array().unwrap() {
            assert!(
                resolve_path(
                    &schema,
                    variant,
                    dynamic["source_property"].as_str().unwrap(),
                )
                .is_some()
            );
            match dynamic["kind"].as_str().unwrap() {
                "condition_inputs" => {
                    assert_item_path(
                        &schema,
                        variant,
                        dynamic["source_property"].as_str().unwrap(),
                        dynamic["name_property"].as_str().unwrap(),
                    );
                    assert_item_path(
                        &schema,
                        variant,
                        dynamic["source_property"].as_str().unwrap(),
                        dynamic["type_property"].as_str().unwrap(),
                    );
                }
                "fork_join_phases" => {
                    assert!(
                        resolve_path(
                            &schema,
                            variant,
                            dynamic["phase_property"].as_str().unwrap(),
                        )
                        .is_some()
                    );
                    assert_item_path(
                        &schema,
                        variant,
                        dynamic["source_property"].as_str().unwrap(),
                        dynamic["name_property"].as_str().unwrap(),
                    );
                    for (role, direction, cardinality) in [
                        ("fork_input", "input", "one_or_more"),
                        ("fork_output", "output", "zero_or_one"),
                        ("join_input", "input", "exactly_one"),
                        ("join_output", "output", "zero_or_one"),
                    ] {
                        assert_eq!(dynamic[format!("{role}_direction")], direction);
                        assert_eq!(dynamic[format!("{role}_cardinality")], cardinality);
                    }
                }
                kind => panic!("unknown dynamic catalog declaration {kind}"),
            }
        }
    }
}

#[test]
fn every_snapshot_projection_id_is_stable_unique_and_within_the_public_bound() {
    let catalog: Value = serde_json::from_slice(CHECKED_PORT_CATALOG).unwrap();
    let mut ids = BTreeSet::new();
    for mapping in catalog["state_fields"].as_array().unwrap() {
        let field: litehouse_rule_json::StateFieldRef =
            serde_json::from_value(mapping["field"].clone()).unwrap();
        let id = field.catalog_name();
        assert!(id.len() <= 64, "{id}");
        assert!(
            id.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                && id
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'),
            "{id}",
        );
        assert!(ids.insert(id));
    }
    assert_eq!(ids.len(), catalog["state_fields"].as_array().unwrap().len());
}

#[test]
fn receipt_graph_must_match_the_declared_effect_and_planner_suspends_before_success() {
    let mut mismatched = fixture("fixtures/valid-all-operations.json");
    let receipt = mismatched["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "receipt_01")
        .unwrap();
    receipt["effect_node_id"] = json!("effect_00");
    assert!(matches!(
        compile_error(&mismatched),
        CompileError::IncompatiblePorts { .. } | CompileError::ReceiptEffectMismatch { .. }
    ));

    let compiled = compile_rule_json(planner_bytes()).unwrap();
    let transition = plan(
        &compiled,
        &input(ObservationValue::Known {
            value: ScalarValue::Bool { value: true },
        }),
    )
    .unwrap();
    assert_eq!(transition.outcome(), PlanOutcome::AwaitingReceipt);
    assert_eq!(transition.intents().len(), 1);
    assert_eq!(transition.intents()[0].node_id, "set_light");
    assert!(!transition.visited_nodes().contains(&"receipt".to_owned()));
    assert!(
        !transition
            .visited_nodes()
            .contains(&"end_success".to_owned())
    );
}

#[test]
fn type_ref_receipts_and_contract_payloads_are_closed_enums() {
    let schema: Value = serde_json::from_slice(CHECKED_RULE_SCHEMA).unwrap();
    let serialized = serde_json::to_string(&schema["$defs"]["TypeRef"]).unwrap();
    assert!(serialized.contains("CapabilityOperationKind"));
    assert!(serialized.contains("ContractTypeRef"));
    assert!(!serialized.contains("operation_id"));
    assert!(!serialized.contains("type_name"));

    let operations = schema["$defs"]["CapabilityOperationKind"]["enum"]
        .as_array()
        .unwrap();
    assert_eq!(operations.len(), 29);
    let contract_types = schema["$defs"]["ContractTypeRef"]["enum"]
        .as_array()
        .unwrap();
    assert!(
        contract_types
            .iter()
            .any(|value| value == "litehouse:boolean-state/types@1/occurrence")
    );
}

#[test]
fn event_inventory_payloads_and_selector_pairings_match_current_wit() {
    let expected = BTreeMap::from([
        ("on-off", BTreeSet::from(["state-observation"])),
        (
            "boolean-state",
            BTreeSet::from(["state-observation", "occurrence"]),
        ),
        ("level-control", BTreeSet::from(["state-observation"])),
        ("color-control", BTreeSet::from(["state-observation"])),
        ("measurement", BTreeSet::from(["observation"])),
        ("thermostat", BTreeSet::from(["state-observation"])),
        ("window-covering", BTreeSet::from(["state-observation"])),
    ]);
    for (package, events) in &expected {
        assert_eq!(wit_enum(package, "event-kind"), *events, "{package}");
    }

    let catalog: Value = serde_json::from_slice(CHECKED_PORT_CATALOG).unwrap();
    let events = catalog["contract_events"].as_array().unwrap();
    assert_eq!(events.len(), 8);
    let occurrence = events
        .iter()
        .find(|mapping| {
            mapping["event"]["contract"] == "litehouse:boolean-state@1.0.0"
                && mapping["event"]["event"] == "occurrence"
        })
        .unwrap();
    assert_eq!(
        occurrence["selector_contract"],
        json!({
            "namespace": "litehouse",
            "package": "boolean-state",
            "interface": "capability",
            "major": 1
        })
    );
    assert_eq!(occurrence["payload_type"]["kind"], "contract");
    assert_eq!(
        occurrence["payload_type"]["type_ref"],
        "litehouse:boolean-state/types@1/occurrence"
    );

    let mut invalid = fixture("fixtures/valid-all-operations.json");
    invalid["semantic"]["trigger"]["selector"]["contract"]["package"] = json!("on-off");
    assert!(matches!(
        compile_error(&invalid),
        CompileError::EventContractMismatch
    ));
}

#[test]
fn state_field_catalog_is_faithful_to_wit_and_revision_is_u64() {
    let expected = BTreeMap::from([
        ("on-off", BTreeSet::from(["current"])),
        ("boolean-state", BTreeSet::from(["current"])),
        (
            "level-control",
            BTreeSet::from(["current-level", "target-level", "transition-remaining"]),
        ),
        (
            "color-control",
            BTreeSet::from(["current-color", "target-color", "active-model"]),
        ),
        (
            "measurement",
            BTreeSet::from(["current", "native-representation"]),
        ),
        (
            "thermostat",
            BTreeSet::from([
                "current-mode",
                "target-mode",
                "action",
                "running",
                "targets",
                "fan",
                "swing",
                "preset",
            ]),
        ),
        (
            "window-covering",
            BTreeSet::from([
                "current-position",
                "target-position",
                "movement",
                "safety-status",
                "fault-status",
            ]),
        ),
    ]);
    let catalog: Value = serde_json::from_slice(CHECKED_PORT_CATALOG).unwrap();
    let mappings = catalog["state_fields"].as_array().unwrap();
    for (package, wit_fields) in expected {
        assert_eq!(wit_enum(package, "state-field"), wit_fields, "{package}");
        let catalog_fields = mappings
            .iter()
            .filter(|mapping| mapping["wit_evidence"]["package"] == package)
            .filter_map(|mapping| {
                let field = mapping["wit_evidence"]["state_field"].as_str().unwrap();
                (!field.starts_with("observation-meta.")).then_some(field)
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(catalog_fields, wit_fields, "{package}");
    }

    let revisions = mappings
        .iter()
        .filter(|mapping| mapping["wit_evidence"]["state_field"] == "observation-meta.revision")
        .collect::<Vec<_>>();
    assert_eq!(revisions.len(), StatefulContractKind::ALL.len());
    assert!(
        revisions
            .iter()
            .all(|mapping| mapping["type_ref"]["kind"] == "u64")
    );
    assert!(mappings.iter().any(|mapping| {
        mapping["field"]["kind"] == "thermostat_current_mode"
            && mapping["type_ref"]["item"]["type_ref"] == "litehouse:thermostat/types@1/mode"
    }));
    assert!(mappings.iter().any(|mapping| {
        mapping["field"]["kind"] == "thermostat_target_mode"
            && mapping["type_ref"]["item"]["type_ref"] == "litehouse:thermostat/types@1/mode"
    }));
    assert!(mappings.iter().any(|mapping| {
        mapping["field"]["kind"] == "thermostat_targets"
            && mapping["type_ref"]["item"]["type_ref"] == "litehouse:thermostat/types@1/targets"
    }));
    assert!(mappings.iter().any(|mapping| {
        mapping["field"]["kind"] == "covering_current_position"
            && mapping["field"]["axis"] == "lift"
    }));
    assert!(mappings.iter().any(|mapping| {
        mapping["field"]["kind"] == "covering_current_position"
            && mapping["field"]["axis"] == "tilt"
    }));
}

#[test]
fn snapshot_field_must_belong_to_the_selector_contract_before_lowering() {
    let mut invalid = planner_fixture();
    invalid["semantic"]["nodes"][0]["field"] = json!({"kind": "boolean_current"});
    assert!(matches!(
        compile_error(&invalid),
        CompileError::SnapshotContractMismatch { ref node } if node == "snapshot"
    ));
}

#[test]
fn desired_patch_schema_and_semantics_enforce_target_path_and_value_compatibility() {
    let desired = fixture("fixtures/valid-all-desired-paths.json");
    let paths = desired["semantic"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["kind"] == "desired_patch")
        .flat_map(|node| node["operations"].as_array().unwrap())
        .filter_map(|operation| {
            operation
                .get("value")
                .or_else(|| operation.get("expected"))
                .and_then(|value| value["path"].as_str())
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        paths,
        BTreeSet::from([
            "/on",
            "/level",
            "/color",
            "/thermostat/mode",
            "/thermostat/single_target",
            "/thermostat/heat_target",
            "/thermostat/cool_target",
            "/thermostat/fan",
            "/thermostat/swing",
            "/thermostat/preset",
            "/covering/lift",
            "/covering/tilt",
        ])
    );

    let mut wrong_type = fixture("fixtures/valid-all-node-families.json");
    wrong_type["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "desired")
        .unwrap()["operations"][1]["value"] = json!({"path": "/level", "value": true});
    assert!(!validator().is_valid(&wrong_type));
    assert!(matches!(
        compile_value(&wrong_type),
        Err(RuleJsonError::InvalidShape(_))
    ));

    let mut missing_target = fixture("fixtures/valid-all-node-families.json");
    missing_target["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "desired")
        .unwrap()["snapshot_node_id"] = json!("missing");
    assert!(matches!(
        compile_error(&missing_target),
        CompileError::InvalidDesiredPatchTarget { .. }
    ));

    let mut wrong_contract = fixture("fixtures/valid-all-node-families.json");
    wrong_contract["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "desired")
        .unwrap()["operations"][1]["value"] = json!({"path": "/level", "value": "0.5"});
    assert!(matches!(
        compile_error(&wrong_contract),
        CompileError::DesiredPatchContractMismatch { .. }
    ));
}

#[test]
fn all_operation_templates_match_manifest_arguments_constraints_and_service_projection() {
    let catalog: Value = serde_json::from_slice(CHECKED_PORT_CATALOG).unwrap();
    let templates = catalog["capability_operations"].as_array().unwrap();
    assert_eq!(templates.len(), 29);
    let manifest = management_manifest().unwrap();
    for template in templates {
        let id = template["operation"].as_str().unwrap();
        let operation = manifest
            .operations
            .iter()
            .find(|operation| operation.id == id)
            .unwrap();
        assert_eq!(
            template["service_binding"],
            serde_json::to_value(operation.service_binding.as_ref().unwrap()).unwrap(),
            "{id}"
        );
        assert_eq!(
            template["selector_contract"]["package"],
            operation.service_binding.as_ref().unwrap().package["litehouse:".len()..]
                .split('@')
                .next()
                .unwrap(),
            "{id}"
        );
        let properties = operation.schemas.input.schema["properties"]
            .as_object()
            .unwrap();
        let required = operation.schemas.input.schema["required"]
            .as_array()
            .unwrap();
        let arguments = template["arguments"].as_array().unwrap();
        let expected_names = properties
            .keys()
            .filter(|name| !matches!(name.as_str(), "facet_id" | "facet_generation"))
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let actual_names = arguments
            .iter()
            .map(|argument| argument["name"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_names, expected_names, "{id}");
        for argument in arguments {
            let name = argument["name"].as_str().unwrap();
            assert_eq!(
                argument["canonical_schema"], properties[name],
                "{id}/{name}"
            );
            assert_eq!(
                argument["required"],
                required.iter().any(|field| field == name),
                "{id}/{name}"
            );
            assert_eq!(
                argument["type_ref"],
                expected_argument_type(id, name, &properties[name]),
                "{id}/{name}"
            );
        }
        let retry = template["retry"].as_str().unwrap();
        if never_retry_operation(id) {
            assert_eq!(retry, "never", "{id}");
        } else {
            assert_eq!(
                retry, "definitely_pre_dispatch_with_matching_provider_deduplication",
                "{id}"
            );
        }
    }
}

#[test]
fn retry_is_fail_closed_requires_matching_deduplication_and_one_deadline() {
    let mut eligible = planner_fixture();
    eligible["semantic"]["nodes"][3]["retry"] = json!({
        "kind": "definitely_pre_dispatch",
        "max_attempts": 2,
        "backoff_ms": 100,
        "provider_deduplication": "matching_provider_generation"
    });
    assert!(matches!(
        compile_error(&eligible),
        CompileError::PlannerSubsetUnsupported { ref kind, .. } if kind == "effect retry policy"
    ));

    let mut escape_hatch = eligible.clone();
    escape_hatch["semantic"]["nodes"][3]["retry"] = json!({
        "kind": "definitely_pre_dispatch",
        "max_attempts": 2,
        "backoff_ms": 100,
        "require_provider_deduplication": false
    });
    assert!(!validator().is_valid(&escape_hatch));
    assert!(matches!(
        compile_value(&escape_hatch),
        Err(RuleJsonError::InvalidShape(_))
    ));

    let mut never = planner_fixture();
    never["semantic"]["nodes"][3]["operation"] = json!({"operation": "capability.on-off.toggle@1"});
    never["semantic"]["nodes"][3]["retry"] = eligible["semantic"]["nodes"][3]["retry"].clone();
    assert!(matches!(
        compile_error(&never),
        CompileError::InvalidRetryPolicy { .. }
    ));

    let mut deadline = eligible;
    deadline["semantic"]["nodes"][3]["deadline_ms"] = json!(100);
    deadline["semantic"]["nodes"][3]["retry"]["backoff_ms"] = json!(100);
    assert!(matches!(
        compile_error(&deadline),
        CompileError::InvalidRetryPolicy { .. }
    ));
}

#[test]
fn debounce_retained_state_is_parameterized_for_all_three_trigger_contexts() {
    let catalog: Value = serde_json::from_slice(CHECKED_PORT_CATALOG).unwrap();
    let debounce = catalog["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["kind"] == "debounce_throttle")
        .unwrap();
    let winning = debounce["runtime_state"]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["name"] == "winning_input")
        .unwrap();
    assert_eq!(winning["type_ref"]["kind"], "trigger_context");
    assert_eq!(winning["type_ref"]["trigger_property"], "trigger");
    assert_eq!(winning["type_ref"]["optional"], true);

    let mut time = fixture("fixtures/valid-all-node-families.json");
    set_admission_trigger(&mut time, "time_schedule");
    assert_semantically_valid(&time);

    let mut state = time.clone();
    state["semantic"]["trigger"] = json!({
        "kind": "state_observation",
        "id": "trigger",
        "version": 1,
        "selector": state["semantic"]["nodes"].as_array().unwrap()
            .iter().find(|node| node["id"] == "switch_snapshot").unwrap()["selector"].clone(),
        "mode": "value_change",
        "initial": "ignore"
    });
    set_admission_trigger(&mut state, "state_observation");
    assert_semantically_valid(&state);

    let mut event = time;
    event["semantic"]["trigger"] =
        fixture("fixtures/valid-all-operations.json")["semantic"]["trigger"].clone();
    set_admission_trigger(&mut event, "discrete_event");
    assert_semantically_valid(&event);

    let mut mismatch = fixture("fixtures/valid-all-node-families.json");
    set_admission_trigger(&mut mismatch, "state_observation");
    assert!(matches!(
        compile_error(&mismatch),
        CompileError::InvalidNodeData { ref node, .. } if node == "admission"
    ));
}

#[test]
fn authored_artifact_ids_are_schema_fixed_and_activation_revisions_are_not_fabricated() {
    let schema: Value = serde_json::from_slice(CHECKED_RULE_SCHEMA).unwrap();
    for (definition, value) in [
        ("CompilerArtifactId", "litehouse:rule-compiler@1"),
        ("CelArtifactId", "litehouse-cel@1"),
        ("QuantityCatalogArtifactId", "litehouse:quantity-catalog@1"),
        ("RuntimeProfileArtifactId", "litehouse:rule-runtime@1"),
        ("TimezoneDatabaseArtifactId", "jiff-tzdb@0.1.8+2026c"),
        ("ContractCatalogArtifactId", "litehouse:contract-catalog@1"),
        (
            "ManagementOperationsArtifactId",
            "litehouse:management-operations@1",
        ),
        (
            "PortCatalogArtifactId",
            "https://schemas.litehouse.dev/rule-ir/v1.0.0/node-port-catalog.json",
        ),
    ] {
        assert_eq!(schema["$defs"][definition]["enum"], json!([value]));
    }
    let fixture = planner_fixture();
    let semantic = &fixture["semantic"];
    let text = serde_json::to_string(semantic).unwrap();
    assert!(!text.contains("provider_generation"));
    assert!(!text.contains("facet_revision"));
    assert!(!text.contains("grant_revision"));
    assert!(semantic["locks"].get("contract_catalog").is_some());
    assert!(semantic["locks"].get("management_operations").is_some());
    assert!(semantic["locks"].get("port_catalog").is_some());

    let manifest: Value = serde_json::from_slice(CHECKED_EVALUATION_ARTIFACTS).unwrap();
    assert_eq!(manifest["version"], 1);
    assert_eq!(
        manifest["rule_schema"]["sha256"],
        digest_bytes(CHECKED_RULE_SCHEMA)
    );
    let entries = manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| (entry["id"].as_str().unwrap(), entry))
        .collect::<BTreeMap<_, _>>();
    let concrete = [
        (
            "litehouse-cel@1",
            b"litehouse-cel@1\ncel=0.14.0\ncompatibility-subset=one-input-comparison-no-u64\n"
                .as_slice(),
        ),
        ("litehouse:contract-catalog@1", CONTRACT_ARTIFACT_MANIFEST),
        (
            "litehouse:management-operations@1",
            MANAGEMENT_OPERATIONS_V1_JSON,
        ),
        (
            "litehouse:rule-compiler@1",
            b"litehouse:rule-compiler@1\nrule-json=1.0.0\ncompatibility-planner=t8.1a\n".as_slice(),
        ),
        (
            "litehouse:quantity-catalog@1",
            include_bytes!(
                "../../litehouse-contracts/packages/quantity/1.0.0/quantity/profiles.json"
            )
            .as_slice(),
        ),
        (
            "litehouse:rule-runtime@1",
            include_bytes!("../../litehouse-contracts/packages/runtime-profile/1.0.0/profile.json")
                .as_slice(),
        ),
        (
            "https://schemas.litehouse.dev/rule-ir/v1.0.0/node-port-catalog.json",
            CHECKED_PORT_CATALOG,
        ),
        (
            "jiff-tzdb@0.1.8+2026c",
            b"jiff-tzdb@0.1.8+2026c\njiff=0.2.32\ntzdb-source=jiff-tzdb-bundle-always\n".as_slice(),
        ),
    ];
    assert_eq!(entries.len(), concrete.len());
    for (id, bytes) in concrete {
        let expected = digest_bytes(bytes);
        assert_eq!(entries[id]["sha256"], expected, "{id}");
        assert_eq!(semantic["locks"][lock_name(id)]["sha256"], expected, "{id}");
    }
    assert_eq!(
        fixture["schema"]["profile_sha256"],
        evaluation_profile_sha256()
    );

    for lock in [
        "compiler",
        "cel",
        "quantity_catalog",
        "runtime_profile",
        "timezone_database",
        "contract_catalog",
        "management_operations",
        "port_catalog",
    ] {
        let mut noncurrent = fixture.clone();
        noncurrent["semantic"]["locks"][lock]["sha256"] = json!("a".repeat(64));
        assert!(validator().is_valid(&noncurrent), "{lock}");
        assert!(matches!(
            compile_error(&noncurrent),
            CompileError::ArtifactDigestMismatch { artifact } if artifact == lock
        ));
    }
    let mut noncurrent_profile = fixture;
    noncurrent_profile["schema"]["profile_sha256"] = json!("a".repeat(64));
    assert!(matches!(
        compile_error(&noncurrent_profile),
        CompileError::EvaluationProfileDigestMismatch
    ));
}

#[test]
fn runtime_enforces_narrow_bounds_before_compatibility_lowering() {
    let mut deadline = planner_fixture();
    deadline["semantic"]["nodes"][3]["deadline_ms"] = json!(30_001);
    assert!(matches!(
        compile_error(&deadline),
        CompileError::InvalidNodeData { .. }
    ));

    let mut cel = planner_fixture();
    cel["semantic"]["nodes"][1]["expression"]["source"] = json!("x".repeat(4_097));
    assert!(matches!(
        compile_error(&cel),
        CompileError::InvalidNodeData { .. }
    ));

    let mut inputs = planner_fixture();
    let condition_inputs = inputs["semantic"]["nodes"][1]["expression"]["inputs"]
        .as_array_mut()
        .unwrap();
    for index in 1..17 {
        condition_inputs.push(json!({
            "name": format!("input_{index}"),
            "type_ref": {"kind": "bool"}
        }));
    }
    assert!(matches!(
        compile_error(&inputs),
        CompileError::InvalidNodeData { .. }
    ));

    let mut annotations = planner_fixture();
    let map = annotations["semantic"]["annotations"]
        .as_object_mut()
        .unwrap();
    map.clear();
    for index in 0..65 {
        map.insert(format!("test:key{index:02}"), json!("v"));
    }
    assert!(matches!(
        compile_error(&annotations),
        CompileError::BoundExceeded { .. }
    ));

    let mut editor = planner_fixture();
    let layout = editor["editor"]["nodes"]["trigger"].clone();
    let map = editor["editor"]["nodes"].as_object_mut().unwrap();
    map.clear();
    for index in 0..129 {
        map.insert(format!("node_{index:03}"), layout.clone());
    }
    assert!(matches!(
        compile_error(&editor),
        CompileError::BoundExceeded { .. }
    ));

    let mut patches = fixture("fixtures/valid-all-node-families.json");
    patches["semantic"]["limits"]["max_patch_operations"] = json!(1);
    assert!(matches!(
        compile_error(&patches),
        CompileError::InvalidLimit { name, .. } if name == "patch_operations"
    ));

    let mut effects = fixture("fixtures/valid-all-operations.json");
    effects["semantic"]["limits"]["max_effect_intents"] = json!(1);
    assert!(matches!(
        compile_error(&effects),
        CompileError::InvalidLimit { name, .. } if name == "effect_intents"
    ));

    let mut timers = fixture("fixtures/valid-all-node-families.json");
    let delay = timers["semantic"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == "delay")
        .unwrap()
        .clone();
    let mut second = delay;
    second["id"] = json!("delay_two");
    timers["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(second);
    timers["semantic"]["limits"]["max_timers"] = json!(1);
    assert!(matches!(
        compile_error(&timers),
        CompileError::InvalidLimit { name, .. } if name == "timers"
    ));

    let mut parallel = fixture("fixtures/valid-maximal.json");
    parallel["semantic"]["limits"]["max_parallel_branches"] = json!(3);
    assert!(matches!(
        compile_error(&parallel),
        CompileError::InvalidNodeData { .. }
    ));

    let mut schedule = fixture("fixtures/valid-maximal.json");
    schedule["semantic"]["trigger"]["schedule"]["days"] = json!([
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
        "monday"
    ]);
    assert!(matches!(
        compile_error(&schedule),
        CompileError::InvalidNodeData { .. }
    ));

    let mut limits = planner_fixture();
    limits["semantic"]["limits"]["max_cel_steps"] = json!(10_001);
    assert!(matches!(
        compile_error(&limits),
        CompileError::InvalidLimit { name, .. } if name == "max_cel_steps"
    ));

    let oversized = vec![b' '; litehouse_rule_json::HARD_MAX_JSON_BYTES + 1];
    assert!(matches!(
        compile_rule_json(&oversized),
        Err(RuleJsonError::JsonTooLarge { .. })
    ));
}

#[test]
fn evaluation_input_and_effective_planner_bounds_fail_closed_before_planning() {
    let time = Timestamp::new(1_721_234_567, 123_000_000).unwrap();
    let trigger = StateObservationInput::new(trigger_selector(), 1, 1, time).unwrap();

    let mut invalid_selector = trigger_selector();
    invalid_selector.facet_key = "f".repeat(65);
    assert!(matches!(
        TargetInput::new(invalid_selector, 1, 1, 1, true),
        Err(InputError::InvalidSelector)
    ));
    assert!(matches!(
        SnapshotInput::new(
            "snapshot",
            trigger_selector(),
            "on_off_current",
            1,
            1,
            time,
            Freshness::Fresh,
            ObservationValue::Known {
                value: ScalarValue::String {
                    value: "v".repeat(litehouse_rule_json::HARD_MAX_STRING_BYTES + 1),
                },
            },
        ),
        Err(InputError::ValueTooLarge { .. })
    ));

    let snapshots = (0..=litehouse_rule_json::HARD_MAX_NODES)
        .map(|index| {
            SnapshotInput::new(
                format!("snapshot_{index:03}"),
                trigger_selector(),
                "on_off_current",
                1,
                1,
                time,
                Freshness::Fresh,
                ObservationValue::Known {
                    value: ScalarValue::Bool { value: true },
                },
            )
            .unwrap()
        })
        .collect();
    assert!(matches!(
        EvaluationInput::new(
            RULE_ID,
            7,
            evaluation_profile_sha256(),
            time,
            trigger.clone(),
            snapshots,
            vec![],
        ),
        Err(InputError::TooManySnapshots { .. })
    ));

    let targets = (0..=litehouse_rule_json::HARD_MAX_EFFECT_INTENTS)
        .map(|index| {
            TargetInput::new(
                selector(&format!("{index:032x}"), "target", "on-off"),
                1,
                1,
                1,
                true,
            )
            .unwrap()
        })
        .collect();
    assert!(matches!(
        EvaluationInput::new(
            RULE_ID,
            7,
            evaluation_profile_sha256(),
            time,
            trigger.clone(),
            vec![],
            targets,
        ),
        Err(InputError::TooManyTargets { .. })
    ));

    let large_snapshots = (0..120)
        .map(|index| {
            SnapshotInput::new(
                format!("snapshot_{index:03}"),
                trigger_selector(),
                "on_off_current",
                1,
                1,
                time,
                Freshness::Fresh,
                ObservationValue::Known {
                    value: ScalarValue::String {
                        value: "v".repeat(512),
                    },
                },
            )
            .unwrap()
        })
        .collect();
    assert!(matches!(
        EvaluationInput::new(
            RULE_ID,
            7,
            evaluation_profile_sha256(),
            time,
            trigger.clone(),
            large_snapshots,
            vec![],
        ),
        Err(InputError::InputTooLarge { .. })
    ));

    let mut narrow_input = planner_fixture();
    narrow_input["semantic"]["limits"]["max_plan_input_bytes"] = json!(1_024);
    let compiled = compile_value(&narrow_input).unwrap();
    assert!(matches!(
        plan(
            &compiled,
            &input(ObservationValue::Known {
                value: ScalarValue::Bool { value: true },
            }),
        ),
        Err(PlanError::InputTooLarge { maximum: 1_024, .. })
    ));

    let mut trace_document = fixture("fixtures/valid-minimal.json");
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut previous = "trigger".to_owned();
    for index in 0..16 {
        let id = format!("s{index:02}{}", "a".repeat(61));
        nodes.push(json!({
            "kind": "snapshot",
            "id": id,
            "version": 1,
            "selector": serde_json::to_value(trigger_selector()).unwrap(),
            "field": {"kind": "on_off_current"}
        }));
        edges.push(json!({
            "id": format!("e{index:02}"),
            "from": {"node": previous, "port": "next"},
            "to": {"node": id, "port": "in"},
            "order": 0
        }));
        previous = id;
    }
    nodes.push(json!({"kind": "end", "id": "end", "version": 1, "outcome": "noop"}));
    edges.push(json!({
        "id": "e_end",
        "from": {"node": previous, "port": "next"},
        "to": {"node": "end", "port": "in"},
        "order": 0
    }));
    trace_document["semantic"]["nodes"] = Value::Array(nodes);
    trace_document["semantic"]["edges"] = Value::Array(edges);
    trace_document["semantic"]["limits"]["max_trace_bytes"] = json!(1_024);
    let compiled = compile_value(&trace_document).unwrap();
    let snapshots = trace_document["semantic"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["kind"] == "snapshot")
        .map(|node| {
            SnapshotInput::new(
                node["id"].as_str().unwrap(),
                trigger_selector(),
                "on_off_current",
                1,
                1,
                time,
                Freshness::Fresh,
                ObservationValue::Known {
                    value: ScalarValue::Bool { value: true },
                },
            )
            .unwrap()
        })
        .collect();
    let trace_input = EvaluationInput::new(
        "00000000000000000000000000000000",
        1,
        evaluation_profile_sha256(),
        time,
        trigger,
        snapshots,
        vec![],
    )
    .unwrap();
    assert!(matches!(
        plan(&compiled, &trace_input),
        Err(PlanError::TraceTooLarge { maximum: 1_024, .. })
    ));
}

#[test]
fn remaining_model_and_rule_local_bounds_fail_closed_before_subset_dispatch() {
    for (field, value) in [
        ("max_nodes", json!(0)),
        ("max_edges", json!(0)),
        ("max_depth", json!(33)),
        ("max_parallel_branches", json!(1)),
        ("max_queued_triggers", json!(17)),
        ("max_condition_nodes", json!(33)),
        ("max_effect_intents", json!(33)),
        ("max_patch_operations", json!(0)),
        ("max_timers", json!(33)),
        ("max_cel_steps", json!(0)),
        ("max_cel_memory_bytes", json!(1_048_577)),
        ("max_cel_eval_ms", json!(11)),
        ("max_plan_input_bytes", json!(1_023)),
        ("max_trace_bytes", json!(32_769)),
    ] {
        let mut document = planner_fixture();
        document["semantic"]["limits"][field] = value;
        assert!(
            matches!(compile_error(&document), CompileError::InvalidLimit { .. }),
            "{field}"
        );
    }

    let mut empty_name = planner_fixture();
    empty_name["semantic"]["name"] = json!("");
    assert!(matches!(
        compile_error(&empty_name),
        CompileError::EmptyValue { .. }
    ));
    let mut long_description = planner_fixture();
    long_description["semantic"]["description"] = json!("d".repeat(4_097));
    assert!(matches!(
        compile_error(&long_description),
        CompileError::BoundExceeded { .. }
    ));

    let mut annotation_value = planner_fixture();
    annotation_value["semantic"]["annotations"]["litehouse:fixture"] = json!("v".repeat(1_025));
    assert!(matches!(
        compile_error(&annotation_value),
        CompileError::BoundExceeded { .. }
    ));
    let mut annotation_key = planner_fixture();
    annotation_key["semantic"]["annotations"]["Not:valid"] = json!("v");
    assert!(matches!(
        compile_error(&annotation_key),
        CompileError::InvalidNodeData { .. }
    ));

    let mut editor_revision = planner_fixture();
    editor_revision["editor"]["revision"] = json!(0);
    assert!(matches!(
        compile_error(&editor_revision),
        CompileError::InvalidRevision
    ));
    let mut editor_groups = planner_fixture();
    let groups = editor_groups["editor"]["groups"].as_object_mut().unwrap();
    for index in 0..33 {
        groups.insert(
            format!("group_{index:02}"),
            json!({
                "label": "group",
                "x": 0,
                "y": 0,
                "width": 1,
                "height": 1,
                "collapsed": false
            }),
        );
    }
    assert!(matches!(
        compile_error(&editor_groups),
        CompileError::BoundExceeded { .. }
    ));
    let mut viewport = planner_fixture();
    viewport["editor"]["viewport"]["zoom_milli"] = json!(99);
    assert!(matches!(
        compile_error(&viewport),
        CompileError::InvalidNodeData { .. }
    ));

    let mut one_shot = fixture("fixtures/valid-time-one-shot.json");
    one_shot["semantic"]["trigger"]["schedule"]["at"]["nanosecond"] = json!(1_000_000_000_u64);
    assert!(matches!(
        compile_error(&one_shot),
        CompileError::InvalidNodeData { .. }
    ));
    let mut interval = fixture("fixtures/valid-time-fixed-interval.json");
    interval["semantic"]["trigger"]["schedule"]["interval_ms"] = json!(0);
    assert!(matches!(
        compile_error(&interval),
        CompileError::InvalidNodeData { .. }
    ));
    let mut weekly = fixture("fixtures/valid-maximal.json");
    weekly["semantic"]["trigger"]["schedule"]["timezone"] = json!("T".repeat(65));
    assert!(matches!(
        compile_error(&weekly),
        CompileError::InvalidNodeData { .. }
    ));

    let mut empty_inputs = planner_fixture();
    empty_inputs["semantic"]["nodes"][1]["expression"]["inputs"] = json!([]);
    assert!(matches!(
        compile_error(&empty_inputs),
        CompileError::InvalidNodeData { .. }
    ));
    let mut list_bound = planner_fixture();
    list_bound["semantic"]["nodes"][1]["expression"]["inputs"][0]["type_ref"] = json!({
        "kind": "list",
        "item": {"kind": "bool"},
        "max_items": 0
    });
    assert!(matches!(
        compile_error(&list_bound),
        CompileError::InvalidNodeData { .. }
    ));

    let mut delay = fixture("fixtures/valid-all-node-families.json");
    delay["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "delay")
        .unwrap()["duration"]["milliseconds"] = json!(0);
    assert!(matches!(
        compile_error(&delay),
        CompileError::InvalidNodeData { .. }
    ));
    let mut admission = fixture("fixtures/valid-all-node-families.json");
    admission["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "admission")
        .unwrap()["policy"]["window_ms"] = json!(0);
    assert!(matches!(
        compile_error(&admission),
        CompileError::InvalidNodeData { .. }
    ));

    let mut edge_order = planner_fixture();
    edge_order["semantic"]["edges"][0]["order"] = json!(256);
    assert!(matches!(
        compile_error(&edge_order),
        CompileError::InvalidNodeData { .. }
    ));

    let mut empty_patch = fixture("fixtures/valid-all-node-families.json");
    empty_patch["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "desired")
        .unwrap()["operations"] = json!([]);
    assert!(matches!(
        compile_error(&empty_patch),
        CompileError::InvalidNodeData { .. }
    ));

    let mut decimal = fixture("fixtures/valid-all-operations.json");
    decimal["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "effect_02")
        .unwrap()["operation"]["level"] = json!("1".repeat(33));
    assert!(matches!(
        compile_error(&decimal),
        CompileError::InvalidNodeData { .. }
    ));
    let mut choice = fixture("fixtures/valid-all-operations.json");
    choice["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "effect_19")
        .unwrap()["operation"]["fan_choice"] = json!("f".repeat(65));
    assert!(matches!(
        compile_error(&choice),
        CompileError::InvalidNodeData { .. }
    ));
    let mut transition = fixture("fixtures/valid-all-operations.json");
    transition["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "effect_06")
        .unwrap()["operation"]["duration_ms"] = json!(0);
    assert!(matches!(
        compile_error(&transition),
        CompileError::InvalidNodeData { .. }
    ));
}

#[test]
fn invalid_kinds_legacy_spellings_and_schema_bounds_fail_closed() {
    let validator = validator();
    for fixture in CHECKED_FIXTURES
        .iter()
        .filter(|fixture| !fixture.schema_valid)
    {
        let value: Value = serde_json::from_slice(fixture.bytes).unwrap();
        assert!(!validator.is_valid(&value), "{}", fixture.path);
        assert!(
            compile_rule_json(fixture.bytes).is_err(),
            "{}",
            fixture.path
        );
    }

    let mut trigger = planner_fixture();
    trigger["semantic"]["trigger"]["kind"] = json!("ambient_magic");
    assert!(!validator.is_valid(&trigger));
    assert!(matches!(
        compile_value(&trigger),
        Err(RuleJsonError::InvalidShape(_))
    ));

    let mut node = planner_fixture();
    node["semantic"]["nodes"][0]["kind"] = json!("script");
    assert!(!validator.is_valid(&node));
    assert!(matches!(
        compile_value(&node),
        Err(RuleJsonError::InvalidShape(_))
    ));

    let float = String::from_utf8(planner_bytes().to_vec())
        .unwrap()
        .replacen("\"revision\": 7", "\"revision\": 7.5", 1);
    assert!(matches!(
        compile_rule_json(float.as_bytes()),
        Err(RuleJsonError::FloatingPointNumber)
    ));
}

#[test]
fn operation_contract_is_checked_before_precise_subset_lowering() {
    let mut invalid = planner_fixture();
    invalid["semantic"]["nodes"][3]["operation"] = json!({
        "operation": "capability.level.set@1",
        "level": "0.5"
    });
    assert!(matches!(
        compile_error(&invalid),
        CompileError::OperationContractMismatch { .. }
    ));

    let mut valid = invalid;
    valid["semantic"]["nodes"][3]["selector"]["contract"]["package"] = json!("level-control");
    assert!(matches!(
        compile_error(&valid),
        CompileError::PlannerSubsetUnsupported { ref kind, .. }
            if kind == "capability.level.set@1"
    ));
}

#[test]
fn schedule_and_dst_policy_matrix_is_closed_and_semantically_valid() {
    for nonexistent in ["skip", "shift_forward"] {
        for duplicate in ["first", "second", "both"] {
            let mut value = fixture("fixtures/valid-all-node-families.json");
            value["semantic"]["trigger"]["schedule"]["nonexistent_time"] = json!(nonexistent);
            value["semantic"]["trigger"]["schedule"]["duplicate_time"] = json!(duplicate);
            assert!(validator().is_valid(&value), "{nonexistent}/{duplicate}");
            assert_semantically_valid(&value);
        }
    }
    let mut cron = fixture("fixtures/valid-time-fixed-interval.json");
    cron["semantic"]["trigger"]["schedule"] = json!({"kind": "cron", "value": "* * * * *"});
    assert!(!validator().is_valid(&cron));
}

#[test]
fn minimal_maximal_and_matrix_fixtures_cover_the_frozen_release_contract() {
    let minimal = fixture("fixtures/valid-minimal.json");
    assert_eq!(minimal["semantic"]["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(minimal["semantic"]["edges"].as_array().unwrap().len(), 1);

    let maximal = fixture("fixtures/valid-maximal.json");
    assert_eq!(maximal["semantic"]["name"].as_str().unwrap().len(), 128);
    assert_eq!(
        maximal["semantic"]["description"].as_str().unwrap().len(),
        4_096
    );
    assert_eq!(
        maximal["semantic"]["annotations"]
            .as_object()
            .unwrap()
            .len(),
        64
    );
    assert_eq!(maximal["editor"]["nodes"].as_object().unwrap().len(), 128);
    assert_eq!(maximal["editor"]["groups"].as_object().unwrap().len(), 32);
    assert!(
        serde_json::to_vec(&maximal).unwrap().len() <= litehouse_rule_json::HARD_MAX_JSON_BYTES
    );
    assert!(
        serde_json::to_vec(&maximal["semantic"]).unwrap().len()
            > litehouse_rule_json::HARD_MAX_CANONICAL_SEMANTIC_BYTES
    );
    assert_semantically_valid(&maximal);

    let operations = fixture("fixtures/valid-all-operations.json");
    let authored_operations = operations["semantic"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|node| node["operation"]["operation"].as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        authored_operations,
        CAPABILITY_OPERATION_IDS
            .into_iter()
            .collect::<BTreeSet<_>>()
    );

    let all_nodes = fixture("fixtures/valid-all-node-families.json");
    let node_kinds = all_nodes["semantic"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["kind"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(node_kinds.len(), 11);
}

#[test]
fn compatibility_planner_routes_false_unknown_stale_and_error_without_dispatch() {
    let compiled = compile_rule_json(planner_bytes()).unwrap();
    let false_plan = plan(
        &compiled,
        &input(ObservationValue::Known {
            value: ScalarValue::Bool { value: false },
        }),
    )
    .unwrap();
    assert_eq!(false_plan.outcome(), PlanOutcome::Noop);
    assert!(false_plan.intents().is_empty());

    let unknown_plan = plan(
        &compiled,
        &input(ObservationValue::Unknown {
            reason: UnknownReason::Unavailable,
        }),
    )
    .unwrap();
    assert_eq!(unknown_plan.outcome(), PlanOutcome::Unknown);

    let error_plan = plan(
        &compiled,
        &input(ObservationValue::Error {
            code: EvaluationErrorCode::SourceError,
        }),
    )
    .unwrap();
    assert_eq!(error_plan.outcome(), PlanOutcome::Error);

    let time = Timestamp::new(1_721_234_567, 123_000_000).unwrap();
    let stale_input = EvaluationInput::new(
        RULE_ID,
        7,
        evaluation_profile_sha256(),
        time,
        StateObservationInput::new(trigger_selector(), 3, 40, time).unwrap(),
        vec![
            SnapshotInput::new(
                "snapshot",
                trigger_selector(),
                "on_off_current",
                3,
                40,
                time,
                Freshness::Stale,
                ObservationValue::Known {
                    value: ScalarValue::Bool { value: true },
                },
            )
            .unwrap(),
        ],
        vec![TargetInput::new(target_selector(), 8, 12, 5, true).unwrap()],
    )
    .unwrap();
    assert_eq!(
        plan(&compiled, &stale_input).unwrap().outcome(),
        PlanOutcome::Unknown
    );
}

#[test]
fn bool_string_and_quantity_comparisons_lower_safely() {
    struct Case {
        field: Value,
        selector: Selector,
        field_name: String,
        type_ref: TypeRef,
        source: &'static str,
        observed: ScalarValue,
        quantity: Option<(QuantityProfileRef, Value)>,
    }
    let cases = [
        Case {
            field: json!({"kind": "on_off_current"}),
            selector: trigger_selector(),
            field_name: "on_off_current".to_owned(),
            type_ref: TypeRef::Optional {
                item: Box::new(TypeRef::Bool),
            },
            source: "left == true",
            observed: ScalarValue::Bool { value: true },
            quantity: None,
        },
        Case {
            field: json!({"kind": "thermostat_fan"}),
            selector: selector(
                "99999999999999999999999999999999",
                "thermostat",
                "thermostat",
            ),
            field_name: "thermostat_fan".to_owned(),
            type_ref: TypeRef::Optional {
                item: Box::new(TypeRef::String),
            },
            source: "left == \"auto\"",
            observed: ScalarValue::String {
                value: "auto".to_owned(),
            },
            quantity: None,
        },
        Case {
            field: json!({"kind": "measurement_current", "profile": profile_ids::POWER}),
            selector: selector("99999999999999999999999999999999", "power", "measurement"),
            field_name: "measurement_current_power".to_owned(),
            type_ref: TypeRef::Optional {
                item: Box::new(TypeRef::Quantity {
                    catalog: QuantityCatalogRef::V1,
                    profile: QuantityProfileRef::Power,
                }),
            },
            source: "",
            observed: ScalarValue::Quantity {
                value: Quantity::parse(profile_ids::POWER, "41", "W").unwrap(),
            },
            quantity: Some((
                QuantityProfileRef::Power,
                json!({
                    "kind": "compare",
                    "operator": "greater_or_equal",
                    "right": {"profile": profile_ids::POWER, "magnitude": "40", "unit": "W"}
                }),
            )),
        },
    ];

    for case in cases {
        let mut document = planner_fixture();
        document["semantic"]["nodes"][0]["field"] = case.field;
        document["semantic"]["nodes"][0]["selector"] =
            serde_json::to_value(&case.selector).unwrap();
        if let Some((profile, operation)) = case.quantity {
            document["semantic"]["nodes"][1] = json!({
                "kind": "quantity",
                "id": "condition",
                "version": 1,
                "input_profile": profile,
                "operation": operation
            });
            document["semantic"]["edges"][2]["to"]["port"] = json!("value");
        } else {
            document["semantic"]["nodes"][1]["expression"]["inputs"][0]["type_ref"] =
                serde_json::to_value(case.type_ref).unwrap();
            document["semantic"]["nodes"][1]["expression"]["source"] = json!(case.source);
            document["semantic"]["nodes"][1]["expression"]["normalized_source_sha256"] =
                json!(digest(case.source));
        }
        let compiled = compile_value(&document).unwrap();
        let transition = plan(
            &compiled,
            &input_for_snapshot(
                ObservationValue::Known {
                    value: case.observed,
                },
                case.selector,
                &case.field_name,
            ),
        )
        .unwrap();
        assert_eq!(transition.outcome(), PlanOutcome::AwaitingReceipt);
    }
}

#[test]
fn unsuffixed_cel_int_is_not_reinterpreted_as_u64() {
    let mut document = planner_fixture();
    document["semantic"]["nodes"][0]["field"] =
        json!({"kind": "observation_revision", "contract": "on-off"});
    document["semantic"]["nodes"][1]["expression"]["inputs"][0]["type_ref"] =
        serde_json::to_value(TypeRef::U64).unwrap();
    let source = "left > 40";
    document["semantic"]["nodes"][1]["expression"]["source"] = json!(source);
    document["semantic"]["nodes"][1]["expression"]["normalized_source_sha256"] =
        json!(digest(source));
    assert!(matches!(
        compile_error(&document),
        CompileError::PlannerSubsetUnsupported { ref node, ref kind }
            if node == "condition" && kind.contains("type-faithful CEL uint token parsing")
    ));
}

#[test]
fn stale_transition_assumption_is_detected_without_dispatch() {
    let compiled = compile_rule_json(planner_bytes()).unwrap();
    let transition = plan(
        &compiled,
        &input(ObservationValue::Known {
            value: ScalarValue::Bool { value: true },
        }),
    )
    .unwrap();
    let current = [
        ObservedRevision {
            selector: trigger_selector(),
            provider_generation: 3,
            facet_revision: 40,
            grant_revision: None,
            absolute_set: false,
        },
        ObservedRevision {
            selector: target_selector(),
            provider_generation: 9,
            facet_revision: 12,
            grant_revision: Some(5),
            absolute_set: true,
        },
    ];
    assert!(matches!(
        transition.check_assumptions(&current),
        Err(AssumptionError::Stale {
            expected_generation: 8,
            actual_generation: 9,
            ..
        })
    ));
}

fn set_admission_trigger(document: &mut Value, kind: &str) {
    document["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == "admission")
        .unwrap()["trigger"] = json!(kind);
}

fn wit_enum(package: &str, name: &str) -> BTreeSet<&'static str> {
    let path = format!(
        "{}/../litehouse-contracts/packages/{package}/1.0.0/wit/{package}.wit",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(path).unwrap();
    let marker = format!("enum {name} {{");
    let body = source
        .split_once(&marker)
        .unwrap()
        .1
        .split_once('}')
        .unwrap()
        .0;
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_end_matches(',').to_owned())
        .map(|line| Box::leak(line.into_boxed_str()) as &'static str)
        .collect()
}

fn expected_argument_type(id: &str, name: &str, schema: &Value) -> Value {
    let base = if schema.get("oneOf").is_some() {
        json!({"kind": "contract", "type_ref": "litehouse:color-control/types@1/color-value"})
    } else if schema["type"] == "boolean" {
        json!({"kind": "bool"})
    } else if schema["type"] == "integer" {
        json!({"kind": "u64"})
    } else if schema["pattern"] == "^-?(0|[1-9][0-9]*)(\\.[0-9]+)?$" {
        json!({"kind": "contract", "type_ref": "litehouse:quantity/types@1/canonical-decimal"})
    } else if id == "capability.thermostat.set-mode@1" && name == "mode" {
        json!({"kind": "contract", "type_ref": "litehouse:thermostat/types@1/mode"})
    } else {
        json!({"kind": "string"})
    };
    if id == "capability.notification.send@1" && name == "expires_in_ms" {
        json!({"kind": "optional", "item": base})
    } else {
        base
    }
}

fn never_retry_operation(id: &str) -> bool {
    matches!(
        id,
        "capability.on-off.toggle@1"
            | "capability.level.step@1"
            | "capability.level.move@1"
            | "capability.level.stop@1"
            | "capability.level.step-coupled@1"
            | "capability.level.move-coupled@1"
            | "capability.color.step@1"
            | "capability.color.move@1"
            | "capability.color.stop@1"
            | "capability.covering.open@1"
            | "capability.covering.close@1"
            | "capability.covering.stop@1"
            | "capability.momentary.invoke@1"
            | "capability.notification.send@1"
    )
}

#[allow(dead_code)]
fn _closed_map_marker(_: Map<String, Value>) {}
