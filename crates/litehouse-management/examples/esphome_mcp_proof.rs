use litehouse_management::{
    FacetQueryResult, FacetState, InventorySnapshot, ManagementError, ManagementService,
    McpAdapter, OnOffSetResult, Principal,
};
use litehouse_provider_host::{ProviderConfig, start_provider};
use litehouse_service_fabric as fabric;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, CallToolResult, ClientInfo},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Map, Value, json};
use std::{
    collections::BTreeSet,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use wasmtime::{Config, Engine, component::Component};
use zeroize::Zeroizing;

const RELAY_LABEL: &str = "Relay 1";
const CHANGE_KEY: &str = "11111111111111111111111111111111";
const RESTORE_KEY: &str = "22222222222222222222222222222222";

#[derive(Serialize)]
struct ProofEvidence {
    proof: &'static str,
    instance_id: String,
    facet_id: String,
    generation: u64,
    initial: bool,
    target: bool,
    change_receipt: OnOffSetResult,
    changed_revision: u64,
    restore_receipt: OnOffSetResult,
    restored_revision: u64,
    restored: bool,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    if std::env::var("LITEHOUSE_APPROVE_RELAY1_PROOF").as_deref() != Ok("yes") {
        anyhow::bail!("set LITEHOUSE_APPROVE_RELAY1_PROOF=yes for the approved Relay 1 proof");
    }
    let component_path = std::env::var("LITEHOUSE_ESPHOME_COMPONENT")?;
    let address = std::env::var("LITEHOUSE_ESPHOME_ADDRESS")?.parse::<SocketAddr>()?;
    let noise_key = Zeroizing::new(std::env::var("LITEHOUSE_ESPHOME_NOISE_KEY")?);
    let instance_id =
        std::env::var("LITEHOUSE_ESPHOME_INSTANCE").unwrap_or_else(|_| "esphome-node".into());

    let engine = engine()?;
    let epoch_engine = engine.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_millis(10));
        loop {
            ticker.tick().await;
            epoch_engine.increment_epoch();
        }
    });
    let component = Component::from_file(&engine, component_path)?;
    let generation = fabric::Generation::new(1)?;
    let handle = start_provider(
        &engine,
        &component,
        ProviderConfig {
            instance: fabric::InstanceId::new(&instance_id)?,
            generation,
            config_json: serde_json::to_string(&json!({
                "address": address.to_string(),
                "noise_key": noise_key.as_str()
            }))?,
            allowed_tcp_addresses: BTreeSet::from([address]),
            startup_timeout: Duration::from_secs(15),
        },
        Arc::new(Mutex::new(fabric::ServiceFabric::default())),
    )
    .await?;

    let adapter = McpAdapter::new(
        ManagementService::for_esphome(handle),
        Principal::proving("relay-1-proof")?,
    );
    let (server_transport, client_transport) = tokio::io::duplex(16 * 1024);
    let server = tokio::spawn(async move {
        adapter.serve(server_transport).await?.waiting().await?;
        anyhow::Ok(())
    });
    let client = ClientInfo::default().serve(client_transport).await?;

    let proof = run_proof(&client).await;
    client.cancel().await?;
    server.await??;
    let evidence = proof?;
    println!("{}", serde_json::to_string_pretty(&evidence)?);
    Ok(())
}

