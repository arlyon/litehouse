//! Run with
//!
//! ```nAppState { field1: connections } -p example-sse
//! ```
//! Test with
//! ```not_rust
//! cargo test -p example-sse
//! ```

#![feature(btree_cursors)]

mod future_on_close_stream;

use ::core::marker::Send;
use axum::{
    body::Body,
    extract::{ConnectInfo, FromRequestParts, Path, State},
    http::{request::Parts, StatusCode},
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    routing::{get, post, put},
    Json, Router,
};
use axum_extra::TypedHeader;
use future_on_close_stream::FutureOnCloseStream;
use futures::{
    future::Either,
    stream::{self, Stream},
};
use headers::{authorization::Bearer, Authorization};
use std::{
    collections::BTreeMap,
    convert::Infallible,
    net::{IpAddr, SocketAddr},
    ops::Bound,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::{oneshot::Sender, Mutex};
use tokio_stream::StreamExt;
use tower_http::{cors::CorsLayer, services::ServeDir, trace::TraceLayer};
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

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
        broker_pool: Arc::new(Mutex::new(Default::default())),
    };

    Router::new()
        .fallback_service(static_files_service)
        .route("/litehouse/{node_id}", post(wait_for_connection))
        .route("/litehouse/{node_id}", put(finalize_connection))
        .route("/litehouse", post(wait_for_connection))
        .route("/client", get(list_connections))
        .route("/client/{node_id}", post(client_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
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
    known_connections: Arc<Mutex<BTreeMap<(String, String), Sender<(u64, RTCSessionDescription)>>>>,
    /// Open connections to unknown accounts
    unknown_connections: Arc<Mutex<BTreeMap<IpAddr, Sender<(u64, RTCSessionDescription)>>>>,
    /// Pool for open client connections waiting for server finalization
    ///
    /// Modeled as a BTreeMap for quick cleanup of expired connections
    broker_pool: Arc<Mutex<BTreeMap<u64, Sender<RTCSessionDescription>>>>,
}

enum Auth {
    Unauth,
    Auth { account: String, token: String },
}

impl<S> FromRequestParts<S> for Auth
where
    S: Send + Sync,
{
    type Rejection = (axum::http::StatusCode, &'static str);

    async fn from_request_parts<'a, 'b>(
        parts: &'a mut Parts,
        state: &'b S,
    ) -> Result<Self, Self::Rejection> {
        let a = Path::<String>::from_request_parts(parts, state).await;
        let b = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await;

        match (a, b) {
            (Ok(a), Ok(b)) => Ok(Auth::Auth {
                account: b.0 .0.token().to_owned(),
                token: a.0,
            }),
            (Ok(_), Err(_)) => Err((axum::http::StatusCode::UNAUTHORIZED, "Missing token")),
            (Err(_), Ok(_)) => Err((axum::http::StatusCode::BAD_REQUEST, "Missing account")),
            (Err(_), Err(_)) => Ok(Auth::Unauth),
        }
    }
}

async fn wait_for_connection(
    State(AppState {
        known_connections,
        unknown_connections,
        ..
    }): State<AppState>,
    auth: Auth,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::trace!("connection from {}", remote_addr);

    let (tx, rx) = tokio::sync::oneshot::channel::<(u64, RTCSessionDescription)>();
    let stream = stream::unfold(Some(rx), |rx| async move {
        match rx?.await.ok() {
            Some((id, offer)) => Some((
                Ok(Event::default()
                    .data(serde_json::to_string(&offer).unwrap())
                    .id(id.to_string())),
                None,
            )),
            None => None,
        }
    })
    .fuse();

    let completion = match auth {
        Auth::Auth { account, token } => {
            {
                tracing::trace!("adding auth connection from {}", account);
                let mut connections = known_connections.lock().await;
                connections.insert((account.clone(), token.clone()), tx);
            }

            Either::Left(async move {
                tracing::trace!("removing connection from {}", account);
                let mut connections = known_connections.lock().await;
                connections.remove(&(account, token));
            })
        }
        _ => {
            {
                tracing::trace!("adding unauth connection from {}", remote_addr);
                let mut unknown_connections = unknown_connections.lock().await;
                unknown_connections.insert(remote_addr.ip(), tx);
            }

            Either::Right(async move {
                tracing::trace!("removing connection from {}", remote_addr);
                let mut unknown_connections = unknown_connections.lock().await;
                unknown_connections.remove(&remote_addr.ip());
            })
        }
    };

    Sse::new(FutureOnCloseStream::new(stream, completion)).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(60))
            .text("ka"),
    )
}

