use jsonschema::{Draft, JSONSchema};
use litehouse_contracts::quantity::{Quantity, profile_ids};
use litehouse_rule_json::{
    AssumptionError, CompileError, ContractRef, EvaluationErrorCode, EvaluationInput, Freshness,
    ObservationValue, ObservedRevision, PlanOutcome, RuleJsonError, ScalarValue, Selector,
    SnapshotInput, StateObservationInput, TargetInput, Timestamp, UnknownReason, compile_rule_json,
    plan,
    schema::{
        CHECKED_PORT_CATALOG, CHECKED_RULE_SCHEMA, generated_port_catalog_bytes,
        generated_rule_schema_bytes, schema_sha256,
    },
};
use serde_json::{Value, json};

const PROFILE_DIGEST: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const RULE_ID: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn fixture_bytes() -> &'static [u8] {
    include_bytes!("../fixtures/valid-bool-on-off.json")
}

fn fixture() -> Value {
    serde_json::from_slice(fixture_bytes()).expect("fixture JSON")
}

fn trigger_selector() -> Selector {
    Selector {
        provider_instance_id: "11111111111111111111111111111111".to_owned(),
        facet_key: "presence".to_owned(),
        contract: ContractRef {
            namespace: "example".to_owned(),
            package: "presence".to_owned(),
            interface: "state".to_owned(),
            major: 1,
        },
    }
}

fn target_selector() -> Selector {
    Selector {
        provider_instance_id: "22222222222222222222222222222222".to_owned(),
        facet_key: "light".to_owned(),
        contract: ContractRef {
            namespace: "litehouse".to_owned(),
            package: "on-off".to_owned(),
            interface: "capability".to_owned(),
            major: 1,
        },
    }
}

fn input(value: ObservationValue) -> EvaluationInput {
    let time = Timestamp::new(1_721_234_567, 123_000_000).unwrap();
    EvaluationInput::new(
        RULE_ID,
        7,
        PROFILE_DIGEST,
        time,
        StateObservationInput::new(trigger_selector(), 3, 40, time).unwrap(),
        vec![
            SnapshotInput::new(
                "snapshot",
                trigger_selector(),
                "occupied",
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

#[test]
fn generated_schema_and_port_catalog_equal_reviewed_artifacts() {
    assert_eq!(generated_rule_schema_bytes(), CHECKED_RULE_SCHEMA);
    assert_eq!(generated_port_catalog_bytes(), CHECKED_PORT_CATALOG);
    assert_eq!(
        schema_sha256(),
        "a1ac20dc95e3c4484a9668b8c301b0722f8592f6c7a26b5ced670a2909f9c70a"
    );

    let schema: Value = serde_json::from_slice(CHECKED_RULE_SCHEMA).unwrap();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert_eq!(
        schema["$id"],
        "https://schemas.litehouse.dev/rule-ir/v1.0.0/schema.json"
    );
    let validator = JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .unwrap();
    let valid = fixture();
    assert!(validator.is_valid(&valid));
    assert!(compile_rule_json(fixture_bytes()).is_ok());

    let mut open_metadata = valid;
    open_metadata["semantic"]["metadata"] = json!({"unreviewed": true});
    assert!(!validator.is_valid(&open_metadata));
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&open_metadata).unwrap()),
        Err(RuleJsonError::InvalidShape(_))
    ));
}

#[test]
fn valid_rule_compiles_and_plans_without_mutating_input() {
    let first = compile_rule_json(fixture_bytes()).unwrap();
    let second = compile_rule_json(fixture_bytes()).unwrap();
    assert_eq!(first.topological_order(), second.topological_order());
    assert_eq!(first.rule_id(), RULE_ID);
    assert_eq!(first.revision(), 7);

    let input = input(ObservationValue::Known {
        value: ScalarValue::Bool { value: true },
    });
    let before = input.clone();
    let transition = plan(&first, &input).unwrap();
    assert_eq!(input, before);
    assert_eq!(transition.outcome(), PlanOutcome::Success);
    assert_eq!(transition.intents().len(), 1);
    assert_eq!(transition.intents()[0].node_id, "set_light");
    assert!(transition.intents()[0].arguments.on);
    assert_eq!(transition.intents()[0].expected_provider_generation, 8);
    assert_eq!(transition.intents()[0].expected_facet_revision, 12);
    assert_eq!(transition.assumptions().len(), 2);
    assert_eq!(
        transition.visited_nodes(),
        [
            "trigger",
            "snapshot",
            "compare",
            "branch",
            "set_light",
            "end_success"
        ]
    );
}

