//! Bounded ntfy notification provider for one fixed origin and topic.
//!
//! The component exposes one notification sink. It sends only exact plain-text
//! publishes and reports server acceptance, never downstream delivery.
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
    world: "litehouse:ntfy/component@1.0.0",
    generate_all,
});

use exports::litehouse::provider::provider;
use litehouse::{
    boolean_state::types as boolean, color_control::types as color,
    descriptor::types as descriptor, errors::types as errors, level_control::types as level,
    measurement::types as measurement, momentary_action::types as momentary,
    notification_sink::types as notification, on_off::types as on_off, receipt::types as receipt,
    thermostat::types as thermostat, window_covering::types as covering,
};
use protocol::{Config, ProtocolError, RequestSpec};
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use wasi::http::{
    client,
    types::{Fields, Method, Request, RequestOptions, Response, Scheme},
};
use zeroize::Zeroize;

const CONNECT_TIMEOUT_NS: u64 = 2_000_000_000;
const REQUEST_TIMEOUT_NS: u64 = 8_000_000_000;
static ERROR_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct NtfyProvider;
struct Runner {
    instance_id: String,
    config: Arc<Config>,
}
struct NotificationSinkFacet {
    instance_id: String,
    config: Arc<Config>,
}
enum NeverOnOffFacet {}
enum NeverBooleanFacet {}
enum NeverMeasurementFacet {}
enum NeverLevelControlFacet {}
enum NeverColorControlFacet {}
enum NeverThermostatFacet {}
enum NeverWindowCoveringFacet {}
enum NeverMomentaryActionFacet {}

impl provider::Guest for NtfyProvider {
    type Runner = Runner;
    type OnOffFacet = NeverOnOffFacet;
    type BooleanFacet = NeverBooleanFacet;
    type MeasurementFacet = NeverMeasurementFacet;
    type NotificationSinkFacet = NotificationSinkFacet;
    type LevelControlFacet = NeverLevelControlFacet;
    type ColorControlFacet = NeverColorControlFacet;
    type ThermostatFacet = NeverThermostatFacet;
    type WindowCoveringFacet = NeverWindowCoveringFacet;
    type MomentaryActionFacet = NeverMomentaryActionFacet;

