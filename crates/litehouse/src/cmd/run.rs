use futures::{StreamExt, future::Either};
use jsonc_parser::CollectOptions;
use litehouse_config::{LitehouseConfig, PluginConfig, SandboxStrategy};
use litehouse_contracts::config::environment_variable_name;
use miette::{Context, IntoDiagnostic, NamedSource, Result, miette};
use std::{collections::HashSet, future, path::Path, sync::Arc, time::SystemTime};
use tokio::{
    sync::{broadcast, broadcast::Receiver, broadcast::channel},
    time::interval,
};
use tracing::Instrument;
use wasmtime::{
    AsContextMut, Trap,
    component::{Component, Instance, Linker},
};

use crate::{
    LogMessage,
    runtime::{
        DEFAULT_GUEST_CALL_TIMEOUT, PluginInstance, PluginRunner, PluginRunnerFactory,
        RuntimeUpdate,
        bindings::{
            exports::litehouse::plugin::plugin::{
                Event, Every, Subscription, TimeSubscription, TimeUnit, UpdateSubscription,
            },
            litehouse::plugin::update::Update,
        },
        epoch_ticks, instantiate_instance, instantiate_plugin_hosts, set_up_engine,
    },
    server::{Authed, Credentials},
    store::{StoreRef, StoreStrategy},
};

type EventSender = broadcast::Sender<RuntimeUpdate>;
type RuntimeStore = StoreRef<EventSender>;
type RuntimeLinker = Linker<PluginRunner<EventSender>>;

#[derive(Debug)]
enum ActorInput {
    Update {
        event: Update,
        admitted_at: tokio::time::Instant,
        continuity_required: bool,
    },
    ContinuityLost(u64),
}

fn classify_actor_input(
    result: std::result::Result<
        RuntimeUpdate,
        tokio_stream::wrappers::errors::BroadcastStreamRecvError,
    >,
    nickname: &str,
    listen_types: &[UpdateSubscription],
) -> Option<ActorInput> {
    match result {
        Ok((name, event, admitted_at))
            if name != nickname
                && listen_types
                    .iter()
                    .any(|subscription| subscription.matches(&event)) =>
        {
            Some(ActorInput::Update {
                event,
                admitted_at,
                continuity_required: true,
            })
        }
        Ok(_) => None,
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(skipped))
            if !listen_types.is_empty() =>
        {
            Some(ActorInput::ContinuityLost(skipped))
        }
        Err(_) => None,
    }
}

async fn restart_plugin(
    store_strategy: &StoreStrategy<EventSender>,
    linker: &RuntimeLinker,
    component: &Component,
    nickname: &str,
    instance: &PluginConfig,
) -> Result<(RuntimeStore, Instance, PluginInstance)> {
    let mut store = store_strategy
        .reset(&instance.plugin.file_name(), &instance.capabilities)
        .await?;
    let host = instantiate_instance(&mut store, linker, component).await?;
    let (plugin, _subscriptions, _metadata) = PluginInstance::new(
        &mut store,
        &host,
        nickname,
        instance.instance_id.as_deref(),
        instance.config.as_ref(),
    )
    .await?;
    Ok((store, host, plugin))
}

fn validate_runtime_configuration(config: &LitehouseConfig) -> Result<()> {
    if !matches!(config.engine.sandbox_strategy, SandboxStrategy::Instance)
        && config
            .plugins
            .values()
            .any(|plugin| !plugin.capabilities.is_empty())
    {
        return Err(miette!(
            "per-instance capabilities require engine.sandbox_strategy = instance"
        ));
    }

    let mut seen = HashSet::new();
    for (nickname, plugin) in &config.plugins {
        let Some(instance_id) = plugin.instance_id.as_deref() else {
            continue;
        };
        environment_variable_name(instance_id, "INSTANCE")
            .map_err(|error| miette!("plugin {nickname} has an invalid instance_id: {error}"))?;
        if !seen.insert(instance_id) {
            return Err(miette!(
                "plugin {nickname} reuses instance_id {instance_id}; instance IDs must be unique"
            ));
        }
    }
    Ok(())
}

async fn contain_actor(
    nickname: &str,
    actor: impl std::future::Future<Output = Result<()>>,
) -> Result<()> {
    if let Err(error) = actor.await {
        tracing::error!(plugin = nickname, ?error, "plugin actor disabled");
        future::pending::<()>().await;
    }
    Ok(())
}

fn timer_duration(unit: TimeUnit, amount: u64) -> Result<std::time::Duration> {
    let seconds_per_unit = match unit {
        TimeUnit::Second => 1,
        TimeUnit::Minute => 60,
        TimeUnit::Hour => 60 * 60,
        TimeUnit::Day => 60 * 60 * 24,
        TimeUnit::Week => 60 * 60 * 24 * 7,
        TimeUnit::Month => 60 * 60 * 24 * 30,
        TimeUnit::Year => 60 * 60 * 24 * 365,
    };
    let seconds = amount
        .checked_mul(seconds_per_unit)
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| miette!("plugin declared an invalid timer period"))?;
    Ok(std::time::Duration::from_secs(seconds))
}

