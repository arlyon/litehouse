//! Store utilities for plugin runners

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

    pub fn get(&self, key: &str) -> StoreRef<T> {
        match self {
            Self::Global(store, _) => StoreRef::Shared(store.to_owned()),
            Self::PerInstance(engine, factory) => {
                StoreRef::Exclusive(Store::new(engine, factory.create()))
            }
            Self::PerPlugin(engine, factory, stores) => {
                let mut stores = stores.lock().unwrap();
                let store = stores
                    .entry(key.to_owned())
                    .or_insert_with(|| Arc::new(Mutex::new(Store::new(engine, factory.create()))));

                StoreRef::Shared(store.to_owned())
            }
        }
    }

    /// Use the factor to create a new instance of the plugin runner and replace the existing one.
    pub async fn reset(&self, key: &str) -> StoreRef<T> {
        match self {
            Self::Global(store, factory) => {
                {
                    let mut store = store.lock().await;
                    *store = Store::new(store.engine(), factory.create());
                }
                StoreRef::Shared(store.to_owned())
            }
            Self::PerInstance(engine, factory) => {
                StoreRef::Exclusive(Store::new(engine, factory.create()))
            }
            Self::PerPlugin(engine, factory, stores) => {
                let mut stores = stores.lock().unwrap();
                let store = stores
                    .entry(key.to_owned())
                    .or_insert_with(|| Arc::new(Mutex::new(Store::new(engine, factory.create()))));

                {
                    let mut store = store.lock().await;
                    *store = Store::new(store.engine(), factory.create());
                }
                StoreRef::Shared(store.to_owned())
            }
        }
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
