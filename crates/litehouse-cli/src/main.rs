mod cache_layer;

use litehouse_config::{Import, LitehouseConfig};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::process::Command;

use clap::{Parser, Subcommand};
use opendal::{services::S3, Builder, Entry, Operator};

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
    Build {
        package: String,
        #[clap(default_value = "wasm")]
        wasm_path: PathBuf,
    },
}

#[tokio::main]
async fn main() {
    let dirs = litehouse_config::directories().unwrap();
    let registry = Registry::new("default".to_string(), Some(dirs.cache_dir()));
    if !registry.check().await {
        println!("Failed to connect to the registry");
        return;
    }
    let opts = Opts::parse();

    match opts.command {
        Options::Publish { package } => publish(package, &registry.with_capability(Upload)).await,
        Options::Fetch { wasm_path } => fetch(&registry.with_capability(Download(wasm_path))).await,
        Options::Build { wasm_path, package } => build(&package, &wasm_path).await,
    }
}

struct Registry<T> {
    op: Operator,
    name: String,
    capability: T,
}

struct Download(PathBuf);
struct Upload;

impl Registry<()> {
    pub fn new(name: String, local_cache: Option<&Path>) -> Self {
        // Create s3 backend builder.
        let mut builder = S3::default();
        builder.root("v1");
        builder.bucket("abcd-test");
        builder.region("us-east-1");
        builder.endpoint("https://ams1.vultrobjects.com");
        builder.access_key_id("QN6P0OVEVPJI59ZSSI3S");
        builder.secret_access_key("DSxNmS7hyMT7scscrELh3lg7KlX32sWhOiSZXcAz");

        let op = Operator::new(builder).unwrap();

        let op = if let Some(local_cache) = local_cache {
            let mut fs_cache = opendal::services::Fs::default();
            fs_cache.root(local_cache.to_str().unwrap());
            let fs_cache = fs_cache.build().unwrap();
            op.layer(cache_layer::CacheLayer::new(fs_cache)).finish()
        } else {
            op.finish()
        };

        Self {
            op,
            name,
            capability: (),
        }
    }

    pub fn with_capability<T>(self, cap: T) -> Registry<T> {
        Registry {
            op: self.op,
            name: self.name,
            capability: cap,
        }
    }
}

impl<T> Registry<T> {
    async fn check(&self) -> bool {
        self.op.check().await.is_ok()
    }

    pub async fn list(&self, prefix: Option<&Import>) -> impl Iterator<Item = (Import, Entry)> {
        self.op
            .list(prefix.map(|p| p.plugin.as_str()).unwrap_or_default())
            .await
            .unwrap()
            .into_iter()
            .filter_map(|e| {
                let name = e.name().strip_suffix(".wasm")?;
                let import = name.parse::<Import>().ok()?;
                Some((import, e))
            })
    }

    pub async fn publish(&self, plugin: &Import, path: &Path) -> bool {
        let mut writer = self
            .op
            .writer_with(&plugin.file_name())
            .buffer(8 * 1024 * 1024)
            .await
            .unwrap();
        let mut file = tokio::fs::File::open(&path).await.unwrap();
        let bytes = tokio::io::copy(&mut file, &mut writer).await.unwrap();
        println!("wrote {} bytes to {}", bytes, &plugin.file_name());
        writer.close().await.unwrap();
        true
    }
}

impl Registry<Download> {
    async fn download_package(&self, import: Import) -> bool {
        if let Some(registry) = &import.registry {
            if self.name.ne(registry) {
                return false;
            }
        }

        // if we have the version, just try to nab it
        if import.version.is_some() {
            return self.download_file(&import.file_name()).await.is_some();
        }

        // list all files using the package name as a prefix
        let files = self.list(Some(&import)).await;

        // otherwise select the latest version
        let selected = files.max_by(|a, b| a.0.cmp(&b.0));

        let Some((_, entry)) = selected else {
            println!("no matches found for {:?}", import.plugin);
            return false;
        };

        self.download_file(entry.path()).await.is_some()
    }

    async fn download_file(&self, file: &str) -> Option<u64> {
        // mk_dir_all on the path
        tokio::fs::create_dir_all(&self.capability.0).await.unwrap();

        let plugin_path = self.capability.0.join(file);
        let mut reader = self.op.reader(file).await.unwrap();
        let mut file = tokio::fs::File::create(&plugin_path).await.unwrap();
        let bytes = tokio::io::copy(&mut reader, &mut file).await.unwrap();
        Some(bytes)
    }
}

async fn build_in_temp(package: &str) -> Option<(Import, PathBuf)> {
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
        .args([
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

async fn build(package: &str, wasm_path: &Path) {
    let (import, path) = build_in_temp(package).await.unwrap();
    tokio::fs::create_dir_all(wasm_path).await.unwrap();
    let dest_file = wasm_path.join(import.file_name());
    tokio::fs::copy(&path, &dest_file).await.unwrap();
}

async fn publish(package: String, op: &Registry<Upload>) {
    let (import, path) = build_in_temp(&package).await.unwrap();

    let success = op.publish(&import, &path).await;
    if success {
        println!("Published {}", import.file_name());
    } else {
        println!("Failed to publish {}", import.file_name());
    }
}

async fn fetch(op: &Registry<Download>) {
    let config = LitehouseConfig::load().unwrap();
    for file in config.imports {
        op.download_package(file).await;
    }
}