async fn run_proof(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ClientInfo>,
) -> anyhow::Result<ProofEvidence> {
    let inventory: InventorySnapshot = typed_result(
        client
            .call_tool(CallToolRequestParams::new("litehouse_inventory"))
            .await?,
    )?;
    let instance = inventory
        .instances
        .first()
        .ok_or_else(|| anyhow::anyhow!("inventory contains no provider instance"))?;
    let relay = instance
        .facets
        .iter()
        .filter(|facet| facet.label == RELAY_LABEL && facet.contract == "on-off@1")
        .collect::<Vec<_>>();
    if relay.len() != 1 {
        anyhow::bail!("inventory must contain exactly one Relay 1 On/Off facet");
    }
    let relay = relay[0];
    let initial_query = query(client, &relay.facet_id, relay.generation).await?;
    let initial = current_on_off(&initial_query)?;
    let target = !initial;

    let mutation = async {
        let receipt = interact(
            client,
            &relay.facet_id,
            relay.generation,
            target,
            CHANGE_KEY,
        )
        .await?;
        let observed = observe(client, &relay.facet_id, relay.generation, target).await?;
        anyhow::Ok((receipt, observed))
    }
    .await;

    // Once the mutation is attempted, restoration is mandatory even if its
    // receipt or observation was outcome-unknown.
    let restore_receipt = interact(
        client,
        &relay.facet_id,
        relay.generation,
        initial,
        RESTORE_KEY,
    )
    .await?;
    let restored = observe(client, &relay.facet_id, relay.generation, initial).await?;
    let (change_receipt, changed) = mutation?;

    Ok(ProofEvidence {
        proof: "inventory-query-absolute-set-observe-absolute-restore-observe",
        instance_id: instance.instance_id.clone(),
        facet_id: relay.facet_id.clone(),
        generation: relay.generation,
        initial,
        target,
        change_receipt,
        changed_revision: changed.revision,
        restore_receipt,
        restored_revision: restored.revision,
        restored: current_on_off(&restored)? == initial,
    })
}

async fn query(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ClientInfo>,
    facet_id: &str,
    generation: u64,
) -> anyhow::Result<FacetQueryResult> {
    typed_result(
        client
            .call_tool(
                CallToolRequestParams::new("litehouse_query").with_arguments(arguments(json!({
                    "facet_id": facet_id,
                    "facet_generation": generation,
                    "timeout_ms": 10000
                }))),
            )
            .await?,
    )
}

async fn interact(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ClientInfo>,
    facet_id: &str,
    generation: u64,
    on: bool,
    idempotency_key: &str,
) -> anyhow::Result<OnOffSetResult> {
    typed_result(
        client
            .call_tool(
                CallToolRequestParams::new("litehouse_interact").with_arguments(arguments(json!({
                    "facet_id": facet_id,
                    "facet_generation": generation,
                    "on": on,
                    "timeout_ms": 30000,
                    "idempotency_key": idempotency_key
                }))),
            )
            .await?,
    )
}

async fn observe(
    client: &rmcp::service::RunningService<rmcp::RoleClient, ClientInfo>,
    facet_id: &str,
    generation: u64,
    expected: bool,
) -> anyhow::Result<FacetQueryResult> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    loop {
        let state = query(client, facet_id, generation).await?;
        if current_on_off(&state)? == expected {
            return Ok(state);
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("timed out waiting for the requested Relay 1 observation");
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn current_on_off(result: &FacetQueryResult) -> anyhow::Result<bool> {
    match result.state {
        FacetState::OnOff {
            current: Some(value),
        } => Ok(value),
        _ => anyhow::bail!("Relay 1 has no current On/Off observation"),
    }
}

fn typed_result<T: DeserializeOwned>(result: CallToolResult) -> anyhow::Result<T> {
    let value = result
        .structured_content
        .ok_or_else(|| anyhow::anyhow!("MCP tool returned no structured content"))?;
    if result.is_error == Some(true) {
        let error = serde_json::from_value::<ManagementError>(value)?;
        anyhow::bail!(
            "canonical operation {} failed with {:?}",
            error.operation_id,
            error.inner.code
        );
    }
    Ok(serde_json::from_value(value)?)
}

fn arguments(value: Value) -> Map<String, Value> {
    value
        .as_object()
        .expect("tool arguments are objects")
        .clone()
}

fn engine() -> anyhow::Result<Engine> {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .wasm_component_model_async(true)
        .concurrency_support(true)
        .epoch_interruption(true);
    Ok(Engine::new(&config)?)
}
