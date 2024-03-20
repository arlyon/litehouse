use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, MutexGuard};

use wasmtime::{AsContext, AsContextMut, Engine, Store};

use crate::runtime::{PluginRunner, PluginRunnerFactory};

pub enum StoreStrategy<T> {
    Global(Arc<Mutex<Store<PluginRunner<T>>>>),
    #[allow(dead_code)]
    PerPlugin(
        Engine,
        PluginRunnerFactory<T>,
        std::sync::Mutex<HashMap<String, Arc<Mutex<Store<PluginRunner<T>>>>>>,
    ),
    PerInstance(Engine, PluginRunnerFactory<T>),
}

impl<T: Clone> StoreStrategy<T> {
    pub fn global(engine: Engine, factory: PluginRunnerFactory<T>) -> Self {
        Self::Global(Arc::new(Mutex::new(Store::new(&engine, factory.create()))))
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
            Self::Global(store) => StoreRef::Locked(store.to_owned()),
            Self::PerInstance(engine, factory) => {
                StoreRef::Unlocked(Store::new(engine, factory.create()))
            }
            Self::PerPlugin(engine, factory, stores) => {
                let mut stores = stores.lock().unwrap();
                let store = stores
                    .entry(key.to_owned())
                    .or_insert_with(|| Arc::new(Mutex::new(Store::new(engine, factory.create()))));

                StoreRef::Locked(store.to_owned())
            }
        }
    }
}

pub enum StoreRef<T> {
    Locked(Arc<Mutex<Store<PluginRunner<T>>>>),
    Unlocked(Store<PluginRunner<T>>),
}

pub enum StoreLock<'a, T> {
    Locked(MutexGuard<'a, Store<PluginRunner<T>>>),
    Unlocked(&'a mut Store<PluginRunner<T>>),
}

impl<T> StoreRef<T> {
    pub async fn enter(&mut self) -> StoreLock<T> {
        match self {
            Self::Locked(store) => StoreLock::Locked(store.lock().await),
            Self::Unlocked(store) => StoreLock::Unlocked(store),
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
