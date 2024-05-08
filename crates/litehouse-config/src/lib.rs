//! Configuration types for the Litehouse home automation system.
//!
//! This crate provides the necessary types and functions to manage the configuration
//! of the Litehouse system, including plugin management, registry settings, and system
//! capabilities.

#![feature(let_chains)]

mod hash_read;

use std::{
    cmp::Ordering, collections::HashMap, fmt::Display, num::NonZeroU8, path::Path, str::FromStr,
};

use hash_read::HashRead;
use miette::{Diagnostic, NamedSource, SourceOffset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};

const REGISTRY_SEPARATOR: &str = "::";
const VERSION_SEPARATOR: &str = "@";
const SHA_SEPERATOR: &str = "~";

#[derive(JsonSchema, Serialize, Deserialize, Debug, Default)]
pub struct LitehouseConfig {
    #[serde(rename = "$schema", default = "default_schema")]
    pub schema: String,
    /// The list of plugins to use in this litehouse
    pub plugins: HashMap<String, PluginInstance>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[schemars(with = "Vec<String>")]
    pub capabilities: Vec<Capability>,
    /// Advanced engine configuration
    #[serde(default, skip_serializing_if = "is_default")]
    pub engine: Engine,
}

fn default_schema() -> String {
    "./schema.json".to_string()
}

#[derive(JsonSchema, Serialize, Deserialize, Debug, Default, PartialEq)]
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

#[derive(JsonSchema, Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxStrategy {
    /// All plugins are run in the same storage sandbox
    Global,
    /// Each plugin type is run in its own storage sandbox
    Plugin,
    /// Each plugin instance is run in its own storage sandbox
    #[default]
    Instance,
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
        let data = std::fs::read_to_string("settings.json")?;
        let config: LitehouseConfig = serde_json::from_str(&data).map_err(|e| {
            ConfigError::Parse(ParseError {
                err_span: SourceOffset::from_location(&data, e.line() - 1, e.column()).into(),
                src: NamedSource::new("settings.json", data),
                error: e.to_string(),
            })
        })?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let file = std::fs::File::create("settings.json")?;
        serde_json::to_writer_pretty(&file, self).map_err(ConfigError::Write)?;
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
    pub fn add_import<'a>(&'a mut self, import: Import) -> ImportAddResult<'a> {
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
                return ImportAddResult::Replaced(old);
            }
            (None, Some(i)) => {
                return ImportAddResult::Ignored(&self.imports[i]);
            }
            (None, None) => {
                self.imports.push(import);
                return ImportAddResult::Added(self.imports.last_mut().unwrap());
            }
            _ => unreachable!(),
        }
    }
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

#[derive(Debug, Clone)]
pub enum Capability {
    HttpServer(usize),
    HttpClient(String),
}

impl Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Capability::HttpServer(port) => write!(f, "http-server:{}", port),
            Capability::HttpClient(url) => write!(f, "http-client:{}", url),
        }
    }
}

impl FromStr for Capability {
    type Err = CapabilityParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, value) = s
            .split_once(':')
            .map(|(name, value)| (name, value.to_string()))
            .ok_or_else(|| CapabilityParseError::MissingDelimiter)?;
        match name {
            "http-server" => Ok(value
                .parse()
                .map(Capability::HttpServer)
                .map_err(|_| CapabilityParseError::InvalidPort(value)))?,
            "http-client" => Ok(Capability::HttpClient(value)),
            variant => Err(CapabilityParseError::UnknownVariant(variant.to_string())),
        }
    }
}

#[derive(Error, Diagnostic, Debug)]
#[error("invalid capability")]
#[diagnostic(
    code(config::invalid_capability),
    url(docsrs),
    help("check the capability name and value")
)]
pub enum CapabilityParseError {
    #[error("unknown variant: {0}")]
    UnknownVariant(String),
    #[error("missing delimiter")]
    MissingDelimiter,
    #[error("invalid port: {0}")]
    InvalidPort(String),
}

