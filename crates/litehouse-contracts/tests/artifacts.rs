#[path = "../source_bundle.rs"]
mod source_bundle;

use litehouse_contracts::{
    CONTRACT_ARTIFACT_MANIFEST, CONTRACT_ARTIFACTS,
    quantity::{ErrorCode, Quantity},
};
use serde::Deserialize;
use std::{collections::BTreeSet, fs, path::Path};
use wit_component::{DecodedWasm, WitPrinter};
use wit_parser::{Resolve, UnresolvedPackageGroup};

#[derive(Deserialize)]
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

#[test]
fn emits_sixteen_independent_deterministic_packages() {
    let artifacts: Vec<Artifact> = serde_json::from_slice(CONTRACT_ARTIFACT_MANIFEST).unwrap();
    assert_eq!(artifacts.len(), 16);
    assert_eq!(CONTRACT_ARTIFACTS.len(), 16);
    let packages = artifacts
        .iter()
        .map(|artifact| artifact.package.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(packages.len(), 16);
    assert!(packages.contains("litehouse:provider"));
    for artifact in artifacts {
        assert_eq!(artifact.version, "1.0.0");
        assert_eq!(artifact.abi_sha256.len(), 64);
        assert_eq!(artifact.source_sha256.len(), 64);
        assert!(artifact.size > 8);
        assert!(artifact.source_size > 8);
        assert!(artifact.file.ends_with("-1.0.0.wasm"));
        assert!(artifact.source_file.ends_with("-1.0.0.tar.gz"));
    }
}

#[test]
fn every_binary_wit_decodes_to_its_source_package_graph() {
    let artifacts: Vec<Artifact> = serde_json::from_slice(CONTRACT_ARTIFACT_MANIFEST).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages");
    let mut sources = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path().join("1.0.0/wit"))
        .collect::<Vec<_>>();
    sources.sort();
    let groups = sources
        .iter()
        .map(|path| UnresolvedPackageGroup::parse_dir(path).unwrap())
        .collect::<Vec<_>>();

    for artifact in artifacts {
        let index = groups
            .iter()
            .position(|group| {
                group.main.name.namespace == artifact.package.split(':').next().unwrap()
                    && group.main.name.name == artifact.package.split(':').nth(1).unwrap()
            })
            .unwrap();
        let mut source_resolve = Resolve::default();
        let source_package = source_resolve
            .push_groups(
                groups[index].clone(),
                groups
                    .iter()
                    .enumerate()
                    .filter(|(candidate, _)| *candidate != index)
                    .map(|(_, group)| group.clone())
                    .collect(),
            )
            .unwrap();
        let bytes = fs::read(
            Path::new(env!("OUT_DIR"))
                .join("contracts")
                .join(&artifact.file),
        )
        .unwrap();
        let DecodedWasm::WitPackage(decoded_resolve, decoded_package) =
            wit_component::decode(&bytes).unwrap()
        else {
            panic!("{} did not decode as binary WIT", artifact.package);
        };
        assert_eq!(
            decoded_resolve.packages[decoded_package].name,
            source_resolve.packages[source_package].name
        );
        let mut source = WitPrinter::default();
        source.print(&source_resolve, source_package, &[]).unwrap();
        let mut decoded = WitPrinter::default();
        decoded
            .print(&decoded_resolve, decoded_package, &[])
            .unwrap();
        assert_eq!(
            source.output.to_string(),
            decoded.output.to_string(),
            "binary/source WIT drift for {}",
            artifact.package
        );
    }
}

#[test]
fn same_major_wit_compatibility_passes_and_required_signature_drift_fails() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/semver.wit");
    let group = UnresolvedPackageGroup::parse(&fixture, &fs::read_to_string(&fixture).unwrap())
        .map_err(|(_, error)| error)
        .unwrap();
    let mut resolve = Resolve::default();
    let package = resolve.push_group(group).unwrap();
    let previous = resolve.packages[package].worlds["previous"];
    let compatible = resolve.packages[package].worlds["compatible-same-major"];
    let breaking = resolve.packages[package].worlds["breaking-same-major"];
    assert!(wit_component::semver_check(resolve.clone(), previous, compatible).is_ok());
    assert!(wit_component::semver_check(resolve, previous, breaking).is_err());
}

