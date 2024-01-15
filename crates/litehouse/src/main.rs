use std::sync::Arc;

use clap::{Parser, Subcommand};
use futures::{future::join_all, StreamExt};
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
use tokio::{sync::Mutex, time::interval};

use wasmtime::{
    component::{Linker as ComponentLinker, *},
    Config, Engine, Store,
};

mod runtime;

#[derive(Parser)]
struct Opt {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Generate,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let opt = Opt::parse();

    match opt.command {
        None => start().await,
        Some(Command::Generate) => generate().await,
    }
}

async fn start() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.wasm_component_model(true).async_support(true);

    let engine = Engine::new(&config)?;

    let mut linker = ComponentLinker::new(&engine);
    wasmtime_wasi::preview2::command::add_to_linker(&mut linker)?;
    wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |s| s)?;
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker(&mut linker, |s| s)?;
    PluginHost::add_root_to_linker(&mut linker, |c| c).unwrap();

    let store = Arc::new(Mutex::new(Store::new(&engine, PluginRunner::new())));
    let linker = Arc::new(linker);

    let bindings = join_all(
        ["tasmota.wasm"]
            .into_iter()
            .map(|p| Component::from_file(&engine, p).unwrap())
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

    let timers = bindings.into_iter().map(|b| {
        let store = store.clone();
        async move {
            let runner = b.litehouse_plugin_plugin().runner();

            let instance = runner
                .call_constructor(&mut *store.lock().await)
                .await
                .unwrap();

            let subs = runner
                .call_subscribe(&mut *store.lock().await, instance)
                .await
                .unwrap()
                .unwrap();

            let mut streams = tokio_stream::StreamMap::new();
            for (idx, sub) in subs.into_iter().enumerate() {
                let (unit, amount) = match sub {
                    Subscription::Time(TimeSubscription::Every(Every { amount, unit })) => {
                        (unit, amount)
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

            while let Some(_next) = streams.next().await {
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

    futures::future::join_all(timers).await;

    Ok(())
}

#[derive(plugin::JsonSchema)]
struct LitehouseConfig {
    plugins: Vec<PluginInstance>,
}

#[derive(plugin::JsonSchema)]
struct PluginInstance {
    identifier: String,
    version: String,
    config: serde_json::Map<String, serde_json::Value>,
}

async fn generate() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = Config::new();
    config.wasm_component_model(true).async_support(true);

    let engine = Engine::new(&config)?;

    let mut linker = ComponentLinker::new(&engine);
    wasmtime_wasi::preview2::command::add_to_linker(&mut linker)?;
    wasmtime_wasi_http::bindings::http::types::add_to_linker(&mut linker, |s| s)?;
    wasmtime_wasi_http::bindings::http::outgoing_handler::add_to_linker(&mut linker, |s| s)?;
    PluginHost::add_root_to_linker(&mut linker, |c| c).unwrap();

    let store = Arc::new(Mutex::new(Store::new(&engine, PluginRunner::new())));
    let linker = Arc::new(linker);

    let bindings = join_all(
        std::fs::read_dir("wasm")
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
            .map(|(identifier, version, schema)| {
                let mut config_base = base.clone();
                *config_base
                    .get_mut("properties")
                    .unwrap()
                    .get_mut("identifier")
                    .unwrap() =
                    serde_json::Map::from_iter([("const".into(), identifier.into())].into_iter())
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

    Ok(())
}
