use std::{collections::BTreeMap, net::IpAddr, sync::Arc};

use aide::{
    OperationInput, OperationOutput,
    axum::IntoApiResponse,
    openapi::{ExternalDocumentation, MediaType, Response},
};
use axum::{async_trait, extract::FromRequestParts, response::IntoResponse};
use headers::authorization::Basic;
use tokio::sync::{Mutex, oneshot::Sender};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

pub struct Resp<T: IntoApiResponse, const S: u16 = 200>(T);
impl<T: IntoApiResponse, const S: u16> IntoResponse for Resp<T, S> {
    fn into_response(self) -> axum::response::Response {
        (axum::http::StatusCode::from_u16(S).unwrap(), self.0).into_response()
    }
}
impl<T: IntoApiResponse, const S: u16> OperationOutput for Resp<T, S> {
    type Inner = T;
    fn inferred_responses(
        ctx: &mut aide::r#gen::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, aide::openapi::Response)> {
        T::inferred_responses(ctx, operation)
            .into_iter()
            .map(|(status, resp)| (Some(S), resp))
            .collect()
    }
}
impl<T: IntoApiResponse, const S: u16> Resp<T, S> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }
}

pub enum EitherResponse<A, B> {
    A(A),
    B(B),
}
impl<A: IntoResponse, B: IntoResponse> IntoResponse for EitherResponse<A, B> {
    fn into_response(self) -> axum::response::Response {
        match self {
            EitherResponse::A(a) => a.into_response(),
            EitherResponse::B(b) => b.into_response(),
        }
    }
}
impl<A: OperationOutput, B: OperationOutput> OperationOutput for EitherResponse<A, B> {
    type Inner = A::Inner;

    fn inferred_responses(
        ctx: &mut aide::r#gen::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, aide::openapi::Response)> {
        let mut a = A::inferred_responses(ctx, operation);
        a.extend_from_slice(&B::inferred_responses(ctx, operation));
        a
    }
}

/// A finalization sent by the server to complete a WebRTC connection
#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct Finalize {
    /// The ephemeral id of the connection
    pub id: u64,
    /// A WebRTC counter-offer to complete the connection
    pub offer: JsonSchemaRTCSessionDescription,
}

/// The finalization was rejected
#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct Reject {
    /// The ephemeral id of the connection
    pub id: u64,
}

#[derive(schemars::JsonSchema)]
struct ExampleRtcSessionDescription {
    #[schemars(rename = "type")]
    sdp_type: String,
    sdp: String,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(transparent)]
pub struct JsonSchemaRTCSessionDescription(pub RTCSessionDescription);
impl schemars::JsonSchema for JsonSchemaRTCSessionDescription {
    fn schema_name() -> String {
        "RTCSessionDescription".to_string()
    }

    fn json_schema(_abc: &mut schemars::r#gen::SchemaGenerator) -> schemars::schema::Schema {
        ExampleRtcSessionDescription::json_schema(_gen)
    }
}

/// An identifier for a particular node
#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct NodeId {
    /// The id of the node
    pub id: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Offer {
    pub offer: String,
    pub account: String,
    pub target: String,
}

#[derive(Clone)]
pub struct AppState {
    /// Open connections to known accounts
    pub known_connections:
        Arc<Mutex<BTreeMap<(String, String), Sender<(u64, RTCSessionDescription)>>>>,
    /// Open connections to unknown accounts
    pub unknown_connections:
        Arc<Mutex<BTreeMap<IpAddr, Sender<(u64, RTCSessionDescription, String, String)>>>>,
    /// Pool for open client connections waiting for server finalization
    ///
    /// Modeled as a BTreeMap for quick cleanup of expired connections
    pub broker_pool: Arc<Mutex<BTreeMap<u64, Sender<RTCSessionDescription>>>>,
}

pub enum Auth {
    Unauth(Option<Basic>),
    Auth { account: String, node_id: String },
}

pub struct OpenApiSse<SSE>(pub SSE);
impl<SSE: IntoResponse> IntoResponse for OpenApiSse<SSE> {
    fn into_response(self) -> axum::response::Response {
        self.0.into_response()
    }
}
impl<SSE> OperationOutput for OpenApiSse<SSE> {
    type Inner = ();

    fn inferred_responses(
        ctx: &mut aide::r#gen::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, Response)> {
        let schema = ctx
            .schema
            .subschema_for::<JsonSchemaRTCSessionDescription>()
            .into_object();
        vec![(Some(200),aide::openapi::Response {
            content: [(
                "text/event-stream".into(),
                MediaType {
                    schema: Some(aide::openapi::SchemaObject {
                        example: None,
                        external_docs: Some(ExternalDocumentation {
                            description: Some("SSE".to_string()),
                            url: "https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events".to_string(),
                            ..Default::default()
                        }),
                        json_schema: schema.into(),
                    }),
                    ..Default::default()
                },
            )]
            .into(),
            description: "An SSE stream that will produce an offer once it is ready".to_string(),
            ..Default::default()
        })]
    }
}

#[derive(serde::Serialize, schemars::JsonSchema)]
#[serde(tag = "type")]
/// A pending connection to the server
pub enum Connection {
    #[serde(rename = "known")]
    Known {
        /// The account that owns the node
        account: String,
        /// The node_id of the node
        node_id: String,
    },
    #[serde(rename = "unknown")]
    Unknown {
        /// The ip address of the node
        ip: IpAddr,
    },
}

/// A wrapper for extractors that should no appear in the openapi docs
pub struct TransparentOperation<T>(pub T);
impl<T> OperationInput for TransparentOperation<T> {
    fn operation_input(
        ctx: &mut aide::r#gen::GenContext,
        operation: &mut aide::openapi::Operation,
    ) {
    }

    fn inferred_early_responses(
        ctx: &mut aide::r#gen::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, Response)> {
        Vec::new()
    }
}
#[async_trait]
impl<T: FromRequestParts<S>, S: Sync> FromRequestParts<S> for TransparentOperation<T> {
    type Rejection = T::Rejection;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        T::from_request_parts(parts, state)
            .await
            .map(TransparentOperation)
    }
}
