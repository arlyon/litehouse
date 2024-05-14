//! A macro for generating Litehouse plugins.
//!
//! This crate provides a procedural macro to easily generate boilerplate code required for creating Litehouse plugins.

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, Ident, Token};

#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input with Punctuated::<Ident, Token![,]>::parse_terminated);
    let mut list = input.into_iter();
    let plugin_type = list.next().expect("need to specify a plugin type");

    let config = list.next();
    let (struct_def, ident, config) = config
        .map(|ident| {
            (
                None,
                ident.clone(),
                quote! {litehouse_plugin::serde_json::to_string(&litehouse_plugin::schemars::schema_for!(#ident)).ok()},
            )
        })
        .unwrap_or_else(|| {
            (
                Some(quote! {struct Config;}),
                Ident::new("Config", Span::call_site()),
                quote! {None},
            )
        });

    let impl_block = quote! {
        #struct_def

        // TODO: post-build step to inject metadata here
        #[no_mangle]
        #[link_section = "litehouse_metadata"]
        pub static METADATA: [u8; 0] = [];

        impl exports::litehouse::plugin::plugin::Guest for #ident {
            type Runner = #plugin_type;
            fn get_metadata() -> exports::litehouse::plugin::plugin::Metadata {
                exports::litehouse::plugin::plugin::Metadata {
                    identifier: core::env!("CARGO_PKG_NAME").to_string(),
                    version: core::env!("CARGO_PKG_VERSION").to_string(),
                    readme: Some(include_str!(concat!("../",  core::env!("CARGO_PKG_README"))).to_string()),
                    config_schema: #config,
                    capabilities: vec![],
                    description: Some(core::env!("CARGO_PKG_DESCRIPTION").to_string()),
                    author: Some(core::env!("CARGO_PKG_AUTHORS").to_string()),
                    homepage: Some(core::env!("CARGO_PKG_HOMEPAGE").to_string()),
                    source: Some(core::env!("CARGO_PKG_REPOSITORY").to_string()),
                }
            }
        }

        export!(#ident);
    };

    let wit_dir = std::env!("WIT_DIR");
    let wit_dir = format!("{}/wit", wit_dir);
    quote! {
        litehouse_plugin::wit_bindgen::generate!({
            path: #wit_dir,
            world: "plugin-host",
            runtime_path: "litehouse_plugin::wit_bindgen::rt",
            std_feature,
        });

        #impl_block
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
            path: #wit_dir
        });
    }
    .into()
}

#[proc_macro_derive(Config)]
pub fn config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::ItemStruct);
    let out = quote! {
        #[derive(litehouse_plugin::JsonSchema)]
        #input
    };
    println!("{:#?}", out);
    out.into()
}
