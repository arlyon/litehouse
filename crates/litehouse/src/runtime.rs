//! Plugin runtime

use std::{
    collections::{HashMap, HashSet},
    future::Future,
    net::SocketAddr,
    ops::Deref,
    path::Path,
    sync::Arc,
};

use crate::store::{StoreRef, StoreStrategy};

use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
use itertools::Itertools;
use jsonc_parser::common::Ranged;
use litehouse_config::{Capability, PluginConfig};
use litehouse_contracts::component::{ComponentKind, validate_component};
use tokio::sync::broadcast::Sender;
use wasmtime::{
    AsContextMut, Config, Engine,
    component::{Component, Linker, ResourceAny, ResourceTable},
};
use wasmtime_wasi::{
    TrappableError, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView, sockets::SocketAddrUse,
};
use wasmtime_wasi_http::{
    WasiHttpCtx,
    p3::{
        RequestOptions, WasiHttpCtxView, WasiHttpHooks, WasiHttpView,
        bindings::http::types::ErrorCode, default_send_request,
    },
};

use crate::cache::ModuleCache;

use miette::{Context, Diagnostic, NamedSource, Result, SourceSpan, miette};

pub mod bindings {
    litehouse_plugin::generate_host!();

    impl exports::litehouse::plugin::plugin::UpdateSubscription {
        pub fn matches(&self, event: &litehouse::plugin::host::Update) -> bool {
            matches!(
                (self, event),
                (
                    Self::Temperature,
                    &litehouse::plugin::host::Update::Temperature(_)
                )
            )
        }
    }
}

pub struct PluginRunner<T> {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
    http_hooks: HttpHooks,
    event_sink: T,
}

struct HttpHooks {
    allowed_authorities: HashSet<String>,
}

type HttpBody = UnsyncBoxBody<Bytes, ErrorCode>;
type HttpIo = Box<dyn Future<Output = Result<(), ErrorCode>> + Send>;
type HttpResponse = (hyper::Response<HttpBody>, HttpIo);
type HttpFuture = Box<dyn Future<Output = Result<HttpResponse, TrappableError<ErrorCode>>> + Send>;

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

fn socket_address_allowed(
    allowed_tcp_addresses: &HashSet<SocketAddr>,
    address: SocketAddr,
    usage: SocketAddrUse,
) -> bool {
    matches!(usage, SocketAddrUse::TcpConnect) && allowed_tcp_addresses.contains(&address)
}

impl<T> PluginRunner<T> {
    pub fn new(event_sink: T, capabilities: Vec<Capability>) -> Self {
        let mut wasi = WasiCtxBuilder::new();
        wasi.inherit_stdio()
            .env("RUST_LOG", std::env::var("RUST_LOG").unwrap_or_default());
        let allowed_tcp_addresses: HashSet<_> = capabilities
            .iter()
            .filter_map(|capability| match capability {
                Capability::TcpClient(address) => Some(*address),
                _ => None,
            })
            .collect();
        wasi.socket_addr_check(move |address, usage| {
            let allowed = socket_address_allowed(&allowed_tcp_addresses, address, usage);
            Box::pin(async move { allowed })
        });

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
            http_hooks: HttpHooks {
                allowed_authorities,
            },
            event_sink,
        }
    }
}

impl<T: Send> WasiView for PluginRunner<T> {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl<T: Send> WasiHttpView for PluginRunner<T> {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: &mut self.http_hooks,
        }
    }
}

impl WasiHttpHooks for HttpHooks {
    fn send_request(
        &mut self,
        request: hyper::Request<HttpBody>,
        options: Option<RequestOptions>,
        response_io: HttpIo,
    ) -> HttpFuture {
        if let Some(authority) = request.uri().authority() {
            let authority = authority.as_str();
            let port_stripped_authority = authority
                .rsplit_once(':')
                .map(|(authority, _)| authority)
                .unwrap_or(authority);
            if !self.allowed_authorities.contains(port_stripped_authority) {
                tracing::error!(
                    "plugin tried to access {} which is not in the list of allowed authorities",
                    authority
                );
                return Box::new(async { Err(ErrorCode::HttpRequestDenied.into()) });
            }
        }

        // Match Wasmtime's default hook: this future reports how the guest
        // consumes the response and is optional for this stateless client.
        drop(response_io);
        Box::new(async move {
            let (response, response_io) = default_send_request(request, options).await?;
            Ok((
                response.map(BodyExt::boxed_unsync),
                Box::new(response_io) as HttpIo,
            ))
        })
    }
}

