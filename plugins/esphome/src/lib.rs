//! ESPHome Native API provider component.
//!
//! One configured instance owns one encrypted API 1.14 session and cache shared
//! by every typed facet. Only source-advertised, losslessly representable
//! capabilities are published; every command is absolute or momentary.

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
        "../../crates/litehouse-contracts/packages/level-control/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/color-control/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/thermostat/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/window-covering/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/momentary-action/1.0.0/wit",
        "../../crates/litehouse-contracts/packages/provider/1.0.0/wit",
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
use session::{
    BooleanProfile, ClimateChoice, ClimateEntity, ClimateWrite, CoverEntity, CoverWrite,
    EntityKind, FacetSeed, HydrationStatus, LightColorCommand, LightEntity, Locator,
    MeasurementKind, NodeSession, Observation, PROFILE_RATIO, PROFILE_TEMPERATURE,
    ProjectedObservation, SessionError, decimal, kelvin_to_source_temperature,
    now_unix_milliseconds, source_temperature_to_kelvin,
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::{
    runtime::{Builder as RuntimeBuilder, Runtime},
    sync::Mutex,
};

const PROFILE_TEMPERATURE_DIFFERENCE: &str = "litehouse:quantity/temperature-difference@1";
const COMMAND_DEADLINE_MILLISECONDS: u32 = 30_000;
static ERROR_SEQUENCE: AtomicU64 = AtomicU64::new(1);

struct EspHomeProvider;

struct Runner {
    instance_id: String,
    runtime: Arc<Runtime>,
    session: Arc<Mutex<NodeSession>>,
}

macro_rules! facet {
    ($name:ident) => {
        struct $name {
            instance_id: String,
            locator: Locator,
            incarnation: u64,
            runtime: Arc<Runtime>,
            session: Arc<Mutex<NodeSession>>,
        }
    };
}
facet!(OnOffFacet);
facet!(BooleanFacet);
facet!(MeasurementFacet);
facet!(LevelControlFacet);
facet!(ColorControlFacet);
facet!(ThermostatFacet);
facet!(WindowCoveringFacet);
facet!(MomentaryActionFacet);

enum NeverNotificationSinkFacet {}

impl provider::Guest for EspHomeProvider {
    type Runner = Runner;
    type OnOffFacet = OnOffFacet;
    type BooleanFacet = BooleanFacet;
    type MeasurementFacet = MeasurementFacet;
    type NotificationSinkFacet = NeverNotificationSinkFacet;
    type LevelControlFacet = LevelControlFacet;
    type ColorControlFacet = ColorControlFacet;
    type ThermostatFacet = ThermostatFacet;
    type WindowCoveringFacet = WindowCoveringFacet;
    type MomentaryActionFacet = MomentaryActionFacet;

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

impl Runner {
    fn make_facet<T>(&self, seed: FacetSeed, make: impl FnOnce(FacetFields) -> T) -> T {
        make(FacetFields {
            instance_id: self.instance_id.clone(),
            locator: seed.locator,
            incarnation: seed.incarnation,
            runtime: Arc::clone(&self.runtime),
            session: Arc::clone(&self.session),
        })
    }
}

struct FacetFields {
    instance_id: String,
    locator: Locator,
    incarnation: u64,
    runtime: Arc<Runtime>,
    session: Arc<Mutex<NodeSession>>,
}

macro_rules! from_fields {
    ($name:ident) => {
        impl From<FacetFields> for $name {
            fn from(value: FacetFields) -> Self {
                Self {
                    instance_id: value.instance_id,
                    locator: value.locator,
                    incarnation: value.incarnation,
                    runtime: value.runtime,
                    session: value.session,
                }
            }
        }
    };
}
from_fields!(OnOffFacet);
from_fields!(BooleanFacet);
from_fields!(MeasurementFacet);
from_fields!(LevelControlFacet);
from_fields!(ColorControlFacet);
from_fields!(ThermostatFacet);
from_fields!(WindowCoveringFacet);
from_fields!(MomentaryActionFacet);

impl provider::GuestRunner for Runner {
    async fn on_off_facets(&self) -> Result<Vec<provider::OnOffFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .on_off_seeds()
            .into_iter()
            .map(|seed| provider::OnOffFacet::new(self.make_facet(seed, OnOffFacet::from)))
            .collect())
    }

    async fn boolean_facets(&self) -> Result<Vec<provider::BooleanFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .boolean_seeds()
            .into_iter()
            .map(|seed| provider::BooleanFacet::new(self.make_facet(seed, BooleanFacet::from)))
            .collect())
    }

    async fn measurement_facets(&self) -> Result<Vec<provider::MeasurementFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .measurement_seeds()
            .into_iter()
            .map(|seed| {
                provider::MeasurementFacet::new(self.make_facet(seed, MeasurementFacet::from))
            })
            .collect())
    }

    async fn notification_sink_facets(
        &self,
    ) -> Result<Vec<provider::NotificationSinkFacet>, provider::Error> {
        Ok(Vec::new())
    }

    async fn level_control_facets(
        &self,
    ) -> Result<Vec<provider::LevelControlFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .level_seeds()
            .into_iter()
            .map(|seed| {
                provider::LevelControlFacet::new(self.make_facet(seed, LevelControlFacet::from))
            })
            .collect())
    }

    async fn color_control_facets(
        &self,
    ) -> Result<Vec<provider::ColorControlFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .color_seeds()
            .into_iter()
            .map(|seed| {
                provider::ColorControlFacet::new(self.make_facet(seed, ColorControlFacet::from))
            })
            .collect())
    }

    async fn thermostat_facets(&self) -> Result<Vec<provider::ThermostatFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .thermostat_seeds()
            .into_iter()
            .map(|seed| {
                provider::ThermostatFacet::new(self.make_facet(seed, ThermostatFacet::from))
            })
            .collect())
    }

    async fn window_covering_facets(
        &self,
    ) -> Result<Vec<provider::WindowCoveringFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .covering_seeds()
            .into_iter()
            .map(|seed| {
                provider::WindowCoveringFacet::new(self.make_facet(seed, WindowCoveringFacet::from))
            })
            .collect())
    }

    async fn momentary_action_facets(
        &self,
    ) -> Result<Vec<provider::MomentaryActionFacet>, provider::Error> {
        let session = self.runtime.block_on(self.session.lock());
        Ok(session
            .momentary_seeds()
            .into_iter()
            .map(|seed| {
                provider::MomentaryActionFacet::new(
                    self.make_facet(seed, MomentaryActionFacet::from),
                )
            })
            .collect())
    }

    async fn drain_boolean_occurrences(
        &self,
        max_items: u16,
    ) -> Result<provider::BooleanOccurrenceBatch, provider::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime
            .block_on(session.refresh_occurrences())
            .map_err(provider_error)?;
        let (events, dropped) = session.drain_boolean_occurrences(usize::from(max_items));
        Ok(provider::BooleanOccurrenceBatch {
            events: events
                .into_iter()
                .map(|event| provider::BooleanOccurrenceEvent {
                    sequence: event.sequence,
                    facet_key: event.facet_key.clone(),
                    occurrence: boolean::Occurrence {
                        event_id: format!("{}-{:016x}", self.instance_id, event.sequence),
                        source_id: event.facet_key,
                        generation: event.generation,
                        received_at_unix_milliseconds: event.received_at_unix_milliseconds,
                        value: Some(event.value),
                    },
                })
                .collect(),
            dropped,
        })
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

impl provider::GuestOnOffFacet for OnOffFacet {
    fn key(&self) -> String {
        self.locator.facet_key()
    }

