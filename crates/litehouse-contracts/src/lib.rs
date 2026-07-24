//! Litehouse's versioned contract kernel.
//!
//! This crate owns deterministic WIT artifacts, immutable quantity semantics,
//! typed plugin metadata, config annotations, and the closed management
//! operation inventory. It does not route endpoints or perform I/O.

pub mod component;
pub mod config;
pub mod identity;
pub mod management;
pub mod metadata;
pub mod quantity;

/// A deterministic binary WIT package emitted into Cargo's `OUT_DIR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractArtifact {
    pub package: &'static str,
    pub version: &'static str,
    pub abi_sha256: &'static str,
    pub source_sha256: &'static str,
    pub size: usize,
    pub source_size: usize,
}

include!(concat!(env!("OUT_DIR"), "/contract_artifacts.rs"));

/// JSON inventory of all deterministic binary WIT artifacts.
pub const CONTRACT_ARTIFACT_MANIFEST: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/contract-artifacts.json"));
