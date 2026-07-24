use litehouse_plugin::{Config, JsonSchema, contracts::config::*};

#[allow(dead_code)]
#[derive(serde::Serialize, JsonSchema, Config)]
#[serde(rename_all = "camelCase")]
struct FixtureConfig {
    #[litehouse(environment_secret = "API_TOKEN", secret_encoding = "utf8")]
    api_token: String,
    #[litehouse(private_local = "omitted")]
    private_address: String,
    label: String,
}

#[test]
fn derive_emits_only_safe_typed_annotations() {
    let annotations = FixtureConfig::annotations();
    assert_eq!(annotations.len(), 2);
    assert_eq!(annotations[0].json_pointer, "/apiToken");
    assert_eq!(annotations[1].json_pointer, "/privateAddress");
    assert!(matches!(
        annotations[0].annotation,
        ConfigFieldAnnotation::EnvironmentSecret {
            ref field_token,
            encoding: SecretEncoding::Utf8,
        } if field_token == "API_TOKEN"
    ));
    assert!(matches!(
        annotations[1].annotation,
        ConfigFieldAnnotation::PrivateLocal {
            projection: PrivateProjection::Omitted,
        }
    ));
    let schema = FixtureConfig::schema_json();
    assert!(schema.contains("apiToken"));
    assert!(!schema.contains("API_TOKEN"));
}
