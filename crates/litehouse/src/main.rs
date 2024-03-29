use std::collections::HashMap;
use std::future;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::Parser;
use futures::StreamExt;
use miette::Context;
use registry::{Download, Registry, Upload};
use tokio::process::Command;

use cache::ModuleCache;
use futures::TryStreamExt as _;
use itertools::Itertools;
use jsonc_parser::common::Ranged;
use jsonc_parser::CollectOptions;
use jsonschema::error::ValidationErrorKind;
use jsonschema::paths::{JSONPointer, PathChunk};
use litehouse_config::{
    ConfigError, Import, LitehouseConfig, ParseError, PluginInstance, SandboxStrategy,
};
use litehouse_plugin::serde_json::{self, Value};
use miette::{miette, Diagnostic, IntoDiagnostic, NamedSource, Result, SourceSpan};
use runtime::PluginRunnerFactory;
use runtime::{
    bindings::{
        exports::litehouse::plugin::plugin::{
            Event, Every, Metadata, Subscription, TimeSubscription, TimeUnit, Update,
        },
        PluginHost,
    },
    PluginRunner,
};
use store::{StoreRef, StoreStrategy};
use tokio::{
    sync::broadcast::{channel, Sender},
    time::interval,
};
use tokio_stream::wrappers::ReadDirStream;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use wasmtime::{
    component::{Linker as ComponentLinker, *},
    Config, Engine,
};

mod cache;
mod cache_layer;
mod registry;
mod runtime;
mod store;

#[derive(clap::Parser)]
struct Opt {
    #[command(subcommand)]
    command: Subcommand,
}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

const WASM_PROCESS_FILE: &[u8] =
    include_bytes!("../../litehouse/wasi_snapshot_preview1.reactor.wasm");

#[derive(clap::Subcommand)]
enum Subcommand {
    /// Run the litehouse server
    Run {
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
        /// Whether to enable the wasm cache
        #[clap(long)]
        no_cache: bool,
    },
    /// Generate a jsonschema for the config file, based on the
    /// plugins that are in your wasm path
    Generate {
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
        /// Whether to enable the wasm cache
        #[clap(long, default_value_t = true)]
        no_cache: bool,
    },
    Init,
    Validate {
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
        /// Whether to enable the wasm cache
        #[clap(long, default_value_t = true)]
        no_cache: bool,
    },
    /// Publish a package to the registry. Run this in the root of your package.
    Publish {
        package: String,
        #[clap(long)]
        access_key: String,
        #[clap(long)]
        secret_key: String,
    },
    /// Fetch packages from the registry, based on the imports in your settings file.
    Fetch {
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
    },
    /// Build a package and write the wasm file to the specified path.
    Build {
        package: String,
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
        #[clap(long, default_value_t = false)]
        debug: bool,
    },
    /// Search for a package in the registry.
    Search {
        /// The plugin to search for.
        query: Option<String>,
    },
    /// Lock all the packages in your settings file, by setting their
    /// hashes (if they don't exist).
    Lock {
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
    },
}

fn main() -> Result<()> {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    main_inner()
}

