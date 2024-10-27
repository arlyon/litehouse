macro_rules! static_error {
    ($name:ident, $msg:expr, $desc:expr, $code:expr) => {
        pub struct $name;

        impl IntoResponse for $name {
            fn into_response(self) -> axum::response::Response {
                Resp::<_, $code>::new($msg).into_response()
            }
        }

        impl OperationOutput for $name {
            type Inner = &'static str;

            fn inferred_responses(
                ctx: &mut aide::gen::GenContext,
                operation: &mut aide::openapi::Operation,
            ) -> Vec<(Option<u16>, aide::openapi::Response)> {
                vec![(
                    Some($code),
                    aide::openapi::Response {
                        description: $desc.to_string(),
                        content: [(
                            "text/plain".into(),
                            aide::openapi::MediaType {
                                schema: Some(aide::openapi::SchemaObject {
                                    json_schema: schemars::schema::Schema::Object(
                                        schemars::schema::SchemaObject {
                                            const_value: Some(serde_json::json!($msg)),
                                            ..Default::default()
                                        },
                                    ),
                                    external_docs: None,
                                    example: None,
                                }),
                                example: Some(serde_json::json!($msg)),
                                ..Default::default()
                            },
                        )]
                        .into(),
                        ..Default::default()
                    },
                )]
            }
        }
    };
}
pub(crate) use static_error;
