//! Run with
//!
//! ```nAppState { field1: connections } -p example-sse
//! ```
//! Test with
//! ```not_rust
//! cargo test -p example-sse
//! ```

mod future_on_close_stream;

use ::core::{future::Future, marker::Send, pin::Pin};
use axum::{
    body::Body,
    extract::{ConnectInfo, FromRequestParts, Path, State},
    http::request::Parts,
    response::sse::{Event, Sse},
    routing::get,
    Json, Router,
};
use axum_extra::TypedHeader;
use future_on_close_stream::FutureOnCloseStream;
use futures::stream::{self, Stream};
use std::{
    collections::BTreeMap, convert::Infallible, net::SocketAddr, path::PathBuf, sync::Arc,
    time::Duration,
};
use tokio::sync::{oneshot::Sender, Mutex};
use tower_http::{services::ServeDir, trace::TraceLayer};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // build our application
    let app = app();
    let port = std::env::var("PORT")
        .map(|p| p.parse().unwrap())
        .unwrap_or_else(|_| 3000u16);

    // run it
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

fn app() -> Router {
    let assets_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    let static_files_service = ServeDir::new(assets_dir).append_index_html_on_directories(true);
    let state = AppState {
        known_connections: Arc::new(Mutex::new(Default::default())),
        unknown_connections: Arc::new(Mutex::new(Default::default())),
    };
    Router::new()
        .fallback_service(static_files_service)
        .route("/litehouse/{account}", get(known_connection))
        .route("/litehouse", get(known_connection))
        .route("/client", get(client_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// To open a connection, the client sends a `GET` request to `/litehouse` with
/// a token
#[derive(serde::Deserialize, serde::Serialize)]
struct SseOpen {
    // server-issued JWT with the identifier
    token: String,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
struct Offer {
    offer: String,
    account: String,
    target: String,
}

#[derive(Clone)]
struct AppState {
    /// Open connections to known accounts
    known_connections: Arc<Mutex<BTreeMap<(String, String), Sender<Offer>>>>,
    /// Open connections to unknown accounts
    unknown_connections: Arc<Mutex<BTreeMap<SocketAddr, Sender<Offer>>>>,
}

enum Auth {
    Unauth,
    Auth(
        Path<String>,
        TypedHeader<headers::Authorization<headers::authorization::Bearer>>,
    ),
}

impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = ();

    async fn from_request_parts<'a, 'b>(
        parts: &'a mut Parts,
        state: &'b S,
    ) -> Result<Self, Self::Rejection> {
        let a = Path::<String>::from_request_parts(parts, state).await;
        let b = TypedHeader::<headers::Authorization<headers::authorization::Bearer>>::from_request_parts(
            parts, state,
        ).await;

        match (a, b) {
            (Ok(a), Ok(b)) => Ok(Auth::Auth(a, b)),
            (Ok(_), Err(_)) => Ok(Auth::Unauth),
            (Err(_), Ok(_)) => Ok(Auth::Unauth),
            (Err(_), Err(_)) => Err(()),
        }
    }
}

#[axum::debug_handler]
async fn known_connection(
    State(AppState {
        known_connections: connections,
        ..
    }): State<AppState>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
    account: Option<Path<String>>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    Json(request): Json<SseOpen>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!("`{}` connected from {}", user_agent, remote_addr);

    let (tx, rx) = tokio::sync::oneshot::channel();
    let stream = stream::unfold(Some(rx), |rx| async move {
        match rx?.await.ok() {
            Some(offer) => Some((
                Ok(Event::default().data(serde_json::to_string(&offer).unwrap())),
                None,
            )),
            None => None,
        }
    });

    let Some(Path(account)) = account else {
        tracing::warn!("no account provided");
        panic!()
    };

    {
        let mut connections = connections.lock().await;
        connections.insert((account.clone(), request.token.clone()), tx);
        tracing::debug!("got {} connections", connections.len());
    }

    let stream = FutureOnCloseStream::new(stream, async move {
        tracing::debug!("removing connection from {}", account);
        let mut connections = connections.lock().await;
        connections.remove(&(account, request.token));
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(60))
            .text("keep-alive-text"),
    )
}

async fn client_handler(
    State(AppState {
        known_connections: connections,
        ..
    }): State<AppState>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    Json(offer): Json<Offer>,
) -> Body {
    let mut connections = connections.lock().await;
    let selected = connections
        .remove(&(offer.account.clone(), offer.target.clone()))
        .expect("account not found");

    selected.send(offer);
    Body::empty()
}
