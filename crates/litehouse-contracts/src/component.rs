//! Structural Component Model admission against the frozen T1 runtime profile.

use serde::Deserialize;
use std::{collections::BTreeSet, error::Error, fmt};
use wit_component::DecodedWasm;
use wit_parser::WorldItem;

const PROFILE_JSON: &str = include_str!("../packages/runtime-profile/1.0.0/profile.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    Plugin,
    Provider,
    QuantityEngine,
}

impl ComponentKind {
    fn required_export(self) -> &'static str {
        match self {
            Self::Plugin => "litehouse:plugin/plugin@1.0.0",
            Self::Provider => "litehouse:provider/provider@1.0.0",
            Self::QuantityEngine => "litehouse:quantity/engine@1.0.0",
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RuntimeProfile {
    identity: String,
    component_model_features: Vec<String>,
    wasmtime_version: String,
    sandbox: String,
    accepted_imports: Vec<AcceptedImport>,
    limits: RuntimeLimits,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptedImport {
    package: String,
    version: String,
    interfaces: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RuntimeLimits {
    maximum_call_milliseconds: u32,
    maximum_in_flight_per_instance: u32,
    maximum_waiting_per_instance: u32,
    maximum_linear_memory_bytes: u64,
    maximum_resource_table_entries: u32,
}

pub fn validate_component(bytes: &[u8], kind: ComponentKind) -> Result<(), ComponentProfileError> {
    let profile: RuntimeProfile =
        serde_json::from_str(PROFILE_JSON).map_err(|_| ComponentProfileError::InvalidProfile)?;
    validate_profile_header(&profile)?;
    let allowed = profile
        .accepted_imports
        .iter()
        .flat_map(|entry| {
            entry
                .interfaces
                .iter()
                .map(|interface| format!("{}/{}@{}", entry.package, interface, entry.version))
        })
        .collect::<BTreeSet<_>>();

    let DecodedWasm::Component(resolve, world_id) =
        wit_component::decode(bytes).map_err(|_| ComponentProfileError::InvalidComponent)?
    else {
        return Err(ComponentProfileError::NotAComponent);
    };
    let world = &resolve.worlds[world_id];
    for item in world.imports.values() {
        let WorldItem::Interface { id, .. } = item else {
            return Err(ComponentProfileError::DirectWorldImport);
        };
        let import = interface_identity(&resolve, *id)?;
        if !allowed.contains(&import) {
            return Err(ComponentProfileError::ImportNotAllowed(import));
        }
    }

    let exports = world
        .exports
        .values()
        .filter_map(|item| match item {
            WorldItem::Interface { id, .. } => interface_identity(&resolve, *id).ok(),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let required = kind.required_export();
    if !exports.contains(required) {
        return Err(ComponentProfileError::MissingExport(required));
    }
    Ok(())
}

fn validate_profile_header(profile: &RuntimeProfile) -> Result<(), ComponentProfileError> {
    if profile.identity != "litehouse:runtime-profile@1.0.0"
        || profile.wasmtime_version != "46.0.1"
        || profile.sandbox != "per-instance-store"
        || !profile
            .component_model_features
            .iter()
            .map(String::as_str)
            .eq(["component-model", "async-functions", "concurrent-calls"])
        || profile.limits.maximum_call_milliseconds != 30_000
        || profile.limits.maximum_in_flight_per_instance != 32
        || profile.limits.maximum_waiting_per_instance != 64
        || profile.limits.maximum_linear_memory_bytes != 67_108_864
        || profile.limits.maximum_resource_table_entries != 4_096
    {
        return Err(ComponentProfileError::InvalidProfile);
    }
    Ok(())
}

fn interface_identity(
    resolve: &wit_parser::Resolve,
    id: wit_parser::InterfaceId,
) -> Result<String, ComponentProfileError> {
    let interface = &resolve.interfaces[id];
    let name = interface
        .name
        .as_deref()
        .ok_or(ComponentProfileError::AnonymousInterface)?;
    let package_id = interface
        .package
        .ok_or(ComponentProfileError::AnonymousInterface)?;
    let package = &resolve.packages[package_id].name;
    let version = package
        .version
        .as_ref()
        .ok_or(ComponentProfileError::UnversionedInterface)?;
    Ok(format!(
        "{}:{}/{}@{}",
        package.namespace, package.name, name, version
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentProfileError {
    InvalidProfile,
    InvalidComponent,
    NotAComponent,
    DirectWorldImport,
    AnonymousInterface,
    UnversionedInterface,
    ImportNotAllowed(String),
    MissingExport(&'static str),
}

impl fmt::Display for ComponentProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile => formatter.write_str("invalid frozen runtime profile"),
            Self::InvalidComponent => formatter.write_str("malformed component binary"),
            Self::NotAComponent => formatter.write_str("input is not a Component Model component"),
            Self::DirectWorldImport => {
                formatter.write_str("direct world-function imports are not allowed")
            }
            Self::AnonymousInterface => formatter.write_str("anonymous interface is not allowed"),
            Self::UnversionedInterface => {
                formatter.write_str("unversioned interface is not allowed")
            }
            Self::ImportNotAllowed(import) => {
                write!(formatter, "component import is not allowed: {import}")
            }
            Self::MissingExport(export) => {
                write!(formatter, "component is missing required export {export}")
            }
        }
    }
}

impl Error for ComponentProfileError {}

#[cfg(test)]
mod tests {
    use super::ComponentKind;

    #[test]
    fn provider_admission_requires_the_exact_provider_interface() {
        assert_eq!(
            ComponentKind::Provider.required_export(),
            "litehouse:provider/provider@1.0.0"
        );
    }
}
