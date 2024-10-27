use axum::routing::get;
use std::sync::Arc;

use aide::{
    axum::{ApiRouter, IntoApiResponse},
    openapi::OpenApi,
    transform::TransformOpenApi,
};
use axum::{response::IntoResponse, Extension, Json};

use crate::types::AppState;

pub fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
    api.title("Litehouse Cockpit")
        .summary("A webrtc signalling server for Litehouse")
        .description(include_str!("../readme.md"))
        .title("Cockpit Docs")
}

pub fn docs_routes(state: AppState) -> ApiRouter {
    let router: ApiRouter = ApiRouter::new()
        .route(
            "/",
            get(crate::scalar::Scalar::new("/docs/private/api.json")
                .with_title("Aide Axum")
                .axum_handler()),
        )
        .route("/private/api.json", get(serve_docs))
        .with_state(state);

    router
}

async fn serve_docs(Extension(api): Extension<Arc<OpenApi>>) -> impl IntoApiResponse {
    Json(api.as_ref()).into_response()
}
