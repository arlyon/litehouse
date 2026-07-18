use std::{env, fs, path::PathBuf};

use litehouse_rule_json::schema::{
    CHECKED_EVALUATION_ARTIFACTS, CHECKED_FIXTURE_INVENTORY, CHECKED_FIXTURES,
    CHECKED_PORT_CATALOG, CHECKED_RULE_SCHEMA, generated_evaluation_artifacts_bytes,
    generated_fixture_artifacts, generated_fixture_inventory_bytes, generated_port_catalog_bytes,
    generated_rule_schema_bytes,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = env::args().nth(1).unwrap_or_else(|| "--check".to_owned());
    let schema = generated_rule_schema_bytes();
    let catalog = generated_port_catalog_bytes();
    let evaluation_artifacts = generated_evaluation_artifacts_bytes();
    let fixtures = generated_fixture_artifacts();
    let fixture_inventory = generated_fixture_inventory_bytes();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    match mode.as_str() {
        "--check" => {
            if schema != CHECKED_RULE_SCHEMA {
                return Err("checked Rule JSON schema differs from normative Rust".into());
            }
            if catalog != CHECKED_PORT_CATALOG {
                return Err("checked port catalog differs from normative Rust".into());
            }
            if evaluation_artifacts != CHECKED_EVALUATION_ARTIFACTS {
                return Err(
                    "checked evaluation artifact manifest differs from normative sources".into(),
                );
            }
            if fixture_inventory != CHECKED_FIXTURE_INVENTORY {
                return Err("checked fixture inventory differs from generator".into());
            }
            for (generated, checked) in fixtures
                .iter()
                .take(CHECKED_FIXTURES.len())
                .zip(CHECKED_FIXTURES)
            {
                if generated.path != checked.path
                    || generated.schema_valid != checked.schema_valid
                    || generated.semantic_valid != checked.semantic_valid
                    || generated.bytes != checked.bytes
                {
                    return Err(format!(
                        "checked fixture {} differs from generator",
                        generated.path
                    )
                    .into());
                }
            }
            for generated in &fixtures {
                let checked = fs::read(root.join(&generated.path))?;
                if generated.bytes != checked {
                    return Err(format!(
                        "checked fixture {} differs from generator",
                        generated.path
                    )
                    .into());
                }
            }
            println!(
                "Rule JSON schema, port catalog, evaluation artifacts, and fixtures are reproducible"
            );
        }
        "--write" => {
            let schema_root = root.join("schema");
            fs::create_dir_all(&schema_root)?;
            fs::write(schema_root.join("rule-ir-v1.0.0.schema.json"), schema)?;
            fs::write(schema_root.join("node-port-catalog-v1.0.0.json"), catalog)?;
            fs::write(
                schema_root.join("evaluation-artifacts-v1.json"),
                evaluation_artifacts,
            )?;
            fs::write(
                root.join("fixtures/fixture-inventory-v1.json"),
                fixture_inventory,
            )?;
            for fixture in fixtures {
                let path = root.join(fixture.path);
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(path, fixture.bytes)?;
            }
            println!("wrote Rule JSON schema, port catalog, evaluation artifacts, and fixtures");
        }
        _ => return Err("usage: generate-rule-schema [--check|--write]".into()),
    }
    Ok(())
}
