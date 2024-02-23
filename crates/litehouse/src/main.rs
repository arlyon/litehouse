use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use eyre::{eyre, Result, WrapErr};
use futures::future::{join_all, try_join_all};
use plugin::serde_json;
use runtime::{
    bindings::{
        exports::litehouse::plugin::plugin::{
            Event, Every, Metadata, Subscription, TimeSubscription, TimeUnit, Update,
        },
        PluginHost,
    },
    PluginRunner,
};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{
        broadcast::{channel, Sender},
        Mutex,
    },
    time::interval,
};
use tokio_stream::StreamExt as _;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use wasmtime::{
    component::{Linker as ComponentLinker, *},
    Config, Engine, Store,
};

mod runtime;

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
            let default = LitehouseConfig { plugins: vec![] };
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
        Command::Generate { wasm_path } => generate(&wasm_path).await,
    }
}

async fn start(wasm_path: &Path) -> Result<()> {
    tracing::info!("booting litehouse");

    let file = std::fs::File::open("settings.json").wrap_err("unable to open settings.json")?;
    let config: LitehouseConfig =
        serde_json::from_reader(&file).wrap_err("invalid settings.json")?;

    let (engine, linker) = set_up_engine()?;

    tracing::debug!("linking complete");

    let (tx, rx) = channel(1000);
    let store = Arc::new(Mutex::new(Store::new(&engine, PluginRunner::new(tx))));

    let linker = Arc::new(linker);
    let rx = Arc::new(rx);

    // todo: only produce as many plugin hosts as there are plugin types rather than instances
    let bindings = instantiate_plugins(&engine, &store, &linker, config.plugins, wasm_path)
        .await
        .wrap_err("unable to instantiate plugins")?;

    let timers = bindings.into_iter().map(|(host, p)| {
        let store = store.clone();
        let rx = rx.resubscribe();
        async move {
            let runner = host.litehouse_plugin_plugin().runner();

            let config = serde_json::to_string(&p.config).expect("can't fail");

            let instance = runner
                .call_constructor(&mut *store.lock().await, &p.nickname, Some(&config))
                .await
                .map_err(|e| eyre!("failed to construct plugin: {:?}", e))?;

            let subs = runner
                .call_subscribe(&mut *store.lock().await, instance)
                .await
                .map_err(|e| eyre!("plugin {} failed to subscribe: {:?}", p.nickname, e))?
                .map_err(|e| eyre!("plugin {} failed to subscribe: {}", p.nickname, e))?;

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
                    TimeUnit::Month => std::time::Duration::from_secs(amount * 60 * 60 * 24 * 30),
                    TimeUnit::Year => std::time::Duration::from_secs(amount * 60 * 60 * 24 * 365),
                };

                streams.insert(
                    idx,
                    tokio_stream::wrappers::IntervalStream::new(interval(duration)),
                );
            }

            let event_stream = tokio_stream::wrappers::BroadcastStream::new(rx);

            let mut merged_stream = streams
                .map(|_| runtime::bindings::litehouse::plugin::plugin::Update::Time(0))
                .merge(
                    event_stream
                        .filter_map(|u| u.ok())
                        .filter(|(name, event)| {
                            name != &p.nickname && listen_types.iter().any(|t| t.matches(event))
                        })
                        .map(|(_, update)| update),
                );

            while let Some(_next) = merged_stream.next().await {
                let mut store = store.lock().await;
                runner
                    .call_update(
                        &mut *store,
                        instance,
                        &[Event {
                            id: 0,
                            timestamp: 0,
                            inner: Update::Time(0),
                        }],
                    )
                    .await
                    .map_err(|e| eyre!("plugin {} failed to update: {:?}", p.nickname, e))?
                    .map_err(|e| eyre!("plugin {} failed to subscribe: {}", p.nickname, e))?;
            }

            Ok(())
        }
    });

    tracing::info!("running litehouse");

    futures::future::try_join_all(timers).await.map(|_| ())
}

#[tracing::instrument]
fn set_up_engine() -> Result<
    (
        Engine,
        Linker<
            PluginRunner<Sender<(String, runtime::bindings::litehouse::plugin::plugin::Update)>>,
        >,
    ),
    eyre::Error,
> {
    tracing::debug!("setting up engine");
    let mut wasm_config = Config::new();
    wasm_config.wasm_component_model(true).async_support(true);
    let engine = Engine::new(&wasm_config).map_err(|_| eyre!("invalid wasm config"))?;
    let mut linker = ComponentLinker::new(&engine);

    tracing::debug!("linking command");
    wasmtime_wasi::preview2::command::add_to_linker(&mut linker)
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

    Ok((engine, linker))
}

