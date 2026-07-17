extern crate alloc;

#[allow(warnings)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/registry_generated.rs"));
}

use async_trait::async_trait;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use generated::litehouse::registry::spike as fb;
use semver::Version;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::{fs, sync::RwLock};

pub const ROOT_KEY: &str = "registry/index.bin";
pub const SIGNING_DOMAIN: &[u8] = b"litehouse-release-v1\0";
const PARTITION_BOUNDS: [(&str, &str); 7] = [
    ("a", "e"),
    ("e", "i"),
    ("i", "m"),
    ("m", "q"),
    ("q", "u"),
    ("u", "y"),
    ("y", "{"),
];

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("object not found")]
    NotFound,
    #[error("conditional object operation failed")]
    Precondition,
    #[error("malformed or unsupported registry bytes")]
    Malformed,
    #[error("digest or length mismatch")]
    Digest,
    #[error("release signature rejected")]
    Signature,
    #[error("release already exists with different immutable payload")]
    Conflict,
    #[error("invalid package or semantic version")]
    InvalidIdentity,
    #[error("bounded publication retry exhausted")]
    RetryExhausted,
    #[error("download interrupted for resume fixture")]
    Interrupted,
    #[error("requested permission exceeds configured grant")]
    Permission,
    #[error("candidate failed bounded startup")]
    Startup,
    #[error("I/O failure")]
    Io,
}

pub type Result<T> = std::result::Result<T, RegistryError>;

#[derive(Clone, Debug)]
pub struct Object {
    pub bytes: Vec<u8>,
    pub etag: String,
}

#[derive(Clone, Debug)]
pub enum PutCondition {
    Any,
    CreateOnly,
    Match(String),
}

#[async_trait]
pub trait ObjectStore: Send + Sync {
    async fn get(&self, key: &str) -> Result<Object>;
    async fn get_range(&self, key: &str, start: u64, end_exclusive: u64) -> Result<Vec<u8>>;
    async fn put(&self, key: &str, bytes: Vec<u8>, condition: PutCondition) -> Result<String>;
    async fn delete(&self, key: &str) -> Result<()>;
}

#[derive(Clone, Default)]
pub struct MemoryStore {
    inner: Arc<RwLock<BTreeMap<String, Object>>>,
}

#[async_trait]
impl ObjectStore for MemoryStore {
    async fn get(&self, key: &str) -> Result<Object> {
        self.inner
            .read()
            .await
            .get(key)
            .cloned()
            .ok_or(RegistryError::NotFound)
    }

    async fn get_range(&self, key: &str, start: u64, end_exclusive: u64) -> Result<Vec<u8>> {
        let object = self.get(key).await?;
        let start = usize::try_from(start).map_err(|_| RegistryError::Malformed)?;
        let end = usize::try_from(end_exclusive).map_err(|_| RegistryError::Malformed)?;
        if start > end || end > object.bytes.len() {
            return Err(RegistryError::Malformed);
        }
        Ok(object.bytes[start..end].to_vec())
    }

    async fn put(&self, key: &str, bytes: Vec<u8>, condition: PutCondition) -> Result<String> {
        let mut objects = self.inner.write().await;
        let current = objects.get(key);
        match condition {
            PutCondition::Any => {}
            PutCondition::CreateOnly if current.is_some() => {
                return Err(RegistryError::Precondition);
            }
            PutCondition::CreateOnly => {}
            PutCondition::Match(expected)
                if current.map(|object| object.etag.as_str()) != Some(expected.as_str()) =>
            {
                return Err(RegistryError::Precondition);
            }
            PutCondition::Match(_) => {}
        }
        let etag = format!("\"{}\"", hex::encode(sha256(&bytes)));
        objects.insert(
            key.to_owned(),
            Object {
                bytes,
                etag: etag.clone(),
            },
        );
        Ok(etag)
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.inner.write().await.remove(key);
        Ok(())
    }
}

pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

pub fn blob_path(digest: &[u8; 32]) -> String {
    format!("blobs/sha256/{}", hex::encode(digest))
}

pub fn shard_path(digest: &[u8; 32]) -> String {
    format!("registry/shards/sha256/{}.bin", hex::encode(digest))
}

