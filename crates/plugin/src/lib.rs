pub use plugin_macro::{generate, generate_host, Config};
// pub use wasmtime_wasi_http;
pub use schemars::{self, schema_for, JsonSchema};
pub use serde_json;
use tracing::dispatcher::set_global_default;
use tracing_subscriber::{fmt, EnvFilter};
pub use wit_bindgen;

// pub mod http;

pub fn tracing_subscriber() {
    let subscriber = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    if let Err(_) = set_global_default(subscriber.into()) {
        tracing::trace!("unable to register trace module")
    }
}