#[tracing::instrument(skip(engine, store, linker, plugins))]
async fn instantiate_plugins<T: Send>(
    engine: &Engine,
    store: &Mutex<Store<PluginRunner<T>>>,
    linker: &ComponentLinker<PluginRunner<T>>,
    plugins: Vec<PluginInstance>,
    base_path: &Path,
) -> Result<Vec<(PluginHost, PluginInstance)>> {
    tracing::debug!("instantiating plugins");
    let plugins = try_join_all(plugins.into_iter().map(|instance| async move {
        Ok::<_, eyre::ErrReport>((
            instantiate_plugin(&instance, engine, store, linker, base_path).await?,
            instance,
        ))
    }))
    .await?;
    tracing::debug!("instantiated plugins");
    Ok(plugins)
}

#[tracing::instrument(skip(engine, store, linker, instance, base_path), fields(instance = %instance.plugin_name, name=%instance.nickname))]
async fn instantiate_plugin<T: Send>(
    instance: &PluginInstance,
    engine: &Engine,
    store: &Mutex<Store<PluginRunner<T>>>,
    linker: &ComponentLinker<PluginRunner<T>>,
    base_path: &Path,
) -> Result<PluginHost> {
    let component = Component::from_file(
        &engine,
        base_path.join(format!("{}.wasm", instance.plugin_name)),
    )
    .map_err(|e| eyre!("unable to load plugin: {}", e))?;

    let (bindings, _) =
        PluginHost::instantiate_async(&mut *store.lock().await, &component, &linker)
            .await
            .map_err(|e| eyre!("unable to instantiate plugin: {}", e))?;

    tracing::debug!("complete");

    Ok(bindings)
}

#[derive(plugin::JsonSchema, Serialize, Deserialize, Debug)]
struct LitehouseConfig {
    plugins: Vec<PluginInstance>,
}

#[derive(plugin::JsonSchema, Serialize, Deserialize, Debug)]
struct PluginInstance {
    plugin_name: String,
    version: String,
    nickname: String,
    config: Option<serde_json::Value>,
}

async fn generate(wasm_path: &Path) -> Result<()> {
    let (engine, linker) = set_up_engine()?;

    let (tx, _rx) = channel(1000);

    let store = Arc::new(Mutex::new(Store::new(&engine, PluginRunner::new(tx))));
    let linker = Arc::new(linker);

    let bindings = join_all(
        std::fs::read_dir(wasm_path)
            .wrap_err_with(|| format!("unable to read modules in `{}`", wasm_path.display()))?
            .map(|p| Component::from_file(&engine, p.unwrap().path()).unwrap())
            .map(|c| {
                let store = store.clone();
                let linker = linker.clone();
                async move {
                    let (bindings, _) =
                        PluginHost::instantiate_async(&mut *store.lock().await, &c, &linker)
                            .await
                            .unwrap();

                    bindings
                }
            }),
    )
    .await;

    if bindings.is_empty() {
        tracing::warn!("no plugins found in `{}`", wasm_path.display());
    }

    let jobs = bindings.into_iter().map(|plugin| {
        let store = store.clone();
        async move {
            let store = &mut *store.lock().await;
            let metadata = plugin
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
                        identifier,
                        version,
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

    let schemas: Vec<(_, _, Option<serde_json::Value>)> = futures::future::join_all(jobs).await;

    let config_schema = schemars::schema_for!(LitehouseConfig);
    let mut json = serde_json::to_value(&config_schema).expect("can't fail");

    let definitions = json
        .get_mut("definitions")
        .expect("this is always present")
        .get_mut("PluginInstance")
        .expect("always exists")
        .as_object_mut()
        .expect("is always an object");

    let base = std::mem::replace(definitions, Default::default());

    definitions.insert(
        "oneOf".to_string(),
        schemas
            .into_iter()
            .map(|(plugin_name, version, schema)| {
                let mut config_base = base.clone();
                let properties = config_base
                    .get_mut("properties")
                    .expect("always exists")
                    .as_object_mut()
                    .expect("is always an object");
                *properties.get_mut("plugin_name").unwrap() =
                    serde_json::Map::from_iter([("const".into(), plugin_name.into())].into_iter())
                        .into();
                *properties.get_mut("version").unwrap() =
                    serde_json::Map::from_iter([("const".into(), version.into())].into_iter())
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

    // write file
    let mut file = std::fs::File::create("schema.json").unwrap();
    serde_json::to_writer_pretty(&mut file, &json).unwrap();

    println!("wrote jsonschema to schema.json");

    Ok(())
}