    fn incarnation(&self) -> u64 {
        self.incarnation
    }

    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<on_off::OnOffDescriptor, on_off::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .on_off(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_on_off)?;
        Ok(on_off::OnOffDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                locator: &self.locator,
                incarnation: self.incarnation,
                generation: session.generation(),
                label: &entity.common().label,
                contract: "on-off",
                semantic_role: entity.semantic_role(),
                semantic_profile: None,
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
                default_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
                maximum_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
            }],
        })
    }

    async fn query(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<on_off::State, on_off::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime.block_on(session.refresh());
        let observation = session
            .on_off_state(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_on_off)?;
        Ok(on_off::State {
            meta: projected_meta(session.generation(), &observation),
            availability: projected_availability(&session, &observation),
            quality: observation.current.map(|_| {
                if observation.assumed {
                    state::Quality::SourceAssumed
                } else {
                    state::Quality::SourceReported
                }
            }),
            current: observation.current,
        })
    }

    async fn set(
        &self,
        context: receipt::RequestContext,
        request: on_off::SetRequest,
    ) -> Result<on_off::OperationReceipt, on_off::Error> {
        if request.timed.is_some() {
            return Err(on_off_unsupported_feature(on_off::Feature::Timed));
        }
        if request.effect.is_some() {
            return Err(on_off_unsupported_feature(on_off::Feature::Effect));
        }
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime
            .block_on(session.set_on_off(&self.locator, self.incarnation, request.value))
            .map_err(|error| on_off_session_error(error, &context))?;
        Ok(on_off::OperationReceipt {
            operation: on_off::OperationIntent::Set(request),
            receipt: command_receipt(context, session.generation()),
        })
    }
}

impl provider::GuestBooleanFacet for BooleanFacet {
    fn key(&self) -> String {
        self.locator.facet_key()
    }
    fn incarnation(&self) -> u64 {
        self.incarnation
    }
    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<boolean::BooleanDescriptor, boolean::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .binary_sensor(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_boolean)?;
        let profile = boolean_profile(entity.profile);
        Ok(boolean::BooleanDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                locator: &self.locator,
                incarnation: self.incarnation,
                generation: session.generation(),
                label: &entity.common.label,
                contract: "boolean-state",
                semantic_role: "binary-sensor",
                semantic_profile: Some(boolean_profile_name(profile)),
                operations: &["query"],
                events: &["state-observation", "occurrence"],
            }),
            profile,
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
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime.block_on(session.refresh());
        let entity = session
            .binary_sensor(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_boolean)?;
        Ok(boolean::State {
            meta: observation_meta(session.generation(), &entity.observation),
            availability: observation_availability(&session, &entity.observation),
            quality: entity
                .observation
                .current
                .map(|_| state::Quality::SourceReported),
            current: entity.observation.current,
        })
    }
}

impl provider::GuestMeasurementFacet for MeasurementFacet {
    fn key(&self) -> String {
        self.locator.facet_key()
    }
    fn incarnation(&self) -> u64 {
        self.incarnation
    }
    async fn describe(
        &self,
        _context: receipt::RequestContext,
    ) -> Result<measurement::MeasurementDescriptor, measurement::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .sensor(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_measurement)?;
        Ok(measurement::MeasurementDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                locator: &self.locator,
                incarnation: self.incarnation,
                generation: session.generation(),
                label: &entity.common.label,
                contract: "measurement",
                semantic_role: "sensor",
                semantic_profile: Some(entity.profile),
                operations: &["query"],
                events: &["observation"],
            }),
            profile: entity.profile.into(),
            kind: match entity.kind {
                MeasurementKind::Instantaneous => measurement::MeasurementKind::Instantaneous,
                MeasurementKind::Total => measurement::MeasurementKind::Total,
                MeasurementKind::TotalIncreasing => measurement::MeasurementKind::TotalIncreasing,
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
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime.block_on(session.refresh());
        let entity = session
            .sensor(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_measurement)?;
        let current = entity
            .observation
            .current
            .as_ref()
            .map(|value| quantity::Quantity {
                profile: entity.profile.into(),
                magnitude: value.canonical_magnitude.clone(),
                unit: entity.canonical_unit.into(),
            });
        let native =
            entity
                .observation
                .current
                .as_ref()
                .map(|value| quantity::NativeRepresentation {
                    magnitude: value.native_magnitude.clone(),
                    unit: entity.native_unit.clone(),
                    quantization: entity
                        .accuracy_decimals
                        .map(|scale| quantity::Quantization {
                            scale,
                            mode: quantity::RoundingMode::HalfEven,
                        }),
                });
        Ok(measurement::State {
            meta: observation_meta(session.generation(), &entity.observation),
            availability: observation_availability(&session, &entity.observation),
            quality: current.as_ref().map(|_| state::Quality::SourceReported),
            current,
            native,
        })
    }
}

impl provider::GuestLevelControlFacet for LevelControlFacet {
    fn key(&self) -> String {
        self.locator.facet_key()
    }
    fn incarnation(&self) -> u64 {
        self.incarnation
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<level::LevelDescriptor, level::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let (label, native_range) = match self.locator.kind {
            EntityKind::Light => {
                let entity = session
                    .light(&self.locator, self.incarnation)
                    .filter(|entity| entity.supports_level)
                    .ok_or_else(endpoint_removed_level)?;
                (&entity.common.label, None)
            }
            EntityKind::Fan => {
                let entity = session
                    .fan(&self.locator, self.incarnation)
                    .filter(|entity| entity.speed_count.is_some())
                    .ok_or_else(endpoint_removed_level)?;
                let count = entity.speed_count.expect("filtered speed count");
                (
                    &entity.common.label,
                    Some(level::NativeRange {
                        minimum: "1".into(),
                        maximum: count.to_string(),
                        step: Some("1".into()),
                        unit: "speed-level".into(),
                    }),
                )
            }
            _ => return Err(endpoint_removed_level()),
        };
        let mut features = Vec::new();
        if native_range.is_some() {
            features.push(level::Feature::NativeRange);
        }
        Ok(level::LevelDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                locator: &self.locator,
                incarnation: self.incarnation,
                generation: session.generation(),
                label,
                contract: "level-control",
                semantic_role: if self.locator.kind == EntityKind::Light {
                    "brightness"
                } else {
                    "fan-speed"
                },
                semantic_profile: Some(PROFILE_RATIO),
                operations: &["query", "set-level"],
                events: &["state-observation"],
            }),
            role: if self.locator.kind == EntityKind::Light {
                level::LevelRole::Brightness
            } else {
                level::LevelRole::Generic
            },
            features,
            operations: vec![level::Operation::SetLevel],
            state_fields: vec![level::StateField::CurrentLevel],
            events: vec![level::EventKind::StateObservation],
            native_range,
            operation_support: vec![level::OperationSupport {
                operation: level::Operation::SetLevel,
                kind: receipt::OperationKind::AbsoluteConvergent,
                deduplication: receipt::DeduplicationGuarantee::None,
                default_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
                maximum_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
            }],
        })
    }
    async fn query(&self, _: receipt::RequestContext) -> Result<level::State, level::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime.block_on(session.refresh());
        let (observation, assumed) = match self.locator.kind {
            EntityKind::Light => {
                let entity = session
                    .light(&self.locator, self.incarnation)
                    .filter(|entity| entity.supports_level)
                    .ok_or_else(endpoint_removed_level)?;
                (
                    project_observation(&entity.observation, |value| {
                        valid_ratio(value.brightness).map(ratio_quantity)
                    }),
                    false,
                )
            }
            EntityKind::Fan => {
                let entity = session
                    .fan(&self.locator, self.incarnation)
                    .filter(|entity| entity.speed_count.is_some())
                    .ok_or_else(endpoint_removed_level)?;
                let count = entity.speed_count.expect("filtered speed count") as f32;
                (
                    project_observation(&entity.observation, |value| {
                        let ratio = value.1 as f32 / count;
                        valid_ratio(ratio).map(ratio_quantity)
                    }),
                    false,
                )
            }
            _ => return Err(endpoint_removed_level()),
        };
        Ok(level::State {
            meta: observation.meta(session.generation()),
            availability: observation.availability(&session),
            quality: observation.current.as_ref().map(|_| {
                if assumed {
                    state::Quality::SourceAssumed
                } else {
                    state::Quality::SourceReported
                }
            }),
            current: observation.current.flatten(),
            target: None,
            transition_remaining_milliseconds: None,
        })
    }
    async fn set_level(
        &self,
        context: receipt::RequestContext,
        request: level::SetRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        if request.transition.is_some() {
            return Err(level_contract_error(
                level::ContractErrorDetail::UnsupportedTransition,
            ));
        }
        let ratio = parse_ratio(&request.level).ok_or_else(|| {
            level_contract_error(level::ContractErrorDetail::OutOfRange(
                request.level.clone(),
            ))
        })?;
        if self.locator.kind == EntityKind::Fan && ratio == 0.0 {
            return Err(level_contract_error(
                level::ContractErrorDetail::OutOfRange(request.level.clone()),
            ));
        }
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime
            .block_on(session.set_level(&self.locator, self.incarnation, ratio))
            .map_err(|error| level_session_error(error, &context))?;
        Ok(level::OperationReceipt {
            operation: level::OperationIntent::SetLevel(request),
            receipt: command_receipt(context, session.generation()),
        })
    }
    async fn step(
        &self,
        _: receipt::RequestContext,
        _: level::StepRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Err(level_unsupported(level::Operation::Step))
    }
    async fn move_(
        &self,
        _: receipt::RequestContext,
        _: level::MoveRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Err(level_unsupported(level::Operation::Move))
    }
    async fn stop(
        &self,
        _: receipt::RequestContext,
    ) -> Result<level::OperationReceipt, level::Error> {
        Err(level_unsupported(level::Operation::Stop))
    }
    async fn set_level_with_on_off(
        &self,
        _: receipt::RequestContext,
        _: level::CoupledRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Err(level_unsupported(level::Operation::SetLevelWithOnOff))
    }
    async fn step_with_on_off(
        &self,
        _: receipt::RequestContext,
        _: level::CoupledStepRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Err(level_unsupported(level::Operation::StepWithOnOff))
    }
    async fn move_with_on_off(
        &self,
        _: receipt::RequestContext,
        _: level::CoupledMoveRequest,
    ) -> Result<level::OperationReceipt, level::Error> {
        Err(level_unsupported(level::Operation::MoveWithOnOff))
    }
}