#[derive(serde::Deserialize)]
struct Finalize {
    id: u64,
    offer: RTCSessionDescription,
}

async fn finalize_connection(
    State(AppState { broker_pool, .. }): State<AppState>,
    auth: Auth,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    Json(body): Json<Finalize>,
) -> impl IntoResponse {
    tracing::trace!("finalize from {} ({})", remote_addr, body.id);

    {
        let mut broker_pool = broker_pool.lock().await;
        let result = broker_pool.remove(&body.id);
        let Some(result) = result else {
            return (StatusCode::NOT_FOUND, "broker not found".to_string());
        };
        result.send(body.offer).unwrap();
    }

    (StatusCode::OK, "ok".to_string())
}

#[derive(serde::Serialize)]
#[serde(tag = "type")]
enum Connection {
    #[serde(rename = "known")]
    Known { account: String, identifier: String },
    #[serde(rename = "unknown")]
    Unknown { ip: IpAddr },
}

async fn list_connections(
    State(AppState {
        known_connections,
        unknown_connections,
        ..
    }): State<AppState>,
    TypedHeader(account): TypedHeader<Authorization<Bearer>>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let mut items = vec![];
    let account = account.token().to_owned();

    {
        let connections = known_connections.lock().await;
        let mut lb = connections.lower_bound(Bound::Included(&(account.clone(), String::new())));
        let ub = {
            let mut next_account = account.clone();
            let bytes = unsafe { next_account.as_bytes_mut() };
            for b in bytes.iter_mut().rev() {
                *b = b.wrapping_add(1);
                if *b != 0 {
                    break;
                }
            }

            tracing::info!("{} -> {}", account, next_account);

            connections
                .upper_bound(Bound::Excluded(&(next_account, String::new())))
                .next()
        };

        loop {
            let next = lb.next();
            tracing::trace!(
                "next: {:?} {:?}",
                next.as_ref().map(|(a, b)| a),
                ub.as_ref().map(|(a, b)| a)
            );
            match (next, ub) {
                (Some(((account, token), _)), Some(((next_account, _), _)))
                    if account != next_account =>
                {
                    items.push(Connection::Known {
                        account: account.to_owned(),
                        identifier: token.to_owned(),
                    });
                }
                (Some(((account, token), _)), None) => {
                    tracing::trace!("adding unknown connection from {}", account);
                    items.push(Connection::Known {
                        account: account.to_owned(),
                        identifier: token.to_owned(),
                    });
                }

                (None, Some(_)) => unreachable!(), // lower bound cannot move past upper bound
                (None, None) | (Some(_), Some(_)) => break,
            }
        }
    }

    {
        let unknown_connections = unknown_connections.lock().await;
        let mut unknown_lb = unknown_connections.lower_bound(Bound::Included(&remote_addr.ip()));

        loop {
            let next = unknown_lb.next();
            match next {
                Some((addr, _)) if *addr == remote_addr.ip() => {
                    items.push(Connection::Unknown { ip: *addr });
                }
                None | Some(_) => break,
            }
        }
    }

    Json(items)
}

async fn client_handler(
    State(AppState {
        known_connections,
        broker_pool,
        ..
    }): State<AppState>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    TypedHeader(account): TypedHeader<Authorization<Bearer>>,
    Path(node_id): Path<String>,
    Json(offer): Json<RTCSessionDescription>,
) -> impl IntoResponse {
    let mut connections = known_connections.lock().await;
    let account = account.token().to_owned();
    let Some(selected) = connections.remove(&(account.clone(), node_id.clone())) else {
        return (StatusCode::NOT_FOUND, "account not found".to_string());
    };

    let x = {
        let mut broker_pool = broker_pool.lock().await;

        loop {
            // hard limit on 1 billion connections a second
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                // there are 10^15 nanos in 54y since epoch
                // and 10^19 nanos in a u64. this will
                // overflow in roughly 50_000 years
                .as_nanos() as u64;

            let (tx, rx) = tokio::sync::oneshot::channel();
            broker_pool.insert(timestamp, tx);
            selected.send((timestamp, offer)).unwrap();
            break rx;
        }
    };

    let x = x.await.unwrap();
    let body = serde_json::to_string(&x).unwrap();

    tracing::info!("connected {} to {}", remote_addr, node_id);

    (StatusCode::OK, body)
}
