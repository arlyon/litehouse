//! Plugin runtime

use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    future::Future,
    net::SocketAddr,
    ops::Deref,
    path::Path,
    sync::Arc,
    time::Duration,
};

use crate::store::{StoreLock, StoreRef, StoreStrategy};

use base64::Engine as _;
use bytes::Bytes;
use futures::{StreamExt, TryStreamExt};
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
use itertools::Itertools;
use litehouse_config::{Capability, PluginConfig};
use litehouse_contracts::{
    component::{ComponentKind, validate_component},
    config::{ConfigFieldAnnotation, SecretEncoding, environment_variable_name},
    metadata::RuntimeMetadataV1,
};
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

use miette::{Context, IntoDiagnostic, NamedSource, Result, miette};

const EPOCH_TICK: Duration = Duration::from_millis(10);
pub const DEFAULT_GUEST_CALL_TIMEOUT: Duration = Duration::from_secs(30);
const METADATA_CALL_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PROTECTED_VALUE_BYTES: usize = 64 * 1024;

pub mod bindings {
    litehouse_plugin::generate_host!();

    impl exports::litehouse::plugin::plugin::UpdateSubscription {
        pub fn matches(&self, event: &litehouse::plugin::host::Update) -> bool {
            matches!(
                (self, event),
                (
                    Self::Temperature,
                    litehouse::plugin::host::Update::Temperature(_)
                ) | (
                    Self::WindSpeed,
                    litehouse::plugin::host::Update::WindSpeed(_)
                ) | (Self::Current, litehouse::plugin::host::Update::Current(_))
                    | (Self::Voltage, litehouse::plugin::host::Update::Voltage(_))
                    | (Self::Power, litehouse::plugin::host::Update::Power(_))
                    | (Self::OnOff, litehouse::plugin::host::Update::OnOff(_))
            )
        }
    }
}

pub type RuntimeUpdate = (
    String,
    bindings::litehouse::plugin::host::Update,
    tokio::time::Instant,
);

pub struct PluginRunner<T> {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
    http_hooks: HttpHooks,
    event_sink: T,
}

struct HttpHooks {
    allowed_origins: HashSet<AllowedHttpOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AllowedHttpOrigin {
    Exact {
        scheme: String,
        host: String,
        port: u16,
    },
    Legacy {
        host: String,
        port: Option<u16>,
    },
}

impl AllowedHttpOrigin {
    fn parse(value: &str) -> Self {
        if let Ok(url) = url::Url::parse(value)
            && let (Some(host), Some(port)) = (url.host_str(), url.port_or_known_default())
        {
            return Self::Exact {
                scheme: url.scheme().to_ascii_lowercase(),
                host: host.to_ascii_lowercase(),
                port,
            };
        }

        if let Ok(authority) = value.parse::<hyper::http::uri::Authority>() {
            return Self::Legacy {
                host: authority.host().to_ascii_lowercase(),
                port: authority.port_u16(),
            };
        }
        Self::Legacy {
            host: value.to_ascii_lowercase(),
            port: None,
        }
    }