impl Serialize for Capability {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string = self.to_string();
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for Capability {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct Registry {
    /// The local name of the registry
    pub name: String,
    /// The url to use for interacting with the registry
    pub url: String,
}

/// A plugin import. Serializes to a string with the format `registry::plugin`
#[derive(Debug, Clone)]
pub struct Import {
    pub registry: Option<String>,
    pub plugin: String,
    pub version: Option<semver::Version>,
    pub sha: Option<Blake3>,
}

impl Import {
    pub fn file_name(&self) -> String {
        let version = self
            .version
            .as_ref()
            .map(|v| format!("{}{}", VERSION_SEPARATOR, v))
            .unwrap_or_default();
        format!("{}{}.wasm", self.plugin, version)
    }

    pub async fn read_sha(&mut self, base_dir: &Path) {
        use futures::StreamExt;

        // if there is no version, we need to resolve it
        if self.version.is_none() {
            let files = tokio::fs::read_dir(base_dir).await.unwrap();
            let stream = tokio_stream::wrappers::ReadDirStream::new(files);
            let max_version = stream
                .filter_map(|entry| {
                    let import = Import::from_str(
                        entry
                            .unwrap()
                            .file_name()
                            .to_string_lossy()
                            .strip_suffix(".wasm")
                            .unwrap(),
                    )
                    .unwrap();
                    let plugin = &self.plugin;
                    async move {
                        if import.plugin.eq(plugin) {
                            Some(import)
                        } else {
                            None
                        }
                    }
                })
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .max();

            if let Some(import) = max_version {
                self.version = import.version;
            } else {
                return;
            }
        }

        let plugin_path = base_dir.join(self.file_name());
        let hasher = blake3::Hasher::new();
        let file = tokio::fs::File::open(plugin_path).await.unwrap();
        let mut hasher = HashRead::new(file, hasher);
        tokio::io::copy(&mut hasher, &mut tokio::io::empty())
            .await
            .unwrap();
        let output = hasher.finalize();
        let b: [u8; 32] = output.as_slice().try_into().unwrap();
        self.sha = Some(Blake3(b));
    }

    /// Verify that the plugin at this path matches
    /// this import. This validates the version
    /// via the file name as well as the sha if
    /// one is specified.
    pub async fn verify(&self, path: &Path) -> Option<()> {
        self.sha.as_ref()?;

        let mut file = tokio::fs::File::open(path).await.unwrap();
        self.copy(&mut file, &mut tokio::io::empty())
            .await
            .map(|_| ())
    }

    /// Copy the plugin from src to dest, validating the sha in the
    /// process.
    pub async fn copy<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
        &self,
        src: R,
        dest: &mut W,
    ) -> Option<u64> {
        let hasher = blake3::Hasher::new();
        let mut hasher = HashRead::new(src, hasher);
        let bytes = tokio::io::copy(&mut hasher, dest).await.unwrap();
        let output = hasher.finalize();

        if let Some(Blake3(sha)) = self.sha {
            // maybe consider constant time comparison fn
            if *output != sha {
                eprintln!("sha mismatch\n  got {:02X?}\n  exp {:02X?}", &*output, sha);
                return None;
            }
        }

        Some(bytes)
    }

    /// Returns how specific `rhs` is relative to `self`.
    ///
    /// An import is considered more specific if for each
    /// field that `self` is defined, `rhs` also has that
    /// field, the values are equal, and that `rhs` also
    /// specifies fields that `self` does not.
    ///
    /// If `self` and `rhs` both specify a field but they
    /// are not equal, then `None` is returned. If fields
    /// are set on each size that are not on the other,
    /// then `None` is returned.
    pub fn specificity(&self, rhs: &Import) -> Option<Ordering> {
        // is lhs greater than rhs?
        let mut left_view = false;
        // is rhs greater than lhs?
        let mut right_view = false;

        match (&self.plugin, &rhs.plugin) {
            (l, r) if l != r => return None,
            _ => {}
        };

        match (&self.registry, &rhs.registry) {
            (Some(_), None) => left_view = true,
            (None, Some(_)) => right_view = true,
            (Some(l), Some(r)) if l != r => return None,
            _ => {}
        };

        match (&self.version, &rhs.version) {
            (Some(l), Some(r)) if l != r => return None,
            (Some(_), None) => left_view = true,
            (None, Some(_)) => right_view = true,
            _ => {}
        };

        match (&self.sha, &rhs.sha) {
            (Some(l), Some(r)) if l != r => return None,
            (Some(_), None) => left_view = true,
            (None, Some(_)) => right_view = true,
            _ => {}
        };

        match (left_view, right_view) {
            (true, true) => None,
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => Some(Ordering::Equal),
        }
    }
}

impl PartialEq for Import {
    fn eq(&self, other: &Self) -> bool {
        self.plugin == other.plugin && self.version == other.version && self.sha == other.sha
    }
}

impl Eq for Import {}

impl PartialOrd for Import {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Import {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.plugin.cmp(&other.plugin) {
            std::cmp::Ordering::Equal => self.version.cmp(&other.version),
            other => other,
        }
    }
}

impl Serialize for Import {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let string = self.to_string();
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for Import {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[derive(Error, Debug, Diagnostic)]
#[error("failed to parse import")]
#[diagnostic(
    code(import::invalid_format),
    url(docsrs),
    help("check the documentation for the correct format")
)]
pub enum ImportParseError {
    SemverParseError(#[from] SemverParseError),
    Blake3ParseError(#[from] Blake3ParseError),
}

#[derive(Error, Debug, Diagnostic)]
#[error("failed to parse import")]
#[diagnostic(
    code(import::invalid_format),
    url(docsrs),
    help("check the documentation for the correct format")
)]
pub struct SemverParseError {
    #[source_code]
    src: String,

