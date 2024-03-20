use std::collections::HashMap;
use std::future;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use cache::ModuleCache;
use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use futures::{StreamExt, TryStreamExt as _};
use itertools::Itertools;
use litehouse_config::{Import, LitehouseConfig, PluginInstance, SandboxStrategy};
use plugin::serde_json::{self, Value};
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
mod runtime;
mod store;

#[derive(Parser)]
struct Opt {
    #[command(subcommand)]
    command: Command,
}

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[derive(Subcommand)]
enum Command {
    /// Run the litehouse server
    Run {
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
    },
    /// Generate a jsonschema for the config file, based on the
    /// plugins that are in your wasm path
    Generate {
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
        wasm_path: PathBuf,
    },
    Init,
    Validate {
        /// The path to look for wasm files in.
        #[clap(default_value = "./wasm", short, long)]
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
    color_eyre::install()?;
    let console_layer = console_subscriber::spawn();

    tracing_subscriber::registry()
        .with(console_layer)
        .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env()))
        .init();

    let opt = Opt::parse();

    match opt.command {
        Command::Init => {
            std::fs::create_dir_all("wasm").wrap_err("unable to create wasm directory")?;
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
                .wrap_err("unable to create settings.json")?;
            serde_json::to_writer_pretty(&mut file, &value)
                .wrap_err("unable to write settings.json")?;

            Ok(())
        }
        Command::Run { wasm_path } => start(&wasm_path).await,
        Command::Generate { wasm_path } => {
            let schema = generate(&wasm_path).await?;

            // write file
            let mut file = std::fs::File::create("schema.json").unwrap();
            serde_json::to_writer_pretty(&mut file, &schema).unwrap();

            println!("wrote jsonschema to schema.json");
            Ok(())
        }
        Command::Validate { wasm_path } => {
            let schema = generate(&wasm_path).await?;
            let schema = jsonschema::JSONSchema::compile(&schema).expect("can't fail");
            let settings =
                std::fs::File::open("settings.json").wrap_err("unable to open settings.json")?;
            let settings: serde_json::Value =
                serde_json::from_reader(&settings).wrap_err("invalid settings.json")?;
            if let Err(errors) = schema.validate(&settings) {
                for error in errors {
                    println!("Validation error: {}", error);
                    println!("Instance path: {}", error.instance_path);
                }
            }
            Ok(())
        }
    }
}

