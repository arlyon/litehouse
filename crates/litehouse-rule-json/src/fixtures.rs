use std::collections::BTreeMap;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    artifacts::ArtifactDigests,
    model::{
        ActivationPolicy, AdmissionKey, AdmissionPolicy, AnnotationValue, BooleanStateEventKind,
        CapabilityOperation, CelArtifactId, CelArtifactLock, CelCondition, ChoiceValue, CivilTime,
        ColorDirection, ColorSpace, ColorValue, CompareOperator, CompilerArtifactId,
        CompilerArtifactLock, ConditionInput, ContractCatalogArtifactId,
        ContractCatalogArtifactLock, ContractEventRef, ContractRef, CoveringAxis, DecimalValue,
        DependencyHydrationPolicy, DesiredPatchOperation, DesiredProjectionValue,
        DuplicateTimePolicy, DurationValue, Edge, EdgeEndpoint, EditorData, EditorGroupData,
        EditorGroupKey, EditorNodeData, EditorNodeKey, EditorPresentation, EditorViewport,
        EndOutcome, EventDeduplicationPolicy, ForkBranch, ForkJoinPhase, InitialObservationPolicy,
        LevelDirection, ManagementOperationsArtifactId, ManagementOperationsArtifactLock,
        NamespacedAnnotationKey, Node, NonexistentTimePolicy, NotificationPriority,
        ObservationMode, PatchConflictPolicy, PortCatalogArtifactId, PortCatalogArtifactLock,
        ProviderDeduplicationRequirement, QuantityCatalogArtifactId, QuantityCatalogArtifactLock,
        QuantityCatalogRef, QuantityOperation, QuantityProfileRef, QuantityWire, RebindPolicy,
        ReceiptRequirement, RetryPolicy, RuleDocument, RuleLimits, RuleSchemaUrl,
        RuleSchemaVersion, RuntimeProfileArtifactId, RuntimeProfileArtifactLock, Schedule,
        SchemaLock, Selector, SelectorCardinality, SemanticLocks, SemanticPolicies, SemanticRule,
        StateFieldRef, StatefulContractKind, ThermostatMode, TimestampValue,
        TimezoneDatabaseArtifactId, TimezoneDatabaseArtifactLock, TriggerContextKind, TriggerNode,
        TriggerQueuePolicy, TypeRef, Weekday,
    },
};

pub(crate) struct FixtureValue {
    pub path: String,
    pub schema_valid: bool,
    pub semantic_valid: bool,
    pub value: Value,
}

pub(crate) fn fixture_values(schema_digest: &ArtifactDigests) -> Vec<FixtureValue> {
    let minimal = value(minimal_document(schema_digest));
    let planner = value(planner_document(schema_digest));
    let operations = value(operation_document(schema_digest));
    let all_nodes = value(all_node_families_document(schema_digest));
    let desired_paths = value(desired_paths_document(schema_digest));
    let one_shot = value(time_document(
        schema_digest,
        "dddddddddddddddddddddddddddddddd",
        Schedule::OneShot {
            at: TimestampValue {
                unix_seconds: 1_800_000_000,
                nanosecond: 999_999_999,
            },
        },
    ));
    let fixed_interval = value(time_document(
        schema_digest,
        "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        Schedule::FixedInterval {
            interval_ms: 604_800_000,
        },
    ));
    let weekly = value(time_document(
        schema_digest,
        "ffffffffffffffffffffffffffffffff",
        weekly_schedule(),
    ));
    let maximal = value(maximal_document(schema_digest));

    let mut unknown_field = minimal.clone();
    unknown_field["semantic"]["unreviewed"] = json!(true);
    let mut invalid_bound = minimal.clone();
    invalid_bound["semantic"]["limits"]["max_nodes"] = json!(129);
    let mut legacy_compare = planner.clone();
    legacy_compare["semantic"]["nodes"][1]["kind"] = json!("compare");
    let mut legacy_on_off = planner.clone();
    legacy_on_off["semantic"]["nodes"][3]["kind"] = json!("on_off_set");

    let mut fixtures = vec![
        fixture("fixtures/valid-minimal.json", true, minimal.clone()),
        fixture("fixtures/valid-planner-compat.json", true, planner.clone()),
        fixture(
            "fixtures/valid-all-operations.json",
            true,
            operations.clone(),
        ),
        fixture(
            "fixtures/valid-all-node-families.json",
            true,
            all_nodes.clone(),
        ),
        fixture(
            "fixtures/valid-all-desired-paths.json",
            true,
            desired_paths.clone(),
        ),
        fixture("fixtures/valid-time-one-shot.json", true, one_shot.clone()),
        fixture(
            "fixtures/valid-time-fixed-interval.json",
            true,
            fixed_interval.clone(),
        ),
        fixture("fixtures/valid-time-weekly.json", true, weekly.clone()),
        fixture("fixtures/valid-maximal.json", true, maximal.clone()),
        fixture("fixtures/invalid-unknown-field.json", false, unknown_field),
        fixture("fixtures/invalid-bound.json", false, invalid_bound),
        fixture(
            "fixtures/invalid-legacy-compare.json",
            false,
            legacy_compare,
        ),
        fixture(
            "fixtures/invalid-legacy-on-off-set.json",
            false,
            legacy_on_off,
        ),
    ];
    fixtures.extend(invalid_variant_fixtures(
        schema_digest,
        &minimal,
        &planner,
        &all_nodes,
        &desired_paths,
        &one_shot,
        &fixed_interval,
        &weekly,
    ));
    fixtures.extend(invalid_bound_fixtures(
        &minimal,
        &planner,
        &operations,
        &all_nodes,
        &one_shot,
        &fixed_interval,
        &weekly,
    ));
    fixtures
}

fn value(document: RuleDocument) -> Value {
    serde_json::to_value(document).expect("fixture document must serialize")
}

fn fixture(path: impl Into<String>, valid: bool, value: Value) -> FixtureValue {
    FixtureValue {
        path: path.into(),
        schema_valid: valid,
        semantic_valid: valid,
        value,
    }
}

fn semantic_invalid(path: impl Into<String>, value: Value) -> FixtureValue {
    FixtureValue {
        path: path.into(),
        schema_valid: true,
        semantic_valid: false,
        value,
    }
}

fn mutated(
    path: impl Into<String>,
    base: &Value,
    mutation: impl FnOnce(&mut Value),
) -> FixtureValue {
    let mut value = base.clone();
    mutation(&mut value);
    fixture(path, false, value)
}

fn semantically_mutated(
    path: impl Into<String>,
    base: &Value,
    mutation: impl FnOnce(&mut Value),
) -> FixtureValue {
    let mut value = base.clone();
    mutation(&mut value);
    semantic_invalid(path, value)
}