impl provider::GuestColorControlFacet for ColorControlFacet {
    fn key(&self) -> String {
        self.locator.facet_key()
    }
    fn incarnation(&self) -> u64 {
        self.incarnation
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<color::ColorDescriptor, color::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .light(&self.locator, self.incarnation)
            .filter(|entity| has_color(entity))
            .ok_or_else(endpoint_removed_color)?;
        let (models, ranges) = color_models(entity);
        Ok(color::ColorDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                locator: &self.locator,
                incarnation: self.incarnation,
                generation: session.generation(),
                label: &entity.common.label,
                contract: "color-control",
                semantic_role: "light-color",
                semantic_profile: None,
                operations: &["query", "set-color"],
                events: &["state-observation"],
            }),
            models,
            operations: vec![color::Operation::SetColor],
            state_fields: vec![
                color::StateField::CurrentColor,
                color::StateField::ActiveModel,
            ],
            events: vec![color::EventKind::StateObservation],
            ranges,
            operation_support: vec![color::OperationSupport {
                operation: color::Operation::SetColor,
                kind: receipt::OperationKind::AbsoluteConvergent,
                deduplication: receipt::DeduplicationGuarantee::None,
                default_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
                maximum_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
            }],
        })
    }
    async fn query(&self, _: receipt::RequestContext) -> Result<color::State, color::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime.block_on(session.refresh());
        let entity = session
            .light(&self.locator, self.incarnation)
            .filter(|entity| has_color(entity))
            .ok_or_else(endpoint_removed_color)?;
        let current = entity
            .observation
            .current
            .as_ref()
            .and_then(|value| light_color_value(entity, value));
        let active_model = current.as_ref().map(color_model_of);
        Ok(color::State {
            meta: observation_meta(session.generation(), &entity.observation),
            availability: observation_availability(&session, &entity.observation),
            quality: current.as_ref().map(|_| state::Quality::SourceReported),
            current,
            target: None,
            active_model,
        })
    }
    async fn set_color(
        &self,
        context: receipt::RequestContext,
        request: color::SetRequest,
    ) -> Result<color::OperationReceipt, color::Error> {
        if request.transition.is_some() {
            return Err(color_contract_error(
                color::ContractErrorDetail::InvalidTransition,
            ));
        }
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .light(&self.locator, self.incarnation)
            .filter(|entity| has_color(entity))
            .ok_or_else(endpoint_removed_color)?;
        let command = color_command(entity, &request.color)?;
        drop(session);
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime
            .block_on(session.set_light_color(&self.locator, self.incarnation, command))
            .map_err(|error| color_session_error(error, &context))?;
        Ok(color::OperationReceipt {
            operation: color::OperationIntent::SetColor(request),
            receipt: command_receipt(context, session.generation()),
        })
    }
    async fn step_color(
        &self,
        _: receipt::RequestContext,
        _: color::StepRequest,
    ) -> Result<color::OperationReceipt, color::Error> {
        Err(color_unsupported(color::Operation::StepColor))
    }
    async fn move_color(
        &self,
        _: receipt::RequestContext,
        _: color::MoveRequest,
    ) -> Result<color::OperationReceipt, color::Error> {
        Err(color_unsupported(color::Operation::MoveColor))
    }
    async fn stop(
        &self,
        _: receipt::RequestContext,
    ) -> Result<color::OperationReceipt, color::Error> {
        Err(color_unsupported(color::Operation::Stop))
    }
}