#[tokio::main]
async fn main_inner() -> Result<()> {
    let console_layer = console_subscriber::spawn();

    miette::set_hook(Box::new(|_| {
        Box::new(
            miette::MietteHandlerOpts::new()
                .terminal_links(true)
                .context_lines(2)
                .tab_width(2)
                .build(),
        )
    }))
    .unwrap();

    let registry = Registry::build("default".to_string());

    tracing_subscriber::registry()
        .with(console_layer)
        .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env()))
        .init();

    let opt = Opt::parse();

    match opt.command {
        Subcommand::Init => {
            std::fs::create_dir_all("wasm")
                .into_diagnostic()
                .wrap_err("unable to create wasm directory")?;
            if std::fs::metadata("settings.json").is_ok() {
                return Ok(());
            }
            let default = LitehouseConfig::default();
            let mut value = serde_json::to_value(default).expect("can't fail");
            value
                .as_object_mut()
                .expect("can't fail")
                .insert("$schema".to_string(), "schema.json".into());
            let mut file = std::fs::File::create("settings.json")
                .into_diagnostic()
                .wrap_err("unable to create settings.json")?;
            serde_json::to_writer_pretty(&mut file, &value)
                .into_diagnostic()
                .wrap_err("unable to write settings.json")?;

            Ok(())
        }
        Subcommand::Run {
            wasm_path,
            no_cache,
        } => start(&wasm_path, !no_cache)
            .await
            .wrap_err("unable to start litehouse"),
        Subcommand::Generate {
            wasm_path,
            no_cache,
        } => {
            let schema = generate(&wasm_path, !no_cache)
                .await
                .wrap_err("unable to generate schema")?;

            // write file
            let mut file = std::fs::File::create("schema.json")
                .into_diagnostic()
                .wrap_err("unable to create schema file")?;
            serde_json::to_writer_pretty(&mut file, &schema)
                .into_diagnostic()
                .wrap_err("unable to write schema file")?;

            println!("wrote jsonschema to schema.json");
            Ok(())
        }
        Subcommand::Validate {
            wasm_path,
            no_cache,
        } => {
            let schema = generate(&wasm_path, !no_cache)
                .await
                .wrap_err("unable to generate schema")?;
            let schema = jsonschema::JSONSchema::compile(&schema).expect("can't fail");

            let data = std::fs::read_to_string("settings.json")
                .into_diagnostic()
                .wrap_err("could not read settings")?;

            let data2 = data.clone();

            let ast = jsonc_parser::parse_to_ast(
                &data,
                &CollectOptions {
                    comments: true,
                    tokens: true,
                },
                &Default::default(),
            )
            .map_err(|e| {
                ConfigError::Parse(ParseError {
                    err_span: (e.range.start, e.range.end).into(),
                    src: NamedSource::new("settings.json", data.clone()),
                    error: e.message,
                })
            })?;

            let config = ast.value.expect("has a value");

            schema
                .validate(&config.clone().into())
                .map_err(move |e| {
                    FailedValidations::new(
                        e.into_iter()
                            .map(|e| {
                                let mut iter = e.instance_path.iter();
                                let fst = iter.next();
                                let snd = iter.next();
                                let label = match (&e.kind, fst, snd) {
                                    (
                                        ValidationErrorKind::OneOfNotValid,
                                        Some(PathChunk::Property(p)),
                                        Some(PathChunk::Property(plugin)),
                                    ) if p.as_ref() == "plugins" => {
                                        format!("invalid plugin definiton for `{}`", plugin)
                                    }
                                    _ => {
                                        format!("invalid setting")
                                    }
                                };

                                FailedValidation {
                                    span: resolve_span(&config, &e.instance_path),
                                    message: e.to_string(),
                                    label,
                                    kind: e.kind,
                                    error: e.instance.into_owned(),
                                    instance_path: e.instance_path,
                                    schema_path: e.schema_path,
                                }
                            })
                            .collect(),
                        data2,
                    )
                })
                .wrap_err("could not validate settings.json")
        }
        Subcommand::Publish {
            package,
            access_key,
            secret_key,
        } => Ok(publish(
            package,
            &registry
                .with_upload(access_key, secret_key)
                .build()
                .await
                .wrap_err("can't download")?,
        )
        .await),
        Subcommand::Fetch { wasm_path } => {
            let cache_dir = litehouse_config::directories().map(|d| d.cache_dir().to_owned());

            Ok(fetch(
                &registry
                    .with_download(wasm_path, cache_dir)
                    .build()
                    .await
                    .wrap_err("can't fetch")?,
            )
            .await)
        }
        Subcommand::Build {
            wasm_path,
            package,
            debug,
        } => Ok(build(&package, &wasm_path, debug).await),
        Subcommand::Search { query } => {
            let prefix = query.map(|q| Import {
                plugin: q,
                registry: None,
                version: None,
                sha: None,
            });
            let registry = registry.build().await.wrap_err("can't search")?;
            let results = registry.list(prefix.as_ref()).await;
            for (import, _) in results {
                println!("{}", import.to_string());
            }
            Ok(())
        }
        Subcommand::Lock { wasm_path } => Ok(lock(&wasm_path).await),
    }
}