#[test]
fn false_unknown_stale_and_error_are_explicit_routes() {
    let compiled = compile_rule_json(fixture_bytes()).unwrap();

    let false_plan = plan(
        &compiled,
        &input(ObservationValue::Known {
            value: ScalarValue::Bool { value: false },
        }),
    )
    .unwrap();
    assert_eq!(false_plan.outcome(), PlanOutcome::Noop);
    assert!(false_plan.intents().is_empty());
    assert_eq!(false_plan.visited_nodes().last().unwrap(), "end_noop");

    let unknown_plan = plan(
        &compiled,
        &input(ObservationValue::Unknown {
            reason: UnknownReason::Unavailable,
        }),
    )
    .unwrap();
    assert_eq!(unknown_plan.outcome(), PlanOutcome::Unknown);
    assert!(unknown_plan.intents().is_empty());
    assert_eq!(unknown_plan.visited_nodes().last().unwrap(), "end_unknown");

    let error_plan = plan(
        &compiled,
        &input(ObservationValue::Error {
            code: EvaluationErrorCode::SourceError,
        }),
    )
    .unwrap();
    assert_eq!(error_plan.outcome(), PlanOutcome::Error);
    assert!(error_plan.intents().is_empty());
    assert_eq!(error_plan.visited_nodes().last().unwrap(), "end_error");

    let time = Timestamp::new(1_721_234_567, 123_000_000).unwrap();
    let stale_input = EvaluationInput::new(
        RULE_ID,
        7,
        PROFILE_DIGEST,
        time,
        StateObservationInput::new(trigger_selector(), 3, 40, time).unwrap(),
        vec![
            SnapshotInput::new(
                "snapshot",
                trigger_selector(),
                "occupied",
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
    let stale_plan = plan(&compiled, &stale_input).unwrap();
    assert_eq!(stale_plan.outcome(), PlanOutcome::Unknown);
    assert!(stale_plan.intents().is_empty());
}

#[test]
fn bool_string_i64_and_quantity_comparisons_are_typed() {
    struct Case {
        value_type: Value,
        operator: &'static str,
        right: Value,
        observed: ScalarValue,
    }
    let cases = [
        Case {
            value_type: json!({"kind": "bool"}),
            operator: "equal",
            right: json!({"kind": "bool", "value": true}),
            observed: ScalarValue::Bool { value: true },
        },
        Case {
            value_type: json!({"kind": "string"}),
            operator: "equal",
            right: json!({"kind": "string", "value": "home"}),
            observed: ScalarValue::String {
                value: "home".to_owned(),
            },
        },
        Case {
            value_type: json!({"kind": "i64"}),
            operator: "greater",
            right: json!({"kind": "i64", "value": 40}),
            observed: ScalarValue::I64 { value: 41 },
        },
        Case {
            value_type: json!({"kind": "quantity", "profile": profile_ids::POWER}),
            operator: "greater_or_equal",
            right: json!({
                "kind": "quantity",
                "value": {"profile": profile_ids::POWER, "magnitude": "40", "unit": "W"}
            }),
            observed: ScalarValue::Quantity {
                value: Quantity::parse(profile_ids::POWER, "41", "W").unwrap(),
            },
        },
    ];

    for case in cases {
        let mut document = fixture();
        let nodes = document["semantic"]["nodes"].as_array_mut().unwrap();
        nodes[0]["value_type"] = case.value_type.clone();
        nodes[1]["value_type"] = case.value_type;
        nodes[1]["expression"]["operator"] = json!(case.operator);
        nodes[1]["expression"]["right"] = case.right;
        let compiled = compile_rule_json(&serde_json::to_vec(&document).unwrap()).unwrap();
        let transition = plan(
            &compiled,
            &input(ObservationValue::Known {
                value: case.observed,
            }),
        )
        .unwrap();
        assert_eq!(transition.outcome(), PlanOutcome::Success);
        assert_eq!(transition.intents().len(), 1);
    }
}

#[test]
fn malformed_graphs_fail_closed() {
    let mut dangling = fixture();
    dangling["semantic"]["edges"][0]["to"]["node"] = json!("missing");
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&dangling).unwrap()),
        Err(RuleJsonError::Compile(CompileError::DanglingNode { .. }))
    ));

    let mut incompatible = fixture();
    incompatible["semantic"]["nodes"][1]["value_type"] = json!({"kind": "string"});
    incompatible["semantic"]["nodes"][1]["expression"]["right"] =
        json!({"kind": "string", "value": "true"});
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&incompatible).unwrap()),
        Err(RuleJsonError::Compile(
            CompileError::IncompatiblePorts { .. }
        ))
    ));

    let mut duplicate_node = fixture();
    duplicate_node["semantic"]["nodes"][5]["id"] = json!("end_success");
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&duplicate_node).unwrap()),
        Err(RuleJsonError::Compile(CompileError::DuplicateNodeId { .. }))
    ));

    let mut duplicate_order = fixture();
    duplicate_order["semantic"]["edges"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "id": "e_duplicate_order",
            "from": {"node": "branch", "port": "true"},
            "to": {"node": "end_success", "port": "in"},
            "order": 0
        }));
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&duplicate_order).unwrap()),
        Err(RuleJsonError::Compile(
            CompileError::DuplicateEdgeOrder { .. }
        ))
    ));

    let mut unknown_port = fixture();
    unknown_port["semantic"]["edges"][0]["from"]["port"] = json!("mystery");
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&unknown_port).unwrap()),
        Err(RuleJsonError::Compile(CompileError::UnknownPort { .. }))
    ));
}

