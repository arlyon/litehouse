use litehouse_config::Import;
use miette::{Context, IntoDiagnostic, Result};
use opendal_fs_cache::CacheLayer;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use opendal::{services::S3, Builder, Entry, Operator};

pub struct Registry<U, D> {
    op: Operator,
    name: String,
    _upload: U,
    download: D,
}

impl Registry<(), ()> {
    pub fn build(name: String) -> RegistryBuilder<(), ()> {
        RegistryBuilder::new(name)
    }
}

pub struct RegistryBuilder<U, D> {
    name: String,
    upload: U,
    download: D,
}

impl RegistryBuilder<(), ()> {
    pub fn new(name: String) -> Self {
        Self {
            name,
            upload: (),
            download: (),
        }
    }
}

impl<U, D> RegistryBuilder<U, D> {
    pub fn with_download(
        self,
        path: PathBuf,
        cache: Option<PathBuf>,
    ) -> RegistryBuilder<U, Download> {
        RegistryBuilder {
            upload: self.upload,
            name: self.name,
            download: Download(path, cache),
        }
    }
    pub fn with_upload(self, access_key: String, secret_key: String) -> RegistryBuilder<Upload, D> {
        RegistryBuilder {
            download: self.download,
            name: self.name,
            upload: Upload(access_key, secret_key),
        }
    }
}

pub trait GetCache {
    fn get_cache(&self) -> Option<&Path> {
        None
    }
}

impl GetCache for () {}
impl GetCache for Download {
    fn get_cache(&self) -> Option<&Path> {
        self.1.as_deref()
    }
}

pub trait GetCreds {
    fn get_creds(&self) -> Option<(&str, &str)> {
        None
    }
}

impl GetCreds for () {}
impl GetCreds for Upload {
    fn get_creds(&self) -> Option<(&str, &str)> {
        Some((&self.0, &self.1))
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

impl<U: GetCreds, D: GetCache> RegistryBuilder<U, D> {
    pub async fn build(self) -> Result<Registry<U, D>> {
        // Create s3 backend builder.
        let mut builder = S3::default();
        builder.root("v1");
        builder.bucket("litehouse");
        builder.region("us-east-1");
        builder.endpoint("https://ams1.vultrobjects.com");
        builder.allow_anonymous();

        if let Some((access_key_id, secret_key)) = self.upload.get_creds() {
            builder.access_key_id(access_key_id);
            builder.secret_access_key(secret_key);
        } else {
            builder.customed_credential_load(Box::new(AnonymousCredentialLoad));
        }

        let op = Operator::new(builder).unwrap();

        let op = if let Some(local_cache) = self.download.get_cache() {
            let mut fs_cache = opendal::services::Fs::default();
            fs_cache.root(local_cache.to_str().unwrap());
            let fs_cache = fs_cache.build().unwrap();
            op.layer(CacheLayer::new(fs_cache)).finish()
        } else {
            op.finish()
        };

        let r = Registry {
            op,
            name: self.name,
            download: self.download,
            _upload: self.upload,
        };

        r.check()
            .await
            .wrap_err("unable to connect to registry")
            .map(|_| r)
    }
}

pub struct Download(PathBuf, Option<PathBuf>);
pub struct Upload(String, String);

impl<U, D> Registry<U, D> {
    async fn check(&self) -> Result<()> {
        self.op
            .check()
            .await
            .into_diagnostic()
            .wrap_err("auth check failed")
    }

    pub async fn list(&self, prefix: Option<&Import>) -> impl Iterator<Item = (Import, Entry)> {
        self.op
            .list(prefix.map(|p| p.plugin.as_str()).unwrap_or_default())
            .await
            .unwrap()
            .into_iter()
            .filter_map(|e| {
                let name = e.name().strip_suffix(".wasm")?;
                let import = name.parse::<Import>().ok()?;
                Some((import, e))
            })
    }
}

impl<U> Registry<U, Download> {
    pub async fn download_package(&self, import: &Import) -> bool {
        if let Some(registry) = &import.registry {
            if self.name.ne(registry) {
                return false;
            }
        }

        // if we have the version, just try to nab it
        if import.version.is_some() {
            return self.download_file(import, None).await.is_some();
        }

        // list all files using the package name as a prefix
        let files = self.list(Some(import)).await;

        // otherwise select the latest version
        let selected = files.max_by(|a, b| a.0.cmp(&b.0));

        let Some((_, entry)) = selected else {
            println!("no matches found for {:?}", import.plugin);
            return false;
        };

        self.download_file(import, Some(entry.path()))
            .await
            .is_some()
    }

    pub async fn download_file(&self, import: &Import, path: Option<&str>) -> Option<u64> {
        let path = path
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(import.file_name()));

        // mk_dir_all on the path
        tokio::fs::create_dir_all(&self.download.0).await.unwrap();

        let plugin_path = self.download.0.join(&*path);

        let mut reader = self.op.reader(&path).await.unwrap();
        let mut file = tokio::fs::File::create(&plugin_path).await.unwrap();
        let ok = import.copy(&mut reader, &mut file).await;
        if ok.is_none() {
            panic!("sha does not match");
        }
        ok
    }
}

impl<D> Registry<Upload, D> {
    pub async fn publish(&self, plugin: &Import, path: &Path) -> bool {
        let mut writer = self
            .op
            .writer_with(&plugin.file_name())
            .buffer(8 * 1024 * 1024)
            .await
            .unwrap();
        let mut file = tokio::fs::File::open(&path).await.unwrap();
        let _ = tokio::io::copy(&mut file, &mut writer).await.unwrap();
        writer.close().await.unwrap();
        true
    }
}