fn resolve_span(config: &jsonc_parser::ast::Value, pointer: &JSONPointer) -> Option<SourceSpan> {
    let range = resolve_pointer(config, pointer)?;
    let range = range.range();
    Some((range.start, range.width()).into())
}

fn resolve_pointer<'a>(
    config: &'a jsonc_parser::ast::Value,
    pointer: &JSONPointer,
) -> Option<&'a jsonc_parser::ast::Value<'a>> {
    let mut config = config;
    for part in pointer.iter() {
        match part {
            jsonschema::paths::PathChunk::Property(name) => {
                config = &config.as_object()?.get(name)?.value
            }
            jsonschema::paths::PathChunk::Index(idx) => {
                config = config.as_array()?.elements.get(*idx)?;
            }
            jsonschema::paths::PathChunk::Keyword(_) => todo!(),
        };
    }
    Some(config)
}

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
#[error("{count} validation errors found in settings.json")]
#[diagnostic(
    help("resolve all the below errors to continue"),
    url(docsrs),
    code(config::invalid)
)]
struct FailedValidations {
    #[source_code]
    src: NamedSource<String>,
    #[related]
    errors: Vec<FailedValidation>,
    count: usize,
}

impl FailedValidations {
    fn new(errors: Vec<FailedValidation>, src: String) -> Self {
        Self {
            count: errors.len(),
            errors,
            src: NamedSource::new("settings.json", src),
        }
    }
}

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
#[error("failed to validate {instance_path}")]
#[diagnostic(help("{message}"))]
struct FailedValidation {
    error: Value,
    kind: jsonschema::error::ValidationErrorKind,
    message: String,
    label: String,
    #[label("{label}")]
    span: Option<SourceSpan>,

    schema_path: jsonschema::paths::JSONPointer,
    instance_path: jsonschema::paths::JSONPointer,
}

