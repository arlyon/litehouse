use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use futures::FutureExt;
use indicatif::{ProgressFinish, ProgressStyle};
use itertools::Itertools;
use litehouse_config::{Import, LitehouseConfig};
use litehouse_plugin::serde_json;
use miette::{Context, IntoDiagnostic, NamedSource, bail};
use tokio::process::Command;
use tokio_stream::StreamExt;

use litehouse_registry::{Download, Registry, Upload};

const WASM_PROCESS_FILE: &[u8] =
    include_bytes!("../../../litehouse/wasi_snapshot_preview1.reactor.wasm");

const TARGET: &str = "wasm32-wasip1";

pub async fn build_in_temp(
    package: &str,
    release: bool,
    optimise: bool,
    no_package: bool,
) -> Result<(Import, PathBuf), miette::Report> {
    let workspaces_json = Command::new("cargo")
        .arg("metadata")
        .output()
        .await
        .unwrap();
    let data: serde_json::Value = serde_json::from_slice(&workspaces_json.stdout).unwrap();

    let (_name, version, path, is_cdylib) = data["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let v = v.as_str().unwrap();

            // check if package v is a cdylib
            let package = data["packages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["id"] == v)
                .unwrap();

            let is_cdylib = package["targets"].as_array().unwrap().iter().any(|t| {
                t["kind"]
                    .as_array()
                    .unwrap()
                    .contains(&serde_json::Value::String("cdylib".into()))
            });

            let (name, version, path) = if let Some(rest) = v.strip_prefix("path+file://") {
                // macOS, format path+file:///$PATH/$NAME#$VERSION
                let (path, rest) = rest.rsplit_once('/').unwrap();
                let (name, version) = rest.split_once('#').unwrap();
                (name, version, path)
            } else {
                // linux, format $NAME $VERSION (path+file://$PATH)
                let (name, rest) = v.split_once(' ').unwrap();
                let (version, rest) = rest.split_once(' ').unwrap();
                let path = rest
                    .strip_prefix("(path+file://")
                    .unwrap()
                    .strip_suffix(')')
                    .unwrap();
                (name, version, path)
            };

            (name, version, path, is_cdylib)
        })
        .find(|(name, _, _, _)| *name == package)
        .unwrap();

    if !is_cdylib {
        let path = Path::new(path).join(package).join("Cargo.toml");
        tracing::info!("{:?}", path);
        let contents = std::fs::read_to_string(&path).unwrap();

        return Err(miette::miette!(
                help = "Please add a crate-type of cdylib to your Cargo.toml",
                url = "https://rustwasm.github.io/docs/wasm-pack/tutorials/npm-browser-packages/template-deep-dive/cargo-toml.html#1-crate-type",
                "plugin {package} is not a cdylib", package = package
        ).with_source_code(NamedSource::new(path.to_string_lossy(), contents)));
    }

    let spinner_style = ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ");

    let build_output_path = {
        let sp = indicatif::ProgressBar::new_spinner()
            .with_style(spinner_style.clone())
            .with_message("Building...")
            .with_finish(ProgressFinish::WithMessage("Built!".into()))
            .with_prefix("[1/3]");

        sp.enable_steady_tick(Duration::from_millis(100));

        // run cargo build
        let out = Command::new("cargo")
            .args([
                "+nightly",
                "build",
                "-Zbuild-std=panic_abort,std",
                "--target",
                TARGET,
                "-p",
                package,
            ])
            .args(if release { &["--release"][..] } else { &[] })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .expect("failed to run subprocess");

        let disk_name = package.replace("-", "_");
        let pwd = std::env::current_dir().unwrap();
        let build_output_path = format!(
            "target/{}/{}/{}.wasm",
            TARGET,
            if release { "release" } else { "debug" },
            disk_name
        );

        if !out.status.success() {
            sp.abandon_with_message("  Failed!".to_string());
            bail!(
                "cargo build returned a non-zero exit code:\n\n{}",
                String::from_utf8(out.stderr).unwrap()
            );
        }

        tracing::info!("built the binary");

        pwd.join(build_output_path)
    };

    let import = Import {
        registry: None,
        plugin: package.to_owned(),
        version: Some(
            version
                .parse()
                .into_diagnostic()
                .wrap_err("unable to parse version")?,
        ),
        sha: None,
    };

    let tmp = std::env::temp_dir().join("litehouse");
    std::fs::create_dir_all(&tmp)
        .into_diagnostic()
        .wrap_err("unable to create temp dir")?;

    let build_output_path = if optimise {
        let sp = indicatif::ProgressBar::new_spinner()
            .with_style(spinner_style.clone())
            .with_finish(ProgressFinish::WithMessage("Optimising...".into()))
            .with_prefix("[2/3]");
        sp.enable_steady_tick(Duration::from_millis(100));

        // run wasm-opt
        let wasm_path_opt = tmp.join("opt.wasm");
        let mut cmd = Command::new("wasm-opt");
        let out = cmd.args([
            "-Oz",
            "--enable-mutable-globals",
            "--enable-bulk-memory",
            build_output_path.to_str().unwrap(),
            "-o",
            wasm_path_opt.to_str().unwrap(),
        ]);

        tracing::debug!(
            "running {:?} {}",
            out.as_std().get_program(),
            out.as_std()
                .get_args()
                .map(|s| s.to_string_lossy())
                .join(" ")
        );

        let out = out
            .output()
            .await
            .into_diagnostic()
            .wrap_err("unable to run wasm-opt. please make sure it is on your path")?;

        if !out.status.success() {
            sp.abandon_with_message("Failed!".to_string());
            bail!(
                "wasm-opt returned a non-zero exit code:\n\n{}",
                String::from_utf8(out.stderr).unwrap()
            );
        }
        wasm_path_opt
    } else {
        tracing::debug!("skipping optimising");
        build_output_path
    };

    let build_output_path = if !no_package {
        let sp = indicatif::ProgressBar::new_spinner()
            .with_style(spinner_style.clone())
            .with_finish(ProgressFinish::WithMessage("Packaged!".into()))
            .with_prefix("[3/3]");

        sp.enable_steady_tick(Duration::from_millis(100));

        // write the wasm file to a temp dir
        let wasi_path = tmp.join("wasi_snapshot_preview1.wasm");
        let component_path = tmp.join(import.file_name());
        std::fs::write(&wasi_path, WASM_PROCESS_FILE).unwrap();

        tracing::info!("wrote process file to {}", wasi_path.display());

        // run wasm-tools against the wasm file
        let out = Command::new("wasm-tools")
            .args([
                "component",
                "new",
                build_output_path.to_str().unwrap(),
                "--adapt",
                wasi_path.to_str().unwrap(),
                "-o",
                component_path.to_str().unwrap(),
            ])
            .status()
            .await
            .unwrap();

        tracing::info!("attempted to create component");

        if !out.success() {
            bail!("creating component returned a non-zero exit code");
        }

        tracing::info!("created component");
        component_path
    } else {
        tracing::debug!("skipping packaging");
        build_output_path
    };

    Ok((import, build_output_path))
}

pub async fn build(
    package: &str,
    wasm_path: &Path,
    debug: bool,
    optimise: bool,
    no_package: bool,
) -> Result<(), miette::Error> {
    let (import, path) = build_in_temp(package, !debug, optimise, no_package)
        .await
        .context("unable to build")?;
    tokio::fs::create_dir_all(wasm_path).await.unwrap();
    let dest_file = wasm_path.join(import.file_name());
    tokio::fs::copy(&path, &dest_file).await.unwrap();

    println!("");
    println!("");
    println!("Built {} in {}", import.file_name(), wasm_path.display());

    Ok(())
}

pub async fn publish<D>(package: &str, op: &Registry<Upload, D>) {
    let (import, path) = build_in_temp(package, true, true, false).await.unwrap();

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
