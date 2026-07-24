//! Module cache for storing compiled WebAssembly modules.

use miette::{IntoDiagnostic, Result};
use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
use wasmtime::CacheStore;

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(1);

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
        Self::load_from(&Self::cache_path()).await
    }

    async fn load_from(path: &Path) -> Result<Option<Self>> {
        tracing::debug!(path = %path.display(), "loading bytecode cache");
        let data = match tokio::fs::read(path).await {
            Ok(data) => data,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).into_diagnostic(),
        };

        let decompressed = match lz4_flex::decompress_size_prepended(&data) {
            Ok(decompressed) => decompressed,
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    ?error,
                    "ignoring corrupt bytecode cache"
                );
                return Ok(None);
            }
        };
        let inner = match bitcode::decode::<ModuleCacheInner>(&decompressed) {
            Ok(inner) => inner,
            Err(error) => {
                tracing::warn!(
                    path = %path.display(),
                    ?error,
                    "ignoring incompatible bytecode cache"
                );
                return Ok(None);
            }
        };

        Ok(Some(Self(std::sync::Mutex::new(inner))))
    }

    pub async fn drain(&self) -> Result<()> {
        self.drain_to(&Self::cache_path()).await
    }

    async fn drain_to(&self, path: &Path) -> Result<()> {
        let data = {
            let mut map = self.0.lock().unwrap_or_else(|error| error.into_inner());
            let data = bitcode::encode(&*map);
            *map = Default::default();
            data
        };

        let compressed = lz4_flex::compress_prepend_size(&data);
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent).await.into_diagnostic()?;
        }

        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("module.bin.lz4");
        let temporary = path.with_file_name(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
        ));
        if let Err(error) = tokio::fs::write(&temporary, compressed).await {
            let _ = tokio::fs::remove_file(&temporary).await;
            return Err(error).into_diagnostic();
        }
        if let Err(error) = tokio::fs::rename(&temporary, path).await {
            let _ = tokio::fs::remove_file(&temporary).await;
            return Err(error).into_diagnostic();
        }
        Ok(())
    }
}

#[derive(Debug, Default, bitcode::Encode, bitcode::Decode)]
struct ModuleCacheInner(HashMap<Vec<u8>, Vec<u8>>);

impl CacheStore for ModuleCache {
    fn get(&self, key: &[u8]) -> Option<Cow<'_, [u8]>> {
        let map = self.0.lock().unwrap_or_else(|error| error.into_inner());
        let found = map.0.get(key).map(|value| Cow::Owned(value.clone()));
        tracing::trace!(found = found.is_some(), "cache lookup {:x?}", key);
        found
    }

    fn insert(&self, key: &[u8], value: Vec<u8>) -> bool {
        self.0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .0
            .insert(key.to_vec(), value)
            .is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::ModuleCache;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    use wasmtime::CacheStore;

    static NEXT_PATH: AtomicU64 = AtomicU64::new(1);

    fn temporary_path(test: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "litehouse-cache-{test}-{}-{}.lz4",
            std::process::id(),
            NEXT_PATH.fetch_add(1, Ordering::Relaxed)
        ))
    }

    #[tokio::test]
    async fn corrupt_cache_is_a_miss() {
        let path = temporary_path("corrupt");
        tokio::fs::write(&path, b"not an lz4 cache").await.unwrap();

        let loaded = ModuleCache::load_from(&path).await.unwrap();

        assert!(loaded.is_none());
        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn incompatible_cache_is_a_miss() {
        let path = temporary_path("incompatible");
        let data = lz4_flex::compress_prepend_size(b"not bitcode");
        tokio::fs::write(&path, data).await.unwrap();

        let loaded = ModuleCache::load_from(&path).await.unwrap();

        assert!(loaded.is_none());
        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn corrupt_cache_can_be_rebuilt_in_place() {
        let path = temporary_path("rebuild");
        tokio::fs::write(&path, b"corrupt").await.unwrap();
        assert!(ModuleCache::load_from(&path).await.unwrap().is_none());

        let cache = ModuleCache::default();
        assert!(cache.insert(b"component", b"rebuilt".to_vec()));
        cache.drain_to(&path).await.unwrap();
        let loaded = ModuleCache::load_from(&path).await.unwrap().unwrap();

        assert_eq!(loaded.get(b"component").as_deref(), Some(&b"rebuilt"[..]));
        let _ = tokio::fs::remove_file(path).await;
    }

    #[tokio::test]
    async fn drained_cache_roundtrips() {
        let path = temporary_path("roundtrip");
        let cache = ModuleCache::default();
        assert!(cache.insert(b"component", b"compiled".to_vec()));

        cache.drain_to(&path).await.unwrap();
        let loaded = ModuleCache::load_from(&path).await.unwrap().unwrap();

        assert_eq!(loaded.get(b"component").as_deref(), Some(&b"compiled"[..]));
        let _ = tokio::fs::remove_file(path).await;
    }
}