async fn start(wasm_path: &Path, cache: bool) -> Result<()> {
    tracing::info!("booting litehouse");

    let config = LitehouseConfig::load().wrap_err("unable to read settings")?;
    let data = Arc::new(
        std::fs::read_to_string("settings.json")
            .into_diagnostic()
            .wrap_err("could not read settings")?,
    );
    let ast = jsonc_parser::parse_to_ast(
        &data,
        &CollectOptions {
            comments: false,
            tokens: false,
        },
        &Default::default(),
    )
    .unwrap();

    let (engine, linker, cache) = set_up_engine(cache).await?;

    tracing::debug!("linking complete");

    let (tx, rx) = channel(1000);

    let linker = Arc::new(linker);
    let rx = Arc::new(rx);

    let factory = PluginRunnerFactory::new(tx.clone(), config.capabilities.clone());

    let plugins = instantiate_plugin_hosts(
        Some((
            NamedSource::new("settings.json", data.clone()),
            ast.value.as_ref().unwrap(),
        )),
        &engine,
        match config.engine.sandbox_strategy {
            SandboxStrategy::Global => StoreStrategy::global(engine.clone(), factory),
            SandboxStrategy::Plugin => StoreStrategy::per_plugin(engine.clone(), factory),
            SandboxStrategy::Instance => StoreStrategy::per_instance(engine.clone(), factory),
        },
        &linker,
        &config.plugins,
        wasm_path,
        config.engine.max_parallel_builds.into(),
        config.engine.max_parallel_instantiations.into(),
    )
    .await
    .wrap_err("unable to instantiate plugins")?;
    if let Some(cache) = cache {
        if let Err(e) = cache.drain().await {
            tracing::warn!("unable to save cache: {}", e)
        }
    }

    let timers = plugins
        .into_iter()
        .map(|(nickname, instance, host, mut store)| {
            let rx = rx.resubscribe();

            async move {
                let runner = host.litehouse_plugin_plugin().runner();

                let config = serde_json::to_string(&instance.config).expect("can't fail");

                let (instance, subs) = {
                    let mut store = store.enter().await;
                    let instance = runner
                        .call_constructor(&mut store, nickname, Some(&config))
                        .await
                        .map_err(|e| miette!("failed to construct plugin: {:?}", e))?;
                    let subs = runner
                        .call_subscribe(&mut store, instance)
                        .await
                        .map_err(|e| miette!("plugin {} failed to subscribe: {:?}", nickname, e))?
                        .map_err(|e| miette!("plugin {} failed to subscribe: {}", nickname, e))?;
                    (instance, subs)
                };

                let mut listen_types = vec![];

                let mut streams = tokio_stream::StreamMap::new();
                for (idx, sub) in subs.into_iter().enumerate() {
                    let (unit, amount) = match sub {
                        Subscription::Time(TimeSubscription::Every(Every { amount, unit })) => {
                            (unit, amount)
                        }
                        Subscription::Update(u) => {
                            tracing::info!("got subscription for {:?}", u);
                            listen_types.push(u);
                            continue;
                        }
                        _ => continue,
                    };

                    let duration = match unit {
                        TimeUnit::Second => std::time::Duration::from_secs(amount),
                        TimeUnit::Minute => std::time::Duration::from_secs(amount * 60),
                        TimeUnit::Hour => std::time::Duration::from_secs(amount * 60 * 60),
                        TimeUnit::Day => std::time::Duration::from_secs(amount * 60 * 60 * 24),
                        TimeUnit::Week => std::time::Duration::from_secs(amount * 60 * 60 * 24 * 7),
                        TimeUnit::Month => {
                            std::time::Duration::from_secs(amount * 60 * 60 * 24 * 30)
                        }
                        TimeUnit::Year => {
                            std::time::Duration::from_secs(amount * 60 * 60 * 24 * 365)
                        }
                    };

                    streams.insert(
                        idx,
                        tokio_stream::wrappers::IntervalStream::new(interval(duration)),
                    );
                }

                let event_stream = tokio_stream::wrappers::BroadcastStream::new(rx);

                let mut merged_stream = tokio_stream::StreamExt::merge(
                    streams.map(|_| runtime::bindings::litehouse::plugin::plugin::Update::Time(0)),
                    event_stream
                        .filter_map(|u| future::ready(u.ok()))
                        .filter(|(name, event)| {
                            future::ready(
                                name != nickname && listen_types.iter().any(|t| t.matches(event)),
                            )
                        })
                        .map(|(_, update)| update),
                );

                while let Some(_update) = merged_stream.next().await {
                    let store = store.enter().await;
                    match runner
                        .call_update(
                            store,
                            instance,
                            &[Event {
                                id: 0,
                                timestamp: 0,
                                inner: Update::Time(0),
                            }],
                        )
                        .await
                    {
                        Ok(Ok(_)) => {}
                        Ok(Err(e)) => {
                            tracing::error!("plugin {} failed to subscribe: {}", nickname, e)
                        }
                        Err(_) => tracing::error!("plugin {} failed to update", nickname),
                    };
                }

                Ok(())
            }
        });

    tracing::info!("running litehouse");

    tokio::select! {
        d = futures::future::try_join_all(timers) => d.map(|_| ()),
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("interrupt received, exiting");
            Ok(())
        }
    }
}

#[tracing::instrument]
async fn set_up_engine(
    cache: bool,
) -> Result<
    (
        Engine,
        Linker<
            PluginRunner<Sender<(String, runtime::bindings::litehouse::plugin::plugin::Update)>>,
        >,
        Option<Arc<ModuleCache>>,
    ),
    miette::Error,