fn invalid_variant_fixtures(
    schema_digest: &ArtifactDigests,
    minimal: &Value,
    planner: &Value,
    all_nodes: &Value,
    desired_paths: &Value,
    one_shot: &Value,
    fixed_interval: &Value,
    weekly: &Value,
) -> Vec<FixtureValue> {
    let mut fixtures = Vec::new();

    let discrete_event = value(single_operation_document(
        schema_digest,
        "10000000000000000000000000000000",
        CapabilityOperation::OnOffSet { on: true },
        false,
    ));
    for (kind, base) in [
        ("state-observation", minimal),
        ("discrete-event", &discrete_event),
        ("time-schedule", weekly),
    ] {
        fixtures.push(mutated(
            format!("fixtures/invalid/trigger-{kind}-unknown-field.json"),
            base,
            |value| {
                value["semantic"]["trigger"]["unexpected"] = json!(true);
            },
        ));
    }

    for (kind, base) in [
        ("snapshot", planner),
        ("condition", planner),
        ("quantity", all_nodes),
        ("branch", planner),
        ("desired_patch", all_nodes),
        ("effect_intent", planner),
        ("delay", all_nodes),
        ("debounce_throttle", all_nodes),
        ("fork_join", all_nodes),
        ("receipt_branch", planner),
        ("end", planner),
    ] {
        fixtures.push(mutated(
            format!(
                "fixtures/invalid/node-{}-unknown-field.json",
                kind.replace('_', "-")
            ),
            base,
            |value| {
                let node = value["semantic"]["nodes"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|node| node["kind"] == kind)
                    .unwrap();
                node["unexpected"] = json!(true);
            },
        ));
    }

    for (index, operation) in all_operations().into_iter().enumerate() {
        let filename = operation.kind().operation_id().replace(['.', '@'], "-");
        let base = value(single_operation_document(
            schema_digest,
            &format!("{index:032x}"),
            operation,
            false,
        ));
        fixtures.push(mutated(
            format!("fixtures/invalid/operation-{filename}-unknown-field.json"),
            &base,
            |value| {
                find_node(value, "effect_intent")["operation"]["unexpected"] = json!(true);
            },
        ));
    }

    for (kind, base) in [
        ("one-shot", one_shot),
        ("fixed-interval", fixed_interval),
        ("weekly-civil", weekly),
    ] {
        fixtures.push(mutated(
            format!("fixtures/invalid/schedule-{kind}-unknown-field.json"),
            base,
            |value| {
                value["semantic"]["trigger"]["schedule"]["unexpected"] = json!(true);
            },
        ));
    }
    for policy in ["skip", "shift_forward", "error"] {
        fixtures.push(mutated(
            format!(
                "fixtures/invalid/dst-nonexistent-{}-unknown-field.json",
                policy.replace('_', "-")
            ),
            weekly,
            |value| {
                value["semantic"]["trigger"]["schedule"]["nonexistent_time"] = json!(policy);
                value["semantic"]["trigger"]["schedule"]["unexpected"] = json!(true);
            },
        ));
    }
    for policy in ["first", "second", "both"] {
        fixtures.push(mutated(
            format!("fixtures/invalid/dst-duplicate-{policy}-unknown-field.json"),
            weekly,
            |value| {
                value["semantic"]["trigger"]["schedule"]["duplicate_time"] = json!(policy);
                value["semantic"]["trigger"]["schedule"]["unexpected"] = json!(true);
            },
        ));
    }

    fixtures.push(mutated(
        "fixtures/invalid/retry-none-unknown-field.json",
        planner,
        |value| {
            value["semantic"]["nodes"][3]["unexpected"] = json!(true);
        },
    ));
    fixtures.push(mutated(
        "fixtures/invalid/retry-definitely-pre-dispatch-unknown-field.json",
        all_nodes,
        |value| {
            let effect = value["semantic"]["nodes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|node| node["kind"] == "effect_intent")
                .unwrap();
            effect["retry"]["unexpected"] = json!(true);
        },
    ));

    fixtures.push(mutated(
        "fixtures/invalid/receipt-requirement-accepted-for-delivery-unknown-field.json",
        planner,
        |value| {
            value["semantic"]["nodes"][3]["unexpected"] = json!(true);
        },
    ));
    let notification = value(single_operation_document(
        schema_digest,
        "20000000000000000000000000000000",
        CapabilityOperation::NotificationSend {
            notification_template_id: "fixture".to_owned(),
            priority: NotificationPriority::Normal,
            expires_in_ms: None,
        },
        false,
    ));
    fixtures.push(mutated(
        "fixtures/invalid/receipt-requirement-notification-advertised-maximum-unknown-field.json",
        &notification,
        |value| {
            find_node(value, "effect_intent")["unexpected"] = json!(true);
        },
    ));
    let receipt_routes = value(single_operation_document(
        schema_digest,
        "30000000000000000000000000000000",
        CapabilityOperation::OnOffSet { on: true },
        true,
    ));
    for outcome in [
        "accepted_for_delivery",
        "protocol_or_target_acknowledged",
        "completed",
        "observed_completed",
        "rejected",
        "error",
        "cancelled",
        "outcome_unknown",
    ] {
        fixtures.push(mutated(
            format!(
                "fixtures/invalid/receipt-outcome-{}-edge-unknown-field.json",
                outcome.replace('_', "-")
            ),
            &receipt_routes,
            |value| {
                let edge = value["semantic"]["edges"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|edge| edge["from"]["port"] == outcome)
                    .unwrap();
                edge["unexpected"] = json!(true);
            },
        ));
    }

    let desired_inventory = desired_paths["semantic"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["kind"] == "desired_patch")
        .flat_map(|node| node["operations"].as_array().unwrap())
        .filter_map(|operation| operation.get("expected").or_else(|| operation.get("value")))
        .filter_map(|value| value["path"].as_str())
        .map(str::to_owned)
        .collect::<std::collections::BTreeSet<_>>();
    for path in desired_inventory {
        fixtures.push(mutated(
            format!(
                "fixtures/invalid/desired-path-{}-unknown-field.json",
                path.trim_start_matches('/').replace(['/', '_'], "-")
            ),
            desired_paths,
            |value| {
                let projection = value["semantic"]["nodes"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .filter(|node| node["kind"] == "desired_patch")
                    .flat_map(|node| node["operations"].as_array_mut().unwrap())
                    .filter_map(|operation| {
                        if operation.get("expected").is_some() {
                            operation.get_mut("expected")
                        } else {
                            operation.get_mut("value")
                        }
                    })
                    .find(|projection| projection["path"] == path)
                    .unwrap();
                projection["unexpected"] = json!(true);
            },
        ));
    }

    for presentation in ["standard", "compact", "emphasized"] {
        fixtures.push(mutated(
            format!("fixtures/invalid/editor-presentation-{presentation}-unknown-field.json"),
            planner,
            |value| {
                value["editor"]["nodes"]["trigger"]["presentation"] = json!(presentation);
                value["editor"]["nodes"]["trigger"]["unexpected"] = json!(true);
            },
        ));
    }
    fixtures.push(mutated(
        "fixtures/invalid/editor-group-unknown-field.json",
        planner,
        |value| {
            value["editor"]["groups"]["group_00"] = json!({
                "label": "group",
                "x": 0,
                "y": 0,
                "width": 1,
                "height": 1,
                "collapsed": false,
                "unexpected": true
            });
        },
    ));
    fixtures.push(mutated(
        "fixtures/invalid/editor-viewport-unknown-field.json",
        planner,
        |value| {
            value["editor"]["viewport"]["unexpected"] = json!(true);
        },
    ));

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
        fixtures.push(semantically_mutated(
            format!(
                "fixtures/invalid/noncurrent-lock-{}.json",
                lock.replace('_', "-")
            ),
            planner,
            |value| {
                value["semantic"]["locks"][lock]["sha256"] = json!("a".repeat(64));
            },
        ));
    }
    fixtures.push(semantically_mutated(
        "fixtures/invalid/noncurrent-evaluation-profile.json",
        planner,
        |value| {
            value["schema"]["profile_sha256"] = json!("a".repeat(64));
        },
    ));

    fixtures
}

fn invalid_bound_fixtures(
    minimal: &Value,
    planner: &Value,
    operations: &Value,
    all_nodes: &Value,
    one_shot: &Value,
    fixed_interval: &Value,
    weekly: &Value,
) -> Vec<FixtureValue> {
    let mut fixtures = Vec::new();
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
        fixtures.push(mutated(
            format!(
                "fixtures/invalid/bound-limit-{}.json",
                field.replace('_', "-")
            ),
            planner,
            |document| document["semantic"]["limits"][field] = value,
        ));
    }

    let simple_bounds: [(&str, &Value, fn(&mut Value)); 22] = [
        ("rule-id", planner, |v| v["rule_id"] = json!("a".repeat(31))),
        ("rule-revision", planner, |v| v["revision"] = json!(0)),
        ("schema-digest-shape", planner, |v| {
            v["schema"]["sha256"] = json!("g".repeat(64))
        }),
        ("name-empty", planner, |v| v["semantic"]["name"] = json!("")),
        ("name-bytes", planner, |v| {
            v["semantic"]["name"] = json!("n".repeat(129))
        }),
        ("description-bytes", planner, |v| {
            v["semantic"]["description"] = json!("d".repeat(4_097))
        }),
        ("annotation-key", planner, |v| {
            v["semantic"]["annotations"]["Not:valid"] = json!("v")
        }),
        ("annotation-value", planner, |v| {
            v["semantic"]["annotations"]["litehouse:fixture"] = json!("v".repeat(1_025))
        }),
        ("editor-revision", planner, |v| {
            v["editor"]["revision"] = json!(0)
        }),
        ("editor-viewport-zoom", planner, |v| {
            v["editor"]["viewport"]["zoom_milli"] = json!(99)
        }),
        ("one-shot-nanosecond", one_shot, |v| {
            v["semantic"]["trigger"]["schedule"]["at"]["nanosecond"] = json!(1_000_000_000_u64)
        }),
        ("fixed-interval", fixed_interval, |v| {
            v["semantic"]["trigger"]["schedule"]["interval_ms"] = json!(0)
        }),
        ("weekly-timezone", weekly, |v| {
            v["semantic"]["trigger"]["schedule"]["timezone"] = json!("T".repeat(65))
        }),
        ("condition-inputs-empty", planner, |v| {
            v["semantic"]["nodes"][1]["expression"]["inputs"] = json!([])
        }),
        ("condition-source-bytes", planner, |v| {
            v["semantic"]["nodes"][1]["expression"]["source"] = json!("x".repeat(4_097))
        }),
        ("delay-duration", all_nodes, |v| {
            find_node(v, "delay")["duration"]["milliseconds"] = json!(0)
        }),
        ("admission-window", all_nodes, |v| {
            find_node(v, "debounce_throttle")["policy"]["window_ms"] = json!(0)
        }),
        ("edge-order", planner, |v| {
            v["semantic"]["edges"][0]["order"] = json!(256)
        }),
        ("desired-operations-empty", all_nodes, |v| {
            find_node(v, "desired_patch")["operations"] = json!([])
        }),
        ("operation-decimal", operations, |v| {
            find_node_by_id(v, "effect_02")["operation"]["level"] = json!("1".repeat(33))
        }),
        ("operation-choice", operations, |v| {
            find_node_by_id(v, "effect_19")["operation"]["fan_choice"] = json!("f".repeat(65))
        }),
        ("operation-transition", operations, |v| {
            find_node_by_id(v, "effect_06")["operation"]["duration_ms"] = json!(0)
        }),
    ];
    for (name, base, mutation) in simple_bounds {
        fixtures.push(mutated(
            format!("fixtures/invalid/bound-{name}.json"),
            base,
            mutation,
        ));
    }

    fixtures.push(mutated(
        "fixtures/invalid/bound-annotations-count.json",
        planner,
        |value| {
            let annotations = value["semantic"]["annotations"].as_object_mut().unwrap();
            annotations.clear();
            for index in 0..65 {
                annotations.insert(format!("test:key{index:02}"), json!("v"));
            }
        },
    ));
    fixtures.push(mutated(
        "fixtures/invalid/bound-editor-nodes-count.json",
        planner,
        |value| {
            let layout = value["editor"]["nodes"]["trigger"].clone();
            let nodes = value["editor"]["nodes"].as_object_mut().unwrap();
            nodes.clear();
            for index in 0..129 {
                nodes.insert(format!("node_{index:03}"), layout.clone());
            }
        },
    ));
    fixtures.push(mutated(
        "fixtures/invalid/bound-editor-groups-count.json",
        planner,
        |value| {
            let groups = value["editor"]["groups"].as_object_mut().unwrap();
            for index in 0..33 {
                groups.insert(
                    format!("group_{index:02}"),
                    json!({"label":"group","x":0,"y":0,"width":1,"height":1,"collapsed":false}),
                );
            }
        },
    ));
    fixtures.push(mutated(
        "fixtures/invalid/bound-weekly-days-count.json",
        weekly,
        |value| {
            value["semantic"]["trigger"]["schedule"]["days"] = json!([
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
                "sunday",
                "monday"
            ]);
        },
    ));
    fixtures.push(mutated(
        "fixtures/invalid/bound-condition-inputs-count.json",
        planner,
        |value| {
            let inputs = value["semantic"]["nodes"][1]["expression"]["inputs"]
                .as_array_mut()
                .unwrap();
            for index in 1..17 {
                inputs.push(json!({"name":format!("input_{index}"),"type_ref":{"kind":"bool"}}));
            }
        },
    ));
    fixtures.push(mutated(
        "fixtures/invalid/bound-type-ref-list-items.json",
        planner,
        |value| {
            value["semantic"]["nodes"][1]["expression"]["inputs"][0]["type_ref"] =
                json!({"kind":"list","item":{"kind":"bool"},"max_items":0});
        },
    ));

    for (name, base, limit, value) in [
        ("configured-nodes", planner, "max_nodes", json!(1)),
        ("configured-edges", planner, "max_edges", json!(1)),
        ("configured-depth", planner, "max_depth", json!(1)),
        (
            "configured-patch-operations",
            all_nodes,
            "max_patch_operations",
            json!(1),
        ),
        (
            "configured-effect-intents",
            operations,
            "max_effect_intents",
            json!(1),
        ),
    ] {
        fixtures.push(semantically_mutated(
            format!("fixtures/invalid/bound-{name}.json"),
            base,
            |document| document["semantic"]["limits"][limit] = value,
        ));
    }

    fixtures.push(semantically_mutated(
        "fixtures/invalid/bound-configured-parallel-branches.json",
        all_nodes,
        |value| {
            let branch = json!({"id":"extra","order":2});
            find_node_by_id(value, "fork")["phase"]["branches"]
                .as_array_mut()
                .unwrap()
                .push(branch.clone());
            find_node_by_id(value, "join")["phase"]["branches"]
                .as_array_mut()
                .unwrap()
                .push(branch);
            value["semantic"]["limits"]["max_parallel_branches"] = json!(2);
        },
    ));
    fixtures.push(semantically_mutated(
        "fixtures/invalid/bound-configured-condition-nodes.json",
        planner,
        |value| {
            let mut condition = value["semantic"]["nodes"][1].clone();
            condition["id"] = json!("condition_two");
            value["semantic"]["nodes"]
                .as_array_mut()
                .unwrap()
                .push(condition);
            value["semantic"]["limits"]["max_condition_nodes"] = json!(1);
        },
    ));
    fixtures.push(semantically_mutated(
        "fixtures/invalid/bound-configured-timers.json",
        all_nodes,
        |value| {
            let mut delay = find_node(value, "delay").clone();
            delay["id"] = json!("delay_two");
            value["semantic"]["nodes"]
                .as_array_mut()
                .unwrap()
                .push(delay);
            value["semantic"]["limits"]["max_timers"] = json!(1);
        },
    ));
    fixtures.push(semantically_mutated(
        "fixtures/invalid/bound-json-depth.json",
        planner,
        |value| {
            let mut nested = json!({"kind":"bool"});
            for _ in 0..33 {
                nested = json!({"kind":"optional","item":nested});
            }
            value["semantic"]["nodes"][1]["expression"]["inputs"][0]["type_ref"] = nested;
        },
    ));
    fixtures.push(semantically_mutated(
        "fixtures/invalid/bound-schema-digest-noncurrent.json",
        minimal,
        |value| value["schema"]["sha256"] = json!("a".repeat(64)),
    ));
    fixtures
}