impl bindings::litehouse::plugin::host::Host
    for PluginRunner<Sender<(String, bindings::litehouse::plugin::host::Update)>>
{
    fn send_update(&mut self, nickname: String, event: bindings::litehouse::plugin::host::Update) {
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
        host: &wasmtime::component::Instance,
        nickname: &str,
        config: &str,
    ) -> Result<(
        Self,
        Vec<crate::runtime::bindings::exports::litehouse::plugin::plugin::Subscription>,
    )> {
        let mut store = store.enter().await;

        let indices =
            crate::runtime::bindings::exports::litehouse::plugin::plugin::GuestIndices::new(
                &host.instance_pre(&mut store),
            )
            .unwrap();

        let guest = indices.load(&mut store, host).unwrap();
        let runner = guest.runner();

        let instance = runner
            .call_constructor(&mut store, nickname, Some(config))
            .await
            .map_err(|e| miette!("failed to construct plugin: {:?}", e))?;
        let res = runner
            .call_subscribe(&mut store, instance)
            .await
            .map_err(|e| miette!("plugin {} failed to subscribe: {:?}", nickname, e))?
            .map_err(|e| miette!("plugin {} failed to subscribe: {}", nickname, e))?;
        let outputs = store
            .as_context_mut()
            .run_concurrent(async |accessor| runner.call_outputs(accessor, instance).await)
            .await
            .and_then(|result| result)
            .map_err(|e| miette!("plugin {} failed to report outputs: {:?}", nickname, e))?
            .map_err(|e| miette!("plugin {} rejected output discovery: {}", nickname, e))?;

        tracing::info!("got outputs: {:?}", outputs);

        Ok((Self { inner: instance }, res))
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
                    crate::runtime::bindings::litehouse::plugin::update::Update,
                )>,
            >,
        >,
        Option<Arc<ModuleCache>>,
    ),
    miette::Error,
> {
    tracing::debug!("setting up engine");
    let mut wasm_config = Config::new();

    wasm_config
        .wasm_component_model(true)
        .wasm_component_model_async(true)
        .concurrency_support(true)
        .cranelift_opt_level(wasmtime::OptLevel::Speed);

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

    // Rust's standard library still targets WASI 0.2 while the wasm32-wasip3
    // target is incomplete. Keep Preview 2 linked for the standard-library
    // adapter until plugins can be built directly for wasm32-wasip3.
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
        .map_err(|e| miette!("unable to add WASI 0.2 compatibility interfaces: {}", e))?;

    wasmtime_wasi::p3::add_to_linker(&mut linker)
        .map_err(|e| miette!("unable to add WASI 0.3 interfaces to linker: {}", e))?;

    wasmtime_wasi_http::p3::add_to_linker(&mut linker)
        .map_err(|e| miette!("unable to add WASI HTTP 0.3 to linker: {}", e))?;

    bindings::litehouse::plugin::host::add_to_linker::<_, wasmtime::component::HasSelf<_>>(
        &mut linker,
        |context| context,
    )
    .map_err(|error| miette!("unable to add Litehouse plugin host interface: {error}"))?;

    // bindings::litehouse::plugin::notify::add_to_linker(&mut linker, |c| c)
    //     .map_err(|e| miette!("unable to add http to linker: {}", e))?;

    // bindings::litehouse::plugin::notify::add_to_linker(&mut linker, |c| c);

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
        wasmtime::component::Instance,
        StoreRef<T>,
        Component,
    )>,
> {
    tracing::debug!(plugin_count = plugins.len(), "instantiating plugin hosts");

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
                validate_component(&contents, ComponentKind::Plugin).map_err(|error| {
                    miette!("plugin {} violates runtime profile: {error}", file_name)
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
                let host = instantiate_instance(&mut store, linker, &component)
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
pub async fn instantiate_instance<T: Send + Clone>(
    store: &mut StoreRef<T>,
    linker: &Linker<PluginRunner<T>>,
    component: &Component,
) -> Result<wasmtime::component::Instance> {
    tracing::debug!("instantiating");
    let store_lock = store.enter().await;

    let instance = linker
        .instantiate_async(store_lock, component)
        .await
        .unwrap();

    Ok(instance)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, net::SocketAddr};

    use wasmtime_wasi::sockets::SocketAddrUse;

    use super::socket_address_allowed;

    #[test]
    fn tcp_capability_allows_only_the_declared_connect_destination() {
        let television = SocketAddr::from(([192, 168, 1, 50], 8002));
        let other_port = SocketAddr::from(([192, 168, 1, 50], 8001));
        let allowed = HashSet::from([television]);

        assert!(socket_address_allowed(
            &allowed,
            television,
            SocketAddrUse::TcpConnect
        ));
        assert!(!socket_address_allowed(
            &allowed,
            other_port,
            SocketAddrUse::TcpConnect
        ));
        assert!(!socket_address_allowed(
            &allowed,
            television,
            SocketAddrUse::TcpBind
        ));
    }
}
