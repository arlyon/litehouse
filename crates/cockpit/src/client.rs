use std::{
    net::SocketAddr,
    ops::Bound,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::types::{
    AppState, Connection, EitherResponse, JsonSchemaRTCSessionDescription, NodeId, Resp,
    TransparentOperation,
};
use aide::{OperationOutput, axum::IntoApiResponse};
use axum::{
    Json,
    extract::{ConnectInfo, Path, State},
    response::IntoResponse,
};
use axum_client_ip::SecureClientIp;
use axum_extra::TypedHeader;
use headers::{
    Authorization,
    authorization::{Basic, Bearer},
};

pub async fn list_connections(
    State(AppState {
        known_connections,
        unknown_connections,
        ..
    }): State<AppState>,
    TypedHeader(account): TypedHeader<Authorization<Bearer>>,
    TransparentOperation(SecureClientIp(remote_addr)): TransparentOperation<SecureClientIp>,
) -> impl IntoApiResponse {
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

            connections
                .upper_bound(Bound::Excluded(&(next_account, String::new())))
                .next()
        };

        loop {
            let next = lb.next();

            match (next, ub) {
                (Some(((account, token), _)), Some(((next_account, _), _)))
                    if account != next_account =>
                {
                    items.push(Connection::Known {
                        account: account.to_owned(),
                        node_id: token.to_owned(),
                    });
                }
                (Some(((account, token), _)), None) => {
                    items.push(Connection::Known {
                        account: account.to_owned(),
                        node_id: token.to_owned(),
                    });
                }

                (None, Some(_)) => unreachable!(), // lower bound cannot move past upper bound
                (None, None) | (Some(_), Some(_)) => break,
            }
        }
    }

    {
        let unknown_connections = unknown_connections.lock().await;
        let mut unknown_lb = unknown_connections.lower_bound(Bound::Included(&remote_addr));

        loop {
            let next = unknown_lb.next();
            match next {
                Some((addr, _)) if *addr == remote_addr => {
                    items.push(Connection::Unknown { ip: *addr });
                }
                None | Some(_) => break,
            }
        }
    }

    Json(items)
}

crate::macros::static_error!(
    MissingServerError,
    "no_server",
    "There is no matching server waiting for a connection",
    400
);
crate::macros::static_error!(
    RejectedRequestError,
    "rejected",
    "The connection request was rejected by the litehouse instance",
    401
);

pub async fn client_handler_anon(
    State(AppState {
        unknown_connections,
        broker_pool,
        ..
    }): State<AppState>,
    TransparentOperation(SecureClientIp(remote_addr)): TransparentOperation<SecureClientIp>,
    TypedHeader(account): TypedHeader<Authorization<Basic>>,
    Json(JsonSchemaRTCSessionDescription(offer)): Json<JsonSchemaRTCSessionDescription>,
) -> impl IntoResponse + OperationOutput {
    let seed = account.0.username();
    let password = account.0.password();
    // TODO: validate password

    let selected = {
        let mut connections = unknown_connections.lock().await;
        tracing::info!("pairing state {:?}", connections);
        connections.remove(&remote_addr)
    }
    .ok_or(EitherResponse::A(MissingServerError))?;

    let rx = {
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
            let existing = broker_pool.get(&timestamp);
            if let Some(existing) = existing {
                if !existing.is_closed() {
                    continue;
                }
            }
            broker_pool.insert(timestamp, tx);
            selected
                .send((timestamp, offer, seed.to_owned(), password.to_owned()))
                .unwrap();
            break rx;
        }
    };

    tracing::info!("waiting for pairing reply");

    let body = match rx.await {
        Ok(body) => body,
        Err(_) => {
            tracing::info!("pairing rejected");
            return Err(EitherResponse::B(RejectedRequestError));
        }
    };

    tracing::info!("got pairing reply");

    Ok(Resp::<_, 200>::new(Json(JsonSchemaRTCSessionDescription(
        body,
    ))))
}

pub async fn client_handler(
    State(AppState {
        known_connections,
        broker_pool,
        ..
    }): State<AppState>,
    Path(NodeId { id: node_id }): Path<NodeId>,
    TypedHeader(Authorization(account)): TypedHeader<Authorization<Bearer>>,
    Json(JsonSchemaRTCSessionDescription(offer)): Json<JsonSchemaRTCSessionDescription>,
) -> impl IntoResponse + OperationOutput {
    let mut connections = known_connections.lock().await;
    tracing::info!("pairing state {:?}", connections);
    let selected = connections
        .remove(&(account.token().to_owned(), node_id.clone()))
        .ok_or(EitherResponse::A(MissingServerError))?;

    let rx = {
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
            let existing = broker_pool.get(&timestamp);
            if let Some(existing) = existing {
                if !existing.is_closed() {
                    continue;
                }
            }
            broker_pool.insert(timestamp, tx);
            selected.send((timestamp, offer)).unwrap();
            break rx;
        }
    };

    tracing::info!("waiting for pairing reply");

    rx.await
        .map(|body| {
            tracing::info!("got pairing reply");
            Json(JsonSchemaRTCSessionDescription(body))
        })
        .map_err(|_| {
            tracing::info!("pairing rejected");
            EitherResponse::B(RejectedRequestError)
        })
}