fn find_node<'a>(value: &'a mut Value, kind: &str) -> &'a mut Value {
    value["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["kind"] == kind)
        .unwrap()
}

fn find_node_by_id<'a>(value: &'a mut Value, id: &str) -> &'a mut Value {
    value["semantic"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["id"] == id)
        .unwrap()
}

fn minimal_document(schema_digest: &ArtifactDigests) -> RuleDocument {
    let trigger_selector = selector(
        "11111111111111111111111111111111",
        "source_switch",
        "on-off",
    );
    document(
        schema_digest,
        "00000000000000000000000000000000",
        1,
        "Minimal valid rule",
        TriggerNode::StateObservation {
            id: "trigger".to_owned(),
            version: 1,
            selector: trigger_selector,
            mode: ObservationMode::AnyObservation,
            initial: InitialObservationPolicy::Ignore,
        },
        vec![end("end", EndOutcome::Noop)],
        vec![edge("e_trigger_end", "trigger", "next", "end", "in")],
    )
}

fn planner_document(schema_digest: &ArtifactDigests) -> RuleDocument {
    let trigger_selector = selector(
        "11111111111111111111111111111111",
        "source_switch",
        "on-off",
    );
    let target_selector = selector("22222222222222222222222222222222", "target_light", "on-off");
    let source = "left == true";
    let nodes = vec![
        Node::Snapshot {
            id: "snapshot".to_owned(),
            version: 1,
            selector: trigger_selector.clone(),
            field: StateFieldRef::OnOffCurrent,
        },
        Node::Condition {
            id: "condition".to_owned(),
            version: 1,
            expression: CelCondition {
                source: source.to_owned(),
                normalized_source_sha256: sha256(source.as_bytes()),
                inputs: vec![ConditionInput {
                    name: "left".to_owned(),
                    type_ref: TypeRef::Optional {
                        item: Box::new(TypeRef::Bool),
                    },
                }],
            },
        },
        Node::Branch {
            id: "branch".to_owned(),
            version: 1,
        },
        Node::EffectIntent {
            id: "set_light".to_owned(),
            version: 1,
            selector: target_selector,
            operation: CapabilityOperation::OnOffSet { on: true },
            deadline_ms: 30_000,
            required_receipt: ReceiptRequirement::AcceptedForDelivery,
            retry: RetryPolicy::None,
        },
        Node::ReceiptBranch {
            id: "receipt".to_owned(),
            version: 1,
            effect_node_id: "set_light".to_owned(),
        },
        end("end_success", EndOutcome::Success),
        end("end_noop", EndOutcome::Noop),
        end("end_unknown", EndOutcome::Unknown),
        end("end_error", EndOutcome::Error),
    ];
    let edges = vec![
        edge("e_trigger_snapshot", "trigger", "next", "snapshot", "in"),
        edge(
            "e_snapshot_condition",
            "snapshot",
            "next",
            "condition",
            "in",
        ),
        edge("e_snapshot_value", "snapshot", "value", "condition", "left"),
        edge("e_condition_branch", "condition", "next", "branch", "in"),
        edge(
            "e_condition_result",
            "condition",
            "result",
            "branch",
            "predicate",
        ),
        edge("e_true_set", "branch", "true", "set_light", "in"),
        edge(
            "e_set_receipt",
            "set_light",
            "receipt",
            "receipt",
            "receipt",
        ),
        edge(
            "e_receipt_success",
            "receipt",
            "accepted_for_delivery",
            "end_success",
            "in",
        ),
        edge("e_false_noop", "branch", "false", "end_noop", "in"),
        edge("e_unknown_end", "branch", "unknown", "end_unknown", "in"),
        edge("e_error_end", "branch", "error", "end_error", "in"),
    ];
    document(
        schema_digest,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        7,
        "Observed On/Off emits an effect and waits for its typed receipt",
        TriggerNode::StateObservation {
            id: "trigger".to_owned(),
            version: 1,
            selector: trigger_selector,
            mode: ObservationMode::ValueChange,
            initial: InitialObservationPolicy::Ignore,
        },
        nodes,
        edges,
    )
}

