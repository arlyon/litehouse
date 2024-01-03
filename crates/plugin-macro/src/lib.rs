use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, Ident, Token};

#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input with Punctuated::<Ident, Token![,]>::parse_terminated);
    let mut list = input.into_iter();
    let plugin_type = list.next().expect("need to specify a plugin type");

    // let config_type = list.next();

    let wit_dir = std::env!("WIT_DIR");
    let wit_dir = format!("{}/wit", wit_dir);
    quote! {
        plugin::wit_bindgen::generate!({
            path: #wit_dir,
            world: "plugin-host",
            runtime_path: "plugin::wit_bindgen::rt",
            exports: {
                "litehouse:plugin/plugin/runner": #plugin_type,
            },
            // with: {
                // "wasi:http/outgoing-handler@0.2.0-rc-2023-12-05": plugin::wasmtime_wasi_http::bindings::http::outgoing_handler,
                // "wasi:http/types@0.2.0-rc-2023-12-05": plugin::wasmtime_wasi_http::bindings::http::types,
            // }
        });


    }
    .into()
}

#[proc_macro]
pub fn generate_host(_input: TokenStream) -> TokenStream {
    let wit_dir = std::env!("WIT_DIR");
    let wit_dir = format!("{}/wit", wit_dir);

    quote! {
        wasmtime::component::bindgen!({
            async: true,
            path: #wit_dir,
            with: {
                "wasi:cli/environment": wasmtime_wasi::preview2::bindings::cli::environment,
                "wasi:cli/exit": wasmtime_wasi::preview2::bindings::cli::exit,
                "wasi:cli/stderr": wasmtime_wasi::preview2::bindings::cli::stderr,
                "wasi:cli/stdin": wasmtime_wasi::preview2::bindings::cli::stdin,
                "wasi:cli/stdout": wasmtime_wasi::preview2::bindings::cli::stdout,
                "wasi:clocks/monotonic-clock": wasmtime_wasi::preview2::bindings::clocks::monotonic_clock,
                "wasi:clocks/timezone": wasmtime_wasi::preview2::bindings::clocks::timezone,
                "wasi:clocks/wall-clock": wasmtime_wasi::preview2::bindings::clocks::wall_clock,
                "wasi:filesystem/preopens": wasmtime_wasi::preview2::bindings::filesystem::preopens,
                "wasi:filesystem/types": wasmtime_wasi::preview2::bindings::filesystem::types,
                "wasi:http/incoming-handler": wasmtime_wasi_http::bindings::http::incoming_handler,
                "wasi:http/outgoing-handler": wasmtime_wasi_http::bindings::http::outgoing_handler,
                "wasi:http/types": wasmtime_wasi_http::bindings::http::types,
                "wasi:io/streams": wasmtime_wasi::preview2::bindings::io::streams,
                "wasi:io/poll": wasmtime_wasi::preview2::bindings::io::poll,
                "wasi:random/random": wasmtime_wasi::preview2::bindings::random::random,
                "wasi:io/error": wasmtime_wasi::preview2::bindings::io::error,
            }
        });
    }.into()
}
