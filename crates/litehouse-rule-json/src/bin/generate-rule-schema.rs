use std::{env, fs, path::PathBuf};

use litehouse_rule_json::schema::{
    CHECKED_PORT_CATALOG, CHECKED_RULE_SCHEMA, generated_port_catalog_bytes,
    generated_rule_schema_bytes,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mode = env::args().nth(1).unwrap_or_else(|| "--check".to_owned());
    let schema = generated_rule_schema_bytes();
    let catalog = generated_port_catalog_bytes();
    match mode.as_str() {
        "--check" => {
            if schema != CHECKED_RULE_SCHEMA {
                return Err("checked Rule JSON schema differs from normative Rust".into());
            }
            if catalog != CHECKED_PORT_CATALOG {
                return Err("checked port catalog differs from normative Rust".into());
            }
            println!("Rule JSON schema and port catalog are reproducible");
        }
        "--write" => {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schema");
            fs::create_dir_all(&root)?;
            fs::write(root.join("rule-ir-v1.0.0.schema.json"), schema)?;
            fs::write(root.join("node-port-catalog-v1.0.0.json"), catalog)?;
            println!("wrote Rule JSON schema and port catalog");
        }
        _ => return Err("usage: generate-rule-schema [--check|--write]".into()),
    }
    Ok(())
}