fn operation_document(schema_digest: &ArtifactDigests) -> RuleDocument {
    let source = selector(
        "33333333333333333333333333333333",
        "occupancy",
        "boolean-state",
    );
    let operations = all_operations();
    let mut nodes = Vec::with_capacity(operations.len() * 2 + 1);
    let mut edges = Vec::with_capacity(operations.len() * 2 + 1);
    for (index, operation) in operations.into_iter().enumerate() {
        let effect = format!("effect_{index:02}");
        let receipt = format!("receipt_{index:02}");
        let package = operation.kind().package();
        nodes.push(Node::EffectIntent {
            id: effect.clone(),
            version: 1,
            selector: selector(
                "44444444444444444444444444444444",
                &format!("facet_{index:02}"),
                package,
            ),
            required_receipt: if operation.is_notification() {
                ReceiptRequirement::NotificationAdvertisedMaximum
            } else {
                ReceiptRequirement::AcceptedForDelivery
            },
            operation,
            deadline_ms: 30_000,
            retry: RetryPolicy::None,
        });
        nodes.push(Node::ReceiptBranch {
            id: receipt.clone(),
            version: 1,
            effect_node_id: effect.clone(),
        });
        edges.push(edge(
            &format!("e_{index:02}_receipt"),
            &effect,
            "receipt",
            &receipt,
            "receipt",
        ));
    }
    edges.push(edge(
        "e_trigger_effect",
        "trigger",
        "next",
        "effect_00",
        "in",
    ));
    let receipt_ports = [
        "accepted_for_delivery",
        "protocol_or_target_acknowledged",
        "completed",
        "observed_completed",
        "rejected",
        "error",
        "cancelled",
        "outcome_unknown",
    ];
    for (parent, children) in [
        (0_usize, 1_usize..9_usize),
        (1, 9..17),
        (2, 17..25),
        (3, 25..29),
    ] {
        for (port, child) in receipt_ports.iter().zip(children) {
            edges.push(edge(
                &format!("e_{parent:02}_{child:02}_route"),
                &format!("receipt_{parent:02}"),
                port,
                &format!("effect_{child:02}"),
                "in",
            ));
        }
    }
    nodes.push(end("end", EndOutcome::Success));
    edges.push(edge(
        "e_receipt_end",
        "receipt_04",
        "accepted_for_delivery",
        "end",
        "in",
    ));
    document(
        schema_digest,
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        1,
        "All 29 exact capability operation shapes and typed receipt continuations",
        TriggerNode::DiscreteEvent {
            id: "trigger".to_owned(),
            version: 1,
            selector: source,
            event: ContractEventRef::BooleanState {
                event: BooleanStateEventKind::Occurrence,
            },
            deduplication: EventDeduplicationPolicy::ProcessEpochAtLeastOnce,
        },
        nodes,
        edges,
    )
}