#[test]
fn cycles_are_rejected_before_planning() {
    let mut cyclic = fixture();
    cyclic["semantic"]["edges"][0]["to"]["node"] = json!("end_success");
    cyclic["semantic"]["edges"][6]["to"]["node"] = json!("snapshot");
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&cyclic).unwrap()),
        Err(RuleJsonError::Compile(CompileError::Cycle))
    ));
}

#[test]
fn unknown_fields_versions_floats_and_bounds_are_rejected() {
    let mut unknown_field = fixture();
    unknown_field["semantic"]["trigger"]["metadata"] = json!({});
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&unknown_field).unwrap()),
        Err(RuleJsonError::InvalidShape(_))
    ));

    let float = String::from_utf8(fixture_bytes().to_vec())
        .unwrap()
        .replacen("\"revision\": 7", "\"revision\": 7.5", 1);
    assert!(matches!(
        compile_rule_json(float.as_bytes()),
        Err(RuleJsonError::FloatingPointNumber)
    ));

    let mut unknown_version = fixture();
    unknown_version["schema"]["version"] = json!("2.0.0");
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&unknown_version).unwrap()),
        Err(RuleJsonError::Compile(
            CompileError::UnsupportedSchemaVersion { .. }
        ))
    ));

    let mut node_bound = fixture();
    node_bound["semantic"]["limits"]["max_nodes"] = json!(8);
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&node_bound).unwrap()),
        Err(RuleJsonError::Compile(CompileError::TooManyNodes { .. }))
    ));

    let mut edge_bound = fixture();
    edge_bound["semantic"]["limits"]["max_edges"] = json!(9);
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&edge_bound).unwrap()),
        Err(RuleJsonError::Compile(CompileError::TooManyEdges { .. }))
    ));

    let mut depth_bound = fixture();
    depth_bound["semantic"]["limits"]["max_depth"] = json!(4);
    assert!(matches!(
        compile_rule_json(&serde_json::to_vec(&depth_bound).unwrap()),
        Err(RuleJsonError::Compile(CompileError::GraphTooDeep { .. }))
    ));

    let oversized = vec![b' '; litehouse_rule_json::HARD_MAX_JSON_BYTES + 1];
    assert!(matches!(
        compile_rule_json(&oversized),
        Err(RuleJsonError::JsonTooLarge { .. })
    ));
}

#[test]
fn stale_transition_assumption_is_detected_without_dispatch() {
    let compiled = compile_rule_json(fixture_bytes()).unwrap();
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
