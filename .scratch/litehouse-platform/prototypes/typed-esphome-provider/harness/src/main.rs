use anyhow::{Context, Result, anyhow, ensure};
use litehouse_contracts::identity::Version;
use litehouse_service_fabric as fabric;
use std::{collections::BTreeMap, path::PathBuf};
use wasmtime::component::{Component, Linker, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

mod provider_component {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "provider-world",
        exports: { default: async },
        require_store_data_send: true,
    });
}

struct StoreData {
    table: ResourceTable,
    wasi: WasiCtx,
}

impl WasiView for StoreData {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

fn descriptor(
    contract_name: &str,
    facet_key: &str,
    operations: &[&str],
    features: &[&str],
) -> Result<fabric::EndpointDescriptor> {
    Ok(fabric::EndpointDescriptor::new(
        fabric::EndpointIdentity::new(
            fabric::InstanceId::new("esphome-node")?,
            fabric::FacetKey::new(facet_key)?,
            fabric::ContractId::new("litehouse", contract_name, 1)?,
        ),
        Version::V1,
        fabric::Generation::new(1)?,
        fabric::Incarnation::new(1)?,
        operations
            .iter()
            .map(|operation| fabric::OperationId::new(*operation))
            .collect::<Result<Vec<_>, _>>()?,
        features
            .iter()
            .map(|feature| fabric::FeatureId::new(*feature))
            .collect::<Result<Vec<_>, _>>()?,
    )?)
}

#[tokio::main]
async fn main() -> Result<()> {
    let component_path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .context("usage: typed-esphome-provider-harness <component.wasm>")?;

    let mut config = Config::new();
    config.wasm_component_model(true);
    let engine = Engine::new(&config)?;
    let component = Component::from_file(&engine, &component_path)
        .map_err(|error| anyhow!("load {}: {error}", component_path.display()))?;
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
    let mut store = Store::new(
        &engine,
        StoreData {
            table: ResourceTable::new(),
            wasi: WasiCtxBuilder::new().build(),
        },
    );
    let bindings =
        provider_component::ProviderWorld::instantiate_async(&mut store, &component, &linker)
            .await?;

    let on_off = bindings.litehouse_esphome_resource_spike_on_off_provider();
    let level = bindings.litehouse_esphome_resource_spike_level_provider();
    let color = bindings.litehouse_esphome_resource_spike_color_provider();

    let mut endpoint_registry = fabric::EndpointRegistry::default();
    let mut on_off_handles = BTreeMap::new();
    for handle in on_off.call_discover(&mut store).await? {
        let key = on_off.endpoint().call_key(&mut store, handle).await?;
        endpoint_registry.publish(descriptor(
            "on-off",
            &key,
            &["query", "set"],
            &["observable"],
        )?)?;
        on_off_handles.insert(key, handle);
    }

    let mut level_handles = BTreeMap::new();
    for handle in level.call_discover(&mut store).await? {
        let key = level.endpoint().call_key(&mut store, handle).await?;
        endpoint_registry.publish(descriptor(
            "level-control",
            &key,
            &["query", "set-level"],
            &["observable", "target-level"],
        )?)?;
        level_handles.insert(key, handle);
    }

    let mut color_handles = BTreeMap::new();
    for handle in color.call_discover(&mut store).await? {
        let key = color.endpoint().call_key(&mut store, handle).await?;
        endpoint_registry.publish(descriptor(
            "color-control",
            &key,
            &["query", "set-color"],
            &["observable", "uncalibrated-rgb"],
        )?)?;
        color_handles.insert(key, handle);
    }

    ensure!(
        on_off_handles
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            == ["led-power", "switch-a", "switch-b"],
        "unexpected On/Off facets"
    );
    ensure!(
        level_handles.keys().map(String::as_str).collect::<Vec<_>>() == ["led-brightness"],
        "unexpected Level facets"
    );
    ensure!(
        color_handles.keys().map(String::as_str).collect::<Vec<_>>() == ["led-rgb"],
        "unexpected Color facets"
    );
    ensure!(
        endpoint_registry.len() == 5,
        "expected five typed endpoints"
    );

    let led_power = on_off_handles["led-power"];
    let led_level = level_handles["led-brightness"];
    let led_color = color_handles["led-rgb"];

    on_off
        .endpoint()
        .call_set(&mut store, led_power, true)
        .await?;
    ensure!(
        on_off
            .endpoint()
            .call_query(&mut store, led_power)
            .await?
            .current,
        "LED should be on"
    );

    level.endpoint().call_set(&mut store, led_level, 0).await?;
    ensure!(
        !on_off
            .endpoint()
            .call_query(&mut store, led_power)
            .await?
            .current,
        "setting shared LED brightness to zero should update its power facet"
    );

    color
        .endpoint()
        .call_set(
            &mut store,
            led_color,
            provider_component::litehouse::esphome_resource_spike::model::Rgb {
                red: 1,
                green: 2,
                blue: 3,
            },
        )
        .await?;
    ensure!(
        on_off
            .endpoint()
            .call_query(&mut store, led_power)
            .await?
            .current,
        "setting shared LED color should update its power facet"
    );

    println!(
        "typed ESPHome resource spike passed: 3 On/Off + 1 Level + 1 Color handles; 5 handle-free service-fabric descriptors"
    );
    Ok(())
}
