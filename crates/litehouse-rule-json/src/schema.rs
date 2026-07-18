use std::collections::BTreeMap;

use schemars::generate::SchemaSettings;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::{
    artifacts::{ArtifactDigests, digests, generated_manifest_bytes},
    catalog::catalog,
    fixtures::fixture_values,
    model::{RuleDocument, SCHEMA_URL, SCHEMA_VERSION},
};

pub const PORT_CATALOG_URL: &str = crate::catalog::PORT_CATALOG_URL;

pub const CHECKED_RULE_SCHEMA: &[u8] = include_bytes!("../schema/rule-ir-v1.0.0.schema.json");
pub const CHECKED_PORT_CATALOG: &[u8] = include_bytes!("../schema/node-port-catalog-v1.0.0.json");
pub const CHECKED_EVALUATION_ARTIFACTS: &[u8] =
    include_bytes!("../schema/evaluation-artifacts-v1.json");
pub const CHECKED_FIXTURE_INVENTORY: &[u8] =
    include_bytes!("../fixtures/fixture-inventory-v1.json");

pub const CHECKED_FIXTURES: [CheckedFixture; 13] = [
    checked(
        "fixtures/valid-minimal.json",
        true,
        include_bytes!("../fixtures/valid-minimal.json"),
    ),
    checked(
        "fixtures/valid-planner-compat.json",
        true,
        include_bytes!("../fixtures/valid-planner-compat.json"),
    ),
    checked(
        "fixtures/valid-all-operations.json",
        true,
        include_bytes!("../fixtures/valid-all-operations.json"),
    ),
    checked(
        "fixtures/valid-all-node-families.json",
        true,
        include_bytes!("../fixtures/valid-all-node-families.json"),
    ),
    checked(
        "fixtures/valid-all-desired-paths.json",
        true,
        include_bytes!("../fixtures/valid-all-desired-paths.json"),
    ),
    checked(
        "fixtures/valid-time-one-shot.json",
        true,
        include_bytes!("../fixtures/valid-time-one-shot.json"),
    ),
    checked(
        "fixtures/valid-time-fixed-interval.json",
        true,
        include_bytes!("../fixtures/valid-time-fixed-interval.json"),
    ),
    checked(
        "fixtures/valid-time-weekly.json",
        true,
        include_bytes!("../fixtures/valid-time-weekly.json"),
    ),
    checked(
        "fixtures/valid-maximal.json",
        true,
        include_bytes!("../fixtures/valid-maximal.json"),
    ),
    checked(
        "fixtures/invalid-unknown-field.json",
        false,
        include_bytes!("../fixtures/invalid-unknown-field.json"),
    ),
    checked(
        "fixtures/invalid-bound.json",
        false,
        include_bytes!("../fixtures/invalid-bound.json"),
    ),
    checked(
        "fixtures/invalid-legacy-compare.json",
        false,
        include_bytes!("../fixtures/invalid-legacy-compare.json"),
    ),
    checked(
        "fixtures/invalid-legacy-on-off-set.json",
        false,
        include_bytes!("../fixtures/invalid-legacy-on-off-set.json"),
    ),
];

#[derive(Clone, Copy)]
pub struct CheckedFixture {
    pub path: &'static str,
    pub schema_valid: bool,
    pub semantic_valid: bool,
    pub bytes: &'static [u8],
}

pub struct GeneratedArtifact {
    pub path: String,
    pub schema_valid: bool,
    pub semantic_valid: bool,
    pub bytes: Vec<u8>,
}

const fn checked(path: &'static str, valid: bool, bytes: &'static [u8]) -> CheckedFixture {
    CheckedFixture {
        path,
        schema_valid: valid,
        semantic_valid: valid,
        bytes,
    }
}

pub fn generated_rule_schema_bytes() -> Vec<u8> {
    let generator = SchemaSettings::draft2020_12().into_generator();
    let root = generator.into_root_schema_for::<RuleDocument>();
    let mut value = serde_json::to_value(root).expect("RuleDocument schema must serialize");
    let object = value
        .as_object_mut()
        .expect("schemars root schema must be an object");
    object.insert(
        "$schema".to_owned(),
        Value::String("https://json-schema.org/draft/2020-12/schema".to_owned()),
    );
    object.insert("$id".to_owned(), Value::String(SCHEMA_URL.to_owned()));
    bound_explicit_maps(&mut value);
    close_object_schemas(&mut value);
    sorted_pretty_json(value)
}

