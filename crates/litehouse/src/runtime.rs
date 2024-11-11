//! Plugin runtime

use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    path::Path,
    sync::Arc,
};

use crate::{
    runtime::bindings::PluginHost,
    store::{StoreRef, StoreStrategy},
};

use bindings::{
    exports::litehouse::plugin::plugin::GuestRunner,
    litehouse::plugin::plugin::{Host, HostRunner},
    wasi::{self, sockets::instance_network},
};

use futures::{StreamExt, TryStreamExt};
use itertools::Itertools;
use jsonc_parser::common::Ranged;
use litehouse_config::{Capability, PluginConfig};
use tokio::sync::broadcast::Sender;
use wasmtime::{
    component::{Component, Linker, Resource, ResourceAny, ResourceTable},
    Config, Engine,
};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};
use wasmtime_wasi_http::{
    bindings::http::{outgoing_handler::ErrorCode, types::Scheme},
    body::HyperOutgoingBody,
    types::{
        default_send_request, HostFutureIncomingResponse, HostOutgoingRequest,
        OutgoingRequestConfig,
    },
    HttpResult, WasiHttpCtx, WasiHttpView,
};

use crate::cache::ModuleCache;

use miette::{miette, Context, Diagnostic, NamedSource, Result, SourceSpan};

pub mod bindings {
    litehouse_plugin::generate_host!();

    impl exports::litehouse::plugin::plugin::UpdateSubscription {
        pub fn matches(&self, event: &host::Update) -> bool {
            matches!(
                (self, event),
                (Self::Temperature, &host::Update::Temperature(_))
            )
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
        let http = WasiHttpCtx::new();

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
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }

    /// Send an outgoing request.
    fn send_request(
        &mut self,
        request: hyper::Request<HyperOutgoingBody>,
        config: OutgoingRequestConfig,
    ) -> HttpResult<HostFutureIncomingResponse> {
        if let Some(authority) = request.uri().authority() {
            let authority = authority.as_str();
            let port_stripped_authority = authority
                .rsplit_once(':')
                .map(|(authority, _)| authority)
                .unwrap_or(&authority);
            if !self.allowed_authorities.contains(port_stripped_authority) {
                tracing::error!(
                    "plugin tried to access {} which is not in the list of allowed authorities",
                    authority
                );
                return Err(ErrorCode::HttpRequestDenied.into());
            }
        }

        Ok(default_send_request(request, config))
    }
}

#[async_trait::async_trait]
impl bindings::host::Host for PluginRunner<Sender<(String, bindings::host::Update)>> {
    async fn send_update(&mut self, nickname: String, event: bindings::host::Update) {
        tracing::trace!(target: "litehouse::plugin", plugin = nickname, "{:?}", event);
        self.event_sink.send((nickname, event)).unwrap();
    }
}

pub struct PluginInstance {
    inner: ResourceAny,
}

impl Deref for PluginInstance {
    type Target = ResourceAny;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl PluginInstance {
    pub async fn new<T: Send>(
        store: &mut StoreRef<T>,
        runner: &GuestRunner<'_>,
        nickname: &str,
        config: &str,
    ) -> Result<(
        Self,
        Vec<crate::runtime::bindings::exports::litehouse::plugin::plugin::Subscription>,
    )> {
        let mut store = store.enter().await;
        let instance = runner
            .call_constructor(&mut store, nickname, Some(config))
            .await
            .map_err(|e| miette!("failed to construct plugin: {:?}", e))?;
        let subs = runner
            .call_subscribe(&mut store, instance)
            .await
            .map_err(|e| miette!("plugin {} failed to subscribe: {:?}", nickname, e))?
            .map_err(|e| miette!("plugin {} failed to subscribe: {}", nickname, e))?;
        Ok((Self { inner: instance }, subs))
    }
}

#[tracing::instrument]
pub async fn set_up_engine(
    cache: bool,
) -> Result<
    (
        Engine,
        Linker<
            PluginRunner<
                Sender<(
                    String,
                    crate::runtime::bindings::litehouse::plugin::plugin::Update,
                )>,
            >,
        >,
        Option<Arc<ModuleCache>>,
    ),
    miette::Error,
> {
    tracing::debug!("setting up engine");
    let mut wasm_config = Config::new();

    wasm_config.wasm_component_model(true).async_support(true);

    let cache = if cache {
        let cache = Arc::new(ModuleCache::load().await?.unwrap_or_default());
        wasm_config
            .enable_incremental_compilation(cache.clone())
            .unwrap();
        Some(cache)
    } else {
        None
    };

    let engine = Engine::new(&wasm_config).map_err(|_| miette!("invalid wasm config"))?;
    let mut linker = Linker::new(&engine);

    wasmtime_wasi::add_to_linker_async(&mut linker)
        .map_err(|e| miette!("unable to add command to linker: {}", e))?;

    wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)
        .map_err(|e| miette!("unable to add http to linker: {}", e))?;

