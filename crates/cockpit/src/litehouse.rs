use std::{convert::Infallible, net::SocketAddr, time::Duration};

use crate::client::MissingServerError;
use crate::future_on_close_stream::FutureOnCloseStream;
use crate::types::{
    AppState, EitherResponse, Finalize, JsonSchemaRTCSessionDescription, NodeId, OpenApiSse,
    Reject, Resp,
};
use aide::axum::IntoApiResponse;
use aide::OperationOutput;
use axum::response::IntoResponse;
use axum::{
    extract::{ConnectInfo, Path, State},
    response::sse::{Event, Sse},
    Json,
};
use axum_extra::TypedHeader;
use futures::stream::{self};
use headers::{
    authorization::{Basic, Bearer},
    Authorization,
};
use tokio_stream::StreamExt;

#[derive(serde::Serialize, schemars::JsonSchema)]
pub struct AnonConnection {
    offer: JsonSchemaRTCSessionDescription,
    seed: String,
    password: String,
}

pub async fn wait_for_connection_anon(
    State(AppState {
        known_connections,
        unknown_connections,
        ..
    }): State<AppState>,
    TypedHeader(account): TypedHeader<Authorization<Basic>>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> impl IntoApiResponse {
    tracing::trace!("connection from {}", remote_addr);
    let (tx, rx) = tokio::sync::oneshot::channel();
    let stream = stream::unfold(Some(rx), |rx| async move {
        rx?.await
            .ok()
            .map(|(id, offer, seed, password): (u64, _, _, _)| {
                (
                    Ok::<_, Infallible>(
                        Event::default()
                            .data(
                                serde_json::to_string(&AnonConnection {
                                    offer: JsonSchemaRTCSessionDescription(offer),
                                    seed,
                                    password,
                                })
                                .unwrap(),
                            )
                            .id(id.to_string()),
                    ),
                    None,
                )
            })
    })
    .fuse();

    {
        tracing::trace!("adding unauth connection from {}", remote_addr);
        let mut unknown_connections = unknown_connections.lock().await;
        unknown_connections.insert(remote_addr.ip(), tx);
    }

    let completion = async move {
        tracing::trace!("removing connection from {}", remote_addr);
        let mut unknown_connections = unknown_connections.lock().await;
        unknown_connections.remove(&remote_addr.ip());
    };

    OpenApiSse(
        Sse::new(FutureOnCloseStream::new(stream, completion)).keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(60))
                .text("ka"),
        ),
    )
}

pub async fn wait_for_connection(
    State(AppState {
        known_connections,
        unknown_connections,
        ..
    }): State<AppState>,
    Path(NodeId { id: node_id }): Path<NodeId>,
    TypedHeader(Authorization(account)): TypedHeader<Authorization<Bearer>>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
) -> impl IntoApiResponse {
    tracing::trace!("connection from {}", remote_addr);
    let (tx, rx) = tokio::sync::oneshot::channel();

    let stream = stream::unfold(Some(rx), |rx| async move {
        rx?.await.ok().map(|(id, offer): (u64, _)| {
            (
                Ok::<_, Infallible>(
                    Event::default()
                        .data(serde_json::to_string(&offer).unwrap())
                        .id(id.to_string()),
                ),
                None,
            )
        })
    })
    .fuse();

    let account = account.token().to_owned();

    {
        tracing::trace!("adding auth connection from {}", account);
        let mut connections = known_connections.lock().await;
        connections.insert((account.clone(), node_id.clone()), tx);
    }

    let completion = async move {
        tracing::trace!("removing connection from {}", account);
        let mut connections = known_connections.lock().await;
        connections.remove(&(account, node_id));
    };

    OpenApiSse(
        Sse::new(FutureOnCloseStream::new(stream, completion)).keep_alive(
            axum::response::sse::KeepAlive::new()
                .interval(Duration::from_secs(60))
                .text("ka"),
        ),
    )
}

crate::macros::static_error!(
    ClientGoneError,
    "client_gone",
    "Client is no longer waiting for a connection",
    404
);

pub async fn finalize_connection(
    State(AppState { broker_pool, .. }): State<AppState>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    Json(Finalize { id, offer }): Json<Finalize>,
) -> impl IntoApiResponse {
    tracing::trace!("finalize from {} ({})", remote_addr, id);
    let result = {
        let mut broker_pool = broker_pool.lock().await;
        broker_pool.remove(&id)
    }
    .ok_or(EitherResponse::A(MissingServerError))?;

    result
        .send(offer.0)
        .map(|_| Resp::<_, 200>::new(()))
        .map_err(|_| EitherResponse::B(ClientGoneError))
}

pub async fn reject_connection(
    State(AppState { broker_pool, .. }): State<AppState>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    Json(Reject { id }): Json<Reject>,
) -> impl IntoApiResponse {
    tracing::trace!("finalize from {} ({})", remote_addr, id);
    let result = {
        let mut broker_pool = broker_pool.lock().await;
        broker_pool.remove(&id)
    };
    if let Some(result) = result {
        drop(result);
    }
    Resp::<_, 200>::new("ok")
}