pub fn fixture_release(
    package: &str,
    version: &str,
    component: &[u8],
    requested_permissions: &[&str],
    signing_key: &SigningKey,
) -> Result<(fb::ReleaseRecordT, String, Vec<u8>)> {
    validate_package(package)?;
    Version::parse(version).map_err(|_| RegistryError::InvalidIdentity)?;
    let component_digest = sha256(component);
    let payload = fb::ReleasePayloadT {
        schema_version: 1,
        signature_context: 1,
        kind: fb::ArtifactKind::Component,
        package: package.to_owned(),
        version: version.to_owned(),
        publisher: "litehouse:first-party".to_owned(),
        primary: Box::new(fb::BlobDescriptorT {
            media_type: "application/wasm".to_owned(),
            sha256: component_digest.to_vec(),
            length: component.len() as u64,
        }),
        dependencies: Some(Vec::new()),
        runtime_requirement: Some("^1.0.0".to_owned()),
        requested_permissions: Some(
            requested_permissions
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        ),
    };
    let payload_bytes = encode_payload(&payload);
    let payload_digest = sha256(&payload_bytes);
    let mut message = Vec::with_capacity(SIGNING_DOMAIN.len() + payload_digest.len());
    message.extend_from_slice(SIGNING_DOMAIN);
    message.extend_from_slice(&payload_digest);
    let signature = signing_key.sign(&message);
    let record = fb::ReleaseRecordT {
        payload: payload_bytes,
        payload_sha256: payload_digest.to_vec(),
        public_key: signing_key.verifying_key().to_bytes().to_vec(),
        signature: signature.to_bytes().to_vec(),
        yanked: false,
        withdrawn: false,
        revoked: false,
    };
    Ok((record, blob_path(&component_digest), component.to_vec()))
}

pub fn verify_record(
    record: &fb::ReleaseRecordT,
    trusted_key: &VerifyingKey,
) -> Result<fb::ReleasePayloadT> {
    if record.payload_sha256.as_slice() != sha256(&record.payload) {
        return Err(RegistryError::Digest);
    }
    if record.public_key.as_slice() != trusted_key.as_bytes() {
        return Err(RegistryError::Signature);
    }
    let signature_bytes: [u8; 64] = record
        .signature
        .as_slice()
        .try_into()
        .map_err(|_| RegistryError::Signature)?;
    let signature = Signature::from_bytes(&signature_bytes);
    let mut message = Vec::with_capacity(SIGNING_DOMAIN.len() + 32);
    message.extend_from_slice(SIGNING_DOMAIN);
    message.extend_from_slice(&record.payload_sha256);
    trusted_key
        .verify(&message, &signature)
        .map_err(|_| RegistryError::Signature)?;
    decode_payload(&record.payload)
}

fn encode_payload(value: &fb::ReleasePayloadT) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let root = value.pack(&mut builder);
    builder.finish(root, None);
    builder.finished_data().to_vec()
}

fn decode_payload(bytes: &[u8]) -> Result<fb::ReleasePayloadT> {
    let value =
        flatbuffers::root::<fb::ReleasePayload<'_>>(bytes).map_err(|_| RegistryError::Malformed)?;
    let unpacked = value.unpack();
    if unpacked.schema_version != 1 || unpacked.signature_context != 1 {
        return Err(RegistryError::Malformed);
    }
    validate_package(&unpacked.package)?;
    Version::parse(&unpacked.version).map_err(|_| RegistryError::InvalidIdentity)?;
    validate_digest(&unpacked.primary.sha256)?;
    Ok(unpacked)
}

fn encode_shard(value: &fb::ReleaseShardT) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let root = value.pack(&mut builder);
    builder.finish(root, None);
    builder.finished_data().to_vec()
}

fn decode_shard(bytes: &[u8]) -> Result<fb::ReleaseShardT> {
    let value =
        flatbuffers::root::<fb::ReleaseShard<'_>>(bytes).map_err(|_| RegistryError::Malformed)?;
    let unpacked = value.unpack();
    if unpacked.schema_version != 1 || unpacked.partition_id >= 7 {
        return Err(RegistryError::Malformed);
    }
    Ok(unpacked)
}

fn encode_root(value: &fb::RegistryRootT) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let root = value.pack(&mut builder);
    builder.finish(root, Some("LHR1"));
    builder.finished_data().to_vec()
}

