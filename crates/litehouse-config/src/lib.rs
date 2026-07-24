//! Configuration types for the Litehouse home automation system.
//!
//! This crate provides the necessary types and functions to manage the configuration
//! of the Litehouse system, including plugin management, registry settings, and system
//! capabilities.

mod capabilities;
mod hash_read;
mod import;
mod manifest;
mod parallelism;

use std::{
    cmp::Ordering,
    collections::{BTreeMap, btree_map::Entry},
    io::{BufWriter, Write},
    num::NonZeroU8,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering as AtomicOrdering},
};

use miette::{NamedSource, SourceOffset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use capabilities::Capability;
pub use import::Import;
pub use manifest::{Manifest, ManifestImport};
pub use parallelism::SandboxStrategy;

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct LitehouseConfig {
    #[serde(rename = "$schema", default = "default_schema")]
    pub schema: String,
    /// Directory containing one authored Rule JSON workflow per JSON or YAML file.
    #[serde(default = "default_workflows_directory")]
    pub workflows: PathBuf,
    /// The list of plugins to use in this litehouse.
    /// A sorted map keeps serialized settings deterministic and reviewable.
    pub plugins: BTreeMap<String, PluginConfig>,
    /// Additional registries to look for plugins in
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registries: Vec<Registry>,
    /// Additional plugins to import from registries. Without a registry prefix, uses the default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(with = "Vec<String>")]
    pub imports: Vec<Import>,
    /// The capabilities of this litehouse. Plugins that attempt to use capabilities not present in
    /// this list will fail. By default, plugins are not given any capabilities and are completely
    /// sandboxed.
    ///
    /// Can be one of the following:
    /// - `http-server:<port>`: Start an HTTP server on the given port
    /// - `http-client:<url>`: Make HTTP requests to the given URL
    /// - `tcp-client:<ip>:<port>`: Open TCP connections to one IPv4 address and port
    /// - `tcp-client:[<ipv6>]:<port>`: Open TCP connections to one IPv6 address and port
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(with = "Vec<String>")]
    pub capabilities: Vec<Capability>,
    /// Advanced engine configuration
    #[serde(default, skip_serializing_if = "is_default")]
    pub engine: Engine,
    /// Configuration to connect to a litehouse broker
    #[serde(default, skip_serializing_if = "is_default")]
    pub broker: Option<Broker>,
    #[serde(skip)]
    #[schemars(skip)]
    source_digest: Option<[u8; 32]>,
}

fn default_schema() -> String {
    "./schema.json".to_string()
}

fn default_workflows_directory() -> PathBuf {
    PathBuf::from("./workflows")
}

impl Default for LitehouseConfig {
    fn default() -> Self {
        Self {
            schema: default_schema(),
            workflows: default_workflows_directory(),
            plugins: BTreeMap::new(),
            registries: Vec::new(),
            imports: Vec::new(),
            capabilities: Vec::new(),
            engine: Engine::default(),
            broker: None,
            source_digest: None,
        }
    }
}

#[derive(JsonSchema, Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Broker {
    /// The url of the broker to facilitate webrtc connections
    ///
    /// If not provided, defaults to `https://cockpit.litehouse.arlyon.dev`.
    pub url: Option<url::Url>,
    /// A jwt certificate signed by some authority that can be used to prove
    /// the identity of the instance
    pub cert: Option<String>,
    /// A password that local network clients must know to claim the litehouse instance
    pub password: [u8; 6],
}

impl Broker {
    pub fn url(&self) -> url::Url {
        self.url
            .clone()
            .unwrap_or_else(|| "https://cockpit.litehouse.arlyon.dev".parse().unwrap())
    }
}

