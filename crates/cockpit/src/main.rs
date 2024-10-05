//! Run with
//!
//! ```nAppState { field1: connections } -p example-sse
//! ```
//! Test with
//! ```not_rust
//! cargo test -p example-sse
//! ```

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    response::sse::{Event, Sse},
    routing::get,
    Json, Router,
};
use axum_extra::TypedHeader;
use futures::stream::{self, Stream};
use std::{
    collections::{BTreeMap, HashMap},
    convert::Infallible,
    future::Future,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::sync::{oneshot::Sender, Mutex};
use tokio_stream::StreamExt as _;
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
    account: Option<String>,
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

#[axum::debug_handler]
async fn known_connection(
    State(AppState {
        known_connections: connections,
        ..
    }): State<AppState>,
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
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

    let Some(account) = request.account else {
        tracing::warn!("no account provided");
        panic!()
    };

    {
        let mut connections = connections.lock().await;
        connections.insert((account.clone(), request.token.clone()), tx);
        tracing::debug!("got {} connections", connections.len());
    }

    let stream = FutureOnCloseStream {
        stream,
        future: async move {
            tracing::debug!("removing connection from {}", account);
            let mut connections = connections.lock().await;
            connections.remove(&(account, request.token));
        },
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(60))
            .text("keep-alive-text"),
    )
}

pin_project_lite::pin_project! {
    /// Stream adaptor to run a future once after the stream is closed
    struct FutureOnCloseStream<S, F> {
        #[pin]
        stream: S,
        #[pin]
        future: F,
    }
}

impl<S, F> Stream for FutureOnCloseStream<S, F>
where
    S: Stream,
    F: Future<Output = ()>,
{
    type Item = S::Item;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.as_mut().project();
        match this.stream.poll_next(cx) {
            std::task::Poll::Ready(Some(item)) => std::task::Poll::Ready(Some(item)),
            std::task::Poll::Ready(None) => match this.future.poll(cx) {
                std::task::Poll::Ready(_) => std::task::Poll::Ready(None),
                std::task::Poll::Pending => std::task::Poll::Pending,
            },
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
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