fn all_node_families_document(schema_digest: &ArtifactDigests) -> RuleDocument {
    let measurement = selector("55555555555555555555555555555555", "power", "measurement");
    let switch = selector("66666666666666666666666666666666", "switch", "on-off");
    let condition_source = "left == true";
    let branches = vec![
        ForkBranch {
            id: "effect".to_owned(),
            order: 0,
        },
        ForkBranch {
            id: "noop".to_owned(),
            order: 1,
        },
    ];
    let nodes = vec![
        Node::Snapshot {
            id: "measurement_snapshot".to_owned(),
            version: 1,
            selector: measurement,
            field: StateFieldRef::MeasurementCurrent {
                profile: QuantityProfileRef::Power,
            },
        },
        Node::Quantity {
            id: "quantity".to_owned(),
            version: 1,
            input_profile: QuantityProfileRef::Power,
            operation: QuantityOperation::Compare {
                operator: CompareOperator::GreaterOrEqual,
                right: QuantityWire {
                    profile: QuantityProfileRef::Power,
                    magnitude: "40".to_owned(),
                    unit: "W".to_owned(),
                },
            },
        },
        Node::Branch {
            id: "quantity_branch".to_owned(),
            version: 1,
        },
        Node::Snapshot {
            id: "switch_snapshot".to_owned(),
            version: 1,
            selector: switch.clone(),
            field: StateFieldRef::OnOffCurrent,
        },
        Node::Condition {
            id: "condition".to_owned(),
            version: 1,
            expression: CelCondition {
                source: condition_source.to_owned(),
                normalized_source_sha256: sha256(condition_source.as_bytes()),
                inputs: vec![ConditionInput {
                    name: "left".to_owned(),
                    type_ref: TypeRef::Optional {
                        item: Box::new(TypeRef::Bool),
                    },
                }],
            },
        },
        Node::Branch {
            id: "condition_branch".to_owned(),
            version: 1,
        },
        Node::DesiredPatch {
            id: "desired".to_owned(),
            version: 1,
            snapshot_node_id: "switch_snapshot".to_owned(),
            operations: vec![
                DesiredPatchOperation::Test {
                    expected: DesiredProjectionValue::On(false),
                },
                DesiredPatchOperation::Replace {
                    value: DesiredProjectionValue::On(true),
                },
            ],
        },
        Node::Delay {
            id: "delay".to_owned(),
            version: 1,
            duration: DurationValue {
                milliseconds: 86_400_000,
            },
        },
        Node::DebounceThrottle {
            id: "admission".to_owned(),
            version: 1,
            trigger: TriggerContextKind::TimeSchedule,
            key: AdmissionKey::RuleTrigger,
            policy: AdmissionPolicy::ThrottleLeading {
                window_ms: 1_000,
                trailing: true,
            },
        },
        Node::ForkJoin {
            id: "fork".to_owned(),
            version: 1,
            phase: ForkJoinPhase::Fork {
                join_node_id: "join".to_owned(),
                branches: branches.clone(),
            },
            conflict_policy: PatchConflictPolicy::RejectConflict,
        },
        Node::EffectIntent {
            id: "effect".to_owned(),
            version: 1,
            selector: switch,
            operation: CapabilityOperation::OnOffSet { on: true },
            deadline_ms: 30_000,
            required_receipt: ReceiptRequirement::AcceptedForDelivery,
            retry: RetryPolicy::DefinitelyPreDispatch {
                max_attempts: 2,
                backoff_ms: 100,
                provider_deduplication:
                    ProviderDeduplicationRequirement::MatchingProviderGeneration,
            },
        },
        Node::ReceiptBranch {
            id: "receipt".to_owned(),
            version: 1,
            effect_node_id: "effect".to_owned(),
        },
        Node::ForkJoin {
            id: "join".to_owned(),
            version: 1,
            phase: ForkJoinPhase::Join {
                fork_node_id: "fork".to_owned(),
                branches,
            },
            conflict_policy: PatchConflictPolicy::RejectConflict,
        },
        end("end_success", EndOutcome::Success),
        end("end_halt", EndOutcome::Unknown),
    ];
    let edges = vec![
        edge(
            "e_trigger_snapshot",
            "trigger",
            "next",
            "measurement_snapshot",
            "in",
        ),
        edge(
            "e_snapshot_quantity",
            "measurement_snapshot",
            "next",
            "quantity",
            "in",
        ),
        edge(
            "e_snapshot_value",
            "measurement_snapshot",
            "value",
            "quantity",
            "value",
        ),
        edge(
            "e_quantity_branch",
            "quantity",
            "next",
            "quantity_branch",
            "in",
        ),
        edge(
            "e_quantity_result",
            "quantity",
            "result",
            "quantity_branch",
            "predicate",
        ),
        edge(
            "e_q_true_snapshot",
            "quantity_branch",
            "true",
            "switch_snapshot",
            "in",
        ),
        edge(
            "e_q_false_halt",
            "quantity_branch",
            "false",
            "end_halt",
            "in",
        ),
        edge(
            "e_q_unknown_halt",
            "quantity_branch",
            "unknown",
            "end_halt",
            "in",
        ),
        edge(
            "e_q_error_halt",
            "quantity_branch",
            "error",
            "end_halt",
            "in",
        ),
        edge(
            "e_switch_condition",
            "switch_snapshot",
            "next",
            "condition",
            "in",
        ),
        edge(
            "e_switch_value",
            "switch_snapshot",
            "value",
            "condition",
            "left",
        ),
        edge(
            "e_condition_branch",
            "condition",
            "next",
            "condition_branch",
            "in",
        ),
        edge(
            "e_condition_result",
            "condition",
            "result",
            "condition_branch",
            "predicate",
        ),
        edge(
            "e_c_true_desired",
            "condition_branch",
            "true",
            "desired",
            "in",
        ),
        edge(
            "e_c_false_halt",
            "condition_branch",
            "false",
            "end_halt",
            "in",
        ),
        edge(
            "e_c_unknown_halt",
            "condition_branch",
            "unknown",
            "end_halt",
            "in",
        ),
        edge(
            "e_c_error_halt",
            "condition_branch",
            "error",
            "end_halt",
            "in",
        ),
        edge("e_desired_delay", "desired", "next", "delay", "in"),
        edge("e_delay_admission", "delay", "next", "admission", "in"),
        edge("e_admission_fork", "admission", "admitted", "fork", "in"),
        edge("e_fork_effect", "fork", "branch:effect", "effect", "in"),
        edge(
            "e_effect_receipt",
            "effect",
            "receipt",
            "receipt",
            "receipt",
        ),
        edge(
            "e_receipt_join",
            "receipt",
            "accepted_for_delivery",
            "join",
            "arrival:effect",
        ),
        edge(
            "e_fork_noop_join",
            "fork",
            "branch:noop",
            "join",
            "arrival:noop",
        ),
        edge("e_join_end", "join", "next", "end_success", "in"),
    ];
    document(
        schema_digest,
        "cccccccccccccccccccccccccccccccc",
        1,
        "Every published node family in one coherent typed graph",
        TriggerNode::TimeSchedule {
            id: "trigger".to_owned(),
            version: 1,
            schedule: Schedule::WeeklyCivil {
                days: vec![Weekday::Monday, Weekday::Wednesday, Weekday::Friday],
                time: CivilTime {
                    hour: 2,
                    minute: 30,
                    second: 0,
                },
                timezone: "America/New_York".to_owned(),
                nonexistent_time: NonexistentTimePolicy::ShiftForward,
                duplicate_time: DuplicateTimePolicy::Both,
            },
        },
        nodes,
        edges,
    )
}

