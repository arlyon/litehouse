use std::collections::HashSet;

use self::bindings::PluginHostImports;

use litehouse_config::Capability;
use tokio::sync::broadcast::Sender;
use wasmtime::component::ResourceTable;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{
    bindings::http::outgoing_handler::ErrorCode, types::default_send_request, WasiHttpCtx,
    WasiHttpView,
};

pub mod bindings {
    plugin::generate_host!();

    impl crate::runtime::bindings::exports::litehouse::plugin::plugin::UpdateSubscription {
        pub fn matches(&self, event: &Update) -> bool {
            matches!((self, event), (Self::Temperature, Update::Temperature(_)))
        }
    }
}

pub struct PluginRunner<T> {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
    event_sink: T,
    allowed_authorities: HashSet<String>,
}

pub struct PluginRunnerFactory<T> {
    event_sink: T,
    capabilities: Vec<Capability>,
}

impl<T: Clone> PluginRunnerFactory<T> {
    pub fn new(event_sink: T, capabilities: Vec<Capability>) -> Self {
        Self {
            event_sink,
            capabilities,
        }
    }

    pub fn create(&self) -> PluginRunner<T> {
        PluginRunner::new(self.event_sink.clone(), self.capabilities.clone())
    }
}

impl<T> PluginRunner<T> {
    pub fn new(event_sink: T, capabilities: Vec<Capability>) -> Self {
        let mut wasi = WasiCtxBuilder::new();
        wasi.inherit_stdio()
            .env("RUST_LOG", std::env::var("RUST_LOG").unwrap_or_default());
        let http = WasiHttpCtx;

        let allowed_authorities = capabilities
            .iter()
            .filter_map(|c| match c {
                Capability::HttpClient(authority) => Some(authority.to_owned()),
                _ => None,
            })
            .collect();

        Self {
            table: ResourceTable::new(),
            wasi: wasi.build(),
            http,
            event_sink,
            allowed_authorities,
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

    fn send_request(
        &mut self,
        request: wasmtime_wasi_http::types::OutgoingRequest,
    ) -> wasmtime::Result<
        wasmtime::component::Resource<wasmtime_wasi_http::types::HostFutureIncomingResponse>,
    >
    where
        Self: Sized,
    {
        // try to remove the port if it exists
        let authority = request
            .authority
            .rsplit_once(':')
            .map(|(authority, _)| authority)
            .unwrap_or(&request.authority);

        if !self.allowed_authorities.contains(authority) {
            tracing::error!(
                "plugin tried to access {} which is not in the list of allowed authorities",
                authority
            );
            return Err(ErrorCode::HttpRequestDenied.into());
        }

        default_send_request(self, request)
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
