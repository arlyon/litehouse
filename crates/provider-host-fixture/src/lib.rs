//! Test-only component for the production provider host adapter.

wit_bindgen::generate!({
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
    generate_all,
});

use exports::litehouse::provider::provider;
use litehouse::{
    boolean_state::types as boolean, descriptor::types as descriptor, errors::types as errors,
    measurement::types as measurement, notification_sink::types as notification,
    on_off::types as on_off, quantity::types as quantity, receipt::types as receipt,
    state::types as state,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

struct Provider;
struct Runner;

struct OnOffFacet {
    key: &'static str,
    label: &'static str,
    current: Arc<AtomicBool>,
    revision: AtomicU64,
}

struct BooleanFacet {
    current: AtomicBool,
    revision: AtomicU64,
}

struct MeasurementFacet;
struct NotificationSinkFacet;

fn common(
    key: &str,
    label: &str,
    incarnation: u64,
    contract: &str,
    operations: &[&str],
) -> descriptor::FacetDescriptor {
    descriptor::FacetDescriptor {
        provider_instance_id: "fixture-node".into(),
        facet_key: key.into(),
        incarnation,
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
        primary_group_key: "fixture-node".into(),
        direction: descriptor::FacetDirection::Provided,
        label: Some(label.into()),
        semantic_role: "fixture".into(),
        semantic_profile: None,
        features: vec![],
        operations: operations.iter().map(|value| (*value).into()).collect(),
        events: vec![],
        tags: vec![],
    }
}

fn meta(revision: u64) -> state::ObservationMeta {
    state::ObservationMeta {
        generation: 0,
        revision,
        received_at: state::WallClockInstant {
            unix_milliseconds: 1_700_000_000_000 + revision,
        },
        source_time: None,
    }
}

fn never_error() -> errors::RetryAdvice {
    errors::RetryAdvice::Never
}

impl provider::GuestMeasurementFacet for MeasurementFacet {
    fn key(&self) -> String {
        "temperature-1".into()
    }

    fn incarnation(&self) -> u64 {
        1
    }

    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<measurement::MeasurementDescriptor, measurement::Error> {
        Ok(measurement::MeasurementDescriptor {
            common: common(
                "temperature-1",
                "Temperature 1",
                1,
                "measurement",
                &["query"],
            ),
            profile: "temperature".into(),
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
        })
    }

    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<measurement::State, measurement::Error> {
        Ok(measurement::State {
            meta: meta(1),
            availability: state::Availability::Available,
            quality: Some(state::Quality::SourceReported),
            current: Some(quantity::Quantity {
                profile: "temperature".into(),
                magnitude: "21.5".into(),
                unit: "Cel".into(),
            }),
            native: Some(quantity::NativeRepresentation {
                magnitude: "21.5".into(),
                unit: "°C".into(),
                quantization: None,
            }),
        })
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
        Ok(notification::NotificationDescriptor {
            common: common(
                "notification",
                "Notification",
                1,
                "notification-sink",
                &["send"],
            ),
            supported_fields: vec![
                notification::OptionalField::Title,
                notification::OptionalField::Priority,
            ],
            maximum_title_bytes: 256,
            maximum_body_bytes: 4096,
            maximum_receipt_confidence: receipt::ReceiptConfidence::AcceptedForDelivery,
            operation: notification::OperationSupport {
                kind: receipt::OperationKind::Momentary,
                deduplication: receipt::DeduplicationGuarantee::None,
                default_deadline_milliseconds: 8_000,
                maximum_deadline_milliseconds: 8_000,
            },
        })
    }

    async fn send(
        &self,
        context: receipt::RequestContext,
        _value: notification::Notification,
    ) -> Result<notification::SendReceipt, notification::Error> {
        Ok(notification::SendReceipt {
            receipt: receipt::CommandReceipt {
                request_id: context.request_id,
                attempt: context.attempt,
                provider_generation: 0,
                confidence: receipt::ReceiptConfidence::AcceptedForDelivery,
                accepted_at_unix_milliseconds: 1_700_000_000_200,
                completed_at_unix_milliseconds: None,
                idempotency_key: context.idempotency.map(|value| value.key),
            },
            provider_message_id: Some("fixture-message".into()),
        })
    }
}

impl provider::GuestOnOffFacet for OnOffFacet {
    fn key(&self) -> String {
        self.key.into()
    }

    fn incarnation(&self) -> u64 {
        1
    }

    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<on_off::OnOffDescriptor, on_off::Error> {
        Ok(on_off::OnOffDescriptor {
            common: common(self.key, self.label, 1, "on-off", &["query", "set"]),
            features: vec![],
            operations: vec![on_off::Operation::Set],
            state_fields: vec![on_off::StateField::Current],
            events: vec![on_off::EventKind::StateObservation],
            operation_support: vec![on_off::OperationSupport {
                operation: on_off::Operation::Set,
                kind: receipt::OperationKind::AbsoluteConvergent,
                deduplication: receipt::DeduplicationGuarantee::ProviderGeneration,
                default_deadline_milliseconds: 30_000,
                maximum_deadline_milliseconds: 30_000,
            }],
        })
    }

    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<on_off::State, on_off::Error> {
        Ok(on_off::State {
            meta: meta(self.revision.load(Ordering::Relaxed)),
            availability: state::Availability::Available,
            quality: Some(state::Quality::SourceReported),
            current: Some(self.current.load(Ordering::Relaxed)),
        })
    }

    async fn set(
        &self,
        context: receipt::RequestContext,
        request: on_off::SetRequest,
    ) -> Result<on_off::OperationReceipt, on_off::Error> {
        self.current.store(request.value, Ordering::Relaxed);
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(on_off::OperationReceipt {
            operation: on_off::OperationIntent::Set(request),
            receipt: receipt::CommandReceipt {
                request_id: context.request_id,
                attempt: context.attempt,
                provider_generation: 0,
                confidence: receipt::ReceiptConfidence::AcceptedForDelivery,
                accepted_at_unix_milliseconds: 1_700_000_000_100,
                completed_at_unix_milliseconds: None,
                idempotency_key: context.idempotency.map(|value| value.key),
            },
        })
    }
}

impl provider::GuestBooleanFacet for BooleanFacet {
    fn key(&self) -> String {
        "button-1".into()
    }

    fn incarnation(&self) -> u64 {
        1
    }

    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<boolean::BooleanDescriptor, boolean::Error> {
        Ok(boolean::BooleanDescriptor {
            common: common("button-1", "Button 1", 1, "boolean-state", &["query"]),
            profile: boolean::BooleanProfile::Generic,
            state_fields: vec![boolean::StateField::Current],
            events: vec![boolean::EventKind::StateObservation],
        })
    }

    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<boolean::State, boolean::Error> {
        Ok(boolean::State {
            meta: meta(self.revision.load(Ordering::Relaxed)),
            availability: state::Availability::Available,
            quality: Some(state::Quality::SourceReported),
            current: Some(self.current.load(Ordering::Relaxed)),
        })
    }
}

impl provider::GuestRunner for Runner {
    async fn on_off_facets(&self) -> Result<Vec<provider::OnOffFacet>, provider::Error> {
        Ok(vec![
            provider::OnOffFacet::new(OnOffFacet {
                key: "relay-1",
                label: "Relay 1",
                current: Arc::new(AtomicBool::new(false)),
                revision: AtomicU64::new(1),
            }),
            provider::OnOffFacet::new(OnOffFacet {
                key: "relay-2",
                label: "Relay 2",
                current: Arc::new(AtomicBool::new(true)),
                revision: AtomicU64::new(1),
            }),
        ])
    }

    async fn boolean_facets(&self) -> Result<Vec<provider::BooleanFacet>, provider::Error> {
        Ok(vec![provider::BooleanFacet::new(BooleanFacet {
            current: AtomicBool::new(true),
            revision: AtomicU64::new(1),
        })])
    }

    async fn measurement_facets(&self) -> Result<Vec<provider::MeasurementFacet>, provider::Error> {
        Ok(vec![provider::MeasurementFacet::new(MeasurementFacet)])
    }

    async fn notification_sink_facets(
        &self,
    ) -> Result<Vec<provider::NotificationSinkFacet>, provider::Error> {
        Ok(vec![provider::NotificationSinkFacet::new(
            NotificationSinkFacet,
        )])
    }
}

impl provider::Guest for Provider {
    type Runner = Runner;
    type OnOffFacet = OnOffFacet;
    type BooleanFacet = BooleanFacet;
    type MeasurementFacet = MeasurementFacet;
    type NotificationSinkFacet = NotificationSinkFacet;

    async fn start(
        instance_id: String,
        config: String,
    ) -> Result<provider::Runner, provider::Error> {
        if instance_id != "fixture-node" || config != "fixture" {
            return Err(provider::Error {
                inner: errors::CoreError::ContractError,
                retry: never_error(),
                message: None,
                error_id: "fixture-config".into(),
            });
        }
        Ok(provider::Runner::new(Runner))
    }
}

export!(Provider);
