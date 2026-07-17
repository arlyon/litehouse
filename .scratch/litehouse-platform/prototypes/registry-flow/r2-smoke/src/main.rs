use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::{Client, config::Region, primitives::ByteStream};
use rand::RngCore;
use registry_flow_core::{
    MemoryStore, Object, ObjectStore, Publisher, PutCondition, ROOT_KEY, RegistryError, Result,
    download_verified, fixture_release, query_exact, sha256,
};
use serde::Deserialize;
use std::{env, path::PathBuf, sync::Arc};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Target {
    endpoint: String,
    bucket: String,
    region: String,
    access_key_id: String,
    secret_access_key: String,
}

#[derive(Clone)]
struct R2Store {
    client: Client,
    bucket: String,
    prefix: String,
}

impl R2Store {
    fn key(&self, key: &str) -> String {
        format!("{}{key}", self.prefix)
    }

    async fn conditional_not_modified(&self, key: &str, etag: &str) -> Result<bool> {
        let result = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(self.key(key))
            .if_none_match(etag)
            .send()
            .await;
        match result {
            Ok(_) => Ok(false),
            Err(error)
                if error
                    .raw_response()
                    .is_some_and(|response| response.status().as_u16() == 304) =>
            {
                Ok(true)
            }
            Err(_) => Err(RegistryError::Io),
        }
    }

    async fn cleanup(&self) -> Result<()> {
        let mut continuation = None;
        loop {
            let mut request = self
                .client
                .list_objects_v2()
                .bucket(&self.bucket)
                .prefix(&self.prefix);
            if let Some(token) = continuation.take() {
                request = request.continuation_token(token);
            }
            let output = request.send().await.map_err(|_| RegistryError::Io)?;
            for object in output.contents() {
                if let Some(key) = object.key() {
                    self.client
                        .delete_object()
                        .bucket(&self.bucket)
                        .key(key)
                        .send()
                        .await
                        .map_err(|_| RegistryError::Io)?;
                }
            }
            if !output.is_truncated().unwrap_or(false) {
                break;
            }
            continuation = output.next_continuation_token().map(str::to_owned);
        }
        Ok(())
    }
}

#[async_trait]
impl ObjectStore for R2Store {
    async fn get(&self, key: &str) -> Result<Object> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(self.key(key))
            .send()
            .await
            .map_err(|error| {
                if error
                    .raw_response()
                    .is_some_and(|response| response.status().as_u16() == 404)
                {
                    RegistryError::NotFound
                } else {
                    RegistryError::Io
                }
            })?;
        let etag = output.e_tag().ok_or(RegistryError::Malformed)?.to_owned();
        let bytes = output
            .body
            .collect()
            .await
            .map_err(|_| RegistryError::Io)?
            .into_bytes()
            .to_vec();
        Ok(Object { bytes, etag })
    }

    async fn get_range(&self, key: &str, start: u64, end_exclusive: u64) -> Result<Vec<u8>> {
        if end_exclusive <= start {
            return Err(RegistryError::Malformed);
        }
        let range = format!("bytes={start}-{}", end_exclusive - 1);
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(self.key(key))
            .range(range)
            .send()
            .await
            .map_err(|_| RegistryError::Io)?;
        output
            .body
            .collect()
            .await
            .map(|body| body.into_bytes().to_vec())
            .map_err(|_| RegistryError::Io)
    }

    async fn put(&self, key: &str, bytes: Vec<u8>, condition: PutCondition) -> Result<String> {
        let mut request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(self.key(key))
            .body(ByteStream::from(bytes));
        request = match condition {
            PutCondition::Any => request,
            PutCondition::CreateOnly => request.if_none_match("*"),
            PutCondition::Match(etag) => request.if_match(etag),
        };
        let output = request.send().await.map_err(|error| {
            if error
                .raw_response()
                .is_some_and(|response| response.status().as_u16() == 412)
            {
                RegistryError::Precondition
            } else {
                RegistryError::Io
            }
        })?;
        output
            .e_tag()
            .map(str::to_owned)
            .ok_or(RegistryError::Malformed)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(self.key(key))
            .send()
            .await
            .map_err(|_| RegistryError::Io)?;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let mut target = match read_target() {
        Ok(target) => target,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    };
    let store = build_store(&target);
    target.zeroize();

    let outcome = run_smoke(store.clone()).await;
    let cleanup = store.cleanup().await;
    match (outcome, cleanup) {
        (Ok(()), Ok(())) => println!("R2 registry smoke passed; disposable object prefix removed"),
        (Err(error), Ok(())) => {
            eprintln!("R2 registry smoke failed safely: {error}");
            std::process::exit(1);
        }
        (_, Err(_)) => {
            eprintln!("R2 registry cleanup failed; protected bucket name remains in runtime file");
            std::process::exit(1);
        }
    }
}

