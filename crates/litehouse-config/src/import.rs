use std::{cmp::Ordering, fmt::Display, path::Path, str::FromStr};

use miette::Diagnostic;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::hash_read::HashRead;

const REGISTRY_SEPARATOR: &str = "::";
const VERSION_SEPARATOR: &str = "@";
const SHA_SEPERATOR: &str = "~";

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
            .map(|v| format!("{}{}", SHA_SEPERATOR, v))
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
