//! Bounded Streamable HTTP transport for the closed Litehouse MCP adapter.
//!
//! This crate intentionally provides no authentication. Any client that can reach the
//! configured socket can use the three tools exposed by [`McpAdapter`]. The adapter factory is
//! the single seam reserved for later authentication middleware and principal injection.

use axum::Router;
use litehouse_management::McpAdapter;
use rmcp::{
    RoleServer,
    transport::streamable_http_server::{
        RestoreOutcome, SessionId, SessionManager, StreamableHttpServerConfig,
        StreamableHttpService,
        session::local::{LocalSessionManager, LocalSessionManagerError},
    },
};
use std::{collections::HashMap, io, net::SocketAddr, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::{
    net::TcpListener,
    sync::{Mutex, OwnedSemaphorePermit, Semaphore, TryAcquireError},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

pub const MCP_ROUTE: &str = "/mcp";

/// Per-session adapter construction is the only extension seam in this transport crate.
///
/// T13a callers intentionally capture the proving principal in this factory. A later
/// authenticated release can replace that construction boundary with principal-aware
/// middleware without adding a second MCP route or changing the closed tool adapter.
pub type McpAdapterFactory = Arc<dyn Fn() -> io::Result<McpAdapter> + Send + Sync>;

#[derive(Clone, Debug)]
pub struct McpServerConfig {
    pub bind_addr: SocketAddr,
    pub allowed_hosts: Vec<String>,
    pub allowed_origins: Vec<String>,
    pub max_sessions: usize,
    pub idle_timeout: Duration,
    pub shutdown_grace: Duration,
}

impl McpServerConfig {
    pub fn new(
        bind_addr: SocketAddr,
        allowed_origins: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            bind_addr,
            allowed_hosts: vec![bind_addr.ip().to_string()],
            allowed_origins: allowed_origins.into_iter().map(Into::into).collect(),
            max_sessions: 32,
            idle_timeout: Duration::from_secs(5 * 60),
            shutdown_grace: Duration::from_secs(10),
        }
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.max_sessions == 0 {
            return Err(ConfigError::ZeroMaxSessions);
        }
        if self.idle_timeout.is_zero() {
            return Err(ConfigError::ZeroIdleTimeout);
        }
        if self.shutdown_grace.is_zero() {
            return Err(ConfigError::ZeroShutdownGrace);
        }
        if self.allowed_hosts.is_empty() {
            return Err(ConfigError::EmptyAllowedHosts);
        }
        for host in &self.allowed_hosts {
            if host.trim().is_empty() || host.contains('*') {
                return Err(ConfigError::InvalidAllowedHost(host.clone()));
            }
        }
        if self.allowed_origins.is_empty() {
            return Err(ConfigError::EmptyAllowedOrigins);
        }
        for origin in &self.allowed_origins {
            validate_origin(origin)?;
        }
        Ok(())
    }
}

fn validate_origin(origin: &str) -> Result<(), ConfigError> {
    if origin.contains('*') {
        return Err(ConfigError::WildcardOrigin(origin.to_owned()));
    }
    if origin == "null" {
        return Ok(());
    }
    let uri = origin
        .parse::<axum::http::Uri>()
        .map_err(|_| ConfigError::InvalidOrigin(origin.to_owned()))?;
    let valid_scheme = matches!(uri.scheme_str(), Some("http" | "https"));
    let origin_only = matches!(uri.path(), "" | "/") && uri.query().is_none();
    if !valid_scheme || uri.authority().is_none() || !origin_only {
        return Err(ConfigError::InvalidOrigin(origin.to_owned()));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("max_sessions must be greater than zero")]
    ZeroMaxSessions,
    #[error("idle_timeout must be greater than zero")]
    ZeroIdleTimeout,
    #[error("shutdown_grace must be greater than zero")]
    ZeroShutdownGrace,
    #[error("allowed_hosts must not be empty")]
    EmptyAllowedHosts,
    #[error("allowed host must be non-empty and cannot contain a wildcard: {0}")]
    InvalidAllowedHost(String),
    #[error("allowed_origins must not be empty")]
    EmptyAllowedOrigins,
    #[error("wildcard origins are forbidden: {0}")]
    WildcardOrigin(String),
    #[error("allowed origin must be an http(s) origin without a path, query, or fragment: {0}")]
    InvalidOrigin(String),
}

#[derive(Debug, Error)]
pub enum StartError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("failed to bind MCP listener at {address}: {source}")]
    Bind {
        address: SocketAddr,
        #[source]
        source: io::Error,
    },
}

