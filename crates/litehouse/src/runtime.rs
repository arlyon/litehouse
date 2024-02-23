use self::bindings::PluginHostImports;

use tokio::sync::broadcast::Sender;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::preview2::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView};

pub mod bindings {
    plugin::generate_host!();

    impl crate::runtime::bindings::exports::litehouse::plugin::plugin::UpdateSubscription {
        pub fn matches(&self, event: &Update) -> bool {
            match (self, event) {
                (Self::Temperature, Update::Temperature(_)) => true,
                _ => false,
            }
        }
    }
}

pub struct PluginRunner<T> {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
    event_sink: T,
}

impl<T> PluginRunner<T> {
    pub fn new(event_sink: T) -> Self {
        let mut wasi = WasiCtxBuilder::new();
        wasi.inherit_stdio()
            .allow_tcp(true)
            .allow_ip_name_lookup(true)
            .allow_udp(true)
            .inherit_network()
            .env("RUST_LOG", std::env::var("RUST_LOG").unwrap_or_default());
        let http = WasiHttpCtx;
        Self {
            table: ResourceTable::new(),
            wasi: wasi.build(),
            http,
            event_sink,
        }
    }
}

impl<T: Send> WasiView for PluginRunner<T> {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

impl<T: Send> WasiHttpView for PluginRunner<T> {
    fn ctx(&mut self) -> &mut wasmtime_wasi_http::WasiHttpCtx {
        &mut self.http
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

#[async_trait::async_trait]
impl PluginHostImports for PluginRunner<Sender<(String, bindings::Update)>> {
    async fn send_update(
        &mut self,
        nickname: String,
        event: bindings::Update,
    ) -> wasmtime::Result<()> {
        tracing::trace!("received update {:?}", event);
        self.event_sink.send((nickname, event)).unwrap();
        return Ok(());
    }
}

// impl plugin::Host for PluginRunner {}
