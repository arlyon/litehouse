use super::*;
use bytes::Bytes;
use futures::{StreamExt, stream::BoxStream};
use http::{HeaderName, HeaderValue, Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use rmcp::{
    ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolRequestParams, CallToolResult, ClientInfo, ClientJsonRpcMessage,
        ServerCapabilities, ServerInfo, ServerJsonRpcMessage,
    },
    tool, tool_handler, tool_router,
    transport::{
        StreamableHttpClientTransport,
        streamable_http_client::{
            StreamableHttpClient, StreamableHttpClientTransportConfig, StreamableHttpError,
            StreamableHttpPostResponse,
        },
    },
};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use sse_stream::{Sse, SseStream};
use std::{borrow::Cow, collections::HashMap, sync::Arc};

const ALLOWED_ORIGIN: &str = "http://127.0.0.1:3000";

#[derive(Clone)]
struct ClosedTools {
    tool_router: ToolRouter<Self>,
}

impl ClosedTools {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[derive(Deserialize, schemars::JsonSchema)]
struct InventoryInput {
    timeout_ms: Option<u32>,
}

#[tool_router]
impl ClosedTools {
    #[tool(name = "litehouse_inventory", description = "inventory")]
    async fn inventory(&self, Parameters(input): Parameters<InventoryInput>) -> CallToolResult {
        CallToolResult::structured(json!({"timeout_ms": input.timeout_ms, "instances": []}))
    }

    #[tool(name = "litehouse_query", description = "query")]
    async fn query(&self) -> CallToolResult {
        CallToolResult::structured(json!({"state": "closed"}))
    }

