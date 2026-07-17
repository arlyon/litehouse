use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use futures::FutureExt;
use indicatif::{ProgressFinish, ProgressStyle};
use litehouse_config::{Import, LitehouseConfig};
use litehouse_contracts::component::{ComponentKind, validate_component};
use litehouse_plugin::serde_json;
use miette::{Context, IntoDiagnostic, bail};
use tokio::process::Command;
use tokio_stream::StreamExt;

use litehouse_registry::{Download, Registry, Upload};

const TARGET: &str = "wasm32-wasip2";

pub async fn build_in_temp(
    package: &str,
    release: bool,
) -> Result<(Import, PathBuf), miette::Report> {
    let metadata = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .await
        .into_diagnostic()
        .wrap_err("unable to inspect Cargo workspace")?;
    if !metadata.status.success() {
        bail!(
            "cargo metadata returned a non-zero exit code:\n\n{}",
            String::from_utf8_lossy(&metadata.stderr)
        );
    }
    let data: serde_json::Value = serde_json::from_slice(&metadata.stdout)
        .into_diagnostic()
        .wrap_err("cargo metadata returned malformed JSON")?;
    let target_directory = data["target_directory"]
        .as_str()
        .map(PathBuf::from)
        .ok_or_else(|| miette::miette!("cargo metadata has no target directory"))?;
    let package_metadata = data["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|candidate| candidate["name"] == package)
        })
        .ok_or_else(|| miette::miette!("package `{package}` is not in the workspace"))?;
    let version = package_metadata["version"]
        .as_str()
        .ok_or_else(|| miette::miette!("package `{package}` has no version"))?;
    let is_cdylib = package_metadata["targets"]
        .as_array()
        .is_some_and(|targets| {
            targets.iter().any(|target| {
                target["crate_types"]
                    .as_array()
                    .is_some_and(|types| types.iter().any(|kind| kind == "cdylib"))
            })
        });
    if !is_cdylib {
        return Err(miette::miette!(
            help = "Add `crate-type = [\"cdylib\"]` to the package's [lib] section",
            "plugin `{package}` must build as a Component Model cdylib"
        ));
    }

    let spinner = indicatif::ProgressBar::new_spinner()
        .with_style(
            ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
                .expect("valid spinner template")
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
        )
        .with_message("Building wasm32-wasip2 component...")
        .with_finish(ProgressFinish::WithMessage("Built!".into()))
        .with_prefix("[1/1]");
    spinner.enable_steady_tick(Duration::from_millis(100));

    let output = Command::new("cargo")
        .args(["build", "--target", TARGET, "-p", package])
        .args(if release { &["--release"][..] } else { &[] })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .into_diagnostic()
        .wrap_err("unable to run cargo build")?;
    if !output.status.success() {
        spinner.abandon_with_message("Failed!".to_string());
        bail!(
            "cargo build returned a non-zero exit code:\n\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let disk_name = package.replace('-', "_");
    let component = target_directory
        .join(TARGET)
        .join(if release { "release" } else { "debug" })
        .join(format!("{disk_name}.wasm"));
    if !component.is_file() {
        bail!("cargo did not produce `{}`", component.display());
    }

    let validation = Command::new("wasm-tools")
        .args(["validate", "--features", "cm-async"])
        .arg(&component)
        .output()
        .await
        .into_diagnostic()
        .wrap_err("unable to run pinned wasm-tools")?;
    if !validation.status.success() {
        bail!(
            "component validation failed:\n\n{}",
            String::from_utf8_lossy(&validation.stderr)
        );
    }
    let component_bytes = tokio::fs::read(&component)
        .await
        .into_diagnostic()
        .wrap_err("unable to read built component for runtime-profile validation")?;
    validate_component(&component_bytes, ComponentKind::Plugin)
        .into_diagnostic()
        .wrap_err("component violates the frozen runtime profile")?;

    Ok((
        Import {
            registry: None,
            plugin: package.to_owned(),
            version: Some(version.parse().into_diagnostic()?),
            sha: None,
        },
        component,
    ))
}

pub async fn build(package: &str, wasm_path: &Path, debug: bool) -> Result<(), miette::Error> {
    let (import, path) = build_in_temp(package, !debug)
        .await
        .context("unable to build")?;
    tokio::fs::create_dir_all(wasm_path).await.unwrap();
    let dest_file = wasm_path.join(import.file_name());
    tokio::fs::copy(&path, &dest_file).await.unwrap();

    println!();
    println!();
    println!("Built {} in {}", import.file_name(), wasm_path.display());

    Ok(())
}

pub async fn publish<D>(package: &str, op: &Registry<Upload, D>) {
    let (import, path) = build_in_temp(package, true).await.unwrap();

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
