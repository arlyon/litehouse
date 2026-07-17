use litehouse_contracts::{CONTRACT_ARTIFACT_MANIFEST, management::management_manifest};
use serde::Serialize;
use std::{env, fs, path::PathBuf, process::ExitCode};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedBundle<'a, T> {
    format: &'static str,
    source: &'static str,
    operations: T,
    contract_artifacts: serde_json::Value,
    output_kind: &'a str,
    adapter_targets: [&'static str; 4],
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("contract-codegen: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args_os().skip(1);
    let first = arguments.next();
    let (check, output) = match first.as_deref().and_then(|value| value.to_str()) {
        Some("--check") => (true, None),
        Some("--out") => {
            let path = arguments.next().ok_or("--out requires a directory")?;
            (false, Some(PathBuf::from(path)))
        }
        None => return Err("expected --check or --out <directory>".into()),
        Some(_) => return Err("expected --check or --out <directory>".into()),
    };
    if arguments.next().is_some() {
        return Err("unexpected additional argument".into());
    }

    let first = generate()?;
    let second = generate()?;
    if first != second {
        return Err("generation differed for identical inputs".into());
    }
    if check {
        println!("contract generation deterministic ({} bytes)", first.len());
        return Ok(());
    }

    let output = output.expect("non-check output has a directory");
    fs::create_dir_all(&output)?;
    fs::write(output.join("management-projections-v1.json"), first)?;
    fs::write(
        output.join("contract-artifacts-v1.json"),
        CONTRACT_ARTIFACT_MANIFEST,
    )?;
    Ok(())
}

fn generate() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let manifest = management_manifest()?;
    let artifacts: serde_json::Value = serde_json::from_slice(CONTRACT_ARTIFACT_MANIFEST)?;
    let bundle = GeneratedBundle {
        format: "litehouse-management-projections-v1",
        source: "management/management-operations-v1.json",
        operations: manifest.projection_metadata(),
        contract_artifacts: artifacts,
        output_kind: "closed-adapter-inputs",
        adapter_targets: ["http", "webrtc", "web", "mcp"],
    };
    let mut bytes = serde_json::to_vec_pretty(&bundle)?;
    bytes.push(b'\n');
    Ok(bytes)
}
