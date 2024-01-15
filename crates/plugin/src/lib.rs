pub use plugin_macro::{generate, generate_host, Config};
// pub use wasmtime_wasi_http;
pub use schemars::{self, schema_for, JsonSchema};
pub use serde_json;
pub use wit_bindgen;

// pub mod http;

pub fn tracing_subscriber() {
    tracing_subscriber::fmt::init();
}
