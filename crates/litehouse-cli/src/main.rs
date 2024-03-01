mod cache_layer;
mod registry;

use clap::{Parser, Subcommand};
use eyre::Context;
use litehouse_config::{Import, LitehouseConfig};
use registry::{Download, Registry, Upload};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::process::Command;

const WASM_PROCESS_FILE: &[u8] =
    include_bytes!("../../litehouse/wasi_snapshot_preview1.reactor.wasm");

#[derive(Parser)]
struct Opts {
    #[command(subcommand)]
    command: Options,
}

#[derive(Subcommand)]
enum Options {
    /// Publish a package to the registry. Run this in the root of your package.
    Publish {
        package: String,
        #[clap(long)]
        access_key: String,
        #[clap(long)]
        secret_key: String,
    },
    /// Fetch packages from the registry, based on the imports in your settings file.
    Fetch {
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
    },
    /// Build a package and write the wasm file to the specified path.
    Build {
        package: String,
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
        #[clap(long, default_value_t = false)]
        debug: bool,
    },
    /// Search for a package in the registry.
    Search {
        /// The plugin to search for.
        query: Option<String>,
    },
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let opts = Opts::parse();

    let registry = Registry::build("default".to_string());

    match opts.command {
        Options::Publish {
            package,
            access_key,
            secret_key,
        } => {
            publish(
                package,
                &registry
                    .with_upload(access_key, secret_key)
                    .build()
                    .await
                    .wrap_err("can't download")?,
            )
            .await
        }
        Options::Fetch { wasm_path } => {
            let cache_dir = litehouse_config::directories().map(|d| d.cache_dir().to_owned());

            fetch(
                &registry
                    .with_download(wasm_path, cache_dir)
                    .build()
                    .await
                    .wrap_err("can't fetch")?,
            )
            .await
        }
        Options::Build {
            wasm_path,
            package,
            debug,
        } => build(&package, &wasm_path, debug).await,
        Options::Search { query } => {
            let prefix = query.map(|q| Import {
                plugin: q,
                registry: None,
                version: None,
            });
            let registry = registry.build().await.wrap_err("can't search")?;
            let results = registry.list(prefix.as_ref()).await;
            for (import, _) in results {
                println!("{}", import.to_string());
            }
        }
    }

    Ok(())
}

async fn build_in_temp(package: &str, release: bool) -> Option<(Import, PathBuf)> {
    let workspaces_json = Command::new("cargo")
        .arg("metadata")
        .output()
        .await
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&workspaces_json.stdout).unwrap();

    let members: HashMap<&str, (&str, &str)> = data["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let v = v.as_str().unwrap();
            let (name, rest) = v.split_once(' ').unwrap();
            let (version, rest) = rest.split_once(' ').unwrap();
            let path = rest
                .strip_prefix("(path+file://")
                .unwrap()
                .strip_suffix(')')
                .unwrap();
            (name, (version, path))
        })
        .collect();

    let (version, _path) = members
        .get(package)
        .expect("Package not found in workspace");

    // run cargo build
    let out = Command::new("cargo")
        .args(["build", "--target", "wasm32-wasi", "-p", &package])
        .args(if release { &["--release"][..] } else { &[] })
        .status()
        .await
        .unwrap();

    if !out.success() {
        return None;
    }

    let import = Import {
        registry: None,
        plugin: package.to_owned(),
        version: Some(version.parse().unwrap()),
    };

    // write the wasm file to a temp dir
    let tmp = std::env::temp_dir().join("litehouse");
    let wasi_path = tmp.join("wasi_snapshot_preview1.wasm");
    let out_path = tmp.join(import.file_name());
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(&wasi_path, WASM_PROCESS_FILE).unwrap();

    // run wasm-tools against the wasm file
    let out = Command::new("wasm-tools")
        .args([
            "component",
            "new",
            &format!("./target/wasm32-wasi/release/{}.wasm", package),
            "--adapt",
            wasi_path.to_str().unwrap(),
            "-o",
            out_path.to_str().unwrap(),
        ])
        .status()
        .await
        .unwrap();

    if !out.success() {
        return None;
    }

    Some((import, out_path))
}

async fn build(package: &str, wasm_path: &Path, debug: bool) {
    let (import, path) = build_in_temp(package, !debug).await.unwrap();
    tokio::fs::create_dir_all(wasm_path).await.unwrap();
    let dest_file = wasm_path.join(import.file_name());
    tokio::fs::copy(&path, &dest_file).await.unwrap();
}

async fn publish<D>(package: String, op: &Registry<Upload, D>) {
    let (import, path) = build_in_temp(&package, true).await.unwrap();

    let success = op.publish(&import, &path).await;
    if success {
        println!("Published {}", import.file_name());
    } else {
        println!("Failed to publish {}", import.file_name());
    }
}

async fn fetch<U>(op: &Registry<U, Download>) {
    let config = LitehouseConfig::load().unwrap();
    for file in config.imports {
        op.download_package(file).await;
    }
}
