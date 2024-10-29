// pin project lite has some specific syntax requirements
#![allow(clippy::multiple_bound_locations)]
#![feature(btree_cursors)]

mod client;
mod docs;
mod future_on_close_stream;
mod litehouse;
mod macros;
mod scalar;
mod types;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use aide::{
    axum::{
        routing::{get_with, post_with},
        ApiRouter,
    },
    openapi::OpenApi,
};
use axum::{Extension, Router};
use axum_client_ip::SecureClientIpSource;
use client::{client_handler, client_handler_anon, list_connections};
use docs::{api_docs, docs_routes};
use litehouse::{
    finalize_connection, reject_connection, wait_for_connection, wait_for_connection_anon,
};
use tokio::sync::Mutex;
use tower::Layer;
use tower_governor::{governor::GovernorConfig, GovernorLayer};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use types::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // build our application
    let app = app();
    let port = std::env::var("PORT")
        .map(|p| p.parse().unwrap())
        .unwrap_or_else(|_| 3000u16);

    // run it
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
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
    aide::gen::on_error(|error| {
        tracing::error!("{error}");
    });

    aide::gen::extract_schemas(true);
    aide::gen::infer_responses(true);

    let state = AppState {
        known_connections: Arc::new(Mutex::new(Default::default())),
        unknown_connections: Arc::new(Mutex::new(Default::default())),
        broker_pool: Arc::new(Mutex::new(Default::default())),
    };

    let mut api = OpenApi::default();

    // Default config, allow 2 requests per second, bursts up to 8
    // Should prevent most DoS or abuse
    let governor_conf = Arc::new(GovernorConfig::default());

    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(60);
    // a separate background task to clean up
    std::thread::spawn(move || loop {
        std::thread::sleep(interval);
        tracing::info!("rate limiting storage size: {}", governor_limiter.len());
        governor_limiter.retain_recent();
    });

    ApiRouter::new()
        .api_route(
            "/litehouse",
            get_with(wait_for_connection_anon, |d| {
                d.description("Wait for a connection from a client on the local network")
            }),
        )
        .api_route(
            "/litehouse",
            post_with(finalize_connection, |d| {
                d.description("Finalize a pending connection")
            })
            .delete_with(reject_connection, |d| {
                d.description("Reject a pending connection")
            })
            .layer(GovernorLayer {
                config: governor_conf,
            }),
        )
        .api_route(
            "/litehouse/:id",
            get_with(wait_for_connection, |d| {
                d.description("Wait for a connection from a previously-paired client")
            }),
        )
        .api_route(
            "/client",
            get_with(list_connections, |d| {
                d.description("Get all connections available to a given client")
                    .security_requirement("test")
            })
            .post_with(client_handler_anon, |d| {
                d.description("Start a new connection to an anonymous litehouse instance")
            }),
        )
        .api_route(
            "/client/:id",
            post_with(client_handler, |d| {
                d.description("Start a new connection to a previously-paired litehouse instance")
            }),
        )
        .nest_api_service("/docs", docs_routes(state.clone()))
        .finish_api_with(&mut api, api_docs)
        .layer(TraceLayer::new_for_http())
        .layer(SecureClientIpSource::RightmostXForwardedFor.into_extension())
        .layer(CorsLayer::permissive())
        .layer(Extension(Arc::new(api)))
        .with_state(state)
}
