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
        "../litehouse-contracts/packages/level-control/1.0.0/wit",
        "../litehouse-contracts/packages/color-control/1.0.0/wit",
        "../litehouse-contracts/packages/thermostat/1.0.0/wit",
        "../litehouse-contracts/packages/window-covering/1.0.0/wit",
        "../litehouse-contracts/packages/momentary-action/1.0.0/wit",
        "../litehouse-contracts/packages/provider/1.0.0/wit",
    ],
    world: "litehouse:provider/component@1.0.0",
    generate_all,
});

use exports::litehouse::provider::provider;
use litehouse::{
    boolean_state::types as boolean, color_control::types as color,
    descriptor::types as descriptor, errors::types as errors, level_control::types as level,
    measurement::types as measurement, momentary_action::types as momentary,
    notification_sink::types as notification, on_off::types as on_off, quantity::types as quantity,
    receipt::types as receipt, state::types as state, thermostat::types as thermostat,
    window_covering::types as covering,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

struct Provider;
struct Runner {
    occurrence_pending: AtomicBool,
}

struct OnOffFacet {
    key: &'static str,
    label: &'static str,
    current: Arc<AtomicBool>,
    revision: AtomicU64,
    fail_set: bool,
}

struct BooleanFacet {
    current: AtomicBool,
    revision: AtomicU64,
}

struct MeasurementFacet;
struct NotificationSinkFacet;
struct LevelControlFacet;
struct ColorControlFacet;
struct ThermostatFacet;
struct WindowCoveringFacet;
struct MomentaryActionFacet;

fn common(
    key: &str,
    label: &str,
    incarnation: u64,
    contract: &str,
    operations: &[&str],
) -> descriptor::FacetDescriptor {
    descriptor::FacetDescriptor {
        provider_instance_id: "f1111111111111111111111111111111".into(),
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
        primary_group_key: "f1111111111111111111111111111111".into(),
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
fn quantity(profile: &str, magnitude: &str, unit: &str) -> quantity::Quantity {
    quantity::Quantity {
        profile: profile.into(),
        magnitude: magnitude.into(),
        unit: unit.into(),
    }
}
fn command_receipt(context: receipt::RequestContext) -> receipt::CommandReceipt {
    receipt::CommandReceipt {
        request_id: context.request_id,
        attempt: context.attempt,
        provider_generation: 0,
        confidence: receipt::ReceiptConfidence::AcceptedForDelivery,
        accepted_at_unix_milliseconds: 1_700_000_000_300,
        completed_at_unix_milliseconds: None,
        idempotency_key: context.idempotency.map(|value| value.key),
    }
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
        if self.fail_set {
            return Err(on_off::Error {
                inner: errors::CoreError::Availability(errors::AvailabilityError::Unavailable),
                detail: None,
                retry: errors::RetryAdvice::Never,
                message: None,
                error_id: "fixture-relay-unavailable".to_owned(),
            });
        }
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
        let mut common = common("button-1", "Button 1", 1, "boolean-state", &["query"]);
        common.events = vec!["state-observation".into(), "occurrence".into()];
        Ok(boolean::BooleanDescriptor {
            common,
            profile: boolean::BooleanProfile::Generic,
            state_fields: vec![boolean::StateField::Current],
            events: vec![
                boolean::EventKind::StateObservation,
                boolean::EventKind::Occurrence,
            ],
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

impl provider::GuestLevelControlFacet for LevelControlFacet {
    fn key(&self) -> String {
        "dimmer-1".into()
    }
    fn incarnation(&self) -> u64 {
        1
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<level::LevelDescriptor, level::Error> {
        Ok(level::LevelDescriptor {
            common: common(
                "dimmer-1",
                "Dimmer 1",
                1,
                "level-control",
                &[
                    "query",
                    "set-level",
                    "step",
                    "move",
                    "stop",
                    "set-level-with-on-off",
                    "step-with-on-off",
                    "move-with-on-off",
                ],
            ),
            role: level::LevelRole::Brightness,
            features: vec![level::Feature::CoupledOnOff],
            operations: vec![
                level::Operation::SetLevel,
                level::Operation::Step,
                level::Operation::Move,
                level::Operation::Stop,
                level::Operation::SetLevelWithOnOff,
                level::Operation::StepWithOnOff,
                level::Operation::MoveWithOnOff,
            ],
            state_fields: vec![level::StateField::CurrentLevel],
            events: vec![level::EventKind::StateObservation],
            native_range: None,
            operation_support: vec![],
        })
    }
    async fn query(&self, _: receipt::RequestContext) -> Result<level::State, level::Error> {
        Ok(level::State {
            meta: meta(1),
            availability: state::Availability::Available,
            quality: Some(state::Quality::SourceReported),
            current: Some(quantity("ratio", "0.5", "1")),
            target: None,
            transition_remaining_milliseconds: None,
        })
    }
    async fn set_level(
        &self,
        c: receipt::RequestContext,
        r: level::SetRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Ok(level::OperationReceipt {
            operation: level::OperationIntent::SetLevel(r),
            receipt: command_receipt(c),
        })
    }
    async fn step(
        &self,
        c: receipt::RequestContext,
        r: level::StepRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Ok(level::OperationReceipt {
            operation: level::OperationIntent::Step(r),
            receipt: command_receipt(c),
        })
    }
    async fn move_(
        &self,
        c: receipt::RequestContext,
        r: level::MoveRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Ok(level::OperationReceipt {
            operation: level::OperationIntent::Move(r),
            receipt: command_receipt(c),
        })
    }
    async fn stop(
        &self,
        c: receipt::RequestContext,
    ) -> Result<level::OperationReceipt, level::Error> {
        Ok(level::OperationReceipt {
            operation: level::OperationIntent::Stop,
            receipt: command_receipt(c),
        })
    }
    async fn set_level_with_on_off(
        &self,
        c: receipt::RequestContext,
        r: level::CoupledRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Ok(level::OperationReceipt {
            operation: level::OperationIntent::SetLevelWithOnOff(r),
            receipt: command_receipt(c),
        })
    }
    async fn step_with_on_off(
        &self,
        c: receipt::RequestContext,
        r: level::CoupledStepRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Ok(level::OperationReceipt {
            operation: level::OperationIntent::StepWithOnOff(r),
            receipt: command_receipt(c),
        })
    }
    async fn move_with_on_off(
        &self,
        c: receipt::RequestContext,
        r: level::CoupledMoveRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Ok(level::OperationReceipt {
            operation: level::OperationIntent::MoveWithOnOff(r),
            receipt: command_receipt(c),
        })
    }
}
impl provider::GuestColorControlFacet for ColorControlFacet {
    fn key(&self) -> String {
        "color-1".into()
    }
    fn incarnation(&self) -> u64 {
        1
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<color::ColorDescriptor, color::Error> {
        Ok(color::ColorDescriptor {
            common: common(
                "color-1",
                "Color 1",
                1,
                "color-control",
                &["query", "set-color", "step-color", "move-color", "stop"],
            ),
            models: vec![color::ColorModel::HueSaturation],
            operations: vec![
                color::Operation::SetColor,
                color::Operation::StepColor,
                color::Operation::MoveColor,
                color::Operation::Stop,
            ],
            state_fields: vec![color::StateField::CurrentColor],
            events: vec![color::EventKind::StateObservation],
            ranges: vec![],
            operation_support: vec![],
        })
    }
    async fn query(&self, _: receipt::RequestContext) -> Result<color::State, color::Error> {
        Ok(color::State {
            meta: meta(1),
            availability: state::Availability::Available,
            quality: Some(state::Quality::SourceReported),
            current: Some(color::ColorValue::HueSaturation(color::HueSaturation {
                hue_degrees: "120".into(),
                saturation_ratio: "0.5".into(),
            })),
            target: None,
            active_model: Some(color::ColorModel::HueSaturation),
        })
    }
    async fn set_color(
        &self,
        c: receipt::RequestContext,
        r: color::SetRequest,
    ) -> Result<color::OperationReceipt, color::Error> {
        Ok(color::OperationReceipt {
            operation: color::OperationIntent::SetColor(r),
            receipt: command_receipt(c),
        })
    }
    async fn step_color(
        &self,
        c: receipt::RequestContext,
        r: color::StepRequest,
    ) -> Result<color::OperationReceipt, color::Error> {
        Ok(color::OperationReceipt {
            operation: color::OperationIntent::StepColor(r),
            receipt: command_receipt(c),
        })
    }
    async fn move_color(
        &self,
        c: receipt::RequestContext,
        r: color::MoveRequest,
    ) -> Result<color::OperationReceipt, color::Error> {
        Ok(color::OperationReceipt {
            operation: color::OperationIntent::MoveColor(r),
            receipt: command_receipt(c),
        })
    }
    async fn stop(
        &self,
        c: receipt::RequestContext,
    ) -> Result<color::OperationReceipt, color::Error> {
        Ok(color::OperationReceipt {
            operation: color::OperationIntent::Stop,
            receipt: command_receipt(c),
        })
    }
}
impl provider::GuestThermostatFacet for ThermostatFacet {
    fn key(&self) -> String {
        "thermostat-1".into()
    }
    fn incarnation(&self) -> u64 {
        1
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<thermostat::ThermostatDescriptor, thermostat::Error> {
        Ok(thermostat::ThermostatDescriptor {
            common: common(
                "thermostat-1",
                "Thermostat 1",
                1,
                "thermostat",
                &[
                    "query",
                    "set-mode",
                    "set-single-target",
                    "set-dual-target",
                    "set-fan",
                    "set-swing",
                    "set-preset",
                ],
            ),
            modes: vec![thermostat::Mode::Known(thermostat::KnownMode::Heat)],
            target_policy: thermostat::TargetPolicy {
                model: thermostat::SetpointModel::Single,
                minimum: quantity("temperature", "10", "Cel"),
                maximum: quantity("temperature", "30", "Cel"),
                step: quantity("temperature-delta", "0.5", "K"),
                minimum_deadband: None,
            },
            features: vec![],
            operations: vec![
                thermostat::Operation::SetMode,
                thermostat::Operation::SetSingleTarget,
                thermostat::Operation::SetDualTarget,
                thermostat::Operation::SetFan,
                thermostat::Operation::SetSwing,
                thermostat::Operation::SetPreset,
            ],
            state_fields: vec![thermostat::StateField::CurrentMode],
            events: vec![thermostat::EventKind::StateObservation],
            fan_choices: vec![],
            swing_choices: vec![],
            preset_choices: vec![],
            operation_support: vec![],
        })
    }
    async fn query(
        &self,
        _: receipt::RequestContext,
    ) -> Result<thermostat::State, thermostat::Error> {
        Ok(thermostat::State {
            meta: meta(1),
            availability: state::Availability::Available,
            quality: Some(state::Quality::SourceReported),
            current_mode: Some(thermostat::Mode::Known(thermostat::KnownMode::Heat)),
            target_mode: None,
            current_action: Some(thermostat::Action::Heating),
            running: Some(true),
            targets: Some(thermostat::Targets::Single(thermostat::SingleTarget {
                temperature: quantity("temperature", "21", "Cel"),
            })),
            fan: None,
            swing: None,
            preset: None,
        })
    }
    async fn set_mode(
        &self,
        c: receipt::RequestContext,
        r: thermostat::SetModeRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        Ok(thermostat::OperationReceipt {
            operation: thermostat::OperationIntent::SetMode(r),
            receipt: command_receipt(c),
        })
    }
    async fn set_single_target(
        &self,
        c: receipt::RequestContext,
        r: thermostat::SetSingleTargetRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        Ok(thermostat::OperationReceipt {
            operation: thermostat::OperationIntent::SetSingleTarget(r),
            receipt: command_receipt(c),
        })
    }
    async fn set_dual_target(
        &self,
        c: receipt::RequestContext,
        r: thermostat::SetDualTargetRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        Ok(thermostat::OperationReceipt {
            operation: thermostat::OperationIntent::SetDualTarget(r),
            receipt: command_receipt(c),
        })
    }
    async fn set_fan(
        &self,
        c: receipt::RequestContext,
        r: thermostat::SetChoiceRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        Ok(thermostat::OperationReceipt {
            operation: thermostat::OperationIntent::SetFan(r),
            receipt: command_receipt(c),
        })
    }
    async fn set_swing(
        &self,
        c: receipt::RequestContext,
        r: thermostat::SetChoiceRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        Ok(thermostat::OperationReceipt {
            operation: thermostat::OperationIntent::SetSwing(r),
            receipt: command_receipt(c),
        })
    }
    async fn set_preset(
        &self,
        c: receipt::RequestContext,
        r: thermostat::SetChoiceRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        Ok(thermostat::OperationReceipt {
            operation: thermostat::OperationIntent::SetPreset(r),
            receipt: command_receipt(c),
        })
    }
}
impl provider::GuestWindowCoveringFacet for WindowCoveringFacet {
    fn key(&self) -> String {
        "covering-1".into()
    }
    fn incarnation(&self) -> u64 {
        1
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<covering::CoveringDescriptor, covering::Error> {
        Ok(covering::CoveringDescriptor {
            common: common(
                "covering-1",
                "Covering 1",
                1,
                "window-covering",
                &["query", "open", "close", "stop", "set-position"],
            ),
            axes: vec![covering::AxisDescriptor {
                axis: covering::Axis::Lift,
                awareness: covering::PositionAwareness::SourceReported,
                native_step: None,
            }],
            features: vec![covering::Feature::Lift],
            operations: vec![
                covering::Operation::Open,
                covering::Operation::Close,
                covering::Operation::Stop,
                covering::Operation::SetPosition,
            ],
            state_fields: vec![covering::StateField::CurrentPosition],
            events: vec![covering::EventKind::StateObservation],
            operation_support: vec![],
        })
    }
    async fn query(&self, _: receipt::RequestContext) -> Result<covering::State, covering::Error> {
        Ok(covering::State {
            meta: meta(1),
            availability: state::Availability::Available,
            quality: Some(state::Quality::SourceReported),
            axes: vec![covering::AxisState {
                axis: covering::Axis::Lift,
                current: Some(quantity("ratio", "0.25", "1")),
                target: None,
                movement: covering::Movement::Idle,
            }],
            safety_status: vec![],
            fault_status: vec![],
        })
    }
    async fn open(
        &self,
        c: receipt::RequestContext,
        r: covering::AxisRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        Ok(covering::OperationReceipt {
            operation: covering::OperationIntent::Open(r),
            receipt: command_receipt(c),
        })
    }
    async fn close(
        &self,
        c: receipt::RequestContext,
        r: covering::AxisRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        Ok(covering::OperationReceipt {
            operation: covering::OperationIntent::Close(r),
            receipt: command_receipt(c),
        })
    }
    async fn stop(
        &self,
        c: receipt::RequestContext,
        r: covering::AxisRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        Ok(covering::OperationReceipt {
            operation: covering::OperationIntent::Stop(r),
            receipt: command_receipt(c),
        })
    }
    async fn set_position(
        &self,
        c: receipt::RequestContext,
        r: covering::PositionRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        Ok(covering::OperationReceipt {
            operation: covering::OperationIntent::SetPosition(r),
            receipt: command_receipt(c),
        })
    }
}
impl provider::GuestMomentaryActionFacet for MomentaryActionFacet {
    fn key(&self) -> String {
        "action-1".into()
    }
    fn incarnation(&self) -> u64 {
        1
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<momentary::ActionDescriptor, momentary::Error> {
        Ok(momentary::ActionDescriptor {
            common: common("action-1", "Action 1", 1, "momentary-action", &["invoke"]),
            action_id: "fixture-action".into(),
            risk: momentary::Risk::Routine,
            operation: momentary::OperationSupport {
                kind: receipt::OperationKind::Momentary,
                deduplication: receipt::DeduplicationGuarantee::None,
                default_deadline_milliseconds: 5_000,
                maximum_deadline_milliseconds: 5_000,
            },
        })
    }
    async fn invoke(
        &self,
        c: receipt::RequestContext,
    ) -> Result<momentary::InvokeReceipt, momentary::Error> {
        Ok(momentary::InvokeReceipt {
            action_id: "fixture-action".into(),
            receipt: command_receipt(c),
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
                fail_set: false,
            }),
            provider::OnOffFacet::new(OnOffFacet {
                key: "relay-2",
                label: "Relay 2",
                current: Arc::new(AtomicBool::new(true)),
                revision: AtomicU64::new(1),
                fail_set: true,
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
    async fn level_control_facets(
        &self,
    ) -> Result<Vec<provider::LevelControlFacet>, provider::Error> {
        Ok(vec![provider::LevelControlFacet::new(LevelControlFacet)])
    }
    async fn color_control_facets(
        &self,
    ) -> Result<Vec<provider::ColorControlFacet>, provider::Error> {
        Ok(vec![provider::ColorControlFacet::new(ColorControlFacet)])
    }
    async fn thermostat_facets(&self) -> Result<Vec<provider::ThermostatFacet>, provider::Error> {
        Ok(vec![provider::ThermostatFacet::new(ThermostatFacet)])
    }
    async fn window_covering_facets(
        &self,
    ) -> Result<Vec<provider::WindowCoveringFacet>, provider::Error> {
        Ok(vec![provider::WindowCoveringFacet::new(
            WindowCoveringFacet,
        )])
    }
    async fn momentary_action_facets(
        &self,
    ) -> Result<Vec<provider::MomentaryActionFacet>, provider::Error> {
        Ok(vec![provider::MomentaryActionFacet::new(
            MomentaryActionFacet,
        )])
    }

    async fn drain_boolean_occurrences(
        &self,
        max_items: u16,
    ) -> Result<provider::BooleanOccurrenceBatch, provider::Error> {
        let events = if max_items > 0 && self.occurrence_pending.swap(false, Ordering::SeqCst) {
            vec![provider::BooleanOccurrenceEvent {
                sequence: 1,
                facet_key: "button-1".to_owned(),
                occurrence: boolean::Occurrence {
                    event_id: "fixture-occurrence-1".to_owned(),
                    source_id: "button-1".to_owned(),
                    generation: 1,
                    received_at_unix_milliseconds: 1_700_000_000_500,
                    value: Some(true),
                },
            }]
        } else {
            Vec::new()
        };
        Ok(provider::BooleanOccurrenceBatch { events, dropped: 0 })
    }
}

impl provider::Guest for Provider {
    type Runner = Runner;
    type OnOffFacet = OnOffFacet;
    type BooleanFacet = BooleanFacet;
    type MeasurementFacet = MeasurementFacet;
    type NotificationSinkFacet = NotificationSinkFacet;
    type LevelControlFacet = LevelControlFacet;
    type ColorControlFacet = ColorControlFacet;
    type ThermostatFacet = ThermostatFacet;
    type WindowCoveringFacet = WindowCoveringFacet;
    type MomentaryActionFacet = MomentaryActionFacet;

    async fn start(
        instance_id: String,
        config: String,
    ) -> Result<provider::Runner, provider::Error> {
        if instance_id != "f1111111111111111111111111111111" || config != "fixture" {
            return Err(provider::Error {
                inner: errors::CoreError::ContractError,
                retry: never_error(),
                message: None,
                error_id: "fixture-config".into(),
            });
        }
        Ok(provider::Runner::new(Runner {
            occurrence_pending: AtomicBool::new(true),
        }))
    }
}

export!(Provider);
