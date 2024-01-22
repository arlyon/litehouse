use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::{Parser, Subcommand};
use futures::future::join_all;
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
use serde::Deserialize;
use tokio::{
    sync::{broadcast::channel, Mutex},
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

#[derive(Subcommand)]
enum Command {
    /// Run the litehouse server
    Run {
        /// The path to look for wasm files in.
        #[clap(default_value = "wasm", short, long)]
        wasm_path: PathBuf,
    },
    /// Generate a jsonschema for the config file, based on the
    /// plugins that are in your wasm path
    Generate {
        /// The path to look for wasm files in.
        #[clap(default_value = "wasm", short, long)]
        wasm_path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let console_layer = console_subscriber::spawn();

    tracing_subscriber::registry()
        .with(console_layer)
        .with(tracing_subscriber::fmt::layer().with_filter(EnvFilter::from_default_env()))
        .init();

    let opt = Opt::parse();

    match opt.command {
        Command::Run { wasm_path } => start(&wasm_path).await,
        Command::Generate { wasm_path } => generate(&wasm_path).await,
    }
}

async fn start(wasm_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("booting litehouse");

    let file = std::fs::File::open("settings.json").unwrap();
    let config: LitehouseConfig = serde_json::from_reader(&file).unwrap();

    let mut wasm_config = Config::new();
    wasm_config.wasm_component_model(true).async_support(true);

    let engine = Engine::new(&wasm_config)?;

    let mut linker = ComponentLinker::new(&engine);
    wasmtime_wasi::preview2::command::add_to_linker(&mut linker)?;
    wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |s| s)?;
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker(&mut linker, |s| s)?;
    PluginHost::add_root_to_linker(&mut linker, |c| c).unwrap();

    tracing::debug!("linking complete");

    let (tx, rx) = channel(1000);
    let store = Arc::new(Mutex::new(Store::new(&engine, PluginRunner::new(tx))));

    let linker = Arc::new(linker);
    let rx = Arc::new(rx);

    // todo: only produce as many plugin hosts as there are plugin types rather than instances
    let bindings = instantiate_plugins(&engine, &store, &linker, config.plugins, wasm_path).await;

    let timers = bindings.into_iter().map(|(p, host)| {
        let store = store.clone();
        let rx = rx.resubscribe();
        async move {
            let runner = host.litehouse_plugin_plugin().runner();

            let config = serde_json::to_string(&p.config).unwrap();

            let instance = runner
                .call_constructor(&mut *store.lock().await, &p.nickname, Some(&config))
                .await
                .unwrap();

            let subs = runner
                .call_subscribe(&mut *store.lock().await, instance)
                .await
                .unwrap()
                .unwrap();

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
                    .unwrap()
                    .unwrap();
            }
        }
    });

    tracing::info!("running litehouse");

    futures::future::join_all(timers).await;

    Ok(())
}

#[tracing::instrument(skip(engine, store, linker, plugins))]
async fn instantiate_plugins<T: Send>(
    engine: &Engine,
    store: &Mutex<Store<PluginRunner<T>>>,
    linker: &ComponentLinker<PluginRunner<T>>,
    plugins: Vec<PluginInstance>,
    wasm_path: &Path,
) -> Vec<(PluginInstance, PluginHost)> {
    tracing::debug!("instantiating plugins");
    let plugins = join_all(
        plugins
            .into_iter()
            .map(|p| {
                (
                    Component::from_file(
                        &engine,
                        wasm_path.join(format!("{}.wasm", p.plugin_name)),
                    )
                    .unwrap(),
                    p,
                )
            })
            .map(|(c, p)| async move {
                let (bindings, _) =
                    PluginHost::instantiate_async(&mut *store.lock().await, &c, &linker)
                        .await
                        .unwrap();

                tracing::debug!("instantiated plugin {:?}", p.nickname);

                (p, bindings)
            }),
    )
    .await;
    tracing::debug!("plugins instantiated");
    plugins
}

#[derive(plugin::JsonSchema, Deserialize, Debug)]
struct LitehouseConfig {
    plugins: Vec<PluginInstance>,
}

#[derive(plugin::JsonSchema, Deserialize, Debug)]
struct PluginInstance {
    plugin_name: String,
    version: String,
    nickname: String,
    config: serde_json::Map<String, serde_json::Value>,
}

async fn generate(wasm_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.wasm_component_model(true).async_support(true);

    let engine = Engine::new(&config)?;

    let mut linker = ComponentLinker::new(&engine);
    wasmtime_wasi::preview2::command::add_to_linker(&mut linker)?;
    wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |s| s)?;
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker(&mut linker, |s| s)?;
    PluginHost::add_root_to_linker(&mut linker, |c| c).unwrap();

    let (tx, _rx) = channel(1000);

    let store = Arc::new(Mutex::new(Store::new(&engine, PluginRunner::new(tx))));
    let linker = Arc::new(linker);

    let bindings = join_all(
        std::fs::read_dir(wasm_path)
            .unwrap()
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
    let mut json = serde_json::to_value(&config_schema).unwrap();

    let definitions = json
        .get_mut("definitions")
        .unwrap()
        .get_mut("PluginInstance")
        .unwrap()
        .as_object_mut()
        .unwrap();

    let base = std::mem::replace(definitions, Default::default());

    definitions.insert(
        "oneOf".to_string(),
        schemas
            .into_iter()
            .map(|(plugin_name, version, schema)| {
                let mut config_base = base.clone();
                *config_base
                    .get_mut("properties")
                    .unwrap()
                    .get_mut("plugin_name")
                    .unwrap() =
                    serde_json::Map::from_iter([("const".into(), plugin_name.into())].into_iter())
                        .into();
                *config_base
                    .get_mut("properties")
                    .unwrap()
                    .get_mut("version")
                    .unwrap() =
                    serde_json::Map::from_iter([("const".into(), version.into())].into_iter())
                        .into();

                let mut schema = schema.unwrap().take();
                schema.as_object_mut().unwrap().remove("$schema");
                schema.as_object_mut().unwrap().remove("title");

                *config_base
                    .get_mut("properties")
                    .unwrap()
                    .get_mut("config")
                    .unwrap() = schema;

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
