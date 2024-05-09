//! Parallelism strategies for Litehouse plugins.
//!
//! This module explains the different parallelism strategies available in Litehouse for running plugins.
//! These strategies determine how plugin instances are sandboxed and executed in parallel.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Defines the strategy to use for sandboxing plugins.
#[derive(Debug, Serialize, Deserialize, PartialEq, Default, JsonSchema)]
pub enum SandboxStrategy {
    /// All plugins are run in the same storage sandbox.
    #[serde(rename = "global")]
    Global,
    /// Each plugin type is run in its own storage sandbox.
    #[serde(rename = "plugin")]
    Plugin,
    /// Each plugin instance is run in its own storage sandbox.
    #[serde(rename = "instance")]
    #[default]
    Instance,
}