impl provider::GuestThermostatFacet for ThermostatFacet {
    fn key(&self) -> String {
        self.locator.facet_key()
    }
    fn incarnation(&self) -> u64 {
        self.incarnation
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<thermostat::ThermostatDescriptor, thermostat::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .climate(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_thermostat)?;
        let mut features = Vec::new();
        let mut operations = vec![thermostat::Operation::SetMode];
        if entity.dual_target {
            operations.push(thermostat::Operation::SetDualTarget);
        } else {
            operations.push(thermostat::Operation::SetSingleTarget);
        }
        if !entity.fan_choices.is_empty() {
            features.push(thermostat::Feature::FanChoice);
            operations.push(thermostat::Operation::SetFan);
        }
        if !entity.swing_choices.is_empty() {
            features.push(thermostat::Feature::SwingChoice);
            operations.push(thermostat::Operation::SetSwing);
        }
        if !entity.preset_choices.is_empty() {
            features.push(thermostat::Feature::PresetChoice);
            operations.push(thermostat::Operation::SetPreset);
        }
        let mut common_operations = vec!["query", "set-mode"];
        common_operations.push(if entity.dual_target {
            "set-dual-target"
        } else {
            "set-single-target"
        });
        if !entity.fan_choices.is_empty() {
            common_operations.push("set-fan");
        }
        if !entity.swing_choices.is_empty() {
            common_operations.push("set-swing");
        }
        if !entity.preset_choices.is_empty() {
            common_operations.push("set-preset");
        }
        let operation_support = operations
            .iter()
            .copied()
            .map(|operation| thermostat::OperationSupport {
                operation,
                kind: receipt::OperationKind::AbsoluteConvergent,
                deduplication: receipt::DeduplicationGuarantee::None,
                default_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
                maximum_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
            })
            .collect();
        let minimum =
            source_temperature_to_kelvin(entity.minimum_temperature, entity.temperature_unit)
                .ok_or_else(endpoint_removed_thermostat)?;
        let maximum =
            source_temperature_to_kelvin(entity.maximum_temperature, entity.temperature_unit)
                .ok_or_else(endpoint_removed_thermostat)?;
        let step = temperature_step_kelvin(entity.target_step, entity.temperature_unit)
            .ok_or_else(endpoint_removed_thermostat)?;
        Ok(thermostat::ThermostatDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                locator: &self.locator,
                incarnation: self.incarnation,
                generation: session.generation(),
                label: &entity.common.label,
                contract: "thermostat",
                semantic_role: "climate",
                semantic_profile: Some(PROFILE_TEMPERATURE),
                operations: &common_operations,
                events: &["state-observation"],
            }),
            modes: entity
                .source_modes
                .iter()
                .filter_map(|mode| source_mode(*mode))
                .collect(),
            target_policy: thermostat::TargetPolicy {
                model: if entity.dual_target {
                    thermostat::SetpointModel::HeatCool
                } else {
                    thermostat::SetpointModel::Single
                },
                minimum: temperature_quantity(minimum),
                maximum: temperature_quantity(maximum),
                step: quantity::Quantity {
                    profile: PROFILE_TEMPERATURE_DIFFERENCE.into(),
                    magnitude: decimal(f64::from(step)),
                    unit: "K".into(),
                },
                minimum_deadband: None,
            },
            features,
            operations,
            state_fields: vec![
                thermostat::StateField::CurrentMode,
                thermostat::StateField::Action,
                thermostat::StateField::Running,
                thermostat::StateField::Targets,
                thermostat::StateField::Fan,
                thermostat::StateField::Swing,
                thermostat::StateField::Preset,
            ],
            events: vec![thermostat::EventKind::StateObservation],
            fan_choices: entity.fan_choices.keys().cloned().collect(),
            swing_choices: entity.swing_choices.keys().cloned().collect(),
            preset_choices: entity.preset_choices.keys().cloned().collect(),
            operation_support,
        })
    }
    async fn query(
        &self,
        _: receipt::RequestContext,
    ) -> Result<thermostat::State, thermostat::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime.block_on(session.refresh());
        let entity = session
            .climate(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_thermostat)?;
        let state_value = entity.observation.current.as_ref();
        let current_mode = state_value.and_then(|value| source_mode(value.mode));
        let current_action = state_value.and_then(|value| source_action(value.action));
        let running = state_value.map(|value| !matches!(value.action, 0 | 4));
        let targets = state_value.and_then(|value| climate_targets(entity, value));
        let fan = state_value.and_then(|value| {
            current_choice(&entity.fan_choices, value.fan_mode, &value.custom_fan_mode)
        });
        let swing = state_value
            .and_then(|value| current_choice(&entity.swing_choices, value.swing_mode, ""));
        let preset = state_value.and_then(|value| {
            current_choice(&entity.preset_choices, value.preset, &value.custom_preset)
        });
        Ok(thermostat::State {
            meta: observation_meta(session.generation(), &entity.observation),
            availability: observation_availability(&session, &entity.observation),
            quality: state_value.map(|_| state::Quality::SourceReported),
            current_mode,
            target_mode: None,
            current_action,
            running,
            targets,
            fan,
            swing,
            preset,
        })
    }
    async fn set_mode(
        &self,
        context: receipt::RequestContext,
        request: thermostat::SetModeRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .climate(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_thermostat)?;
        let mode = requested_mode(&request.mode).filter(|mode| entity.source_modes.contains(mode));
        let Some(mode) = mode else {
            return Err(thermostat_contract_error(
                thermostat::ContractErrorDetail::UnsupportedMode(request.mode),
            ));
        };
        drop(session);
        self.write_climate(
            context,
            request,
            ClimateWrite::Mode(mode),
            thermostat::OperationIntent::SetMode,
        )
    }
    async fn set_single_target(
        &self,
        context: receipt::RequestContext,
        request: thermostat::SetSingleTargetRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .climate(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_thermostat)?;
        if entity.dual_target {
            return Err(thermostat_contract_error(
                thermostat::ContractErrorDetail::WrongSetpointModel(
                    thermostat::SetpointModel::Single,
                ),
            ));
        }
        let source = validate_target(entity, &request.target.temperature)?;
        drop(session);
        self.write_climate(
            context,
            request,
            ClimateWrite::SingleTarget(source),
            thermostat::OperationIntent::SetSingleTarget,
        )
    }
    async fn set_dual_target(
        &self,
        context: receipt::RequestContext,
        request: thermostat::SetDualTargetRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .climate(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_thermostat)?;
        if !entity.dual_target {
            return Err(thermostat_contract_error(
                thermostat::ContractErrorDetail::WrongSetpointModel(
                    thermostat::SetpointModel::HeatCool,
                ),
            ));
        }
        let low = validate_target(entity, &request.target.heat)?;
        let high = validate_target(entity, &request.target.cool)?;
        if low > high {
            return Err(thermostat_contract_error(
                thermostat::ContractErrorDetail::InvertedRange,
            ));
        }
        drop(session);
        self.write_climate(
            context,
            request,
            ClimateWrite::DualTarget { low, high },
            thermostat::OperationIntent::SetDualTarget,
        )
    }
    async fn set_fan(
        &self,
        context: receipt::RequestContext,
        request: thermostat::SetChoiceRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        self.write_choice(
            context,
            request,
            |entity| &entity.fan_choices,
            ClimateWrite::Fan,
            thermostat::OperationIntent::SetFan,
        )
    }
    async fn set_swing(
        &self,
        context: receipt::RequestContext,
        request: thermostat::SetChoiceRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        self.write_choice(
            context,
            request,
            |entity| &entity.swing_choices,
            ClimateWrite::Swing,
            thermostat::OperationIntent::SetSwing,
        )
    }
    async fn set_preset(
        &self,
        context: receipt::RequestContext,
        request: thermostat::SetChoiceRequest,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        self.write_choice(
            context,
            request,
            |entity| &entity.preset_choices,
            ClimateWrite::Preset,
            thermostat::OperationIntent::SetPreset,
        )
    }
}

impl ThermostatFacet {
    #[allow(
        clippy::result_large_err,
        reason = "generated thermostat ABI owns the error layout"
    )]
    fn write_climate<T>(
        &self,
        context: receipt::RequestContext,
        request: T,
        write: ClimateWrite,
        intent: impl FnOnce(T) -> thermostat::OperationIntent,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime
            .block_on(session.write_climate(&self.locator, self.incarnation, write))
            .map_err(|error| thermostat_session_error(error, &context))?;
        Ok(thermostat::OperationReceipt {
            operation: intent(request),
            receipt: command_receipt(context, session.generation()),
        })
    }

    #[allow(
        clippy::result_large_err,
        reason = "generated thermostat ABI owns the error layout"
    )]
    fn write_choice(
        &self,
        context: receipt::RequestContext,
        request: thermostat::SetChoiceRequest,
        choices: impl FnOnce(&ClimateEntity) -> &std::collections::BTreeMap<String, ClimateChoice>,
        write: impl FnOnce(ClimateChoice) -> ClimateWrite,
        intent: impl FnOnce(thermostat::SetChoiceRequest) -> thermostat::OperationIntent,
    ) -> Result<thermostat::OperationReceipt, thermostat::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .climate(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_thermostat)?;
        let choice = choices(entity)
            .get(&request.choice)
            .cloned()
            .ok_or_else(|| {
                thermostat_contract_error(thermostat::ContractErrorDetail::UnsupportedChoice(
                    request.choice.clone(),
                ))
            })?;
        drop(session);
        self.write_climate(context, request, write(choice), intent)
    }
}

