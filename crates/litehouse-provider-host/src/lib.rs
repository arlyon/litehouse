//! Store-local Wasmtime adapter for `litehouse:provider@1` components.
//!
//! Guest resource handles remain in the actor that owns their store. Public
//! inventory contains only host-owned identities and descriptors.

use bytes::Bytes;
use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
use litehouse_contracts::identity::Version;
use litehouse_service_fabric as fabric;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    fmt,
    future::Future,
    net::SocketAddr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant as StdInstant},
};
use tokio::sync::{mpsc, oneshot};
use wasmtime::component::{
    Component as WasmComponent, HasSelf, Linker, ResourceAny, ResourceTable,
};
use wasmtime::{Engine, Store};
use wasmtime_wasi::{
    HostMonotonicClock, TrappableError, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView,
    sockets::SocketAddrUse,
};
use wasmtime_wasi_http::{
    WasiHttpCtx,
    p3::{
        RequestOptions, WasiHttpCtxView, WasiHttpHooks, WasiHttpView,
        bindings::http::types::ErrorCode, default_send_request,
    },
};

const MAX_FACETS: usize = 512;
const ACTOR_QUEUE_CAPACITY: usize = 64;
const EPOCH_TICK: Duration = Duration::from_millis(10);
static ERROR_SEQUENCE: AtomicU64 = AtomicU64::new(1);

fn tcp_address_allowed(
    allowed: &BTreeSet<SocketAddr>,
    address: SocketAddr,
    usage: SocketAddrUse,
) -> bool {
    matches!(usage, SocketAddrUse::TcpConnect) && allowed.contains(&address)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HttpOrigin {
    scheme: String,
    host: String,
    port: u16,
}

impl HttpOrigin {
    pub fn new(scheme: &str, host: &str, port: u16) -> Result<Self, &'static str> {
        let scheme = scheme.to_ascii_lowercase();
        let host = host.to_ascii_lowercase();
        if !matches!(scheme.as_str(), "http" | "https")
            || host.is_empty()
            || host.len() > 253
            || !host.is_ascii()
            || host
                .bytes()
                .any(|byte| byte.is_ascii_control() || matches!(byte, b'/' | b'@' | b'#' | b'?'))
            || port == 0
        {
            return Err("invalid exact HTTP origin");
        }
        Ok(Self { scheme, host, port })
    }

    fn permits(&self, scheme: &str, authority: &hyper::http::uri::Authority) -> bool {
        let request_port = authority.port_u16().or(match scheme {
            "http" => Some(80),
            "https" => Some(443),
            _ => None,
        });
        self.scheme.eq_ignore_ascii_case(scheme)
            && self.host.eq_ignore_ascii_case(authority.host())
            && request_port == Some(self.port)
    }
}

type HttpBody = UnsyncBoxBody<Bytes, ErrorCode>;
type HttpIo = Box<dyn Future<Output = Result<(), ErrorCode>> + Send>;
type HttpResponse = (hyper::Response<HttpBody>, HttpIo);
type HttpFuture = Box<dyn Future<Output = Result<HttpResponse, TrappableError<ErrorCode>>> + Send>;

struct ProviderHttpHooks {
    allowed_origins: BTreeSet<HttpOrigin>,
}

impl WasiHttpHooks for ProviderHttpHooks {
    fn send_request(
        &mut self,
        request: hyper::Request<HttpBody>,
        options: Option<RequestOptions>,
        response_io: HttpIo,
    ) -> HttpFuture {
        let permitted = request
            .uri()
            .scheme_str()
            .zip(request.uri().authority())
            .is_some_and(|(scheme, authority)| {
                self.allowed_origins
                    .iter()
                    .any(|origin| origin.permits(scheme, authority))
            });
        if !permitted {
            return Box::new(async { Err(ErrorCode::HttpRequestDenied.into()) });
        }
        drop(response_io);
        Box::new(async move {
            let (response, response_io) = default_send_request(request, options).await?;
            Ok((
                response.map(BodyExt::boxed_unsync),
                Box::new(response_io) as HttpIo,
            ))
        })
    }
}

mod bindings {
    wasmtime::component::bindgen!({
        path: [
            "../litehouse-contracts/packages/descriptor/1.0.0/wit",
            "../litehouse-contracts/packages/errors/1.0.0/wit",
            "../litehouse-contracts/packages/state/1.0.0/wit",
            "../litehouse-contracts/packages/receipt/1.0.0/wit",
            "../litehouse-contracts/packages/on-off/1.0.0/wit",
            "../litehouse-contracts/packages/boolean-state/1.0.0/wit",
            "../litehouse-contracts/packages/quantity/1.0.0/wit",
            "../litehouse-contracts/packages/measurement/1.0.0/wit",
            "../litehouse-contracts/packages/notification-sink/1.0.0/wit",
            "../litehouse-contracts/packages/provider/1.0.0/wit",
        ],
        world: "litehouse:provider/component@1.0.0",
        imports: { default: async },
        exports: { default: async },
        require_store_data_send: true,
    });
}