fn desired_paths_document(schema_digest: &ArtifactDigests) -> RuleDocument {
    let specifications = [
        (
            "on_off",
            selector("77777777777777777777777777777777", "on_off", "on-off"),
            StateFieldRef::OnOffCurrent,
            vec![DesiredProjectionValue::On(true)],
        ),
        (
            "level",
            selector("77777777777777777777777777777777", "level", "level-control"),
            StateFieldRef::LevelCurrentLevel,
            vec![DesiredProjectionValue::Level(DecimalValue::new("0.5"))],
        ),
        (
            "color",
            selector("77777777777777777777777777777777", "color", "color-control"),
            StateFieldRef::ColorCurrentColor,
            vec![DesiredProjectionValue::Color(
                ColorValue::ColorTemperature {
                    kelvin: DecimalValue::new("3000"),
                },
            )],
        ),
        (
            "thermostat",
            selector(
                "77777777777777777777777777777777",
                "thermostat",
                "thermostat",
            ),
            StateFieldRef::ThermostatCurrentMode,
            vec![
                DesiredProjectionValue::ThermostatMode(ThermostatMode::Auto),
                DesiredProjectionValue::ThermostatSingleTarget(DecimalValue::new("21")),
                DesiredProjectionValue::ThermostatHeatTarget(DecimalValue::new("20")),
                DesiredProjectionValue::ThermostatCoolTarget(DecimalValue::new("24")),
                DesiredProjectionValue::ThermostatFan(ChoiceValue::new("auto")),
                DesiredProjectionValue::ThermostatSwing(ChoiceValue::new("vertical")),
                DesiredProjectionValue::ThermostatPreset(ChoiceValue::new("home")),
            ],
        ),
        (
            "covering",
            selector(
                "77777777777777777777777777777777",
                "covering",
                "window-covering",
            ),
            StateFieldRef::CoveringCurrentPosition {
                axis: CoveringAxis::Lift,
            },
            vec![
                DesiredProjectionValue::CoveringLift(DecimalValue::new("0.5")),
                DesiredProjectionValue::CoveringTilt(DecimalValue::new("0.25")),
            ],
        ),
    ];
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut previous_node = "trigger".to_owned();
    let mut previous_port = "next".to_owned();
    for (index, (name, selector, field, values)) in specifications.into_iter().enumerate() {
        let snapshot = format!("snapshot_{name}");
        let patch = format!("patch_{name}");
        nodes.push(Node::Snapshot {
            id: snapshot.clone(),
            version: 1,
            selector,
            field,
        });
        let mut operations = Vec::new();
        for value in values {
            operations.push(DesiredPatchOperation::Test {
                expected: value.clone(),
            });
            operations.push(DesiredPatchOperation::Replace { value });
        }
        nodes.push(Node::DesiredPatch {
            id: patch.clone(),
            version: 1,
            snapshot_node_id: snapshot.clone(),
            operations,
        });
        edges.push(edge(
            &format!("e_{index}_snapshot"),
            &previous_node,
            &previous_port,
            &snapshot,
            "in",
        ));
        edges.push(edge(
            &format!("e_{index}_patch"),
            &snapshot,
            "next",
            &patch,
            "in",
        ));
        previous_node = patch;
        previous_port = "next".to_owned();
    }
    nodes.push(end("end", EndOutcome::Success));
    edges.push(edge(
        "e_patch_end",
        &previous_node,
        &previous_port,
        "end",
        "in",
    ));
    document(
        schema_digest,
        "12121212121212121212121212121212",
        1,
        "Every closed writable desired projection path and exact value type",
        TriggerNode::TimeSchedule {
            id: "trigger".to_owned(),
            version: 1,
            schedule: Schedule::FixedInterval {
                interval_ms: 60_000,
            },
        },
        nodes,
        edges,
    )
}

