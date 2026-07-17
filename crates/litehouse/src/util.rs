use jsonc_parser::common::Ranged;
use jsonschema::paths::JSONPointer;
use miette::SourceSpan;

pub fn resolve_span(
    config: &jsonc_parser::ast::Value,
    pointer: &JSONPointer,
) -> Option<SourceSpan> {
    let range = resolve_pointer(config, pointer)?;
    let range = range.range();
    Some((range.start, range.width()).into())
}

pub fn resolve_pointer<'a>(
    config: &'a jsonc_parser::ast::Value,
    pointer: &JSONPointer,
) -> Option<&'a jsonc_parser::ast::Value<'a>> {
    let mut config = config;
    for part in pointer.iter() {
        match part {
            jsonschema::paths::PathChunk::Property(name) => {
                config = &config.as_object()?.get(name)?.value
            }
            jsonschema::paths::PathChunk::Index(idx) => {
                config = config.as_array()?.elements.get(*idx)?;
            }
            jsonschema::paths::PathChunk::Keyword(_) => return None,
        };
    }
    Some(config)
}

#[cfg(test)]
mod tests {
    use jsonc_parser::CollectOptions;
    use jsonschema::paths::{JSONPointer, PathChunk};

    use super::{resolve_pointer, resolve_span};

    fn parse(source: &str) -> jsonc_parser::ast::Value<'_> {
        let parsed = match jsonc_parser::parse_to_ast(
            source,
            &CollectOptions {
                comments: false,
                tokens: false,
            },
            &Default::default(),
        ) {
            Ok(parsed) => parsed,
            Err(error) => panic!("invalid JSONC fixture: {error:?}"),
        };
        let Some(value) = parsed.value else {
            panic!("fixture has no value");
        };
        value
    }

    #[test]
    fn resolves_root_properties_indexes_and_escaped_names() {
        let config = parse(r#"{"a/b~c":{"items":[1]}}"#);

        assert!(resolve_pointer(&config, &JSONPointer::default()).is_some());

        let chunks = [
            PathChunk::Property("a/b~c".into()),
            PathChunk::Property("items".into()),
            PathChunk::Index(0),
        ];
        let pointer = JSONPointer::from(chunks.as_slice());
        assert!(resolve_pointer(&config, &pointer).is_some());
        assert!(resolve_span(&config, &pointer).is_some());
    }

    #[test]
    fn returns_none_for_missing_or_mismatched_segments() {
        let config = parse(r#"{"items":[1]}"#);

        for chunks in [
            vec![PathChunk::Property("missing".into())],
            vec![PathChunk::Property("items".into()), PathChunk::Index(1)],
            vec![PathChunk::Index(0)],
        ] {
            let pointer = JSONPointer::from(chunks.as_slice());
            assert!(resolve_pointer(&config, &pointer).is_none());
            assert!(resolve_span(&config, &pointer).is_none());
        }
    }

    #[test]
    fn schema_keywords_fail_closed_without_panicking() {
        let config = parse(r#"{"items":[1]}"#);

        for chunks in [
            vec![PathChunk::Keyword("required")],
            vec![
                PathChunk::Property("items".into()),
                PathChunk::Keyword("minItems"),
            ],
            vec![
                PathChunk::Keyword("properties"),
                PathChunk::Property("items".into()),
            ],
        ] {
            let pointer = JSONPointer::from(chunks.as_slice());
            assert!(resolve_pointer(&config, &pointer).is_none());
            assert!(resolve_span(&config, &pointer).is_none());
        }
    }
}