#[derive(JsonSchema, Serialize, Deserialize, Debug, Default, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Engine {
    /// The strategy to use for sandboxing plugins. By default, each plugin instance is run in its
    /// own storage sandbox, for maximum parallelism and isolation. If you are in a constrained
    /// environment, you may want to put all plugins in the same storage instead.
    #[serde(default, skip_serializing_if = "is_default")]
    pub sandbox_strategy: SandboxStrategy,
    #[serde(default, skip_serializing_if = "is_default")]
    pub max_parallel_builds: MaxBuildCount,
    #[serde(default, skip_serializing_if = "is_default")]
    pub max_parallel_instantiations: MaxBuildCount,
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    *t == Default::default()
}

#[derive(JsonSchema, Serialize, Deserialize, Debug, PartialEq)]
pub struct MaxBuildCount(NonZeroU8);

impl Default for MaxBuildCount {
    fn default() -> Self {
        MaxBuildCount(NonZeroU8::new(10).unwrap())
    }
}

impl From<MaxBuildCount> for u8 {
    fn from(count: MaxBuildCount) -> Self {
        count.0.get()
    }
}

impl LitehouseConfig {
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from("settings.json")
    }

    pub fn load_from(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let data = std::fs::read_to_string(path)?;
        let source_digest = content_digest(data.as_bytes());
        let mut config: LitehouseConfig = serde_json::from_str(&data).map_err(|e| {
            ConfigError::Parse(ParseError {
                err_span: SourceOffset::from_location(&data, e.line() - 1, e.column()).into(),
                src: NamedSource::new(path.display().to_string(), data),
                error: e.to_string(),
            })
        })?;
        config.source_digest = Some(source_digest);
        Ok(config)
    }

    pub fn save(&mut self) -> Result<(), ConfigError> {
        self.save_to("settings.json")
    }

    pub fn save_to(&mut self, path: impl AsRef<Path>) -> Result<(), ConfigError> {
        self.source_digest = Some(atomic_write_json(path.as_ref(), self, self.source_digest)?);
        Ok(())
    }

    /// Add a new plugin to the configuration. If a plugin with same name already exists, a new
    /// one will only be added if either the version, SHA, or registry is different.
    ///
    /// This is not strictly the same as 'equality', since our PartialEq and Eq implementations
    /// compare all fields piecewise. This instead checks for 'compatibility'.
    ///
    /// - if the target is more specific than the existing plugin, the existing definition is
    ///   replaced with the new one
    /// - if the target is less specific than the existing plugin, the existing definition is
    ///   kept
    /// - if the target is equally specific as the existing plugin, and they are not equal, the
    ///   existing definition is kept, and the new one is ignored
    pub fn add_import(&mut self, import: Import) -> ImportAddResult<'_> {
        // this is awkward but the borrow checker does not understand
        // returning in the match statement
        let mut ret_replace = None;
        let mut ret_existing = None;

        for (i, existing) in self.imports.iter().enumerate() {
            match existing.specificity(&import) {
                Some(Ordering::Less) => {
                    // The existing import is less specific than the new one, replace
                    ret_replace = Some(i);
                    break;
                }
                Some(Ordering::Greater) => {
                    // The existing import is more specific than the new one, keep the existing
                    ret_existing = Some(i);
                    break;
                }
                Some(Ordering::Equal) => {
                    // The existing import is equally specific as the new one (and they are equal)
                    ret_existing = Some(i);
                    break;
                }
                None => {
                    // The existing import is not compatible with the new one
                }
            }
        }

        match (ret_replace, ret_existing) {
            (Some(i), None) => {
                let old = std::mem::replace(&mut self.imports[i], import);
                ImportAddResult::Replaced(old)
            }
            (None, Some(i)) => ImportAddResult::Ignored(&self.imports[i]),
            (None, None) => {
                self.imports.push(import);
                ImportAddResult::Added(self.imports.last_mut().unwrap())
            }
            _ => unreachable!(),
        }
    }

    pub fn add_manifest(
        &mut self,
        manifest: Manifest,
        replace: bool,
    ) -> impl Iterator<Item = ManifestAddResult> + '_ {
        self.add_import(manifest.import.clone());
        manifest
            .config
            .into_iter()
            .map(move |(k, config)| match self.plugins.entry(k.clone()) {
                Entry::Occupied(mut e) => {
                    // check if they are equal, and ignore
                    let val = e.get_mut();
                    if val.plugin == manifest.import && val.config == config {
                        ManifestAddResult::Ignored(k)
                    } else if replace {
                        let instance_id = val.instance_id.clone();
                        let capabilities = val.capabilities.clone();
                        let v_old = e.insert(PluginConfig {
                            plugin: manifest.import.clone(),
                            instance_id,
                            capabilities,
                            config,
                        });
                        ManifestAddResult::Replaced(k, v_old.config)
                    } else {
                        ManifestAddResult::WouldReplace(k, config)
                    }
                }
                Entry::Vacant(entry) => {
                    entry.insert(PluginConfig {
                        plugin: manifest.import.clone(),
                        instance_id: None,
                        capabilities: Vec::new(),
                        config,
                    });
                    ManifestAddResult::Added(k)
                }
            })
    }
}

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn atomic_write_json(
    path: &Path,
    value: &impl Serialize,
    expected_digest: Option<[u8; 32]>,
) -> Result<[u8; 32], ConfigError> {
    let mut encoded = Vec::new();
    serde_json::to_writer_pretty(&mut encoded, value).map_err(ConfigError::Write)?;
    encoded.push(b'\n');

    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "settings path has no file name",
        )
    })?;
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(parent.join(".litehouse-authoring.lock"))?;
    lock_file.lock()?;
    check_expected_digest(path, expected_digest)?;

    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
    let temporary_path = parent.join(format!(
        ".{}.{}.{}.tmp",
        file_name.to_string_lossy(),
        std::process::id(),
        sequence
    ));
    let mut cleanup = TemporaryFile::new(temporary_path.clone());
    let file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary_path)?;
    if let Ok(metadata) = std::fs::metadata(path) {
        file.set_permissions(metadata.permissions())?;
    }
    {
        let mut writer = BufWriter::new(&file);
        writer.write_all(&encoded)?;
        writer.flush()?;
    }
    file.sync_all()?;
    check_expected_digest(path, expected_digest)?;
    #[cfg(unix)]
    let parent_directory = std::fs::File::open(parent)?;
    std::fs::rename(&temporary_path, path)?;
    cleanup.disarm();
    #[cfg(unix)]
    parent_directory
        .sync_all()
        .map_err(ConfigError::CommitIndeterminate)?;
    Ok(content_digest(&encoded))
}

