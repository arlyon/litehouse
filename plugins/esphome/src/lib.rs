//! ESPHome Native API provider component.
//!
//! One configured instance owns one encrypted session and cache shared by all
//! switch and binary-sensor facet resources. Commands are absolute and a
//! successful socket write claims accepted-for-delivery only.

mod session;

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
    ],
    world: "litehouse:provider/component@1.0.0",
    generate_all,
});

use exports::litehouse::provider::provider;
use litehouse::{
    boolean_state::types as boolean, descriptor::types as descriptor, errors::types as errors,
    measurement::types as measurement, notification_sink::types as notification,
    on_off::types as on_off, receipt::types as receipt, state::types as state,
};
use session::{BooleanProfile, Locator, NodeSession, SessionError, now_unix_milliseconds};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::{
    runtime::{Builder as RuntimeBuilder, Runtime},
    sync::Mutex,
};

static ERROR_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct EspHomeProvider;

struct Runner {
    instance_id: String,
    runtime: Arc<Runtime>,
    session: Arc<Mutex<NodeSession>>,
}

struct OnOffFacet {
    instance_id: String,
    locator: Locator,
    runtime: Arc<Runtime>,
    session: Arc<Mutex<NodeSession>>,
}

struct BooleanFacet {
    instance_id: String,
    locator: Locator,
    runtime: Arc<Runtime>,
    session: Arc<Mutex<NodeSession>>,
}

enum NeverMeasurementFacet {}
enum NeverNotificationSinkFacet {}

impl provider::Guest for EspHomeProvider {
    type Runner = Runner;
    type OnOffFacet = OnOffFacet;
    type BooleanFacet = BooleanFacet;
    type MeasurementFacet = NeverMeasurementFacet;
    type NotificationSinkFacet = NeverNotificationSinkFacet;

    async fn start(
        instance_id: String,
        mut config: String,
    ) -> Result<provider::Runner, provider::Error> {
        if instance_id.is_empty() || instance_id.len() > 128 {
            return Err(provider_error(SessionError::InvalidConfiguration));
        }
        let mut session = NodeSession::from_json(&mut config).map_err(provider_error)?;
        let runtime = Arc::new(
            RuntimeBuilder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| provider_error(SessionError::Unavailable))?,
        );
        runtime.block_on(session.start()).map_err(provider_error)?;
        Ok(provider::Runner::new(Runner {
            instance_id,
            runtime,
            session: Arc::new(Mutex::new(session)),
        }))
    }
}

impl provider::GuestRunner for Runner {
    async fn on_off_facets(&self) -> Result<Vec<provider::OnOffFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .switches()
            .map(|entity| {
                provider::OnOffFacet::new(OnOffFacet {
                    instance_id: self.instance_id.clone(),
                    locator: entity.locator.clone(),
                    runtime: Arc::clone(&self.runtime),
                    session: Arc::clone(&self.session),
                })
            })
            .collect())
    }

    async fn boolean_facets(&self) -> Result<Vec<provider::BooleanFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .binary_sensors()
            .map(|entity| {
                provider::BooleanFacet::new(BooleanFacet {
                    instance_id: self.instance_id.clone(),
                    locator: entity.locator.clone(),
                    runtime: Arc::clone(&self.runtime),
                    session: Arc::clone(&self.session),
                })
            })
            .collect())
    }

    async fn measurement_facets(&self) -> Result<Vec<provider::MeasurementFacet>, provider::Error> {
        Ok(Vec::new())
    }

    async fn notification_sink_facets(
        &self,
    ) -> Result<Vec<provider::NotificationSinkFacet>, provider::Error> {
        Ok(Vec::new())
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
        _context: receipt::RequestContext,
    ) -> Result<measurement::MeasurementDescriptor, measurement::Error> {
        match *self {}
    }

    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<measurement::State, measurement::Error> {
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
        _context: receipt::RequestContext,
    ) -> Result<notification::NotificationDescriptor, notification::Error> {
        match *self {}
    }

    async fn send(
        &self,
        _context: receipt::RequestContext,
        _value: notification::Notification,
    ) -> Result<notification::SendReceipt, notification::Error> {
        match *self {}
    }
}

impl provider::GuestOnOffFacet for OnOffFacet {
    fn key(&self) -> String {
        self.locator.switch_key()
    }

