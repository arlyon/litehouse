use litehouse_contracts::{CONTRACT_ARTIFACT_MANIFEST, management::MANAGEMENT_OPERATIONS_V1_JSON};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::model::{
    CEL_PROFILE, COMPILER_PROFILE, CONTRACT_CATALOG_ARTIFACT, MANAGEMENT_OPERATIONS_ARTIFACT,
    PORT_CATALOG_ARTIFACT, RUNTIME_PROFILE, SCHEMA_URL, TIMEZONE_DATABASE,
};

const QUANTITY_PROFILE_BYTES: &[u8] =
    include_bytes!("../../litehouse-contracts/packages/quantity/1.0.0/quantity/profiles.json");
const RUNTIME_PROFILE_BYTES: &[u8] =
    include_bytes!("../../litehouse-contracts/packages/runtime-profile/1.0.0/profile.json");
const COMPILER_PROFILE_BYTES: &[u8] = b"litehouse:rule-compiler@1\nrule-json=1.0.0\ncanonical-semantic=rfc8785-jcs-64k\ncompiler=t8.1c\n";
const CEL_PROFILE_BYTES: &[u8] = b"litehouse-cel@1\ncel=0.14.0\nmode=bounded-dynamic-interpreter\ninputs=declared-immutable-only\ncost=closed-ast-precharged\nmemory=checked-ast-plus-runtime-values\nchrono=timestamp-boundary-only\n";
const TIMEZONE_PROFILE_BYTES: &[u8] =
    b"jiff-tzdb@0.1.8+2026c\njiff=0.2.32\ntzdb-source=jiff-tzdb-bundle-always\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ArtifactDigests {
    pub schema: String,
    pub profile: String,
    pub compiler: String,
    pub cel: String,
    pub quantity_catalog: String,
    pub runtime_profile: String,
    pub timezone_database: String,
    pub contract_catalog: String,
    pub management_operations: String,
    pub port_catalog: String,
}

#[derive(Serialize)]
struct EvaluationArtifacts<'a> {
    #[serde(rename = "$schema")]
    schema: &'static str,
    #[serde(rename = "$id")]
    id: &'static str,
    version: u8,
    rule_schema: LockedArtifact<'a>,
    entries: Vec<LockedArtifact<'a>>,
}

#[derive(Serialize)]
struct LockedArtifact<'a> {
    id: &'a str,
    sha256: String,
    source: &'static str,
}

pub(crate) fn generated_manifest_bytes(rule_schema: &[u8], port_catalog: &[u8]) -> Vec<u8> {
    let value = EvaluationArtifacts {
        schema: "https://json-schema.org/draft/2020-12/schema",
        id: "https://schemas.litehouse.dev/rule-ir/v1.0.0/evaluation-artifacts-v1.json",
        version: 1,
        rule_schema: artifact(SCHEMA_URL, rule_schema, "generated_rule_schema"),
        entries: vec![
            artifact(CEL_PROFILE, CEL_PROFILE_BYTES, "explicit_versioned_profile"),
            artifact(
                CONTRACT_CATALOG_ARTIFACT,
                CONTRACT_ARTIFACT_MANIFEST,
                "litehouse_contracts_generated_abi_manifest",
            ),
            artifact(
                MANAGEMENT_OPERATIONS_ARTIFACT,
                MANAGEMENT_OPERATIONS_V1_JSON,
                "management_operations_v1_json",
            ),
            artifact(
                COMPILER_PROFILE,
                COMPILER_PROFILE_BYTES,
                "explicit_versioned_profile",
            ),
            artifact(
                "litehouse:quantity-catalog@1",
                QUANTITY_PROFILE_BYTES,
                "quantity_profiles_v1_json",
            ),
            artifact(
                RUNTIME_PROFILE,
                RUNTIME_PROFILE_BYTES,
                "runtime_profile_v1_json",
            ),
            artifact(
                PORT_CATALOG_ARTIFACT,
                port_catalog,
                "generated_node_port_catalog",
            ),
            artifact(
                TIMEZONE_DATABASE,
                TIMEZONE_PROFILE_BYTES,
                "explicit_versioned_profile",
            ),
        ],
    };
    let mut bytes = serde_json::to_vec_pretty(&value).expect("artifact manifest must serialize");
    bytes.push(b'\n');
    bytes
}

pub(crate) fn digests(rule_schema: &[u8], port_catalog: &[u8]) -> ArtifactDigests {
    let manifest = generated_manifest_bytes(rule_schema, port_catalog);
    ArtifactDigests {
        schema: sha256(rule_schema),
        profile: sha256(&manifest),
        compiler: sha256(COMPILER_PROFILE_BYTES),
        cel: sha256(CEL_PROFILE_BYTES),
        quantity_catalog: sha256(QUANTITY_PROFILE_BYTES),
        runtime_profile: sha256(RUNTIME_PROFILE_BYTES),
        timezone_database: sha256(TIMEZONE_PROFILE_BYTES),
        contract_catalog: sha256(CONTRACT_ARTIFACT_MANIFEST),
        management_operations: sha256(MANAGEMENT_OPERATIONS_V1_JSON),
        port_catalog: sha256(port_catalog),
    }
}

fn artifact<'a>(id: &'a str, bytes: &[u8], source: &'static str) -> LockedArtifact<'a> {
    LockedArtifact {
        id,
        sha256: sha256(bytes),
        source,
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
