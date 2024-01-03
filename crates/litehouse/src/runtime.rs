use self::bindings::PluginHostImports;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::preview2::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

pub mod bindings {
    plugin::generate_host!();
}

pub struct PluginRunner {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
}

impl PluginRunner {
    pub fn new() -> Self {
        let mut wasi = WasiCtxBuilder::new();
        wasi.inherit_stdio()
            .socket_addr_check(|x1, x2| {
                println!("{} {:?}", x1, x2);
                true
            })
            .allow_tcp(true)
            .allow_udp(true)
            .allow_ip_name_lookup(true);
        let http = WasiHttpCtx;
        Self {
            table: ResourceTable::new(),
            wasi: wasi.build(),
            http,
        }
    }
}

impl WasiView for PluginRunner {
    fn table(&self) -> &ResourceTable {
        &self.table
    }

    fn table_mut(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.wasi
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl WasiHttpView for PluginRunner {
    fn ctx(&mut self) -> &mut wasmtime_wasi_http::WasiHttpCtx {
        &mut self.http
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

#[async_trait::async_trait]
impl PluginHostImports for PluginRunner {
    async fn update(&mut self, event: bindings::Event) -> wasmtime::Result<()> {
        tracing::info!("update: {:?}", event);
        return Ok(());
    }
}

// impl plugin::Host for PluginRunner {}