impl provider::GuestWindowCoveringFacet for WindowCoveringFacet {
    fn key(&self) -> String {
        self.locator.facet_key()
    }
    fn incarnation(&self) -> u64 {
        self.incarnation
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<covering::CoveringDescriptor, covering::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .cover(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_covering)?;
        let mut axes = vec![covering::AxisDescriptor {
            axis: covering::Axis::Lift,
            awareness: if entity.supports_position {
                covering::PositionAwareness::SourceReported
            } else {
                covering::PositionAwareness::Unaware
            },
            native_step: None,
        }];
        let mut features = vec![covering::Feature::Lift];
        if entity.supports_tilt {
            axes.push(covering::AxisDescriptor {
                axis: covering::Axis::Tilt,
                awareness: covering::PositionAwareness::SourceReported,
                native_step: None,
            });
            features.push(covering::Feature::Tilt);
        }
        let mut operations = vec![covering::Operation::Open, covering::Operation::Close];
        if entity.supports_position || entity.supports_tilt {
            features.push(covering::Feature::TargetPosition);
            operations.push(covering::Operation::SetPosition);
        }
        if entity.supports_stop {
            features.push(covering::Feature::Stop);
            operations.push(covering::Operation::Stop);
        }
        let operation_support = operations
            .iter()
            .copied()
            .map(|operation| covering::OperationSupport {
                operation,
                kind: receipt::OperationKind::AbsoluteConvergent,
                deduplication: receipt::DeduplicationGuarantee::None,
                default_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
                maximum_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
            })
            .collect();
        let common_operations = operations
            .iter()
            .map(|operation| match operation {
                covering::Operation::Open => "open",
                covering::Operation::Close => "close",
                covering::Operation::Stop => "stop",
                covering::Operation::SetPosition => "set-position",
            })
            .chain(std::iter::once("query"))
            .collect::<Vec<_>>();
        Ok(covering::CoveringDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                locator: &self.locator,
                incarnation: self.incarnation,
                generation: session.generation(),
                label: &entity.common.label,
                contract: "window-covering",
                semantic_role: "cover",
                semantic_profile: Some(PROFILE_RATIO),
                operations: &common_operations,
                events: &["state-observation"],
            }),
            axes,
            features,
            operations,
            state_fields: vec![
                covering::StateField::CurrentPosition,
                covering::StateField::Movement,
            ],
            events: vec![covering::EventKind::StateObservation],
            operation_support,
        })
    }
    async fn query(&self, _: receipt::RequestContext) -> Result<covering::State, covering::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime.block_on(session.refresh());
        let entity = session
            .cover(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_covering)?;
        let current = entity.observation.current.as_ref();
        let mut axes = vec![covering::AxisState {
            axis: covering::Axis::Lift,
            current: current
                .filter(|_| entity.supports_position)
                .and_then(|value| valid_ratio(value.position))
                .map(ratio_quantity),
            target: None,
            movement: current
                .map(|value| cover_movement(value.operation))
                .unwrap_or(covering::Movement::Unknown),
        }];
        if entity.supports_tilt {
            axes.push(covering::AxisState {
                axis: covering::Axis::Tilt,
                current: current
                    .and_then(|value| valid_ratio(value.tilt))
                    .map(ratio_quantity),
                target: None,
                movement: covering::Movement::Unknown,
            });
        }
        Ok(covering::State {
            meta: observation_meta(session.generation(), &entity.observation),
            availability: observation_availability(&session, &entity.observation),
            quality: current.map(|_| {
                if entity.assumed_state {
                    state::Quality::SourceAssumed
                } else {
                    state::Quality::SourceReported
                }
            }),
            axes,
            safety_status: vec![],
            fault_status: vec![],
        })
    }
    async fn open(
        &self,
        context: receipt::RequestContext,
        request: covering::AxisRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        let write = self.axis_write(request.axis, true)?;
        self.write_cover(context, request, write, covering::OperationIntent::Open)
    }
    async fn close(
        &self,
        context: receipt::RequestContext,
        request: covering::AxisRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        let write = self.axis_write(request.axis, false)?;
        self.write_cover(context, request, write, covering::OperationIntent::Close)
    }
    async fn stop(
        &self,
        context: receipt::RequestContext,
        request: covering::AxisRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .cover(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_covering)?;
        if !axis_supported(entity, request.axis) {
            return Err(covering_contract_error(
                covering::ContractErrorDetail::UnsupportedAxis(request.axis),
            ));
        }
        if !entity.supports_stop {
            return Err(covering_contract_error(
                covering::ContractErrorDetail::UnsupportedStop(request.axis),
            ));
        }
        drop(session);
        self.write_cover(
            context,
            request,
            CoverWrite::Stop,
            covering::OperationIntent::Stop,
        )
    }
    async fn set_position(
        &self,
        context: receipt::RequestContext,
        request: covering::PositionRequest,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .cover(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_covering)?;
        let supported = match request.axis {
            covering::Axis::Lift => entity.supports_position,
            covering::Axis::Tilt => entity.supports_tilt,
        };
        if !supported {
            return Err(covering_contract_error(
                covering::ContractErrorDetail::UnsupportedPosition(request.axis),
            ));
        }
        let position = parse_ratio(&request.position).ok_or_else(|| {
            covering_contract_error(covering::ContractErrorDetail::InvalidPosition(
                request.position.clone(),
            ))
        })?;
        let write = match request.axis {
            covering::Axis::Lift => CoverWrite::PositionLift(position),
            covering::Axis::Tilt => CoverWrite::PositionTilt(position),
        };
        drop(session);
        self.write_cover(
            context,
            request,
            write,
            covering::OperationIntent::SetPosition,
        )
    }
}

impl WindowCoveringFacet {
    #[allow(
        clippy::result_large_err,
        reason = "generated covering ABI owns the error layout"
    )]
    fn axis_write(&self, axis: covering::Axis, open: bool) -> Result<CoverWrite, covering::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .cover(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_covering)?;
        if !axis_supported(entity, axis) {
            return Err(covering_contract_error(
                covering::ContractErrorDetail::UnsupportedAxis(axis),
            ));
        }
        Ok(match (axis, open) {
            (covering::Axis::Lift, true) => CoverWrite::OpenLift,
            (covering::Axis::Lift, false) => CoverWrite::CloseLift,
            (covering::Axis::Tilt, true) => CoverWrite::OpenTilt,
            (covering::Axis::Tilt, false) => CoverWrite::CloseTilt,
        })
    }

    #[allow(
        clippy::result_large_err,
        reason = "generated covering ABI owns the error layout"
    )]
    fn write_cover<T>(
        &self,
        context: receipt::RequestContext,
        request: T,
        write: CoverWrite,
        intent: impl FnOnce(T) -> covering::OperationIntent,
    ) -> Result<covering::OperationReceipt, covering::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime
            .block_on(session.write_cover(&self.locator, self.incarnation, write))
            .map_err(|error| covering_session_error(error, &context))?;
        Ok(covering::OperationReceipt {
            operation: intent(request),
            receipt: command_receipt(context, session.generation()),
        })
    }
}

impl provider::GuestMomentaryActionFacet for MomentaryActionFacet {
    fn key(&self) -> String {
        self.locator.facet_key()
    }
    fn incarnation(&self) -> u64 {
        self.incarnation
    }
    async fn describe(
        &self,
        _: receipt::RequestContext,
    ) -> Result<momentary::ActionDescriptor, momentary::Error> {
        let session = self.runtime.block_on(self.session.lock());
        let entity = session
            .button(&self.locator, self.incarnation)
            .ok_or_else(endpoint_removed_momentary)?;
        Ok(momentary::ActionDescriptor {
            common: common_descriptor(CommonDescriptor {
                instance_id: &self.instance_id,
                locator: &self.locator,
                incarnation: self.incarnation,
                generation: session.generation(),
                label: &entity.common.label,
                contract: "momentary-action",
                semantic_role: "button",
                semantic_profile: Some(&entity.device_class),
                operations: &["invoke"],
                events: &[],
            }),
            action_id: self.locator.facet_key(),
            risk: button_risk(&entity.device_class),
            operation: momentary::OperationSupport {
                kind: receipt::OperationKind::Momentary,
                deduplication: receipt::DeduplicationGuarantee::None,
                default_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
                maximum_deadline_milliseconds: COMMAND_DEADLINE_MILLISECONDS,
            },
        })
    }
    async fn invoke(
        &self,
        context: receipt::RequestContext,
    ) -> Result<momentary::InvokeReceipt, momentary::Error> {
        let mut session = self.runtime.block_on(self.session.lock());
        self.runtime
            .block_on(session.invoke_button(&self.locator, self.incarnation))
            .map_err(|error| momentary_session_error(error, &context))?;
        Ok(momentary::InvokeReceipt {
            action_id: self.locator.facet_key(),
            receipt: command_receipt(context, session.generation()),
        })
    }
}

struct CommonDescriptor<'a> {
    instance_id: &'a str,
    locator: &'a Locator,
    incarnation: u64,
    generation: u64,
    label: &'a str,
    contract: &'a str,
    semantic_role: &'a str,
    semantic_profile: Option<&'a str>,
    operations: &'a [&'a str],
    events: &'a [&'a str],
}

fn common_descriptor(value: CommonDescriptor<'_>) -> descriptor::FacetDescriptor {
    let key = value.locator.facet_key();
    descriptor::FacetDescriptor {
        provider_instance_id: value.instance_id.into(),
        facet_key: key.clone(),
        incarnation: value.incarnation,
        generation: value.generation,
        contract: descriptor::PackageIdentity {
            namespace: "litehouse".into(),
            name: value.contract.into(),
            version: descriptor::Version {
                major: 1,
                minor: 0,
                patch: 0,
            },
        },
        primary_group_key: key,
        direction: descriptor::FacetDirection::Provided,
        label: Some(value.label.into()),
        semantic_role: value.semantic_role.into(),
        semantic_profile: value.semantic_profile.map(Into::into),
        features: vec![],
        operations: value
            .operations
            .iter()
            .map(|value| (*value).into())
            .collect(),
        events: value.events.iter().map(|value| (*value).into()).collect(),
        tags: vec![],
    }
}

fn command_receipt(context: receipt::RequestContext, generation: u64) -> receipt::CommandReceipt {
    receipt::CommandReceipt {
        request_id: context.request_id,
        attempt: context.attempt,
        provider_generation: generation,
        confidence: receipt::ReceiptConfidence::AcceptedForDelivery,
        accepted_at_unix_milliseconds: now_unix_milliseconds(),
        completed_at_unix_milliseconds: None,
        idempotency_key: context.idempotency.map(|value| value.key),
    }
}

