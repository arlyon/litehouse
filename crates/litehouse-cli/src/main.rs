use litehouse_config::{Import, LitehouseConfig};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::process::Command;

use clap::{Parser, Subcommand};
use opendal::{services::S3, Operator};

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
    Publish { package: String },
    Fetch {
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    // Create s3 backend builder.
    let mut builder = S3::default();
    builder.root("v1");
    builder.bucket("abcd-test");
    builder.region("us-east-1");
    builder.endpoint("https://ams1.vultrobjects.com");
    builder.access_key_id("QN6P0OVEVPJI59ZSSI3S");
    builder.secret_access_key("DSxNmS7hyMT7scscrELh3lg7KlX32sWhOiSZXcAz");

    let op = Operator::new(builder).unwrap().finish();
    _ = op.check().await.unwrap();

    let opts = Opts::parse();

    match opts.command {
        Options::Publish { package } => publish(package, &op).await,
        Options::Fetch { wasm_path } => fetch(&wasm_path, &op).await,
    }
}

async fn publish(package: String, op: &Operator) {
    let workspaces_json = Command::new("cargo")
        .arg("metadata")
        .output()
        .await
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&workspaces_json.stdout).unwrap();

    let members: HashMap<&str, (&str, &str)> = data["workspace_members"]
        .as_array()
        .unwrap()
        .into_iter()
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
        .get(package.as_str())
        .expect("Package not found in workspace");

    // run cargo build
    let out = Command::new("cargo")
        .args(&[
            "build",
            "--release",
            "--target",
            "wasm32-wasi",
            "-p",
            &package,
        ])
        .status()
        .await
        .unwrap();

    if !out.success() {
        return;
    }

    let import = Import {
        registry: None,
        plugin: package.clone(),
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
        .args(&[
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
        return;
    }

    let mut writer = op
        .writer_with(&import.file_name())
        .buffer(8 * 1024 * 1024)
        .await
        .unwrap();
    let mut file = tokio::fs::File::open(&out_path).await.unwrap();
    let bytes = tokio::io::copy(&mut file, &mut writer).await.unwrap();
    println!("wrote {} bytes to {}", bytes, &import.file_name());
    writer.close().await.unwrap();

    let files = op.list("").await.unwrap();
    println!("Files: {:?}", files);
}

async fn fetch(wasm_path: &Path, op: &Operator) {
    let config = LitehouseConfig::load().unwrap();
    tokio::fs::create_dir_all(wasm_path).await.unwrap();

    for file in config.imports {
        // list all files using the package name as a prefix
        let mut files = op
            .list(&file.plugin)
            .await
            .unwrap()
            .into_iter()
            .filter_map(|e| {
                let name = e.name().strip_suffix(".wasm")?;
                Some((name.parse::<Import>().unwrap(), e))
            });

        let selected = if file.version.is_some() {
            // if there is a version, select it
            files.find(|(name, _)| file.eq(name))
        } else {
            // otherwise select the latest version
            files.max_by(|a, b| a.0.cmp(&b.0))
        };

        let Some((import, entry)) = selected else {
            println!("no matching plugin found for {:?}", file);
            continue;
        };

        let plugin_path = wasm_path.join(import.file_name());
        let mut reader = op.reader(entry.path()).await.unwrap();
        let mut file = tokio::fs::File::create(&plugin_path).await.unwrap();
        let bytes = tokio::io::copy(&mut reader, &mut file).await.unwrap();
        println!("wrote {} bytes to {:?}", bytes, plugin_path);
    }
}
