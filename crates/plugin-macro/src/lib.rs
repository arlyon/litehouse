//! A macro for generating Litehouse plugins.
//!
//! This crate provides a procedural macro to easily generate boilerplate code required for creating Litehouse plugins.

use heck::{ToKebabCase, ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Data, DeriveInput, Fields, Ident, LitStr, Token, parse_macro_input, punctuated::Punctuated,
};

#[proc_macro]
pub fn generate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input with Punctuated::<Ident, Token![,]>::parse_terminated);
    let mut list = input.into_iter();
    let plugin_type = list.next().expect("need to specify a plugin type");

    let config = list.next();
    let (config_schema, config_annotations) = config
        .map(|ident| {
            (
                quote! {
                    Some(<#ident as litehouse_plugin::contracts::config::ConfigMetadata>::schema_json())
                },
                quote! {
                    <#ident as litehouse_plugin::contracts::config::ConfigMetadata>::annotations()
                },
            )
        })
        .unwrap_or_else(|| (quote! {None}, quote! {Vec::new()}));

    let impl_block = quote! {
        struct DefaultA;
        impl exports::litehouse::plugin::plugin::GuestA for DefaultA {
            fn new() -> Self {
                Self
            }
        }

        struct DefaultB;
        impl exports::litehouse::plugin::plugin::GuestB for DefaultB {
            fn new() -> Self {
                Self
            }
        }

        struct RunnerInterface;
        impl exports::litehouse::plugin::plugin::Guest for RunnerInterface {
            type Runner = #plugin_type;
            type A = DefaultA;
            type B = DefaultB;
            fn get_metadata() -> Vec<u8> {
                use litehouse_plugin::contracts::{
                    identity::{PackageIdentity, PackageRequirement, Version},
                    metadata::{HostFacilityRequests, RuntimeMetadataV1},
                };

                let version = Version {
                    major: core::env!("CARGO_PKG_VERSION_MAJOR").parse().expect("Cargo major version"),
                    minor: core::env!("CARGO_PKG_VERSION_MINOR").parse().expect("Cargo minor version"),
                    patch: core::env!("CARGO_PKG_VERSION_PATCH").parse().expect("Cargo patch version"),
                };
                RuntimeMetadataV1 {
                    package: PackageIdentity::new("litehouse", core::env!("CARGO_PKG_NAME"), version)
                        .expect("Cargo package name must be a bounded kebab identifier"),
                    runtime_profile: PackageRequirement {
                        namespace: "litehouse".to_string(),
                        name: "runtime-profile".to_string(),
                        minimum: Version::V1,
                    },
                    description: Some(core::env!("CARGO_PKG_DESCRIPTION").to_string()),
                    provides: Vec::new(),
                    requires: Vec::new(),
                    host_facilities: HostFacilityRequests {
                        exact_http_origins: Vec::new(),
                        exact_tcp_origins: Vec::new(),
                        clocks: false,
                        random: false,
                        tracing: true,
                    },
                    config_schema: #config_schema,
                    config_annotations: #config_annotations,
                }
                .encode_section()
                .expect("generated plugin metadata must satisfy the bounded v1 contract")
            }
        }

        export!(RunnerInterface);
    };

    let wit_dir = std::env!("WIT_DIR");
    let wit_dir = format!("{}/wit", wit_dir);
    quote! {
        litehouse_plugin::wit_bindgen::generate!({
            path: #wit_dir,
            world: "plugin-host",
            runtime_path: "litehouse_plugin::wit_bindgen::rt",
            generate_all,
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
            path: #wit_dir,
            exports: { default: async },
            require_store_data_send: true,
            with: {
                "wasi:http/client": wasmtime_wasi_http::p3::bindings::http::client,
                "wasi:http/types": wasmtime_wasi_http::p3::bindings::http::types,
                "wasi:sockets/types": wasmtime_wasi::p3::bindings::sockets::types,
            }
        });
    }
    .into()
}

#[proc_macro_derive(Config, attributes(litehouse))]
pub fn config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match derive_config_metadata(input) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