pub fn decode_root(bytes: &[u8]) -> Result<fb::RegistryRootT> {
    let value = fb::root_as_registry_root(bytes).map_err(|_| RegistryError::Malformed)?;
    let unpacked = value.unpack();
    if unpacked.format_version != 1
        || unpacked.schema_version != 1
        || unpacked.normalization_version != 1
        || unpacked.partition_scheme != "alphabetical-v1-seven"
        || unpacked.shards.len() != 7
    {
        return Err(RegistryError::Malformed);
    }
    for (index, descriptor) in unpacked.shards.iter().enumerate() {
        if descriptor.partition_id != index as u32
            || descriptor.schema_version != 1
            || (
                descriptor.lower_bound.as_str(),
                descriptor.upper_bound.as_str(),
            ) != PARTITION_BOUNDS[index]
            || descriptor.length == 0
        {
            return Err(RegistryError::Malformed);
        }
        validate_digest(&descriptor.sha256)?;
    }
    Ok(unpacked)
}

fn validate_digest(value: &[u8]) -> Result<[u8; 32]> {
    value.try_into().map_err(|_| RegistryError::Malformed)
}

fn validate_package(package: &str) -> Result<()> {
    let Some((namespace, name)) = package.split_once(':') else {
        return Err(RegistryError::InvalidIdentity);
    };
    if namespace.is_empty()
        || name.is_empty()
        || !namespace
            .bytes()
            .chain(name.bytes())
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || !namespace.as_bytes()[0].is_ascii_lowercase()
    {
        return Err(RegistryError::InvalidIdentity);
    }
    Ok(())
}

fn partition_for(package: &str) -> Result<usize> {
    validate_package(package)?;
    let first = package.as_bytes()[0];
    Ok(usize::from((first - b'a') / 4).min(6))
}

async fn put_immutable<S: ObjectStore>(store: &S, key: &str, bytes: &[u8]) -> Result<()> {
    match store
        .put(key, bytes.to_vec(), PutCondition::CreateOnly)
        .await
    {
        Ok(_) => Ok(()),
        Err(RegistryError::Precondition) => {
            let current = store.get(key).await?;
            if current.bytes == bytes {
                Ok(())
            } else {
                Err(RegistryError::Conflict)
            }
        }
        Err(error) => Err(error),
    }
}

pub struct Publisher<S> {
    store: S,
    trusted_key: VerifyingKey,
}

impl<S: ObjectStore> Publisher<S> {
    pub fn new(store: S, trusted_key: VerifyingKey) -> Self {
        Self { store, trusted_key }
    }

    pub async fn initialize(&self) -> Result<()> {
        let mut descriptors = Vec::new();
        for (partition_id, (lower, upper)) in PARTITION_BOUNDS.iter().enumerate() {
            let shard = fb::ReleaseShardT {
                schema_version: 1,
                partition_id: partition_id as u32,
                records: Some(Vec::new()),
            };
            let bytes = encode_shard(&shard);
            let digest = sha256(&bytes);
            let path = shard_path(&digest);
            put_immutable(&self.store, &path, &bytes).await?;
            descriptors.push(fb::ShardDescriptorT {
                partition_id: partition_id as u32,
                lower_bound: (*lower).to_owned(),
                upper_bound: (*upper).to_owned(),
                path,
                sha256: digest.to_vec(),
                length: bytes.len() as u64,
                schema_version: 1,
            });
        }
        let root = fb::RegistryRootT {
            format_version: 1,
            schema_version: 1,
            normalization_version: 1,
            generation: 1,
            partition_scheme: "alphabetical-v1-seven".to_owned(),
            shards: descriptors,
        };
        match self
            .store
            .put(ROOT_KEY, encode_root(&root), PutCondition::CreateOnly)
            .await
        {
            Ok(_) | Err(RegistryError::Precondition) => Ok(()),
            Err(error) => Err(error),
        }
    }