    bindings::host::add_to_linker(&mut linker, |c| c);

    tracing::debug!("set up engine");

    Ok((engine, linker, cache))
}

#[derive(Debug, thiserror::Error, Diagnostic)]
#[error("plugin failed to load")]
#[diagnostic(
    code(litehouse::plugin::load_error),
    url(docsrs),
    help("make sure the file at {file_path} exists and is a valid plugin")
)]
pub struct PluginLoadError {
    #[source_code]
    src: NamedSource<Arc<String>>,
    file_path: String,
    #[label("referenced here")]
    label: SourceSpan,
    #[source]
    source: std::io::Error,
}

/// Set up a plugin host for each of the plugins.
///
/// The plugin host is the memory space in which an instance of a plugin runs.
/// See [StoreStrategy] for more information on how the store is set up.
#[tracing::instrument(skip(ast, engine, store_builder, linker, plugins))]
pub async fn instantiate_plugin_hosts<'a, T: Send + Clone>(
    ast: Option<(NamedSource<Arc<String>>, &jsonc_parser::ast::Value<'a>)>,
    engine: &Engine,
    store_builder: &StoreStrategy<T>,
    linker: &Linker<PluginRunner<T>>,
    plugins: &'a HashMap<String, PluginConfig>,
    base_path: &Path,
    max_parallel_builds: u8,
    max_parallel_instantiations: u8,
) -> Result<
    Vec<(
        &'a String,
        &'a PluginConfig,
        PluginHost,
        StoreRef<T>,
        Component,
    )>,
> {
    tracing::debug!("instantiating plugin hosts {:?}", plugins);

    let map = plugins
        .iter()
        .into_grouping_map_by(|(_, instance)| instance.plugin.file_name())
        .collect::<Vec<_>>();

    let store_builder = Arc::new(store_builder);
    let ast = Arc::new(ast);

    let hosts = tokio_stream::iter(map.into_iter())
        .map(|(file_name, plugins)| {
            let engine = engine.clone();
            let ast = ast.clone();
            async move {
                tracing::debug!("building");
                let file_path = base_path.join(&file_name);
                tracing::info!("loading plugin from {}", file_path.display());
                let contents = tokio::fs::read(&file_path).await.map_err(|e| {
                    if let Some((src, ast)) = ast.as_ref() {
                        let node = ast
                            .as_object()
                            .unwrap()
                            .get("plugins")
                            .unwrap()
                            .value
                            .as_object()
                            .unwrap()
                            .get(plugins.first().unwrap().0)
                            .unwrap()
                            .value
                            .as_object()
                            .unwrap()
                            .get("plugin")
                            .unwrap()
                            .value
                            .as_string_lit()
                            .unwrap()
                            .range;
                        PluginLoadError {
                            file_path: file_path.to_string_lossy().to_string(),
                            label: SourceSpan::new(node.start.into(), node.width()),
                            src: src.clone(),
                            source: e,
                        }
                    } else {
                        println!("{:?}", ast.as_ref());
                        panic!();
                    }
                })?;
                let c = tokio::task::spawn_blocking(move || Component::new(&engine, contents)) // TODO(MEM): allocates 7MB of RAM here
                    .await
                    .expect("no panic")
                    .map_err(|e| miette!("unable to build {}: {}", &file_name, e))?;

                Ok::<_, miette::ErrReport>(tokio_stream::iter(plugins.into_iter().map(
                    move |(string, instance)| {
                        Ok::<_, miette::ErrReport>((string, instance, c.clone()))
                    },
                )))
            }
        })
        .buffer_unordered(max_parallel_builds.into()) // max parallel builds
        .try_flatten_unordered(None)
        .map(|res| {
            let store_builder = store_builder.clone();
            async move {
                let (nick, instance, component) = res?;
                let mut store = store_builder.get(&instance.plugin.file_name());
                let host = instantiate_plugin_host(&mut store, linker, &component)
                    .await
                    .wrap_err_with(|| {
                        format!(
                            "when instantiating {} from {}",
                            nick,
                            instance.plugin.file_name()
                        )
                    })?;
                Ok((nick, instance, host, store, component))
            }
        })
        .buffer_unordered(max_parallel_instantiations.into()) // max parallel instantiations
        .try_collect::<Vec<_>>()
        .await;

    tracing::debug!("instantiated plugin hosts");

    hosts
}

#[tracing::instrument(skip(store, linker, component))]
pub async fn instantiate_plugin_host<T: Send + Clone>(
    store: &mut StoreRef<T>,
    linker: &Linker<PluginRunner<T>>,
    component: &Component,
) -> Result<PluginHost> {
    tracing::debug!("instantiating");
    let store_lock = store.enter().await;
    let host = PluginHost::instantiate_async(store_lock, component, linker)
        .await
        .map_err(|e| miette!("unable to instantiate: {}", e))?;

    Ok(host)
}