fn time_document(
    schema_digest: &ArtifactDigests,
    rule_id: &str,
    schedule: Schedule,
) -> RuleDocument {
    document(
        schema_digest,
        rule_id,
        1,
        "Explicit bounded time schedule",
        TriggerNode::TimeSchedule {
            id: "trigger".to_owned(),
            version: 1,
            schedule,
        },
        vec![end("end", EndOutcome::Noop)],
        vec![edge("e_trigger_end", "trigger", "next", "end", "in")],
    )
}

fn weekly_schedule() -> Schedule {
    Schedule::WeeklyCivil {
        days: vec![Weekday::Monday, Weekday::Friday],
        time: CivilTime {
            hour: 9,
            minute: 30,
            second: 0,
        },
        timezone: "Europe/London".to_owned(),
        nonexistent_time: NonexistentTimePolicy::Skip,
        duplicate_time: DuplicateTimePolicy::First,
    }
}

fn single_operation_document(
    schema_digest: &ArtifactDigests,
    rule_id: &str,
    operation: CapabilityOperation,
    route_every_receipt: bool,
) -> RuleDocument {
    let operation_kind = operation.kind();
    let effect = Node::EffectIntent {
        id: "effect".to_owned(),
        version: 1,
        selector: selector(
            "44444444444444444444444444444444",
            "target",
            operation_kind.package(),
        ),
        required_receipt: if operation.is_notification() {
            ReceiptRequirement::NotificationAdvertisedMaximum
        } else {
            ReceiptRequirement::AcceptedForDelivery
        },
        operation,
        deadline_ms: 30_000,
        retry: RetryPolicy::None,
    };
    let mut edges = vec![
        edge("e_trigger_effect", "trigger", "next", "effect", "in"),
        edge(
            "e_effect_receipt",
            "effect",
            "receipt",
            "receipt",
            "receipt",
        ),
    ];
    let receipt_ports = [
        "accepted_for_delivery",
        "protocol_or_target_acknowledged",
        "completed",
        "observed_completed",
        "rejected",
        "error",
        "cancelled",
        "outcome_unknown",
    ];
    for port in if route_every_receipt {
        receipt_ports.as_slice()
    } else {
        &receipt_ports[..1]
    } {
        edges.push(edge(
            &format!("e_receipt_{port}"),
            "receipt",
            port,
            "end",
            "in",
        ));
    }
    document(
        schema_digest,
        rule_id,
        1,
        "One exact typed capability operation",
        TriggerNode::DiscreteEvent {
            id: "trigger".to_owned(),
            version: 1,
            selector: selector(
                "33333333333333333333333333333333",
                "occurrence",
                "boolean-state",
            ),
            event: ContractEventRef::BooleanState {
                event: BooleanStateEventKind::Occurrence,
            },
            deduplication: EventDeduplicationPolicy::ProcessEpochAtLeastOnce,
        },
        vec![
            effect,
            Node::ReceiptBranch {
                id: "receipt".to_owned(),
                version: 1,
                effect_node_id: "effect".to_owned(),
            },
            end("end", EndOutcome::Success),
        ],
        edges,
    )
}

fn maximal_document(schema_digest: &ArtifactDigests) -> RuleDocument {
    let branches = vec![
        ForkBranch {
            id: "a".to_owned(),
            order: 0,
        },
        ForkBranch {
            id: "b".to_owned(),
            order: 1,
        },
        ForkBranch {
            id: "c".to_owned(),
            order: 2,
        },
        ForkBranch {
            id: "d".to_owned(),
            order: 3,
        },
    ];
    let nodes = vec![
        Node::ForkJoin {
            id: "fork".to_owned(),
            version: 1,
            phase: ForkJoinPhase::Fork {
                join_node_id: "join".to_owned(),
                branches: branches.clone(),
            },
            conflict_policy: PatchConflictPolicy::OrderedLastWins,
        },
        Node::ForkJoin {
            id: "join".to_owned(),
            version: 1,
            phase: ForkJoinPhase::Join {
                fork_node_id: "fork".to_owned(),
                branches: branches.clone(),
            },
            conflict_policy: PatchConflictPolicy::OrderedLastWins,
        },
        end("end", EndOutcome::Success),
    ];
    let mut edges = vec![edge("e_trigger_fork", "trigger", "next", "fork", "in")];
    for branch in &branches {
        edges.push(edge(
            &format!("e_{}_join", branch.id),
            "fork",
            &format!("branch:{}", branch.id),
            "join",
            &format!("arrival:{}", branch.id),
        ));
    }
    edges.push(edge("e_join_end", "join", "next", "end", "in"));
    let mut result = document(
        schema_digest,
        "ffffffffffffffffffffffffffffffff",
        u64::MAX,
        &"n".repeat(128),
        TriggerNode::TimeSchedule {
            id: "trigger".to_owned(),
            version: 1,
            schedule: Schedule::WeeklyCivil {
                days: vec![
                    Weekday::Monday,
                    Weekday::Tuesday,
                    Weekday::Wednesday,
                    Weekday::Thursday,
                    Weekday::Friday,
                    Weekday::Saturday,
                    Weekday::Sunday,
                ],
                time: CivilTime {
                    hour: 23,
                    minute: 59,
                    second: 59,
                },
                timezone: "A".repeat(64),
                nonexistent_time: NonexistentTimePolicy::Skip,
                duplicate_time: DuplicateTimePolicy::Second,
            },
        },
        nodes,
        edges,
    );
    result.semantic.description = "d".repeat(4_096);
    result.semantic.annotations.clear();
    for index in 0..64 {
        result.semantic.annotations.insert(
            NamespacedAnnotationKey::new(format!("fixture:key{index:02}")),
            AnnotationValue::new("v".repeat(1_024)),
        );
    }
    result.editor.nodes.clear();
    for index in 0..128 {
        result.editor.nodes.insert(
            EditorNodeKey::new(format!("editor_{index:03}")),
            EditorNodeData {
                x: i32::MIN + index,
                y: i32::MAX - index,
                collapsed: index % 2 == 0,
                group: None,
                presentation: EditorPresentation::Compact,
            },
        );
    }
    for index in 0..32 {
        result.editor.groups.insert(
            EditorGroupKey::new(format!("group_{index:02}")),
            EditorGroupData {
                label: "g".repeat(128),
                x: i32::MIN,
                y: i32::MAX,
                width: 100_000,
                height: 100_000,
                collapsed: index % 2 == 0,
            },
        );
    }
    result.editor.viewport = EditorViewport {
        x: i32::MIN,
        y: i32::MAX,
        zoom_milli: 4_000,
    };
    result
}

