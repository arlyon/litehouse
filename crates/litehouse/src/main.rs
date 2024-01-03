use std::sync::Arc;

use futures::{future::join_all, StreamExt};
use runtime::{
    bindings::{
        exports::litehouse::plugin::plugin::{
            Event, Every, Subscription, TimeSubscription, TimeUnit, Update,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

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
        ["weather.wasm", "tasmota.wasm"]
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
