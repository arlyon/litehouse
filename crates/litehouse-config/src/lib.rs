mod hash_read;

use std::{collections::HashMap, fmt::Display, path::Path, str::FromStr};

use hash_read::HashRead;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};

const REGISTRY_SEPARATOR: &str = "::";
const VERSION_SEPARATOR: &str = "@";
const SHA_SEPERATOR: &str = "~";

#[derive(JsonSchema, Serialize, Deserialize, Debug, Default)]
pub struct LitehouseConfig {
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
}

impl LitehouseConfig {
    pub fn load() -> Result<Self, Error> {
        let file = std::fs::File::open("settings.json")?;
        let config: LitehouseConfig = serde_json::from_reader(&file)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), Error> {
        let file = std::fs::File::create("settings.json")?;
        serde_json::to_writer_pretty(&file, self)?;
        Ok(())
    }
}

pub fn directories() -> Option<directories_next::ProjectDirs> {
    directories_next::ProjectDirs::from("com", "litehouse", "litehouse")
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("serde error")]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug)]
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
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name, value) = s
            .split_once(':')
            .ok_or(())
            .map(|(name, value)| (name, value.to_string()))?;
        match name {
            "http-server" => value
                .parse()
                .map(Capability::HttpServer)
                .map_err(|_| ())
                .map_err(|_| ()),
            "http-client" => Ok(Capability::HttpClient(value)),
            _ => Err(()),
        }
    }
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
        Ok(s.parse().unwrap())
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
#[derive(Debug)]
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
                        &entry
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
        Ok(s.parse().unwrap())
    }
}

impl FromStr for Import {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_suffix(".wasm").unwrap_or(s); // remove file extension
        let (registry, rest) = s
            .split_once(REGISTRY_SEPARATOR)
            .map(|(registry, rest)| (Some(registry), rest))
            .unwrap_or((None, s));
        let (sha, rest) = rest
            .rsplit_once(SHA_SEPERATOR)
            .map(|(rest, sha)| (Some(sha), rest))
            .unwrap_or((None, rest));
        let (package, version) = rest
            .split_once(VERSION_SEPARATOR)
            .map(|(package, version)| (package, Some(version.parse().unwrap())))
            .unwrap_or((rest, None));

        Ok(Import {
            registry: registry.map(str::to_string),
            plugin: package.to_string(),
            version,
            sha: sha.map(|s| Blake3::from_str(s).unwrap()),
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

#[derive(Debug, PartialEq, Eq)]
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
}
