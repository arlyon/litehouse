use std::collections::BTreeMap;

use schemars::generate::SchemaSettings;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::model::{RuleDocument, SCHEMA_URL, SCHEMA_VERSION};

pub const PORT_CATALOG_URL: &str =
    "https://schemas.litehouse.dev/rule-ir/v1.0.0/node-port-catalog.json";

pub const CHECKED_RULE_SCHEMA: &[u8] = include_bytes!("../schema/rule-ir-v1.0.0.schema.json");
pub const CHECKED_PORT_CATALOG: &[u8] = include_bytes!("../schema/node-port-catalog-v1.0.0.json");

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
    close_object_schemas(&mut value);
    sorted_pretty_json(value)
}

pub fn generated_port_catalog_bytes() -> Vec<u8> {
    let catalog = PortCatalog {
        schema: "https://json-schema.org/draft/2020-12/schema",
        id: PORT_CATALOG_URL,
        schema_version: SCHEMA_VERSION,
        nodes: vec![
            node("state_observation", &[output("next", "control")]),
            node(
                "snapshot",
                &[
                    input("in", "control", "one-or-more"),
                    output("next", "control"),
                    output("value", "$node.value_type"),
                ],
            ),
            node(
                "compare",
                &[
                    input("in", "control", "one-or-more"),
                    input("left", "$node.value_type", "exactly-one"),
                    output("next", "control"),
                    output("result", "comparison-outcome"),
                ],
            ),
            node(
                "branch",
                &[
                    input("in", "control", "one-or-more"),
                    input("condition", "comparison-outcome", "exactly-one"),
                    output("true", "control"),
                    output("false", "control"),
                    output("unknown", "control"),
                    output("error", "control"),
                ],
            ),
            node(
                "on_off_set",
                &[
                    input("in", "control", "one-or-more"),
                    output("next", "control"),
                ],
            ),
            node("end", &[input("in", "control", "one-or-more")]),
        ],
    };
    let value = serde_json::to_value(catalog).expect("port catalog must serialize");
    sorted_pretty_json(value)
}

pub fn schema_sha256() -> String {
    format!("{:x}", Sha256::digest(CHECKED_RULE_SCHEMA))
}

pub fn port_catalog_sha256() -> String {
    format!("{:x}", Sha256::digest(CHECKED_PORT_CATALOG))
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
                object.insert("additionalProperties".to_owned(), Value::Bool(false));
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

#[derive(Serialize)]
struct PortCatalog<'a> {
    #[serde(rename = "$schema")]
    schema: &'a str,
    #[serde(rename = "$id")]
    id: &'a str,
    schema_version: &'a str,
    nodes: Vec<NodePorts<'a>>,
}

#[derive(Serialize)]
struct NodePorts<'a> {
    kind: &'a str,
    version: u8,
    ports: Vec<Port<'a>>,
}

#[derive(Clone, Serialize)]
struct Port<'a> {
    name: &'a str,
    direction: &'a str,
    type_ref: &'a str,
    cardinality: &'a str,
}

fn node<'a>(kind: &'a str, ports: &[Port<'a>]) -> NodePorts<'a> {
    NodePorts {
        kind,
        version: 1,
        ports: ports.to_vec(),
    }
}

fn input<'a>(name: &'a str, type_ref: &'a str, cardinality: &'a str) -> Port<'a> {
    Port {
        name,
        direction: "input",
        type_ref,
        cardinality,
    }
}

fn output<'a>(name: &'a str, type_ref: &'a str) -> Port<'a> {
    Port {
        name,
        direction: "output",
        type_ref,
        cardinality: "exactly-one",
    }
}
