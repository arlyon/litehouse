//! Open-Meteo current-conditions provider for one configured WGS84 coordinate.
//!
//! The component issues only one fixed HTTPS GET to the free non-commercial
//! endpoint, exposes four read-only measurement facets, and embeds mandatory
//! Open-Meteo/CC BY 4.0 attribution in every descriptor.
mod protocol;

wit_bindgen::generate!({
    path: [
        "../../crates/litehouse-contracts/packages/descriptor/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/errors/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/state/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/receipt/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/on-off/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/boolean-state/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/quantity/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/measurement/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/notification-sink/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/level-control/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/color-control/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/thermostat/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/window-covering/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/momentary-action/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/provider/1.0.0/wit",
        "wit/http.wit",
        "wit/world.wit",
    ],
    world: "litehouse:open-meteo/component@1.0.0",
    generate_all,
});

use exports::litehouse::provider::provider;
use futures::lock::Mutex;
use litehouse::{
    boolean_state::types as boolean, color_control::types as color,
    descriptor::types as descriptor, errors::types as errors, level_control::types as level,
    measurement::types as measurement, momentary_action::types as momentary,
    notification_sink::types as notification, on_off::types as on_off, quantity::types as quantity,
    receipt::types as receipt, state::types as state, thermostat::types as thermostat,
    window_covering::types as covering,
};
use protocol::{
    Config, ProtocolError, RateBudget, RequestSpec, WeatherField, WeatherSnapshot, parse_response,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use wasi::http::{
    client,
    types::{Fields, Method, Request, RequestOptions, Response, Scheme},
};

const CONNECT_TIMEOUT_NS: u64 = 2_000_000_000;
const REQUEST_TIMEOUT_NS: u64 = 8_000_000_000;
const SOURCE_TAG_NAMESPACE: &str = "source";
const SOURCE_TAG_NAME: &str = "open-meteo-best-match-model";
const ATTRIBUTION_TAG_NAMESPACE: &str = "attribution";
const ATTRIBUTION_TAG_NAME: &str = "open-meteo-cc-by-4.0";
const ATTRIBUTION_TEXT: &str = "Weather data by Open-Meteo.com (normalized by Litehouse)";
const OPEN_METEO_URL: &str = "https://open-meteo.com/";
const CC_BY_URL: &str = "https://creativecommons.org/licenses/by/4.0/";
static ERROR_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct OpenMeteoProvider;
struct Runner {
    instance_id: String,
    shared: Arc<Mutex<Session>>,
}
struct MeasurementFacet {
    instance_id: String,
    field: WeatherField,
    shared: Arc<Mutex<Session>>,
}
enum NeverOnOffFacet {}
enum NeverBooleanFacet {}
enum NeverNotificationSinkFacet {}
enum NeverLevelControlFacet {}
enum NeverColorControlFacet {}
enum NeverThermostatFacet {}
enum NeverWindowCoveringFacet {}
enum NeverMomentaryActionFacet {}

struct Session {
    config: Config,
    cache: Cache,
    budget: RateBudget,
    budget_started: Instant,
    next_poll: Instant,
}

struct Cache {
    snapshot: WeatherSnapshot,
    revision: u64,
    received_at: u64,
    consecutive_failures: u8,
    failure_since: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionError {
    Protocol(ProtocolError),
    Transport,
}

impl provider::Guest for OpenMeteoProvider {
    type Runner = Runner;
    type OnOffFacet = NeverOnOffFacet;
    type BooleanFacet = NeverBooleanFacet;
    type MeasurementFacet = MeasurementFacet;
    type NotificationSinkFacet = NeverNotificationSinkFacet;
    type LevelControlFacet = NeverLevelControlFacet;
    type ColorControlFacet = NeverColorControlFacet;
    type ThermostatFacet = NeverThermostatFacet;
    type WindowCoveringFacet = NeverWindowCoveringFacet;
    type MomentaryActionFacet = NeverMomentaryActionFacet;

    async fn start(
        instance_id: String,
        config: String,
    ) -> Result<provider::Runner, provider::Error> {
        if instance_id.is_empty() || instance_id.len() > 128 {
            return Err(provider_error(SessionError::Protocol(
                ProtocolError::InvalidConfiguration,
            )));
        }
        let config = Config::from_json(&config)
            .map_err(|error| provider_error(SessionError::Protocol(error)))?;
        let session = Session::activate(config, &instance_id)
            .await
            .map_err(provider_error)?;
        Ok(provider::Runner::new(Runner {
            instance_id,
            shared: Arc::new(Mutex::new(session)),
        }))
    }
}

impl provider::GuestRunner for Runner {
    async fn on_off_facets(&self) -> Result<Vec<provider::OnOffFacet>, provider::Error> {
        Ok(vec![])
    }

    async fn boolean_facets(&self) -> Result<Vec<provider::BooleanFacet>, provider::Error> {
        Ok(vec![])
    }

    async fn measurement_facets(&self) -> Result<Vec<provider::MeasurementFacet>, provider::Error> {
        Ok(WeatherField::ALL
            .into_iter()
            .map(|field| {
                provider::MeasurementFacet::new(MeasurementFacet {
                    instance_id: self.instance_id.clone(),
                    field,
                    shared: Arc::clone(&self.shared),
                })
            })
            .collect())
    }

    async fn notification_sink_facets(
        &self,
    ) -> Result<Vec<provider::NotificationSinkFacet>, provider::Error> {
        Ok(vec![])
    }
    async fn level_control_facets(
        &self,
    ) -> Result<Vec<provider::LevelControlFacet>, provider::Error> {
        Ok(vec![])
    }
    async fn color_control_facets(
        &self,
    ) -> Result<Vec<provider::ColorControlFacet>, provider::Error> {
        Ok(vec![])
    }
    async fn thermostat_facets(&self) -> Result<Vec<provider::ThermostatFacet>, provider::Error> {
        Ok(vec![])
    }
    async fn window_covering_facets(
        &self,
    ) -> Result<Vec<provider::WindowCoveringFacet>, provider::Error> {
        Ok(vec![])
    }
    async fn momentary_action_facets(
        &self,
    ) -> Result<Vec<provider::MomentaryActionFacet>, provider::Error> {
        Ok(vec![])
    }
}

impl provider::GuestLevelControlFacet for NeverLevelControlFacet {
    fn key(&self) -> String {
        match *self {}
    }
    fn incarnation(&self) -> u64 {
        match *self {}
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<level::LevelDescriptor, level::Error> {
        match *self {}
    }
    async fn query(&self, _: receipt::RequestContext) -> Result<level::State, level::Error> {
        match *self {}
    }
    async fn set_level(
        &self,
        _: receipt::RequestContext,
        _: level::SetRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        match *self {}
    }
    async fn step(
        &self,
        _: receipt::RequestContext,
        _: level::StepRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        match *self {}
    }
    async fn move_(
        &self,
        _: receipt::RequestContext,
        _: level::MoveRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        match *self {}
    }
    async fn stop(
        &self,
        _: receipt::RequestContext,
    ) -> Result<level::OperationReceipt, level::Error> {
        match *self {}
    }
    async fn set_level_with_on_off(
        &self,
        _: receipt::RequestContext,
        _: level::CoupledRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        match *self {}
    }
    async fn step_with_on_off(
        &self,
        _: receipt::RequestContext,
        _: level::CoupledStepRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        match *self {}
    }
    async fn move_with_on_off(
        &self,
        _: receipt::RequestContext,
        _: level::CoupledMoveRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        match *self {}
    }
}

impl provider::GuestColorControlFacet for NeverColorControlFacet {
    fn key(&self) -> String {
        match *self {}
    }
    fn incarnation(&self) -> u64 {
        match *self {}
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<color::ColorDescriptor, color::Error> {
        match *self {}
    }
    async fn query(&self, _: receipt::RequestContext) -> Result<color::State, color::Error> {
        match *self {}
    }
    async fn set_color(
        &self,
        _: receipt::RequestContext,
        _: color::SetRequest,
    ) -> Result<color::OperationReceipt, color::Error> {
        match *self {}
    }
    async fn step_color(
        &self,
        _: receipt::RequestContext,
        _: color::StepRequest,
    ) -> Result<color::OperationReceipt, color::Error> {
        match *self {}
    }
    async fn move_color(
        &self,
        _: receipt::RequestContext,
        _: color::MoveRequest,
    ) -> Result<color::OperationReceipt, color::Error> {
        match *self {}
    }
    async fn stop(
        &self,
        _: receipt::RequestContext,
    ) -> Result<color::OperationReceipt, color::Error> {
        match *self {}
    }
}

impl provider::GuestThermostatFacet for NeverThermostatFacet {
    fn key(&self) -> String {
        match *self {}
    }
    fn incarnation(&self) -> u64 {
        match *self {}
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<thermostat::ThermostatDescriptor, thermostat::Error> {
        match *self {}
    }
    async fn query(
        &self,
        _: receipt::RequestContext,
    ) -> Result<thermostat::State, thermostat::Error> {
        match *self {}
    }
    async fn set_mode(
        &self,
        _: receipt::RequestContext,
        _: thermostat::SetModeRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        match *self {}
    }
    async fn set_single_target(
        &self,
        _: receipt::RequestContext,
        _: thermostat::SetSingleTargetRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        match *self {}
    }
    async fn set_dual_target(
        &self,
        _: receipt::RequestContext,
        _: thermostat::SetDualTargetRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        match *self {}
    }
    async fn set_fan(
        &self,
        _: receipt::RequestContext,
        _: thermostat::SetChoiceRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        match *self {}
    }
    async fn set_swing(
        &self,
        _: receipt::RequestContext,
        _: thermostat::SetChoiceRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        match *self {}
    }
    async fn set_preset(
        &self,
        _: receipt::RequestContext,
        _: thermostat::SetChoiceRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        match *self {}
    }
}

impl provider::GuestWindowCoveringFacet for NeverWindowCoveringFacet {
    fn key(&self) -> String {
        match *self {}
    }
    fn incarnation(&self) -> u64 {
        match *self {}
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<covering::CoveringDescriptor, covering::Error> {
        match *self {}
    }
    async fn query(&self, _: receipt::RequestContext) -> Result<covering::State, covering::Error> {
        match *self {}
    }
    async fn open(
        &self,
        _: receipt::RequestContext,
        _: covering::AxisRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        match *self {}
    }
    async fn close(
        &self,
        _: receipt::RequestContext,
        _: covering::AxisRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        match *self {}
    }
    async fn stop(
        &self,
        _: receipt::RequestContext,
        _: covering::AxisRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        match *self {}
    }
    async fn set_position(
        &self,
        _: receipt::RequestContext,
        _: covering::PositionRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        match *self {}
    }
}

impl provider::GuestMomentaryActionFacet for NeverMomentaryActionFacet {
    fn key(&self) -> String {
        match *self {}
    }
    fn incarnation(&self) -> u64 {
        match *self {}
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<momentary::ActionDescriptor, momentary::Error> {
        match *self {}
    }
    async fn invoke(
        &self,
        _: receipt::RequestContext,
    ) -> Result<momentary::InvokeReceipt, momentary::Error> {
        match *self {}
    }
}

impl provider::GuestMeasurementFacet for MeasurementFacet {
    fn key(&self) -> String {
        self.field.key().into()
    }

    fn incarnation(&self) -> u64 {
        1
    }

    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<measurement::MeasurementDescriptor, measurement::Error> {
        let session = self.shared.lock().await;
        Ok(measurement_descriptor(
            &self.instance_id,
            self.field,
            session.config.label.as_deref(),
        ))
    }

    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<measurement::State, measurement::Error> {
        let mut session = self.shared.lock().await;
        session.poll_if_due().await;
        let cache = &session.cache;
        let magnitude = cache.snapshot.values[self.field.index()].clone();
        Ok(measurement::State {
            meta: state::ObservationMeta {
                generation: 0,
                revision: cache.revision,
                received_at: state::WallClockInstant {
                    unix_milliseconds: cache.received_at,
                },
                source_time: Some(state::SourceTime {
                    instant: state::WallClockInstant {
                        unix_milliseconds: cache.snapshot.source_time_ms,
                    },
                    provenance: state::SourceTimeProvenance::ProviderClock,
                }),
            },
            availability: cache_availability(cache),
            quality: Some(state::Quality::SourceReported),
            current: magnitude.as_ref().map(|magnitude| quantity::Quantity {
                profile: self.field.profile().into(),
                magnitude: magnitude.clone(),
                unit: self.field.unit().into(),
            }),
            native: magnitude.map(|magnitude| quantity::NativeRepresentation {
                magnitude,
                unit: self.field.api_unit().into(),
                quantization: None,
            }),
        })
    }
}

impl provider::GuestOnOffFacet for NeverOnOffFacet {
    fn key(&self) -> String {
        match *self {}
    }

    fn incarnation(&self) -> u64 {
        match *self {}
    }

    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<on_off::OnOffDescriptor, on_off::Error> {
        match *self {}
    }

    async fn query(&self, _: receipt::RequestContext) -> Result<on_off::State, on_off::Error> {
        match *self {}
    }

    async fn set(
        &self,
        _: receipt::RequestContext,
        _: on_off::SetRequest,
    ) -> Result<on_off::OperationReceipt, on_off::Error> {
        match *self {}
    }
}

impl provider::GuestBooleanFacet for NeverBooleanFacet {
    fn key(&self) -> String {
        match *self {}
    }

    fn incarnation(&self) -> u64 {
        match *self {}
    }

    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<boolean::BooleanDescriptor, boolean::Error> {
        match *self {}
    }

    async fn query(&self, _: receipt::RequestContext) -> Result<boolean::State, boolean::Error> {
        match *self {}
    }
}

impl provider::GuestNotificationSinkFacet for NeverNotificationSinkFacet {
    fn key(&self) -> String {
        match *self {}
    }

    fn incarnation(&self) -> u64 {
        match *self {}
    }

    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<notification::NotificationDescriptor, notification::Error> {
        match *self {}
    }

    async fn send(
        &self,
        _: receipt::RequestContext,
        _: notification::Notification,
    ) -> Result<notification::SendReceipt, notification::Error> {
        match *self {}
    }
}

impl Session {
    async fn activate(config: Config, instance_id: &str) -> Result<Self, SessionError> {
        let budget_started = Instant::now();
        let mut budget = RateBudget::new(0);
        if !budget.take(0) {
            return Err(SessionError::Protocol(ProtocolError::RateLimited));
        }
        let snapshot = request(config.request()).await?;
        let received_at = now_ms();
        let delay = Duration::from_secs(
            u64::from(config.poll_seconds) + startup_jitter_seconds(instance_id),
        );
        Ok(Self {
            config,
            cache: Cache {
                snapshot,
                revision: 1,
                received_at,
                consecutive_failures: 0,
                failure_since: None,
            },
            budget,
            budget_started,
            next_poll: Instant::now() + delay,
        })
    }

    async fn poll_if_due(&mut self) {
        if Instant::now() < self.next_poll {
            return;
        }
        self.next_poll = Instant::now() + Duration::from_secs(u64::from(self.config.poll_seconds));
        let elapsed = self.budget_started.elapsed().as_secs();
        if !self.budget.take(elapsed) {
            self.cache.mark_failure(now_ms());
            return;
        }
        match request(self.config.request()).await {
            Ok(snapshot) => self.cache.apply_success(snapshot, now_ms()),
            Err(_) => self.cache.mark_failure(now_ms()),
        }
    }
}

impl Cache {
    fn apply_success(&mut self, snapshot: WeatherSnapshot, received_at: u64) {
        self.snapshot = snapshot;
        self.revision = self.revision.saturating_add(1);
        self.received_at = received_at;
        self.consecutive_failures = 0;
        self.failure_since = None;
    }

    fn mark_failure(&mut self, at: u64) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.failure_since.get_or_insert(at);
    }
}

async fn request(spec: RequestSpec) -> Result<WeatherSnapshot, SessionError> {
    let response = send_http(spec).await?;
    parse_response(response.status, &response.body).map_err(SessionError::Protocol)
}

struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

async fn send_http(spec: RequestSpec) -> Result<HttpResponse, SessionError> {
    let options = RequestOptions::new();
    options
        .set_connect_timeout(Some(CONNECT_TIMEOUT_NS))
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    options
        .set_first_byte_timeout(Some(REQUEST_TIMEOUT_NS))
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    options
        .set_between_bytes_timeout(Some(REQUEST_TIMEOUT_NS))
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    let (trailers_writer, trailers_reader) = wit_future::new(|| Ok(None));
    let (request, request_result) =
        Request::new(Fields::new(), None, trailers_reader, Some(options));
    request
        .set_method(&Method::Get)
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    request
        .set_scheme(Some(&Scheme::Https))
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    request
        .set_authority(Some(spec.authority))
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    request
        .set_path_with_query(Some(&spec.path_with_query))
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    drop(trailers_writer);

    let response = client::send(request)
        .await
        .map_err(|_| SessionError::Transport)?;
    request_result.await.map_err(|_| SessionError::Transport)?;
    let status = response.get_status_code();
    let (body_writer, body_reader) = wit_future::new(|| Ok(()));
    let (mut stream, response_trailers) = Response::consume_body(response, body_reader);
    let mut body = Vec::new();
    while let Some(byte) = stream.next().await {
        if body.len() == protocol::MAX_RESPONSE_BYTES {
            return Err(SessionError::Protocol(ProtocolError::Protocol));
        }
        body.push(byte);
    }
    response_trailers
        .await
        .map_err(|_| SessionError::Transport)?;
    drop(body_writer);
    Ok(HttpResponse { status, body })
}

fn measurement_descriptor(
    instance_id: &str,
    field: WeatherField,
    configured_label: Option<&str>,
) -> measurement::MeasurementDescriptor {
    let label = facet_label(configured_label, field);
    measurement::MeasurementDescriptor {
        common: descriptor::FacetDescriptor {
            provider_instance_id: instance_id.into(),
            facet_key: field.key().into(),
            incarnation: 1,
            generation: 0,
            contract: descriptor::PackageIdentity {
                namespace: "litehouse".into(),
                name: "measurement".into(),
                version: descriptor::Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
            },
            primary_group_key: field.key().into(),
            direction: descriptor::FacetDirection::Provided,
            label: Some(label),
            semantic_role: "weather-model-current-condition".into(),
            semantic_profile: Some(field.semantic_name().into()),
            features: vec![],
            operations: vec!["query".into()],
            events: vec!["observation".into()],
            tags: required_tags(field),
        },
        profile: field.profile().into(),
        kind: measurement::MeasurementKind::Instantaneous,
        state_fields: vec![
            measurement::StateField::Current,
            measurement::StateField::NativeRepresentation,
        ],
        events: vec![measurement::EventKind::Observation],
        bounds: measurement::MeasurementBounds {
            minimum: None,
            maximum: None,
            resolution: None,
            step: None,
            tolerance: None,
        },
    }
}

fn facet_label(configured: Option<&str>, field: WeatherField) -> String {
    configured.map_or_else(
        || field.label().into(),
        |configured| {
            let combined = format!("{configured}: {}", field.label());
            if combined.len() <= 256 {
                combined
            } else {
                field.label().into()
            }
        },
    )
}

fn required_tags(field: WeatherField) -> Vec<descriptor::SemanticTag> {
    vec![
        descriptor::SemanticTag {
            namespace: "weather".into(),
            name: field.semantic_name().into(),
            value: None,
        },
        descriptor::SemanticTag {
            namespace: SOURCE_TAG_NAMESPACE.into(),
            name: SOURCE_TAG_NAME.into(),
            value: None,
        },
        descriptor::SemanticTag {
            namespace: ATTRIBUTION_TAG_NAMESPACE.into(),
            name: ATTRIBUTION_TAG_NAME.into(),
            value: Some(format!(
                "{ATTRIBUTION_TEXT} | {OPEN_METEO_URL} | {CC_BY_URL}"
            )),
        },
    ]
}

fn cache_availability(cache: &Cache) -> state::Availability {
    match cache.consecutive_failures {
        0 => state::Availability::Available,
        1 => state::Availability::Stale(state::StaleStatus {
            since: state::WallClockInstant {
                unix_milliseconds: cache.failure_since.unwrap_or_default(),
            },
        }),
        _ => state::Availability::Unavailable(state::UnavailabilityReason::ProviderFailed),
    }
}

fn provider_error(error: SessionError) -> provider::Error {
    let (inner, retry, message) = core_error(error);
    provider::Error {
        inner,
        retry,
        message: Some(message.into()),
        error_id: next_error_id(),
    }
}

fn core_error(error: SessionError) -> (errors::CoreError, errors::RetryAdvice, &'static str) {
    match error {
        SessionError::Transport | SessionError::Protocol(ProtocolError::UpstreamUnavailable) => (
            errors::CoreError::Availability(errors::AvailabilityError::Unavailable),
            errors::RetryAdvice::Reconnect,
            "provider-unavailable",
        ),
        SessionError::Protocol(ProtocolError::InvalidConfiguration) => (
            errors::CoreError::ContractError,
            errors::RetryAdvice::Never,
            "invalid-configuration",
        ),
        SessionError::Protocol(ProtocolError::RequestRejected) => (
            errors::CoreError::Execution(errors::ExecutionError::ProtocolFailed),
            errors::RetryAdvice::Never,
            "provider-request-rejected",
        ),
        SessionError::Protocol(ProtocolError::RateLimited) => (
            errors::CoreError::Capacity(errors::CapacityError::Overloaded(errors::Overloaded {
                retry_after_milliseconds: None,
            })),
            errors::RetryAdvice::RetryAfter(None),
            "rate-limited",
        ),
        SessionError::Protocol(ProtocolError::UnsupportedAccessTier) => (
            errors::CoreError::Authorization(errors::AuthorizationError::Denied),
            errors::RetryAdvice::Never,
            "unsupported-access-tier",
        ),
        SessionError::Protocol(ProtocolError::Protocol) => (
            errors::CoreError::Execution(errors::ExecutionError::ProtocolFailed),
            errors::RetryAdvice::Never,
            "provider-protocol",
        ),
    }
}

fn next_error_id() -> String {
    format!(
        "open-meteo-{:016x}",
        ERROR_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn startup_jitter_seconds(instance_id: &str) -> u64 {
    instance_id
        .bytes()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            hash.wrapping_mul(0x100000001b3) ^ u64::from(byte)
        })
        % 61
}

export!(OpenMeteoProvider);

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> WeatherSnapshot {
        WeatherSnapshot {
            grid_latitude: 52.52,
            grid_longitude: 13.42,
            source_time_ms: 1_710_000_000_000,
            values: std::array::from_fn(|_| Some("1".into())),
        }
    }

    fn cache() -> Cache {
        Cache {
            snapshot: snapshot(),
            revision: 1,
            received_at: 100,
            consecutive_failures: 0,
            failure_since: None,
        }
    }

    #[test]
    fn cache_is_truthfully_stale_unavailable_and_recovers() {
        let mut cache = cache();
        cache.mark_failure(200);
        assert!(matches!(
            cache_availability(&cache),
            state::Availability::Stale(_)
        ));
        assert_eq!(cache.snapshot.values[0].as_deref(), Some("1"));
        cache.mark_failure(300);
        assert!(matches!(
            cache_availability(&cache),
            state::Availability::Unavailable(state::UnavailabilityReason::ProviderFailed)
        ));
        let mut recovered = snapshot();
        recovered.values[0] = None;
        cache.apply_success(recovered, 400);
        assert!(matches!(
            cache_availability(&cache),
            state::Availability::Available
        ));
        assert!(cache.snapshot.values[0].is_none());
        assert_eq!(cache.revision, 2);
    }

    #[test]
    fn all_facets_have_stable_units_source_and_non_removable_attribution() {
        for field in WeatherField::ALL {
            let descriptor = measurement_descriptor("weather-1", field, Some("Home"));
            assert_eq!(descriptor.common.facet_key, field.key());
            assert_eq!(
                descriptor.common.semantic_profile.as_deref(),
                Some(field.semantic_name())
            );
            assert_eq!(descriptor.profile, field.profile());
            assert_eq!(descriptor.common.operations, ["query"]);
            assert!(descriptor.common.features.is_empty());
            assert!(descriptor.common.tags.iter().any(|tag| {
                tag.namespace == SOURCE_TAG_NAMESPACE && tag.name == SOURCE_TAG_NAME
            }));
            assert!(descriptor.common.tags.iter().any(|tag| {
                tag.namespace == ATTRIBUTION_TAG_NAMESPACE
                    && tag.name == ATTRIBUTION_TAG_NAME
                    && tag.value.as_deref().is_some_and(|value| {
                        value.contains(ATTRIBUTION_TEXT)
                            && value.contains(OPEN_METEO_URL)
                            && value.contains(CC_BY_URL)
                    })
            }));
        }
        assert_eq!(
            ATTRIBUTION_TEXT,
            "Weather data by Open-Meteo.com (normalized by Litehouse)"
        );
        assert_eq!(OPEN_METEO_URL, "https://open-meteo.com/");
        assert_eq!(CC_BY_URL, "https://creativecommons.org/licenses/by/4.0/");
    }

    #[test]
    fn startup_jitter_is_deterministic_and_bounded() {
        let jitter = startup_jitter_seconds("weather-1");
        assert_eq!(jitter, startup_jitter_seconds("weather-1"));
        assert!(jitter <= 60);
    }

    #[test]
    fn errors_are_closed_and_never_echo_upstream_content() {
        for (error, expected) in [
            (ProtocolError::RequestRejected, "provider-request-rejected"),
            (ProtocolError::RateLimited, "rate-limited"),
            (
                ProtocolError::UnsupportedAccessTier,
                "unsupported-access-tier",
            ),
            (ProtocolError::Protocol, "provider-protocol"),
        ] {
            let (_, _, message) = core_error(SessionError::Protocol(error));
            assert_eq!(message, expected);
        }
    }
}
