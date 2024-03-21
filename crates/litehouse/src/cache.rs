use eyre::Result;
use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
};
use wasmtime::CacheStore;

#[derive(Debug, Default)]
pub struct ModuleCache(std::sync::Mutex<ModuleCacheInner>);

impl ModuleCache {
    fn cache_path() -> PathBuf {
        litehouse_config::directories()
            .as_ref()
            .map(|d| d.cache_dir())
            .unwrap_or_else(|| Path::new(""))
            .join("module.bin.lz4")
    }

    pub async fn load() -> Result<Option<Self>> {
        let path = Self::cache_path();
        tracing::debug!("loading bytecode cache from {}", path.display());
        let data = tokio::fs::read(path).await;
        match data {
            Ok(data) => {
                let decompressed = lz4_flex::decompress_size_prepended(&data).unwrap();
                let inner = bitcode::decode::<ModuleCacheInner>(&decompressed).unwrap();
                Ok(Some(ModuleCache(std::sync::Mutex::new(inner))))
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    pub async fn drain(&self) -> Result<()> {
        let data = {
            let mut map = self.0.lock().unwrap();
            let data = bitcode::encode(&*map);
            *map = Default::default();
            data
        };

        let compressed = lz4_flex::compress_prepend_size(&data);
        tokio::fs::write(Self::cache_path(), compressed).await?;
        Ok(())
    }
}

#[derive(Debug, Default, bitcode::Encode, bitcode::Decode)]
struct ModuleCacheInner(HashMap<Vec<u8>, Vec<u8>>);

impl CacheStore for ModuleCache {
    fn get(&self, key: &[u8]) -> Option<Cow<[u8]>> {
        let map = self.0.lock().unwrap();
        map.0.get(key).map(|v| Cow::Owned(v.clone()))
    }

    fn insert(&self, key: &[u8], value: Vec<u8>) -> bool {
        self.0
            .lock()
            .unwrap()
            .0
            .insert(key.to_vec(), value)
            .is_none()
    }
}
