//! The manifest file format contains a plugin import alongside 1 or more instance configurations

use base64::Engine;

use crate::{import::ImportParseError, Import};
use std::{collections::HashMap, str::FromStr};

#[derive(Clone)]
pub enum ManifestImport {
    Import(Import),
    Manifest(Manifest),
}

#[derive(Debug, Clone)]
pub struct Manifest {
    pub import: Import,
    /// A map of instance names to their configuration
    pub config: HashMap<String, Option<serde_json::Value>>,
}

#[derive(thiserror::Error, Debug)]
pub enum ManifestImportError {
    #[error("when parsing import {0}")]
    Import(#[from] ImportParseError),
    #[error("expected a '#' between the import and manifest")]
    MissingDelimiter,
    #[error("malformed manifest")]
    InvalidBase64,
    #[error("manifest not valid json")]
    InvalidManifest,
}

impl FromStr for ManifestImport {
    type Err = ManifestImportError; // superset

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains('#') {
            Ok(ManifestImport::Manifest(Manifest::from_str(s)?))
        } else {
            Ok(ManifestImport::Import(Import::from_str(s)?))
        }
    }
}

impl FromStr for Manifest {
    type Err = ManifestImportError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (import, base64_json) = s
            .split_once('#')
            .ok_or(ManifestImportError::MissingDelimiter)?;
        let import = Import::from_str(import)?;
        let json = base64::prelude::BASE64_STANDARD
            .decode(base64_json)
            .map_err(|_| ManifestImportError::InvalidBase64)?;
        let config =
            serde_json::from_slice(&json).map_err(|_| ManifestImportError::InvalidManifest)?;

        Ok(Manifest { import, config })
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use crate::manifest::ManifestImportError;

    use super::Manifest;
    use test_case::test_case;

    #[test_case("tasmota#abcd" => matches Err(ManifestImportError::InvalidManifest))]
    #[test_case("tasmota" => matches Err(ManifestImportError::MissingDelimiter))]
    fn parse(import: &str) -> Result<Manifest, ManifestImportError> {
        Manifest::from_str(import)
    }
}