    #[tool(name = "litehouse_interact", description = "interact")]
    async fn interact(&self) -> CallToolResult {
        CallToolResult::structured(json!({"accepted": true}))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for ClosedTools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}

#[derive(Clone, Debug)]
struct HyperTestClient;

#[derive(Debug, thiserror::Error)]
enum HyperTestError {
    #[error(transparent)]
    Http(#[from] http::Error),
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("URI has no authority")]
    MissingAuthority,
}

async fn send(request: Request<Full<Bytes>>) -> Result<Response<Incoming>, HyperTestError> {
    let authority = request
        .uri()
        .authority()
        .ok_or(HyperTestError::MissingAuthority)?
        .clone();
    let stream = tokio::net::TcpStream::connect(authority.as_str()).await?;
    let (mut sender, connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(stream)).await?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    Ok(sender.send_request(request).await?)
}

fn request(
    method: Method,
    uri: &str,
    body: Bytes,
    custom_headers: HashMap<HeaderName, HeaderValue>,
) -> Result<http::request::Builder, StreamableHttpError<HyperTestError>> {
    let parsed = uri
        .parse::<http::Uri>()
        .map_err(|error| StreamableHttpError::Client(HyperTestError::Http(error.into())))?;
    let authority = parsed
        .authority()
        .cloned()
        .ok_or_else(|| StreamableHttpError::Client(HyperTestError::MissingAuthority))?;
    let mut builder = Request::builder()
        .method(method)
        .uri(parsed)
        .header(http::header::HOST, authority.as_str())
        .header(http::header::ACCEPT, "application/json, text/event-stream");
    for (name, value) in custom_headers {
        builder = builder.header(name, value);
    }
    let _ = body;
    Ok(builder)
}

impl StreamableHttpClient for HyperTestClient {
    type Error = HyperTestError;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        _auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        let body = Bytes::from(
            serde_json::to_vec(&message)
                .map_err(HyperTestError::from)
                .map_err(StreamableHttpError::Client)?,
        );
        let mut builder = request(Method::POST, &uri, body.clone(), custom_headers)?
            .header(http::header::CONTENT_TYPE, "application/json");
        let session_was_attached = session_id.is_some();
        if let Some(session_id) = session_id {
            builder = builder.header("mcp-session-id", session_id.as_ref());
        }
        let response = send(
            builder
                .body(Full::new(body))
                .map_err(HyperTestError::from)
                .map_err(StreamableHttpError::Client)?,
        )
        .await
        .map_err(StreamableHttpError::Client)?;
        let status = response.status();
        if matches!(status, StatusCode::ACCEPTED | StatusCode::NO_CONTENT) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }
        if status == StatusCode::NOT_FOUND && session_was_attached {
            return Err(StreamableHttpError::SessionExpired);
        }
        if !status.is_success() {
            return Err(StreamableHttpError::UnexpectedServerResponse(Cow::Owned(
                format!("HTTP {status}"),
            )));
        }
        let session_id = response
            .headers()
            .get("mcp-session-id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let content_type = response
            .headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        match content_type.as_deref() {
            Some(value) if value.starts_with("text/event-stream") => {
                Ok(StreamableHttpPostResponse::Sse(
                    SseStream::new(response.into_body()).boxed(),
                    session_id,
                ))
            }
            Some(value) if value.starts_with("application/json") => {
                let bytes = response
                    .into_body()
                    .collect()
                    .await
                    .map_err(HyperTestError::from)
                    .map_err(StreamableHttpError::Client)?
                    .to_bytes();
                let message = serde_json::from_slice::<ServerJsonRpcMessage>(&bytes)?;
                Ok(StreamableHttpPostResponse::Json(message, session_id))
            }
            _ => Err(StreamableHttpError::UnexpectedContentType(content_type)),
        }
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        _auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        let response = send(
            request(Method::DELETE, &uri, Bytes::new(), custom_headers)?
                .header("mcp-session-id", session_id.as_ref())
                .body(Full::new(Bytes::new()))
                .map_err(HyperTestError::from)
                .map_err(StreamableHttpError::Client)?,
        )
        .await
        .map_err(StreamableHttpError::Client)?;
        if response.status().is_success() || response.status() == StatusCode::METHOD_NOT_ALLOWED {
            Ok(())
        } else {
            Err(StreamableHttpError::UnexpectedServerResponse(Cow::Owned(
                format!("HTTP {}", response.status()),
            )))
        }
    }

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        _auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<BoxStream<'static, Result<Sse, sse_stream::Error>>, StreamableHttpError<Self::Error>>
    {
        let mut builder = request(Method::GET, &uri, Bytes::new(), custom_headers)?
            .header("mcp-session-id", session_id.as_ref());
        if let Some(last_event_id) = last_event_id {
            builder = builder.header("last-event-id", last_event_id);
        }
        let response = send(
            builder
                .body(Full::new(Bytes::new()))
                .map_err(HyperTestError::from)
                .map_err(StreamableHttpError::Client)?,
        )
        .await
        .map_err(StreamableHttpError::Client)?;
        if response.status() == StatusCode::METHOD_NOT_ALLOWED {
            return Err(StreamableHttpError::ServerDoesNotSupportSse);
        }
        if !response.status().is_success() {
            return Err(StreamableHttpError::UnexpectedServerResponse(Cow::Owned(
                format!("HTTP {}", response.status()),
            )));
        }
        Ok(SseStream::new(response.into_body()).boxed())
    }
}

fn client_transport(
    address: SocketAddr,
    origin: &str,
) -> StreamableHttpClientTransport<HyperTestClient> {
    let mut headers = HashMap::new();
    headers.insert(http::header::ORIGIN, HeaderValue::from_str(origin).unwrap());
    StreamableHttpClientTransport::with_client(
        HyperTestClient,
        StreamableHttpClientTransportConfig::with_uri(format!("http://{address}{MCP_ROUTE}"))
            .custom_headers(headers),
    )
}

fn config(max_sessions: usize) -> McpServerConfig {
    let mut config = McpServerConfig::new("127.0.0.1:0".parse().unwrap(), [ALLOWED_ORIGIN]);
    config.max_sessions = max_sessions;
    config.idle_timeout = Duration::from_secs(10);
    config.shutdown_grace = Duration::from_secs(2);
    config
}

fn arguments(value: Value) -> Map<String, Value> {
    value.as_object().unwrap().clone()
}

#[tokio::test]
async fn rmcp_client_initializes_lists_calls_and_rejects_unknown_tool() -> anyhow::Result<()> {
    let server = start_service(config(2), || Ok(ClosedTools::new())).await?;
    let client = ClientInfo::default()
        .serve(client_transport(server.local_addr(), ALLOWED_ORIGIN))
        .await?;

    assert!(
        client.peer().peer_info().is_some(),
        "initialize must complete"
    );
    let tools = client.list_tools(None).await?.tools;
    let names = tools
        .iter()
        .map(|tool| tool.name.as_ref())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        names,
        std::collections::BTreeSet::from([
            "litehouse_interact",
            "litehouse_inventory",
            "litehouse_query",
        ])
    );

