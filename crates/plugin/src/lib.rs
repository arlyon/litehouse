//! Library for generating Litehouse plugins.
//!
//! This crate provides utilities and macros to facilitate the creation of plugins for the Litehouse home automation system. It includes functionality for schema generation, serialization, and integration with the Litehouse plugin host.

pub use litehouse_plugin_macro::{generate, generate_host, Config};
// pub use wasmtime_wasi_http;
pub use schemars::{self, schema_for, JsonSchema};
pub use serde_json;
use tracing::dispatcher::set_global_default;
use tracing_subscriber::{fmt, EnvFilter};
pub use wit_bindgen;

// pub mod http;

/// Initializes the tracing subscriber for logging within plugins.
///
/// This function sets up the environment filter and logging format for the plugin's tracing subscriber, allowing for customizable log output.
pub fn tracing_subscriber() {
    let subscriber = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    if let Err(_err) = set_global_default(subscriber.into()) {
        tracing::trace!("unable to register trace module")
    }
}
