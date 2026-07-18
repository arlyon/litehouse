//! Tasmota HTTP provider for one explicitly configured private LAN node.
//!
//! All facets share one serialized session and cache. The only outbound
//! requests are four fixed reads and absolute indexed power writes.
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
        "../../crates/litehouse-contracts/packages/provider/1.0.0/wit",
        "wit/http.wit",
        "wit/world.wit",
    ],
    world: "litehouse:tasmota/component@1.0.0",
    generate_all,
});

use exports::litehouse::provider::provider;
use futures::lock::Mutex;
use litehouse::{
    boolean_state::types as boolean, descriptor::types as descriptor, errors::types as errors,
    measurement::types as measurement, notification_sink::types as notification,
    on_off::types as on_off, quantity::types as quantity, receipt::types as receipt,
    state::types as state,
};
use protocol::{
    Config, FixedCommand, MeasurementKind, MeasurementSample, ProtocolError, RequestSpec,
    StatusSnapshot, parse_measurements, parse_response, parse_status, parse_status2,
    parse_template, routing_digest, validate_power_ack,
};
use std::{
    collections::BTreeMap,
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
use zeroize::Zeroize;

const CONNECT_TIMEOUT_NS: u64 = 2_000_000_000;
const REQUEST_TIMEOUT_NS: u64 = 5_000_000_000;
static ERROR_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct TasmotaProvider;
struct Runner {
    instance_id: String,
    shared: Arc<SharedSession>,
}
struct OnOffFacet {
    instance_id: String,
    channel: u8,
    shared: Arc<SharedSession>,
}
struct MeasurementFacet {
    instance_id: String,
    key: String,
    incarnation: u64,
    shared: Arc<SharedSession>,
}
enum NeverBooleanFacet {}
enum NeverNotificationSinkFacet {}
struct SharedSession {
    inner: Mutex<Session>,
    power_incarnation: AtomicU64,
}
struct Session {
    config: Config,
    _firmware_digest: [u8; 32],
    template_digest: [u8; 32],
    cache: Cache,
    next_poll: Instant,
}
#[derive(Clone)]
struct PowerObservation {
    current: bool,
    label: String,
    revision: u64,
    received_at: u64,
}
#[derive(Clone)]
struct MeasurementObservation {
    sample: MeasurementSample,
    incarnation: u64,
    revision: u64,
    received_at: u64,
}
struct Cache {
    routing_digest: [u8; 32],
    power: Vec<PowerObservation>,
    measurements: BTreeMap<String, MeasurementObservation>,
    measurement_incarnations: BTreeMap<String, u64>,
    revision: u64,
    consecutive_failures: u8,
    failure_since: Option<u64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SessionError {
    Protocol(ProtocolError),
    Transport,
}

impl provider::Guest for TasmotaProvider {
    type Runner = Runner;
    type OnOffFacet = OnOffFacet;
    type BooleanFacet = NeverBooleanFacet;
    type MeasurementFacet = MeasurementFacet;
    type NotificationSinkFacet = NeverNotificationSinkFacet;
    async fn start(
        instance_id: String,
        mut config: String,
    ) -> Result<provider::Runner, provider::Error> {
        if instance_id.is_empty() || instance_id.len() > 128 {
            config.zeroize();
            return Err(provider_error(SessionError::Protocol(
                ProtocolError::InvalidConfiguration,
            )));
        }
        let parsed = Config::from_json(&config);
        config.zeroize();
        let config = parsed.map_err(|e| provider_error(SessionError::Protocol(e)))?;
        let session = Session::activate(config, &instance_id)
            .await
            .map_err(provider_error)?;
        Ok(provider::Runner::new(Runner {
            instance_id,
            shared: Arc::new(SharedSession {
                inner: Mutex::new(session),
                power_incarnation: AtomicU64::new(1),
            }),
        }))
    }
}
impl provider::GuestRunner for Runner {
    async fn on_off_facets(&self) -> Result<Vec<provider::OnOffFacet>, provider::Error> {
        let session = self.shared.inner.lock().await;
        Ok((1..=session.cache.power.len())
            .map(|channel| {
                provider::OnOffFacet::new(OnOffFacet {
                    instance_id: self.instance_id.clone(),
                    channel: channel as u8,
                    shared: Arc::clone(&self.shared),
                })
            })
            .collect())
    }
    async fn boolean_facets(&self) -> Result<Vec<provider::BooleanFacet>, provider::Error> {
        Ok(vec![])
    }
    async fn measurement_facets(&self) -> Result<Vec<provider::MeasurementFacet>, provider::Error> {
        let session = self.shared.inner.lock().await;
        Ok(session
            .cache
            .measurements
            .values()
            .map(|o| {
                provider::MeasurementFacet::new(MeasurementFacet {
                    instance_id: self.instance_id.clone(),
                    key: o.sample.key.clone(),
                    incarnation: o.incarnation,
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
}

impl provider::GuestOnOffFacet for OnOffFacet {
    fn key(&self) -> String {
        format!("power/{}", self.channel)
    }
    fn incarnation(&self) -> u64 {
        self.shared.power_incarnation.load(Ordering::Relaxed)
    }
    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<on_off::OnOffDescriptor, on_off::Error> {
        let session = self.shared.inner.lock().await;
        let o = session
            .cache
            .power
            .get(usize::from(self.channel) - 1)
            .ok_or_else(endpoint_removed_on_off)?;
        Ok(on_off::OnOffDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                key: &self.key(),
                incarnation: self.incarnation(),
                primary_group_key: &self.key(),
                label: &o.label,
                contract: "on-off",
                semantic_profile: Some("power-channel"),
                operations: &["query", "set"],
                events: &["state-observation"],
            }),
            features: vec![],
            operations: vec![on_off::Operation::Set],
            state_fields: vec![on_off::StateField::Current],
            events: vec![on_off::EventKind::StateObservation],
            operation_support: vec![on_off::OperationSupport {
                operation: on_off::Operation::Set,
                kind: receipt::OperationKind::AbsoluteConvergent,
                deduplication: receipt::DeduplicationGuarantee::None,
                default_deadline_milliseconds: 5_000,
                maximum_deadline_milliseconds: 5_000,
            }],
        })
    }
    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<on_off::State, on_off::Error> {
        let mut session = self.shared.inner.lock().await;
        session.poll_if_due(&self.shared.power_incarnation).await;
        let o = session
            .cache
            .power
            .get(usize::from(self.channel) - 1)
            .ok_or_else(endpoint_removed_on_off)?;
        Ok(on_off::State {
            meta: observation_meta(o.revision, o.received_at),
            availability: cache_availability(&session.cache),
            quality: Some(state::Quality::SourceReported),
            current: Some(o.current),
        })
    }
    async fn set(
        &self,
        context: receipt::RequestContext,
        request: on_off::SetRequest,
    ) -> Result<on_off::OperationReceipt, on_off::Error> {
        if request.timed.is_some() {
            return Err(unsupported_feature(on_off::Feature::Timed));
        }
        if request.effect.is_some() {
            return Err(unsupported_feature(on_off::Feature::Effect));
        }
        let session = self.shared.inner.lock().await;
        let count = session.cache.power.len();
        if usize::from(self.channel) > count {
            return Err(endpoint_removed_on_off());
        }
        let value = session
            .request(FixedCommand::SetPower {
                channel: self.channel,
                value: request.value,
            })
            .await
            .map_err(|e| command_error(e, &context))?;
        validate_power_ack(&value, self.channel, count)
            .map_err(|e| command_error(SessionError::Protocol(e), &context))?;
        Ok(on_off::OperationReceipt {
            operation: on_off::OperationIntent::Set(request),
            receipt: receipt::CommandReceipt {
                request_id: context.request_id,
                attempt: context.attempt,
                provider_generation: 0,
                confidence: receipt::ReceiptConfidence::ProtocolOrTargetAcknowledged,
                accepted_at_unix_milliseconds: now_ms(),
                completed_at_unix_milliseconds: None,
                idempotency_key: context.idempotency.map(|x| x.key),
            },
        })
    }
}
impl provider::GuestMeasurementFacet for MeasurementFacet {
    fn key(&self) -> String {
        self.key.clone()
    }
    fn incarnation(&self) -> u64 {
        self.incarnation
    }
    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<measurement::MeasurementDescriptor, measurement::Error> {
        let session = self.shared.inner.lock().await;
        let o = session
            .cache
            .measurements
            .get(&self.key)
            .filter(|o| o.incarnation == self.incarnation)
            .ok_or_else(endpoint_removed_measurement)?;
        Ok(measurement::MeasurementDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                key: &self.key,
                incarnation: self.incarnation,
                primary_group_key: &self.key,
                label: &o.sample.label,
                contract: "measurement",
                semantic_profile: Some(o.sample.profile),
                operations: &["query"],
                events: &["observation"],
            }),
            profile: o.sample.profile.into(),
            kind: match o.sample.kind {
                MeasurementKind::Instantaneous => measurement::MeasurementKind::Instantaneous,
                MeasurementKind::Total => measurement::MeasurementKind::Total,
            },
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
        })
    }
    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<measurement::State, measurement::Error> {
        let mut session = self.shared.inner.lock().await;
        session.poll_if_due(&self.shared.power_incarnation).await;
        let o = session
            .cache
            .measurements
            .get(&self.key)
            .filter(|o| o.incarnation == self.incarnation)
            .ok_or_else(endpoint_removed_measurement)?;
        Ok(measurement::State {
            meta: observation_meta(o.revision, o.received_at),
            availability: cache_availability(&session.cache),
            quality: Some(state::Quality::SourceReported),
            current: Some(quantity::Quantity {
                profile: o.sample.profile.into(),
                magnitude: o.sample.magnitude.clone(),
                unit: o.sample.unit.into(),
            }),
            native: Some(quantity::NativeRepresentation {
                magnitude: o.sample.magnitude.clone(),
                unit: o.sample.unit.into(),
                quantization: None,
            }),
        })
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
        let firmware = parse_status2(&request(&config, FixedCommand::Status2).await?)
            .map_err(SessionError::Protocol)?;
        let status = parse_status(&request(&config, FixedCommand::Status).await?)
            .map_err(SessionError::Protocol)?;
        let template = parse_template(&request(&config, FixedCommand::Template).await?)
            .map_err(SessionError::Protocol)?;
        let measurements = parse_measurements(&request(&config, FixedCommand::Status10).await?)
            .map_err(SessionError::Protocol)?;
        let route = routing_digest(&status.module, &template, status.power.len())
            .map_err(SessionError::Protocol)?;
        let mut cache = Cache {
            routing_digest: route,
            power: vec![],
            measurements: BTreeMap::new(),
            measurement_incarnations: BTreeMap::new(),
            revision: 0,
            consecutive_failures: 0,
            failure_since: None,
        };
        cache.apply_success(status, measurements, route, now_ms(), None);
        let delay = startup_poll_delay(instance_id, config.poll_seconds);
        Ok(Self {
            config,
            _firmware_digest: firmware,
            template_digest: template,
            cache,
            next_poll: Instant::now() + delay,
        })
    }
    async fn request(&self, c: FixedCommand) -> Result<serde_json::Value, SessionError> {
        request(&self.config, c).await
    }
    async fn poll_if_due(&mut self, power_incarnation: &AtomicU64) {
        if Instant::now() < self.next_poll {
            return;
        }
        self.next_poll = Instant::now() + Duration::from_secs(u64::from(self.config.poll_seconds));
        let result = async {
            let status = parse_status(&self.request(FixedCommand::Status).await?)
                .map_err(SessionError::Protocol)?;
            let measurements = parse_measurements(&self.request(FixedCommand::Status10).await?)
                .map_err(SessionError::Protocol)?;
            let route = routing_digest(&status.module, &self.template_digest, status.power.len())
                .map_err(SessionError::Protocol)?;
            Ok::<_, SessionError>((status, measurements, route))
        }
        .await;
        match result {
            Ok((s, m, r)) => self
                .cache
                .apply_success(s, m, r, now_ms(), Some(power_incarnation)),
            Err(_) => self.cache.mark_failure(now_ms()),
        }
    }
}
impl Cache {
    fn apply_success(
        &mut self,
        status: StatusSnapshot,
        samples: Vec<MeasurementSample>,
        route: [u8; 32],
        received: u64,
        power_incarnation: Option<&AtomicU64>,
    ) {
        if !self.power.is_empty()
            && route != self.routing_digest
            && let Some(i) = power_incarnation
        {
            i.fetch_add(1, Ordering::Relaxed);
        }
        self.routing_digest = route;
        self.revision = self.revision.saturating_add(1).max(1);
        self.power = status
            .power
            .into_iter()
            .enumerate()
            .map(|(i, current)| PowerObservation {
                current,
                label: status
                    .labels
                    .get(i)
                    .and_then(Clone::clone)
                    .unwrap_or_else(|| format!("Power {}", i + 1)),
                revision: self.revision,
                received_at: received,
            })
            .collect();
        let previous = std::mem::take(&mut self.measurements);
        self.measurements = samples
            .into_iter()
            .map(|sample| {
                let incarnation = previous.get(&sample.key).map_or_else(
                    || {
                        let next = self
                            .measurement_incarnations
                            .get(&sample.key)
                            .copied()
                            .unwrap_or(0)
                            .saturating_add(1);
                        self.measurement_incarnations
                            .insert(sample.key.clone(), next);
                        next
                    },
                    |o| o.incarnation,
                );
                (
                    sample.key.clone(),
                    MeasurementObservation {
                        sample,
                        incarnation,
                        revision: self.revision,
                        received_at: received,
                    },
                )
            })
            .collect();
        self.consecutive_failures = 0;
        self.failure_since = None;
    }
    fn mark_failure(&mut self, at: u64) {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        self.failure_since.get_or_insert(at);
    }
}

async fn request(
    config: &Config,
    command: FixedCommand,
) -> Result<serde_json::Value, SessionError> {
    let r = send_http(config.request(command)).await?;
    parse_response(r.status, &r.body).map_err(SessionError::Protocol)
}
struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}
async fn send_http(spec: RequestSpec) -> Result<HttpResponse, SessionError> {
    let headers = Fields::new();
    headers
        .append("Authorization", spec.authorization())
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
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
    let (tw, tr) = wit_future::new(|| Ok(None));
    let (req, request_result) = Request::new(headers, None, tr, Some(options));
    req.set_method(&Method::Get)
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    req.set_scheme(Some(&Scheme::Http))
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    req.set_authority(Some(&spec.authority))
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    req.set_path_with_query(Some(&spec.path_with_query))
        .map_err(|_| SessionError::Protocol(ProtocolError::Protocol))?;
    drop(tw);
    let response = client::send(req)
        .await
        .map_err(|_| SessionError::Transport)?;
    request_result.await.map_err(|_| SessionError::Transport)?;
    let status = response.get_status_code();
    let (bw, br) = wit_future::new(|| Ok(()));
    let (mut stream, trailers) = Response::consume_body(response, br);
    let mut body = Vec::new();
    while let Some(byte) = stream.next().await {
        if body.len() == protocol::MAX_RESPONSE_BYTES {
            return Err(SessionError::Protocol(ProtocolError::Protocol));
        }
        body.push(byte)
    }
    trailers.await.map_err(|_| SessionError::Transport)?;
    drop(bw);
    Ok(HttpResponse { status, body })
}

struct CommonDescriptor<'a> {
    instance_id: &'a str,
    key: &'a str,
    incarnation: u64,
    primary_group_key: &'a str,
    label: &'a str,
    contract: &'a str,
    semantic_profile: Option<&'a str>,
    operations: &'a [&'a str],
    events: &'a [&'a str],
}
fn common_descriptor(s: CommonDescriptor<'_>) -> descriptor::FacetDescriptor {
    descriptor::FacetDescriptor {
        provider_instance_id: s.instance_id.into(),
        facet_key: s.key.into(),
        incarnation: s.incarnation,
        generation: 0,
        contract: descriptor::PackageIdentity {
            namespace: "litehouse".into(),
            name: s.contract.into(),
            version: descriptor::Version {
                major: 1,
                minor: 0,
                patch: 0,
            },
        },
        primary_group_key: s.primary_group_key.into(),
        direction: descriptor::FacetDirection::Provided,
        label: Some(s.label.into()),
        semantic_role: if s.contract == "on-off" {
            "power-channel"
        } else {
            "measurement"
        }
        .into(),
        semantic_profile: s.semantic_profile.map(Into::into),
        features: vec![],
        operations: s.operations.iter().map(|x| (*x).into()).collect(),
        events: s.events.iter().map(|x| (*x).into()).collect(),
        tags: vec![],
    }
}
fn observation_meta(revision: u64, received: u64) -> state::ObservationMeta {
    state::ObservationMeta {
        generation: 0,
        revision,
        received_at: state::WallClockInstant {
            unix_milliseconds: received,
        },
        source_time: None,
    }
}
fn cache_availability(c: &Cache) -> state::Availability {
    match c.consecutive_failures {
        0 => state::Availability::Available,
        1 => state::Availability::Stale(state::StaleStatus {
            since: state::WallClockInstant {
                unix_milliseconds: c.failure_since.unwrap_or_default(),
            },
        }),
        _ => state::Availability::Unavailable(state::UnavailabilityReason::ProviderFailed),
    }
}
fn provider_error(e: SessionError) -> provider::Error {
    let (inner, retry, message) = core_error(e);
    provider::Error {
        inner,
        retry,
        message,
        error_id: next_error_id(),
    }
}
fn command_error(e: SessionError, c: &receipt::RequestContext) -> on_off::Error {
    if e == SessionError::Transport {
        return on_off::Error {
            inner: errors::CoreError::OutcomeUnknown(errors::OutcomeUnknownError {
                cause: errors::UnknownOutcomeCause::Disconnected,
                request_id: c.request_id.clone(),
                attempt: c.attempt,
                idempotency_key: c.idempotency.as_ref().map(|x| x.key.clone()),
            }),
            detail: None,
            retry: errors::RetryAdvice::Never,
            message: Some("outcome-unknown".into()),
            error_id: next_error_id(),
        };
    }
    let (inner, retry, message) = core_error(e);
    on_off::Error {
        inner,
        detail: None,
        retry,
        message,
        error_id: next_error_id(),
    }
}
fn core_error(e: SessionError) -> (errors::CoreError, errors::RetryAdvice, Option<String>) {
    match e {
        SessionError::Transport => (
            errors::CoreError::Availability(errors::AvailabilityError::Unavailable),
            errors::RetryAdvice::Reconnect,
            Some("provider-unavailable".into()),
        ),
        SessionError::Protocol(ProtocolError::InvalidConfiguration) => (
            errors::CoreError::ContractError,
            errors::RetryAdvice::Never,
            Some("invalid-configuration".into()),
        ),
        SessionError::Protocol(ProtocolError::AuthenticationFailed) => (
            errors::CoreError::Authorization(errors::AuthorizationError::Denied),
            errors::RetryAdvice::Never,
            Some("authentication-failed".into()),
        ),
        SessionError::Protocol(ProtocolError::RefererBlocked) => (
            errors::CoreError::Execution(errors::ExecutionError::ProtocolFailed),
            errors::RetryAdvice::Never,
            Some("http-api-referer-blocked".into()),
        ),
        SessionError::Protocol(ProtocolError::UpstreamHttpStatus(status)) => (
            errors::CoreError::Execution(errors::ExecutionError::ProtocolFailed),
            errors::RetryAdvice::RetryAfter(None),
            Some(format!("upstream-http-status-{status}")),
        ),
        SessionError::Protocol(ProtocolError::Protocol) => (
            errors::CoreError::Execution(errors::ExecutionError::ProtocolFailed),
            errors::RetryAdvice::Never,
            Some("provider-protocol".into()),
        ),
    }
}
fn unsupported_feature(f: on_off::Feature) -> on_off::Error {
    on_off::Error {
        inner: errors::CoreError::ContractError,
        detail: Some(on_off::ContractErrorDetail::UnsupportedFeature(f)),
        retry: errors::RetryAdvice::Never,
        message: None,
        error_id: next_error_id(),
    }
}
fn endpoint_removed_on_off() -> on_off::Error {
    on_off::Error {
        inner: errors::CoreError::Binding(errors::BindingError::EndpointRemoved),
        detail: None,
        retry: errors::RetryAdvice::Rebind,
        message: None,
        error_id: next_error_id(),
    }
}
fn endpoint_removed_measurement() -> measurement::Error {
    measurement::Error {
        inner: errors::CoreError::Binding(errors::BindingError::EndpointRemoved),
        detail: None,
        retry: errors::RetryAdvice::Rebind,
        message: None,
        error_id: next_error_id(),
    }
}
fn next_error_id() -> String {
    format!(
        "tasmota-{:016x}",
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
fn startup_poll_delay(instance: &str, poll: u16) -> Duration {
    let hash = instance.bytes().fold(0xcbf29ce484222325_u64, |h, b| {
        h.wrapping_mul(0x100000001b3) ^ u64::from(b)
    });
    let base = u64::from(poll) * 1000;
    Duration::from_millis(base - base / 10 + hash % (base / 5 + 1))
}

export!(TasmotaProvider);

#[cfg(test)]
mod tests {
    use super::*;
    fn cache() -> Cache {
        Cache {
            routing_digest: [0; 32],
            power: vec![],
            measurements: BTreeMap::new(),
            measurement_incarnations: BTreeMap::new(),
            revision: 0,
            consecutive_failures: 0,
            failure_since: None,
        }
    }
    #[test]
    fn stale_unavailable_recovery() {
        let mut c = cache();
        c.mark_failure(100);
        assert!(matches!(
            cache_availability(&c),
            state::Availability::Stale(_)
        ));
        c.mark_failure(200);
        assert!(matches!(
            cache_availability(&c),
            state::Availability::Unavailable(state::UnavailabilityReason::ProviderFailed)
        ));
        c.consecutive_failures = 0;
        c.failure_since = None;
        assert!(matches!(
            cache_availability(&c),
            state::Availability::Available
        ));
    }
    #[test]
    fn jitter_is_deterministic_and_bounded() {
        let d = startup_poll_delay("instance-a", 30).as_millis();
        assert_eq!(d, startup_poll_delay("instance-a", 30).as_millis());
        assert!((27_000..=33_000).contains(&d));
    }
    #[test]
    fn post_dispatch_failure_is_outcome_unknown_never_retry() {
        let c = receipt::RequestContext {
            request_id: "request-1".into(),
            correlation_id: None,
            deadline: receipt::Deadline {
                absolute_monotonic_nanoseconds: u64::MAX,
            },
            idempotency: None,
            attempt: 1,
        };
        let e = command_error(SessionError::Transport, &c);
        assert!(matches!(e.inner, errors::CoreError::OutcomeUnknown(_)));
        assert!(matches!(e.retry, errors::RetryAdvice::Never));
    }
    #[test]
    fn descriptor_has_no_toggle_or_timed_surface() {
        let d = common_descriptor(CommonDescriptor {
            instance_id: "node",
            key: "power/1",
            incarnation: 1,
            primary_group_key: "power/1",
            label: "Power 1",
            contract: "on-off",
            semantic_profile: Some("power-channel"),
            operations: &["query", "set"],
            events: &["state-observation"],
        });
        assert_eq!(d.operations, ["query", "set"]);
        assert!(!d.operations.iter().any(|x| x == "toggle"));
        assert!(d.features.is_empty());
    }
    #[test]
    fn errors_never_include_upstream_bodies() {
        let (_, _, m) = core_error(SessionError::Protocol(ProtocolError::UpstreamHttpStatus(
            503,
        )));
        assert_eq!(m.as_deref(), Some("upstream-http-status-503"));
        let (_, _, m) = core_error(SessionError::Protocol(ProtocolError::RefererBlocked));
        assert_eq!(m.as_deref(), Some("http-api-referer-blocked"));
    }
    #[test]
    fn routing_change_reincarnates_every_power_channel() {
        let mut cache = cache();
        let incarnation = AtomicU64::new(1);
        let status = || StatusSnapshot {
            module: serde_json::json!(1),
            power: vec![false, true],
            labels: vec![None, None],
        };
        cache.apply_success(status(), vec![], [1; 32], 1, Some(&incarnation));
        assert_eq!(incarnation.load(Ordering::Relaxed), 1);
        cache.apply_success(status(), vec![], [2; 32], 2, Some(&incarnation));
        assert_eq!(incarnation.load(Ordering::Relaxed), 2);
    }
    #[test]
    fn removed_then_reappearing_measurement_gets_new_incarnation() {
        let mut cache = cache();
        let status = || StatusSnapshot {
            module: serde_json::json!(1),
            power: vec![false],
            labels: vec![None],
        };
        let sample = || MeasurementSample {
            key: "measurement/ENERGY.Power".into(),
            label: "ENERGY.Power".into(),
            profile: "power",
            kind: MeasurementKind::Instantaneous,
            magnitude: "10".into(),
            unit: "W",
        };
        cache.apply_success(status(), vec![sample()], [1; 32], 1, None);
        assert_eq!(cache.measurements.values().next().unwrap().incarnation, 1);
        cache.apply_success(status(), vec![], [1; 32], 2, None);
        cache.apply_success(status(), vec![sample()], [1; 32], 3, None);
        assert_eq!(cache.measurements.values().next().unwrap().incarnation, 2);
    }
}