    fn incarnation(&self) -> u64 {
        1
    }

    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<on_off::OnOffDescriptor, on_off::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .switch(&self.locator)
            .ok_or_else(endpoint_removed_on_off)?;
        Ok(on_off::OnOffDescriptor {
            common: common_descriptor(
                &self.instance_id,
                &self.key(),
                &entity.label,
                "on-off",
                None,
                &["query", "set"],
            ),
            features: vec![],
            operations: vec![on_off::Operation::Set],
            state_fields: vec![on_off::StateField::Current],
            events: vec![on_off::EventKind::StateObservation],
            operation_support: vec![on_off::OperationSupport {
                operation: on_off::Operation::Set,
                kind: receipt::OperationKind::AbsoluteConvergent,
                deduplication: receipt::DeduplicationGuarantee::None,
                default_deadline_milliseconds: 30_000,
                maximum_deadline_milliseconds: 30_000,
            }],
        })
    }

    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<on_off::State, on_off::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime.block_on(session.refresh());
        let connected = session.connected();
        let entity = session
            .switch(&self.locator)
            .ok_or_else(endpoint_removed_on_off)?;
        Ok(on_off::State {
            meta: observation_meta(entity.revision, entity.received_at_unix_milliseconds),
            availability: availability(connected),
            quality: entity.current.map(|_| {
                if entity.assumed_state {
                    state::Quality::SourceAssumed
                } else {
                    state::Quality::SourceReported
                }
            }),
            current: entity.current,
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
        let value = request.value;
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime
            .block_on(session.set_switch(&self.locator, value))
            .map_err(|error| on_off_session_error(error, &context))?;
        Ok(on_off::OperationReceipt {
            operation: on_off::OperationIntent::Set(request),
            receipt: receipt::CommandReceipt {
                request_id: context.request_id,
                attempt: context.attempt,
                provider_generation: 0,
                confidence: receipt::ReceiptConfidence::AcceptedForDelivery,
                accepted_at_unix_milliseconds: now_unix_milliseconds(),
                completed_at_unix_milliseconds: None,
                idempotency_key: context.idempotency.map(|value| value.key),
            },
        })
    }
}

impl provider::GuestBooleanFacet for BooleanFacet {
    fn key(&self) -> String {
        self.locator.binary_key()
    }

    fn incarnation(&self) -> u64 {
        1
    }

    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<boolean::BooleanDescriptor, boolean::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .binary_sensor(&self.locator)
            .ok_or_else(endpoint_removed_boolean)?;
        let profile = boolean_profile(entity.profile);
        let semantic_profile = Some(boolean_profile_name(profile));
        Ok(boolean::BooleanDescriptor {
            common: common_descriptor(
                &self.instance_id,
                &self.key(),
                &entity.label,
                "boolean-state",
                semantic_profile,
                &["query"],
            ),
            profile,
            state_fields: vec![boolean::StateField::Current],
            events: vec![boolean::EventKind::StateObservation],
        })
    }

    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<boolean::State, boolean::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime.block_on(session.refresh());
        let connected = session.connected();
        let entity = session
            .binary_sensor(&self.locator)
            .ok_or_else(endpoint_removed_boolean)?;
        Ok(boolean::State {
            meta: observation_meta(entity.revision, entity.received_at_unix_milliseconds),
            availability: availability(connected),
            quality: entity.current.map(|_| state::Quality::SourceReported),
            current: entity.current,
        })
    }
}

fn common_descriptor(
    instance_id: &str,
    facet_key: &str,
    label: &str,
    contract: &str,
    semantic_profile: Option<String>,
    operations: &[&str],
) -> descriptor::FacetDescriptor {
    descriptor::FacetDescriptor {
        provider_instance_id: instance_id.into(),
        facet_key: facet_key.into(),
        incarnation: 1,
        generation: 0,
        contract: descriptor::PackageIdentity {
            namespace: "litehouse".into(),
            name: contract.into(),
            version: descriptor::Version {
                major: 1,
                minor: 0,
                patch: 0,
            },
        },
        primary_group_key: instance_id.into(),
        direction: descriptor::FacetDirection::Provided,
        label: Some(label.into()),
        semantic_role: if contract == "on-off" {
            "switch"
        } else {
            "binary-sensor"
        }
        .into(),
        semantic_profile,
        features: vec![],
        operations: operations.iter().map(|value| (*value).into()).collect(),
        events: vec!["state-observation".into()],
        tags: vec![],
    }
}

fn observation_meta(revision: u64, received_at: u64) -> state::ObservationMeta {
    state::ObservationMeta {
        generation: 0,
        revision,
        received_at: state::WallClockInstant {
            unix_milliseconds: received_at,
        },
        source_time: None,
    }
}

fn availability(connected: bool) -> state::Availability {
    if connected {
        state::Availability::Available
    } else {
        state::Availability::Unavailable(state::UnavailabilityReason::Disconnected)
    }
}

