//! Parallelism strategies for Litehouse plugins.
//!
//! This module explains the different parallelism strategies available in Litehouse for running plugins.
//! These strategies determine how plugin instances are sandboxed and executed in parallel.

use serde::{Deserialize, Serialize};

/// Defines the strategy to use for sandboxing plugins.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum SandboxStrategy {
    /// All plugins are run in the same storage sandbox.
    #[serde(rename = "global")]
    Global,
    /// Each plugin type is run in its own storage sandbox.
    #[serde(rename = "plugin")]
    Plugin,
    /// Each plugin instance is run in its own storage sandbox.
    #[serde(rename = "instance")]
    Instance,
}

impl SandboxStrategy {
    /// Provides a detailed explanation of the sandbox strategy.
    pub fn description(&self) -> &'static str {
        match self {
            SandboxStrategy::Global => "All plugins are run in the same storage sandbox. This strategy is useful for environments with limited resources, as it minimizes the number of sandboxes required. However, it also means that all plugins share the same storage space, which can lead to conflicts and security concerns.",
            SandboxStrategy::Plugin => "Each plugin type is run in its own storage sandbox. This strategy provides a balance between resource usage and isolation. Plugins of the same type share a sandbox, which can be useful for sharing state between instances of the same plugin.",
            SandboxStrategy::Instance => "Each plugin instance is run in its own storage sandbox. This strategy provides the highest level of isolation, ensuring that each plugin instance has its own separate storage space. It is the default strategy and recommended for most use cases.",
        }
    }
}

/// Example usage of sandbox strategies.
///
/// This function demonstrates how to select and use a sandbox strategy for a hypothetical plugin.
pub fn example_usage() {
    let strategy = SandboxStrategy::Instance;
    println!("Selected sandbox strategy: {:?}", strategy);
    println!("Description: {}", strategy.description());
}
