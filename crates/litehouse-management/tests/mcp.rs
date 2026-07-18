use litehouse_contracts::management::management_manifest;
use litehouse_management::{
    ErrorCode, FacetQueryResult, FacetState, InventorySnapshot, ManagementService, McpAdapter,
    OnOffSetRequest, OnOffSetResult, Principal,
};
use litehouse_provider_host::{ProviderConfig, start_provider};
use litehouse_service_fabric as fabric;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientInfo},
};
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Duration,
};
use wasmtime::{Config, Engine, component::Component};

fn engine() -> Engine {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .wasm_component_model_async(true)
        .concurrency_support(true)
        .epoch_interruption(true);
    Engine::new(&config).unwrap()
}

fn arguments(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires LITEHOUSE_PROVIDER_FIXTURE_WASM built for wasm32-wasip2"]
async fn three_rmcp_tools_use_canonical_handlers_and_no_generic_dispatch() -> anyhow::Result<()> {
    let engine = engine();
    let epoch_engine = engine.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(10));
        loop {
            ticker.tick().await;
            epoch_engine.increment_epoch();
        }
    });
    let component =
        Component::from_file(&engine, std::env::var("LITEHOUSE_PROVIDER_FIXTURE_WASM")?)?;
    let generation = fabric::Generation::new(7)?;
    let handle = start_provider(
        &engine,
        &component,
        ProviderConfig {
            instance: fabric::InstanceId::new("fixture-node")?,
            generation,
            config_json: "fixture".into(),
            allowed_tcp_addresses: BTreeSet::new(),
            allowed_http_origins: BTreeSet::new(),
            startup_timeout: Duration::from_secs(5),
        },
        Arc::new(Mutex::new(fabric::ServiceFabric::default())),
    )
    .await?;

    let management = ManagementService::for_esphome(handle);
    let denied = management
        .on_off_set(
            &Principal::new(
                "read-only-principal",
                [litehouse_contracts::management::OAuthScope::Read],
            )?,
            OnOffSetRequest {
                facet_id: "missing-facet".into(),
                facet_generation: generation.get(),
                on: true,
                timeout_ms: Some(5_000),
                idempotency_key: "0123456789abcdef0123456789abcdef".into(),
            },
        )
        .await
        .unwrap_err();
    assert_eq!(denied.inner.code, ErrorCode::InsufficientScope);

    let adapter = McpAdapter::new(management, Principal::proving("mcp-proving-principal")?);
    let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
    let server = tokio::spawn(async move {
        adapter.serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });
    let client = ClientInfo::default().serve(client_transport).await?;

    let tools = client.list_tools(None).await?.tools;
    let names = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        names,
        BTreeSet::from([
            "litehouse_interact",
            "litehouse_inventory",
            "litehouse_query"
        ])
    );
    let manifest = management_manifest()?;
    for tool in &tools {
        let operation = manifest
            .operations
            .iter()
            .find(|operation| operation.mcp.tool == tool.name)
            .unwrap();
        assert_eq!(
            tool.input_schema.as_ref(),
            operation.schemas.mcp_projection.schema.as_object().unwrap()
        );
        assert_eq!(
            tool.output_schema.as_deref().unwrap(),
            operation.schemas.output.schema.as_object().unwrap()
        );
    }

    let inventory = client
        .call_tool(
            CallToolRequestParams::new("litehouse_inventory")
                .with_arguments(arguments(json!({"timeout_ms": 5000}))),
        )
        .await?;
    assert_eq!(inventory.is_error, Some(false));
    let inventory: InventorySnapshot =
        serde_json::from_value(inventory.structured_content.unwrap())?;
    assert_eq!(inventory.instances.len(), 1);
    assert_eq!(inventory.instances[0].facets.len(), 10);
    let relay = inventory.instances[0]
        .facets
        .iter()
        .find(|facet| facet.label == "Relay 1")
        .unwrap();
    assert_eq!(
        relay.operations,
        ["facets.query@1", "capability.on-off.set@1"]
    );
    let dimmer = inventory.instances[0]
        .facets
        .iter()
        .find(|facet| facet.label == "Dimmer 1")
        .unwrap();
    let level_query = client
        .call_tool(
            CallToolRequestParams::new("litehouse_query").with_arguments(arguments(json!({
                "facet_id": dimmer.facet_id,
                "facet_generation": dimmer.generation,
                "timeout_ms": 5000
            }))),
        )
        .await?;
    let level_query: FacetQueryResult =
        serde_json::from_value(level_query.structured_content.unwrap())?;
    assert!(matches!(level_query.state, FacetState::Level { .. }));

    let query_arguments = json!({
        "facet_id": relay.facet_id,
        "facet_generation": relay.generation,
        "timeout_ms": 5000
    });
    let before = client
        .call_tool(
            CallToolRequestParams::new("litehouse_query")
                .with_arguments(arguments(query_arguments.clone())),
        )
        .await?;
    let before: FacetQueryResult = serde_json::from_value(before.structured_content.unwrap())?;
    assert_eq!(
        before.state,
        FacetState::OnOff {
            current: Some(false)
        }
    );

    let set = client
        .call_tool(
            CallToolRequestParams::new("litehouse_interact").with_arguments(arguments(json!({
                "facet_id": relay.facet_id,
                "facet_generation": relay.generation,
                "on": true,
                "timeout_ms": 5000,
                "idempotency_key": "0123456789abcdef0123456789abcdef"
            }))),
        )
        .await?;
    assert_eq!(set.is_error, Some(false));
    let set: OnOffSetResult = serde_json::from_value(set.structured_content.unwrap())?;
    assert!(!set.deduplicated);

    let after = client
        .call_tool(
            CallToolRequestParams::new("litehouse_query")
                .with_arguments(arguments(query_arguments.clone())),
        )
        .await?;
    let after: FacetQueryResult = serde_json::from_value(after.structured_content.unwrap())?;
    assert_eq!(
        after.state,
        FacetState::OnOff {
            current: Some(true)
        }
    );
    assert!(after.revision > before.revision);

    let repeated = client
        .call_tool(
            CallToolRequestParams::new("litehouse_interact").with_arguments(arguments(json!({
                "facet_id": relay.facet_id,
                "facet_generation": relay.generation,
                "on": true,
                "timeout_ms": 5000,
                "idempotency_key": "0123456789abcdef0123456789abcdef"
            }))),
        )
        .await?;
    let repeated: OnOffSetResult = serde_json::from_value(repeated.structured_content.unwrap())?;
    assert!(repeated.deduplicated);
    assert_eq!(repeated.receipt_id, set.receipt_id);
    let after_repeat = client
        .call_tool(
            CallToolRequestParams::new("litehouse_query")
                .with_arguments(arguments(query_arguments)),
        )
        .await?;
    let after_repeat: FacetQueryResult =
        serde_json::from_value(after_repeat.structured_content.unwrap())?;
    assert_eq!(after_repeat.revision, after.revision);

    assert!(
        client
            .call_tool(CallToolRequestParams::new("litehouse_toggle"))
            .await
            .is_err()
    );
    client.cancel().await?;
    server.await??;
    Ok(())
}
