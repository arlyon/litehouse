use litehouse_plugin::serde_json::Value;
use miette::{NamedSource, SourceSpan};

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
#[error("{count} validation errors found in settings.json")]
#[diagnostic(
    help("resolve all the below errors to continue"),
    url(docsrs),
    code(config::invalid)
)]
pub struct FailedValidations {
    #[source_code]
    src: NamedSource<String>,
    #[related]
    errors: Vec<FailedValidation>,
    count: usize,
}

impl FailedValidations {
    pub fn new(errors: Vec<FailedValidation>, src: String) -> Self {
        Self {
            count: errors.len(),
            errors,
            src: NamedSource::new("settings.json", src),
        }
    }
}

#[derive(thiserror::Error, Debug, miette::Diagnostic)]
#[error("failed to validate {instance_path}")]
#[diagnostic(help("{message}"))]
pub struct FailedValidation {
    pub error: Value,
    pub kind: jsonschema::error::ValidationErrorKind,
    pub message: String,
    pub label: String,
    #[label("{label}")]
    pub span: Option<SourceSpan>,

    pub schema_path: jsonschema::paths::JSONPointer,
    pub instance_path: jsonschema::paths::JSONPointer,
}