> {
    tracing::debug!("setting up engine");
    let mut wasm_config = Config::new();

    wasm_config
        .wasm_component_model(true)
        .async_support(true)
        .async_stack_size(1 << 20); // 1MiB

    let cache = if cache == true {
        let cache = Arc::new(ModuleCache::load().await?.unwrap_or_default());
        wasm_config
            .enable_incremental_compilation(cache.clone())
            .unwrap();
        Some(cache)
    } else {
        None
    };

    let engine = Engine::new(&wasm_config).map_err(|_| miette!("invalid wasm config"))?;
    let mut linker = ComponentLinker::new(&engine);

    tracing::debug!("linking command");
    wasmtime_wasi::command::add_to_linker(&mut linker)
        .map_err(|_| miette!("unable to add command to linker"))?;

    tracing::debug!("linking http types");
    wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |s| s)
        .map_err(|_| miette!("unable to add http to linker"))?;

    tracing::debug!("linking outgoing handler");
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker(&mut linker, |s| s)
        .map_err(|_| miette!("unable to add outgoing handler to linker"))?;

    tracing::debug!("linking plugin host");
    PluginHost::add_root_to_linker(&mut linker, |c| c)
        .map_err(|_| miette!("unable to add plugin host to linker"))?;

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
struct PluginLoadError {
    #[source_code]
    src: NamedSource<Arc<String>>,
    file_path: String,
    #[label("referenced here")]
    label: SourceSpan,
    #[source]
    source: std::io::Error,
}

#[tracing::instrument(skip(ast, engine, store_builder, linker, plugins))]
async fn instantiate_plugin_hosts<'a, T: Send + Clone>(
    ast: Option<(NamedSource<Arc<String>>, &jsonc_parser::ast::Value<'a>)>,
    engine: &Engine,
    store_builder: StoreStrategy<T>,
    linker: &ComponentLinker<PluginRunner<T>>,
    plugins: &'a HashMap<String, PluginInstance>,
    base_path: &Path,
    max_parallel_builds: u8,
    max_parallel_instantiations: u8,
) -> Result<Vec<(&'a String, &'a PluginInstance, PluginHost, StoreRef<T>)>> {
    tracing::debug!("instantiating plugin hosts");

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
                        todo!()
                    }
                })?;
                let c = tokio::task::spawn_blocking(move || Component::new(&engine, contents))
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
                let (host, store) = instantiate_plugin_host(
                    instance.plugin.file_name(),
                    &store_builder,
                    linker,
                    &component,
                )
                .await?;
                Ok((nick, instance, host, store))
            }
        })
        .buffer_unordered(max_parallel_instantiations.into()) // max parallel instantiations
        .try_collect::<Vec<_>>()
        .await;

    tracing::debug!("instantiated plugin hosts");

    hosts
}

#[tracing::instrument(skip(store_builder, linker, component))]
async fn instantiate_plugin_host<T: Send + Clone>(
    file_name: String,
    store_builder: &StoreStrategy<T>,
    linker: &ComponentLinker<PluginRunner<T>>,
    component: &Component,
) -> Result<(PluginHost, StoreRef<T>)> {
    tracing::debug!("instantiating");
    let mut store = store_builder.get(&file_name);
    let store_lock = store.enter().await;
    let (host, _) = PluginHost::instantiate_async(store_lock, component, linker)
        .await
        .map_err(|e| miette!("unable to instantiate {}: {}", &file_name, e))?;

    Ok((host, store))
}