    async fn start(
        instance_id: String,
        mut config: String,
    ) -> Result<provider::Runner, provider::Error> {
        if instance_id.is_empty() || instance_id.len() > 128 {
            config.zeroize();
            return Err(provider_error(ProtocolError::InvalidConfiguration));
        }
        let parsed = Config::from_json(&config);
        config.zeroize();
        let config = parsed.map_err(provider_error)?;
        Ok(provider::Runner::new(Runner {
            instance_id,
            config: Arc::new(config),
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
        Ok(vec![])
    }

    async fn notification_sink_facets(
        &self,
    ) -> Result<Vec<provider::NotificationSinkFacet>, provider::Error> {
        Ok(vec![provider::NotificationSinkFacet::new(
            NotificationSinkFacet {
                instance_id: self.instance_id.clone(),
                config: Arc::clone(&self.config),
            },
        )])
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

impl provider::GuestNotificationSinkFacet for NotificationSinkFacet {
    fn key(&self) -> String {
        "notification".into()
    }

    fn incarnation(&self) -> u64 {
        1
    }

    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<notification::NotificationDescriptor, notification::Error> {
        Ok(notification_descriptor(&self.instance_id))
    }

    async fn send(
        &self,
        context: receipt::RequestContext,
        value: notification::Notification,
    ) -> Result<notification::SendReceipt, notification::Error> {
        if value.expires_at_unix_milliseconds.is_some() {
            return Err(unsupported_field(notification::OptionalField::Expiry));
        }
        if value.body.len() > protocol::MAX_BODY_BYTES {
            return Err(content_error(
                notification::ContractErrorDetail::OversizedContent,
            ));
        }
        if let Some(title) = value.title.as_deref() {
            if title.len() > protocol::MAX_TITLE_BYTES
                || title.chars().count() > protocol::MAX_TITLE_SCALARS
            {
                return Err(content_error(
                    notification::ContractErrorDetail::OversizedContent,
                ));
            }
            if !protocol::valid_title(title) {
                return Err(content_error(
                    notification::ContractErrorDetail::InvalidContent,
                ));
            }
        }
        debug_assert!(protocol::valid_body(&value.body));
        let priority = value.priority.map(priority_header);
        let spec = self
            .config
            .request(value.body, value.title, priority)
            .map_err(|error| send_error(error, &context))?;
        let response = send_http(spec)
            .await
            .map_err(|error| send_error(error, &context))?;
        let provider_message_id = protocol::parse_response(response.status, &response.body)
            .map_err(|error| send_error(error, &context))?;
        Ok(notification::SendReceipt {
            receipt: receipt::CommandReceipt {
                request_id: context.request_id,
                attempt: context.attempt,
                provider_generation: 0,
                confidence: receipt::ReceiptConfidence::AcceptedForDelivery,
                accepted_at_unix_milliseconds: now_ms(),
                completed_at_unix_milliseconds: None,
                idempotency_key: context.idempotency.map(|value| value.key),
            },
            provider_message_id: Some(provider_message_id),
        })
    }
}

fn priority_header(priority: notification::Priority) -> &'static str {
    match priority {
        notification::Priority::Low => "low",
        notification::Priority::Normal => "default",
        notification::Priority::High => "high",
        notification::Priority::Urgent => "max",
    }
}

struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

async fn send_http(spec: RequestSpec) -> Result<HttpResponse, ProtocolError> {
    let headers = Fields::new();
    append_header(&headers, "Authorization", spec.authorization())?;
    append_header(
        &headers,
        "Content-Type",
        "text/plain; charset=utf-8".as_bytes(),
    )?;
    append_header(&headers, "Content-Length", spec.content_length())?;
    if let Some(title) = spec.title() {
        append_header(&headers, "Title", title)?;
    }
    if let Some(priority) = spec.priority() {
        append_header(&headers, "Priority", priority)?;
    }

    let options = RequestOptions::new();
    options
        .set_connect_timeout(Some(CONNECT_TIMEOUT_NS))
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    options
        .set_first_byte_timeout(Some(REQUEST_TIMEOUT_NS))
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    options
        .set_between_bytes_timeout(Some(REQUEST_TIMEOUT_NS))
        .map_err(|_| ProtocolError::OutcomeUnknown)?;

    let (mut body_writer, body_reader) = wit_stream::new();
    let (trailers_writer, trailers_reader) = wit_future::new(|| Ok(None));
    let (request, request_result) =
        Request::new(headers, Some(body_reader), trailers_reader, Some(options));
    request
        .set_method(&Method::Post)
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    request
        .set_scheme(Some(match spec.scheme {
            protocol::Scheme::Http => &Scheme::Http,
            protocol::Scheme::Https => &Scheme::Https,
        }))
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    request
        .set_authority(Some(&spec.authority))
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    request
        .set_path_with_query(Some(&spec.path_with_query))
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    drop(trailers_writer);

    wit_bindgen::spawn_local(async move {
        let remaining = body_writer.write_all(spec.body).await;
        debug_assert!(remaining.is_empty());
    });

    let response = client::send(request)
        .await
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    request_result
        .await
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    let status = response.get_status_code();
    let response_headers = response.get_headers();
    let response_header_entries = response_headers.copy_all();
    drop(response_headers);
    if !protocol::response_headers_within_bound(&response_header_entries) {
        return Err(ProtocolError::OutcomeUnknown);
    }

    let (body_result_writer, body_result_reader) = wit_future::new(|| Ok(()));
    let (mut stream, response_trailers) = Response::consume_body(response, body_result_reader);
    let mut body = Vec::new();
    while let Some(byte) = stream.next().await {
        if body.len() == protocol::MAX_RESPONSE_BYTES {
            return Err(ProtocolError::OutcomeUnknown);
        }
        body.push(byte);
    }
    response_trailers
        .await
        .map_err(|_| ProtocolError::OutcomeUnknown)?;
    drop(body_result_writer);
    Ok(HttpResponse { status, body })
}

fn append_header(headers: &Fields, name: &str, value: &[u8]) -> Result<(), ProtocolError> {
    headers
        .append(name, value)
        .map_err(|_| ProtocolError::OutcomeUnknown)
}

fn notification_descriptor(instance_id: &str) -> notification::NotificationDescriptor {
    notification::NotificationDescriptor {
        common: descriptor::FacetDescriptor {
            provider_instance_id: instance_id.into(),
            facet_key: "notification".into(),
            incarnation: 1,
            generation: 0,
            contract: descriptor::PackageIdentity {
                namespace: "litehouse".into(),
                name: "notification-sink".into(),
                version: descriptor::Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
            },
            primary_group_key: "notification".into(),
            direction: descriptor::FacetDirection::Provided,
            label: Some("ntfy notification sink".into()),
            semantic_role: "notification-sink".into(),
            semantic_profile: Some("ntfy-topic".into()),
            features: vec![],
            operations: vec!["send".into()],
            events: vec![],
            tags: vec![],
        },
        supported_fields: vec![
            notification::OptionalField::Title,
            notification::OptionalField::Priority,
        ],
        maximum_title_bytes: protocol::MAX_TITLE_BYTES as u32,
        maximum_body_bytes: protocol::MAX_BODY_BYTES as u32,
        maximum_receipt_confidence: receipt::ReceiptConfidence::AcceptedForDelivery,
        operation: notification::OperationSupport {
            kind: receipt::OperationKind::Momentary,
            deduplication: receipt::DeduplicationGuarantee::None,
            default_deadline_milliseconds: 8_000,
            maximum_deadline_milliseconds: 8_000,
        },
    }
}

fn provider_error(error: ProtocolError) -> provider::Error {
    let (inner, message) = match error {
        ProtocolError::InvalidConfiguration => {
            (errors::CoreError::ContractError, "invalid-configuration")
        }
        _ => (
            errors::CoreError::Execution(errors::ExecutionError::ProtocolFailed),
            "provider-protocol",
        ),
    };
    provider::Error {
        inner,
        retry: errors::RetryAdvice::Never,
        message: Some(message.into()),
        error_id: next_error_id(),
    }
}

fn send_error(error: ProtocolError, context: &receipt::RequestContext) -> notification::Error {
    let (inner, detail, message) = match error {
        ProtocolError::InvalidConfiguration => (
            errors::CoreError::ContractError,
            None,
            "invalid-configuration",
        ),
        ProtocolError::RequestRejected => (
            errors::CoreError::Execution(errors::ExecutionError::ProtocolFailed),
            Some(notification::ContractErrorDetail::ProviderRejected),
            "provider-rejected",
        ),
        ProtocolError::AuthorizationFailed => (
            errors::CoreError::Authorization(errors::AuthorizationError::Denied),
            Some(notification::ContractErrorDetail::ProviderRejected),
            "authorization-failed",
        ),
        ProtocolError::SizeRejected => (
            errors::CoreError::ContractError,
            Some(notification::ContractErrorDetail::OversizedContent),
            "provider-size-rejected",
        ),
        ProtocolError::RateLimited => (
            errors::CoreError::Capacity(errors::CapacityError::Overloaded(errors::Overloaded {
                retry_after_milliseconds: None,
            })),
            Some(notification::ContractErrorDetail::RateLimited),
            "rate-limited",
        ),
        ProtocolError::OutcomeUnknown => {
            return outcome_unknown(context);
        }
    };
    notification::Error {
        inner,
        detail,
        retry: errors::RetryAdvice::Never,
        message: Some(message.into()),
        error_id: next_error_id(),
    }
}

fn outcome_unknown(context: &receipt::RequestContext) -> notification::Error {
    notification::Error {
        inner: errors::CoreError::OutcomeUnknown(errors::OutcomeUnknownError {
            cause: errors::UnknownOutcomeCause::Disconnected,
            request_id: context.request_id.clone(),
            attempt: context.attempt,
            idempotency_key: context.idempotency.as_ref().map(|value| value.key.clone()),
        }),
        detail: None,
        retry: errors::RetryAdvice::Never,
        message: Some("outcome-unknown".into()),
        error_id: next_error_id(),
    }
}

fn unsupported_field(field: notification::OptionalField) -> notification::Error {
    notification::Error {
        inner: errors::CoreError::ContractError,
        detail: Some(notification::ContractErrorDetail::UnsupportedField(field)),
        retry: errors::RetryAdvice::Never,
        message: None,
        error_id: next_error_id(),
    }
}

fn content_error(detail: notification::ContractErrorDetail) -> notification::Error {
    notification::Error {
        inner: errors::CoreError::ContractError,
        detail: Some(detail),
        retry: errors::RetryAdvice::Never,
        message: None,
        error_id: next_error_id(),
    }
}

fn next_error_id() -> String {
    format!(
        "ntfy-{:016x}",
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

impl provider::GuestMeasurementFacet for NeverMeasurementFacet {
    fn key(&self) -> String {
        match *self {}
    }
    fn incarnation(&self) -> u64 {
        match *self {}
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<measurement::MeasurementDescriptor, measurement::Error> {
        match *self {}
    }
    async fn query(
        &self,
        _: receipt::RequestContext,
    ) -> Result<measurement::State, measurement::Error> {
        match *self {}
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

export!(NtfyProvider);

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> receipt::RequestContext {
        receipt::RequestContext {
            request_id: "request-1".into(),
            correlation_id: None,
            deadline: receipt::Deadline {
                absolute_monotonic_nanoseconds: u64::MAX,
            },
            idempotency: None,
            attempt: 1,
        }
    }

    #[test]
    fn descriptor_is_one_acceptance_only_non_deduplicating_sink() {
        let descriptor = notification_descriptor("alerts");
        assert_eq!(descriptor.common.facet_key, "notification");
        assert_eq!(descriptor.common.operations, ["send"]);
        assert!(descriptor.common.events.is_empty());
        assert_eq!(descriptor.supported_fields.len(), 2);
        assert!(
            descriptor
                .supported_fields
                .iter()
                .any(|field| matches!(field, notification::OptionalField::Title))
        );
        assert!(
            descriptor
                .supported_fields
                .iter()
                .any(|field| matches!(field, notification::OptionalField::Priority))
        );
        assert_eq!(descriptor.maximum_body_bytes, 4_096);
        assert_eq!(descriptor.maximum_title_bytes, 256);
        assert!(matches!(
            descriptor.maximum_receipt_confidence,
            receipt::ReceiptConfidence::AcceptedForDelivery
        ));
        assert!(matches!(
            descriptor.operation.kind,
            receipt::OperationKind::Momentary
        ));
        assert!(matches!(
            descriptor.operation.deduplication,
            receipt::DeduplicationGuarantee::None
        ));
        assert_eq!(descriptor.operation.maximum_deadline_milliseconds, 8_000);
    }

    #[test]
    fn priorities_map_only_to_ntfy_native_values() {
        for (priority, expected) in [
            (notification::Priority::Low, "low"),
            (notification::Priority::Normal, "default"),
            (notification::Priority::High, "high"),
            (notification::Priority::Urgent, "max"),
        ] {
            assert_eq!(priority_header(priority), expected);
            assert!(["min", "low", "default", "high", "max"].contains(&expected));
        }
    }

    #[test]
    fn every_ambiguous_failure_is_outcome_unknown_and_never_retry() {
        let error = send_error(ProtocolError::OutcomeUnknown, &context());
        assert!(matches!(error.inner, errors::CoreError::OutcomeUnknown(_)));
        assert!(matches!(error.retry, errors::RetryAdvice::Never));
        assert_eq!(error.message.as_deref(), Some("outcome-unknown"));
    }

    #[test]
    fn definite_send_failures_also_never_authorize_retry() {
        for error in [
            ProtocolError::RequestRejected,
            ProtocolError::AuthorizationFailed,
            ProtocolError::SizeRejected,
            ProtocolError::RateLimited,
        ] {
            assert!(matches!(
                send_error(error, &context()).retry,
                errors::RetryAdvice::Never
            ));
        }
    }

    #[test]
    fn errors_are_closed_and_cannot_echo_secrets_or_content() {
        let secret = "do-not-leak-token";
        for error in [
            ProtocolError::InvalidConfiguration,
            ProtocolError::RequestRejected,
            ProtocolError::AuthorizationFailed,
            ProtocolError::SizeRejected,
            ProtocolError::RateLimited,
            ProtocolError::OutcomeUnknown,
        ] {
            let error = send_error(error, &context());
            assert!(
                !error
                    .message
                    .as_deref()
                    .unwrap_or_default()
                    .contains(secret)
            );
            assert!(!error.error_id.contains(secret));
        }
    }
}