use bindings::exports::litehouse::provider::provider as wire_provider;
use bindings::litehouse::{
    boolean_state::types as wire_boolean, descriptor::types as wire_descriptor,
    errors::types as wire_error, measurement::types as wire_measurement,
    notification_sink::types as wire_notification, on_off::types as wire_on_off,
    quantity::types as wire_quantity, receipt::types as wire_receipt, state::types as wire_state,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetKind {
    OnOff,
    BooleanState,
    Measurement,
    NotificationSink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFacetDescriptor {
    endpoint: fabric::EndpointDescriptor,
    kind: FacetKind,
    label: Option<String>,
    semantic_profile: Option<String>,
    tags: Vec<SemanticTag>,
}

impl HostFacetDescriptor {
    pub fn endpoint(&self) -> &fabric::EndpointDescriptor {
        &self.endpoint
    }

    pub fn kind(&self) -> FacetKind {
        self.kind
    }

    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn semantic_profile(&self) -> Option<&str> {
        self.semantic_profile.as_deref()
    }

    pub fn tags(&self) -> &[SemanticTag] {
        &self.tags
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticTag {
    pub namespace: String,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    SourceReported,
    SourceAssumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailabilityReason {
    Disconnected,
    Reconnecting,
    ProviderFailed,
    EndpointRemoved,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    Available,
    Unavailable(UnavailabilityReason),
    Stale { since_unix_milliseconds: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FacetValue {
    OnOff(Option<bool>),
    BooleanState(Option<bool>),
    Measurement(Option<MeasurementValue>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementValue {
    pub profile: String,
    pub magnitude: String,
    pub unit: String,
    pub native_magnitude: Option<String>,
    pub native_unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FacetState {
    pub endpoint: fabric::EndpointIdentity,
    pub generation: fabric::Generation,
    pub revision: u64,
    pub received_at_unix_milliseconds: u64,
    pub availability: Availability,
    pub quality: Option<Quality>,
    pub value: FacetValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptConfidence {
    AcceptedForDelivery,
    ProtocolOrTargetAcknowledged,
    Completed,
    ObservedCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandReceipt {
    pub endpoint: fabric::EndpointIdentity,
    pub request_id: String,
    pub attempt: u32,
    pub provider_generation: fabric::Generation,
    pub confidence: ReceiptConfidence,
    pub accepted_at_unix_milliseconds: u64,
    pub completed_at_unix_milliseconds: Option<u64>,
    pub idempotency_key: Option<String>,
    pub deduplicated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationRequest {
    pub body: String,
    pub title: Option<String>,
    pub priority: Option<NotificationPriority>,
    pub expires_at_unix_milliseconds: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationReceipt {
    pub endpoint: fabric::EndpointIdentity,
    pub request_id: String,
    pub attempt: u32,
    pub provider_generation: fabric::Generation,
    pub confidence: ReceiptConfidence,
    pub accepted_at_unix_milliseconds: u64,
    pub idempotency_key: Option<String>,
    pub provider_message_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CallContext {
    pub request_id: String,
    pub correlation_id: Option<String>,
    pub deadline: tokio::time::Instant,
    pub idempotency_key: Option<String>,
    pub attempt: u32,
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub instance: fabric::InstanceId,
    pub generation: fabric::Generation,
    pub config_json: String,
    pub allowed_tcp_addresses: BTreeSet<SocketAddr>,
    pub allowed_http_origins: BTreeSet<HttpOrigin>,
    pub startup_timeout: Duration,
}

#[derive(Debug, Clone)]
pub enum ProviderHostError {
    ComponentLinkFailed,
    ComponentInstantiationFailed,
    ProviderStartFailed,
    InvalidConfiguration,
    InvalidDescriptor,
    TooManyFacets,
    DuplicateFacet,
    FabricUnavailable,
    FabricRejected,
    ActorStopped,
    Service(fabric::ServiceError),
}

impl fmt::Display for ProviderHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ComponentLinkFailed => {
                formatter.write_str("provider component could not be linked")
            }
            Self::ComponentInstantiationFailed => {
                formatter.write_str("provider component could not be instantiated")
            }
            Self::ProviderStartFailed => {
                formatter.write_str("provider component start call failed")
            }
            Self::InvalidConfiguration => formatter.write_str("invalid provider configuration"),
            Self::InvalidDescriptor => {
                formatter.write_str("provider returned an invalid descriptor")
            }
            Self::TooManyFacets => formatter.write_str("provider exceeded the facet limit"),
            Self::DuplicateFacet => formatter.write_str("provider returned a duplicate facet"),
            Self::FabricUnavailable => formatter.write_str("service fabric is unavailable"),
            Self::FabricRejected => {
                formatter.write_str("service fabric rejected provider lifecycle")
            }
            Self::ActorStopped => formatter.write_str("provider actor stopped"),
            Self::Service(error) => error.fmt(formatter),
        }
    }
}

impl Error for ProviderHostError {}

#[derive(Clone)]
struct SharedMonotonicClock {
    started: StdInstant,
}

impl HostMonotonicClock for SharedMonotonicClock {
    fn resolution(&self) -> u64 {
        1
    }

    fn now(&self) -> u64 {
        self.started
            .elapsed()
            .as_nanos()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

struct ProviderStore {
    table: ResourceTable,
    wasi: WasiCtx,
    http: WasiHttpCtx,
    http_hooks: ProviderHttpHooks,
    clock_started: StdInstant,
}

impl WasiView for ProviderStore {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for ProviderStore {
    fn http(&mut self) -> WasiHttpCtxView<'_> {
        WasiHttpCtxView {
            ctx: &mut self.http,
            table: &mut self.table,
            hooks: &mut self.http_hooks,
        }
    }
}

impl wire_descriptor::Host for ProviderStore {}
impl wire_error::Host for ProviderStore {}
impl wire_receipt::Host for ProviderStore {}
impl wire_state::Host for ProviderStore {}
impl wire_boolean::Host for ProviderStore {}
impl wire_measurement::Host for ProviderStore {}
impl wire_notification::Host for ProviderStore {}
impl wire_on_off::Host for ProviderStore {}
impl wire_quantity::Host for ProviderStore {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HandleKey {
    kind: FacetKind,
    facet: String,
}

impl PartialOrd for FacetKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FacetKind {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (*self as u8).cmp(&(*other as u8))
    }
}

struct ActorState {
    store: Store<ProviderStore>,
    component: bindings::Component,
    _runner: ResourceAny,
    handles: BTreeMap<HandleKey, ResourceAny>,
    provider: fabric::InstanceGeneration,
    fabric: Arc<Mutex<fabric::ServiceFabric>>,
    idempotency: IdempotencyCache,
}

const MAX_IDEMPOTENCY_ENTRIES: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct IdempotencyKey {
    facet: String,
    generation: fabric::Generation,
    key: String,
}

#[derive(Default)]
struct IdempotencyCache {
    entries: BTreeMap<IdempotencyKey, Result<CommandReceipt, ProviderHostError>>,
    order: VecDeque<IdempotencyKey>,
}

impl IdempotencyCache {
    fn get(&self, key: &IdempotencyKey) -> Option<Result<CommandReceipt, ProviderHostError>> {
        self.entries.get(key).cloned().map(|result| {
            result.map(|mut receipt| {
                receipt.deduplicated = true;
                receipt
            })
        })
    }

    fn insert(&mut self, key: IdempotencyKey, result: Result<CommandReceipt, ProviderHostError>) {
        if self.entries.len() == MAX_IDEMPOTENCY_ENTRIES
            && let Some(oldest) = self.order.pop_front()
        {
            self.entries.remove(&oldest);
        }
        self.order.push_back(key.clone());
        self.entries.insert(key, result);
    }
}

enum ActorRequest {
    Query {
        endpoint: fabric::EndpointIdentity,
        context: CallContext,
        reply: oneshot::Sender<Result<FacetState, ProviderHostError>>,
    },
    SetOnOff {
        endpoint: fabric::EndpointIdentity,
        value: bool,
        context: CallContext,
        reply: oneshot::Sender<Result<CommandReceipt, ProviderHostError>>,
    },
    SendNotification {
        endpoint: fabric::EndpointIdentity,
        value: NotificationRequest,
        context: CallContext,
        reply: oneshot::Sender<Result<NotificationReceipt, ProviderHostError>>,
    },
}

#[derive(Clone)]
pub struct ProviderHandle {
    provider: fabric::InstanceGeneration,
    descriptors: Arc<Vec<HostFacetDescriptor>>,
    sender: mpsc::Sender<ActorRequest>,
}

impl ProviderHandle {
    pub fn provider(&self) -> &fabric::InstanceGeneration {
        &self.provider
    }

    pub fn descriptors(&self) -> &[HostFacetDescriptor] {
        self.descriptors.as_slice()
    }

    pub fn is_running(&self) -> bool {
        !self.sender.is_closed()
    }

    pub async fn query(
        &self,
        endpoint: &fabric::EndpointIdentity,
        expected_generation: fabric::Generation,
        context: CallContext,
    ) -> Result<FacetState, ProviderHostError> {
        self.validate_target(endpoint, expected_generation)?;
        let deadline = context.deadline;
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::Query {
            endpoint: endpoint.clone(),
            context,
            reply,
        });
        tokio::time::timeout_at(deadline, result)
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?
    }

    pub async fn set_on_off(
        &self,
        endpoint: &fabric::EndpointIdentity,
        expected_generation: fabric::Generation,
        value: bool,
        context: CallContext,
    ) -> Result<CommandReceipt, ProviderHostError> {
        self.validate_target(endpoint, expected_generation)?;
        let deadline = context.deadline;
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let idempotency_key = context.idempotency_key.clone();
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::SetOnOff {
            endpoint: endpoint.clone(),
            value,
            context,
            reply,
        });
        tokio::time::timeout_at(deadline, result)
            .await
            .map_err(|_| {
                outcome_unknown(
                    fabric::UnknownOutcomeCause::DeadlineExceeded,
                    request_id,
                    attempt,
                    idempotency_key,
                )
            })?
            .map_err(|_| ProviderHostError::ActorStopped)?
    }

    pub async fn send_notification(
        &self,
        endpoint: &fabric::EndpointIdentity,
        expected_generation: fabric::Generation,
        value: NotificationRequest,
        context: CallContext,
    ) -> Result<NotificationReceipt, ProviderHostError> {
        self.validate_target(endpoint, expected_generation)?;
        let deadline = context.deadline;
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let idempotency_key = context.idempotency_key.clone();
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::SendNotification {
            endpoint: endpoint.clone(),
            value,
            context,
            reply,
        });
        tokio::time::timeout_at(deadline, result)
            .await
            .map_err(|_| {
                outcome_unknown(
                    fabric::UnknownOutcomeCause::DeadlineExceeded,
                    request_id,
                    attempt,
                    idempotency_key,
                )
            })?
            .map_err(|_| ProviderHostError::ActorStopped)?
    }

    fn validate_target(
        &self,
        endpoint: &fabric::EndpointIdentity,
        expected_generation: fabric::Generation,
    ) -> Result<(), ProviderHostError> {
        if endpoint.provider() != self.provider.instance()
            || expected_generation != self.provider.generation()
        {
            return Err(service_error(
                fabric::CoreError::Binding(fabric::BindingError::StaleHandle),
                fabric::RetryAdvice::Rebind,
            ));
        }
        if !self
            .descriptors
            .iter()
            .any(|descriptor| descriptor.endpoint().identity() == endpoint)
        {
            return Err(service_error(
                fabric::CoreError::Binding(fabric::BindingError::NotFound),
                fabric::RetryAdvice::Never,
            ));
        }
        Ok(())
    }
}

pub async fn start_provider(
    engine: &Engine,
    component: &WasmComponent,
    config: ProviderConfig,
    fabric: Arc<Mutex<fabric::ServiceFabric>>,
) -> Result<ProviderHandle, ProviderHostError> {
    if config.config_json.len() > 1024 * 1024 || config.startup_timeout.is_zero() {
        return Err(ProviderHostError::InvalidConfiguration);
    }
    let deadline = tokio::time::Instant::now() + config.startup_timeout;
    let clock_started = StdInstant::now();
    let clock = SharedMonotonicClock {
        started: clock_started,
    };
    let allowed_tcp_addresses = config.allowed_tcp_addresses;
    let allowed_http_origins = config.allowed_http_origins;
    let mut wasi = WasiCtxBuilder::new();
    wasi.monotonic_clock(clock)
        .socket_addr_check(move |address, usage| {
            let permitted = tcp_address_allowed(&allowed_tcp_addresses, address, usage);
            Box::pin(async move { permitted })
        });
    let mut store = Store::new(
        engine,
        ProviderStore {
            table: ResourceTable::new(),
            wasi: wasi.build(),
            http: WasiHttpCtx::new(),
            http_hooks: ProviderHttpHooks {
                allowed_origins: allowed_http_origins,
            },
            clock_started,
        },
    );
    set_store_deadline(&mut store, deadline)?;

    let mut linker = Linker::new(engine);
    wasmtime_wasi::p2::add_to_linker_async(&mut linker)
        .map_err(|_| ProviderHostError::ComponentLinkFailed)?;
    wasmtime_wasi_http::p3::add_to_linker(&mut linker)
        .map_err(|_| ProviderHostError::ComponentLinkFailed)?;
    bindings::Component::add_to_linker::<_, HasSelf<_>>(&mut linker, |data| data)
        .map_err(|_| ProviderHostError::ComponentLinkFailed)?;

    let bound = tokio::time::timeout_at(
        deadline,
        bindings::Component::instantiate_async(&mut store, component, &linker),
    )
    .await
    .map_err(|_| deadline_error(deadline))?
    .map_err(|_| ProviderHostError::ComponentInstantiationFailed)?;
    let guest = bound.litehouse_provider_provider();
    set_store_deadline(&mut store, deadline)?;
    let instance_id = config.instance.as_str().to_owned();
    let runner = tokio::time::timeout_at(
        deadline,
        store.run_concurrent(async |accessor| {
            guest
                .call_start(accessor, instance_id, config.config_json)
                .await
        }),
    )
    .await
    .map_err(|_| deadline_error(deadline))?
    .map_err(|_| ProviderHostError::ProviderStartFailed)?
    .map_err(|_| ProviderHostError::ProviderStartFailed)?
    .map_err(provider_error)?;

    set_store_deadline(&mut store, deadline)?;
    let (on_off_handles, boolean_handles, measurement_handles, notification_handles) =
        tokio::time::timeout_at(
            deadline,
            store.run_concurrent(async |accessor| {
                let on_off = guest.runner().call_on_off_facets(accessor, runner).await?;
                let boolean = guest.runner().call_boolean_facets(accessor, runner).await?;
                let measurement = guest
                    .runner()
                    .call_measurement_facets(accessor, runner)
                    .await?;
                let notification = guest
                    .runner()
                    .call_notification_sink_facets(accessor, runner)
                    .await?;
                Ok::<_, wasmtime::Error>((on_off, boolean, measurement, notification))
            }),
        )
        .await
        .map_err(|_| deadline_error(deadline))?
        .map_err(|_| ProviderHostError::ProviderStartFailed)?
        .map_err(|_| ProviderHostError::ProviderStartFailed)?;
    let on_off_handles = on_off_handles.map_err(provider_error)?;
    let boolean_handles = boolean_handles.map_err(provider_error)?;
    let measurement_handles = measurement_handles.map_err(provider_error)?;
    let notification_handles = notification_handles.map_err(provider_error)?;
    if on_off_handles.len()
        + boolean_handles.len()
        + measurement_handles.len()
        + notification_handles.len()
        > MAX_FACETS
    {
        return Err(ProviderHostError::TooManyFacets);
    }

    let mut handles = BTreeMap::new();
    let mut descriptors = Vec::new();
    for handle in on_off_handles {
        let (key, _incarnation, descriptor) = describe_on_off(
            &mut store,
            guest,
            handle,
            &config.instance,
            config.generation,
            deadline,
        )
        .await?;
        if handles
            .insert(
                HandleKey {
                    kind: FacetKind::OnOff,
                    facet: key,
                },
                handle,
            )
            .is_some()
        {
            return Err(ProviderHostError::DuplicateFacet);
        }
        descriptors.push(descriptor);
    }
    for handle in boolean_handles {
        let (key, incarnation, descriptor) = describe_boolean(
            &mut store,
            guest,
            handle,
            &config.instance,
            config.generation,
            deadline,
        )
        .await?;
        if incarnation == 0
            || handles
                .insert(
                    HandleKey {
                        kind: FacetKind::BooleanState,
                        facet: key,
                    },
                    handle,
                )
                .is_some()
        {
            return Err(ProviderHostError::DuplicateFacet);
        }
        descriptors.push(descriptor);
    }
    for handle in measurement_handles {
        let (key, incarnation, descriptor) = describe_measurement(
            &mut store,
            guest,
            handle,
            &config.instance,
            config.generation,
            deadline,
        )
        .await?;
        if incarnation == 0
            || handles
                .insert(
                    HandleKey {
                        kind: FacetKind::Measurement,
                        facet: key,
                    },
                    handle,
                )
                .is_some()
        {
            return Err(ProviderHostError::DuplicateFacet);
        }
        descriptors.push(descriptor);
    }
    for handle in notification_handles {
        let (key, incarnation, descriptor) = describe_notification(
            &mut store,
            guest,
            handle,
            &config.instance,
            config.generation,
            deadline,
        )
        .await?;
        if incarnation == 0
            || handles
                .insert(
                    HandleKey {
                        kind: FacetKind::NotificationSink,
                        facet: key,
                    },
                    handle,
                )
                .is_some()
        {
            return Err(ProviderHostError::DuplicateFacet);
        }
        descriptors.push(descriptor);
    }
    descriptors.sort_by(|left, right| left.endpoint().identity().cmp(right.endpoint().identity()));

    let provider = fabric::InstanceGeneration::new(config.instance, config.generation);
    publish_provider(&fabric, &provider, &descriptors)?;
    let (sender, receiver) = mpsc::channel(ACTOR_QUEUE_CAPACITY);
    let actor = ActorState {
        store,
        component: bound,
        _runner: runner,
        handles,
        provider: provider.clone(),
        fabric,
        idempotency: IdempotencyCache::default(),
    };
    tokio::spawn(run_actor(actor, receiver));
    Ok(ProviderHandle {
        provider,
        descriptors: Arc::new(descriptors),
        sender,
    })
}

async fn describe_on_off(
    store: &mut Store<ProviderStore>,
    guest: &wire_provider::Guest,
    handle: ResourceAny,
    instance: &fabric::InstanceId,
    generation: fabric::Generation,
    deadline: tokio::time::Instant,
) -> Result<(String, u64, HostFacetDescriptor), ProviderHostError> {
    set_store_deadline(store, deadline)?;
    let key = guest
        .on_off_facet()
        .call_key(&mut *store, handle)
        .await
        .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let incarnation = guest
        .on_off_facet()
        .call_incarnation(&mut *store, handle)
        .await
        .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let context = wire_context(
        store.data().clock_started,
        deadline,
        "describe-on-off",
        None,
        None,
        0,
    );
    let descriptor = tokio::time::timeout_at(
        deadline,
        store.run_concurrent(async |accessor| {
            guest
                .on_off_facet()
                .call_describe(accessor, handle, context)
                .await
        }),
    )
    .await
    .map_err(|_| deadline_error(deadline))?
    .map_err(|_| ProviderHostError::InvalidDescriptor)?
    .map_err(|_| ProviderHostError::InvalidDescriptor)?
    .map_err(on_off_error)?;
    let host = host_on_off_descriptor(instance, generation, &key, incarnation, descriptor)?;
    Ok((key, incarnation, host))
}

async fn describe_boolean(
    store: &mut Store<ProviderStore>,
    guest: &wire_provider::Guest,
    handle: ResourceAny,
    instance: &fabric::InstanceId,
    generation: fabric::Generation,
    deadline: tokio::time::Instant,
) -> Result<(String, u64, HostFacetDescriptor), ProviderHostError> {
    set_store_deadline(store, deadline)?;
    let key = guest
        .boolean_facet()
        .call_key(&mut *store, handle)
        .await
        .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let incarnation = guest
        .boolean_facet()
        .call_incarnation(&mut *store, handle)
        .await
        .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let context = wire_context(
        store.data().clock_started,
        deadline,
        "describe-boolean",
        None,
        None,
        0,
    );
    let descriptor = tokio::time::timeout_at(
        deadline,
        store.run_concurrent(async |accessor| {
            guest
                .boolean_facet()
                .call_describe(accessor, handle, context)
                .await
        }),
    )
    .await
    .map_err(|_| deadline_error(deadline))?
    .map_err(|_| ProviderHostError::InvalidDescriptor)?
    .map_err(|_| ProviderHostError::InvalidDescriptor)?
    .map_err(boolean_error)?;
    let host = host_boolean_descriptor(instance, generation, &key, incarnation, descriptor)?;
    Ok((key, incarnation, host))
}

async fn describe_measurement(
    store: &mut Store<ProviderStore>,
    guest: &wire_provider::Guest,
    handle: ResourceAny,
    instance: &fabric::InstanceId,
    generation: fabric::Generation,
    deadline: tokio::time::Instant,
) -> Result<(String, u64, HostFacetDescriptor), ProviderHostError> {
    set_store_deadline(store, deadline)?;
    let key = guest
        .measurement_facet()
        .call_key(&mut *store, handle)
        .await
        .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let incarnation = guest
        .measurement_facet()
        .call_incarnation(&mut *store, handle)
        .await
        .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let context = wire_context(
        store.data().clock_started,
        deadline,
        "describe-measurement",
        None,
        None,
        0,
    );
    let descriptor = tokio::time::timeout_at(
        deadline,
        store.run_concurrent(async |accessor| {
            guest
                .measurement_facet()
                .call_describe(accessor, handle, context)
                .await
        }),
    )
    .await
    .map_err(|_| deadline_error(deadline))?
    .map_err(|_| ProviderHostError::InvalidDescriptor)?
    .map_err(|_| ProviderHostError::InvalidDescriptor)?
    .map_err(measurement_error)?;
    let host = host_measurement_descriptor(instance, generation, &key, incarnation, descriptor)?;
    Ok((key, incarnation, host))
}

async fn describe_notification(
    store: &mut Store<ProviderStore>,
    guest: &wire_provider::Guest,
    handle: ResourceAny,
    instance: &fabric::InstanceId,
    generation: fabric::Generation,
    deadline: tokio::time::Instant,
) -> Result<(String, u64, HostFacetDescriptor), ProviderHostError> {
    set_store_deadline(store, deadline)?;
    let key = guest
        .notification_sink_facet()
        .call_key(&mut *store, handle)
        .await
        .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let incarnation = guest
        .notification_sink_facet()
        .call_incarnation(&mut *store, handle)
        .await
        .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let context = wire_context(
        store.data().clock_started,
        deadline,
        "describe-notification-sink",
        None,
        None,
        0,
    );
    let descriptor = tokio::time::timeout_at(
        deadline,
        store.run_concurrent(async |accessor| {
            guest
                .notification_sink_facet()
                .call_describe(accessor, handle, context)
                .await
        }),
    )
    .await
    .map_err(|_| deadline_error(deadline))?
    .map_err(|_| ProviderHostError::InvalidDescriptor)?
    .map_err(|_| ProviderHostError::InvalidDescriptor)?
    .map_err(notification_error)?;
    let host = host_notification_descriptor(instance, generation, &key, incarnation, descriptor)?;
    Ok((key, incarnation, host))
}

fn publish_provider(
    fabric: &Arc<Mutex<fabric::ServiceFabric>>,
    provider: &fabric::InstanceGeneration,
    descriptors: &[HostFacetDescriptor],
) -> Result<(), ProviderHostError> {
    let mut fabric = fabric
        .lock()
        .map_err(|_| ProviderHostError::FabricUnavailable)?;
    fabric
        .activate_provider(provider.clone())
        .map_err(|_| ProviderHostError::FabricRejected)?;
    for descriptor in descriptors {
        if fabric
            .publish_endpoint(descriptor.endpoint.clone())
            .is_err()
        {
            let _ = fabric.trap_provider(provider);
            return Err(ProviderHostError::FabricRejected);
        }
    }
    Ok(())
}

async fn run_actor(mut actor: ActorState, mut receiver: mpsc::Receiver<ActorRequest>) {
    while let Some(request) = receiver.recv().await {
        let fatal = match request {
            ActorRequest::Query {
                endpoint,
                context,
                reply,
            } => {
                let (result, fatal) = actor.query(endpoint, context).await;
                let _ = reply.send(result);
                fatal
            }
            ActorRequest::SetOnOff {
                endpoint,
                value,
                context,
                reply,
            } => {
                let (result, fatal) = actor.set_on_off(endpoint, value, context).await;
                let _ = reply.send(result);
                fatal
            }
            ActorRequest::SendNotification {
                endpoint,
                value,
                context,
                reply,
            } => {
                let (result, fatal) = actor.send_notification(endpoint, value, context).await;
                let _ = reply.send(result);
                fatal
            }
        };
        if fatal {
            break;
        }
    }
    if let Ok(mut fabric) = actor.fabric.lock() {
        let _ = fabric.trap_provider(&actor.provider);
    }
}

impl ActorState {
    async fn query(
        &mut self,
        endpoint: fabric::EndpointIdentity,
        context: CallContext,
    ) -> (Result<FacetState, ProviderHostError>, bool) {
        let kind = if endpoint.contract().name() == "on-off" {
            FacetKind::OnOff
        } else if endpoint.contract().name() == "boolean-state" {
            FacetKind::BooleanState
        } else if endpoint.contract().name() == "measurement" {
            FacetKind::Measurement
        } else {
            return (
                Err(service_error(
                    fabric::CoreError::Execution(fabric::ExecutionError::Unsupported),
                    fabric::RetryAdvice::Never,
                )),
                false,
            );
        };
        let key = HandleKey {
            kind,
            facet: endpoint.facet().as_str().to_owned(),
        };
        let Some(handle) = self.handles.get(&key).copied() else {
            return (
                Err(service_error(
                    fabric::CoreError::Binding(fabric::BindingError::EndpointRemoved),
                    fabric::RetryAdvice::Rebind,
                )),
                false,
            );
        };
        if let Err(error) = set_store_deadline(&mut self.store, context.deadline) {
            return (Err(error), false);
        }
        let wire = wire_context(
            self.store.data().clock_started,
            context.deadline,
            &context.request_id,
            context.correlation_id.clone(),
            None,
            context.attempt,
        );
        let call = match kind {
            FacetKind::OnOff => {
                let result = tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |accessor| {
                        self.component
                            .litehouse_provider_provider()
                            .on_off_facet()
                            .call_query(accessor, handle, wire)
                            .await
                    }),
                )
                .await;
                match result {
                    Err(_) => return (Err(deadline_error(context.deadline)), true),
                    Ok(Err(_)) | Ok(Ok(Err(_))) => return (Err(provider_trapped()), true),
                    Ok(Ok(Ok(Err(error)))) => return (Err(on_off_error(error)), false),
                    Ok(Ok(Ok(Ok(state)))) => {
                        host_on_off_state(endpoint, self.provider.generation(), state)
                    }
                }
            }
            FacetKind::BooleanState => {
                let result = tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |accessor| {
                        self.component
                            .litehouse_provider_provider()
                            .boolean_facet()
                            .call_query(accessor, handle, wire)
                            .await
                    }),
                )
                .await;
                match result {
                    Err(_) => return (Err(deadline_error(context.deadline)), true),
                    Ok(Err(_)) | Ok(Ok(Err(_))) => return (Err(provider_trapped()), true),
                    Ok(Ok(Ok(Err(error)))) => return (Err(boolean_error(error)), false),
                    Ok(Ok(Ok(Ok(state)))) => {
                        host_boolean_state(endpoint, self.provider.generation(), state)
                    }
                }
            }
            FacetKind::Measurement => {
                let result = tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |accessor| {
                        self.component
                            .litehouse_provider_provider()
                            .measurement_facet()
                            .call_query(accessor, handle, wire)
                            .await
                    }),
                )
                .await;
                match result {
                    Err(_) => return (Err(deadline_error(context.deadline)), true),
                    Ok(Err(_)) | Ok(Ok(Err(_))) => return (Err(provider_trapped()), true),
                    Ok(Ok(Ok(Err(error)))) => return (Err(measurement_error(error)), false),
                    Ok(Ok(Ok(Ok(state)))) => {
                        host_measurement_state(endpoint, self.provider.generation(), state)
                    }
                }
            }
            FacetKind::NotificationSink => unreachable!("notification sinks are not queryable"),
        };
        (call, false)
    }

    async fn set_on_off(
        &mut self,
        endpoint: fabric::EndpointIdentity,
        value: bool,
        context: CallContext,
    ) -> (Result<CommandReceipt, ProviderHostError>, bool) {
        let handle_key = HandleKey {
            kind: FacetKind::OnOff,
            facet: endpoint.facet().as_str().to_owned(),
        };
        let Some(handle) = self.handles.get(&handle_key).copied() else {
            return (
                Err(service_error(
                    fabric::CoreError::Binding(fabric::BindingError::EndpointRemoved),
                    fabric::RetryAdvice::Rebind,
                )),
                false,
            );
        };
        let idempotency_key = context.idempotency_key.as_ref().map(|key| IdempotencyKey {
            facet: endpoint.facet().as_str().to_owned(),
            generation: self.provider.generation(),
            key: key.clone(),
        });
        if let Some(cached) = idempotency_key
            .as_ref()
            .and_then(|key| self.idempotency.get(key))
        {
            return (cached, false);
        }
        if let Err(error) = set_store_deadline(&mut self.store, context.deadline) {
            return (Err(error), false);
        }
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let request_idempotency_key = context.idempotency_key.clone();
        let wire_context = wire_context(
            self.store.data().clock_started,
            context.deadline,
            &request_id,
            context.correlation_id.clone(),
            request_idempotency_key.clone(),
            attempt,
        );
        let request = wire_on_off::SetRequest {
            value,
            timed: None,
            effect: None,
        };
        let call = tokio::time::timeout_at(
            context.deadline,
            self.store.run_concurrent(async |accessor| {
                self.component
                    .litehouse_provider_provider()
                    .on_off_facet()
                    .call_set(accessor, handle, wire_context, request)
                    .await
            }),
        )
        .await;
        let (result, fatal) = match call {
            Err(_) => (
                Err(outcome_unknown(
                    fabric::UnknownOutcomeCause::DeadlineExceeded,
                    request_id,
                    attempt,
                    request_idempotency_key,
                )),
                true,
            ),
            Ok(Err(_)) | Ok(Ok(Err(_))) => (
                Err(outcome_unknown(
                    fabric::UnknownOutcomeCause::ProviderFailed,
                    request_id,
                    attempt,
                    request_idempotency_key,
                )),
                true,
            ),
            Ok(Ok(Ok(Err(error)))) => (Err(on_off_error(error)), false),
            Ok(Ok(Ok(Ok(receipt)))) => (
                host_receipt(
                    endpoint,
                    self.provider.generation(),
                    value,
                    &request_id,
                    attempt,
                    request_idempotency_key.as_deref(),
                    receipt,
                ),
                false,
            ),
        };
        if !fatal && let Some(key) = idempotency_key {
            self.idempotency.insert(key, result.clone());
        }
        (result, fatal)
    }

    async fn send_notification(
        &mut self,
        endpoint: fabric::EndpointIdentity,
        value: NotificationRequest,
        context: CallContext,
    ) -> (Result<NotificationReceipt, ProviderHostError>, bool) {
        let handle_key = HandleKey {
            kind: FacetKind::NotificationSink,
            facet: endpoint.facet().as_str().to_owned(),
        };
        let Some(handle) = self.handles.get(&handle_key).copied() else {
            return (
                Err(service_error(
                    fabric::CoreError::Binding(fabric::BindingError::EndpointRemoved),
                    fabric::RetryAdvice::Rebind,
                )),
                false,
            );
        };
        if let Err(error) = set_store_deadline(&mut self.store, context.deadline) {
            return (Err(error), false);
        }
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let idempotency_key = context.idempotency_key.clone();
        let wire_context = wire_context(
            self.store.data().clock_started,
            context.deadline,
            &request_id,
            context.correlation_id,
            idempotency_key.clone(),
            attempt,
        );
        let notification = wire_notification::Notification {
            body: value.body,
            title: value.title,
            priority: value.priority.map(|priority| match priority {
                NotificationPriority::Low => wire_notification::Priority::Low,
                NotificationPriority::Normal => wire_notification::Priority::Normal,
                NotificationPriority::High => wire_notification::Priority::High,
                NotificationPriority::Urgent => wire_notification::Priority::Urgent,
            }),
            expires_at_unix_milliseconds: value.expires_at_unix_milliseconds,
        };
        let call = tokio::time::timeout_at(
            context.deadline,
            self.store.run_concurrent(async |accessor| {
                self.component
                    .litehouse_provider_provider()
                    .notification_sink_facet()
                    .call_send(accessor, handle, wire_context, notification)
                    .await
            }),
        )
        .await;
        match call {
            Err(_) => (
                Err(outcome_unknown(
                    fabric::UnknownOutcomeCause::DeadlineExceeded,
                    request_id,
                    attempt,
                    idempotency_key,
                )),
                true,
            ),
            Ok(Err(_)) | Ok(Ok(Err(_))) => (
                Err(outcome_unknown(
                    fabric::UnknownOutcomeCause::ProviderFailed,
                    request_id,
                    attempt,
                    idempotency_key,
                )),
                true,
            ),
            Ok(Ok(Ok(Err(error)))) => (Err(notification_error(error)), false),
            Ok(Ok(Ok(Ok(receipt)))) => (
                host_notification_receipt(
                    endpoint,
                    self.provider.generation(),
                    &request_id,
                    attempt,
                    idempotency_key.as_deref(),
                    receipt,
                ),
                false,
            ),
        }
    }
}

fn host_on_off_descriptor(
    instance: &fabric::InstanceId,
    generation: fabric::Generation,
    key: &str,
    incarnation: u64,
    descriptor: wire_on_off::OnOffDescriptor,
) -> Result<HostFacetDescriptor, ProviderHostError> {
    validate_common(&descriptor.common, instance, key, incarnation, "on-off")?;
    let incarnation =
        fabric::Incarnation::new(incarnation).map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let identity = fabric::EndpointIdentity::new(
        instance.clone(),
        fabric::FacetKey::new(key).map_err(|_| ProviderHostError::InvalidDescriptor)?,
        fabric::ContractId::new("litehouse", "on-off", 1)
            .map_err(|_| ProviderHostError::InvalidDescriptor)?,
    );
    let mut operations = vec![operation("query")?];
    if descriptor.operations.contains(&wire_on_off::Operation::Set) {
        operations.push(operation("set")?);
    }
    let features = descriptor
        .features
        .into_iter()
        .map(|feature| {
            fabric::FeatureId::new(match feature {
                wire_on_off::Feature::Timed => "timed",
                wire_on_off::Feature::Startup => "startup",
                wire_on_off::Feature::Effect => "effect",
                wire_on_off::Feature::DeadFront => "dead-front",
                wire_on_off::Feature::OffOnly => "off-only",
            })
            .map_err(|_| ProviderHostError::InvalidDescriptor)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let endpoint = fabric::EndpointDescriptor::new(
        identity,
        Version {
            major: 1,
            minor: 0,
            patch: 0,
        },
        generation,
        incarnation,
        operations,
        features,
    )
    .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    Ok(HostFacetDescriptor {
        endpoint,
        kind: FacetKind::OnOff,
        label: descriptor.common.label,
        semantic_profile: descriptor.common.semantic_profile,
        tags: host_tags(descriptor.common.tags)?,
    })
}

fn host_boolean_descriptor(
    instance: &fabric::InstanceId,
    generation: fabric::Generation,
    key: &str,
    incarnation: u64,
    descriptor: wire_boolean::BooleanDescriptor,
) -> Result<HostFacetDescriptor, ProviderHostError> {
    validate_common(
        &descriptor.common,
        instance,
        key,
        incarnation,
        "boolean-state",
    )?;
    let incarnation =
        fabric::Incarnation::new(incarnation).map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let identity = fabric::EndpointIdentity::new(
        instance.clone(),
        fabric::FacetKey::new(key).map_err(|_| ProviderHostError::InvalidDescriptor)?,
        fabric::ContractId::new("litehouse", "boolean-state", 1)
            .map_err(|_| ProviderHostError::InvalidDescriptor)?,
    );
    let endpoint = fabric::EndpointDescriptor::new(
        identity,
        Version {
            major: 1,
            minor: 0,
            patch: 0,
        },
        generation,
        incarnation,
        [operation("query")?],
        [],
    )
    .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    Ok(HostFacetDescriptor {
        endpoint,
        kind: FacetKind::BooleanState,
        label: descriptor.common.label,
        semantic_profile: Some(
            match descriptor.profile {
                wire_boolean::BooleanProfile::Contact => "contact",
                wire_boolean::BooleanProfile::Occupancy => "occupancy",
                wire_boolean::BooleanProfile::Smoke => "smoke",
                wire_boolean::BooleanProfile::Status => "status",
                wire_boolean::BooleanProfile::Moisture => "moisture",
                wire_boolean::BooleanProfile::Vibration => "vibration",
                wire_boolean::BooleanProfile::Generic => "generic",
            }
            .to_owned(),
        ),
        tags: host_tags(descriptor.common.tags)?,
    })
}

fn host_measurement_descriptor(
    instance: &fabric::InstanceId,
    generation: fabric::Generation,
    key: &str,
    incarnation: u64,
    descriptor: wire_measurement::MeasurementDescriptor,
) -> Result<HostFacetDescriptor, ProviderHostError> {
    validate_common(
        &descriptor.common,
        instance,
        key,
        incarnation,
        "measurement",
    )?;
    if descriptor.profile.is_empty() || descriptor.profile.len() > 128 {
        return Err(ProviderHostError::InvalidDescriptor);
    }
    let incarnation =
        fabric::Incarnation::new(incarnation).map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let identity = fabric::EndpointIdentity::new(
        instance.clone(),
        fabric::FacetKey::new(key).map_err(|_| ProviderHostError::InvalidDescriptor)?,
        fabric::ContractId::new("litehouse", "measurement", 1)
            .map_err(|_| ProviderHostError::InvalidDescriptor)?,
    );
    let endpoint = fabric::EndpointDescriptor::new(
        identity,
        Version {
            major: 1,
            minor: 0,
            patch: 0,
        },
        generation,
        incarnation,
        [operation("query")?],
        [],
    )
    .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    Ok(HostFacetDescriptor {
        endpoint,
        kind: FacetKind::Measurement,
        label: descriptor.common.label,
        semantic_profile: Some(descriptor.profile),
        tags: host_tags(descriptor.common.tags)?,
    })
}

fn host_notification_descriptor(
    instance: &fabric::InstanceId,
    generation: fabric::Generation,
    key: &str,
    incarnation: u64,
    descriptor: wire_notification::NotificationDescriptor,
) -> Result<HostFacetDescriptor, ProviderHostError> {
    validate_common(
        &descriptor.common,
        instance,
        key,
        incarnation,
        "notification-sink",
    )?;
    if descriptor.maximum_body_bytes == 0
        || descriptor.maximum_receipt_confidence
            != wire_receipt::ReceiptConfidence::AcceptedForDelivery
    {
        return Err(ProviderHostError::InvalidDescriptor);
    }
    let incarnation =
        fabric::Incarnation::new(incarnation).map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let identity = fabric::EndpointIdentity::new(
        instance.clone(),
        fabric::FacetKey::new(key).map_err(|_| ProviderHostError::InvalidDescriptor)?,
        fabric::ContractId::new("litehouse", "notification-sink", 1)
            .map_err(|_| ProviderHostError::InvalidDescriptor)?,
    );
    let endpoint = fabric::EndpointDescriptor::new(
        identity,
        Version {
            major: 1,
            minor: 0,
            patch: 0,
        },
        generation,
        incarnation,
        [operation("send")?],
        [],
    )
    .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    Ok(HostFacetDescriptor {
        endpoint,
        kind: FacetKind::NotificationSink,
        label: descriptor.common.label,
        semantic_profile: descriptor.common.semantic_profile,
        tags: host_tags(descriptor.common.tags)?,
    })
}

fn host_tags(
    tags: Vec<wire_descriptor::SemanticTag>,
) -> Result<Vec<SemanticTag>, ProviderHostError> {
    if tags.len() > 32 {
        return Err(ProviderHostError::InvalidDescriptor);
    }
    tags.into_iter()
        .map(|tag| {
            let valid_token = |value: &str| {
                !value.is_empty()
                    && value.len() <= 64
                    && value.bytes().all(|byte| {
                        byte.is_ascii_lowercase()
                            || byte.is_ascii_digit()
                            || matches!(byte, b'-' | b'.')
                    })
            };
            if !valid_token(&tag.namespace)
                || !valid_token(&tag.name)
                || tag.value.as_ref().is_some_and(|value| {
                    value.is_empty()
                        || value.len() > 256
                        || value.bytes().any(|byte| byte.is_ascii_control())
                })
            {
                return Err(ProviderHostError::InvalidDescriptor);
            }
            Ok(SemanticTag {
                namespace: tag.namespace,
                name: tag.name,
                value: tag.value,
            })
        })
        .collect()
}

fn validate_common(
    common: &wire_descriptor::FacetDescriptor,
    instance: &fabric::InstanceId,
    key: &str,
    incarnation: u64,
    contract: &str,
) -> Result<(), ProviderHostError> {
    let version = &common.contract.version;
    if common.provider_instance_id != instance.as_str()
        || common.facet_key != key
        || common.incarnation != incarnation
        || incarnation == 0
        || common.contract.namespace != "litehouse"
        || common.contract.name != contract
        || version.major != 1
        || common.direction != wire_descriptor::FacetDirection::Provided
    {
        return Err(ProviderHostError::InvalidDescriptor);
    }
    Ok(())
}

fn operation(value: &str) -> Result<fabric::OperationId, ProviderHostError> {
    fabric::OperationId::new(value).map_err(|_| ProviderHostError::InvalidDescriptor)
}

fn host_on_off_state(
    endpoint: fabric::EndpointIdentity,
    generation: fabric::Generation,
    state: wire_on_off::State,
) -> Result<FacetState, ProviderHostError> {
    host_state(
        endpoint,
        generation,
        state.meta,
        state.availability,
        state.quality,
        FacetValue::OnOff(state.current),
    )
}

fn host_boolean_state(
    endpoint: fabric::EndpointIdentity,
    generation: fabric::Generation,
    state: wire_boolean::State,
) -> Result<FacetState, ProviderHostError> {
    host_state(
        endpoint,
        generation,
        state.meta,
        state.availability,
        state.quality,
        FacetValue::BooleanState(state.current),
    )
}

fn host_measurement_state(
    endpoint: fabric::EndpointIdentity,
    generation: fabric::Generation,
    state: wire_measurement::State,
) -> Result<FacetState, ProviderHostError> {
    let current = state
        .current
        .map(|value| {
            if value.profile.is_empty()
                || value.profile.len() > 128
                || value.magnitude.is_empty()
                || value.magnitude.len() > 128
                || value.unit.is_empty()
                || value.unit.len() > 64
            {
                return Err(ProviderHostError::InvalidDescriptor);
            }
            Ok(MeasurementValue {
                profile: value.profile,
                magnitude: value.magnitude,
                unit: value.unit,
                native_magnitude: state.native.as_ref().map(|native| native.magnitude.clone()),
                native_unit: state.native.as_ref().map(|native| native.unit.clone()),
            })
        })
        .transpose()?;
    host_state(
        endpoint,
        generation,
        state.meta,
        state.availability,
        state.quality,
        FacetValue::Measurement(current),
    )
}

fn host_state(
    endpoint: fabric::EndpointIdentity,
    generation: fabric::Generation,
    meta: wire_state::ObservationMeta,
    availability: wire_state::Availability,
    quality: Option<wire_state::Quality>,
    value: FacetValue,
) -> Result<FacetState, ProviderHostError> {
    Ok(FacetState {
        endpoint,
        generation,
        revision: meta.revision,
        received_at_unix_milliseconds: meta.received_at.unix_milliseconds,
        availability: match availability {
            wire_state::Availability::Available => Availability::Available,
            wire_state::Availability::Unavailable(reason) => {
                Availability::Unavailable(match reason {
                    wire_state::UnavailabilityReason::Disconnected => {
                        UnavailabilityReason::Disconnected
                    }
                    wire_state::UnavailabilityReason::Reconnecting => {
                        UnavailabilityReason::Reconnecting
                    }
                    wire_state::UnavailabilityReason::ProviderFailed => {
                        UnavailabilityReason::ProviderFailed
                    }
                    wire_state::UnavailabilityReason::EndpointRemoved => {
                        UnavailabilityReason::EndpointRemoved
                    }
                    wire_state::UnavailabilityReason::Unknown => UnavailabilityReason::Unknown,
                })
            }
            wire_state::Availability::Stale(status) => Availability::Stale {
                since_unix_milliseconds: status.since.unix_milliseconds,
            },
        },
        quality: quality.map(|quality| match quality {
            wire_state::Quality::SourceReported => Quality::SourceReported,
            wire_state::Quality::SourceAssumed => Quality::SourceAssumed,
        }),
        value,
    })
}

fn host_receipt(
    endpoint: fabric::EndpointIdentity,
    generation: fabric::Generation,
    requested_value: bool,
    request_id: &str,
    attempt: u32,
    idempotency_key: Option<&str>,
    receipt: wire_on_off::OperationReceipt,
) -> Result<CommandReceipt, ProviderHostError> {
    if !matches!(
        receipt.operation,
        wire_on_off::OperationIntent::Set(wire_on_off::SetRequest {
            value,
            timed: None,
            effect: None,
        }) if value == requested_value
    ) || receipt.receipt.request_id != request_id
        || receipt.receipt.attempt != attempt
        || receipt.receipt.idempotency_key.as_deref() != idempotency_key
    {
        return Err(service_error(
            fabric::CoreError::ContractError,
            fabric::RetryAdvice::Never,
        ));
    }
    Ok(CommandReceipt {
        endpoint,
        request_id: receipt.receipt.request_id,
        attempt: receipt.receipt.attempt,
        provider_generation: generation,
        confidence: match receipt.receipt.confidence {
            wire_receipt::ReceiptConfidence::AcceptedForDelivery => {
                ReceiptConfidence::AcceptedForDelivery
            }
            wire_receipt::ReceiptConfidence::ProtocolOrTargetAcknowledged => {
                ReceiptConfidence::ProtocolOrTargetAcknowledged
            }
            wire_receipt::ReceiptConfidence::Completed => ReceiptConfidence::Completed,
            wire_receipt::ReceiptConfidence::ObservedCompleted => {
                ReceiptConfidence::ObservedCompleted
            }
        },
        accepted_at_unix_milliseconds: receipt.receipt.accepted_at_unix_milliseconds,
        completed_at_unix_milliseconds: receipt.receipt.completed_at_unix_milliseconds,
        idempotency_key: receipt.receipt.idempotency_key,
        deduplicated: false,
    })
}

fn host_notification_receipt(
    endpoint: fabric::EndpointIdentity,
    generation: fabric::Generation,
    request_id: &str,
    attempt: u32,
    idempotency_key: Option<&str>,
    receipt: wire_notification::SendReceipt,
) -> Result<NotificationReceipt, ProviderHostError> {
    if receipt.receipt.request_id != request_id
        || receipt.receipt.attempt != attempt
        || receipt.receipt.idempotency_key.as_deref() != idempotency_key
        || receipt.receipt.confidence != wire_receipt::ReceiptConfidence::AcceptedForDelivery
        || receipt
            .provider_message_id
            .as_ref()
            .is_some_and(|value| value.is_empty() || value.len() > 64 || !value.is_ascii())
    {
        return Err(service_error(
            fabric::CoreError::ContractError,
            fabric::RetryAdvice::Never,
        ));
    }
    Ok(NotificationReceipt {
        endpoint,
        request_id: receipt.receipt.request_id,
        attempt: receipt.receipt.attempt,
        provider_generation: generation,
        confidence: ReceiptConfidence::AcceptedForDelivery,
        accepted_at_unix_milliseconds: receipt.receipt.accepted_at_unix_milliseconds,
        idempotency_key: receipt.receipt.idempotency_key,
        provider_message_id: receipt.provider_message_id,
    })
}

fn wire_context(
    clock_started: StdInstant,
    deadline: tokio::time::Instant,
    request_id: &str,
    correlation_id: Option<String>,
    idempotency_key: Option<String>,
    attempt: u32,
) -> wire_receipt::RequestContext {
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    let absolute = clock_started
        .elapsed()
        .saturating_add(remaining)
        .as_nanos()
        .try_into()
        .unwrap_or(u64::MAX);
    wire_receipt::RequestContext {
        request_id: request_id.to_owned(),
        correlation_id,
        deadline: wire_receipt::Deadline {
            absolute_monotonic_nanoseconds: absolute,
        },
        idempotency: idempotency_key.map(|key| wire_receipt::IdempotencyContext {
            key,
            scope: "provider-facet".to_owned(),
        }),
        attempt,
    }
}

fn set_store_deadline(
    store: &mut Store<ProviderStore>,
    deadline: tokio::time::Instant,
) -> Result<(), ProviderHostError> {
    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
    if remaining.is_zero() {
        return Err(deadline_error(deadline));
    }
    let ticks = remaining.as_nanos().div_ceil(EPOCH_TICK.as_nanos());
    store.set_epoch_deadline(ticks.clamp(1, u64::MAX as u128) as u64);
    Ok(())
}

fn provider_error(error: wire_provider::Error) -> ProviderHostError {
    service_from_wire(error.inner, error.retry)
}

fn on_off_error(error: wire_on_off::Error) -> ProviderHostError {
    service_from_wire(error.inner, error.retry)
}

fn boolean_error(error: wire_boolean::Error) -> ProviderHostError {
    service_from_wire(error.inner, error.retry)
}

fn measurement_error(error: wire_measurement::Error) -> ProviderHostError {
    service_from_wire(error.inner, error.retry)
}

fn notification_error(error: wire_notification::Error) -> ProviderHostError {
    service_from_wire(error.inner, error.retry)
}

fn service_from_wire(
    inner: wire_error::CoreError,
    retry: wire_error::RetryAdvice,
) -> ProviderHostError {
    let inner = match inner {
        wire_error::CoreError::Binding(error) => fabric::CoreError::Binding(match error {
            wire_error::BindingError::NotFound => fabric::BindingError::NotFound,
            wire_error::BindingError::Ambiguous => fabric::BindingError::Ambiguous,
            wire_error::BindingError::StaleHandle => fabric::BindingError::StaleHandle,
            wire_error::BindingError::EndpointRemoved => fabric::BindingError::EndpointRemoved,
        }),
        wire_error::CoreError::Authorization(error) => {
            fabric::CoreError::Authorization(match error {
                wire_error::AuthorizationError::Denied => fabric::AuthorizationError::Denied,
                wire_error::AuthorizationError::ExpiredGrant => {
                    fabric::AuthorizationError::ExpiredGrant
                }
                wire_error::AuthorizationError::InsufficientScope => {
                    fabric::AuthorizationError::InsufficientScope
                }
            })
        }
        wire_error::CoreError::Availability(error) => {
            fabric::CoreError::Availability(match error {
                wire_error::AvailabilityError::Unavailable => {
                    fabric::AvailabilityError::Unavailable
                }
                wire_error::AvailabilityError::Reconnecting => {
                    fabric::AvailabilityError::Reconnecting
                }
                wire_error::AvailabilityError::ShuttingDown => {
                    fabric::AvailabilityError::ShuttingDown
                }
            })
        }
        wire_error::CoreError::Timing(error) => fabric::CoreError::Timing(match error {
            wire_error::TimingError::DeadlineExceeded(error) => {
                fabric::TimingError::DeadlineExceeded {
                    deadline_monotonic_nanoseconds: error.deadline_monotonic_nanoseconds,
                }
            }
            wire_error::TimingError::Cancelled(origin) => {
                fabric::TimingError::Cancelled(cancellation_origin(origin))
            }
        }),
        wire_error::CoreError::Capacity(wire_error::CapacityError::Overloaded(error)) => {
            fabric::CoreError::Capacity(fabric::CapacityError::Overloaded {
                retry_after_milliseconds: error.retry_after_milliseconds,
            })
        }
        wire_error::CoreError::Execution(error) => fabric::CoreError::Execution(match error {
            wire_error::ExecutionError::ProviderTrapped => fabric::ExecutionError::ProviderTrapped,
            wire_error::ExecutionError::ProtocolFailed => fabric::ExecutionError::ProtocolFailed,
            wire_error::ExecutionError::Unsupported => fabric::ExecutionError::Unsupported,
        }),
        wire_error::CoreError::OutcomeUnknown(error) => {
            fabric::CoreError::OutcomeUnknown(fabric::OutcomeUnknownError {
                cause: match error.cause {
                    wire_error::UnknownOutcomeCause::DeadlineExceeded => {
                        fabric::UnknownOutcomeCause::DeadlineExceeded
                    }
                    wire_error::UnknownOutcomeCause::Cancelled(origin) => {
                        fabric::UnknownOutcomeCause::Cancelled(cancellation_origin(origin))
                    }
                    wire_error::UnknownOutcomeCause::Disconnected => {
                        fabric::UnknownOutcomeCause::Disconnected
                    }
                    wire_error::UnknownOutcomeCause::ProviderFailed => {
                        fabric::UnknownOutcomeCause::ProviderFailed
                    }
                },
                request_id: error.request_id,
                attempt: error.attempt,
                idempotency_key: error.idempotency_key,
            })
        }
        wire_error::CoreError::ContractError => fabric::CoreError::ContractError,
    };
    let retry = match retry {
        wire_error::RetryAdvice::Never => fabric::RetryAdvice::Never,
        wire_error::RetryAdvice::Reconnect => fabric::RetryAdvice::Reconnect,
        wire_error::RetryAdvice::Rebind => fabric::RetryAdvice::Rebind,
        wire_error::RetryAdvice::RetryAfter(value) => {
            fabric::RetryAdvice::RetryAfter(value.map(|value| value.milliseconds))
        }
    };
    service_error(inner, retry)
}

fn cancellation_origin(origin: wire_error::CancellationOrigin) -> fabric::CancellationOrigin {
    match origin {
        wire_error::CancellationOrigin::Caller => fabric::CancellationOrigin::Caller,
        wire_error::CancellationOrigin::Provider => fabric::CancellationOrigin::Provider,
        wire_error::CancellationOrigin::Host => fabric::CancellationOrigin::Host,
        wire_error::CancellationOrigin::Shutdown => fabric::CancellationOrigin::Shutdown,
    }
}

fn deadline_error(deadline: tokio::time::Instant) -> ProviderHostError {
    service_error(
        fabric::CoreError::Timing(fabric::TimingError::DeadlineExceeded {
            deadline_monotonic_nanoseconds: deadline
                .saturating_duration_since(tokio::time::Instant::now())
                .as_nanos()
                .try_into()
                .unwrap_or(u64::MAX),
        }),
        fabric::RetryAdvice::Never,
    )
}

fn provider_trapped() -> ProviderHostError {
    service_error(
        fabric::CoreError::Execution(fabric::ExecutionError::ProviderTrapped),
        fabric::RetryAdvice::Reconnect,
    )
}

fn outcome_unknown(
    cause: fabric::UnknownOutcomeCause,
    request_id: String,
    attempt: u32,
    idempotency_key: Option<String>,
) -> ProviderHostError {
    service_error(
        fabric::CoreError::OutcomeUnknown(fabric::OutcomeUnknownError {
            cause,
            request_id,
            attempt,
            idempotency_key,
        }),
        fabric::RetryAdvice::Never,
    )
}

fn service_error(inner: fabric::CoreError, retry: fabric::RetryAdvice) -> ProviderHostError {
    ProviderHostError::Service(fabric::ServiceError {
        inner,
        retry,
        message: None,
        error_id: format!(
            "provider-{:016x}",
            ERROR_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::{HttpOrigin, tcp_address_allowed};
    use std::{collections::BTreeSet, net::SocketAddr};
    use wasmtime_wasi::sockets::SocketAddrUse;

    #[test]
    fn http_authority_is_exact_by_scheme_host_and_port() {
        let origin = HttpOrigin::new("https", "api.example.com", 443).unwrap();
        assert!(origin.permits("https", &"api.example.com".parse().unwrap()));
        assert!(origin.permits("https", &"api.example.com:443".parse().unwrap()));
        assert!(!origin.permits("http", &"api.example.com".parse().unwrap()));
        assert!(!origin.permits("https", &"other.example.com".parse().unwrap()));
        assert!(!origin.permits("https", &"api.example.com:444".parse().unwrap()));
        assert!(HttpOrigin::new("https", "bad.example/path", 443).is_err());
    }

    #[test]
    fn tcp_authority_is_exact_and_connect_only() {
        let allowed = "192.0.2.10:6053".parse::<SocketAddr>().unwrap();
        let denied = "192.0.2.10:6054".parse::<SocketAddr>().unwrap();
        let grants = BTreeSet::from([allowed]);
        assert!(tcp_address_allowed(
            &grants,
            allowed,
            SocketAddrUse::TcpConnect
        ));
        assert!(!tcp_address_allowed(
            &grants,
            denied,
            SocketAddrUse::TcpConnect
        ));
        assert!(!tcp_address_allowed(
            &grants,
            allowed,
            SocketAddrUse::TcpBind
        ));
    }
}