fn document(
    schema_digest: &ArtifactDigests,
    rule_id: &str,
    revision: u64,
    name: &str,
    trigger: TriggerNode,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
) -> RuleDocument {
    let mut annotations = BTreeMap::new();
    annotations.insert(
        NamespacedAnnotationKey::new("litehouse:fixture"),
        AnnotationValue::new("generated"),
    );
    let mut editor_nodes = BTreeMap::new();
    editor_nodes.insert(
        EditorNodeKey::new("trigger"),
        EditorNodeData {
            x: -120,
            y: 40,
            collapsed: false,
            group: None,
            presentation: EditorPresentation::Emphasized,
        },
    );
    RuleDocument {
        schema: SchemaLock {
            url: RuleSchemaUrl::V1,
            version: RuleSchemaVersion::V1,
            sha256: schema_digest.schema.clone(),
            profile_sha256: schema_digest.profile.clone(),
        },
        rule_id: rule_id.to_owned(),
        revision,
        semantic: SemanticRule {
            name: name.to_owned(),
            description: "Deterministically generated Rule JSON V1 fixture.".to_owned(),
            locks: SemanticLocks {
                compiler: CompilerArtifactLock {
                    id: CompilerArtifactId::V1,
                    sha256: schema_digest.compiler.clone(),
                },
                cel: CelArtifactLock {
                    id: CelArtifactId::V1,
                    sha256: schema_digest.cel.clone(),
                },
                quantity_catalog: QuantityCatalogArtifactLock {
                    id: QuantityCatalogArtifactId::V1,
                    sha256: schema_digest.quantity_catalog.clone(),
                },
                runtime_profile: RuntimeProfileArtifactLock {
                    id: RuntimeProfileArtifactId::V1,
                    sha256: schema_digest.runtime_profile.clone(),
                },
                timezone_database: TimezoneDatabaseArtifactLock {
                    id: TimezoneDatabaseArtifactId::V1,
                    sha256: schema_digest.timezone_database.clone(),
                },
                contract_catalog: ContractCatalogArtifactLock {
                    id: ContractCatalogArtifactId::V1,
                    sha256: schema_digest.contract_catalog.clone(),
                },
                management_operations: ManagementOperationsArtifactLock {
                    id: ManagementOperationsArtifactId::V1,
                    sha256: schema_digest.management_operations.clone(),
                },
                port_catalog: PortCatalogArtifactLock {
                    id: PortCatalogArtifactId::V1,
                    sha256: schema_digest.port_catalog.clone(),
                },
            },
            policies: SemanticPolicies {
                activation: ActivationPolicy::Explicit,
                dependency_hydration: DependencyHydrationPolicy::WaitUntilHydratedOrUnavailable,
                trigger_queue: TriggerQueuePolicy::Coalesce,
            },
            limits: RuleLimits::hard_maxima(),
            annotations,
            trigger,
            nodes,
            edges,
        },
        editor: EditorData {
            revision: 1,
            nodes: editor_nodes,
            groups: BTreeMap::new(),
            viewport: EditorViewport {
                x: 0,
                y: 0,
                zoom_milli: 1_000,
            },
        },
    }
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

fn edge(id: &str, from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> Edge {
    Edge {
        id: id.to_owned(),
        from: EdgeEndpoint {
            node: from_node.to_owned(),
            port: from_port.to_owned(),
        },
        to: EdgeEndpoint {
            node: to_node.to_owned(),
            port: to_port.to_owned(),
        },
        order: 0,
    }
}

fn end(id: &str, outcome: EndOutcome) -> Node {
    Node::End {
        id: id.to_owned(),
        version: 1,
        outcome,
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn all_operations() -> Vec<CapabilityOperation> {
    let decimal = || DecimalValue::new("0.5");
    let color = || ColorValue::CalibratedRgb {
        red: DecimalValue::new("0.1"),
        green: DecimalValue::new("0.2"),
        blue: DecimalValue::new("0.3"),
        color_space: ColorSpace::Srgb,
    };
    vec![
        CapabilityOperation::OnOffSet { on: true },
        CapabilityOperation::OnOffToggle,
        CapabilityOperation::LevelSet { level: decimal() },
        CapabilityOperation::LevelStep { step: decimal() },
        CapabilityOperation::LevelMove {
            direction: LevelDirection::Up,
            rate_per_second: decimal(),
        },
        CapabilityOperation::LevelStop,
        CapabilityOperation::LevelTransition {
            level: decimal(),
            duration_ms: 1_000,
        },
        CapabilityOperation::LevelSetCoupled {
            level: decimal(),
            on_when_nonzero: true,
        },
        CapabilityOperation::LevelStepCoupled {
            step: decimal(),
            on_when_nonzero: true,
        },
        CapabilityOperation::LevelMoveCoupled {
            direction: LevelDirection::Down,
            rate_per_second: decimal(),
            on_when_nonzero: false,
        },
        CapabilityOperation::LevelTransitionCoupled {
            level: decimal(),
            duration_ms: 2_000,
            on_when_nonzero: true,
        },
        CapabilityOperation::ColorSet { color: color() },
        CapabilityOperation::ColorStep {
            color: color(),
            direction: ColorDirection::Increase,
        },
        CapabilityOperation::ColorMove {
            color: color(),
            direction: ColorDirection::Decrease,
        },
        CapabilityOperation::ColorStop,
        CapabilityOperation::ColorTransition {
            color: color(),
            duration_ms: 1_500,
        },
        CapabilityOperation::ThermostatSetMode {
            mode: ThermostatMode::Heat,
        },
        CapabilityOperation::ThermostatSetSingleTarget {
            target_celsius: DecimalValue::new("21.5"),
        },
        CapabilityOperation::ThermostatSetDualTarget {
            heat_target_celsius: DecimalValue::new("20"),
            cool_target_celsius: DecimalValue::new("24"),
        },
        CapabilityOperation::ThermostatSetFan {
            fan_choice: ChoiceValue::new("auto"),
        },
        CapabilityOperation::ThermostatSetSwing {
            swing_choice: ChoiceValue::new("vertical"),
        },
        CapabilityOperation::ThermostatSetPreset {
            preset_choice: ChoiceValue::new("home"),
        },
        CapabilityOperation::CoveringOpen {
            axis: CoveringAxis::Lift,
        },
        CapabilityOperation::CoveringClose {
            axis: CoveringAxis::Tilt,
        },
        CapabilityOperation::CoveringStop {
            axis: CoveringAxis::Lift,
        },
        CapabilityOperation::CoveringSetLift {
            position: decimal(),
        },
        CapabilityOperation::CoveringSetTilt {
            position: decimal(),
        },
        CapabilityOperation::MomentaryInvoke,
        CapabilityOperation::NotificationSend {
            notification_template_id: "arrival".to_owned(),
            priority: NotificationPriority::High,
            expires_in_ms: Some(60_000),
        },
    ]
}

#[allow(dead_code)]
fn _closed_catalog_marker(_: QuantityCatalogRef, _: StatefulContractKind) {}
