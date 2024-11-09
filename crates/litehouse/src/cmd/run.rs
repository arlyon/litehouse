use futures::{future::Either, StreamExt};
use jsonc_parser::CollectOptions;
use litehouse_config::{LitehouseConfig, SandboxStrategy};
use litehouse_plugin::serde_json;
use miette::{Context, IntoDiagnostic, NamedSource, Result};
use std::{future, path::Path, sync::Arc};
use tokio::{
    sync::{broadcast::channel, broadcast::Receiver},
    time::interval,
};
use tracing::Instrument;
use wasmtime::Trap;

use crate::{
    runtime::{
        bindings::exports::litehouse::plugin::plugin::{
            Event, Every, Subscription, TimeSubscription, TimeUnit, Update,
        },
        instantiate_plugin_host, instantiate_plugin_hosts, set_up_engine, PluginInstance,
        PluginRunnerFactory,
    },
    server::{Authed, Credentials},
    store::StoreStrategy,
    LogMessage,
};

/// Starts the server and the plugin runners
///
/// # Arguments
///
/// * `wasm_path` - The path to the wasm file
/// * `cache` - Whether to cache the wasm file
/// * `server` - The port to run the server on. If this is `None`, webrtc will not be started.
#[tracing::instrument(skip_all)]
pub async fn run(wasm_path: &Path, cache: bool, logs_rx: Receiver<LogMessage>) -> Result<()> {
    tracing::info!("booting litehouse");
    let config = LitehouseConfig::load().wrap_err("unable to read settings")?;

    let server_fut = if let Some(broker) = config.broker {
        tracing::info!("running server");
        let handle = tokio::spawn(crate::server::facilicate_connections(
            broker.url(),
            broker
                .cert
                .map(Authed)
                .map(Credentials::Authed)
                .unwrap_or(Credentials::Unauthed {
                    password: broker.password,
                }),
            logs_rx,
        ));
        Either::Left(handle)
    } else {
        tracing::info!("starting without broker");
        Either::Right(future::pending())
    };

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

    let store_strategy = match config.engine.sandbox_strategy {
        SandboxStrategy::Global => StoreStrategy::global(engine.clone(), factory),
        SandboxStrategy::Plugin => StoreStrategy::per_plugin(engine.clone(), factory),
        SandboxStrategy::Instance => StoreStrategy::per_instance(engine.clone(), factory),
    };

    let plugins = instantiate_plugin_hosts(
        Some((
            NamedSource::new("settings.json", data.clone()),
            ast.value.as_ref().unwrap(),
        )),
        &engine,
        &store_strategy,
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

    let store_strategy = Arc::new(store_strategy);
    let timers = plugins
        .into_iter()
        .map(|(nickname, instance, mut host, mut store, component)| {
            let rx = rx.resubscribe();
            let store_strategy = store_strategy.clone();
            let linker = linker.clone();
            let component = component.clone();

            async move {
                let mut runner = host.litehouse_plugin_plugin().runner();

                let config = serde_json::to_string(&instance.config).expect("can't fail");

                let (mut plugin, subs) =
                    PluginInstance::new(&mut store, &runner, nickname, &config).await?;

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
                    streams.map(|_| {
                        crate::runtime::bindings::litehouse::plugin::plugin::Update::Time(0)
                    }),
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
                    let store_lock = store.enter().await;
                    match runner
                        .call_update(
                            store_lock,
                            *plugin,
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
                        Err(e) => {
                            // if we have trapped we should restart the plugin
                            if let Some(trap) = e.downcast_ref::<Trap>() {
                                tracing::error!("plugin has crashed, restarting {}", trap);
                                store = store_strategy.reset(nickname).await;
                                host = instantiate_plugin_host(&mut store, &linker, &component)
                                    .await?;
                                runner = host.litehouse_plugin_plugin().runner();
                                (plugin, _) =
                                    PluginInstance::new(&mut store, &runner, nickname, &config)
                                        .await
                                        .expect("can't fail");
                                tracing::debug!("plugin {} restarted", nickname);
                            } else {
                                tracing::error!("plugin {} failed to update: {:?}", nickname, e);
                            }
                        }
                    };
                }

                Ok(())
            }
            .instrument(tracing::info_span!("plugin", nickname = nickname))
        });

    tracing::info!("running litehouse");

    tokio::select! {
        _ = server_fut => Ok(()),
        d = futures::future::try_join_all(timers) => d.map(|_| ()),
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("interrupt received, exiting");
            Ok(())
        }
    }
}