pub fn generated_port_catalog_bytes() -> Vec<u8> {
    let value = serde_json::to_value(catalog(SCHEMA_VERSION)).expect("port catalog must serialize");
    sorted_pretty_json(value)
}

pub fn generated_evaluation_artifacts_bytes() -> Vec<u8> {
    generated_manifest_bytes(
        &generated_rule_schema_bytes(),
        &generated_port_catalog_bytes(),
    )
}

pub fn generated_fixture_artifacts() -> Vec<GeneratedArtifact> {
    let schema = generated_rule_schema_bytes();
    let catalog = generated_port_catalog_bytes();
    let digests = digests(&schema, &catalog);
    fixture_values(&digests)
        .into_iter()
        .map(|fixture| GeneratedArtifact {
            path: fixture.path,
            schema_valid: fixture.schema_valid,
            semantic_valid: fixture.semantic_valid,
            bytes: sorted_pretty_json(fixture.value),
        })
        .collect()
}

#[derive(Serialize)]
struct FixtureInventory {
    version: u8,
    fixtures: Vec<FixtureInventoryEntry>,
}

#[derive(Serialize)]
struct FixtureInventoryEntry {
    path: String,
    schema_valid: bool,
    semantic_valid: bool,
    sha256: String,
}

pub fn generated_fixture_inventory_bytes() -> Vec<u8> {
    let fixtures = generated_fixture_artifacts()
        .into_iter()
        .map(|fixture| FixtureInventoryEntry {
            path: fixture.path,
            schema_valid: fixture.schema_valid,
            semantic_valid: fixture.semantic_valid,
            sha256: format!("{:x}", Sha256::digest(&fixture.bytes)),
        })
        .collect();
    sorted_pretty_json(
        serde_json::to_value(FixtureInventory {
            version: 1,
            fixtures,
        })
        .expect("fixture inventory must serialize"),
    )
}

pub fn schema_sha256() -> String {
    format!("{:x}", Sha256::digest(CHECKED_RULE_SCHEMA))
}

pub fn port_catalog_sha256() -> String {
    format!("{:x}", Sha256::digest(CHECKED_PORT_CATALOG))
}

pub fn evaluation_profile_sha256() -> String {
    format!("{:x}", Sha256::digest(CHECKED_EVALUATION_ARTIFACTS))
}

pub(crate) fn artifact_digests() -> ArtifactDigests {
    digests(CHECKED_RULE_SCHEMA, CHECKED_PORT_CATALOG)
}

fn bound_explicit_maps(value: &mut Value) {
    let Some(definitions) = value.get_mut("$defs").and_then(Value::as_object_mut) else {
        return;
    };
    let maps = [
        ("SemanticRule", "annotations", 64_u64),
        ("EditorData", "nodes", 128_u64),
        ("EditorData", "groups", 32_u64),
    ];
    for (definition, property, maximum) in maps {
        let Some(schema) = definitions
            .get_mut(definition)
            .and_then(Value::as_object_mut)
            .and_then(|schema| schema.get_mut("properties"))
            .and_then(Value::as_object_mut)
            .and_then(|properties| properties.get_mut(property))
            .and_then(Value::as_object_mut)
        else {
            continue;
        };
        schema.insert("maxProperties".to_owned(), Value::from(maximum));
    }
}

fn close_object_schemas(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                close_object_schemas(value);
            }
        }
        Value::Object(object) => {
            let is_object = object.get("type").is_some_and(|value| value == "object")
                || object.contains_key("properties");
            if is_object {
                object
                    .entry("additionalProperties".to_owned())
                    .or_insert(Value::Bool(false));
                object.insert("unevaluatedProperties".to_owned(), Value::Bool(false));
            }
            for nested in object.values_mut() {
                close_object_schemas(nested);
            }
        }
        _ => {}
    }
}

fn sorted_pretty_json(value: Value) -> Vec<u8> {
    fn sort(value: Value) -> Value {
        match value {
            Value::Array(values) => Value::Array(values.into_iter().map(sort).collect()),
            Value::Object(object) => {
                let sorted: BTreeMap<_, _> = object
                    .into_iter()
                    .map(|(key, value)| (key, sort(value)))
                    .collect();
                let mut result = Map::new();
                for (key, value) in sorted {
                    result.insert(key, value);
                }
                Value::Object(result)
            }
            value => value,
        }
    }

    let mut bytes = serde_json::to_vec_pretty(&sort(value)).expect("generated JSON must serialize");
    bytes.push(b'\n');
    bytes
}
