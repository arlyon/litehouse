pub use plugin_macro::{generate, generate_host};
// pub use wasmtime_wasi_http;
pub use wit_bindgen;

// pub mod http;

pub fn tracing_subscriber() {
    tracing_subscriber::fmt::init();
}
