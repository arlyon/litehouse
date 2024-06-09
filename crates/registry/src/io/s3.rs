use std::{
    collections::{hash_map::Entry, HashMap},
    io::ErrorKind,
    path::Path,
    sync::{Arc, Mutex},
};

use flatbuffers::{Follow, Verifiable};
use opendal::{services::S3, Builder, Operator};
use opendal_fs_cache::CacheLayer;
use stable_deref_trait::StableDeref;
use tokio::{fs::OpenOptions, sync::RwLock};

use crate::{naming::NamingScheme, partition::Partition};

use super::{Index, IndexIOScheme, PartitionIOScheme};

pub struct MMapS3IoScheme<'a, T, N>
where
    T: Follow<'a> + Verifiable + 'a,
    N: NamingScheme,
{
    /// SAFETY: you must not remove any partitions from this map
    mmaps: Mutex<HashMap<usize, Box<RwLock<Partition<'a, T>>>>>,
    index: Mutex<Option<Index>>,
    client: opendal::Operator,
    naming_scheme: N,
}

/// Extend the lifetime of stable heap allocated data.
///
/// # Safety
///
/// Ensure that the data being extended is not dropped before the lifetime of the returned reference.
/// In our case, these lifetimes are stored in the map above, with a guarantee that no keys are removed.
pub unsafe fn extend_lifetime<'a, T: StableDeref>(ptr: &T) -> &'a T::Target {
    &*(&**ptr as *const T::Target)
}

impl<'a, T, N> PartitionIOScheme<'a, T> for MMapS3IoScheme<'a, T, N>
where
    T: Follow<'a> + Verifiable,
    N: NamingScheme,
{
    async fn open(&'a self, id: usize) -> &'a RwLock<Partition<'a, T>> {
        let mut mmaps = self.mmaps.lock().unwrap();
        match mmaps.entry(id) {
            Entry::Occupied(e) => unsafe {
                tracing::debug!("partition {} already exists", id);
                extend_lifetime(e.get())
            },
            Entry::Vacant(v) => unsafe {
                tracing::debug!("loading partition {}", id);
                extend_lifetime(v.insert(Box::new(RwLock::new(self.load(id).await))))
            },
        }
    }
}

impl<'a, T, N: NamingScheme> IndexIOScheme<'a, T> for MMapS3IoScheme<'a, T, N>
where
    T: Follow<'a> + Verifiable,
    N: NamingScheme,
{
    async fn open(&self) -> Index {
        let mut index = self.index.lock().unwrap();
        match index.as_ref() {
            Some(i) => i.to_owned(),
            None => {
                let mut index_new = self
                    .client
                    .reader(&self.naming_scheme.index())
                    .await
                    .unwrap();
                let mut file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(false)
                    .open(&self.naming_scheme.index())
                    .await
                    .unwrap();
                _ = tokio::io::copy(&mut index_new, &mut file).await;
                let index_new = Index {
                    mmap: Arc::new(Mutex::new(unsafe { memmap2::Mmap::map(&file).unwrap() })),
                };
                *index = Some(index_new.clone());
                index_new
            }
        }
    }
}

impl<'a, T, N> MMapS3IoScheme<'a, T, N>
where
    T: Follow<'a> + Verifiable,
    N: NamingScheme,
{
    pub fn new(naming_scheme: N, cache: Option<&Path>) -> Self {
        // Create s3 backend builder.
        let mut builder = S3::default();
        builder.root("v1");
        builder.bucket("litehouse");
        builder.region("us-east-1");
        builder.endpoint("https://ams1.vultrobjects.com");
        builder.allow_anonymous();
        builder.customed_credential_load(Box::new(AnonymousCredentialLoad));

        let op = Operator::new(builder).unwrap();

        let op = if let Some(local_cache) = cache {
            let mut fs_cache = opendal::services::Fs::default();
            fs_cache.root(local_cache.to_str().unwrap());
            let fs_cache = fs_cache.build().unwrap();
            op.layer(CacheLayer::new(fs_cache)).finish()
        } else {
            op.finish()
        };

        Self {
            index: Default::default(),
            mmaps: Default::default(),
            client: op,
            naming_scheme,
        }
    }

    async fn load(&'a self, id: usize) -> Partition<'a, T> {
        // attempt to open file and download if it doesn't exist
        let path = self.naming_scheme.name(id);
        let file = OpenOptions::new().read(true).open(&path).await;
        if let Err(err) = file
            && err.kind() == ErrorKind::NotFound
        {
            tracing::debug!("file not found at {path}, downloading");
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&path)
                .await
                .unwrap();
            let mut reader = self.client.reader(&path).await.unwrap();
            let _ = tokio::io::copy(&mut reader, &mut file).await;
        }

        Partition::new(path).await
    }
}

pub struct AnonymousCredentialLoad;

#[async_trait::async_trait]
impl reqsign::AwsCredentialLoad for AnonymousCredentialLoad {
    async fn load_credential(
        &self,
        _: reqwest::Client,
    ) -> anyhow::Result<Option<reqsign::AwsCredential>> {
        Ok(None)
    }
}