async fn generate(wasm_path: &Path, cache: bool) -> Result<serde_json::Value> {
    let (engine, linker, cache) = set_up_engine(cache).await?;

    let (tx, _rx) = channel(1);

    let store = StoreStrategy::global(engine.clone(), PluginRunnerFactory::new(tx, vec![]));

    let linker = Arc::new(linker);
    let engine = Arc::new(engine);

    let dirs: Result<HashMap<_, _>> = ReadDirStream::new(
        tokio::fs::read_dir(wasm_path)
            .await
            .into_diagnostic()
            .wrap_err_with(|| format!("unable to read modules in `{}`", wasm_path.display()))?,
    )
    .map(|dir| {
        let file_name = dir
            .as_ref()
            .unwrap()
            .file_name()
            .into_string()
            .map_err(|_| miette!("unable to parse file name as string"))?;
        let plugin: Import = file_name.parse().wrap_err("invalid wasm plugin name")?;
        Ok((
            dir.unwrap().file_name().into_string().unwrap(),
            PluginInstance {
                config: None,
                plugin,
            },
        ))
    })
    .try_collect()
    .await;

    let dirs = dirs?;

    let hosts =
        instantiate_plugin_hosts(None, &engine, store, &linker, &dirs, wasm_path, 10, 10).await?;
    if let Some(cache) = cache {
        if let Err(e) = cache.drain().await {
            tracing::warn!("unable to save cache: {}", e)
        }
    }

    if hosts.is_empty() {
        tracing::warn!("no plugins found in `{}`", wasm_path.display());
    }

    let jobs = hosts.into_iter().map(|(a, instance, host, mut store)| {
        async move {
            let store = store.enter().await;
            let metadata = host
                .litehouse_plugin_plugin()
                .call_get_metadata(store)
                .await;

            match metadata {
                Ok(Metadata {
                    config_schema,
                    identifier,
                    version,
                }) => {
                    // check that version above and version here match

                    if instance.plugin.plugin != identifier {
                        tracing::error!(
                            "plugin identifier mismatch: {} != {}",
                            instance.plugin.plugin,
                            identifier
                        );
                        return Err(miette!("plugin identifier mismatch"));
                    };

                    let version = version.parse().into_diagnostic()?;
                    if let Some(version_exp) = &instance.plugin.version {
                        if version_exp != &version {
                            return Err(VersionMismatch {
                                file_exp: format!("{}@{}.wasm", identifier, version),
                                file_path: wasm_path.join(a).to_string_lossy().to_string(),
                                plugin: identifier,
                                source_code: format!("{} != {}", version, version_exp),
                                expected: (0, version.to_string().len()).into(),
                                actual: (
                                    version.to_string().len() + 4,
                                    version_exp.to_string().len(),
                                )
                                    .into(),
                            }
                            .into());
                        }
                    }

                    Ok((
                        Import {
                            plugin: identifier,
                            version: Some(version),
                            registry: None,
                            sha: None,
                        },
                        config_schema.and_then(|s| serde_json::from_str(&s).ok()),
                    ))
                }
                Err(_) => {
                    tracing::error!("failed to generate schema: {:?}", metadata);
                    panic!();
                }
            }
        }
    });

    let schemas: Vec<(_, Option<serde_json::Value>)> = futures::future::try_join_all(jobs).await?;

    let config_schema = schemars::schema_for!(LitehouseConfig);
    let json = serde_json::to_value(&config_schema).expect("can't fail");

    Ok(inject_plugin_instance(json, schemas.into_iter()))
}

#[derive(miette::Diagnostic, Debug, thiserror::Error)]
#[error("version mismatch for {plugin}")]
#[diagnostic(help(
    "rename the file at `{file_path}` to `{file_exp}` so it matches the version in the plugin"
))]
struct VersionMismatch {
    plugin: String,
    file_path: String,
    #[label("expected")]
    expected: SourceSpan,
    #[label("actual")]
    actual: SourceSpan,
    file_exp: String,
    #[source_code]
    source_code: String,
}