#[derive(Debug, Error)]
pub enum ShutdownError {
    #[error("MCP server task failed: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("MCP server failed: {0}")]
    Serve(#[from] io::Error),
    #[error("MCP server exceeded its graceful shutdown deadline after {0:?}")]
    Timeout(Duration),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownReport {
    pub closed_sessions: usize,
}

pub struct McpServer {
    local_addr: SocketAddr,
    cancellation: CancellationToken,
    sessions: Arc<BoundedSessionManager>,
    shutdown_grace: Duration,
    task: JoinHandle<io::Result<()>>,
}

impl McpServer {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    pub async fn active_sessions(&self) -> usize {
        self.sessions.active_count().await
    }

    pub async fn shutdown(self) -> Result<ShutdownReport, ShutdownError> {
        self.cancellation.cancel();
        let closed_sessions = self.sessions.close_all().await;
        let mut task = self.task;
        match tokio::time::timeout(self.shutdown_grace, &mut task).await {
            Ok(result) => result??,
            Err(_) => {
                task.abort();
                let _ = task.await;
                return Err(ShutdownError::Timeout(self.shutdown_grace));
            }
        }
        Ok(ShutdownReport { closed_sessions })
    }
}

/// Start the unauthenticated MCP endpoint using the existing closed [`McpAdapter`].
///
/// There is deliberately no authorization fallback: the factory decides which principal the
/// adapter carries, and every reachable client receives that authority.
pub async fn start(
    config: McpServerConfig,
    factory: McpAdapterFactory,
) -> Result<McpServer, StartError> {
    start_service(config, move || factory()).await
}

async fn start_service<S>(
    config: McpServerConfig,
    factory: impl Fn() -> io::Result<S> + Send + Sync + 'static,
) -> Result<McpServer, StartError>
where
    S: rmcp::Service<RoleServer> + Send + 'static,
{
    config.validate()?;
    let listener = TcpListener::bind(config.bind_addr)
        .await
        .map_err(|source| StartError::Bind {
            address: config.bind_addr,
            source,
        })?;
    let local_addr = listener.local_addr().map_err(|source| StartError::Bind {
        address: config.bind_addr,
        source,
    })?;

    let cancellation = CancellationToken::new();
    let sessions = Arc::new(BoundedSessionManager::new(
        config.max_sessions,
        config.idle_timeout,
    ));
    let service = StreamableHttpService::new(
        factory,
        sessions.clone(),
        StreamableHttpServerConfig::default()
            .with_allowed_hosts(config.allowed_hosts.clone())
            .with_allowed_origins(config.allowed_origins.clone())
            .with_cancellation_token(cancellation.child_token()),
    );
    let router = Router::new().nest_service(MCP_ROUTE, service);

    tracing::warn!(
        target: "litehouse_mcp_server::security",
        bind_addr = %local_addr,
        route = MCP_ROUTE,
        authentication = "none",
        access_model = "if_reachable_usable",
        max_sessions = config.max_sessions,
        idle_timeout_ms = config.idle_timeout.as_millis(),
        allowed_origins = ?config.allowed_origins,
        "SECURITY WARNING: starting unauthenticated Litehouse MCP endpoint; every reachable client can use its configured principal"
    );

    let shutdown = cancellation.clone();
    let task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(async move { shutdown.cancelled_owned().await })
            .await
    });

    Ok(McpServer {
        local_addr,
        cancellation,
        sessions,
        shutdown_grace: config.shutdown_grace,
        task,
    })
}

#[derive(Debug, Error)]
enum BoundedSessionError {
    #[error("maximum active MCP sessions reached")]
    Capacity,
    #[error(transparent)]
    Local(#[from] LocalSessionManagerError),
}

#[derive(Debug)]
struct BoundedSessionManager {
    inner: LocalSessionManager,
    capacity: Arc<Semaphore>,
    permits: Mutex<HashMap<SessionId, OwnedSemaphorePermit>>,
}

impl BoundedSessionManager {
    fn new(max_sessions: usize, idle_timeout: Duration) -> Self {
        let mut inner = LocalSessionManager::default();
        inner.session_config.keep_alive = Some(idle_timeout);
        inner.session_config.init_timeout = Some(idle_timeout);
        Self {
            inner,
            capacity: Arc::new(Semaphore::new(max_sessions)),
            permits: Mutex::new(HashMap::with_capacity(max_sessions)),
        }
    }