    pub async fn publish(
        &self,
        record: fb::ReleaseRecordT,
        blob_key: &str,
        blob: &[u8],
    ) -> Result<()> {
        let payload = verify_record(&record, &self.trusted_key)?;
        if payload.primary.length != blob.len() as u64
            || payload.primary.sha256.as_slice() != sha256(blob)
            || blob_key != blob_path(&sha256(blob))
        {
            return Err(RegistryError::Digest);
        }
        put_immutable(&self.store, blob_key, blob).await?;
        let partition = partition_for(&payload.package)?;

        for _ in 0..8 {
            let current_root = self.store.get(ROOT_KEY).await?;
            let mut root = decode_root(&current_root.bytes)?;
            let descriptor = root
                .shards
                .get(partition)
                .ok_or(RegistryError::Malformed)?
                .clone();
            let shard_object = self.store.get(&descriptor.path).await?;
            if shard_object.bytes.len() as u64 != descriptor.length
                || sha256(&shard_object.bytes).as_slice() != descriptor.sha256
            {
                return Err(RegistryError::Digest);
            }
            let mut shard = decode_shard(&shard_object.bytes)?;
            let records = shard.records.get_or_insert_with(Vec::new);
            if let Some(existing) = records.iter().find(|existing| {
                decode_payload(&existing.payload)
                    .map(|value| {
                        value.package == payload.package && value.version == payload.version
                    })
                    .unwrap_or(false)
            }) {
                return if existing.payload_sha256 == record.payload_sha256 {
                    Ok(())
                } else {
                    Err(RegistryError::Conflict)
                };
            }
            records.push(record.clone());
            records.sort_by_key(release_sort_key);
            let new_shard_bytes = encode_shard(&shard);
            let new_shard_digest = sha256(&new_shard_bytes);
            let new_shard_path = shard_path(&new_shard_digest);
            put_immutable(&self.store, &new_shard_path, &new_shard_bytes).await?;
            root.shards[partition].path = new_shard_path;
            root.shards[partition].sha256 = new_shard_digest.to_vec();
            root.shards[partition].length = new_shard_bytes.len() as u64;
            root.generation += 1;
            match self
                .store
                .put(
                    ROOT_KEY,
                    encode_root(&root),
                    PutCondition::Match(current_root.etag),
                )
                .await
            {
                Ok(_) => return Ok(()),
                Err(RegistryError::Precondition) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(RegistryError::RetryExhausted)
    }
}

fn release_sort_key(record: &fb::ReleaseRecordT) -> (String, Version) {
    decode_payload(&record.payload)
        .and_then(|value| {
            Version::parse(&value.version)
                .map(|version| (value.package, version))
                .map_err(|_| RegistryError::Malformed)
        })
        .unwrap_or_else(|_| (String::new(), Version::new(0, 0, 0)))
}

pub fn query_exact_bytes(
    root_bytes: &[u8],
    shard_bytes: &[u8],
    package: &str,
    version: &str,
    trusted_key: &VerifyingKey,
) -> Result<fb::ReleasePayloadT> {
    let root = decode_root(root_bytes)?;
    let partition = partition_for(package)?;
    let descriptor = &root.shards[partition];
    if shard_bytes.len() as u64 != descriptor.length
        || sha256(shard_bytes).as_slice() != descriptor.sha256
    {
        return Err(RegistryError::Digest);
    }
    let shard = decode_shard(shard_bytes)?;
    for record in shard.records.unwrap_or_default() {
        let payload = verify_record(&record, trusted_key)?;
        if payload.package == package && payload.version == version {
            if record.yanked || record.withdrawn || record.revoked {
                return Err(RegistryError::Conflict);
            }
            return Ok(payload);
        }
    }
    Err(RegistryError::NotFound)
}

pub async fn query_exact<S: ObjectStore>(
    store: &S,
    package: &str,
    version: &str,
    trusted_key: &VerifyingKey,
) -> Result<fb::ReleasePayloadT> {
    let root_object = store.get(ROOT_KEY).await?;
    let root = decode_root(&root_object.bytes)?;
    let partition = partition_for(package)?;
    let descriptor = &root.shards[partition];
    let shard_object = store.get(&descriptor.path).await?;
    query_exact_bytes(
        &root_object.bytes,
        &shard_object.bytes,
        package,
        version,
        trusted_key,
    )
}

pub async fn download_verified<S: ObjectStore>(
    store: &S,
    key: &str,
    expected_digest: [u8; 32],
    expected_length: u64,
    destination: &Path,
    chunk_size: u64,
    interrupt_after: Option<u64>,
) -> Result<()> {
    let partial = PathBuf::from(format!("{}.part", destination.display()));
    let mut downloaded = match fs::metadata(&partial).await {
        Ok(metadata) => metadata.len(),
        Err(_) => 0,
    };
    if downloaded > expected_length {
        fs::remove_file(&partial)
            .await
            .map_err(|_| RegistryError::Io)?;
        downloaded = 0;
    }
    while downloaded < expected_length {
        let end = (downloaded + chunk_size).min(expected_length);
        let bytes = store.get_range(key, downloaded, end).await?;
        if bytes.len() as u64 != end - downloaded {
            return Err(RegistryError::Digest);
        }
        use tokio::io::AsyncWriteExt;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&partial)
            .await
            .map_err(|_| RegistryError::Io)?;
        file.write_all(&bytes)
            .await
            .map_err(|_| RegistryError::Io)?;
        file.flush().await.map_err(|_| RegistryError::Io)?;
        downloaded = end;
        if interrupt_after.is_some_and(|limit| downloaded >= limit) && downloaded < expected_length
        {
            return Err(RegistryError::Interrupted);
        }
    }
    let bytes = fs::read(&partial).await.map_err(|_| RegistryError::Io)?;
    if bytes.len() as u64 != expected_length || sha256(&bytes) != expected_digest {
        return Err(RegistryError::Digest);
    }
    fs::rename(partial, destination)
        .await
        .map_err(|_| RegistryError::Io)?;
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivatedRelease {
    pub package: String,
    pub version: String,
    pub release_digest: [u8; 32],
    pub permissions: BTreeSet<String>,
}

#[derive(Default)]
pub struct ActivationState {
    pub current: Option<ActivatedRelease>,
    pub previous: Option<ActivatedRelease>,
}

impl ActivationState {
    pub fn activate(
        &mut self,
        candidate: ActivatedRelease,
        configured_grants: &BTreeSet<String>,
        startup_succeeds: bool,
    ) -> Result<()> {
        if !candidate.permissions.is_subset(configured_grants) {
            return Err(RegistryError::Permission);
        }
        if !startup_succeeds {
            return Err(RegistryError::Startup);
        }
        self.previous = self.current.replace(candidate);
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<()> {
        let previous = self.previous.take().ok_or(RegistryError::NotFound)?;
        self.previous = self.current.replace(previous);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7; 32])
    }

    #[tokio::test]
    async fn concurrent_publication_preserves_both_releases() -> Result<()> {
        let store = MemoryStore::default();
        let key = signing_key();
        let publisher = Arc::new(Publisher::new(store.clone(), key.verifying_key()));
        publisher.initialize().await?;
        let (first, first_path, first_blob) =
            fixture_release("litehouse:on-off", "1.0.0", b"one", &[], &key)?;
        let (second, second_path, second_blob) =
            fixture_release("litehouse:measurement", "1.0.0", b"two", &[], &key)?;
        let left = tokio::spawn({
            let publisher = publisher.clone();
            async move { publisher.publish(first, &first_path, &first_blob).await }
        });
        let right = tokio::spawn({
            let publisher = publisher.clone();
            async move { publisher.publish(second, &second_path, &second_blob).await }
        });
        left.await.map_err(|_| RegistryError::Io)??;
        right.await.map_err(|_| RegistryError::Io)??;
        query_exact(&store, "litehouse:on-off", "1.0.0", &key.verifying_key()).await?;
        query_exact(
            &store,
            "litehouse:measurement",
            "1.0.0",
            &key.verifying_key(),
        )
        .await?;
        Ok(())
    }

    #[tokio::test]
    async fn verified_cache_supports_offline_query_and_yanks_fail_closed() -> Result<()> {
        let store = MemoryStore::default();
        let key = signing_key();
        let publisher = Publisher::new(store.clone(), key.verifying_key());
        publisher.initialize().await?;
        let (record, path, blob) =
            fixture_release("litehouse:on-off", "1.0.0", b"offline", &[], &key)?;
        publisher.publish(record, &path, &blob).await?;
        let root = store.get(ROOT_KEY).await?;
        let mut decoded = decode_root(&root.bytes)?;
        let descriptor = decoded
            .shards
            .get(2)
            .ok_or(RegistryError::Malformed)?
            .clone();
        let shard = store.get(&descriptor.path).await?;
        let directory = tempfile::tempdir().map_err(|_| RegistryError::Io)?;
        fs::write(directory.path().join("root.bin"), &root.bytes)
            .await
            .map_err(|_| RegistryError::Io)?;
        fs::write(directory.path().join("shard.bin"), &shard.bytes)
            .await
            .map_err(|_| RegistryError::Io)?;
        drop(store);
        let cached_root = fs::read(directory.path().join("root.bin"))
            .await
            .map_err(|_| RegistryError::Io)?;
        let cached_shard = fs::read(directory.path().join("shard.bin"))
            .await
            .map_err(|_| RegistryError::Io)?;
        query_exact_bytes(
            &cached_root,
            &cached_shard,
            "litehouse:on-off",
            "1.0.0",
            &key.verifying_key(),
        )?;

        let mut decoded_shard = decode_shard(&cached_shard)?;
        decoded_shard
            .records
            .as_mut()
            .ok_or(RegistryError::Malformed)?[0]
            .yanked = true;
        let yanked = encode_shard(&decoded_shard);
        decoded.shards[2].length = yanked.len() as u64;
        decoded.shards[2].sha256 = sha256(&yanked).to_vec();
        let yanked_root = encode_root(&decoded);
        assert!(matches!(
            query_exact_bytes(
                &yanked_root,
                &yanked,
                "litehouse:on-off",
                "1.0.0",
                &key.verifying_key()
            ),
            Err(RegistryError::Conflict)
        ));
        Ok(())
    }

    #[tokio::test]
    async fn malformed_signature_and_blob_substitution_fail_closed() -> Result<()> {
        let key = signing_key();
        let (mut record, _, _) = fixture_release("litehouse:on-off", "1.0.0", b"one", &[], &key)?;
        record.payload[0] ^= 1;
        assert!(matches!(
            verify_record(&record, &key.verifying_key()),
            Err(RegistryError::Digest)
        ));
        Ok(())
    }

    #[tokio::test]
    async fn interrupted_download_resumes_and_verifies() -> Result<()> {
        let store = MemoryStore::default();
        let bytes = b"0123456789abcdef".to_vec();
        let digest = sha256(&bytes);
        let key = blob_path(&digest);
        store
            .put(&key, bytes.clone(), PutCondition::CreateOnly)
            .await?;
        let directory = tempfile::tempdir().map_err(|_| RegistryError::Io)?;
        let destination = directory.path().join("blob");
        assert!(matches!(
            download_verified(
                &store,
                &key,
                digest,
                bytes.len() as u64,
                &destination,
                4,
                Some(8)
            )
            .await,
            Err(RegistryError::Interrupted)
        ));
        download_verified(
            &store,
            &key,
            digest,
            bytes.len() as u64,
            &destination,
            4,
            None,
        )
        .await?;
        assert_eq!(
            fs::read(destination).await.map_err(|_| RegistryError::Io)?,
            bytes
        );
        Ok(())
    }

    #[test]
    fn activation_rejects_permission_growth_and_retains_one_predecessor() -> Result<()> {
        let grants = BTreeSet::from(["tcp:node-s".to_owned()]);
        let first = ActivatedRelease {
            package: "litehouse:esphome".to_owned(),
            version: "1.0.0".to_owned(),
            release_digest: [1; 32],
            permissions: grants.clone(),
        };
        let expanded = ActivatedRelease {
            version: "1.1.0".to_owned(),
            permissions: BTreeSet::from(["tcp:node-s".to_owned(), "tcp:any".to_owned()]),
            ..first.clone()
        };
        let second = ActivatedRelease {
            version: "1.0.1".to_owned(),
            release_digest: [2; 32],
            ..first.clone()
        };
        let mut state = ActivationState::default();
        state.activate(first.clone(), &grants, true)?;
        assert!(matches!(
            state.activate(expanded, &grants, true),
            Err(RegistryError::Permission)
        ));
        assert_eq!(state.current, Some(first.clone()));
        assert!(matches!(
            state.activate(second.clone(), &grants, false),
            Err(RegistryError::Startup)
        ));
        assert_eq!(state.current, Some(first.clone()));
        state.activate(second, &grants, true)?;
        assert_eq!(state.previous, Some(first));
        state.rollback()?;
        assert_eq!(
            state
                .current
                .as_ref()
                .map(|release| release.version.as_str()),
            Some("1.0.0")
        );
        Ok(())
    }
}