fn observation_meta<T>(generation: u64, observation: &Observation<T>) -> state::ObservationMeta {
    state::ObservationMeta {
        generation,
        revision: observation.revision,
        received_at: state::WallClockInstant {
            unix_milliseconds: observation.received_at_unix_milliseconds,
        },
        source_time: None,
    }
}

fn observation_availability<T>(
    session: &NodeSession,
    observation: &Observation<T>,
) -> state::Availability {
    if !session.connected() {
        if observation.current.is_some() {
            return state::Availability::Stale(state::StaleStatus {
                since: state::WallClockInstant {
                    unix_milliseconds: session
                        .disconnected_at()
                        .unwrap_or(observation.received_at_unix_milliseconds),
                },
            });
        }
        return state::Availability::Unavailable(state::UnavailabilityReason::Disconnected);
    }
    match observation.status {
        HydrationStatus::Hydrated => state::Availability::Available,
        HydrationStatus::Gap if observation.current.is_some() => {
            state::Availability::Stale(state::StaleStatus {
                since: state::WallClockInstant {
                    unix_milliseconds: observation
                        .stale_since_unix_milliseconds
                        .unwrap_or(observation.received_at_unix_milliseconds),
                },
            })
        }
        HydrationStatus::Gap | HydrationStatus::Unhydrated => {
            state::Availability::Unavailable(state::UnavailabilityReason::Reconnecting)
        }
    }
}

fn projected_meta<T>(
    generation: u64,
    observation: &ProjectedObservation<T>,
) -> state::ObservationMeta {
    state::ObservationMeta {
        generation,
        revision: observation.revision,
        received_at: state::WallClockInstant {
            unix_milliseconds: observation.received_at_unix_milliseconds,
        },
        source_time: None,
    }
}

fn projected_availability<T>(
    session: &NodeSession,
    observation: &ProjectedObservation<T>,
) -> state::Availability {
    if !session.connected() {
        if observation.current.is_some() {
            return state::Availability::Stale(state::StaleStatus {
                since: state::WallClockInstant {
                    unix_milliseconds: session
                        .disconnected_at()
                        .unwrap_or(observation.received_at_unix_milliseconds),
                },
            });
        }
        return state::Availability::Unavailable(state::UnavailabilityReason::Disconnected);
    }
    match observation.status {
        HydrationStatus::Hydrated => state::Availability::Available,
        HydrationStatus::Gap if observation.current.is_some() => {
            state::Availability::Stale(state::StaleStatus {
                since: state::WallClockInstant {
                    unix_milliseconds: observation
                        .stale_since_unix_milliseconds
                        .unwrap_or(observation.received_at_unix_milliseconds),
                },
            })
        }
        HydrationStatus::Gap | HydrationStatus::Unhydrated => {
            state::Availability::Unavailable(state::UnavailabilityReason::Reconnecting)
        }
    }
}

struct OptionalObservation<T> {
    current: Option<T>,
    revision: u64,
    received_at: u64,
    status: HydrationStatus,
    stale_since: Option<u64>,
}

impl<T> OptionalObservation<T> {
    fn meta(&self, generation: u64) -> state::ObservationMeta {
        state::ObservationMeta {
            generation,
            revision: self.revision,
            received_at: state::WallClockInstant {
                unix_milliseconds: self.received_at,
            },
            source_time: None,
        }
    }

    fn availability(&self, session: &NodeSession) -> state::Availability {
        if !session.connected() {
            return if self.current.is_some() {
                state::Availability::Stale(state::StaleStatus {
                    since: state::WallClockInstant {
                        unix_milliseconds: session.disconnected_at().unwrap_or(self.received_at),
                    },
                })
            } else {
                state::Availability::Unavailable(state::UnavailabilityReason::Disconnected)
            };
        }
        match self.status {
            HydrationStatus::Hydrated => state::Availability::Available,
            HydrationStatus::Gap if self.current.is_some() => {
                state::Availability::Stale(state::StaleStatus {
                    since: state::WallClockInstant {
                        unix_milliseconds: self.stale_since.unwrap_or(self.received_at),
                    },
                })
            }
            HydrationStatus::Gap | HydrationStatus::Unhydrated => {
                state::Availability::Unavailable(state::UnavailabilityReason::Reconnecting)
            }
        }
    }
}

fn project_observation<T, U>(
    observation: &Observation<T>,
    project: impl FnOnce(&T) -> U,
) -> OptionalObservation<U> {
    OptionalObservation {
        current: observation.current.as_ref().map(project),
        revision: observation.revision,
        received_at: observation.received_at_unix_milliseconds,
        status: observation.status,
        stale_since: observation.stale_since_unix_milliseconds,
    }
}

fn ratio_quantity(value: f32) -> quantity::Quantity {
    quantity::Quantity {
        profile: PROFILE_RATIO.into(),
        magnitude: decimal(f64::from(value)),
        unit: "1".into(),
    }
}

fn temperature_quantity(value: f32) -> quantity::Quantity {
    quantity::Quantity {
        profile: PROFILE_TEMPERATURE.into(),
        magnitude: decimal(f64::from(value)),
        unit: "K".into(),
    }
}

fn valid_ratio(value: f32) -> Option<f32> {
    (value.is_finite() && (0.0..=1.0).contains(&value)).then_some(value)
}

fn parse_ratio(value: &quantity::Quantity) -> Option<f32> {
    if value.profile != PROFILE_RATIO || value.unit != "1" {
        return None;
    }
    valid_ratio(value.magnitude.parse().ok()?)
}

fn parse_temperature(value: &quantity::Quantity) -> Option<f32> {
    if value.profile != PROFILE_TEMPERATURE || value.unit != "K" {
        return None;
    }
    let value = value.magnitude.parse::<f32>().ok()?;
    value.is_finite().then_some(value)
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

fn boolean_profile_name(profile: boolean::BooleanProfile) -> &'static str {
    match profile {
        boolean::BooleanProfile::Contact => "contact",
        boolean::BooleanProfile::Occupancy => "occupancy",
        boolean::BooleanProfile::Smoke => "smoke",
        boolean::BooleanProfile::Status => "status",
        boolean::BooleanProfile::Moisture => "moisture",
        boolean::BooleanProfile::Vibration => "vibration",
        boolean::BooleanProfile::Generic => "generic",
    }
}

fn has_color(entity: &LightEntity) -> bool {
    entity.rgb_mode.is_some()
        || entity.color_temperature_mode.is_some()
        || entity.white_mode.is_some()
}

fn color_models(entity: &LightEntity) -> (Vec<color::ColorModel>, Vec<color::ModelRange>) {
    let mut models = Vec::new();
    let mut ranges = Vec::new();
    if entity.rgb_mode.is_some() {
        models.push(color::ColorModel::UncalibratedRgb);
        ranges.push(color::ModelRange {
            model: color::ColorModel::UncalibratedRgb,
            minimum: vec!["0".into(), "0".into(), "0".into()],
            maximum: vec!["1".into(), "1".into(), "1".into()],
            gamut: None,
            native_step: None,
        });
    }
    if entity.color_temperature_mode.is_some() {
        models.push(color::ColorModel::ColorTemperature);
        ranges.push(color::ModelRange {
            model: color::ColorModel::ColorTemperature,
            minimum: vec![decimal(f64::from(1_000_000.0 / entity.maximum_mireds))],
            maximum: vec![decimal(f64::from(1_000_000.0 / entity.minimum_mireds))],
            gamut: None,
            native_step: None,
        });
    }
    if entity.white_mode.is_some() {
        models.push(color::ColorModel::NativeWhiteChannels);
        ranges.push(color::ModelRange {
            model: color::ColorModel::NativeWhiteChannels,
            minimum: vec!["0".into(), "0".into(), "0".into()],
            maximum: vec!["1".into(), "1".into(), "1".into()],
            gamut: None,
            native_step: None,
        });
    }
    (models, ranges)
}

