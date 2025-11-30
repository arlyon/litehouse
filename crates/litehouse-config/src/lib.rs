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
    collections::{HashMap, hash_map::Entry},
    num::NonZeroU8,
};

use miette::{NamedSource, SourceOffset};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use capabilities::Capability;
pub use import::Import;
pub use manifest::{Manifest, ManifestImport};
pub use parallelism::SandboxStrategy;

#[derive(JsonSchema, Serialize, Deserialize, Debug, Default)]
pub struct LitehouseConfig {
    #[serde(rename = "$schema", default = "default_schema")]
    pub schema: String,
    /// The list of plugins to use in this litehouse
    pub plugins: HashMap<String, PluginConfig>,
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
    /// Configuration to connect to a litehouse broker
    #[serde(default, skip_serializing_if = "is_default")]
    pub broker: Option<Broker>,
}

fn default_schema() -> String {
    "./schema.json".to_string()
}

#[derive(JsonSchema, Serialize, Deserialize, Debug, PartialEq)]
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
                        let v_old = e.insert(PluginConfig {
                            plugin: manifest.import.clone(),
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
                        config,
                    });
                    ManifestAddResult::Added(k)
                }
            })
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
pub struct Registry {
    /// The local name of the registry
    pub name: String,
    /// The url to use for interacting with the registry
    pub url: String,
}

/// A combination of a plugin import and its configuration.
#[derive(JsonSchema, Serialize, Deserialize, Debug)]
pub struct PluginConfig {
    #[schemars(with = "String")]
    pub plugin: Import,
    pub config: Option<serde_json::Value>,
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

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