    fn permits(&self, scheme: &str, authority: &hyper::http::uri::Authority) -> bool {
        let request_host = authority.host().to_ascii_lowercase();
        let request_port = authority.port_u16().or_else(|| default_http_port(scheme));
        match self {
            Self::Exact {
                scheme: allowed_scheme,
                host,
                port,
            } => {
                allowed_scheme.eq_ignore_ascii_case(scheme)
                    && host == &request_host
                    && Some(*port) == request_port
            }
            Self::Legacy { host, port } => {
                host == &request_host && port.is_none_or(|port| Some(port) == request_port)
            }
        }
    }
}

fn default_http_port(scheme: &str) -> Option<u16> {
    match scheme.to_ascii_lowercase().as_str() {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    }
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

    pub fn create_with(&self, instance_capabilities: &[Capability]) -> PluginRunner<T> {
        let mut capabilities = self.capabilities.clone();
        for capability in instance_capabilities {
            if !capabilities.contains(capability) {
                capabilities.push(capability.clone());
            }
        }
        PluginRunner::new(self.event_sink.clone(), capabilities)
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

        let allowed_origins = capabilities
            .iter()
            .filter_map(|capability| match capability {
                Capability::HttpClient(origin) => Some(AllowedHttpOrigin::parse(origin)),
                _ => None,
            })
            .collect();

        Self {
            table: ResourceTable::new(),
            wasi: wasi.build(),
            http,
            http_hooks: HttpHooks { allowed_origins },
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
        let permitted = request
            .uri()
            .scheme_str()
            .zip(request.uri().authority())
            .is_some_and(|(scheme, authority)| {
                self.allowed_origins
                    .iter()
                    .any(|origin| origin.permits(scheme, authority))
            });
        if !permitted {
            tracing::error!(
                authority = request.uri().authority().map(|value| value.as_str()),
                "plugin HTTP request denied by configured origin grants"
            );
            return Box::new(async { Err(ErrorCode::HttpRequestDenied.into()) });
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

impl bindings::litehouse::plugin::host::Host for PluginRunner<Sender<RuntimeUpdate>> {
    fn send_update(&mut self, nickname: String, event: bindings::litehouse::plugin::host::Update) {
        tracing::trace!(target: "litehouse::plugin", plugin = nickname, "{:?}", event);
        if self
            .event_sink
            .send((nickname, event, tokio::time::Instant::now()))
            .is_err()
        {
            tracing::debug!("dropping plugin update because no runtime receiver is active");
        }
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
        instance_id: Option<&str>,
        config: Option<&serde_json::Value>,
    ) -> Result<(
        Self,
        Vec<crate::runtime::bindings::exports::litehouse::plugin::plugin::Subscription>,
        RuntimeMetadataV1,
    )> {
        let metadata_deadline = tokio::time::Instant::now() + METADATA_CALL_TIMEOUT;
        let mut store = enter_store_before(
            store,
            metadata_deadline,
            &format!("plugin {nickname} metadata call"),
        )
        .await?;

        let indices =
            crate::runtime::bindings::exports::litehouse::plugin::plugin::GuestIndices::new(
                &host.instance_pre(&mut store),
            )
            .map_err(|error| miette!("failed to load plugin export indices: {error:?}"))?;

        let guest = indices
            .load(&mut store, host)
            .map_err(|error| miette!("failed to load plugin exports: {error:?}"))?;
        let runner = guest.runner();

        store.set_epoch_deadline(epoch_ticks(remaining(
            metadata_deadline,
            "plugin metadata call",
        )?));
        let metadata =
            tokio::time::timeout_at(metadata_deadline, guest.call_get_metadata(&mut store))
                .await
                .map_err(|_| miette!("plugin {nickname} metadata call timed out"))?
                .map_err(|error| miette!("plugin {nickname} metadata call failed: {error:?}"))?;
        let metadata = RuntimeMetadataV1::decode_section(&metadata)
            .into_diagnostic()
            .wrap_err_with(|| format!("plugin {nickname} returned invalid runtime metadata"))?;
        let config = prepare_plugin_config(instance_id, config, &metadata)?;

        let constructor_deadline = tokio::time::Instant::now() + DEFAULT_GUEST_CALL_TIMEOUT;
        store.set_epoch_deadline(epoch_ticks(remaining(
            constructor_deadline,
            "plugin constructor",
        )?));
        let instance = tokio::time::timeout_at(
            constructor_deadline,
            runner.call_constructor(&mut store, nickname, Some(&config)),
        )
        .await
        .map_err(|_| miette!("plugin {nickname} constructor timed out"))?
        .map_err(|error| miette!("failed to construct plugin {nickname}: {error:?}"))?;

        let subscription_deadline = tokio::time::Instant::now() + DEFAULT_GUEST_CALL_TIMEOUT;
        store.set_epoch_deadline(epoch_ticks(remaining(
            subscription_deadline,
            "plugin subscription discovery",
        )?));
        let subscriptions = tokio::time::timeout_at(
            subscription_deadline,
            runner.call_subscribe(&mut store, instance),
        )
        .await
        .map_err(|_| miette!("plugin {nickname} subscription discovery timed out"))?
        .map_err(|error| miette!("plugin {nickname} failed to subscribe: {error:?}"))?
        .map_err(|error| miette!("plugin {nickname} rejected subscriptions: {error}"))?;

        let output_deadline = tokio::time::Instant::now() + DEFAULT_GUEST_CALL_TIMEOUT;
        store.set_epoch_deadline(epoch_ticks(remaining(
            output_deadline,
            "plugin output discovery",
        )?));
        let outputs = tokio::time::timeout_at(
            output_deadline,
            store
                .as_context_mut()
                .run_concurrent(async |accessor| runner.call_outputs(accessor, instance).await),
        )
        .await
        .map_err(|_| miette!("plugin {nickname} output discovery timed out"))?
        .and_then(|result| result)
        .map_err(|error| miette!("plugin {nickname} failed to report outputs: {error:?}"))?
        .map_err(|error| miette!("plugin {nickname} rejected output discovery: {error}"))?;

        tracing::info!(plugin = nickname, ?outputs, "discovered plugin outputs");

        Ok((Self { inner: instance }, subscriptions, metadata))
    }
}

pub(crate) fn epoch_ticks(duration: Duration) -> u64 {
    let tick_millis = EPOCH_TICK.as_millis().max(1);
    let ticks = duration.as_millis().div_ceil(tick_millis);
    u64::try_from(ticks).unwrap_or(u64::MAX).max(1)
}

fn remaining(deadline: tokio::time::Instant, operation: &str) -> Result<Duration> {
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
        Err(miette!("{operation} timed out"))
    } else {
        Ok(remaining)
    }
}

async fn enter_store_before<'a, T>(
    store: &'a mut StoreRef<T>,
    deadline: tokio::time::Instant,
    operation: &str,
) -> Result<StoreLock<'a, T>> {
    tokio::time::timeout_at(deadline, store.enter())
        .await
        .map_err(|_| miette!("{operation} timed out while waiting for its store"))
}

fn prepare_plugin_config(
    instance_id: Option<&str>,
    config: Option<&serde_json::Value>,
    metadata: &RuntimeMetadataV1,
) -> Result<String> {
    prepare_plugin_config_with(instance_id, config, metadata, |name| std::env::var_os(name))
}

fn prepare_plugin_config_with(
    instance_id: Option<&str>,
    config: Option<&serde_json::Value>,
    metadata: &RuntimeMetadataV1,
    read_environment: impl Fn(&str) -> Option<OsString>,
) -> Result<String> {
    let mut prepared = config.cloned();
    for field in &metadata.config_annotations {
        let ConfigFieldAnnotation::EnvironmentSecret {
            field_token,
            encoding,
        } = &field.annotation
        else {
            continue;
        };
        let instance_id = instance_id.ok_or_else(|| {
            miette!(
                "plugin instance requires an instance_id UUID for protected field {}",
                field.json_pointer
            )
        })?;
        let variable = environment_variable_name(instance_id, field_token)
            .into_diagnostic()
            .wrap_err("unable to derive protected environment-variable name")?;
        let value = read_environment(&variable)
            .ok_or_else(|| miette!("required protected variable {variable} is not configured"))?
            .into_string()
            .map_err(|_| miette!("protected variable {variable} is not valid UTF-8"))?;
        if value.is_empty() || value.len() > MAX_PROTECTED_VALUE_BYTES {
            return Err(miette!(
                "protected variable {variable} must contain 1..={MAX_PROTECTED_VALUE_BYTES} UTF-8 bytes"
            ));
        }
        let value = match encoding {
            SecretEncoding::Utf8 => serde_json::Value::String(value),
            SecretEncoding::Base64 => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(value)
                    .map_err(|_| miette!("protected variable {variable} is not valid base64"))?;
                serde_json::to_value(bytes).into_diagnostic()?
            }
        };

        let config = prepared.as_mut().ok_or_else(|| {
            miette!(
                "plugin instance has protected field {} but no config object",
                field.json_pointer
            )
        })?;
        if config
            .pointer(&field.json_pointer)
            .is_some_and(|current| !current.is_null())
        {
            return Err(miette!(
                "protected field {} must not be persisted in plugin config",
                field.json_pointer
            ));
        }
        set_json_pointer(config, &field.json_pointer, value)?;
    }

    serde_json::to_string(&prepared).into_diagnostic()
}

fn set_json_pointer(
    document: &mut serde_json::Value,
    pointer: &str,
    value: serde_json::Value,
) -> Result<()> {
    if let Some(target) = document.pointer_mut(pointer) {
        *target = value;
        return Ok(());
    }

    let mut segments = pointer
        .strip_prefix('/')
        .filter(|pointer| !pointer.is_empty())
        .ok_or_else(|| miette!("protected config pointer must identify one field"))?
        .split('/')
        .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
        .collect::<Vec<_>>();
    let field = segments
        .pop()
        .ok_or_else(|| miette!("protected config pointer must identify one field"))?;
    let mut parent = document;
    for segment in segments {
        parent = parent
            .as_object_mut()
            .and_then(|object| object.get_mut(&segment))
            .ok_or_else(|| miette!("protected config pointer parent is missing"))?;
    }
    parent
        .as_object_mut()
        .ok_or_else(|| miette!("protected config pointer parent is not an object"))?
        .insert(field, value);
    Ok(())
}

#[tracing::instrument]
pub async fn set_up_engine(
    cache: bool,
) -> Result<
    (
        Engine,
        Linker<PluginRunner<Sender<RuntimeUpdate>>>,
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
        .epoch_interruption(true)
        .cranelift_opt_level(wasmtime::OptLevel::Speed);

    let cache = if cache {
        let cache = Arc::new(ModuleCache::load().await?.unwrap_or_default());
        wasm_config
            .enable_incremental_compilation(cache.clone())
            .map_err(|error| miette!("unable to enable incremental cache: {error}"))?;
        Some(cache)
    } else {
        None
    };

    let engine = Engine::new(&wasm_config).map_err(|_| miette!("invalid wasm config"))?;
    let epoch_engine = engine.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(EPOCH_TICK);
        loop {
            interval.tick().await;
            epoch_engine.increment_epoch();
        }
    });
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
    let _ast = ast;

    let hosts = tokio_stream::iter(map.into_iter())
        .map(|(file_name, plugins)| {
            let engine = engine.clone();
            async move {
                tracing::debug!("building");
                let file_path = base_path.join(&file_name);
                tracing::info!("loading plugin from {}", file_path.display());
                let contents = tokio::fs::read(&file_path)
                    .await
                    .into_diagnostic()
                    .wrap_err_with(|| format!("unable to read plugin {}", file_path.display()))?;
                validate_component(&contents, ComponentKind::Plugin).map_err(|error| {
                    miette!("plugin {} violates runtime profile: {error}", file_name)
                })?;
                let c = tokio::task::spawn_blocking(move || Component::new(&engine, contents))
                    .await
                    .map_err(|error| miette!("plugin compiler task failed: {error}"))?
                    .map_err(|error| miette!("unable to build {}: {error}", &file_name))?;

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
                let mut store =
                    store_builder.get(&instance.plugin.file_name(), &instance.capabilities)?;
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
    let deadline = tokio::time::Instant::now() + DEFAULT_GUEST_CALL_TIMEOUT;
    let mut store_lock = enter_store_before(store, deadline, "plugin instantiation").await?;
    store_lock.set_epoch_deadline(epoch_ticks(remaining(deadline, "plugin instantiation")?));

    let instance =
        tokio::time::timeout_at(deadline, linker.instantiate_async(store_lock, component))
            .await
            .map_err(|_| miette!("plugin instantiation timed out"))?
            .map_err(|error| miette!("unable to instantiate plugin component: {error}"))?;

    Ok(instance)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, ffi::OsString, net::SocketAddr, time::Duration};

    use litehouse_contracts::{
        config::{AnnotatedConfigField, ConfigFieldAnnotation, SecretEncoding},
        identity::{PackageIdentity, PackageRequirement, Version},
        metadata::{HostFacilityRequests, RuntimeMetadataV1},
    };
    use wasmtime_wasi::sockets::SocketAddrUse;

    use super::{
        AllowedHttpOrigin, PluginRunnerFactory, enter_store_before, prepare_plugin_config_with,
        set_up_engine, socket_address_allowed,
    };
    use crate::store::StoreStrategy;

    fn protected_metadata() -> RuntimeMetadataV1 {
        RuntimeMetadataV1 {
            package: PackageIdentity::new("litehouse", "fixture", Version::V1).unwrap(),
            runtime_profile: PackageRequirement {
                namespace: "litehouse".into(),
                name: "runtime-profile".into(),
                minimum: Version::V1,
            },
            description: None,
            provides: Vec::new(),
            requires: Vec::new(),
            host_facilities: HostFacilityRequests::default(),
            config_schema: None,
            config_annotations: vec![AnnotatedConfigField {
                json_pointer: "/token".into(),
                annotation: ConfigFieldAnnotation::EnvironmentSecret {
                    field_token: "API_TOKEN".into(),
                    encoding: SecretEncoding::Utf8,
                },
            }],
        }
    }

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

    #[test]
    fn http_capability_canonicalizes_scheme_host_and_effective_port() {
        let origin = AllowedHttpOrigin::parse("https://Example.COM");
        let default_port = "example.com:443".parse().unwrap();
        let implicit_port = "EXAMPLE.com".parse().unwrap();
        let wrong_port = "example.com:444".parse().unwrap();

        assert!(origin.permits("https", &default_port));
        assert!(origin.permits("https", &implicit_port));
        assert!(!origin.permits("http", &default_port));
        assert!(!origin.permits("https", &wrong_port));
    }

    #[test]
    fn legacy_authority_grants_remain_compatible_and_explicit_ports_stay_exact() {
        let host = AllowedHttpOrigin::parse("api.example.com");
        let exact = AllowedHttpOrigin::parse("192.168.1.50:8002");
        let ipv6 = AllowedHttpOrigin::parse("http://[::1]:8002");

        assert!(host.permits("https", &"API.example.com:8443".parse().unwrap()));
        assert!(exact.permits("http", &"192.168.1.50:8002".parse().unwrap()));
        assert!(!exact.permits("http", &"192.168.1.50:8001".parse().unwrap()));
        assert!(ipv6.permits("http", &"[::1]:8002".parse().unwrap()));
    }

    #[test]
    fn instance_capabilities_extend_without_leaking_into_global_defaults() {
        let factory = PluginRunnerFactory::new(
            (),
            vec![litehouse_config::Capability::HttpClient(
                "https://global.example".into(),
            )],
        );

        let global = factory.create();
        let instance = factory.create_with(&[litehouse_config::Capability::HttpClient(
            "http://192.168.1.50:8002".into(),
        )]);

        assert_eq!(global.http_hooks.allowed_origins.len(), 1);
        assert_eq!(instance.http_hooks.allowed_origins.len(), 2);
        assert!(
            instance
                .http_hooks
                .allowed_origins
                .iter()
                .any(|origin| { origin.permits("http", &"192.168.1.50:8002".parse().unwrap()) })
        );
    }

    #[test]
    fn protected_value_is_injected_without_mutating_persisted_config() {
        let metadata = RuntimeMetadataV1 {
            package: PackageIdentity::new("litehouse", "fixture", Version::V1).unwrap(),
            runtime_profile: PackageRequirement {
                namespace: "litehouse".into(),
                name: "runtime-profile".into(),
                minimum: Version::V1,
            },
            description: None,
            provides: Vec::new(),
            requires: Vec::new(),
            host_facilities: HostFacilityRequests::default(),
            config_schema: None,
            config_annotations: vec![AnnotatedConfigField {
                json_pointer: "/token".into(),
                annotation: ConfigFieldAnnotation::EnvironmentSecret {
                    field_token: "API_TOKEN".into(),
                    encoding: SecretEncoding::Utf8,
                },
            }],
        };
        let persisted = serde_json::json!({"endpoint": "local"});

        let encoded = prepare_plugin_config_with(
            Some("00112233-4455-6677-8899-aabbccddeeff"),
            Some(&persisted),
            &metadata,
            |name| {
                assert_eq!(
                    name,
                    "LITEHOUSE_INSTANCE_00112233_4455_6677_8899_AABBCCDDEEFF_API_TOKEN"
                );
                Some(OsString::from("protected"))
            },
        )
        .unwrap();
        let prepared: serde_json::Value = serde_json::from_str(&encoded).unwrap();

        assert_eq!(prepared["token"], "protected");
        assert!(persisted.get("token").is_none());
    }

    #[test]
    fn persisted_protected_value_is_rejected() {
        let metadata = RuntimeMetadataV1 {
            package: PackageIdentity::new("litehouse", "fixture", Version::V1).unwrap(),
            runtime_profile: PackageRequirement {
                namespace: "litehouse".into(),
                name: "runtime-profile".into(),
                minimum: Version::V1,
            },
            description: None,
            provides: Vec::new(),
            requires: Vec::new(),
            host_facilities: HostFacilityRequests::default(),
            config_schema: None,
            config_annotations: vec![AnnotatedConfigField {
                json_pointer: "/token".into(),
                annotation: ConfigFieldAnnotation::EnvironmentSecret {
                    field_token: "API_TOKEN".into(),
                    encoding: SecretEncoding::Utf8,
                },
            }],
        };
        let persisted = serde_json::json!({"token": "must-not-be-here"});

        let result = prepare_plugin_config_with(
            Some("00112233-4455-6677-8899-aabbccddeeff"),
            Some(&persisted),
            &metadata,
            |_| Some(OsString::from("protected")),
        );

        let error = result.unwrap_err().to_string();
        assert!(!error.contains("must-not-be-here"));
    }

    #[test]
    fn protected_values_are_nonempty_and_bounded() {
        let metadata = protected_metadata();
        let config = serde_json::json!({});
        let id = Some("00112233-4455-6677-8899-aabbccddeeff");

        assert!(
            prepare_plugin_config_with(id, Some(&config), &metadata, |_| {
                Some(OsString::from(""))
            })
            .is_err()
        );
        assert!(
            prepare_plugin_config_with(id, Some(&config), &metadata, |_| {
                Some(OsString::from("x".repeat(64 * 1024 + 1)))
            })
            .is_err()
        );
        assert!(
            prepare_plugin_config_with(id, Some(&config), &metadata, |_| {
                Some(OsString::from("x".repeat(64 * 1024)))
            })
            .is_ok()
        );
    }

    #[tokio::test]
    async fn store_queue_wait_consumes_the_guest_deadline() {
        let strategy = StoreStrategy::global(
            wasmtime::Engine::default(),
            PluginRunnerFactory::new((), Vec::new()),
        );
        let mut occupied = strategy.get("first", &[]).unwrap();
        let mut waiting = strategy.get("second", &[]).unwrap();
        let _guard = occupied.enter().await;
        let deadline = tokio::time::Instant::now() + Duration::from_millis(10);

        assert!(
            enter_store_before(&mut waiting, deadline, "fixture")
                .await
                .is_err()
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn engine_epoch_interrupts_a_non_yielding_guest() {
        let (engine, _linker, _cache) = set_up_engine(false).await.unwrap();
        let task = tokio::task::spawn_blocking(move || {
            let module = wasmtime::Module::new(
                &engine,
                "(module (func (export \"spin\") (loop $spin (br $spin))))",
            )
            .unwrap();
            let mut store = wasmtime::Store::new(&engine, ());
            store.set_epoch_deadline(1);
            let instance = wasmtime::Instance::new(&mut store, &module, &[]).unwrap();
            instance
                .get_typed_func::<(), ()>(&mut store, "spin")
                .unwrap()
                .call(&mut store, ())
                .is_err()
        });

        let interrupted = tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .expect("epoch ticker did not interrupt the guest")
            .unwrap();
        assert!(interrupted);
    }
}