fn light_color_value(
    entity: &LightEntity,
    value: &session::LightState,
) -> Option<color::ColorValue> {
    if entity.rgb_mode == Some(value.color_mode) {
        return Some(color::ColorValue::UncalibratedRgb(color::RgbChannels {
            red: decimal(f64::from(valid_ratio(value.red)?)),
            green: decimal(f64::from(valid_ratio(value.green)?)),
            blue: decimal(f64::from(valid_ratio(value.blue)?)),
        }));
    }
    if entity.color_temperature_mode == Some(value.color_mode) {
        let mireds = value.color_temperature_mireds;
        if !mireds.is_finite() || mireds < entity.minimum_mireds || mireds > entity.maximum_mireds {
            return None;
        }
        return Some(color::ColorValue::ColorTemperature(
            color::ColorTemperature {
                kelvin: decimal(f64::from(1_000_000.0 / mireds)),
            },
        ));
    }
    if entity.white_mode == Some(value.color_mode) {
        let channels = if value.color_mode == 7 {
            color::WhiteChannels {
                white: Some(decimal(f64::from(valid_ratio(value.white)?))),
                cold_white: None,
                warm_white: None,
            }
        } else {
            color::WhiteChannels {
                white: None,
                cold_white: Some(decimal(f64::from(valid_ratio(value.cold_white)?))),
                warm_white: Some(decimal(f64::from(valid_ratio(value.warm_white)?))),
            }
        };
        return Some(color::ColorValue::NativeWhiteChannels(channels));
    }
    None
}

fn color_model_of(value: &color::ColorValue) -> color::ColorModel {
    match value {
        color::ColorValue::HueSaturation(_) => color::ColorModel::HueSaturation,
        color::ColorValue::CieXy(_) => color::ColorModel::CieXy,
        color::ColorValue::ColorTemperature(_) => color::ColorModel::ColorTemperature,
        color::ColorValue::CalibratedRgb(_) => color::ColorModel::CalibratedRgb,
        color::ColorValue::UncalibratedRgb(_) => color::ColorModel::UncalibratedRgb,
        color::ColorValue::NativeWhiteChannels(_) => color::ColorModel::NativeWhiteChannels,
    }
}

#[allow(
    clippy::result_large_err,
    reason = "generated color ABI owns the error layout"
)]
fn color_command(
    entity: &LightEntity,
    value: &color::ColorValue,
) -> Result<LightColorCommand, color::Error> {
    match value {
        color::ColorValue::UncalibratedRgb(channels) if entity.rgb_mode.is_some() => {
            Ok(LightColorCommand::Rgb {
                red: parse_color_ratio(&channels.red, color::ColorModel::UncalibratedRgb)?,
                green: parse_color_ratio(&channels.green, color::ColorModel::UncalibratedRgb)?,
                blue: parse_color_ratio(&channels.blue, color::ColorModel::UncalibratedRgb)?,
            })
        }
        color::ColorValue::ColorTemperature(temperature)
            if entity.color_temperature_mode.is_some() =>
        {
            let kelvin = temperature
                .kelvin
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    color_contract_error(color::ContractErrorDetail::OutOfRange(
                        color::ColorModel::ColorTemperature,
                    ))
                })?;
            let minimum = 1_000_000.0 / entity.maximum_mireds;
            let maximum = 1_000_000.0 / entity.minimum_mireds;
            if !(minimum..=maximum).contains(&kelvin) {
                return Err(color_contract_error(
                    color::ContractErrorDetail::OutOfRange(color::ColorModel::ColorTemperature),
                ));
            }
            Ok(LightColorCommand::Temperature { kelvin })
        }
        color::ColorValue::NativeWhiteChannels(channels) if entity.white_mode.is_some() => {
            let parse = |value: &Option<String>| -> Result<f32, color::Error> {
                value
                    .as_ref()
                    .map(|value| parse_color_ratio(value, color::ColorModel::NativeWhiteChannels))
                    .transpose()
                    .map(|value| value.unwrap_or(0.0))
            };
            Ok(LightColorCommand::White {
                white: parse(&channels.white)?,
                cold_white: parse(&channels.cold_white)?,
                warm_white: parse(&channels.warm_white)?,
            })
        }
        _ => Err(color_contract_error(
            color::ContractErrorDetail::UnsupportedModel(color_model_of(value)),
        )),
    }
}

#[allow(
    clippy::result_large_err,
    reason = "generated color ABI owns the error layout"
)]
fn parse_color_ratio(value: &str, model: color::ColorModel) -> Result<f32, color::Error> {
    value
        .parse::<f32>()
        .ok()
        .and_then(valid_ratio)
        .ok_or_else(|| color_contract_error(color::ContractErrorDetail::OutOfRange(model)))
}

fn source_mode(mode: i32) -> Option<thermostat::Mode> {
    Some(match mode {
        0 => thermostat::Mode::Known(thermostat::KnownMode::Off),
        1 => thermostat::Mode::ProviderQualified("esphome:heat-cool".into()),
        2 => thermostat::Mode::Known(thermostat::KnownMode::Cool),
        3 => thermostat::Mode::Known(thermostat::KnownMode::Heat),
        4 => thermostat::Mode::Known(thermostat::KnownMode::FanOnly),
        5 => thermostat::Mode::Known(thermostat::KnownMode::Dry),
        6 => thermostat::Mode::Known(thermostat::KnownMode::Auto),
        _ => return None,
    })
}

fn requested_mode(mode: &thermostat::Mode) -> Option<i32> {
    match mode {
        thermostat::Mode::Known(thermostat::KnownMode::Off) => Some(0),
        thermostat::Mode::Known(thermostat::KnownMode::Cool) => Some(2),
        thermostat::Mode::Known(thermostat::KnownMode::Heat) => Some(3),
        thermostat::Mode::Known(thermostat::KnownMode::FanOnly) => Some(4),
        thermostat::Mode::Known(thermostat::KnownMode::Dry) => Some(5),
        thermostat::Mode::Known(thermostat::KnownMode::Auto) => Some(6),
        thermostat::Mode::ProviderQualified(value) if value == "esphome:heat-cool" => Some(1),
        thermostat::Mode::ProviderQualified(_) => None,
    }
}

fn source_action(action: i32) -> Option<thermostat::Action> {
    match action {
        0 => None,
        2 => Some(thermostat::Action::Cooling),
        3 => Some(thermostat::Action::Heating),
        4 => Some(thermostat::Action::Idle),
        5 => Some(thermostat::Action::Drying),
        6 => Some(thermostat::Action::Fan),
        7 => Some(thermostat::Action::Defrosting),
        _ => None,
    }
}

fn climate_targets(
    entity: &ClimateEntity,
    value: &session::ClimateState,
) -> Option<thermostat::Targets> {
    if entity.dual_target {
        Some(thermostat::Targets::HeatCool(thermostat::DualTarget {
            heat: temperature_quantity(source_temperature_to_kelvin(
                value.target_temperature_low,
                entity.temperature_unit,
            )?),
            cool: temperature_quantity(source_temperature_to_kelvin(
                value.target_temperature_high,
                entity.temperature_unit,
            )?),
        }))
    } else {
        Some(thermostat::Targets::Single(thermostat::SingleTarget {
            temperature: temperature_quantity(source_temperature_to_kelvin(
                value.target_temperature,
                entity.temperature_unit,
            )?),
        }))
    }
}

fn current_choice(
    choices: &std::collections::BTreeMap<String, ClimateChoice>,
    standard: i32,
    custom: &str,
) -> Option<String> {
    choices.iter().find_map(|(name, choice)| match choice {
        ClimateChoice::Standard(value) if *value == standard && custom.is_empty() => {
            Some(name.clone())
        }
        ClimateChoice::Custom(value) if value == custom => Some(name.clone()),
        _ => None,
    })
}

#[allow(
    clippy::result_large_err,
    reason = "generated thermostat ABI owns the error layout"
)]
fn validate_target(
    entity: &ClimateEntity,
    value: &quantity::Quantity,
) -> Result<f32, thermostat::Error> {
    let kelvin = parse_temperature(value).ok_or_else(|| {
        thermostat_contract_error(thermostat::ContractErrorDetail::OutOfRangeTarget(
            value.clone(),
        ))
    })?;
    let source =
        kelvin_to_source_temperature(kelvin, entity.temperature_unit).ok_or_else(|| {
            thermostat_contract_error(thermostat::ContractErrorDetail::OutOfRangeTarget(
                value.clone(),
            ))
        })?;
    if source < entity.minimum_temperature || source > entity.maximum_temperature {
        return Err(thermostat_contract_error(
            thermostat::ContractErrorDetail::OutOfRangeTarget(value.clone()),
        ));
    }
    Ok(source)
}