    err: semver::Error,

    #[label("{err}")]
    err_span: miette::SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("failed to parse import")]
#[diagnostic(
    code(import::invalid_format),
    url(docsrs),
    help("check the documentation for the correct format")
)]
pub struct Blake3ParseError {
    #[source_code]
    src: String,

    err: blake3::HexError,

    #[label("{err}")]
    err_span: miette::SourceSpan,
}

impl FromStr for Import {
    type Err = ImportParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rest = s.strip_suffix(".wasm").unwrap_or(s); // remove file extension
        let (registry, rest) = rest
            .split_once(REGISTRY_SEPARATOR)
            .map(|(registry, rest)| (Some(registry), rest))
            .unwrap_or((None, rest));
        let (sha, rest) = rest
            .rsplit_once(SHA_SEPERATOR)
            .map(|(rest, sha)| (Some(sha), rest))
            .unwrap_or((None, rest));
        let (package, version) = rest
            .split_once(VERSION_SEPARATOR)
            .map(|(package, version)| {
                version
                    .parse()
                    .map(|v| (package, Some(v)))
                    .map_err(|e| (e, version))
            })
            .unwrap_or(Ok((rest, None)))
            .map_err(|(e, version)| SemverParseError {
                err: e,
                src: s.to_string(),
                err_span: s
                    .find(version)
                    .map(|i| i..i + version.len())
                    .unwrap()
                    .into(),
            })?;

        Ok(Import {
            registry: registry.map(str::to_string),
            plugin: package.to_string(),
            version,
            sha: sha
                .map(|sha| {
                    Blake3::from_str(sha).map_err(|e| Blake3ParseError {
                        err: e,
                        err_span: s.find(sha).map(|i| i..i + s.len()).unwrap().into(),
                        src: s.to_string(),
                    })
                })
                .transpose()?,
        })
    }
}

impl Display for Import {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let registry = self
            .registry
            .as_deref()
            .map(|s| format!("{}{}", s, REGISTRY_SEPARATOR))
            .unwrap_or_default();
        let version = self
            .version
            .as_ref()
            .map(|v| format!("{}{}", VERSION_SEPARATOR, v))
            .unwrap_or_default();
        let sha = self
            .sha
            .as_ref()
            .map(|v| format!("{}{}", SHA_SEPERATOR, v.to_string()))
            .unwrap_or_default();

        write!(f, "{}{}{}{}", registry, self.plugin, version, sha)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Blake3([u8; blake3::OUT_LEN]);

impl FromStr for Blake3 {
    type Err = blake3::HexError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hash = s.strip_prefix("blake3:").unwrap();
        Ok(Self(blake3::Hash::from_hex(hash)?.as_bytes().to_owned()))
    }
}

impl Display for Blake3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hash = blake3::Hash::from_bytes(self.0);
        write!(f, "blake3:{}", hash.to_hex())
    }
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct PluginInstance {
    #[schemars(with = "String")]
    pub plugin: Import,
    pub config: Option<serde_json::Value>,
}

#[cfg(test)]
mod test {
    use super::*;
    use test_case::test_case;

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

    #[test_case(&[], "registry::package" => ImportAddResult::Added ; "basic case")]
    #[test_case(&["registry::package"], "registry::package" => ImportAddResult::Ignored; "duplicate")]
    #[test_case(&["registry::package"], "registry::package@1.0.0" => ImportAddResult::Replaced ; "add more specific version should overwrite")]
    #[test_case(&["registry::package@1.0.0"], "registry::package" => ImportAddResult::Ignored ; "add less specific version should be ignored")]
    #[test_case(&["package@1.0.1"], "package@1.0.2" => ImportAddResult::Added ; "incompatible imports should be added")]
    fn add_import(list: &[&str], import: &str) -> ImportAddResult {
        let mut config = LitehouseConfig {
            imports: list.iter().map(|s| s.parse().unwrap()).collect(),
            ..Default::default()
        };
        config.add_import(import.parse().unwrap())
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