fn inject_plugin_instance(
    mut json: Value,
    plugins: impl Iterator<Item = (Import, Option<Value>)>,
) -> serde_json::Value {
    let definitions = json
        .get_mut("definitions")
        .expect("this is always present")
        .get_mut("PluginInstance")
        .expect("always exists")
        .as_object_mut()
        .expect("is always an object");

    let base = std::mem::take(definitions);

    definitions.insert(
        "oneOf".to_string(),
        plugins
            .map(|(import, schema)| {
                let mut config_base = base.clone();
                let properties = config_base
                    .get_mut("properties")
                    .expect("always exists")
                    .as_object_mut()
                    .expect("is always an object");

                *properties.get_mut("plugin").unwrap() =
                    serde_json::Map::from_iter([("const".into(), import.to_string().into())])
                        .into();

                let set = if let Some(mut schema) = schema {
                    let object = schema.as_object_mut().unwrap();
                    object.remove("$schema");
                    object.remove("title");
                    *properties.get_mut("config").unwrap() = schema;
                    true
                } else {
                    properties.remove("config");
                    false
                };

                let required = config_base
                    .get_mut("required")
                    .unwrap()
                    .as_array_mut()
                    .unwrap();

                match (required.iter().position(|s| s == "config"), set) {
                    (Some(pos), false) => {
                        required.remove(pos);
                    }
                    (None, true) => {
                        required.push("config".into());
                    }
                    _ => {}
                };

                config_base
            })
            .collect(),
    );

    json
}

async fn build_in_temp(package: &str, release: bool) -> Option<(Import, PathBuf)> {
    let workspaces_json = Command::new("cargo")
        .arg("metadata")
        .output()
        .await
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&workspaces_json.stdout).unwrap();

    let members: HashMap<&str, (&str, &str)> = data["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let v = v.as_str().unwrap();
            let (name, rest) = v.split_once(' ').unwrap();
            let (version, rest) = rest.split_once(' ').unwrap();
            let path = rest
                .strip_prefix("(path+file://")
                .unwrap()
                .strip_suffix(')')
                .unwrap();
            (name, (version, path))
        })
        .collect();

    let (version, _path) = members
        .get(package)
        .expect("Package not found in workspace");

    // run cargo build
    let out = Command::new("cargo")
        .args(["build", "--target", "wasm32-wasi", "-p", package])
        .args(if release { &["--release"][..] } else { &[] })
        .status()
        .await
        .unwrap();

    if !out.success() {
        return None;
    }

    let import = Import {
        registry: None,
        plugin: package.to_owned(),
        version: Some(version.parse().unwrap()),
        sha: None,
    };

    // write the wasm file to a temp dir
    let tmp = std::env::temp_dir().join("litehouse");
    let wasi_path = tmp.join("wasi_snapshot_preview1.wasm");
    let out_path = tmp.join(import.file_name());
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(&wasi_path, WASM_PROCESS_FILE).unwrap();

    // run wasm-tools against the wasm file
    let out = Command::new("wasm-tools")
        .args([
            "component",
            "new",
            &format!("./target/wasm32-wasi/release/{}.wasm", package),
            "--adapt",
            wasi_path.to_str().unwrap(),
            "-o",
            out_path.to_str().unwrap(),
        ])
        .status()
        .await
        .unwrap();

    if !out.success() {
        return None;
    }

    Some((import, out_path))
}

async fn build(package: &str, wasm_path: &Path, debug: bool) {
    let (import, path) = build_in_temp(package, !debug).await.unwrap();
    tokio::fs::create_dir_all(wasm_path).await.unwrap();
    let dest_file = wasm_path.join(import.file_name());
    tokio::fs::copy(&path, &dest_file).await.unwrap();
}

async fn publish<D>(package: String, op: &Registry<Upload, D>) {
    let (import, path) = build_in_temp(&package, true).await.unwrap();

    let success = op.publish(&import, &path).await;
    if success {
        println!("Published {}", import.file_name());
    } else {
        println!("Failed to publish {}", import.file_name());
    }
}

async fn fetch<U>(op: &Registry<U, Download>) {
    let config = LitehouseConfig::load().unwrap();

    config
        .imports
        .iter()
        .map(|import| op.download_package(import))
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await;
}

async fn lock(wasm_path: &Path) {
    let mut config = LitehouseConfig::load().unwrap();

    for import in &mut config.imports {
        import.read_sha(wasm_path).await;
    }

    config.save().unwrap();
}