fn derive_config_metadata(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let rename_all = serde_rename_all(&input.attrs)?;
    let name = input.ident;
    let fields = match input.data {
        Data::Struct(data) => match data.fields {
            Fields::Named(fields) => fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Config requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Config can only be derived for structs",
            ));
        }
    };

    let mut annotations = Vec::new();
    for field in fields {
        let serialized_name = serde_field_rename(&field.attrs)?;
        let field_name = field.ident.expect("named fields have identifiers");
        let field_name_text = field_name.to_string();
        let serialized_name = serialized_name.unwrap_or_else(|| {
            apply_rename_rule(&field_name_text, rename_all.as_deref()).unwrap_or(field_name_text)
        });
        let pointer = format!("/{}", serialized_name.replace('~', "~0").replace('/', "~1"));
        let mut environment_secret: Option<String> = None;
        let mut secret_encoding = String::from("utf8");
        let mut private_local: Option<String> = None;

        for attribute in field
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("litehouse"))
        {
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("environment_secret") {
                    environment_secret = Some(meta.value()?.parse::<LitStr>()?.value());
                    return Ok(());
                }
                if meta.path.is_ident("secret_encoding") {
                    secret_encoding = meta.value()?.parse::<LitStr>()?.value();
                    return Ok(());
                }
                if meta.path.is_ident("private_local") {
                    private_local = Some(meta.value()?.parse::<LitStr>()?.value());
                    return Ok(());
                }
                Err(meta.error("unknown litehouse config annotation"))
            })?;
        }

        if environment_secret.is_some() && private_local.is_some() {
            return Err(syn::Error::new_spanned(
                field_name,
                "a config field cannot be both environment-secret and private-local",
            ));
        }
        if let Some(token) = environment_secret {
            if !valid_field_token(&token) {
                return Err(syn::Error::new_spanned(
                    field_name,
                    "environment_secret must match [A-Z][A-Z0-9_]{0,63}",
                ));
            }
            let encoding = match secret_encoding.as_str() {
                "utf8" => quote! { litehouse_plugin::contracts::config::SecretEncoding::Utf8 },
                "base64" => {
                    quote! { litehouse_plugin::contracts::config::SecretEncoding::Base64 }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        field_name,
                        "secret_encoding must be utf8 or base64",
                    ));
                }
            };
            annotations.push(quote! {
                litehouse_plugin::contracts::config::AnnotatedConfigField {
                    json_pointer: #pointer.to_string(),
                    annotation: litehouse_plugin::contracts::config::ConfigFieldAnnotation::EnvironmentSecret {
                        field_token: #token.to_string(),
                        encoding: #encoding,
                    },
                }
            });
        } else if let Some(projection) = private_local {
            let projection = match projection.as_str() {
                "configured-status" => quote! {
                    litehouse_plugin::contracts::config::PrivateProjection::ConfiguredStatus
                },
                "omitted" => {
                    quote! { litehouse_plugin::contracts::config::PrivateProjection::Omitted }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        field_name,
                        "private_local must be configured-status or omitted",
                    ));
                }
            };
            annotations.push(quote! {
                litehouse_plugin::contracts::config::AnnotatedConfigField {
                    json_pointer: #pointer.to_string(),
                    annotation: litehouse_plugin::contracts::config::ConfigFieldAnnotation::PrivateLocal {
                        projection: #projection,
                    },
                }
            });
        }
    }

    Ok(quote! {
        impl litehouse_plugin::contracts::config::ConfigMetadata for #name {
            fn schema_json() -> String {
                litehouse_plugin::serde_json::to_string(
                    &litehouse_plugin::schemars::schema_for!(#name)
                ).expect("config schema must serialize")
            }

            fn annotations() -> Vec<litehouse_plugin::contracts::config::AnnotatedConfigField> {
                vec![#(#annotations),*]
            }
        }
    })
}

fn serde_rename_all(attributes: &[syn::Attribute]) -> syn::Result<Option<String>> {
    let mut rule = None;
    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("serde"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename_all") {
                let value = meta.value()?.parse::<LitStr>()?.value();
                if apply_rename_rule("field_name", Some(&value)).is_none() {
                    return Err(meta.error("unsupported serde rename_all rule for Config"));
                }
                rule = Some(value);
                return Ok(());
            }
            consume_nested_meta(meta)
        })?;
    }
    Ok(rule)
}

fn serde_field_rename(attributes: &[syn::Attribute]) -> syn::Result<Option<String>> {
    let mut rename = None;
    for attribute in attributes
        .iter()
        .filter(|attribute| attribute.path().is_ident("serde"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                rename = Some(meta.value()?.parse::<LitStr>()?.value());
                return Ok(());
            }
            consume_nested_meta(meta)
        })?;
    }
    Ok(rename)
}

fn consume_nested_meta(meta: syn::meta::ParseNestedMeta<'_>) -> syn::Result<()> {
    if meta.input.peek(Token![=]) {
        let _: syn::Expr = meta.value()?.parse()?;
    } else if meta.input.peek(syn::token::Paren) {
        meta.parse_nested_meta(consume_nested_meta)?;
    }
    Ok(())
}

fn apply_rename_rule(value: &str, rule: Option<&str>) -> Option<String> {
    Some(match rule? {
        "lowercase" => value.to_ascii_lowercase(),
        "UPPERCASE" => value.to_ascii_uppercase(),
        "PascalCase" => value.to_upper_camel_case(),
        "camelCase" => value.to_lower_camel_case(),
        "snake_case" => value.to_snake_case(),
        "SCREAMING_SNAKE_CASE" => value.to_shouty_snake_case(),
        "kebab-case" => value.to_kebab_case(),
        "SCREAMING-KEBAB-CASE" => value.to_shouty_snake_case().replace('_', "-"),
        _ => return None,
    })
}

fn valid_field_token(token: &str) -> bool {
    let mut bytes = token.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    token.len() <= 64
        && first.is_ascii_uppercase()
        && bytes.all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}