fn content_digest(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

fn check_expected_digest(
    path: &Path,
    expected_digest: Option<[u8; 32]>,
) -> Result<(), ConfigError> {
    match (expected_digest, std::fs::read(path)) {
        (Some(expected), Ok(bytes)) if content_digest(&bytes) == expected => Ok(()),
        (None, Err(error)) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        (Some(_), Err(error)) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Err(ConfigError::Conflict),
    }
}

struct TemporaryFile {
    path: PathBuf,
    armed: bool,
}

impl TemporaryFile {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

pub enum ManifestAddResult {
    WouldReplace(String, Option<serde_json::Value>),
    Replaced(String, Option<serde_json::Value>),
    Added(String),
    Ignored(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum ImportAddResult<'a> {
    Added(&'a Import),
    Replaced(Import),
    Ignored(&'a Import),
}

pub fn directories() -> Option<directories_next::ProjectDirs> {
    directories_next::ProjectDirs::from("com", "litehouse", "litehouse")
}

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
pub enum ConfigError {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    #[diagnostic(transparent)]
    Parse(#[from] ParseError),
    #[error("write error")]
    Write(serde_json::Error),
    #[error("settings changed since they were loaded")]
    Conflict,
    #[error("settings were replaced but directory durability could not be confirmed")]
    CommitIndeterminate(#[source] std::io::Error),
}

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
#[error("parse error")]
#[diagnostic(
    code(config::invalid),
    url(docsrs),
    help("check the configuration file for errors")
)]
/// Raised when there is an error parsing the configuration file.
/// This is likely a formatting issue.
pub struct ParseError {
    #[source_code]
    pub src: NamedSource<String>,
    pub error: String,

    #[label = "{error}"]
    pub err_span: miette::SourceSpan,
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// The local name of the registry
    pub name: String,
    /// The url to use for interacting with the registry
    pub url: String,
}

/// A combination of a plugin import and its configuration.
#[derive(JsonSchema, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct PluginConfig {
    #[schemars(with = "String")]
    pub plugin: Import,
    /// Stable canonical UUID used to derive protected environment-variable names.
    /// Existing instances without protected fields may omit it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    /// Additional capabilities granted only to this configured instance.
    /// The top-level capability list remains a backwards-compatible default.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(with = "Vec<String>")]
    pub capabilities: Vec<Capability>,
    pub config: Option<serde_json::Value>,
}

#[cfg(test)]
mod test {
    use std::{fs, str::FromStr};

    use serde::Serializer;
    use test_case::test_case;

    use super::*;

    #[test_case("package" ; "just package")]
    #[test_case("registry::package" ; "registry")]
    #[test_case("registry::package@1.0.0" ; "version")]
    #[test_case("registry::package@1.0.0~blake3:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef" ; "everything")]
    #[test_case("registry::package~blake3:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef" ; "no version")]
    #[test_case("package~blake3:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef" ; "just sha")]
    fn roundtrip(import_exp: &str) {
        let package = Import::from_str(import_exp).unwrap();
        let import_actual = package.to_string();
        assert_eq!(import_exp, import_actual);
        assert_eq!(package.plugin, "package");
    }

    #[test_case(&[], "registry::package" => matches ImportAddResult::Added(_) ; "basic case")]
    #[test_case(&["registry::package"], "registry::package" => matches ImportAddResult::Ignored(_); "duplicate")]
    #[test_case(&["registry::package"], "registry::package@1.0.0" => matches ImportAddResult::Replaced(_) ; "add more specific version should overwrite")]
    #[test_case(&["registry::package@1.0.0"], "registry::package" => matches ImportAddResult::Ignored(_) ; "add less specific version should be ignored")]
    #[test_case(&["package@1.0.1"], "package@1.0.2" => matches ImportAddResult::Added(_) ; "incompatible imports should be added")]
    fn add_import(list: &[&str], import: &str) -> ImportAddResult<'static> {
        let mut config = LitehouseConfig {
            imports: list.iter().map(|s| s.parse().unwrap()).collect(),
            ..Default::default()
        };

        // just leak...
        match config.add_import(import.parse().unwrap()) {
            ImportAddResult::Added(x) => {
                let x = Box::leak(Box::new(x.to_owned()));
                ImportAddResult::Added(x)
            }
            ImportAddResult::Ignored(x) => {
                let x = Box::leak(Box::new(x.to_owned()));
                ImportAddResult::Ignored(x)
            }
            ImportAddResult::Replaced(x) => ImportAddResult::Replaced(x),
        }
    }

    #[test]
    fn settings_save_is_atomic_and_deterministically_ordered() {
        let directory = std::env::temp_dir().join(format!(
            "litehouse-config-{}-{}",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        let mut config = LitehouseConfig::default();
        for name in ["z-last", "a-first"] {
            config.plugins.insert(
                name.to_owned(),
                PluginConfig {
                    plugin: Import::from_str("package@1.0.0").unwrap(),
                    instance_id: None,
                    capabilities: Vec::new(),
                    config: None,
                },
            );
        }

        config.save_to(&path).unwrap();
        let first = fs::read_to_string(&path).unwrap();
        config.save_to(&path).unwrap();
        let second = fs::read_to_string(&path).unwrap();
        assert_eq!(first, second);
        assert!(first.ends_with('\n'));
        assert!(first.find("a-first").unwrap() < first.find("z-last").unwrap());
        assert_eq!(
            LitehouseConfig::load_from(&path).unwrap().workflows,
            PathBuf::from("./workflows")
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_settings_encoding_preserves_the_existing_file() {
        struct FailingValue;
        impl Serialize for FailingValue {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                Err(serde::ser::Error::custom("deliberate encoding failure"))
            }
        }

        let directory = std::env::temp_dir().join(format!(
            "litehouse-config-failure-{}-{}",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        fs::write(&path, b"original\n").unwrap();

        assert!(matches!(
            atomic_write_json(&path, &FailingValue, None),
            Err(ConfigError::Write(_))
        ));
        assert_eq!(fs::read(&path).unwrap(), b"original\n");
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn stale_settings_writes_conflict_without_losing_the_first_update() {
        let directory = std::env::temp_dir().join(format!(
            "litehouse-config-conflict-{}-{}",
            std::process::id(),
            TEMP_FILE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed)
        ));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        let mut initial = LitehouseConfig::default();
        initial.save_to(&path).unwrap();
        let mut first = LitehouseConfig::load_from(&path).unwrap();
        let mut stale = LitehouseConfig::load_from(&path).unwrap();
        first.workflows = PathBuf::from("./first");
        stale.workflows = PathBuf::from("./stale");

        first.save_to(&path).unwrap();
        assert!(matches!(stale.save_to(&path), Err(ConfigError::Conflict)));
        assert_eq!(
            LitehouseConfig::load_from(&path).unwrap().workflows,
            PathBuf::from("./first")
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn unknown_settings_fields_fail_closed_before_mutation() {
        let error = serde_json::from_value::<LitehouseConfig>(serde_json::json!({
            "$schema": "./schema.json",
            "workflows": "./workflows",
            "plugins": {},
            "unexpected": true
        }))
        .unwrap_err();
        assert!(error.to_string().contains("unknown field"));

        let error = serde_json::from_value::<LitehouseConfig>(serde_json::json!({
            "$schema": "./schema.json",
            "workflows": "./workflows",
            "plugins": {
                "example": {
                    "plugin": "example@1.0.0",
                    "config": null,
                    "unexpected": true
                }
            }
        }))
        .unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn plugin_instance_capabilities_roundtrip_without_changing_global_defaults() {
        let source = serde_json::json!({
            "$schema": "./schema.json",
            "plugins": {
                "lamp": {
                    "plugin": "noop",
                    "instance_id": "00112233-4455-6677-8899-aabbccddeeff",
                    "capabilities": ["http-client:http://192.168.1.50:8002"],
                    "config": null
                }
            }
        });

        let config: LitehouseConfig = serde_json::from_value(source).unwrap();
        let lamp = &config.plugins["lamp"];

        assert!(config.capabilities.is_empty());
        assert_eq!(
            lamp.instance_id.as_deref(),
            Some("00112233-4455-6677-8899-aabbccddeeff")
        );
        assert_eq!(lamp.capabilities.len(), 1);
        assert_eq!(
            lamp.capabilities[0].to_string(),
            "http-client:http://192.168.1.50:8002"
        );
    }

    #[test_case("package", "package" => Some(Ordering::Equal) ; "same package")]
    #[test_case("package", "registry::package" => Some(Ordering::Less) ; "added registry")]
    #[test_case("registry::package", "package" => Some(Ordering::Greater) ; "removed registry")]
    #[test_case("registry::package", "registry::package" => Some(Ordering::Equal) ; "same registry")]
    #[test_case("registry::package@1.0.0", "registry::package" => Some(Ordering::Greater) ; "added version")]
    #[test_case("registry::package", "registry::package@1.0.0" => Some(Ordering::Less) ; "removed version")]
    #[test_case("registry::package@1.0.0", "registry::package@1.0.0" =>Some(Ordering::Equal) ; "same version")]
    #[test_case("package", "registry::package@0.1.0" => Some(Ordering::Less) ; "lots of extra information")]
    fn cmp_import(a: &str, b: &str) -> Option<std::cmp::Ordering> {
        let a = Import::from_str(a).unwrap();
        let b = Import::from_str(b).unwrap();
        a.specificity(&b)
    }
}
