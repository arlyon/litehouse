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
            "../litehouse-contracts/packages/level-control/1.0.0/wit",
            "../litehouse-contracts/packages/color-control/1.0.0/wit",
            "../litehouse-contracts/packages/thermostat/1.0.0/wit",
            "../litehouse-contracts/packages/window-covering/1.0.0/wit",
            "../litehouse-contracts/packages/momentary-action/1.0.0/wit",
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
    boolean_state::types as wire_boolean, color_control::types as wire_color,
    descriptor::types as wire_descriptor, errors::types as wire_error,
    level_control::types as wire_level, measurement::types as wire_measurement,
    momentary_action::types as wire_momentary, notification_sink::types as wire_notification,
    on_off::types as wire_on_off, quantity::types as wire_quantity, receipt::types as wire_receipt,
    state::types as wire_state, thermostat::types as wire_thermostat,
    window_covering::types as wire_covering,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetKind {
    OnOff,
    BooleanState,
    Measurement,
    NotificationSink,
    LevelControl,
    ColorControl,
    Thermostat,
    WindowCovering,
    MomentaryAction,
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
    Level(LevelState),
    Color(ColorState),
    Thermostat(ThermostatState),
    WindowCovering(WindowCoveringState),
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
pub struct Quantity {
    pub profile: String,
    pub magnitude: String,
    pub unit: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelState {
    pub current: Option<Quantity>,
    pub target: Option<Quantity>,
    pub transition_remaining_milliseconds: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    pub duration_milliseconds: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelSetRequest {
    pub level: Quantity,
    pub transition: Option<Transition>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepDirection {
    Increase,
    Decrease,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelStepRequest {
    pub direction: StepDirection,
    pub amount: Quantity,
    pub transition: Option<Transition>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelMoveRequest {
    pub direction: StepDirection,
    pub rate_per_second: Quantity,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoupledLevelRequest {
    pub level: Quantity,
    pub on_when_nonzero: bool,
    pub transition: Option<Transition>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoupledLevelStepRequest {
    pub direction: StepDirection,
    pub amount: Quantity,
    pub on_when_nonzero: bool,
    pub transition: Option<Transition>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoupledLevelMoveRequest {
    pub direction: StepDirection,
    pub rate_per_second: Quantity,
    pub on_when_nonzero: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelOperation {
    SetLevel(LevelSetRequest),
    Step(LevelStepRequest),
    Move(LevelMoveRequest),
    Stop,
    SetLevelWithOnOff(CoupledLevelRequest),
    StepWithOnOff(CoupledLevelStepRequest),
    MoveWithOnOff(CoupledLevelMoveRequest),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelOperationReceipt {
    pub operation: LevelOperation,
    pub receipt: CommandReceipt,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorValue {
    HueSaturation {
        hue_degrees: String,
        saturation_ratio: String,
    },
    CieXy {
        x: String,
        y: String,
    },
    ColorTemperature {
        kelvin: String,
    },
    CalibratedRgb {
        red: String,
        green: String,
        blue: String,
        color_space: String,
    },
    UncalibratedRgb {
        red: String,
        green: String,
        blue: String,
    },
    NativeWhiteChannels {
        white: Option<String>,
        cold_white: Option<String>,
        warm_white: Option<String>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorModel {
    HueSaturation,
    CieXy,
    ColorTemperature,
    CalibratedRgb,
    UncalibratedRgb,
    NativeWhiteChannels,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorState {
    pub current: Option<ColorValue>,
    pub target: Option<ColorValue>,
    pub active_model: Option<ColorModel>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorSetRequest {
    pub color: ColorValue,
    pub transition: Option<Transition>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorStepRequest {
    pub delta: ColorValue,
    pub transition: Option<Transition>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorMoveRequest {
    pub rate_per_second: ColorValue,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorOperation {
    SetColor(ColorSetRequest),
    StepColor(ColorStepRequest),
    MoveColor(ColorMoveRequest),
    Stop,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorOperationReceipt {
    pub operation: ColorOperation,
    pub receipt: CommandReceipt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownThermostatMode {
    Off,
    Auto,
    Heat,
    Cool,
    Dry,
    FanOnly,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThermostatMode {
    Known(KnownThermostatMode),
    ProviderQualified(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermostatAction {
    Idle,
    Heating,
    Cooling,
    Drying,
    Fan,
    Defrosting,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThermostatTargets {
    Single(Quantity),
    HeatCool { heat: Quantity, cool: Quantity },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermostatState {
    pub current_mode: Option<ThermostatMode>,
    pub target_mode: Option<ThermostatMode>,
    pub current_action: Option<ThermostatAction>,
    pub running: Option<bool>,
    pub targets: Option<ThermostatTargets>,
    pub fan: Option<String>,
    pub swing: Option<String>,
    pub preset: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThermostatOperation {
    SetMode(ThermostatMode),
    SetSingleTarget(Quantity),
    SetDualTarget { heat: Quantity, cool: Quantity },
    SetFan(String),
    SetSwing(String),
    SetPreset(String),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThermostatOperationReceipt {
    pub operation: ThermostatOperation,
    pub receipt: CommandReceipt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoveringAxis {
    Lift,
    Tilt,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoveringMovement {
    Opening,
    Closing,
    Stopped,
    Idle,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveringAxisState {
    pub axis: CoveringAxis,
    pub current: Option<Quantity>,
    pub target: Option<Quantity>,
    pub movement: CoveringMovement,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowCoveringState {
    pub axes: Vec<CoveringAxisState>,
    pub safety_status: Vec<String>,
    pub fault_status: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoveringOperation {
    Open(CoveringAxis),
    Close(CoveringAxis),
    Stop(CoveringAxis),
    SetPosition {
        axis: CoveringAxis,
        position: Quantity,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoveringOperationReceipt {
    pub operation: CoveringOperation,
    pub receipt: CommandReceipt,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MomentaryActionReceipt {
    pub action_id: String,
    pub receipt: CommandReceipt,
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
impl wire_color::Host for ProviderStore {}
impl wire_level::Host for ProviderStore {}
impl wire_measurement::Host for ProviderStore {}
impl wire_momentary::Host for ProviderStore {}
impl wire_notification::Host for ProviderStore {}
impl wire_on_off::Host for ProviderStore {}
impl wire_quantity::Host for ProviderStore {}
impl wire_thermostat::Host for ProviderStore {}
impl wire_covering::Host for ProviderStore {}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BooleanOccurrence {
    pub endpoint: fabric::EndpointIdentity,
    pub sequence: u64,
    pub event_id: String,
    pub source_id: String,
    pub generation: fabric::Generation,
    pub received_at_unix_milliseconds: u64,
    pub value: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BooleanOccurrenceBatch {
    pub events: Vec<BooleanOccurrence>,
    pub dropped: u64,
}

struct ActorState {
    store: Store<ProviderStore>,
    component: bindings::Component,
    runner: ResourceAny,
    handles: BTreeMap<HandleKey, ResourceAny>,
    provider: fabric::InstanceGeneration,
    fabric: Arc<Mutex<fabric::ServiceFabric>>,
    idempotency: IdempotencyCache,
    boolean_occurrence_facets: BTreeSet<String>,
    last_boolean_occurrence_sequence: Option<u64>,
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
    LevelCommand {
        endpoint: fabric::EndpointIdentity,
        operation: LevelOperation,
        context: CallContext,
        reply: oneshot::Sender<Result<CommandReceipt, ProviderHostError>>,
    },
    ColorCommand {
        endpoint: fabric::EndpointIdentity,
        operation: ColorOperation,
        context: CallContext,
        reply: oneshot::Sender<Result<CommandReceipt, ProviderHostError>>,
    },
    ThermostatCommand {
        endpoint: fabric::EndpointIdentity,
        operation: ThermostatOperation,
        context: CallContext,
        reply: oneshot::Sender<Result<CommandReceipt, ProviderHostError>>,
    },
    CoveringCommand {
        endpoint: fabric::EndpointIdentity,
        operation: CoveringOperation,
        context: CallContext,
        reply: oneshot::Sender<Result<CommandReceipt, ProviderHostError>>,
    },
    InvokeMomentary {
        endpoint: fabric::EndpointIdentity,
        context: CallContext,
        reply: oneshot::Sender<Result<MomentaryActionReceipt, ProviderHostError>>,
    },
    DrainBooleanOccurrences {
        maximum: u16,
        deadline: tokio::time::Instant,
        reply: oneshot::Sender<Result<BooleanOccurrenceBatch, ProviderHostError>>,
    },
    Shutdown {
        reply: oneshot::Sender<Result<(), ProviderHostError>>,
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

    pub async fn shutdown(&self, timeout: Duration) -> Result<(), ProviderHostError> {
        if timeout.is_zero() {
            return Err(ProviderHostError::InvalidConfiguration);
        }
        let deadline = tokio::time::Instant::now() + timeout;
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::Shutdown { reply });
        tokio::time::timeout_at(deadline, result)
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?
    }

    pub async fn drain_boolean_occurrences(
        &self,
        maximum: u16,
        timeout: Duration,
    ) -> Result<BooleanOccurrenceBatch, ProviderHostError> {
        if maximum == 0 || maximum > 64 || timeout.is_zero() {
            return Err(ProviderHostError::InvalidConfiguration);
        }
        let deadline = tokio::time::Instant::now() + timeout;
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::DrainBooleanOccurrences {
            maximum,
            deadline,
            reply,
        });
        tokio::time::timeout_at(deadline, result)
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?
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

    pub async fn set_level(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        r: LevelSetRequest,
        c: CallContext,
    ) -> Result<LevelOperationReceipt, ProviderHostError> {
        self.call_level(e, g, LevelOperation::SetLevel(r), c).await
    }
    pub async fn step_level(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        r: LevelStepRequest,
        c: CallContext,
    ) -> Result<LevelOperationReceipt, ProviderHostError> {
        self.call_level(e, g, LevelOperation::Step(r), c).await
    }
    pub async fn move_level(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        r: LevelMoveRequest,
        c: CallContext,
    ) -> Result<LevelOperationReceipt, ProviderHostError> {
        self.call_level(e, g, LevelOperation::Move(r), c).await
    }
    pub async fn stop_level(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        c: CallContext,
    ) -> Result<LevelOperationReceipt, ProviderHostError> {
        self.call_level(e, g, LevelOperation::Stop, c).await
    }
    pub async fn set_level_with_on_off(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        r: CoupledLevelRequest,
        c: CallContext,
    ) -> Result<LevelOperationReceipt, ProviderHostError> {
        self.call_level(e, g, LevelOperation::SetLevelWithOnOff(r), c)
            .await
    }
    pub async fn step_level_with_on_off(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        r: CoupledLevelStepRequest,
        c: CallContext,
    ) -> Result<LevelOperationReceipt, ProviderHostError> {
        self.call_level(e, g, LevelOperation::StepWithOnOff(r), c)
            .await
    }
    pub async fn move_level_with_on_off(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        r: CoupledLevelMoveRequest,
        c: CallContext,
    ) -> Result<LevelOperationReceipt, ProviderHostError> {
        self.call_level(e, g, LevelOperation::MoveWithOnOff(r), c)
            .await
    }
    async fn call_level(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        operation: LevelOperation,
        c: CallContext,
    ) -> Result<LevelOperationReceipt, ProviderHostError> {
        self.validate_target(e, g)?;
        let original = operation.clone();
        let receipt = self.send_level(e.clone(), operation, c).await?;
        Ok(LevelOperationReceipt {
            operation: original,
            receipt,
        })
    }
    async fn send_level(
        &self,
        endpoint: fabric::EndpointIdentity,
        operation: LevelOperation,
        context: CallContext,
    ) -> Result<CommandReceipt, ProviderHostError> {
        let deadline = context.deadline;
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::LevelCommand {
            endpoint,
            operation,
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
                    key,
                )
            })?
            .map_err(|_| ProviderHostError::ActorStopped)?
    }

    pub async fn set_color(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        r: ColorSetRequest,
        c: CallContext,
    ) -> Result<ColorOperationReceipt, ProviderHostError> {
        self.call_color(e, g, ColorOperation::SetColor(r), c).await
    }
    pub async fn step_color(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        r: ColorStepRequest,
        c: CallContext,
    ) -> Result<ColorOperationReceipt, ProviderHostError> {
        self.call_color(e, g, ColorOperation::StepColor(r), c).await
    }
    pub async fn move_color(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        r: ColorMoveRequest,
        c: CallContext,
    ) -> Result<ColorOperationReceipt, ProviderHostError> {
        self.call_color(e, g, ColorOperation::MoveColor(r), c).await
    }
    pub async fn stop_color(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        c: CallContext,
    ) -> Result<ColorOperationReceipt, ProviderHostError> {
        self.call_color(e, g, ColorOperation::Stop, c).await
    }
    async fn call_color(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        operation: ColorOperation,
        context: CallContext,
    ) -> Result<ColorOperationReceipt, ProviderHostError> {
        self.validate_target(e, g)?;
        let original = operation.clone();
        let deadline = context.deadline;
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::ColorCommand {
            endpoint: e.clone(),
            operation,
            context,
            reply,
        });
        let receipt = tokio::time::timeout_at(deadline, result)
            .await
            .map_err(|_| {
                outcome_unknown(
                    fabric::UnknownOutcomeCause::DeadlineExceeded,
                    request_id,
                    attempt,
                    key,
                )
            })?
            .map_err(|_| ProviderHostError::ActorStopped)??;
        Ok(ColorOperationReceipt {
            operation: original,
            receipt,
        })
    }

    pub async fn set_thermostat_mode(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        m: ThermostatMode,
        c: CallContext,
    ) -> Result<ThermostatOperationReceipt, ProviderHostError> {
        self.call_thermostat(e, g, ThermostatOperation::SetMode(m), c)
            .await
    }
    pub async fn set_thermostat_single_target(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        t: Quantity,
        c: CallContext,
    ) -> Result<ThermostatOperationReceipt, ProviderHostError> {
        self.call_thermostat(e, g, ThermostatOperation::SetSingleTarget(t), c)
            .await
    }
    pub async fn set_thermostat_dual_target(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        h: Quantity,
        cool: Quantity,
        c: CallContext,
    ) -> Result<ThermostatOperationReceipt, ProviderHostError> {
        self.call_thermostat(
            e,
            g,
            ThermostatOperation::SetDualTarget { heat: h, cool },
            c,
        )
        .await
    }
    pub async fn set_thermostat_fan(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        v: String,
        c: CallContext,
    ) -> Result<ThermostatOperationReceipt, ProviderHostError> {
        self.call_thermostat(e, g, ThermostatOperation::SetFan(v), c)
            .await
    }
    pub async fn set_thermostat_swing(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        v: String,
        c: CallContext,
    ) -> Result<ThermostatOperationReceipt, ProviderHostError> {
        self.call_thermostat(e, g, ThermostatOperation::SetSwing(v), c)
            .await
    }
    pub async fn set_thermostat_preset(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        v: String,
        c: CallContext,
    ) -> Result<ThermostatOperationReceipt, ProviderHostError> {
        self.call_thermostat(e, g, ThermostatOperation::SetPreset(v), c)
            .await
    }
    async fn call_thermostat(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        operation: ThermostatOperation,
        context: CallContext,
    ) -> Result<ThermostatOperationReceipt, ProviderHostError> {
        self.validate_target(e, g)?;
        let original = operation.clone();
        let deadline = context.deadline;
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::ThermostatCommand {
            endpoint: e.clone(),
            operation,
            context,
            reply,
        });
        let receipt = tokio::time::timeout_at(deadline, result)
            .await
            .map_err(|_| {
                outcome_unknown(
                    fabric::UnknownOutcomeCause::DeadlineExceeded,
                    request_id,
                    attempt,
                    key,
                )
            })?
            .map_err(|_| ProviderHostError::ActorStopped)??;
        Ok(ThermostatOperationReceipt {
            operation: original,
            receipt,
        })
    }

    pub async fn open_covering(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        a: CoveringAxis,
        c: CallContext,
    ) -> Result<CoveringOperationReceipt, ProviderHostError> {
        self.call_covering(e, g, CoveringOperation::Open(a), c)
            .await
    }
    pub async fn close_covering(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        a: CoveringAxis,
        c: CallContext,
    ) -> Result<CoveringOperationReceipt, ProviderHostError> {
        self.call_covering(e, g, CoveringOperation::Close(a), c)
            .await
    }
    pub async fn stop_covering(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        a: CoveringAxis,
        c: CallContext,
    ) -> Result<CoveringOperationReceipt, ProviderHostError> {
        self.call_covering(e, g, CoveringOperation::Stop(a), c)
            .await
    }
    pub async fn set_covering_position(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        a: CoveringAxis,
        p: Quantity,
        c: CallContext,
    ) -> Result<CoveringOperationReceipt, ProviderHostError> {
        self.call_covering(
            e,
            g,
            CoveringOperation::SetPosition {
                axis: a,
                position: p,
            },
            c,
        )
        .await
    }
    async fn call_covering(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        operation: CoveringOperation,
        context: CallContext,
    ) -> Result<CoveringOperationReceipt, ProviderHostError> {
        self.validate_target(e, g)?;
        let original = operation.clone();
        let deadline = context.deadline;
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::CoveringCommand {
            endpoint: e.clone(),
            operation,
            context,
            reply,
        });
        let receipt = tokio::time::timeout_at(deadline, result)
            .await
            .map_err(|_| {
                outcome_unknown(
                    fabric::UnknownOutcomeCause::DeadlineExceeded,
                    request_id,
                    attempt,
                    key,
                )
            })?
            .map_err(|_| ProviderHostError::ActorStopped)??;
        Ok(CoveringOperationReceipt {
            operation: original,
            receipt,
        })
    }

    pub async fn invoke_momentary_action(
        &self,
        e: &fabric::EndpointIdentity,
        g: fabric::Generation,
        context: CallContext,
    ) -> Result<MomentaryActionReceipt, ProviderHostError> {
        self.validate_target(e, g)?;
        let deadline = context.deadline;
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let (reply, result) = oneshot::channel();
        let permit = tokio::time::timeout_at(deadline, self.sender.reserve())
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::ActorStopped)?;
        permit.send(ActorRequest::InvokeMomentary {
            endpoint: e.clone(),
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
                    key,
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
    let (
        on_off_handles,
        boolean_handles,
        measurement_handles,
        notification_handles,
        level_handles,
        color_handles,
        thermostat_handles,
        covering_handles,
        momentary_handles,
    ) = tokio::time::timeout_at(
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
            let level = guest
                .runner()
                .call_level_control_facets(accessor, runner)
                .await?;
            let color = guest
                .runner()
                .call_color_control_facets(accessor, runner)
                .await?;
            let thermostat = guest
                .runner()
                .call_thermostat_facets(accessor, runner)
                .await?;
            let covering = guest
                .runner()
                .call_window_covering_facets(accessor, runner)
                .await?;
            let momentary = guest
                .runner()
                .call_momentary_action_facets(accessor, runner)
                .await?;
            Ok::<_, wasmtime::Error>((
                on_off,
                boolean,
                measurement,
                notification,
                level,
                color,
                thermostat,
                covering,
                momentary,
            ))
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
    let level_handles = level_handles.map_err(provider_error)?;
    let color_handles = color_handles.map_err(provider_error)?;
    let thermostat_handles = thermostat_handles.map_err(provider_error)?;
    let covering_handles = covering_handles.map_err(provider_error)?;
    let momentary_handles = momentary_handles.map_err(provider_error)?;
    if on_off_handles.len()
        + boolean_handles.len()
        + measurement_handles.len()
        + notification_handles.len()
        + level_handles.len()
        + color_handles.len()
        + thermostat_handles.len()
        + covering_handles.len()
        + momentary_handles.len()
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
    for handle in level_handles {
        let (key, incarnation, descriptor) = describe_level(
            &mut store,
            guest,
            handle,
            &config.instance,
            config.generation,
            deadline,
        )
        .await?;
        insert_handle(
            &mut handles,
            &mut descriptors,
            FacetKind::LevelControl,
            key,
            incarnation,
            handle,
            descriptor,
        )?;
    }
    for handle in color_handles {
        let (key, incarnation, descriptor) = describe_color(
            &mut store,
            guest,
            handle,
            &config.instance,
            config.generation,
            deadline,
        )
        .await?;
        insert_handle(
            &mut handles,
            &mut descriptors,
            FacetKind::ColorControl,
            key,
            incarnation,
            handle,
            descriptor,
        )?;
    }
    for handle in thermostat_handles {
        let (key, incarnation, descriptor) = describe_thermostat(
            &mut store,
            guest,
            handle,
            &config.instance,
            config.generation,
            deadline,
        )
        .await?;
        insert_handle(
            &mut handles,
            &mut descriptors,
            FacetKind::Thermostat,
            key,
            incarnation,
            handle,
            descriptor,
        )?;
    }
    for handle in covering_handles {
        let (key, incarnation, descriptor) = describe_covering(
            &mut store,
            guest,
            handle,
            &config.instance,
            config.generation,
            deadline,
        )
        .await?;
        insert_handle(
            &mut handles,
            &mut descriptors,
            FacetKind::WindowCovering,
            key,
            incarnation,
            handle,
            descriptor,
        )?;
    }
    for handle in momentary_handles {
        let (key, incarnation, descriptor) = describe_momentary(
            &mut store,
            guest,
            handle,
            &config.instance,
            config.generation,
            deadline,
        )
        .await?;
        insert_handle(
            &mut handles,
            &mut descriptors,
            FacetKind::MomentaryAction,
            key,
            incarnation,
            handle,
            descriptor,
        )?;
    }
    descriptors.sort_by(|left, right| left.endpoint().identity().cmp(right.endpoint().identity()));
    let boolean_occurrence_facets = descriptors
        .iter()
        .filter(|descriptor| descriptor.kind == FacetKind::BooleanState)
        .filter(|descriptor| {
            descriptor
                .endpoint()
                .features()
                .iter()
                .any(|feature| feature.as_str() == "occurrence")
        })
        .map(|descriptor| descriptor.endpoint().identity().facet().as_str().to_owned())
        .collect();

    let provider = fabric::InstanceGeneration::new(config.instance, config.generation);
    publish_provider(&fabric, &provider, &descriptors)?;
    let (sender, receiver) = mpsc::channel(ACTOR_QUEUE_CAPACITY);
    let actor = ActorState {
        store,
        component: bound,
        runner,
        handles,
        provider: provider.clone(),
        fabric,
        idempotency: IdempotencyCache::default(),
        boolean_occurrence_facets,
        last_boolean_occurrence_sequence: None,
    };
    tokio::spawn(run_actor(actor, receiver));
    Ok(ProviderHandle {
        provider,
        descriptors: Arc::new(descriptors),
        sender,
    })
}

fn insert_handle(
    handles: &mut BTreeMap<HandleKey, ResourceAny>,
    descriptors: &mut Vec<HostFacetDescriptor>,
    kind: FacetKind,
    key: String,
    incarnation: u64,
    handle: ResourceAny,
    descriptor: HostFacetDescriptor,
) -> Result<(), ProviderHostError> {
    if incarnation == 0
        || handles
            .insert(HandleKey { kind, facet: key }, handle)
            .is_some()
    {
        return Err(ProviderHostError::DuplicateFacet);
    }
    descriptors.push(descriptor);
    Ok(())
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

macro_rules! describe_typed_facet {
    ($name:ident,$accessor:ident,$label:literal,$host:ident,$error:ident) => {
        async fn $name(
            store: &mut Store<ProviderStore>,
            guest: &wire_provider::Guest,
            handle: ResourceAny,
            instance: &fabric::InstanceId,
            generation: fabric::Generation,
            deadline: tokio::time::Instant,
        ) -> Result<(String, u64, HostFacetDescriptor), ProviderHostError> {
            set_store_deadline(store, deadline)?;
            let key = guest
                .$accessor()
                .call_key(&mut *store, handle)
                .await
                .map_err(|_| ProviderHostError::InvalidDescriptor)?;
            let incarnation = guest
                .$accessor()
                .call_incarnation(&mut *store, handle)
                .await
                .map_err(|_| ProviderHostError::InvalidDescriptor)?;
            let context = wire_context(store.data().clock_started, deadline, $label, None, None, 0);
            let descriptor = tokio::time::timeout_at(
                deadline,
                store.run_concurrent(async |a| {
                    guest.$accessor().call_describe(a, handle, context).await
                }),
            )
            .await
            .map_err(|_| deadline_error(deadline))?
            .map_err(|_| ProviderHostError::InvalidDescriptor)?
            .map_err(|_| ProviderHostError::InvalidDescriptor)?
            .map_err($error)?;
            let host = $host(instance, generation, &key, incarnation, descriptor)?;
            Ok((key, incarnation, host))
        }
    };
}
describe_typed_facet!(
    describe_level,
    level_control_facet,
    "describe-level-control",
    host_level_descriptor,
    level_error
);
describe_typed_facet!(
    describe_color,
    color_control_facet,
    "describe-color-control",
    host_color_descriptor,
    color_error
);
describe_typed_facet!(
    describe_thermostat,
    thermostat_facet,
    "describe-thermostat",
    host_thermostat_descriptor,
    thermostat_error
);
describe_typed_facet!(
    describe_covering,
    window_covering_facet,
    "describe-window-covering",
    host_covering_descriptor,
    covering_error
);
describe_typed_facet!(
    describe_momentary,
    momentary_action_facet,
    "describe-momentary-action",
    host_momentary_descriptor,
    momentary_error
);

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
            ActorRequest::LevelCommand {
                endpoint,
                operation,
                context,
                reply,
            } => {
                let (r, f) = actor.level_command(endpoint, operation, context).await;
                let _ = reply.send(r);
                f
            }
            ActorRequest::ColorCommand {
                endpoint,
                operation,
                context,
                reply,
            } => {
                let (r, f) = actor.color_command(endpoint, operation, context).await;
                let _ = reply.send(r);
                f
            }
            ActorRequest::ThermostatCommand {
                endpoint,
                operation,
                context,
                reply,
            } => {
                let (r, f) = actor.thermostat_command(endpoint, operation, context).await;
                let _ = reply.send(r);
                f
            }
            ActorRequest::CoveringCommand {
                endpoint,
                operation,
                context,
                reply,
            } => {
                let (r, f) = actor.covering_command(endpoint, operation, context).await;
                let _ = reply.send(r);
                f
            }
            ActorRequest::InvokeMomentary {
                endpoint,
                context,
                reply,
            } => {
                let (r, f) = actor.invoke_momentary(endpoint, context).await;
                let _ = reply.send(r);
                f
            }
            ActorRequest::DrainBooleanOccurrences {
                maximum,
                deadline,
                reply,
            } => {
                let (result, fatal) = actor.drain_boolean_occurrences(maximum, deadline).await;
                let _ = reply.send(result);
                fatal
            }
            ActorRequest::Shutdown { reply } => {
                receiver.close();
                let result = actor
                    .fabric
                    .lock()
                    .map_err(|_| ProviderHostError::FabricUnavailable)
                    .and_then(|mut fabric| {
                        fabric
                            .trap_provider(&actor.provider)
                            .map_err(|_| ProviderHostError::FabricRejected)
                    });
                drop(actor);
                let _ = reply.send(result);
                return;
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

fn validate_boolean_occurrence_sequences(
    previous: Option<u64>,
    dropped: u64,
    sequences: &[u64],
) -> Result<Option<u64>, ProviderHostError> {
    let Some(&first) = sequences.first() else {
        return if dropped == 0 {
            Ok(previous)
        } else {
            Err(ProviderHostError::InvalidDescriptor)
        };
    };
    let expected = previous
        .map_or(Some(1), |sequence| sequence.checked_add(1))
        .and_then(|sequence| sequence.checked_add(dropped))
        .ok_or(ProviderHostError::InvalidDescriptor)?;
    if first != expected
        || sequences
            .windows(2)
            .any(|pair| pair[0].checked_add(1) != Some(pair[1]))
    {
        return Err(ProviderHostError::InvalidDescriptor);
    }
    Ok(sequences.last().copied())
}

impl ActorState {
    async fn drain_boolean_occurrences(
        &mut self,
        maximum: u16,
        deadline: tokio::time::Instant,
    ) -> (Result<BooleanOccurrenceBatch, ProviderHostError>, bool) {
        if let Err(error) = set_store_deadline(&mut self.store, deadline) {
            return (Err(error), false);
        }
        let guest = self.component.litehouse_provider_provider();
        let runner = self.runner;
        let call = tokio::time::timeout_at(
            deadline,
            self.store.run_concurrent(async |accessor| {
                guest
                    .runner()
                    .call_drain_boolean_occurrences(accessor, runner, maximum)
                    .await
            }),
        )
        .await;
        let batch = match call {
            Err(_) => return (Err(deadline_error(deadline)), false),
            Ok(Err(_)) | Ok(Ok(Err(_))) => {
                return (Err(ProviderHostError::ActorStopped), true);
            }
            Ok(Ok(Ok(Err(error)))) => return (Err(provider_error(error)), false),
            Ok(Ok(Ok(Ok(batch)))) => batch,
        };
        if batch.events.len() > usize::from(maximum) {
            return (Err(ProviderHostError::InvalidDescriptor), false);
        }
        let sequences = batch
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>();
        let last_sequence = match validate_boolean_occurrence_sequences(
            self.last_boolean_occurrence_sequence,
            batch.dropped,
            &sequences,
        ) {
            Ok(sequence) => sequence,
            Err(error) => return (Err(error), false),
        };
        let mut seen = BTreeSet::new();
        let mut events = Vec::with_capacity(batch.events.len());
        for event in batch.events {
            if !valid_event_token(&event.occurrence.event_id)
                || !valid_event_token(&event.occurrence.source_id)
                || event.occurrence.generation == 0
                || event.occurrence.received_at_unix_milliseconds == 0
                || !seen.insert(event.occurrence.event_id.clone())
            {
                return (Err(ProviderHostError::InvalidDescriptor), false);
            }
            let Some(descriptor) = self.descriptors_for_boolean(&event.facet_key) else {
                return (Err(ProviderHostError::InvalidDescriptor), false);
            };
            events.push(BooleanOccurrence {
                endpoint: descriptor,
                sequence: event.sequence,
                event_id: event.occurrence.event_id,
                source_id: event.occurrence.source_id,
                generation: self.provider.generation(),
                received_at_unix_milliseconds: event.occurrence.received_at_unix_milliseconds,
                value: event.occurrence.value,
            });
        }
        self.last_boolean_occurrence_sequence = last_sequence;
        (
            Ok(BooleanOccurrenceBatch {
                events,
                dropped: batch.dropped,
            }),
            false,
        )
    }

    fn descriptors_for_boolean(&self, facet_key: &str) -> Option<fabric::EndpointIdentity> {
        self.boolean_occurrence_facets
            .contains(facet_key)
            .then_some(())
            .filter(|()| {
                self.handles.contains_key(&HandleKey {
                    kind: FacetKind::BooleanState,
                    facet: facet_key.to_owned(),
                })
            })
            .and_then(|()| {
                Some(fabric::EndpointIdentity::new(
                    self.provider.instance().clone(),
                    fabric::FacetKey::new(facet_key).ok()?,
                    fabric::ContractId::new("litehouse", "boolean-state", 1).ok()?,
                ))
            })
    }

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
        } else if endpoint.contract().name() == "level-control" {
            FacetKind::LevelControl
        } else if endpoint.contract().name() == "color-control" {
            FacetKind::ColorControl
        } else if endpoint.contract().name() == "thermostat" {
            FacetKind::Thermostat
        } else if endpoint.contract().name() == "window-covering" {
            FacetKind::WindowCovering
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
            FacetKind::LevelControl => {
                let result = tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .level_control_facet()
                            .call_query(a, handle, wire)
                            .await
                    }),
                )
                .await;
                match result {
                    Err(_) => return (Err(deadline_error(context.deadline)), true),
                    Ok(Err(_)) | Ok(Ok(Err(_))) => return (Err(provider_trapped()), true),
                    Ok(Ok(Ok(Err(e)))) => return (Err(level_error(e)), false),
                    Ok(Ok(Ok(Ok(s)))) => host_level_state(endpoint, self.provider.generation(), s),
                }
            }
            FacetKind::ColorControl => {
                let result = tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .color_control_facet()
                            .call_query(a, handle, wire)
                            .await
                    }),
                )
                .await;
                match result {
                    Err(_) => return (Err(deadline_error(context.deadline)), true),
                    Ok(Err(_)) | Ok(Ok(Err(_))) => return (Err(provider_trapped()), true),
                    Ok(Ok(Ok(Err(e)))) => return (Err(color_error(e)), false),
                    Ok(Ok(Ok(Ok(s)))) => host_color_state(endpoint, self.provider.generation(), s),
                }
            }
            FacetKind::Thermostat => {
                let result = tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .thermostat_facet()
                            .call_query(a, handle, wire)
                            .await
                    }),
                )
                .await;
                match result {
                    Err(_) => return (Err(deadline_error(context.deadline)), true),
                    Ok(Err(_)) | Ok(Ok(Err(_))) => return (Err(provider_trapped()), true),
                    Ok(Ok(Ok(Err(e)))) => return (Err(thermostat_error(e)), false),
                    Ok(Ok(Ok(Ok(s)))) => {
                        host_thermostat_state(endpoint, self.provider.generation(), s)
                    }
                }
            }
            FacetKind::WindowCovering => {
                let result = tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .window_covering_facet()
                            .call_query(a, handle, wire)
                            .await
                    }),
                )
                .await;
                match result {
                    Err(_) => return (Err(deadline_error(context.deadline)), true),
                    Ok(Err(_)) | Ok(Ok(Err(_))) => return (Err(provider_trapped()), true),
                    Ok(Ok(Ok(Err(e)))) => return (Err(covering_error(e)), false),
                    Ok(Ok(Ok(Ok(s)))) => {
                        host_covering_state(endpoint, self.provider.generation(), s)
                    }
                }
            }
            FacetKind::NotificationSink | FacetKind::MomentaryAction => {
                unreachable!("facet is not queryable")
            }
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
    async fn level_command(
        &mut self,
        endpoint: fabric::EndpointIdentity,
        operation: LevelOperation,
        context: CallContext,
    ) -> (Result<CommandReceipt, ProviderHostError>, bool) {
        let Some(handle) = self
            .handles
            .get(&HandleKey {
                kind: FacetKind::LevelControl,
                facet: endpoint.facet().as_str().to_owned(),
            })
            .copied()
        else {
            return (Err(endpoint_removed()), false);
        };
        if let Err(e) = set_store_deadline(&mut self.store, context.deadline) {
            return (Err(e), false);
        }
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let wire = wire_context(
            self.store.data().clock_started,
            context.deadline,
            &request_id,
            context.correlation_id,
            key.clone(),
            attempt,
        );
        let expected = operation.clone();
        let call = match operation {
            LevelOperation::SetLevel(v) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .level_control_facet()
                            .call_set_level(a, handle, wire, wire_level_set(v))
                            .await
                    }),
                )
                .await
            }
            LevelOperation::Step(v) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .level_control_facet()
                            .call_step(a, handle, wire, wire_level_step(v))
                            .await
                    }),
                )
                .await
            }
            LevelOperation::Move(v) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .level_control_facet()
                            .call_move(a, handle, wire, wire_level_move(v))
                            .await
                    }),
                )
                .await
            }
            LevelOperation::Stop => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .level_control_facet()
                            .call_stop(a, handle, wire)
                            .await
                    }),
                )
                .await
            }
            LevelOperation::SetLevelWithOnOff(v) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .level_control_facet()
                            .call_set_level_with_on_off(a, handle, wire, wire_coupled_level(v))
                            .await
                    }),
                )
                .await
            }
            LevelOperation::StepWithOnOff(v) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .level_control_facet()
                            .call_step_with_on_off(a, handle, wire, wire_coupled_step(v))
                            .await
                    }),
                )
                .await
            }
            LevelOperation::MoveWithOnOff(v) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .level_control_facet()
                            .call_move_with_on_off(a, handle, wire, wire_coupled_move(v))
                            .await
                    }),
                )
                .await
            }
        };
        finish_level(
            call,
            endpoint,
            self.provider.generation(),
            &expected,
            request_id,
            attempt,
            key,
        )
    }
    async fn color_command(
        &mut self,
        endpoint: fabric::EndpointIdentity,
        operation: ColorOperation,
        context: CallContext,
    ) -> (Result<CommandReceipt, ProviderHostError>, bool) {
        let Some(handle) = self
            .handles
            .get(&HandleKey {
                kind: FacetKind::ColorControl,
                facet: endpoint.facet().as_str().to_owned(),
            })
            .copied()
        else {
            return (Err(endpoint_removed()), false);
        };
        if let Err(e) = set_store_deadline(&mut self.store, context.deadline) {
            return (Err(e), false);
        }
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let wire = wire_context(
            self.store.data().clock_started,
            context.deadline,
            &request_id,
            context.correlation_id,
            key.clone(),
            attempt,
        );
        let expected = operation.clone();
        let call = match operation {
            ColorOperation::SetColor(v) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .color_control_facet()
                            .call_set_color(a, handle, wire, wire_color_set(v))
                            .await
                    }),
                )
                .await
            }
            ColorOperation::StepColor(v) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .color_control_facet()
                            .call_step_color(a, handle, wire, wire_color_step(v))
                            .await
                    }),
                )
                .await
            }
            ColorOperation::MoveColor(v) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .color_control_facet()
                            .call_move_color(a, handle, wire, wire_color_move(v))
                            .await
                    }),
                )
                .await
            }
            ColorOperation::Stop => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .color_control_facet()
                            .call_stop(a, handle, wire)
                            .await
                    }),
                )
                .await
            }
        };
        finish_color(
            call,
            endpoint,
            self.provider.generation(),
            &expected,
            request_id,
            attempt,
            key,
        )
    }
    async fn thermostat_command(
        &mut self,
        endpoint: fabric::EndpointIdentity,
        operation: ThermostatOperation,
        context: CallContext,
    ) -> (Result<CommandReceipt, ProviderHostError>, bool) {
        let Some(handle) = self
            .handles
            .get(&HandleKey {
                kind: FacetKind::Thermostat,
                facet: endpoint.facet().as_str().to_owned(),
            })
            .copied()
        else {
            return (Err(endpoint_removed()), false);
        };
        if let Err(e) = set_store_deadline(&mut self.store, context.deadline) {
            return (Err(e), false);
        }
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let wire = wire_context(
            self.store.data().clock_started,
            context.deadline,
            &request_id,
            context.correlation_id,
            key.clone(),
            attempt,
        );
        let expected = operation.clone();
        let call = match operation {
            ThermostatOperation::SetMode(mode) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .thermostat_facet()
                            .call_set_mode(
                                a,
                                handle,
                                wire,
                                wire_thermostat::SetModeRequest {
                                    mode: wire_mode(mode),
                                },
                            )
                            .await
                    }),
                )
                .await
            }
            ThermostatOperation::SetSingleTarget(target) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .thermostat_facet()
                            .call_set_single_target(
                                a,
                                handle,
                                wire,
                                wire_thermostat::SetSingleTargetRequest {
                                    target: wire_thermostat::SingleTarget {
                                        temperature: wire_quantity(target),
                                    },
                                },
                            )
                            .await
                    }),
                )
                .await
            }
            ThermostatOperation::SetDualTarget { heat, cool } => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .thermostat_facet()
                            .call_set_dual_target(
                                a,
                                handle,
                                wire,
                                wire_thermostat::SetDualTargetRequest {
                                    target: wire_thermostat::DualTarget {
                                        heat: wire_quantity(heat),
                                        cool: wire_quantity(cool),
                                    },
                                },
                            )
                            .await
                    }),
                )
                .await
            }
            ThermostatOperation::SetFan(choice) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .thermostat_facet()
                            .call_set_fan(
                                a,
                                handle,
                                wire,
                                wire_thermostat::SetChoiceRequest { choice },
                            )
                            .await
                    }),
                )
                .await
            }
            ThermostatOperation::SetSwing(choice) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .thermostat_facet()
                            .call_set_swing(
                                a,
                                handle,
                                wire,
                                wire_thermostat::SetChoiceRequest { choice },
                            )
                            .await
                    }),
                )
                .await
            }
            ThermostatOperation::SetPreset(choice) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .thermostat_facet()
                            .call_set_preset(
                                a,
                                handle,
                                wire,
                                wire_thermostat::SetChoiceRequest { choice },
                            )
                            .await
                    }),
                )
                .await
            }
        };
        finish_thermostat(
            call,
            endpoint,
            self.provider.generation(),
            &expected,
            request_id,
            attempt,
            key,
        )
    }
    async fn covering_command(
        &mut self,
        endpoint: fabric::EndpointIdentity,
        operation: CoveringOperation,
        context: CallContext,
    ) -> (Result<CommandReceipt, ProviderHostError>, bool) {
        let Some(handle) = self
            .handles
            .get(&HandleKey {
                kind: FacetKind::WindowCovering,
                facet: endpoint.facet().as_str().to_owned(),
            })
            .copied()
        else {
            return (Err(endpoint_removed()), false);
        };
        if let Err(e) = set_store_deadline(&mut self.store, context.deadline) {
            return (Err(e), false);
        }
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let wire = wire_context(
            self.store.data().clock_started,
            context.deadline,
            &request_id,
            context.correlation_id,
            key.clone(),
            attempt,
        );
        let expected = operation.clone();
        let call = match operation {
            CoveringOperation::Open(axis) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .window_covering_facet()
                            .call_open(
                                a,
                                handle,
                                wire,
                                wire_covering::AxisRequest {
                                    axis: wire_axis(axis),
                                },
                            )
                            .await
                    }),
                )
                .await
            }
            CoveringOperation::Close(axis) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .window_covering_facet()
                            .call_close(
                                a,
                                handle,
                                wire,
                                wire_covering::AxisRequest {
                                    axis: wire_axis(axis),
                                },
                            )
                            .await
                    }),
                )
                .await
            }
            CoveringOperation::Stop(axis) => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .window_covering_facet()
                            .call_stop(
                                a,
                                handle,
                                wire,
                                wire_covering::AxisRequest {
                                    axis: wire_axis(axis),
                                },
                            )
                            .await
                    }),
                )
                .await
            }
            CoveringOperation::SetPosition { axis, position } => {
                tokio::time::timeout_at(
                    context.deadline,
                    self.store.run_concurrent(async |a| {
                        self.component
                            .litehouse_provider_provider()
                            .window_covering_facet()
                            .call_set_position(
                                a,
                                handle,
                                wire,
                                wire_covering::PositionRequest {
                                    axis: wire_axis(axis),
                                    position: wire_quantity(position),
                                },
                            )
                            .await
                    }),
                )
                .await
            }
        };
        finish_covering(
            call,
            endpoint,
            self.provider.generation(),
            &expected,
            request_id,
            attempt,
            key,
        )
    }
    async fn invoke_momentary(
        &mut self,
        endpoint: fabric::EndpointIdentity,
        context: CallContext,
    ) -> (Result<MomentaryActionReceipt, ProviderHostError>, bool) {
        let Some(handle) = self
            .handles
            .get(&HandleKey {
                kind: FacetKind::MomentaryAction,
                facet: endpoint.facet().as_str().to_owned(),
            })
            .copied()
        else {
            return (Err(endpoint_removed()), false);
        };
        if let Err(e) = set_store_deadline(&mut self.store, context.deadline) {
            return (Err(e), false);
        }
        let request_id = context.request_id.clone();
        let attempt = context.attempt;
        let key = context.idempotency_key.clone();
        let wire = wire_context(
            self.store.data().clock_started,
            context.deadline,
            &request_id,
            context.correlation_id,
            key.clone(),
            attempt,
        );
        let call = tokio::time::timeout_at(
            context.deadline,
            self.store.run_concurrent(async |a| {
                self.component
                    .litehouse_provider_provider()
                    .momentary_action_facet()
                    .call_invoke(a, handle, wire)
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
                    key,
                )),
                true,
            ),
            Ok(Err(_)) | Ok(Ok(Err(_))) => (
                Err(outcome_unknown(
                    fabric::UnknownOutcomeCause::ProviderFailed,
                    request_id,
                    attempt,
                    key,
                )),
                true,
            ),
            Ok(Ok(Ok(Err(e)))) => (Err(momentary_error(e)), false),
            Ok(Ok(Ok(Ok(r)))) => (
                host_momentary_receipt(
                    endpoint,
                    self.provider.generation(),
                    &request_id,
                    attempt,
                    key.as_deref(),
                    r,
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
    let features = boolean_event_features(&descriptor.common.events, &descriptor.events)?;
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
        features,
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

fn host_level_descriptor(
    i: &fabric::InstanceId,
    g: fabric::Generation,
    k: &str,
    n: u64,
    d: wire_level::LevelDescriptor,
) -> Result<HostFacetDescriptor, ProviderHostError> {
    let ops = d.operations.into_iter().map(|v| match v {
        wire_level::Operation::SetLevel => "set-level",
        wire_level::Operation::Step => "step",
        wire_level::Operation::Move => "move",
        wire_level::Operation::Stop => "stop",
        wire_level::Operation::SetLevelWithOnOff => "set-level-with-on-off",
        wire_level::Operation::StepWithOnOff => "step-with-on-off",
        wire_level::Operation::MoveWithOnOff => "move-with-on-off",
    });
    let fs = d.features.into_iter().map(|v| match v {
        wire_level::Feature::TargetLevel => "target-level",
        wire_level::Feature::NativeRange => "native-range",
        wire_level::Feature::Transition => "transition",
        wire_level::Feature::RelativeStep => "relative-step",
        wire_level::Feature::ContinuousMove => "continuous-move",
        wire_level::Feature::CoupledOnOff => "coupled-on-off",
    });
    host_typed_descriptor(
        i,
        g,
        k,
        n,
        d.common,
        ("level-control", FacetKind::LevelControl, ops, fs),
    )
}
fn host_color_descriptor(
    i: &fabric::InstanceId,
    g: fabric::Generation,
    k: &str,
    n: u64,
    d: wire_color::ColorDescriptor,
) -> Result<HostFacetDescriptor, ProviderHostError> {
    let ops = d.operations.into_iter().map(|v| match v {
        wire_color::Operation::SetColor => "set-color",
        wire_color::Operation::StepColor => "step-color",
        wire_color::Operation::MoveColor => "move-color",
        wire_color::Operation::Stop => "stop",
    });
    host_typed_descriptor(
        i,
        g,
        k,
        n,
        d.common,
        (
            "color-control",
            FacetKind::ColorControl,
            ops,
            std::iter::empty(),
        ),
    )
}
fn host_thermostat_descriptor(
    i: &fabric::InstanceId,
    g: fabric::Generation,
    k: &str,
    n: u64,
    d: wire_thermostat::ThermostatDescriptor,
) -> Result<HostFacetDescriptor, ProviderHostError> {
    let ops = d.operations.into_iter().map(|v| match v {
        wire_thermostat::Operation::SetMode => "set-mode",
        wire_thermostat::Operation::SetSingleTarget => "set-single-target",
        wire_thermostat::Operation::SetDualTarget => "set-dual-target",
        wire_thermostat::Operation::SetFan => "set-fan",
        wire_thermostat::Operation::SetSwing => "set-swing",
        wire_thermostat::Operation::SetPreset => "set-preset",
    });
    let fs = d.features.into_iter().map(|v| match v {
        wire_thermostat::Feature::FanChoice => "fan-choice",
        wire_thermostat::Feature::SwingChoice => "swing-choice",
        wire_thermostat::Feature::PresetChoice => "preset-choice",
    });
    host_typed_descriptor(
        i,
        g,
        k,
        n,
        d.common,
        ("thermostat", FacetKind::Thermostat, ops, fs),
    )
}
fn host_covering_descriptor(
    i: &fabric::InstanceId,
    g: fabric::Generation,
    k: &str,
    n: u64,
    d: wire_covering::CoveringDescriptor,
) -> Result<HostFacetDescriptor, ProviderHostError> {
    let ops = d.operations.into_iter().map(|v| match v {
        wire_covering::Operation::Open => "open",
        wire_covering::Operation::Close => "close",
        wire_covering::Operation::Stop => "stop",
        wire_covering::Operation::SetPosition => "set-position",
    });
    let fs = d.features.into_iter().map(|v| match v {
        wire_covering::Feature::Lift => "lift",
        wire_covering::Feature::Tilt => "tilt",
        wire_covering::Feature::TargetPosition => "target-position",
        wire_covering::Feature::Stop => "stop",
        wire_covering::Feature::SafetyStatus => "safety-status",
        wire_covering::Feature::FaultStatus => "fault-status",
    });
    host_typed_descriptor(
        i,
        g,
        k,
        n,
        d.common,
        ("window-covering", FacetKind::WindowCovering, ops, fs),
    )
}
fn host_momentary_descriptor(
    i: &fabric::InstanceId,
    g: fabric::Generation,
    k: &str,
    n: u64,
    d: wire_momentary::ActionDescriptor,
) -> Result<HostFacetDescriptor, ProviderHostError> {
    if d.action_id.is_empty() || d.action_id.len() > 128 {
        return Err(ProviderHostError::InvalidDescriptor);
    }
    host_typed_descriptor(
        i,
        g,
        k,
        n,
        d.common,
        (
            "momentary-action",
            FacetKind::MomentaryAction,
            ["invoke"],
            std::iter::empty(),
        ),
    )
}
fn host_typed_descriptor<O, F>(
    i: &fabric::InstanceId,
    g: fabric::Generation,
    k: &str,
    n: u64,
    c: wire_descriptor::FacetDescriptor,
    projection: (&str, FacetKind, O, F),
) -> Result<HostFacetDescriptor, ProviderHostError>
where
    O: IntoIterator<Item = &'static str>,
    F: IntoIterator<Item = &'static str>,
{
    let (contract, kind, operations, features) = projection;
    validate_common(&c, i, k, n, contract)?;
    let incarnation =
        fabric::Incarnation::new(n).map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let identity = fabric::EndpointIdentity::new(
        i.clone(),
        fabric::FacetKey::new(k).map_err(|_| ProviderHostError::InvalidDescriptor)?,
        fabric::ContractId::new("litehouse", contract, 1)
            .map_err(|_| ProviderHostError::InvalidDescriptor)?,
    );
    let mut ops = Vec::new();
    if kind != FacetKind::MomentaryAction {
        ops.push(operation("query")?)
    }
    ops.extend(
        operations
            .into_iter()
            .map(operation)
            .collect::<Result<Vec<_>, _>>()?,
    );
    let fs = features
        .into_iter()
        .map(fabric::FeatureId::new)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    let endpoint = fabric::EndpointDescriptor::new(
        identity,
        Version {
            major: 1,
            minor: 0,
            patch: 0,
        },
        g,
        incarnation,
        ops,
        fs,
    )
    .map_err(|_| ProviderHostError::InvalidDescriptor)?;
    Ok(HostFacetDescriptor {
        endpoint,
        kind,
        label: c.label,
        semantic_profile: c.semantic_profile,
        tags: host_tags(c.tags)?,
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

fn boolean_event_features(
    common_events: &[String],
    events: &[wire_boolean::EventKind],
) -> Result<Vec<fabric::FeatureId>, ProviderHostError> {
    let event_names = events
        .iter()
        .map(|event| match event {
            wire_boolean::EventKind::StateObservation => "state-observation",
            wire_boolean::EventKind::Occurrence => "occurrence",
        })
        .collect::<BTreeSet<_>>();
    if event_names.len() != events.len()
        || common_events.len() != event_names.len()
        || !common_events
            .iter()
            .all(|event| event_names.contains(event.as_str()))
    {
        return Err(ProviderHostError::InvalidDescriptor);
    }
    event_names
        .into_iter()
        .map(|event| {
            fabric::FeatureId::new(event).map_err(|_| ProviderHostError::InvalidDescriptor)
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

fn host_level_state(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    s: wire_level::State,
) -> Result<FacetState, ProviderHostError> {
    let v = LevelState {
        current: s.current.map(host_quantity).transpose()?,
        target: s.target.map(host_quantity).transpose()?,
        transition_remaining_milliseconds: s.transition_remaining_milliseconds,
    };
    host_state(
        e,
        g,
        s.meta,
        s.availability,
        s.quality,
        FacetValue::Level(v),
    )
}
fn host_color_state(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    s: wire_color::State,
) -> Result<FacetState, ProviderHostError> {
    let v = ColorState {
        current: s.current.map(host_color).transpose()?,
        target: s.target.map(host_color).transpose()?,
        active_model: s.active_model.map(host_color_model),
    };
    host_state(
        e,
        g,
        s.meta,
        s.availability,
        s.quality,
        FacetValue::Color(v),
    )
}
fn host_thermostat_state(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    s: wire_thermostat::State,
) -> Result<FacetState, ProviderHostError> {
    let targets = s
        .targets
        .map(|v| match v {
            wire_thermostat::Targets::Single(v) => {
                Ok(ThermostatTargets::Single(host_quantity(v.temperature)?))
            }
            wire_thermostat::Targets::HeatCool(v) => Ok(ThermostatTargets::HeatCool {
                heat: host_quantity(v.heat)?,
                cool: host_quantity(v.cool)?,
            }),
        })
        .transpose()?;
    let v = ThermostatState {
        current_mode: s.current_mode.map(host_mode),
        target_mode: s.target_mode.map(host_mode),
        current_action: s.current_action.map(|v| match v {
            wire_thermostat::Action::Idle => ThermostatAction::Idle,
            wire_thermostat::Action::Heating => ThermostatAction::Heating,
            wire_thermostat::Action::Cooling => ThermostatAction::Cooling,
            wire_thermostat::Action::Drying => ThermostatAction::Drying,
            wire_thermostat::Action::Fan => ThermostatAction::Fan,
            wire_thermostat::Action::Defrosting => ThermostatAction::Defrosting,
        }),
        running: s.running,
        targets,
        fan: s.fan,
        swing: s.swing,
        preset: s.preset,
    };
    host_state(
        e,
        g,
        s.meta,
        s.availability,
        s.quality,
        FacetValue::Thermostat(v),
    )
}
fn host_covering_state(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    s: wire_covering::State,
) -> Result<FacetState, ProviderHostError> {
    let axes = s
        .axes
        .into_iter()
        .map(|v| {
            Ok(CoveringAxisState {
                axis: host_axis(v.axis),
                current: v.current.map(host_quantity).transpose()?,
                target: v.target.map(host_quantity).transpose()?,
                movement: match v.movement {
                    wire_covering::Movement::Opening => CoveringMovement::Opening,
                    wire_covering::Movement::Closing => CoveringMovement::Closing,
                    wire_covering::Movement::Stopped => CoveringMovement::Stopped,
                    wire_covering::Movement::Idle => CoveringMovement::Idle,
                    wire_covering::Movement::Unknown => CoveringMovement::Unknown,
                },
            })
        })
        .collect::<Result<Vec<_>, ProviderHostError>>()?;
    let v = WindowCoveringState {
        axes,
        safety_status: s.safety_status,
        fault_status: s.fault_status,
    };
    host_state(
        e,
        g,
        s.meta,
        s.availability,
        s.quality,
        FacetValue::WindowCovering(v),
    )
}
fn host_quantity(v: wire_quantity::Quantity) -> Result<Quantity, ProviderHostError> {
    if v.profile.is_empty()
        || v.profile.len() > 128
        || v.magnitude.is_empty()
        || v.magnitude.len() > 128
        || v.unit.is_empty()
        || v.unit.len() > 64
    {
        return Err(ProviderHostError::InvalidDescriptor);
    }
    Ok(Quantity {
        profile: v.profile,
        magnitude: v.magnitude,
        unit: v.unit,
    })
}
fn host_color(v: wire_color::ColorValue) -> Result<ColorValue, ProviderHostError> {
    let valid = |v: &str| !v.is_empty() && v.len() <= 128;
    Ok(match v {
        wire_color::ColorValue::HueSaturation(v)
            if valid(&v.hue_degrees) && valid(&v.saturation_ratio) =>
        {
            ColorValue::HueSaturation {
                hue_degrees: v.hue_degrees,
                saturation_ratio: v.saturation_ratio,
            }
        }
        wire_color::ColorValue::CieXy(v) if valid(&v.x) && valid(&v.y) => {
            ColorValue::CieXy { x: v.x, y: v.y }
        }
        wire_color::ColorValue::ColorTemperature(v) if valid(&v.kelvin) => {
            ColorValue::ColorTemperature { kelvin: v.kelvin }
        }
        wire_color::ColorValue::CalibratedRgb(v)
            if valid(&v.channels.red)
                && valid(&v.channels.green)
                && valid(&v.channels.blue)
                && valid(&v.color_space) =>
        {
            ColorValue::CalibratedRgb {
                red: v.channels.red,
                green: v.channels.green,
                blue: v.channels.blue,
                color_space: v.color_space,
            }
        }
        wire_color::ColorValue::UncalibratedRgb(v)
            if valid(&v.red) && valid(&v.green) && valid(&v.blue) =>
        {
            ColorValue::UncalibratedRgb {
                red: v.red,
                green: v.green,
                blue: v.blue,
            }
        }
        wire_color::ColorValue::NativeWhiteChannels(v) => ColorValue::NativeWhiteChannels {
            white: v.white,
            cold_white: v.cold_white,
            warm_white: v.warm_white,
        },
        _ => return Err(ProviderHostError::InvalidDescriptor),
    })
}
fn host_color_model(v: wire_color::ColorModel) -> ColorModel {
    match v {
        wire_color::ColorModel::HueSaturation => ColorModel::HueSaturation,
        wire_color::ColorModel::CieXy => ColorModel::CieXy,
        wire_color::ColorModel::ColorTemperature => ColorModel::ColorTemperature,
        wire_color::ColorModel::CalibratedRgb => ColorModel::CalibratedRgb,
        wire_color::ColorModel::UncalibratedRgb => ColorModel::UncalibratedRgb,
        wire_color::ColorModel::NativeWhiteChannels => ColorModel::NativeWhiteChannels,
    }
}
fn host_mode(v: wire_thermostat::Mode) -> ThermostatMode {
    match v {
        wire_thermostat::Mode::Known(v) => ThermostatMode::Known(match v {
            wire_thermostat::KnownMode::Off => KnownThermostatMode::Off,
            wire_thermostat::KnownMode::Auto => KnownThermostatMode::Auto,
            wire_thermostat::KnownMode::Heat => KnownThermostatMode::Heat,
            wire_thermostat::KnownMode::Cool => KnownThermostatMode::Cool,
            wire_thermostat::KnownMode::Dry => KnownThermostatMode::Dry,
            wire_thermostat::KnownMode::FanOnly => KnownThermostatMode::FanOnly,
        }),
        wire_thermostat::Mode::ProviderQualified(v) => ThermostatMode::ProviderQualified(v),
    }
}
fn host_axis(v: wire_covering::Axis) -> CoveringAxis {
    match v {
        wire_covering::Axis::Lift => CoveringAxis::Lift,
        wire_covering::Axis::Tilt => CoveringAxis::Tilt,
    }
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

fn wire_quantity(v: Quantity) -> wire_quantity::Quantity {
    wire_quantity::Quantity {
        profile: v.profile,
        magnitude: v.magnitude,
        unit: v.unit,
    }
}
fn wire_direction(v: StepDirection) -> wire_level::StepDirection {
    match v {
        StepDirection::Increase => wire_level::StepDirection::Increase,
        StepDirection::Decrease => wire_level::StepDirection::Decrease,
    }
}
fn wire_transition(v: Option<Transition>) -> Option<wire_level::Transition> {
    v.map(|v| wire_level::Transition {
        duration_milliseconds: v.duration_milliseconds,
    })
}
fn wire_level_set(v: LevelSetRequest) -> wire_level::SetRequest {
    wire_level::SetRequest {
        level: wire_quantity(v.level),
        transition: wire_transition(v.transition),
    }
}
fn wire_level_step(v: LevelStepRequest) -> wire_level::StepRequest {
    wire_level::StepRequest {
        direction: wire_direction(v.direction),
        amount: wire_quantity(v.amount),
        transition: wire_transition(v.transition),
    }
}
fn wire_level_move(v: LevelMoveRequest) -> wire_level::MoveRequest {
    wire_level::MoveRequest {
        direction: wire_direction(v.direction),
        rate_per_second: wire_quantity(v.rate_per_second),
    }
}
fn wire_coupled_level(v: CoupledLevelRequest) -> wire_level::CoupledRequest {
    wire_level::CoupledRequest {
        level: wire_quantity(v.level),
        on_when_nonzero: v.on_when_nonzero,
        transition: wire_transition(v.transition),
    }
}
fn wire_coupled_step(v: CoupledLevelStepRequest) -> wire_level::CoupledStepRequest {
    wire_level::CoupledStepRequest {
        direction: wire_direction(v.direction),
        amount: wire_quantity(v.amount),
        on_when_nonzero: v.on_when_nonzero,
        transition: wire_transition(v.transition),
    }
}
fn wire_coupled_move(v: CoupledLevelMoveRequest) -> wire_level::CoupledMoveRequest {
    wire_level::CoupledMoveRequest {
        direction: wire_direction(v.direction),
        rate_per_second: wire_quantity(v.rate_per_second),
        on_when_nonzero: v.on_when_nonzero,
    }
}
fn wire_color(v: ColorValue) -> wire_color::ColorValue {
    match v {
        ColorValue::HueSaturation {
            hue_degrees,
            saturation_ratio,
        } => wire_color::ColorValue::HueSaturation(wire_color::HueSaturation {
            hue_degrees,
            saturation_ratio,
        }),
        ColorValue::CieXy { x, y } => wire_color::ColorValue::CieXy(wire_color::CieXy { x, y }),
        ColorValue::ColorTemperature { kelvin } => {
            wire_color::ColorValue::ColorTemperature(wire_color::ColorTemperature { kelvin })
        }
        ColorValue::CalibratedRgb {
            red,
            green,
            blue,
            color_space,
        } => wire_color::ColorValue::CalibratedRgb(wire_color::CalibratedRgb {
            channels: wire_color::RgbChannels { red, green, blue },
            color_space,
        }),
        ColorValue::UncalibratedRgb { red, green, blue } => {
            wire_color::ColorValue::UncalibratedRgb(wire_color::RgbChannels { red, green, blue })
        }
        ColorValue::NativeWhiteChannels {
            white,
            cold_white,
            warm_white,
        } => wire_color::ColorValue::NativeWhiteChannels(wire_color::WhiteChannels {
            white,
            cold_white,
            warm_white,
        }),
    }
}
fn wire_color_transition(v: Option<Transition>) -> Option<wire_color::Transition> {
    v.map(|v| wire_color::Transition {
        duration_milliseconds: v.duration_milliseconds,
    })
}
fn wire_color_set(v: ColorSetRequest) -> wire_color::SetRequest {
    wire_color::SetRequest {
        color: wire_color(v.color),
        transition: wire_color_transition(v.transition),
    }
}
fn wire_color_step(v: ColorStepRequest) -> wire_color::StepRequest {
    wire_color::StepRequest {
        delta: wire_color(v.delta),
        transition: wire_color_transition(v.transition),
    }
}
fn wire_color_move(v: ColorMoveRequest) -> wire_color::MoveRequest {
    wire_color::MoveRequest {
        rate_per_second: wire_color(v.rate_per_second),
    }
}
fn wire_mode(v: ThermostatMode) -> wire_thermostat::Mode {
    match v {
        ThermostatMode::Known(v) => wire_thermostat::Mode::Known(match v {
            KnownThermostatMode::Off => wire_thermostat::KnownMode::Off,
            KnownThermostatMode::Auto => wire_thermostat::KnownMode::Auto,
            KnownThermostatMode::Heat => wire_thermostat::KnownMode::Heat,
            KnownThermostatMode::Cool => wire_thermostat::KnownMode::Cool,
            KnownThermostatMode::Dry => wire_thermostat::KnownMode::Dry,
            KnownThermostatMode::FanOnly => wire_thermostat::KnownMode::FanOnly,
        }),
        ThermostatMode::ProviderQualified(v) => wire_thermostat::Mode::ProviderQualified(v),
    }
}
fn wire_axis(v: CoveringAxis) -> wire_covering::Axis {
    match v {
        CoveringAxis::Lift => wire_covering::Axis::Lift,
        CoveringAxis::Tilt => wire_covering::Axis::Tilt,
    }
}

type LevelCall = Result<
    Result<
        Result<Result<wire_level::OperationReceipt, wire_level::Error>, wasmtime::Error>,
        wasmtime::Error,
    >,
    tokio::time::error::Elapsed,
>;
type ColorCall = Result<
    Result<
        Result<Result<wire_color::OperationReceipt, wire_color::Error>, wasmtime::Error>,
        wasmtime::Error,
    >,
    tokio::time::error::Elapsed,
>;
type ThermostatCall = Result<
    Result<
        Result<Result<wire_thermostat::OperationReceipt, wire_thermostat::Error>, wasmtime::Error>,
        wasmtime::Error,
    >,
    tokio::time::error::Elapsed,
>;
type CoveringCall = Result<
    Result<
        Result<Result<wire_covering::OperationReceipt, wire_covering::Error>, wasmtime::Error>,
        wasmtime::Error,
    >,
    tokio::time::error::Elapsed,
>;
fn finish_level(
    c: LevelCall,
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    x: &LevelOperation,
    id: String,
    a: u32,
    k: Option<String>,
) -> (Result<CommandReceipt, ProviderHostError>, bool) {
    match c {
        Err(_) => (
            Err(outcome_unknown(
                fabric::UnknownOutcomeCause::DeadlineExceeded,
                id,
                a,
                k,
            )),
            true,
        ),
        Ok(Err(_)) | Ok(Ok(Err(_))) => (
            Err(outcome_unknown(
                fabric::UnknownOutcomeCause::ProviderFailed,
                id,
                a,
                k,
            )),
            true,
        ),
        Ok(Ok(Ok(Err(v)))) => (Err(level_error(v)), false),
        Ok(Ok(Ok(Ok(v)))) => (host_level_receipt(e, g, x, &id, a, k.as_deref(), v), false),
    }
}
fn finish_color(
    c: ColorCall,
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    x: &ColorOperation,
    id: String,
    a: u32,
    k: Option<String>,
) -> (Result<CommandReceipt, ProviderHostError>, bool) {
    match c {
        Err(_) => (
            Err(outcome_unknown(
                fabric::UnknownOutcomeCause::DeadlineExceeded,
                id,
                a,
                k,
            )),
            true,
        ),
        Ok(Err(_)) | Ok(Ok(Err(_))) => (
            Err(outcome_unknown(
                fabric::UnknownOutcomeCause::ProviderFailed,
                id,
                a,
                k,
            )),
            true,
        ),
        Ok(Ok(Ok(Err(v)))) => (Err(color_error(v)), false),
        Ok(Ok(Ok(Ok(v)))) => (host_color_receipt(e, g, x, &id, a, k.as_deref(), v), false),
    }
}
fn finish_thermostat(
    c: ThermostatCall,
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    x: &ThermostatOperation,
    id: String,
    a: u32,
    k: Option<String>,
) -> (Result<CommandReceipt, ProviderHostError>, bool) {
    match c {
        Err(_) => (
            Err(outcome_unknown(
                fabric::UnknownOutcomeCause::DeadlineExceeded,
                id,
                a,
                k,
            )),
            true,
        ),
        Ok(Err(_)) | Ok(Ok(Err(_))) => (
            Err(outcome_unknown(
                fabric::UnknownOutcomeCause::ProviderFailed,
                id,
                a,
                k,
            )),
            true,
        ),
        Ok(Ok(Ok(Err(v)))) => (Err(thermostat_error(v)), false),
        Ok(Ok(Ok(Ok(v)))) => (
            host_thermostat_receipt(e, g, x, &id, a, k.as_deref(), v),
            false,
        ),
    }
}
fn finish_covering(
    c: CoveringCall,
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    x: &CoveringOperation,
    id: String,
    a: u32,
    k: Option<String>,
) -> (Result<CommandReceipt, ProviderHostError>, bool) {
    match c {
        Err(_) => (
            Err(outcome_unknown(
                fabric::UnknownOutcomeCause::DeadlineExceeded,
                id,
                a,
                k,
            )),
            true,
        ),
        Ok(Err(_)) | Ok(Ok(Err(_))) => (
            Err(outcome_unknown(
                fabric::UnknownOutcomeCause::ProviderFailed,
                id,
                a,
                k,
            )),
            true,
        ),
        Ok(Ok(Ok(Err(v)))) => (Err(covering_error(v)), false),
        Ok(Ok(Ok(Ok(v)))) => (
            host_covering_receipt(e, g, x, &id, a, k.as_deref(), v),
            false,
        ),
    }
}

fn host_level_receipt(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    x: &LevelOperation,
    id: &str,
    a: u32,
    k: Option<&str>,
    r: wire_level::OperationReceipt,
) -> Result<CommandReceipt, ProviderHostError> {
    let actual = match r.operation {
        wire_level::OperationIntent::SetLevel(v) => LevelOperation::SetLevel(LevelSetRequest {
            level: host_quantity(v.level)?,
            transition: v.transition.map(|v| Transition {
                duration_milliseconds: v.duration_milliseconds,
            }),
        }),
        wire_level::OperationIntent::Step(v) => LevelOperation::Step(LevelStepRequest {
            direction: match v.direction {
                wire_level::StepDirection::Increase => StepDirection::Increase,
                wire_level::StepDirection::Decrease => StepDirection::Decrease,
            },
            amount: host_quantity(v.amount)?,
            transition: v.transition.map(|v| Transition {
                duration_milliseconds: v.duration_milliseconds,
            }),
        }),
        wire_level::OperationIntent::Move(v) => LevelOperation::Move(LevelMoveRequest {
            direction: match v.direction {
                wire_level::StepDirection::Increase => StepDirection::Increase,
                wire_level::StepDirection::Decrease => StepDirection::Decrease,
            },
            rate_per_second: host_quantity(v.rate_per_second)?,
        }),
        wire_level::OperationIntent::Stop => LevelOperation::Stop,
        wire_level::OperationIntent::SetLevelWithOnOff(v) => {
            LevelOperation::SetLevelWithOnOff(CoupledLevelRequest {
                level: host_quantity(v.level)?,
                on_when_nonzero: v.on_when_nonzero,
                transition: v.transition.map(|v| Transition {
                    duration_milliseconds: v.duration_milliseconds,
                }),
            })
        }
        wire_level::OperationIntent::StepWithOnOff(v) => {
            LevelOperation::StepWithOnOff(CoupledLevelStepRequest {
                direction: match v.direction {
                    wire_level::StepDirection::Increase => StepDirection::Increase,
                    wire_level::StepDirection::Decrease => StepDirection::Decrease,
                },
                amount: host_quantity(v.amount)?,
                on_when_nonzero: v.on_when_nonzero,
                transition: v.transition.map(|v| Transition {
                    duration_milliseconds: v.duration_milliseconds,
                }),
            })
        }
        wire_level::OperationIntent::MoveWithOnOff(v) => {
            LevelOperation::MoveWithOnOff(CoupledLevelMoveRequest {
                direction: match v.direction {
                    wire_level::StepDirection::Increase => StepDirection::Increase,
                    wire_level::StepDirection::Decrease => StepDirection::Decrease,
                },
                rate_per_second: host_quantity(v.rate_per_second)?,
                on_when_nonzero: v.on_when_nonzero,
            })
        }
    };
    if &actual != x {
        return Err(contract_error());
    }
    host_command_receipt(e, g, id, a, k, r.receipt)
}
fn host_color_receipt(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    x: &ColorOperation,
    id: &str,
    a: u32,
    k: Option<&str>,
    r: wire_color::OperationReceipt,
) -> Result<CommandReceipt, ProviderHostError> {
    let actual = match r.operation {
        wire_color::OperationIntent::SetColor(v) => ColorOperation::SetColor(ColorSetRequest {
            color: host_color(v.color)?,
            transition: v.transition.map(|v| Transition {
                duration_milliseconds: v.duration_milliseconds,
            }),
        }),
        wire_color::OperationIntent::StepColor(v) => ColorOperation::StepColor(ColorStepRequest {
            delta: host_color(v.delta)?,
            transition: v.transition.map(|v| Transition {
                duration_milliseconds: v.duration_milliseconds,
            }),
        }),
        wire_color::OperationIntent::MoveColor(v) => ColorOperation::MoveColor(ColorMoveRequest {
            rate_per_second: host_color(v.rate_per_second)?,
        }),
        wire_color::OperationIntent::Stop => ColorOperation::Stop,
    };
    if &actual != x {
        return Err(contract_error());
    }
    host_command_receipt(e, g, id, a, k, r.receipt)
}
fn host_thermostat_receipt(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    x: &ThermostatOperation,
    id: &str,
    a: u32,
    k: Option<&str>,
    r: wire_thermostat::OperationReceipt,
) -> Result<CommandReceipt, ProviderHostError> {
    let actual = match r.operation {
        wire_thermostat::OperationIntent::SetMode(v) => {
            ThermostatOperation::SetMode(host_mode(v.mode))
        }
        wire_thermostat::OperationIntent::SetSingleTarget(v) => {
            ThermostatOperation::SetSingleTarget(host_quantity(v.target.temperature)?)
        }
        wire_thermostat::OperationIntent::SetDualTarget(v) => ThermostatOperation::SetDualTarget {
            heat: host_quantity(v.target.heat)?,
            cool: host_quantity(v.target.cool)?,
        },
        wire_thermostat::OperationIntent::SetFan(v) => ThermostatOperation::SetFan(v.choice),
        wire_thermostat::OperationIntent::SetSwing(v) => ThermostatOperation::SetSwing(v.choice),
        wire_thermostat::OperationIntent::SetPreset(v) => ThermostatOperation::SetPreset(v.choice),
    };
    if &actual != x {
        return Err(contract_error());
    }
    host_command_receipt(e, g, id, a, k, r.receipt)
}
fn host_covering_receipt(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    x: &CoveringOperation,
    id: &str,
    a: u32,
    k: Option<&str>,
    r: wire_covering::OperationReceipt,
) -> Result<CommandReceipt, ProviderHostError> {
    let actual = match r.operation {
        wire_covering::OperationIntent::Open(v) => CoveringOperation::Open(host_axis(v.axis)),
        wire_covering::OperationIntent::Close(v) => CoveringOperation::Close(host_axis(v.axis)),
        wire_covering::OperationIntent::Stop(v) => CoveringOperation::Stop(host_axis(v.axis)),
        wire_covering::OperationIntent::SetPosition(v) => CoveringOperation::SetPosition {
            axis: host_axis(v.axis),
            position: host_quantity(v.position)?,
        },
    };
    if &actual != x {
        return Err(contract_error());
    }
    host_command_receipt(e, g, id, a, k, r.receipt)
}
fn host_momentary_receipt(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    id: &str,
    a: u32,
    k: Option<&str>,
    r: wire_momentary::InvokeReceipt,
) -> Result<MomentaryActionReceipt, ProviderHostError> {
    if r.action_id.is_empty() || r.action_id.len() > 128 {
        return Err(contract_error());
    }
    Ok(MomentaryActionReceipt {
        action_id: r.action_id,
        receipt: host_command_receipt(e, g, id, a, k, r.receipt)?,
    })
}
fn host_command_receipt(
    e: fabric::EndpointIdentity,
    g: fabric::Generation,
    id: &str,
    a: u32,
    k: Option<&str>,
    r: wire_receipt::CommandReceipt,
) -> Result<CommandReceipt, ProviderHostError> {
    if r.request_id != id || r.attempt != a || r.idempotency_key.as_deref() != k {
        return Err(contract_error());
    }
    Ok(CommandReceipt {
        endpoint: e,
        request_id: r.request_id,
        attempt: r.attempt,
        provider_generation: g,
        confidence: match r.confidence {
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
        accepted_at_unix_milliseconds: r.accepted_at_unix_milliseconds,
        completed_at_unix_milliseconds: r.completed_at_unix_milliseconds,
        idempotency_key: r.idempotency_key,
        deduplicated: false,
    })
}
fn contract_error() -> ProviderHostError {
    service_error(fabric::CoreError::ContractError, fabric::RetryAdvice::Never)
}
fn endpoint_removed() -> ProviderHostError {
    service_error(
        fabric::CoreError::Binding(fabric::BindingError::EndpointRemoved),
        fabric::RetryAdvice::Rebind,
    )
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
fn level_error(error: wire_level::Error) -> ProviderHostError {
    service_from_wire(error.inner, error.retry)
}
fn color_error(error: wire_color::Error) -> ProviderHostError {
    service_from_wire(error.inner, error.retry)
}
fn thermostat_error(error: wire_thermostat::Error) -> ProviderHostError {
    service_from_wire(error.inner, error.retry)
}
fn covering_error(error: wire_covering::Error) -> ProviderHostError {
    service_from_wire(error.inner, error.retry)
}
fn momentary_error(error: wire_momentary::Error) -> ProviderHostError {
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

fn valid_event_token(value: &str) -> bool {
    value.len() <= 128
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphanumeric())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
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
    use super::{
        HttpOrigin, boolean_event_features, tcp_address_allowed,
        validate_boolean_occurrence_sequences, wire_boolean,
    };
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
    fn boolean_occurrence_capability_must_match_both_descriptor_views() {
        let common = vec!["state-observation".to_owned(), "occurrence".to_owned()];
        let features = boolean_event_features(
            &common,
            &[
                wire_boolean::EventKind::StateObservation,
                wire_boolean::EventKind::Occurrence,
            ],
        )
        .unwrap();
        assert!(
            features
                .iter()
                .any(|feature| feature.as_str() == "occurrence")
        );
        assert!(
            boolean_event_features(
                &["state-observation".to_owned()],
                &[
                    wire_boolean::EventKind::StateObservation,
                    wire_boolean::EventKind::Occurrence,
                ],
            )
            .is_err()
        );
    }

    #[test]
    fn boolean_occurrence_sequences_make_overflow_and_gaps_explicit() {
        assert_eq!(
            validate_boolean_occurrence_sequences(None, 4, &[5, 6, 7]).unwrap(),
            Some(7)
        );
        assert_eq!(
            validate_boolean_occurrence_sequences(Some(7), 0, &[8, 9]).unwrap(),
            Some(9)
        );
        assert!(validate_boolean_occurrence_sequences(Some(9), 1, &[]).is_err());
        assert!(validate_boolean_occurrence_sequences(Some(9), 0, &[11]).is_err());
        assert!(validate_boolean_occurrence_sequences(Some(9), 1, &[11, 13]).is_err());
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
