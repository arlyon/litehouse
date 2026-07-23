use esphome_native_api::{
    esphomeapi::EspHomeApi,
    parser::ProtoMessage,
    proto::{
        BinarySensorStateResponse, ListEntitiesBinarySensorResponse, ListEntitiesDoneResponse,
        ListEntitiesSwitchResponse, SwitchStateResponse,
    },
};
use litehouse_management::{
    FacetQueryResult, FacetState, InventorySnapshot, ManagementService, McpAdapter, Principal,
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
use tokio::net::TcpListener;
use wasmtime::{Config, Engine, component::Component};

const FIXTURE_KEY: &str = "AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=";

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
#[ignore = "requires LITEHOUSE_ESPHOME_COMPONENT built for wasm32-wasip2"]
async fn noise_to_wasmtime_to_canonical_handlers_to_rmcp_is_one_path() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let noise_server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await?;
        let api = EspHomeApi::builder()
            .name("fixture".into())
            .encryption_key(FIXTURE_KEY.into())
            .api_version_major(1)
            .api_version_minor(14)
            .build();
        let (tx, mut rx) = api
            .start(stream)
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))?;
        while let Ok(message) = rx.recv().await {
            match message {
                ProtoMessage::ListEntitiesRequest(_) => {
                    tx.send(ProtoMessage::ListEntitiesSwitchResponse(switch_entity()))
                        .await?;
                    tx.send(ProtoMessage::ListEntitiesBinarySensorResponse(
                        binary_entity(),
                    ))
                    .await?;
                    tx.send(ProtoMessage::ListEntitiesDoneResponse(
                        ListEntitiesDoneResponse {},
                    ))
                    .await?;
                }
                ProtoMessage::SubscribeStatesRequest(_) => {
                    tx.send(ProtoMessage::SwitchStateResponse(SwitchStateResponse {
                        key: 11,
                        state: false,
                        device_id: 1,
                    }))
                    .await?;
                    tx.send(ProtoMessage::BinarySensorStateResponse(
                        BinarySensorStateResponse {
                            key: 21,
                            state: true,
                            missing_state: false,
                            device_id: 1,
                        },
                    ))
                    .await?;
                }
                ProtoMessage::SwitchCommandRequest(command) => {
                    tx.send(ProtoMessage::SwitchStateResponse(SwitchStateResponse {
                        key: command.key,
                        state: command.state,
                        device_id: command.device_id,
                    }))
                    .await?;
                }
                _ => {}
            }
        }
        anyhow::Ok(())
    });

    let engine = engine();
    let epoch_engine = engine.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(10));
        loop {
            ticker.tick().await;
            epoch_engine.increment_epoch();
        }
    });
    let component = Component::from_file(&engine, std::env::var("LITEHOUSE_ESPHOME_COMPONENT")?)?;
    let generation = fabric::Generation::new(3)?;
    let handle = start_provider(
        &engine,
        &component,
        ProviderConfig {
            instance: fabric::InstanceId::new("esphome-fixture")?,
            generation,
            config_json: format!(r#"{{"address":"{address}","noise_key":"{FIXTURE_KEY}"}}"#),
            allowed_tcp_addresses: BTreeSet::from([address]),
            allowed_http_origins: BTreeSet::new(),
            startup_timeout: Duration::from_secs(10),
        },
        Arc::new(Mutex::new(fabric::ServiceFabric::default())),
    )
    .await?;

    let adapter = McpAdapter::new(
        ManagementService::for_esphome(handle),
        Principal::proving("mcp-esphome-fixture")?,
    );
    let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
    let mcp_server = tokio::spawn(async move {
        adapter.serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });
    let client = ClientInfo::default().serve(client_transport).await?;

    let inventory = client
        .call_tool(CallToolRequestParams::new("litehouse_inventory"))
        .await?;
    let inventory: InventorySnapshot =
        serde_json::from_value(inventory.structured_content.unwrap())?;
    assert_eq!(inventory.instances[0].facets.len(), 2);
    let relay = inventory.instances[0]
        .facets
        .iter()
        .find(|facet| facet.label == "Relay 1")
        .unwrap();
    let relay_id = relay.facet_id.clone();
    let relay_generation = relay.generation;

    let initial = query(&client, &relay_id, relay_generation).await?;
    assert_eq!(
        initial.state,
        FacetState::OnOff {
            current: Some(false)
        }
    );

    interact(
        &client,
        &relay_id,
        relay_generation,
        true,
        "0123456789abcdef0123456789abcdef",
    )
    .await?;
    let changed = query(&client, &relay_id, relay_generation).await?;
    assert_eq!(
        changed.state,
        FacetState::OnOff {
            current: Some(true)
        }
    );

    interact(
        &client,
        &relay_id,
        relay_generation,
        false,
        "fedcba9876543210fedcba9876543210",
    )
    .await?;
    let restored = query(&client, &relay_id, relay_generation).await?;
    assert_eq!(
        restored.state,
        FacetState::OnOff {
            current: Some(false)
        }
    );

    client.cancel().await?;
    mcp_server.await??;
    noise_server.abort();
    Ok(())
}

async fn query(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ClientInfo>,
    facet_id: &str,
    generation: u64,
) -> anyhow::Result<FacetQueryResult> {
    let result = client
        .call_tool(
            CallToolRequestParams::new("litehouse_query").with_arguments(arguments(json!({
                "facet_id": facet_id,
                "facet_generation": generation,
                "timeout_ms": 5000
            }))),
        )
        .await?;
    Ok(serde_json::from_value(result.structured_content.unwrap())?)
}

async fn interact(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ClientInfo>,
    facet_id: &str,
    generation: u64,
    on: bool,
    idempotency_key: &str,
) -> anyhow::Result<()> {
    let result = client
        .call_tool(
            CallToolRequestParams::new("litehouse_interact").with_arguments(arguments(json!({
                "operation": "capability.on-off.set@1",
                "facet_id": facet_id,
                "facet_generation": generation,
                "on": on,
                "timeout_ms": 5000,
                "idempotency_key": idempotency_key
            }))),
        )
        .await?;
    assert_eq!(result.is_error, Some(false));
    Ok(())
}

fn switch_entity() -> ListEntitiesSwitchResponse {
    ListEntitiesSwitchResponse {
        object_id: "relay_1".into(),
        key: 11,
        name: "Relay 1".into(),
        icon: String::new(),
        assumed_state: false,
        disabled_by_default: false,
        entity_category: 0,
        device_class: "switch".into(),
        device_id: 1,
    }
}

fn binary_entity() -> ListEntitiesBinarySensorResponse {
    ListEntitiesBinarySensorResponse {
        object_id: "button_1".into(),
        key: 21,
        name: "Button 1".into(),
        device_class: String::new(),
        is_status_binary_sensor: false,
        disabled_by_default: false,
        icon: String::new(),
        entity_category: 0,
        device_id: 1,
    }
}