fn timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

/// Starts the server and the plugin runners.
#[tracing::instrument(skip_all)]
pub async fn run(wasm_path: &Path, cache: bool, logs_rx: Receiver<LogMessage>) -> Result<()> {
    tracing::info!("booting litehouse");
    let config = LitehouseConfig::load().wrap_err("unable to read settings")?;
    validate_runtime_configuration(&config)?;
    let workflow_repository =
        litehouse_management::WorkflowRepository::open_with_authoring_root(&config.workflows, ".")
            .into_diagnostic()
            .wrap_err("unable to load authored workflows")?;
    tracing::info!(
        workflow_count = workflow_repository
            .snapshot()
            .into_diagnostic()?
            .workflows()
            .len(),
        "validated authored workflows"
    );

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
    .map_err(|error| miette!("unable to parse settings AST: {error}"))?;

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
            ast.value
                .as_ref()
                .ok_or_else(|| miette!("settings document is empty"))?,
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

    if let Some(cache) = cache
        && let Err(error) = cache.drain().await
    {
        tracing::warn!("unable to save cache: {error}");
    }

    let store_strategy = Arc::new(store_strategy);
    let actors = plugins
        .into_iter()
        .map(|(nickname, instance, host, mut store, component)| {
            let rx = rx.resubscribe();
            let store_strategy = store_strategy.clone();
            let linker = linker.clone();

            async move {
                contain_actor(nickname, async {
                let (mut plugin, subscriptions, _metadata) = PluginInstance::new(
                    &mut store,
                    &host,
                    nickname,
                    instance.instance_id.as_deref(),
                    instance.config.as_ref(),
                )
                .await?;
                let mut host = host;

                let mut listen_types = Vec::new();
                let mut streams = tokio_stream::StreamMap::new();
                for (index, subscription) in subscriptions.into_iter().enumerate() {
                    let (unit, amount) = match subscription {
                        Subscription::Time(TimeSubscription::Every(Every { amount, unit })) => {
                            (unit, amount)
                        }
                        Subscription::Update(update) => {
                            tracing::info!(?update, "registered plugin update subscription");
                            listen_types.push(update);
                            continue;
                        }
                        _ => continue,
                    };

                    let duration = timer_duration(unit, amount)?;

                    streams.insert(
                        index,
                        tokio_stream::wrappers::IntervalStream::new(interval(duration)),
                    );
                }

                let event_stream = tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(
                    move |result| {
                        future::ready(classify_actor_input(result, nickname, &listen_types))
                    },
                );

                let timer_stream = streams.map(|(_, admitted_at)| ActorInput::Update {
                    event: Update::Time(0),
                    admitted_at,
                    continuity_required: false,
                });
                let mut merged_stream = tokio_stream::StreamExt::merge(timer_stream, event_stream);
                let mut next_event_id = 1_u64;

                while let Some(input) = merged_stream.next().await {
                    let (event, admitted_at, continuity_required) = match input {
                        ActorInput::Update {
                            event,
                            admitted_at,
                            continuity_required,
                        } => (event, admitted_at, continuity_required),
                        ActorInput::ContinuityLost(skipped) => {
                            return Err(miette!(
                                "plugin {nickname} lost {skipped} subscribed updates and was disabled rather than continuing with incomplete state"
                            ));
                        }
                    };

                    let update_deadline = admitted_at + DEFAULT_GUEST_CALL_TIMEOUT;
                    let update = {
                        let mut store_lock = match tokio::time::timeout_at(
                            update_deadline,
                            store.enter(),
                        )
                        .await
                        {
                            Ok(store) => store,
                            Err(_) if continuity_required => {
                                return Err(miette!(
                                    "plugin {nickname} update expired in its queue and the subscribed actor was disabled"
                                ));
                            }
                            Err(_) => {
                                tracing::warn!(
                                    plugin = nickname,
                                    "skipping timer update that expired in the actor queue"
                                );
                                continue;
                            }
                        };
                        let remaining = update_deadline
                            .saturating_duration_since(tokio::time::Instant::now());
                        if remaining.is_zero() {
                            if continuity_required {
                                return Err(miette!(
                                    "plugin {nickname} update expired in its queue and the subscribed actor was disabled"
                                ));
                            }
                            tracing::warn!(
                                plugin = nickname,
                                "skipping timer update that expired in the actor queue"
                            );
                            continue;
                        }
                        store_lock.set_epoch_deadline(epoch_ticks(remaining));
                        let indices = crate::runtime::bindings::exports::litehouse::plugin::plugin::GuestIndices::new(
                            &host.instance_pre(&mut store_lock),
                        )
                        .map_err(|error| miette!("failed to load plugin export indices: {error:?}"))?;
                        let guest = indices
                            .load(&mut store_lock, &host)
                            .map_err(|error| miette!("failed to load plugin exports: {error:?}"))?;
                        let runner = guest.runner();
                        let event_id = next_event_id;
                        next_event_id = next_event_id
                            .checked_add(1)
                            .ok_or_else(|| miette!("event ID overflow for {nickname}"))?;
                        tokio::time::timeout_at(
                            update_deadline,
                            store_lock.as_context_mut().run_concurrent(async |accessor| {
                                runner
                                    .call_update(
                                        accessor,
                                        *plugin,
                                        vec![Event {
                                            id: event_id,
                                            timestamp: timestamp_millis(),
                                            inner: event,
                                        }],
                                    )
                                    .await
                            }),
                        )
                        .await
                    };

                    let generation_failed = match update {
                        Err(_) => {
                            tracing::error!(
                                plugin = nickname,
                                "plugin update exceeded its deadline"
                            );
                            true
                        }
                        Ok(Ok(Ok(Ok(_)))) => false,
                        Ok(Ok(Ok(Err(error)))) => {
                            tracing::error!(plugin = nickname, error, "plugin rejected update");
                            false
                        }
                        Ok(Ok(Err(error))) | Ok(Err(error)) => {
                            if let Some(trap) = error.downcast_ref::<Trap>() {
                                tracing::error!(plugin = nickname, %trap, "plugin trapped");
                                true
                            } else {
                                tracing::error!(plugin = nickname, ?error, "plugin update failed");
                                false
                            }
                        }
                    };

                    if generation_failed {
                        let replacement = restart_plugin(
                            &store_strategy,
                            &linker,
                            &component,
                            nickname,
                            instance,
                        )
                        .await?;
                        store = replacement.0;
                        host = replacement.1;
                        plugin = replacement.2;
                        tracing::info!(plugin = nickname, "plugin restarted");
                    }
                }

                    Ok(())
                })
                .await
            }
            .instrument(tracing::info_span!("plugin", nickname = nickname))
        });

    tracing::info!("running litehouse");

    tokio::select! {
        _ = server_fut => Ok(()),
        result = futures::future::try_join_all(actors) => result.map(|_| ()),
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("interrupt received, exiting");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActorInput, classify_actor_input, contain_actor, timer_duration,
        validate_runtime_configuration,
    };
    use crate::runtime::bindings::{
        exports::litehouse::plugin::plugin::{TimeUnit, UpdateSubscription},
        litehouse::plugin::update::Update,
    };
    use tokio_stream::wrappers::errors::BroadcastStreamRecvError;

    #[test]
    fn lagged_broadcast_becomes_explicit_continuity_loss() {
        let input = classify_actor_input(
            Err(BroadcastStreamRecvError::Lagged(7)),
            "consumer",
            &[UpdateSubscription::OnOff],
        );

        assert!(matches!(input, Some(ActorInput::ContinuityLost(7))));
    }

    #[test]
    fn lag_without_update_subscriptions_is_ignored() {
        let input =
            classify_actor_input(Err(BroadcastStreamRecvError::Lagged(7)), "timer-only", &[]);

        assert!(input.is_none());
    }

    #[test]
    fn subscribed_update_is_forwarded_without_changing_its_payload() {
        let admitted_at = tokio::time::Instant::now();
        let input = classify_actor_input(
            Ok(("provider".into(), Update::OnOff(true), admitted_at)),
            "consumer",
            &[UpdateSubscription::OnOff],
        );

        assert!(matches!(
            input,
            Some(ActorInput::Update {
                event: Update::OnOff(true),
                admitted_at: actual,
                continuity_required: true,
            }) if actual == admitted_at
        ));
    }

    #[tokio::test]
    async fn one_actor_failure_does_not_finish_the_runtime_actor_set() {
        let contained = contain_actor("fixture", async { Err(miette::miette!("failed")) });

        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), contained)
                .await
                .is_err()
        );
    }

    #[test]
    fn invalid_timer_period_returns_an_error_instead_of_panicking() {
        assert!(timer_duration(TimeUnit::Second, 0).is_err());
        assert!(timer_duration(TimeUnit::Year, u64::MAX).is_err());
    }

    #[test]
    fn per_instance_capabilities_with_shared_stores_are_rejected_before_startup() {
        let config: litehouse_config::LitehouseConfig = serde_json::from_value(serde_json::json!({
            "$schema": "./schema.json",
            "plugins": {
                "fixture": {
                    "plugin": "noop",
                    "capabilities": ["http-client:https://example.com"],
                    "config": null
                }
            },
            "engine": { "sandbox_strategy": "global" }
        }))
        .unwrap();

        assert!(validate_runtime_configuration(&config).is_err());
    }

    #[test]
    fn duplicate_instance_ids_are_rejected_before_startup() {
        let config: litehouse_config::LitehouseConfig = serde_json::from_value(serde_json::json!({
            "$schema": "./schema.json",
            "plugins": {
                "first": {
                    "plugin": "noop",
                    "instance_id": "00112233-4455-6677-8899-aabbccddeeff",
                    "config": null
                },
                "second": {
                    "plugin": "noop",
                    "instance_id": "00112233-4455-6677-8899-aabbccddeeff",
                    "config": null
                }
            }
        }))
        .unwrap();

        assert!(validate_runtime_configuration(&config).is_err());
    }
}