#[test]
fn capability_packages_import_only_litehouse_shared_contracts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages");
    let capabilities = [
        "on-off",
        "level-control",
        "color-control",
        "boolean-state",
        "measurement",
        "thermostat",
        "window-covering",
        "momentary-action",
        "notification-sink",
    ];
    for capability in capabilities {
        let wit = fs::read_to_string(
            root.join(capability)
                .join("1.0.0/wit")
                .join(format!("{capability}.wit")),
        )
        .unwrap();
        assert!(!wit.contains("wasi:"), "{capability} imports WASI");
        for line in wit
            .lines()
            .filter(|line| line.trim_start().starts_with("use "))
        {
            assert!(
                line.contains("litehouse:") || line.trim_start().starts_with("use types."),
                "{capability} has non-Litehouse dependency: {line}"
            );
        }
    }
}

#[test]
fn provider_reuses_execution_free_capability_types() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages");
    let provider = fs::read_to_string(root.join("provider/1.0.0/wit/provider.wit")).unwrap();
    assert!(provider.contains("litehouse:on-off/types@1.0.0"));
    assert!(provider.contains("litehouse:boolean-state/types@1.0.0"));
    assert!(provider.contains("litehouse:measurement/types@1.0.0"));
    assert!(provider.contains("litehouse:notification-sink/types@1.0.0"));
    assert!(provider.contains("litehouse:level-control/types@1.0.0"));
    assert!(provider.contains("litehouse:color-control/types@1.0.0"));
    assert!(provider.contains("litehouse:thermostat/types@1.0.0"));
    assert!(provider.contains("litehouse:window-covering/types@1.0.0"));
    assert!(provider.contains("litehouse:momentary-action/types@1.0.0"));
    for package in [
        "on-off",
        "boolean-state",
        "measurement",
        "notification-sink",
        "level-control",
        "color-control",
        "thermostat",
        "window-covering",
        "momentary-action",
    ] {
        assert!(!provider.contains(&format!("litehouse:{package}/capability@1.0.0")));
    }

    let mut package_roots = fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path().join("1.0.0/wit"))
        .collect::<Vec<_>>();
    package_roots.sort();
    let groups = package_roots
        .iter()
        .map(|path| UnresolvedPackageGroup::parse_dir(path).unwrap())
        .collect::<Vec<_>>();
    for package in [
        "on-off",
        "boolean-state",
        "measurement",
        "notification-sink",
        "level-control",
        "color-control",
        "thermostat",
        "window-covering",
        "momentary-action",
    ] {
        let source = groups
            .iter()
            .position(|group| group.main.name.name == package)
            .unwrap();
        let mut resolve = Resolve::default();
        let package_id = resolve
            .push_groups(
                groups[source].clone(),
                groups
                    .iter()
                    .enumerate()
                    .filter(|(candidate, _)| *candidate != source)
                    .map(|(_, group)| group.clone())
                    .collect(),
            )
            .unwrap();
        let types = resolve.packages[package_id].interfaces["types"];
        assert!(
            resolve.interfaces[types].functions.is_empty(),
            "{package}/types gained executable functions"
        );
    }
}

#[test]
fn every_checked_json_fixture_is_valid() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages");
    for path in walk(&root).into_iter().filter(|path| {
        path.extension()
            .is_some_and(|extension| extension == "json")
    }) {
        serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap())
            .unwrap_or_else(|error| panic!("invalid JSON fixture {}: {error}", path.display()));
    }
}

