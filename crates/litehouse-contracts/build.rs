mod source_bundle;

use anyhow::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use source_bundle::source_bundle;
use std::{
    env, fs,
    path::{Path, PathBuf},
};
use wit_parser::{Resolve, UnresolvedPackageGroup};

#[derive(Clone)]
struct SourcePackage {
    directory: PathBuf,
    group: UnresolvedPackageGroup,
}

#[derive(Serialize)]
struct Artifact {
    package: String,
    version: String,
    abi_sha256: String,
    source_sha256: String,
    source_size: usize,
    size: usize,
    file: String,
    source_file: String,
}

fn main() -> Result<()> {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").context("manifest directory")?);
    let packages_root = root.join("packages");
    let out = PathBuf::from(env::var_os("OUT_DIR").context("OUT_DIR")?);
    let artifact_root = out.join("contracts");
    fs::create_dir_all(&artifact_root)?;

    println!("cargo::rerun-if-changed={}", packages_root.display());
    println!(
        "cargo::rerun-if-changed={}",
        root.join("management/management-operations-v1.json")
            .display()
    );

    let packages = discover_packages(&packages_root)?;
    if packages.len() != 15 {
        bail!("expected 15 contract packages, found {}", packages.len());
    }

    let mut artifacts = Vec::with_capacity(packages.len());
    for (index, source) in packages.iter().enumerate() {
        let mut resolve = Resolve::default();
        let dependencies = packages
            .iter()
            .enumerate()
            .filter(|(candidate, _)| *candidate != index)
            .map(|(_, package)| package.group.clone())
            .collect();
        let package_id = resolve
            .push_groups(source.group.clone(), dependencies)
            .with_context(|| format!("resolve {}", source.directory.display()))?;
        let first = wit_component::encode(&resolve, package_id)?;
        let second = wit_component::encode(&resolve, package_id)?;
        if first != second {
            bail!(
                "non-deterministic WIT encoding for {}",
                source.directory.display()
            );
        }

        let name = &resolve.packages[package_id].name;
        let version = name
            .version
            .as_ref()
            .context("contract package must have an exact version")?
            .to_string();
        if version != "1.0.0" {
            bail!("{} must start at 1.0.0", name);
        }
        let package = format!("{}:{}", name.namespace, name.name);
        let file = format!("{}-{}.wasm", name.name, version);
        let source_file = format!("{}-{}.tar.gz", name.name, version);
        let source_bytes = source_bundle(&source.directory, &packages_root)?;
        if source_bytes != source_bundle(&source.directory, &packages_root)? {
            bail!(
                "non-deterministic source bundle for {}",
                source.directory.display()
            );
        }
        fs::write(artifact_root.join(&file), &first)?;
        fs::write(artifact_root.join(&source_file), &source_bytes)?;
        artifacts.push(Artifact {
            package,
            version,
            abi_sha256: sha256(&first),
            source_sha256: sha256(&source_bytes),
            source_size: source_bytes.len(),
            size: first.len(),
            file,
            source_file,
        });
    }
    artifacts.sort_by(|left, right| left.package.cmp(&right.package));

    let manifest = serde_json::to_vec_pretty(&artifacts)?;
    fs::write(out.join("contract-artifacts.json"), &manifest)?;
    fs::write(out.join("contract_artifacts.rs"), rust_manifest(&artifacts))?;
    Ok(())
}

fn discover_packages(root: &Path) -> Result<Vec<SourcePackage>> {
    let mut directories = Vec::new();
    for package in sorted_directories(root)? {
        for version in sorted_directories(&package)? {
            if version.join("wit").is_dir() {
                directories.push(version);
            }
        }
    }
    directories.sort();
    directories
        .into_iter()
        .map(|directory| {
            let group = UnresolvedPackageGroup::parse_dir(directory.join("wit"))
                .with_context(|| format!("parse {}", directory.display()))?;
            Ok(SourcePackage { directory, group })
        })
        .collect()
}

fn sorted_directories(root: &Path) -> Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(root).with_context(|| format!("read {}", root.display()))? {
        let path = entry?.path();
        let file_type = fs::symlink_metadata(&path)?.file_type();
        if file_type.is_symlink() {
            bail!(
                "contract package discovery rejects symlinks: {}",
                path.display()
            );
        }
        if !file_type.is_dir() {
            bail!(
                "contract package discovery requires directories: {}",
                path.display()
            );
        }
        entries.push(path);
    }
    entries.sort();
    Ok(entries)
}

fn sha256(bytes: &[u8]) -> String {
    hex(Sha256::digest(bytes))
}

fn hex(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn rust_manifest(artifacts: &[Artifact]) -> String {
    let mut output = String::from("pub const CONTRACT_ARTIFACTS: &[ContractArtifact] = &[\n");
    for artifact in artifacts {
        output.push_str(&format!(
            "    ContractArtifact {{ package: {:?}, version: {:?}, abi_sha256: {:?}, source_sha256: {:?}, size: {}, source_size: {} }},\n",
            artifact.package,
            artifact.version,
            artifact.abi_sha256,
            artifact.source_sha256,
            artifact.size,
            artifact.source_size,
        ));
    }
    output.push_str("];\n");
    output
}
