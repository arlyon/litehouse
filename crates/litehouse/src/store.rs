//! Store utilities for plugin runners

use miette::{Result, miette};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};

use wasmtime::{AsContext, AsContextMut, Engine, Store};

use crate::runtime::{PluginRunner, PluginRunnerFactory};

type MutexStore<T> = Arc<Mutex<Store<PluginRunner<T>>>>;

/// A strategy for handling stores between plugin runners.
/// A single runner can instantiate multiple plugins with
/// different configurations, and this enum allows for
/// different strategies for handling the stores for each
/// plugin.
pub enum StoreStrategy<T: 'static> {
    /// All plugins live in the same store. All plugins share the
    /// same memory space, and only one plugin can run at a time.
    Global(MutexStore<T>, PluginRunnerFactory<T>),
    /// All instances of a specific plugin live in the same store.
    /// Different types of plugin run at the same time, but instantiations
    /// of the same type run sequentially.
    PerPlugin(
        Engine,
        PluginRunnerFactory<T>,
        std::sync::Mutex<HashMap<String, MutexStore<T>>>,
    ),
    /// Each plugin instance has its own store, allowing full parallelism.
    PerInstance(Engine, PluginRunnerFactory<T>),
}

impl<T: Clone> StoreStrategy<T> {
    pub fn global(engine: Engine, factory: PluginRunnerFactory<T>) -> Self {
        Self::Global(
            Arc::new(Mutex::new(Store::new(&engine, factory.create()))),
            factory,
        )
    }

    pub fn per_instance(engine: Engine, factory: PluginRunnerFactory<T>) -> Self {
        Self::PerInstance(engine, factory)
    }

    #[allow(dead_code)]
    pub fn per_plugin(engine: Engine, factory: PluginRunnerFactory<T>) -> Self {
        Self::PerPlugin(engine, factory, std::sync::Mutex::new(HashMap::new()))
    }

    pub fn get(
        &self,
        key: &str,
        instance_capabilities: &[litehouse_config::Capability],
    ) -> Result<StoreRef<T>> {
        match self {
            Self::Global(store, _) => {
                reject_instance_capabilities(instance_capabilities)?;
                Ok(StoreRef::Shared(store.to_owned()))
            }
            Self::PerInstance(engine, factory) => Ok(StoreRef::Exclusive(Store::new(
                engine,
                factory.create_with(instance_capabilities),
            ))),
            Self::PerPlugin(engine, factory, stores) => {
                reject_instance_capabilities(instance_capabilities)?;
                let mut stores = stores.lock().unwrap_or_else(|error| error.into_inner());
                let store = stores
                    .entry(key.to_owned())
                    .or_insert_with(|| Arc::new(Mutex::new(Store::new(engine, factory.create()))));

                Ok(StoreRef::Shared(store.to_owned()))
            }
        }
    }

    /// Use the factory to create a new instance of the plugin runner and replace the existing one.
    pub async fn reset(
        &self,
        _key: &str,
        instance_capabilities: &[litehouse_config::Capability],
    ) -> Result<StoreRef<T>> {
        match self {
            Self::Global(..) | Self::PerPlugin(..) => Err(miette!(
                "automatic instance restart requires engine.sandbox_strategy = instance; the affected shared-store actor was disabled without replacing sibling state"
            )),
            Self::PerInstance(engine, factory) => Ok(StoreRef::Exclusive(Store::new(
                engine,
                factory.create_with(instance_capabilities),
            ))),
        }
    }
}

fn reject_instance_capabilities(
    instance_capabilities: &[litehouse_config::Capability],
) -> Result<()> {
    if instance_capabilities.is_empty() {
        Ok(())
    } else {
        Err(miette!(
            "per-instance capabilities require engine.sandbox_strategy = instance"
        ))
    }
}

/// A reference to a store, which, depending on whether multiple
/// plugins need to share it, can be either shared or exclusive.
pub enum StoreRef<T: 'static> {
    /// The store is shared, so it needs to be locked before use.
    Shared(Arc<Mutex<Store<PluginRunner<T>>>>),
    /// The store is not shared, so no synchronization is needed.
    Exclusive(Store<PluginRunner<T>>),
}

/// A lock on a store, which can be either locked or unlocked.
pub enum StoreLock<'a, T: 'static> {
    Locked(MutexGuard<'a, Store<PluginRunner<T>>>),
    Unlocked(&'a mut Store<PluginRunner<T>>),
}

impl<T> StoreRef<T> {
    /// Enter the store, returning a handle that can be used to access it.
    pub async fn enter(&mut self) -> StoreLock<'_, T> {
        match self {
            Self::Shared(store) => StoreLock::Locked(store.lock().await),
            Self::Exclusive(store) => StoreLock::Unlocked(store),
        }
    }
}

impl<T> StoreLock<'_, T> {
    pub fn set_epoch_deadline(&mut self, ticks_beyond_current: u64) {
        match self {
            Self::Locked(store) => store.set_epoch_deadline(ticks_beyond_current),
            Self::Unlocked(store) => store.set_epoch_deadline(ticks_beyond_current),
        }
    }
}

impl<'a, T> AsContext for StoreLock<'a, T> {
    type Data = PluginRunner<T>;

    fn as_context(&self) -> wasmtime::StoreContext<'_, Self::Data> {
        match self {
            Self::Locked(store) => store.as_context(),
            Self::Unlocked(store) => store.as_context(),
        }
    }
}

impl<'a, T> AsContextMut for StoreLock<'a, T> {
    fn as_context_mut(&mut self) -> wasmtime::StoreContextMut<'_, Self::Data> {
        match self {
            Self::Locked(store) => store.as_context_mut(),
            Self::Unlocked(store) => store.as_context_mut(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StoreStrategy;
    use crate::runtime::PluginRunnerFactory;
    use litehouse_config::Capability;

    #[tokio::test]
    async fn shared_store_restart_is_rejected_without_replacing_siblings() {
        let engine = wasmtime::Engine::default();
        let global =
            StoreStrategy::global(engine.clone(), PluginRunnerFactory::new((), Vec::new()));
        let per_plugin =
            StoreStrategy::per_plugin(engine, PluginRunnerFactory::new((), Vec::new()));

        assert!(global.reset("fixture", &[]).await.is_err());
        assert!(per_plugin.reset("fixture", &[]).await.is_err());
    }

    #[test]
    fn per_instance_capabilities_are_rejected_before_entering_a_shared_store() {
        let global = StoreStrategy::global(
            wasmtime::Engine::default(),
            PluginRunnerFactory::new((), Vec::new()),
        );
        let capability = Capability::HttpClient("https://example.com".into());

        assert!(global.get("fixture", &[capability]).is_err());
    }
}