async fn start(wasm_path: &Path) -> Result<()> {
    tracing::info!("booting litehouse");

    let config = LitehouseConfig::load().wrap_err("unable to load settings.json")?;

    let (engine, linker, cache) = set_up_engine().await?;

    tracing::debug!("linking complete");

    let (tx, rx) = channel(1000);

    let linker = Arc::new(linker);
    let rx = Arc::new(rx);

    let factory = PluginRunnerFactory::new(tx.clone(), config.capabilities.clone());

    let plugins = instantiate_plugin_hosts(
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
    if let Err(e) = cache.drain().await {
        tracing::warn!("unable to save cache: {}", e)
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
                        .map_err(|e| eyre!("failed to construct plugin: {:?}", e))?;
                    let subs = runner
                        .call_subscribe(&mut store, instance)
                        .await
                        .map_err(|e| eyre!("plugin {} failed to subscribe: {:?}", nickname, e))?
                        .map_err(|e| eyre!("plugin {} failed to subscribe: {}", nickname, e))?;
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
async fn set_up_engine() -> Result<
    (
        Engine,
        Linker<
            PluginRunner<Sender<(String, runtime::bindings::litehouse::plugin::plugin::Update)>>,
        >,
        Arc<ModuleCache>,
    ),
    eyre::Error,
> {
    tracing::debug!("setting up engine");
    let mut wasm_config = Config::new();

    let cache = Arc::new(ModuleCache::load().await?.unwrap_or_default());

    wasm_config
        .wasm_component_model(true)
        .async_support(true)
        .async_stack_size(1 << 20) // 1MiB
        .enable_incremental_compilation(cache.clone())
        .unwrap();

    let engine = Engine::new(&wasm_config).map_err(|_| eyre!("invalid wasm config"))?;
    let mut linker = ComponentLinker::new(&engine);

    tracing::debug!("linking command");
    wasmtime_wasi::command::add_to_linker(&mut linker)
        .map_err(|_| eyre!("unable to add command to linker"))?;

    tracing::debug!("linking http types");
    wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |s| s)
        .map_err(|_| eyre!("unable to add http to linker"))?;

    tracing::debug!("linking outgoing handler");
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker(&mut linker, |s| s)
        .map_err(|_| eyre!("unable to add outgoing handler to linker"))?;

    tracing::debug!("linking plugin host");
    PluginHost::add_root_to_linker(&mut linker, |c| c)
        .map_err(|_| eyre!("unable to add plugin host to linker"))?;

    tracing::debug!("set up engine");

    Ok((engine, linker, cache))
}

#[tracing::instrument(skip(engine, store_builder, linker, plugins))]
async fn instantiate_plugin_hosts<'a, T: Send + Clone>(
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

    let hosts = tokio_stream::iter(map.into_iter())
        .map(|(file_name, plugins)| {
            let engine = engine.clone();
            async move {
                tracing::debug!("building");
                let contents = tokio::fs::read(base_path.join(&file_name)).await.unwrap();
                let c = tokio::task::spawn_blocking(move || Component::new(&engine, contents))
                    .await
                    .expect("no panic")
                    .map_err(|e| eyre!("unable to build {}: {}", &file_name, e))
                    .unwrap();

                tokio_stream::iter(
                    plugins
                        .into_iter()
                        .map(move |(string, instance)| (string, instance, c.clone())),
                )
            }
        })
        .buffer_unordered(max_parallel_builds.into()) // max parallel builds
        .flatten_unordered(None)
        .map(|(nick, instance, component)| {
            let store_builder = store_builder.clone();
            async move {
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
        .map_err(|e| eyre!("unable to instantiate {}: {}", &file_name, e))?;

    Ok((host, store))
}

async fn generate(wasm_path: &Path) -> Result<serde_json::Value> {
    let (engine, linker, cache) = set_up_engine().await?;

    let (tx, _rx) = channel(1);

    let store = StoreStrategy::global(engine.clone(), PluginRunnerFactory::new(tx, vec![]));

    let linker = Arc::new(linker);
    let engine = Arc::new(engine);

    let dirs: HashMap<_, _> = ReadDirStream::new(
        tokio::fs::read_dir(wasm_path)
            .await
            .wrap_err_with(|| format!("unable to read modules in `{}`", wasm_path.display()))?,
    )
    .map(|dir| {
        let file_name = dir.as_ref().unwrap().file_name().into_string().unwrap();
        let plugin: Import = file_name.parse().unwrap();
        (
            dir.unwrap().file_name().into_string().unwrap(),
            PluginInstance {
                config: None,
                plugin,
            },
        )
    })
    .collect()
    .await;

    let hosts = instantiate_plugin_hosts(&engine, store, &linker, &dirs, wasm_path, 10, 10).await?;
    if let Err(e) = cache.drain().await {
        tracing::warn!("unable to save cache: {}", e)
    }

    if hosts.is_empty() {
        tracing::warn!("no plugins found in `{}`", wasm_path.display());
    }

    let jobs = hosts.into_iter().map(|(_, _, host, mut store)| {
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
                    // write to file
                    (
                        Import {
                            plugin: identifier,
                            version: version.parse().ok(),
                            registry: None,
                            sha: None,
                        },
                        config_schema.and_then(|s| serde_json::from_str(&s).ok()),
                    )
                }
                Err(_) => {
                    tracing::error!("failed to generate schema: {:?}", metadata);
                    panic!();
                }
            }
        }
    });

    let schemas: Vec<(_, Option<serde_json::Value>)> = futures::future::join_all(jobs).await;

    let config_schema = schemars::schema_for!(LitehouseConfig);
    let json = serde_json::to_value(&config_schema).expect("can't fail");

    if let Err(e) = cache.drain().await {
        tracing::warn!("unable to save cache: {}", e)
    }

    Ok(inject_plugin_instance(json, schemas.into_iter()))
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
