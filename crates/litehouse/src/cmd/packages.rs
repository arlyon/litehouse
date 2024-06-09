use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use futures::FutureExt;
use litehouse_config::{Import, LitehouseConfig};
use litehouse_plugin::serde_json;
use tokio::process::Command;
use tokio_stream::StreamExt;

use litehouse_registry::{Download, Registry, Upload};

const WASM_PROCESS_FILE: &[u8] =
    include_bytes!("../../../litehouse/wasi_snapshot_preview1.reactor.wasm");

pub async fn build_in_temp(package: &str, release: bool) -> Option<(Import, PathBuf)> {
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
            if let Some(rest) = v.strip_prefix("path+file") {
                // macOS, format path+file:///$PATH/$NAME#$VERSION
                let (path, rest) = rest.rsplit_once('/').unwrap();
                let (name, version) = rest.split_once('#').unwrap();
                (name, (version, path))
            } else {
                // linux, format $NAME $VERSION (path+file://$PATH)
                let (name, rest) = v.split_once(' ').unwrap();
                let (version, rest) = rest.split_once(' ').unwrap();
                let path = rest
                    .strip_prefix("(path+file://")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap();
                (name, (version, path))
            }
        })
        .collect();

    let (version, _path) = members
        .get(package)
        .expect("Package not found in workspace");

    // run cargo build
    let out = Command::new("cargo")
        .args(["build", "--target", "wasm32-wasi", "-p", package])
        .args(if release { &["--release"][..] } else { &[] })
        .status()
        .await
        .unwrap();

    tracing::info!("built the binary");

    if !out.success() {
        return None;
    }

    let import = Import {
        registry: None,
        plugin: package.to_owned(),
        version: Some(version.parse().unwrap()),
        sha: None,
    };

    // write the wasm file to a temp dir
    let tmp = std::env::temp_dir().join("litehouse");
    let wasi_path = tmp.join("wasi_snapshot_preview1.wasm");
    let out_path = tmp.join(import.file_name());
    std::fs::create_dir_all(&tmp).unwrap();
    std::fs::write(&wasi_path, WASM_PROCESS_FILE).unwrap();

    tracing::info!("wrote process file to {}", wasi_path.display());

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

    tracing::info!("attempted to create component");

    if !out.success() {
        tracing::error!("failed");
        return None;
    }

    tracing::info!("created component");
    Some((import, out_path))
}

pub async fn build(package: &str, wasm_path: &Path, debug: bool) {
    let (import, path) = build_in_temp(package, !debug).await.unwrap();
    tokio::fs::create_dir_all(wasm_path).await.unwrap();
    let dest_file = wasm_path.join(import.file_name());
    tokio::fs::copy(&path, &dest_file).await.unwrap();
}

pub async fn publish<D>(package: &str, op: &Registry<Upload, D>) {
    let (import, path) = build_in_temp(&package, true).await.unwrap();

    let success = op.publish(&import, &path).await;
    if success {
        println!("Published {}", import.file_name());
    } else {
        println!("Failed to publish {}", import.file_name());
    }
}

pub async fn fetch<'a, U>(
    config: &'a LitehouseConfig,
    op: &Registry<U, Download>,
) -> Vec<(&'a Import, bool)> {
    config
        .imports
        .iter()
        .map(|import| op.download_package(import).map(move |pass| (import, pass)))
        .collect::<futures::stream::FuturesUnordered<_>>()
        .collect::<Vec<_>>()
        .await
}

pub async fn lock(wasm_path: &Path) {
    let mut config = LitehouseConfig::load().unwrap();

    for import in &mut config.imports {
        import.read_sha(wasm_path).await;
    }

    config.save().unwrap();
}
