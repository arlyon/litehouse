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
            jsonschema::paths::PathChunk::Keyword(_) => todo!(),
        };
    }
    Some(config)
}