fn build_store(target: &Target) -> R2Store {
    let credentials = Credentials::new(
        target.access_key_id.clone(),
        target.secret_access_key.clone(),
        None,
        None,
        "litehouse-registry-proof",
    );
    let config = aws_sdk_s3::Config::builder()
        .behavior_version_latest()
        .credentials_provider(credentials)
        .region(Region::new(target.region.clone()))
        .endpoint_url(target.endpoint.clone())
        .force_path_style(true)
        .build();
    let mut random = [0_u8; 12];
    rand::rng().fill_bytes(&mut random);
    R2Store {
        client: Client::from_conf(config),
        bucket: target.bucket.clone(),
        prefix: format!("litehouse-registry-proof/{}/", hex::encode(random)),
    }
}

async fn run_smoke(store: R2Store) -> Result<()> {
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
    let publisher = Arc::new(Publisher::new(store.clone(), signing_key.verifying_key()));
    println!("R2 stage: initialize root and empty shards");
    publisher.initialize().await?;

    println!("R2 stage: conditional root operations");
    let root = store.get(ROOT_KEY).await?;
    if !store.conditional_not_modified(ROOT_KEY, &root.etag).await? {
        return Err(RegistryError::Precondition);
    }
    if !matches!(
        store
            .put(
                ROOT_KEY,
                b"wrong".to_vec(),
                PutCondition::Match("\"wrong-etag\"".to_owned())
            )
            .await,
        Err(RegistryError::Precondition)
    ) {
        return Err(RegistryError::Precondition);
    }

    println!("R2 stage: concurrent object-first publication");
    let (first, first_path, first_blob) = fixture_release(
        "litehouse:on-off",
        "1.0.0",
        b"first-r2-component-fixture",
        &["tcp:node-s"],
        &signing_key,
    )?;
    let (second, second_path, second_blob) = fixture_release(
        "litehouse:measurement",
        "1.0.0",
        b"second-r2-component-fixture",
        &[],
        &signing_key,
    )?;
    let left = tokio::spawn({
        let publisher = publisher.clone();
        let task_path = first_path.clone();
        let task_blob = first_blob.clone();
        async move { publisher.publish(first, &task_path, &task_blob).await }
    });
    let right = tokio::spawn({
        let publisher = publisher.clone();
        async move { publisher.publish(second, &second_path, &second_blob).await }
    });
    left.await.map_err(|_| RegistryError::Io)??;
    right.await.map_err(|_| RegistryError::Io)??;

    println!("R2 stage: verified query and resumable download");
    let payload = query_exact(
        &store,
        "litehouse:on-off",
        "1.0.0",
        &signing_key.verifying_key(),
    )
    .await?;
    let digest: [u8; 32] = payload
        .primary
        .sha256
        .as_slice()
        .try_into()
        .map_err(|_| RegistryError::Malformed)?;
    let directory = tempfile::tempdir().map_err(|_| RegistryError::Io)?;
    let destination = directory.path().join("component.wasm");
    let partial = download_verified(
        &store,
        &first_path,
        digest,
        payload.primary.length,
        &destination,
        5,
        Some(10),
    )
    .await;
    if !matches!(partial, Err(RegistryError::Interrupted)) {
        return Err(RegistryError::Interrupted);
    }
    download_verified(
        &store,
        &first_path,
        digest,
        payload.primary.length,
        &destination,
        5,
        None,
    )
    .await?;

    println!("R2 stage: create-only conflict and byte range");
    let create_only_key = "fixtures/create-only";
    store
        .put(create_only_key, b"one".to_vec(), PutCondition::CreateOnly)
        .await?;
    if !matches!(
        store
            .put(create_only_key, b"two".to_vec(), PutCondition::CreateOnly)
            .await,
        Err(RegistryError::Precondition)
    ) {
        return Err(RegistryError::Precondition);
    }
    let range = store.get_range(&first_path, 2, 9).await?;
    if range != first_blob[2..9]
        || sha256(
            &tokio::fs::read(destination)
                .await
                .map_err(|_| RegistryError::Io)?,
        ) != digest
    {
        return Err(RegistryError::Digest);
    }

    let _offline_model = MemoryStore::default();
    Ok(())
}

fn read_target() -> std::result::Result<Target, &'static str> {
    use std::os::unix::fs::MetadataExt;
    let path = PathBuf::from(env::var_os("XDG_RUNTIME_DIR").ok_or("XDG_RUNTIME_DIR is not set")?)
        .join("litehouse-registry-proof/targets.json");
    let metadata = std::fs::symlink_metadata(&path).map_err(|_| "R2 target file is unavailable")?;
    if metadata.file_type().is_symlink()
        || !metadata.file_type().is_file()
        || metadata.mode() & 0o777 != 0o600
    {
        return Err("R2 target file must be a regular non-symlink with mode 0600");
    }
    let text = std::fs::read_to_string(path).map_err(|_| "R2 target file could not be read")?;
    serde_json::from_str(&text).map_err(|_| "R2 target file is invalid")
}