    async fn active_count(&self) -> usize {
        self.permits.lock().await.len()
    }

    async fn close_all(&self) -> usize {
        let ids = self
            .permits
            .lock()
            .await
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for id in &ids {
            let _ = SessionManager::close_session(self, id).await;
        }
        ids.len()
    }
}

impl SessionManager for BoundedSessionManager {
    type Error = BoundedSessionError;
    type Transport = <LocalSessionManager as SessionManager>::Transport;

    async fn create_session(&self) -> Result<(SessionId, Self::Transport), Self::Error> {
        let permit = self
            .capacity
            .clone()
            .try_acquire_owned()
            .map_err(|error| match error {
                TryAcquireError::NoPermits | TryAcquireError::Closed => {
                    BoundedSessionError::Capacity
                }
            })?;
        let (id, transport) = self.inner.create_session().await?;
        self.permits.lock().await.insert(id.clone(), permit);
        Ok((id, transport))
    }

    async fn initialize_session(
        &self,
        id: &SessionId,
        message: rmcp::model::ClientJsonRpcMessage,
    ) -> Result<rmcp::model::ServerJsonRpcMessage, Self::Error> {
        Ok(self.inner.initialize_session(id, message).await?)
    }

    async fn has_session(&self, id: &SessionId) -> Result<bool, Self::Error> {
        Ok(self.inner.has_session(id).await?)
    }

    async fn close_session(&self, id: &SessionId) -> Result<(), Self::Error> {
        let result = self.inner.close_session(id).await;
        self.permits.lock().await.remove(id);
        result.map_err(Into::into)
    }

    async fn create_stream(
        &self,
        id: &SessionId,
        message: rmcp::model::ClientJsonRpcMessage,
    ) -> Result<
        impl futures_core::Stream<
            Item = rmcp::transport::streamable_http_server::session::ServerSseMessage,
        > + Send
        + Sync
        + 'static,
        Self::Error,
    > {
        Ok(self.inner.create_stream(id, message).await?)
    }

    async fn accept_message(
        &self,
        id: &SessionId,
        message: rmcp::model::ClientJsonRpcMessage,
    ) -> Result<(), Self::Error> {
        Ok(self.inner.accept_message(id, message).await?)
    }

    async fn create_standalone_stream(
        &self,
        id: &SessionId,
    ) -> Result<
        impl futures_core::Stream<
            Item = rmcp::transport::streamable_http_server::session::ServerSseMessage,
        > + Send
        + Sync
        + 'static,
        Self::Error,
    > {
        Ok(self.inner.create_standalone_stream(id).await?)
    }

    async fn resume(
        &self,
        id: &SessionId,
        last_event_id: String,
    ) -> Result<
        impl futures_core::Stream<
            Item = rmcp::transport::streamable_http_server::session::ServerSseMessage,
        > + Send
        + Sync
        + 'static,
        Self::Error,
    > {
        Ok(self.inner.resume(id, last_event_id).await?)
    }

    async fn restore_session(
        &self,
        _id: SessionId,
    ) -> Result<RestoreOutcome<Self::Transport>, Self::Error> {
        Ok(RestoreOutcome::NotSupported)
    }
}

#[cfg(test)]
mod transport_tests;

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn wildcard_and_non_origin_values_are_rejected() {
        let address = "127.0.0.1:0".parse().unwrap();
        for origin in [
            "*",
            "https://*.example.test",
            "example.test",
            "https://example.test/path",
        ] {
            let error = McpServerConfig::new(address, [origin])
                .validate()
                .unwrap_err();
            assert!(matches!(
                error,
                ConfigError::WildcardOrigin(_) | ConfigError::InvalidOrigin(_)
            ));
        }
    }

    #[test]
    fn exact_http_origins_are_accepted() {
        let address = "127.0.0.1:0".parse().unwrap();
        McpServerConfig::new(
            address,
            ["http://127.0.0.1:3000", "https://litehouse.example"],
        )
        .validate()
        .unwrap();
    }
}