fn boolean_profile(profile: BooleanProfile) -> boolean::BooleanProfile {
    match profile {
        BooleanProfile::Contact => boolean::BooleanProfile::Contact,
        BooleanProfile::Occupancy => boolean::BooleanProfile::Occupancy,
        BooleanProfile::Smoke => boolean::BooleanProfile::Smoke,
        BooleanProfile::Status => boolean::BooleanProfile::Status,
        BooleanProfile::Moisture => boolean::BooleanProfile::Moisture,
        BooleanProfile::Vibration => boolean::BooleanProfile::Vibration,
        BooleanProfile::Generic => boolean::BooleanProfile::Generic,
    }
}

fn boolean_profile_name(profile: boolean::BooleanProfile) -> String {
    match profile {
        boolean::BooleanProfile::Contact => "contact",
        boolean::BooleanProfile::Occupancy => "occupancy",
        boolean::BooleanProfile::Smoke => "smoke",
        boolean::BooleanProfile::Status => "status",
        boolean::BooleanProfile::Moisture => "moisture",
        boolean::BooleanProfile::Vibration => "vibration",
        boolean::BooleanProfile::Generic => "generic",
    }
    .into()
}

fn provider_error(error: SessionError) -> provider::Error {
    let (inner, retry) = session_core_error(error);
    provider::Error {
        inner,
        retry,
        message: None,
        error_id: next_error_id(),
    }
}

fn on_off_session_error(error: SessionError, context: &receipt::RequestContext) -> on_off::Error {
    if error == SessionError::OutcomeUnknown {
        return on_off::Error {
            inner: errors::CoreError::OutcomeUnknown(errors::OutcomeUnknownError {
                cause: errors::UnknownOutcomeCause::Disconnected,
                request_id: context.request_id.clone(),
                attempt: context.attempt,
                idempotency_key: context.idempotency.as_ref().map(|value| value.key.clone()),
            }),
            detail: None,
            retry: errors::RetryAdvice::Never,
            message: None,
            error_id: next_error_id(),
        };
    }
    let (inner, retry) = session_core_error(error);
    on_off::Error {
        inner,
        detail: None,
        retry,
        message: None,
        error_id: next_error_id(),
    }
}

fn session_core_error(error: SessionError) -> (errors::CoreError, errors::RetryAdvice) {
    match error {
        SessionError::InvalidConfiguration => {
            (errors::CoreError::ContractError, errors::RetryAdvice::Never)
        }
        SessionError::ConnectionFailed
        | SessionError::DiscoveryFailed
        | SessionError::Unavailable => (
            errors::CoreError::Availability(errors::AvailabilityError::Reconnecting),
            errors::RetryAdvice::Reconnect,
        ),
        SessionError::OutcomeUnknown => (
            errors::CoreError::Execution(errors::ExecutionError::ProtocolFailed),
            errors::RetryAdvice::Never,
        ),
    }
}

fn unsupported_feature(feature: on_off::Feature) -> on_off::Error {
    on_off::Error {
        inner: errors::CoreError::ContractError,
        detail: Some(on_off::ContractErrorDetail::UnsupportedFeature(feature)),
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

fn endpoint_removed_boolean() -> boolean::Error {
    boolean::Error {
        inner: errors::CoreError::Binding(errors::BindingError::EndpointRemoved),
        detail: None,
        retry: errors::RetryAdvice::Rebind,
        message: None,
        error_id: next_error_id(),
    }
}

fn next_error_id() -> String {
    format!(
        "esphome-{:016x}",
        ERROR_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

export!(EspHomeProvider);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_expose_only_absolute_set_and_query() {
        let descriptor = common_descriptor(
            "node",
            "switch-1-2",
            "Relay",
            "on-off",
            None,
            &["query", "set"],
        );
        assert_eq!(descriptor.operations, ["query", "set"]);
        assert!(
            !descriptor
                .operations
                .iter()
                .any(|operation| operation == "toggle")
        );
        assert!(descriptor.tags.is_empty());
    }

    #[test]
    fn failed_write_maps_to_outcome_unknown_without_retry() {
        let context = receipt::RequestContext {
            request_id: "request-1".into(),
            correlation_id: None,
            deadline: receipt::Deadline {
                absolute_monotonic_nanoseconds: u64::MAX,
            },
            idempotency: Some(receipt::IdempotencyContext {
                key: "0123456789abcdef0123456789abcdef".into(),
                scope: "test".into(),
            }),
            attempt: 1,
        };
        let error = on_off_session_error(SessionError::OutcomeUnknown, &context);
        assert!(matches!(
            error.inner,
            errors::CoreError::OutcomeUnknown(errors::OutcomeUnknownError {
                cause: errors::UnknownOutcomeCause::Disconnected,
                ..
            })
        ));
        assert!(matches!(error.retry, errors::RetryAdvice::Never));
    }
}
