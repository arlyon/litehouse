#[cfg(test)]
use wasmtime::component::ResourceTable;
#[cfg(test)]
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

wasmtime::component::bindgen!({
    path: "../wit",
    world: "parity-guest",
    exports: { default: async },
    require_store_data_send: true,
});

#[cfg(test)]
struct State {
    table: ResourceTable,
    wasi: WasiCtx,
}

#[cfg(test)]
impl State {
    fn new() -> Self {
        Self {
            table: ResourceTable::new(),
            wasi: WasiCtxBuilder::new().build(),
        }
    }
}

#[cfg(test)]
impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{Context, Result};
    use ed25519_dalek::SigningKey;
    use registry_flow_core::{
        MemoryStore, ObjectStore, Publisher, ROOT_KEY, fixture_release, query_exact_bytes,
    };
    use std::path::PathBuf;
    use wasmtime::{
        Config, Engine, Store,
        component::{Component, Linker},
    };

    #[tokio::test]
    async fn native_and_wasm_queries_are_byte_identical() -> Result<()> {
        let signing_key = SigningKey::from_bytes(&[13; 32]);
        let store = MemoryStore::default();
        let publisher = Publisher::new(store.clone(), signing_key.verifying_key());
        publisher.initialize().await?;
        let (record, blob_path, blob) = fixture_release(
            "litehouse:on-off",
            "1.0.0",
            b"native-wasm-parity-component",
            &[],
            &signing_key,
        )?;
        publisher.publish(record, &blob_path, &blob).await?;
        let root = store.get(ROOT_KEY).await?;
        let decoded = registry_flow_core::decode_root(&root.bytes)?;
        let descriptor = decoded
            .shards
            .iter()
            .find(|descriptor| descriptor.lower_bound == "i")
            .context("litehouse partition descriptor")?;
        let shard = store.get(&descriptor.path).await?;
        let native = query_exact_bytes(
            &root.bytes,
            &shard.bytes,
            "litehouse:on-off",
            "1.0.0",
            &signing_key.verifying_key(),
        )?;
        let native_digest = hex::encode(native.primary.sha256);

        let mut config = Config::new();
        config.wasm_component_model(true);
        let engine = Engine::new(&config)?;
        let mut linker = Linker::<State>::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        let guest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../target/wasm32-wasip2/debug/registry_flow_parity_guest.wasm");
        let component = Component::from_file(&engine, guest_path)?;
        let mut store = Store::new(&engine, State::new());
        let bindings = ParityGuest::instantiate_async(&mut store, &component, &linker).await?;
        let wasm = bindings
            .litehouse_registry_parity_query()
            .call_run(
                &mut store,
                &root.bytes,
                &shard.bytes,
                "litehouse:on-off",
                "1.0.0",
                signing_key.verifying_key().as_bytes(),
            )
            .await?
            .map_err(anyhow::Error::msg)?;
        assert_eq!(native_digest, wasm);
        Ok(())
    }
}