fn temperature_step_kelvin(value: f32, unit: i32) -> Option<f32> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    match unit {
        0 | 2 => Some(value),
        1 => Some(value * (5.0 / 9.0)),
        _ => None,
    }
}

fn axis_supported(entity: &CoverEntity, axis: covering::Axis) -> bool {
    match axis {
        covering::Axis::Lift => true,
        covering::Axis::Tilt => entity.supports_tilt,
    }
}

fn cover_movement(value: i32) -> covering::Movement {
    match value {
        0 => covering::Movement::Idle,
        1 => covering::Movement::Opening,
        2 => covering::Movement::Closing,
        _ => covering::Movement::Unknown,
    }
}

fn button_risk(device_class: &str) -> momentary::Risk {
    match device_class {
        "factory_reset" | "safe_mode" | "shutdown" => momentary::Risk::Dangerous,
        "restart" | "update" => momentary::Risk::Caution,
        _ => momentary::Risk::Routine,
    }
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

fn core_command_error(
    error: SessionError,
    context: &receipt::RequestContext,
) -> (errors::CoreError, errors::RetryAdvice) {
    if error == SessionError::OutcomeUnknown {
        return (
            errors::CoreError::OutcomeUnknown(errors::OutcomeUnknownError {
                cause: errors::UnknownOutcomeCause::Disconnected,
                request_id: context.request_id.clone(),
                attempt: context.attempt,
                idempotency_key: context.idempotency.as_ref().map(|value| value.key.clone()),
            }),
            errors::RetryAdvice::Never,
        );
    }
    session_core_error(error)
}

macro_rules! session_error {
    ($name:ident, $module:ident) => {
        fn $name(error: SessionError, context: &receipt::RequestContext) -> $module::Error {
            let (inner, retry) = core_command_error(error, context);
            $module::Error {
                inner,
                detail: None,
                retry,
                message: None,
                error_id: next_error_id(),
            }
        }
    };
}
session_error!(on_off_session_error, on_off);
session_error!(level_session_error, level);
session_error!(color_session_error, color);
session_error!(thermostat_session_error, thermostat);
session_error!(covering_session_error, covering);
session_error!(momentary_session_error, momentary);

fn on_off_unsupported_feature(feature: on_off::Feature) -> on_off::Error {
    on_off::Error {
        inner: errors::CoreError::ContractError,
        detail: Some(on_off::ContractErrorDetail::UnsupportedFeature(feature)),
        retry: errors::RetryAdvice::Never,
        message: None,
        error_id: next_error_id(),
    }
}

fn level_contract_error(detail: level::ContractErrorDetail) -> level::Error {
    level::Error {
        inner: errors::CoreError::ContractError,
        detail: Some(detail),
        retry: errors::RetryAdvice::Never,
        message: None,
        error_id: next_error_id(),
    }
}
fn level_unsupported(operation: level::Operation) -> level::Error {
    level_contract_error(level::ContractErrorDetail::UnsupportedOperation(operation))
}
fn color_contract_error(detail: color::ContractErrorDetail) -> color::Error {
    color::Error {
        inner: errors::CoreError::ContractError,
        detail: Some(detail),
        retry: errors::RetryAdvice::Never,
        message: None,
        error_id: next_error_id(),
    }
}
fn color_unsupported(operation: color::Operation) -> color::Error {
    color_contract_error(color::ContractErrorDetail::UnsupportedOperation(operation))
}
fn thermostat_contract_error(detail: thermostat::ContractErrorDetail) -> thermostat::Error {
    thermostat::Error {
        inner: errors::CoreError::ContractError,
        detail: Some(detail),
        retry: errors::RetryAdvice::Never,
        message: None,
        error_id: next_error_id(),
    }
}
fn covering_contract_error(detail: covering::ContractErrorDetail) -> covering::Error {
    covering::Error {
        inner: errors::CoreError::ContractError,
        detail: Some(detail),
        retry: errors::RetryAdvice::Never,
        message: None,
        error_id: next_error_id(),
    }
}

macro_rules! endpoint_removed {
    ($name:ident, $module:ident) => {
        fn $name() -> $module::Error {
            $module::Error {
                inner: errors::CoreError::Binding(errors::BindingError::EndpointRemoved),
                detail: None,
                retry: errors::RetryAdvice::Rebind,
                message: None,
                error_id: next_error_id(),
            }
        }
    };
}
endpoint_removed!(endpoint_removed_on_off, on_off);
endpoint_removed!(endpoint_removed_boolean, boolean);
endpoint_removed!(endpoint_removed_measurement, measurement);
endpoint_removed!(endpoint_removed_level, level);
endpoint_removed!(endpoint_removed_color, color);
endpoint_removed!(endpoint_removed_thermostat, thermostat);
endpoint_removed!(endpoint_removed_covering, covering);
endpoint_removed!(endpoint_removed_momentary, momentary);

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

    fn context() -> receipt::RequestContext {
        receipt::RequestContext {
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
        }
    }

    #[test]
    fn descriptors_group_facets_without_toggle() {
        let locator = Locator::new(EntityKind::Light, 1, 9);
        let descriptor = common_descriptor(CommonDescriptor {
            instance_id: "node",
            locator: &locator,
            incarnation: 2,
            generation: 3,
            label: "Lamp",
            contract: "on-off",
            semantic_role: "light",
            semantic_profile: None,
            operations: &["query", "set"],
            events: &["state-observation"],
        });
        assert_eq!(descriptor.facet_key, "light-1-9");
        assert_eq!(descriptor.primary_group_key, "light-1-9");
        assert_eq!(descriptor.operations, ["query", "set"]);
        assert!(
            !descriptor
                .operations
                .iter()
                .any(|operation| operation == "toggle")
        );
        assert_eq!(descriptor.incarnation, 2);
        assert_eq!(descriptor.generation, 3);
    }

    #[test]
    fn possible_dispatch_failure_is_outcome_unknown_without_retry_for_every_command_family() {
        let context = context();
        let on_off = on_off_session_error(SessionError::OutcomeUnknown, &context);
        let level = level_session_error(SessionError::OutcomeUnknown, &context);
        let color = color_session_error(SessionError::OutcomeUnknown, &context);
        let thermostat = thermostat_session_error(SessionError::OutcomeUnknown, &context);
        let covering = covering_session_error(SessionError::OutcomeUnknown, &context);
        let momentary = momentary_session_error(SessionError::OutcomeUnknown, &context);
        for (inner, retry) in [
            (on_off.inner, on_off.retry),
            (level.inner, level.retry),
            (color.inner, color.retry),
            (thermostat.inner, thermostat.retry),
            (covering.inner, covering.retry),
            (momentary.inner, momentary.retry),
        ] {
            assert!(matches!(inner, errors::CoreError::OutcomeUnknown(_)));
            assert!(matches!(retry, errors::RetryAdvice::Never));
        }
    }

    #[test]
    fn absolute_quantity_validation_is_closed() {
        assert_eq!(
            parse_ratio(&quantity::Quantity {
                profile: PROFILE_RATIO.into(),
                magnitude: "0.5".into(),
                unit: "1".into(),
            }),
            Some(0.5)
        );
        assert!(
            parse_ratio(&quantity::Quantity {
                profile: "invented".into(),
                magnitude: "0.5".into(),
                unit: "1".into(),
            })
            .is_none()
        );
        assert!(
            parse_temperature(&quantity::Quantity {
                profile: PROFILE_TEMPERATURE.into(),
                magnitude: "nan".into(),
                unit: "K".into(),
            })
            .is_none()
        );
    }

    #[test]
    fn provider_qualified_heat_cool_does_not_alias_auto() {
        assert!(matches!(
            source_mode(1),
            Some(thermostat::Mode::ProviderQualified(_))
        ));
        assert_eq!(
            requested_mode(&thermostat::Mode::ProviderQualified(
                "esphome:heat-cool".into()
            )),
            Some(1)
        );
        assert_eq!(
            requested_mode(&thermostat::Mode::Known(thermostat::KnownMode::Auto)),
            Some(6)
        );
    }

    #[test]
    fn buttons_get_conservative_risk() {
        assert!(matches!(button_risk("restart"), momentary::Risk::Caution));
        assert!(matches!(
            button_risk("factory_reset"),
            momentary::Risk::Dangerous
        ));
        assert!(matches!(button_risk("identify"), momentary::Risk::Routine));
    }
}