#[test]
fn source_bundles_are_reproducible_and_reject_symlinks() {
    let packages = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages");
    let package = packages.join("on-off/1.0.0");
    assert_eq!(
        source_bundle::source_bundle(&package, &packages).unwrap(),
        source_bundle::source_bundle(&package, &packages).unwrap()
    );

    #[cfg(unix)]
    {
        use std::{os::unix::fs::symlink, os::unix::net::UnixListener};
        let sandbox = std::env::temp_dir().join(format!(
            "litehouse-source-bundle-boundary-{}",
            std::process::id()
        ));
        let trusted = sandbox.join("packages");
        let outside = sandbox.join("outside");
        let _ = fs::remove_dir_all(&sandbox);
        fs::create_dir_all(&trusted).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("secret"), "must not be bundled").unwrap();

        symlink(&outside, trusted.join("symlinked-package")).unwrap();
        let error =
            source_bundle::source_bundle(&trusted.join("symlinked-package"), &trusted).unwrap_err();
        assert!(error.to_string().contains("symlinked roots"));

        let package_root = trusted.join("fixture");
        fs::create_dir_all(&package_root).unwrap();
        symlink(&outside, package_root.join("1.0.0")).unwrap();
        let error =
            source_bundle::source_bundle(&package_root.join("1.0.0"), &trusted).unwrap_err();
        assert!(error.to_string().contains("symlinked roots"));

        let regular_root = trusted.join("regular/1.0.0");
        fs::create_dir_all(&regular_root).unwrap();
        symlink(outside.join("secret"), regular_root.join("escape")).unwrap();
        let error = source_bundle::source_bundle(&regular_root, &trusted).unwrap_err();
        assert!(error.to_string().contains("reject symlinks"));
        fs::remove_file(regular_root.join("escape")).unwrap();

        let socket = UnixListener::bind(regular_root.join("socket")).unwrap();
        let error = source_bundle::source_bundle(&regular_root, &trusted).unwrap_err();
        assert!(error.to_string().contains("regular files"));
        drop(socket);

        fs::remove_dir_all(&sandbox).unwrap();
    }
}

#[test]
fn fixture_quantities_use_real_profiles_and_invalid_vectors_fail_deliberately() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages");
    for path in walk(&root)
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|name| name == "valid.json"))
    {
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        validate_quantities(&value).unwrap_or_else(|error| {
            panic!(
                "valid fixture quantity failed {}: {error:?}",
                path.display()
            )
        });
    }

    let level: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("level-control/1.0.0/fixtures/invalid.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        find_quantity(&level).unwrap().unwrap_err().code,
        ErrorCode::SemanticMismatch
    );
    let measurement: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("measurement/1.0.0/fixtures/invalid.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        find_quantity(&measurement).unwrap().unwrap_err().code,
        ErrorCode::DimensionMismatch
    );
}

#[test]
fn no_generated_contract_artifact_is_tracked_in_source() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("packages");
    let generated = walk(&root)
        .into_iter()
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "wasm")
                || path.file_name().is_some_and(|name| name == "target")
        })
        .collect::<Vec<_>>();
    assert!(
        generated.is_empty(),
        "source-owned generated artifacts: {generated:?}"
    );
}

fn validate_quantities(
    value: &serde_json::Value,
) -> Result<(), litehouse_contracts::quantity::QuantityError> {
    if let Some(result) = first_quantity(value) {
        result?;
    }
    match value {
        serde_json::Value::Object(object) => {
            for child in object.values() {
                validate_quantities(child)?;
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                validate_quantities(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn find_quantity(
    value: &serde_json::Value,
) -> Option<Result<Quantity, litehouse_contracts::quantity::QuantityError>> {
    if let Some(quantity) = first_quantity(value) {
        return Some(quantity);
    }
    match value {
        serde_json::Value::Object(object) => object.values().find_map(find_quantity),
        serde_json::Value::Array(values) => values.iter().find_map(find_quantity),
        _ => None,
    }
}

fn first_quantity(
    value: &serde_json::Value,
) -> Option<Result<Quantity, litehouse_contracts::quantity::QuantityError>> {
    let object = value.as_object()?;
    let profile = object.get("profile")?.as_str()?;
    let magnitude = object.get("magnitude")?.as_str()?;
    let unit = object.get("unit")?.as_str()?;
    Some(Quantity::parse(profile, magnitude, unit))
}

fn walk(root: &Path) -> Vec<std::path::PathBuf> {
    let mut output = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        output.push(path.clone());
        if path.is_dir() {
            output.extend(walk(&path));
        }
    }
    output
}
