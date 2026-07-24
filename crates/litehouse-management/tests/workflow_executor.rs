use litehouse_management::{
    BundledProvider, FacetQueryRequest, FacetState, ManagedProvider, ManagementService, Principal,
    WorkflowExecutor, WorkflowRepository, WorkflowRunStatus, WorkflowRuntime,
};
use litehouse_provider_host::{ProviderConfig, start_provider};
use litehouse_service_fabric as fabric;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use wasmtime::{Config, Engine, component::Component};

const STATE_RULE_ID: &str = "a1111111111111111111111111111111";
const DISCRETE_RULE_ID: &str = "b2222222222222222222222222222222";
const SCHEDULE_RULE_ID: &str = "c3333333333333333333333333333333";

fn engine() -> Engine {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .wasm_component_model_async(true)
        .concurrency_support(true)
        .epoch_interruption(true);
    Engine::new(&config).unwrap()
}

fn selector(package: &str, facet_key: &str) -> Value {
    json!({
        "cardinality": "one",
        "contract": {
            "interface": "capability",
            "major": 1,
            "namespace": "litehouse",
            "package": package
        },
        "facet_key": facet_key,
        "provider_instance_id": "f1111111111111111111111111111111",
        "rebind": "same_identity"
    })
}

fn state_effect_workflow(rule_id: &str, effect_facet: &str) -> Value {
    let mut document: Value = serde_json::from_str(include_str!(
        "../../litehouse-rule-json/fixtures/valid-planner-compat.json"
    ))
    .unwrap();
    document["rule_id"] = json!(rule_id);
    document["semantic"]["trigger"]["selector"] = selector("on-off", "relay-2");
    document["semantic"]["trigger"]["initial"] = json!("run");
    for node in document["semantic"]["nodes"].as_array_mut().unwrap() {
        match node["id"].as_str() {
            Some("snapshot") => node["selector"] = selector("on-off", "relay-2"),
            Some("set_light") => node["selector"] = selector("on-off", effect_facet),
            _ => {}
        }
    }
    document
}

fn discrete_failure_workflow() -> Value {
    let mut document = state_effect_workflow(DISCRETE_RULE_ID, "relay-2");
    document["semantic"]["trigger"] = json!({
        "deduplication": "process_epoch_at_least_once",
        "event": {
            "contract": "litehouse:boolean-state@1.0.0",
            "event": "occurrence"
        },
        "id": "trigger",
        "kind": "discrete_event",
        "selector": selector("boolean-state", "button-1"),
        "version": 1
    });
    document
}

fn scheduled_timer_workflow(scheduled_for: u64) -> Value {
    let mut document: Value = serde_json::from_str(include_str!(
        "../../litehouse-rule-json/fixtures/valid-time-one-shot.json"
    ))
    .unwrap();
    document["rule_id"] = json!(SCHEDULE_RULE_ID);
    document["semantic"]["trigger"]["schedule"]["at"] = json!({
        "nanosecond": 0,
        "unix_seconds": scheduled_for
    });
    document["semantic"]["nodes"] = json!([
        {
            "duration": { "milliseconds": 100 },
            "id": "delay",
            "kind": "delay",
            "version": 1
        },
        {
            "id": "end",
            "kind": "end",
            "outcome": "success",
            "version": 1
        }
    ]);
    document["semantic"]["edges"] = json!([
        {
            "from": { "node": "trigger", "port": "next" },
            "id": "e_trigger_delay",
            "order": 0,
            "to": { "node": "delay", "port": "in" }
        },
        {
            "from": { "node": "delay", "port": "next" },
            "id": "e_delay_end",
            "order": 0,
            "to": { "node": "end", "port": "in" }
        }
    ]);
    document
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "run through scripts/test-provider-workflows.sh after building wasm32-wasip2 fixture"]
async fn provider_component_drives_triggers_receipts_timers_outcomes_and_failure_isolation() {
    let fixture = std::env::var("LITEHOUSE_PROVIDER_FIXTURE_WASM")
        .expect("fixture component path is required for the ignored integration test");
    let engine = engine();
    let epoch_engine = engine.clone();
    let epoch_task = tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(10));
        loop {
            ticker.tick().await;
            epoch_engine.increment_epoch();
        }
    });
    let component = Component::from_file(&engine, fixture).unwrap();
    let fabric = Arc::new(Mutex::new(fabric::ServiceFabric::default()));
    let generation = fabric::Generation::new(7).unwrap();
    let handle = start_provider(
        &engine,
        &component,
        ProviderConfig {
            instance: fabric::InstanceId::new("f1111111111111111111111111111111").unwrap(),
            generation,
            config_json: "fixture".to_owned(),
            allowed_tcp_addresses: BTreeSet::new(),
            allowed_http_origins: BTreeSet::new(),
            startup_timeout: Duration::from_secs(5),
        },
        fabric.clone(),
    )
    .await
    .unwrap();
    let management = ManagementService::from_bundled([ManagedProvider::new(
        BundledProvider::Esphome,
        handle.clone(),
    )])
    .unwrap();
    let principal = Principal::proving("provider-workflow-integration").unwrap();

    let root = std::env::temp_dir().join(format!(
        "litehouse-provider-workflows-{}",
        std::process::id()
    ));
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    std::fs::create_dir_all(&root).unwrap();
    let scheduled_for = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3;
    std::fs::write(
        root.join("state.json"),
        serde_json::to_vec_pretty(&state_effect_workflow(STATE_RULE_ID, "relay-1")).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("discrete.json"),
        serde_json::to_vec_pretty(&discrete_failure_workflow()).unwrap(),
    )
    .unwrap();
    std::fs::write(
        root.join("schedule.json"),
        serde_json::to_vec_pretty(&scheduled_timer_workflow(scheduled_for)).unwrap(),
    )
    .unwrap();
    let repository = Arc::new(WorkflowRepository::open(&root).unwrap());
    let runtime = Arc::new(WorkflowRuntime::new());
    let mut executor = WorkflowExecutor::activate(
        management.clone(),
        principal.clone(),
        repository.snapshot().unwrap(),
        runtime.clone(),
    )
    .await
    .unwrap();
    let executor_task = tokio::spawn(async move { executor.run().await });

    let outcomes = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            let outcomes = runtime.outcomes(None).unwrap();
            if outcomes.terminal.len() == 3 {
                break outcomes;
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .unwrap();
    assert!(outcomes.active.is_empty());
    let status = |rule_id: &str| {
        outcomes
            .terminal
            .iter()
            .find(|outcome| outcome.rule_id == rule_id)
            .unwrap()
            .status
    };
    assert_eq!(status(STATE_RULE_ID), WorkflowRunStatus::Succeeded);
    assert_eq!(status(DISCRETE_RULE_ID), WorkflowRunStatus::Failed);
    assert_eq!(status(SCHEDULE_RULE_ID), WorkflowRunStatus::Succeeded);

    let relay = management
        .facet_query(
            &principal,
            FacetQueryRequest {
                facet_id: "relay-1".to_owned(),
                facet_generation: generation.get(),
                timeout_ms: Some(1_000),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        relay.state,
        FacetState::OnOff {
            current: Some(true)
        }
    );

    executor_task.abort();
    let _ = executor_task.await;
    handle.shutdown(Duration::from_secs(1)).await.unwrap();
    assert!(fabric.lock().unwrap().registry().is_empty());
    epoch_task.abort();
    let _ = epoch_task.await;
    std::fs::remove_dir_all(root).unwrap();
}