    let result = client
        .call_tool(
            CallToolRequestParams::new("litehouse_inventory")
                .with_arguments(arguments(json!({"timeout_ms": 42}))),
        )
        .await?;
    assert_eq!(
        result.structured_content,
        Some(json!({"timeout_ms": 42, "instances": []}))
    );
    assert!(
        client
            .call_tool(CallToolRequestParams::new("litehouse_generic"))
            .await
            .is_err()
    );

    client.cancel().await?;
    assert_eq!(server.shutdown().await?.closed_sessions, 0);
    Ok(())
}

#[tokio::test]
async fn session_capacity_and_shutdown_are_bounded() -> anyhow::Result<()> {
    let server = start_service(config(1), || Ok(ClosedTools::new())).await?;
    let address = server.local_addr();
    let client = ClientInfo::default()
        .serve(client_transport(address, ALLOWED_ORIGIN))
        .await?;
    assert_eq!(server.active_sessions().await, 1);

    let second = tokio::time::timeout(
        Duration::from_secs(2),
        ClientInfo::default().serve(client_transport(address, ALLOWED_ORIGIN)),
    )
    .await?;
    assert!(second.is_err(), "a second active session must be rejected");
    assert_eq!(server.active_sessions().await, 1);

    let report = server.shutdown().await?;
    assert_eq!(report.closed_sessions, 1);
    assert!(
        tokio::time::timeout(Duration::from_secs(1), client.list_tools(None))
            .await?
            .is_err(),
        "requests after shutdown must fail"
    );
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
    Ok(())
}

#[tokio::test]
async fn health_is_live_and_readiness_is_explicit() -> anyhow::Result<()> {
    let server = start_service(config(4), || Ok(ClosedTools::new())).await?;
    let health_uri = format!("http://{}{}", server.local_addr(), HEALTH_ROUTE);
    let ready_uri = format!("http://{}{}", server.local_addr(), READINESS_ROUTE);

    let health = send(Request::get(&health_uri).body(Full::new(Bytes::new()))?).await?;
    assert_eq!(health.status(), StatusCode::OK);
    let not_ready = send(Request::get(&ready_uri).body(Full::new(Bytes::new()))?).await?;
    assert_eq!(not_ready.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(!server.is_ready());

    server.set_ready(true);
    assert!(server.is_ready());
    let ready = send(Request::get(&ready_uri).body(Full::new(Bytes::new()))?).await?;
    assert_eq!(ready.status(), StatusCode::OK);

    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn oversized_fixed_and_chunked_requests_are_rejected() -> anyhow::Result<()> {
    let mut bounded = config(4);
    bounded.max_request_bytes = 128;
    let server = start_service(bounded, || Ok(ClosedTools::new())).await?;
    let uri = format!("http://{}{MCP_ROUTE}", server.local_addr());

    for chunked in [false, true] {
        let mut builder = Request::builder()
            .method(Method::POST)
            .uri(&uri)
            .header(http::header::HOST, server.local_addr().to_string())
            .header(http::header::ORIGIN, ALLOWED_ORIGIN)
            .header(http::header::CONTENT_TYPE, "application/json")
            .header(http::header::ACCEPT, "application/json, text/event-stream");
        if chunked {
            builder = builder.header(http::header::TRANSFER_ENCODING, "chunked");
        }
        let response = send(builder.body(Full::new(Bytes::from(vec![b'x'; 129])))?).await?;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    server.shutdown().await?;
    Ok(())
}

#[tokio::test]
async fn disallowed_origin_cannot_initialize() -> anyhow::Result<()> {
    let server = start_service(config(1), || Ok(ClosedTools::new())).await?;
    let attempt = ClientInfo::default()
        .serve(client_transport(
            server.local_addr(),
            "http://untrusted.example",
        ))
        .await;
    assert!(attempt.is_err());
    assert_eq!(server.active_sessions().await, 0);
    server.shutdown().await?;
    Ok(())
}
