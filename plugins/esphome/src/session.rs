use base64::Engine as _;
use esphome_client::{
    EspHomeClient,
    types::{
        ButtonCommandRequest, ClimateCommandRequest, CoverCommandRequest, EspHomeMessage,
        FanCommandRequest, LightCommandRequest, ListEntitiesBinarySensorResponse,
        ListEntitiesButtonResponse, ListEntitiesClimateResponse, ListEntitiesCoverResponse,
        ListEntitiesFanResponse, ListEntitiesLightResponse, ListEntitiesRequest,
        ListEntitiesSensorResponse, ListEntitiesSirenResponse, ListEntitiesSwitchResponse,
        SensorStateClass, SirenCommandRequest, SubscribeStatesRequest, SwitchCommandRequest,
        TemperatureUnit,
    },
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zeroize::{Zeroize, Zeroizing};

const CLIENT_INFO: &str = "litehouse-esphome/0.1";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(10);
const INITIAL_STATE_GRACE: Duration = Duration::from_millis(500);
const REFRESH_WINDOW: Duration = Duration::from_millis(250);
const MAX_LABEL_BYTES: usize = 128;
const MAX_CHOICES: usize = 64;
const MAX_CHOICE_BYTES: usize = 128;
const MAX_MAPPED_ENTITIES: usize = 512;
const MAX_DIAGNOSTICS: usize = 128;
const MAX_BOOLEAN_OCCURRENCES: usize = 256;

pub(crate) const PROFILE_TEMPERATURE: &str = "litehouse:quantity/temperature-absolute@1";
pub(crate) const PROFILE_RATIO: &str = "litehouse:quantity/ratio@1";
const PROFILE_HUMIDITY: &str = "litehouse:quantity/relative-humidity@1";
const PROFILE_PRESSURE: &str = "litehouse:quantity/pressure@1";
const PROFILE_ILLUMINANCE: &str = "litehouse:quantity/illuminance@1";
const PROFILE_POWER: &str = "litehouse:quantity/power@1";
const PROFILE_ENERGY: &str = "litehouse:quantity/energy@1";
const PROFILE_VOLTAGE: &str = "litehouse:quantity/voltage@1";
const PROFILE_CURRENT: &str = "litehouse:quantity/electric-current@1";
const PROFILE_FREQUENCY: &str = "litehouse:quantity/frequency@1";
const PROFILE_BATTERY: &str = "litehouse:quantity/battery-ratio@1";
const PROFILE_SIGNAL: &str = "litehouse:quantity/signal-strength@1";
const PROFILE_CO2: &str = "litehouse:quantity/carbon-dioxide@1";
const PROFILE_CO: &str = "litehouse:quantity/carbon-monoxide@1";
const PROFILE_VOC: &str = "litehouse:quantity/volatile-organic-compounds@1";
const PROFILE_PM1: &str = "litehouse:quantity/particulate-matter-pm1@1";
const PROFILE_PM2_5: &str = "litehouse:quantity/particulate-matter-pm2-5@1";
const PROFILE_PM10: &str = "litehouse:quantity/particulate-matter-pm10@1";
const PROFILE_DISTANCE: &str = "litehouse:quantity/distance@1";
const PROFILE_SPEED: &str = "litehouse:quantity/speed@1";
const PROFILE_VOLUME: &str = "litehouse:quantity/volume@1";
const PROFILE_FLOW: &str = "litehouse:quantity/volumetric-flow@1";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectorConfig {
    address: String,
    noise_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum EntityKind {
    Switch,
    BinarySensor,
    Sensor,
    Light,
    Fan,
    Siren,
    Climate,
    Cover,
    Button,
}

impl EntityKind {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Switch => "switch",
            Self::BinarySensor => "binary",
            Self::Sensor => "sensor",
            Self::Light => "light",
            Self::Fan => "fan",
            Self::Siren => "siren",
            Self::Climate => "climate",
            Self::Cover => "cover",
            Self::Button => "button",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Locator {
    pub(crate) kind: EntityKind,
    pub(crate) device_id: u32,
    pub(crate) key: u32,
}

impl Locator {
    pub(crate) const fn new(kind: EntityKind, device_id: u32, key: u32) -> Self {
        Self {
            kind,
            device_id,
            key,
        }
    }

    pub(crate) fn facet_key(&self) -> String {
        format!("{}-{}-{}", self.kind.prefix(), self.device_id, self.key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FacetSeed {
    pub(crate) locator: Locator,
    pub(crate) incarnation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HydrationStatus {
    Unhydrated,
    Hydrated,
    Gap,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Observation<T> {
    pub(crate) current: Option<T>,
    pub(crate) revision: u64,
    pub(crate) received_at_unix_milliseconds: u64,
    pub(crate) status: HydrationStatus,
    pub(crate) stale_since_unix_milliseconds: Option<u64>,
}

impl<T: Clone> Observation<T> {
    fn empty() -> Self {
        Self {
            current: None,
            revision: 0,
            received_at_unix_milliseconds: now_unix_milliseconds(),
            status: HydrationStatus::Unhydrated,
            stale_since_unix_milliseconds: None,
        }
    }

    fn observe(&mut self, value: T) {
        self.current = Some(value);
        self.revision = self.revision.saturating_add(1).max(1);
        self.received_at_unix_milliseconds = now_unix_milliseconds();
        self.status = HydrationStatus::Hydrated;
        self.stale_since_unix_milliseconds = None;
    }

    fn gap(&mut self) {
        self.revision = self.revision.saturating_add(1).max(1);
        self.received_at_unix_milliseconds = now_unix_milliseconds();
        self.status = HydrationStatus::Gap;
        self.stale_since_unix_milliseconds
            .get_or_insert(self.received_at_unix_milliseconds);
    }

    fn reconcile(&mut self, previous: &Self) {
        match self.status {
            HydrationStatus::Hydrated => {
                self.revision = previous.revision.saturating_add(self.revision).max(1);
            }
            HydrationStatus::Unhydrated | HydrationStatus::Gap => {
                if previous.current.is_some() {
                    self.current.clone_from(&previous.current);
                    self.revision = previous.revision.saturating_add(1).max(1);
                    self.received_at_unix_milliseconds = previous.received_at_unix_milliseconds;
                    self.status = HydrationStatus::Gap;
                    self.stale_since_unix_milliseconds
                        .get_or_insert_with(now_unix_milliseconds);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EntityCommon {
    pub(crate) locator: Locator,
    pub(crate) label: String,
    pub(crate) incarnation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BooleanProfile {
    Contact,
    Occupancy,
    Smoke,
    Status,
    Moisture,
    Vibration,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MeasurementKind {
    Instantaneous,
    Total,
    TotalIncreasing,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MeasurementValue {
    pub(crate) canonical_magnitude: String,
    pub(crate) native_magnitude: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LightState {
    pub(crate) on: bool,
    pub(crate) brightness: f32,
    pub(crate) color_mode: i32,
    pub(crate) red: f32,
    pub(crate) green: f32,
    pub(crate) blue: f32,
    pub(crate) white: f32,
    pub(crate) color_temperature_mireds: f32,
    pub(crate) cold_white: f32,
    pub(crate) warm_white: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClimateChoice {
    Standard(i32),
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ClimateState {
    pub(crate) mode: i32,
    pub(crate) target_temperature: f32,
    pub(crate) target_temperature_low: f32,
    pub(crate) target_temperature_high: f32,
    pub(crate) action: i32,
    pub(crate) fan_mode: i32,
    pub(crate) swing_mode: i32,
    pub(crate) custom_fan_mode: String,
    pub(crate) preset: i32,
    pub(crate) custom_preset: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CoverState {
    pub(crate) position: f32,
    pub(crate) tilt: f32,
    pub(crate) operation: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct SwitchEntity {
    pub(crate) common: EntityCommon,
    pub(crate) assumed_state: bool,
    pub(crate) observation: Observation<bool>,
}

#[derive(Debug, Clone)]
pub(crate) struct BinaryEntity {
    pub(crate) common: EntityCommon,
    pub(crate) profile: BooleanProfile,
    pub(crate) observation: Observation<bool>,
}

#[derive(Debug, Clone)]
pub(crate) struct SensorEntity {
    pub(crate) common: EntityCommon,
    pub(crate) profile: &'static str,
    pub(crate) canonical_unit: &'static str,
    pub(crate) native_unit: String,
    pub(crate) scale: f64,
    pub(crate) offset: f64,
    pub(crate) kind: MeasurementKind,
    pub(crate) accuracy_decimals: Option<u32>,
    pub(crate) observation: Observation<MeasurementValue>,
}

#[derive(Debug, Clone)]
pub(crate) struct LightEntity {
    pub(crate) common: EntityCommon,
    pub(crate) supports_level: bool,
    pub(crate) rgb_mode: Option<i32>,
    pub(crate) color_temperature_mode: Option<i32>,
    pub(crate) white_mode: Option<i32>,
    pub(crate) minimum_mireds: f32,
    pub(crate) maximum_mireds: f32,
    pub(crate) observation: Observation<LightState>,
}

#[derive(Debug, Clone)]
pub(crate) struct FanEntity {
    pub(crate) common: EntityCommon,
    pub(crate) speed_count: Option<u32>,
    pub(crate) observation: Observation<(bool, i32)>,
}

#[derive(Debug, Clone)]
pub(crate) struct SirenEntity {
    pub(crate) common: EntityCommon,
    pub(crate) observation: Observation<bool>,
}

#[derive(Debug, Clone)]
pub(crate) struct ClimateEntity {
    pub(crate) common: EntityCommon,
    pub(crate) source_modes: Vec<i32>,
    pub(crate) dual_target: bool,
    pub(crate) minimum_temperature: f32,
    pub(crate) maximum_temperature: f32,
    pub(crate) target_step: f32,
    pub(crate) temperature_unit: i32,
    pub(crate) fan_choices: BTreeMap<String, ClimateChoice>,
    pub(crate) swing_choices: BTreeMap<String, ClimateChoice>,
    pub(crate) preset_choices: BTreeMap<String, ClimateChoice>,
    pub(crate) observation: Observation<ClimateState>,
}

#[derive(Debug, Clone)]
pub(crate) struct CoverEntity {
    pub(crate) common: EntityCommon,
    pub(crate) assumed_state: bool,
    pub(crate) supports_position: bool,
    pub(crate) supports_tilt: bool,
    pub(crate) supports_stop: bool,
    pub(crate) observation: Observation<CoverState>,
}

#[derive(Debug, Clone)]
pub(crate) struct ButtonEntity {
    pub(crate) common: EntityCommon,
    pub(crate) device_class: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnsupportedDiagnostic {
    pub(crate) source_locator: String,
    pub(crate) entity_type: &'static str,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionError {
    InvalidConfiguration,
    ConnectionFailed,
    DiscoveryFailed,
    Unavailable,
    OutcomeUnknown,
}

pub(crate) enum OnOffEntityRef<'a> {
    Switch(&'a SwitchEntity),
    Light(&'a LightEntity),
    Fan(&'a FanEntity),
    Siren(&'a SirenEntity),
}

impl OnOffEntityRef<'_> {
    pub(crate) fn common(&self) -> &EntityCommon {
        match self {
            Self::Switch(entity) => &entity.common,
            Self::Light(entity) => &entity.common,
            Self::Fan(entity) => &entity.common,
            Self::Siren(entity) => &entity.common,
        }
    }

    pub(crate) const fn semantic_role(&self) -> &'static str {
        match self {
            Self::Switch(_) => "switch",
            Self::Light(_) => "light",
            Self::Fan(_) => "fan",
            Self::Siren(_) => "siren",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectedObservation<T> {
    pub(crate) current: Option<T>,
    pub(crate) revision: u64,
    pub(crate) received_at_unix_milliseconds: u64,
    pub(crate) status: HydrationStatus,
    pub(crate) stale_since_unix_milliseconds: Option<u64>,
    pub(crate) assumed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LightColorCommand {
    Rgb {
        red: f32,
        green: f32,
        blue: f32,
    },
    Temperature {
        kelvin: f32,
    },
    White {
        white: f32,
        cold_white: f32,
        warm_white: f32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClimateWrite {
    Mode(i32),
    SingleTarget(f32),
    DualTarget { low: f32, high: f32 },
    Fan(ClimateChoice),
    Swing(ClimateChoice),
    Preset(ClimateChoice),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CoverWrite {
    OpenLift,
    CloseLift,
    OpenTilt,
    CloseTilt,
    Stop,
    PositionLift(f32),
    PositionTilt(f32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingBooleanOccurrence {
    pub(crate) facet_key: String,
    pub(crate) sequence: u64,
    pub(crate) generation: u64,
    pub(crate) received_at_unix_milliseconds: u64,
    pub(crate) value: bool,
}

pub(crate) struct NodeSession {
    address: String,
    noise_key: Zeroizing<String>,
    client: Option<EspHomeClient>,
    switches: BTreeMap<Locator, SwitchEntity>,
    binary_sensors: BTreeMap<Locator, BinaryEntity>,
    sensors: BTreeMap<Locator, SensorEntity>,
    lights: BTreeMap<Locator, LightEntity>,
    fans: BTreeMap<Locator, FanEntity>,
    sirens: BTreeMap<Locator, SirenEntity>,
    climates: BTreeMap<Locator, ClimateEntity>,
    covers: BTreeMap<Locator, CoverEntity>,
    buttons: BTreeMap<Locator, ButtonEntity>,
    incarnation_counters: BTreeMap<Locator, u64>,
    diagnostics: Vec<UnsupportedDiagnostic>,
    generation: u64,
    discovered_once: bool,
    connected: bool,
    disconnected_at_unix_milliseconds: Option<u64>,
    boolean_occurrence_continuity_lost: bool,
    boolean_occurrences: VecDeque<PendingBooleanOccurrence>,
    boolean_occurrence_sequence: u64,
    dropped_boolean_occurrences: u64,
}

impl NodeSession {
    pub(crate) fn from_json(json: &mut String) -> Result<Self, SessionError> {
        let parsed = serde_json::from_str::<ConnectorConfig>(json);
        json.zeroize();
        let parsed = parsed.map_err(|_| SessionError::InvalidConfiguration)?;
        let address = parsed
            .address
            .parse::<SocketAddr>()
            .map_err(|_| SessionError::InvalidConfiguration)?
            .to_string();
        let noise_key = Zeroizing::new(parsed.noise_key);
        let mut decoded = base64::engine::general_purpose::STANDARD
            .decode(noise_key.as_bytes())
            .map_err(|_| SessionError::InvalidConfiguration)?;
        let valid_key = decoded.len() == 32;
        decoded.zeroize();
        if !valid_key {
            return Err(SessionError::InvalidConfiguration);
        }
        Ok(Self {
            address,
            noise_key,
            client: None,
            switches: BTreeMap::new(),
            binary_sensors: BTreeMap::new(),
            sensors: BTreeMap::new(),
            lights: BTreeMap::new(),
            fans: BTreeMap::new(),
            sirens: BTreeMap::new(),
            climates: BTreeMap::new(),
            covers: BTreeMap::new(),
            buttons: BTreeMap::new(),
            incarnation_counters: BTreeMap::new(),
            diagnostics: Vec::new(),
            generation: 1,
            discovered_once: false,
            connected: false,
            disconnected_at_unix_milliseconds: None,
            boolean_occurrence_continuity_lost: false,
            boolean_occurrences: VecDeque::new(),
            boolean_occurrence_sequence: 0,
            dropped_boolean_occurrences: 0,
        })
    }

    pub(crate) async fn start(&mut self) -> Result<(), SessionError> {
        self.connect_and_discover().await
    }

    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) const fn connected(&self) -> bool {
        self.connected
    }

    pub(crate) fn disconnected_at(&self) -> Option<u64> {
        self.disconnected_at_unix_milliseconds
    }

    #[allow(
        dead_code,
        reason = "retained for bounded provider diagnostics and deterministic fixtures"
    )]
    pub(crate) fn diagnostics(&self) -> &[UnsupportedDiagnostic] {
        &self.diagnostics
    }

    pub(crate) fn drain_boolean_occurrences(
        &mut self,
        maximum: usize,
    ) -> (Vec<PendingBooleanOccurrence>, u64) {
        let count = maximum
            .min(MAX_BOOLEAN_OCCURRENCES)
            .min(self.boolean_occurrences.len());
        let events = self.boolean_occurrences.drain(..count).collect();
        let dropped = std::mem::take(&mut self.dropped_boolean_occurrences);
        (events, dropped)
    }

    fn record_boolean_occurrence(&mut self, locator: &Locator, value: bool) {
        self.boolean_occurrence_sequence =
            self.boolean_occurrence_sequence.saturating_add(1).max(1);
        if self.boolean_occurrences.len() == MAX_BOOLEAN_OCCURRENCES {
            self.boolean_occurrences.pop_front();
            self.dropped_boolean_occurrences = self.dropped_boolean_occurrences.saturating_add(1);
        }
        self.boolean_occurrences
            .push_back(PendingBooleanOccurrence {
                facet_key: locator.facet_key(),
                sequence: self.boolean_occurrence_sequence,
                generation: self.generation,
                received_at_unix_milliseconds: now_unix_milliseconds(),
                value,
            });
    }

    pub(crate) fn on_off_seeds(&self) -> Vec<FacetSeed> {
        self.switches
            .values()
            .map(seed)
            .chain(self.lights.values().map(seed))
            .chain(self.fans.values().map(seed))
            .chain(self.sirens.values().map(seed))
            .collect()
    }

    pub(crate) fn boolean_seeds(&self) -> Vec<FacetSeed> {
        self.binary_sensors.values().map(seed).collect()
    }

    pub(crate) fn measurement_seeds(&self) -> Vec<FacetSeed> {
        self.sensors.values().map(seed).collect()
    }

    pub(crate) fn level_seeds(&self) -> Vec<FacetSeed> {
        self.lights
            .values()
            .filter(|entity| entity.supports_level)
            .map(seed)
            .chain(
                self.fans
                    .values()
                    .filter(|entity| entity.speed_count.is_some())
                    .map(seed),
            )
            .collect()
    }

    pub(crate) fn color_seeds(&self) -> Vec<FacetSeed> {
        self.lights
            .values()
            .filter(|entity| {
                entity.rgb_mode.is_some()
                    || entity.color_temperature_mode.is_some()
                    || entity.white_mode.is_some()
            })
            .map(seed)
            .collect()
    }

    pub(crate) fn thermostat_seeds(&self) -> Vec<FacetSeed> {
        self.climates.values().map(seed).collect()
    }

    pub(crate) fn covering_seeds(&self) -> Vec<FacetSeed> {
        self.covers.values().map(seed).collect()
    }

    pub(crate) fn momentary_seeds(&self) -> Vec<FacetSeed> {
        self.buttons.values().map(seed).collect()
    }

    pub(crate) fn on_off(&self, locator: &Locator, incarnation: u64) -> Option<OnOffEntityRef<'_>> {
        match locator.kind {
            EntityKind::Switch => self
                .switches
                .get(locator)
                .filter(|entity| entity.common.incarnation == incarnation)
                .map(OnOffEntityRef::Switch),
            EntityKind::Light => self
                .lights
                .get(locator)
                .filter(|entity| entity.common.incarnation == incarnation)
                .map(OnOffEntityRef::Light),
            EntityKind::Fan => self
                .fans
                .get(locator)
                .filter(|entity| entity.common.incarnation == incarnation)
                .map(OnOffEntityRef::Fan),
            EntityKind::Siren => self
                .sirens
                .get(locator)
                .filter(|entity| entity.common.incarnation == incarnation)
                .map(OnOffEntityRef::Siren),
            _ => None,
        }
    }

    pub(crate) fn on_off_state(
        &self,
        locator: &Locator,
        incarnation: u64,
    ) -> Option<ProjectedObservation<bool>> {
        match self.on_off(locator, incarnation)? {
            OnOffEntityRef::Switch(entity) => Some(project(
                &entity.observation,
                entity.assumed_state,
                |value| *value,
            )),
            OnOffEntityRef::Light(entity) => {
                Some(project(&entity.observation, false, |value| value.on))
            }
            OnOffEntityRef::Fan(entity) => {
                Some(project(&entity.observation, false, |value| value.0))
            }
            OnOffEntityRef::Siren(entity) => {
                Some(project(&entity.observation, false, |value| *value))
            }
        }
    }

    pub(crate) fn binary_sensor(
        &self,
        locator: &Locator,
        incarnation: u64,
    ) -> Option<&BinaryEntity> {
        self.binary_sensors
            .get(locator)
            .filter(|entity| entity.common.incarnation == incarnation)
    }

    pub(crate) fn sensor(&self, locator: &Locator, incarnation: u64) -> Option<&SensorEntity> {
        self.sensors
            .get(locator)
            .filter(|entity| entity.common.incarnation == incarnation)
    }

    pub(crate) fn light(&self, locator: &Locator, incarnation: u64) -> Option<&LightEntity> {
        self.lights
            .get(locator)
            .filter(|entity| entity.common.incarnation == incarnation)
    }

    pub(crate) fn fan(&self, locator: &Locator, incarnation: u64) -> Option<&FanEntity> {
        self.fans
            .get(locator)
            .filter(|entity| entity.common.incarnation == incarnation)
    }

    pub(crate) fn climate(&self, locator: &Locator, incarnation: u64) -> Option<&ClimateEntity> {
        self.climates
            .get(locator)
            .filter(|entity| entity.common.incarnation == incarnation)
    }

    pub(crate) fn cover(&self, locator: &Locator, incarnation: u64) -> Option<&CoverEntity> {
        self.covers
            .get(locator)
            .filter(|entity| entity.common.incarnation == incarnation)
    }

    pub(crate) fn button(&self, locator: &Locator, incarnation: u64) -> Option<&ButtonEntity> {
        self.buttons
            .get(locator)
            .filter(|entity| entity.common.incarnation == incarnation)
    }

    pub(crate) async fn refresh_occurrences(&mut self) -> Result<(), SessionError> {
        if self.boolean_occurrence_continuity_lost || self.client.is_none() {
            return Err(SessionError::ConnectionFailed);
        }
        let deadline = tokio::time::Instant::now() + REFRESH_WINDOW;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            let read = {
                let client = self.client.as_mut().ok_or(SessionError::ConnectionFailed)?;
                tokio::time::timeout(remaining, client.try_read()).await
            };
            match read {
                Err(_) => break,
                Ok(Ok(message)) => self.apply_message(message),
                Ok(Err(_)) => {
                    self.client = None;
                    self.mark_disconnected();
                    return Err(SessionError::ConnectionFailed);
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn refresh(&mut self) {
        if self.client.is_none() && self.connect_and_discover().await.is_err() {
            self.mark_disconnected();
            return;
        }
        let deadline = tokio::time::Instant::now() + REFRESH_WINDOW;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            let read = {
                let Some(client) = self.client.as_mut() else {
                    break;
                };
                tokio::time::timeout(remaining, client.try_read()).await
            };
            match read {
                Err(_) => break,
                Ok(Ok(message)) => self.apply_message(message),
                Ok(Err(_)) => {
                    self.client = None;
                    self.mark_disconnected();
                    break;
                }
            }
        }
    }

    #[allow(deprecated)]
    pub(crate) async fn set_on_off(
        &mut self,
        locator: &Locator,
        incarnation: u64,
        value: bool,
    ) -> Result<(), SessionError> {
        if self.on_off(locator, incarnation).is_none() {
            return Err(SessionError::Unavailable);
        }
        self.ensure_connected().await?;
        if self.on_off(locator, incarnation).is_none() {
            return Err(SessionError::Unavailable);
        }
        let write = match locator.kind {
            EntityKind::Switch => {
                self.client
                    .as_mut()
                    .expect("connected client")
                    .try_write(SwitchCommandRequest {
                        key: locator.key,
                        state: value,
                        device_id: locator.device_id,
                    })
                    .await
            }
            EntityKind::Light => {
                self.client
                    .as_mut()
                    .expect("connected client")
                    .try_write(empty_light_command(locator, Some(value)))
                    .await
            }
            EntityKind::Fan => {
                self.client
                    .as_mut()
                    .expect("connected client")
                    .try_write(FanCommandRequest {
                        key: locator.key,
                        has_state: true,
                        state: value,
                        has_speed: false,
                        speed: 0,
                        has_oscillating: false,
                        oscillating: false,
                        has_direction: false,
                        direction: 0,
                        has_speed_level: false,
                        speed_level: 0,
                        has_preset_mode: false,
                        preset_mode: String::new(),
                        device_id: locator.device_id,
                    })
                    .await
            }
            EntityKind::Siren => {
                self.client
                    .as_mut()
                    .expect("connected client")
                    .try_write(SirenCommandRequest {
                        key: locator.key,
                        has_state: true,
                        state: value,
                        has_tone: false,
                        tone: String::new(),
                        has_duration: false,
                        duration: 0,
                        has_volume: false,
                        volume: 0.0,
                        device_id: locator.device_id,
                    })
                    .await
            }
            _ => return Err(SessionError::Unavailable),
        };
        self.finish_write(write)
    }

    #[allow(deprecated)]
    pub(crate) async fn set_level(
        &mut self,
        locator: &Locator,
        incarnation: u64,
        ratio: f32,
    ) -> Result<(), SessionError> {
        let fan_speed = match locator.kind {
            EntityKind::Light
                if self
                    .light(locator, incarnation)
                    .is_some_and(|entity| entity.supports_level) =>
            {
                None
            }
            EntityKind::Fan => Some(
                self.fan(locator, incarnation)
                    .and_then(|entity| entity.speed_count)
                    .ok_or(SessionError::Unavailable)?,
            ),
            _ => return Err(SessionError::Unavailable),
        };
        self.ensure_connected().await?;
        let write = if let Some(count) = fan_speed {
            if self.fan(locator, incarnation).is_none() {
                return Err(SessionError::Unavailable);
            }
            let speed = (ratio * count as f32).round().clamp(1.0, count as f32) as i32;
            self.client
                .as_mut()
                .expect("connected client")
                .try_write(FanCommandRequest {
                    key: locator.key,
                    has_state: false,
                    state: false,
                    has_speed: false,
                    speed: 0,
                    has_oscillating: false,
                    oscillating: false,
                    has_direction: false,
                    direction: 0,
                    has_speed_level: true,
                    speed_level: speed,
                    has_preset_mode: false,
                    preset_mode: String::new(),
                    device_id: locator.device_id,
                })
                .await
        } else {
            if !self
                .light(locator, incarnation)
                .is_some_and(|entity| entity.supports_level)
            {
                return Err(SessionError::Unavailable);
            }
            let mut command = empty_light_command(locator, None);
            command.has_brightness = true;
            command.brightness = ratio;
            self.client
                .as_mut()
                .expect("connected client")
                .try_write(command)
                .await
        };
        self.finish_write(write)
    }

    pub(crate) async fn set_light_color(
        &mut self,
        locator: &Locator,
        incarnation: u64,
        value: LightColorCommand,
    ) -> Result<(), SessionError> {
        let entity = self
            .light(locator, incarnation)
            .ok_or(SessionError::Unavailable)?;
        let mut command = empty_light_command(locator, None);
        command.has_color_mode = true;
        match value {
            LightColorCommand::Rgb { red, green, blue } => {
                command.color_mode = entity.rgb_mode.ok_or(SessionError::Unavailable)?;
                command.has_rgb = true;
                command.red = red;
                command.green = green;
                command.blue = blue;
            }
            LightColorCommand::Temperature { kelvin } => {
                command.color_mode = entity
                    .color_temperature_mode
                    .ok_or(SessionError::Unavailable)?;
                command.has_color_temperature = true;
                command.color_temperature = 1_000_000.0 / kelvin;
            }
            LightColorCommand::White {
                white,
                cold_white,
                warm_white,
            } => {
                command.color_mode = entity.white_mode.ok_or(SessionError::Unavailable)?;
                if command.color_mode == 7 {
                    command.has_white = true;
                    command.white = white;
                } else {
                    command.has_cold_white = true;
                    command.cold_white = cold_white;
                    command.has_warm_white = true;
                    command.warm_white = warm_white;
                }
            }
        }
        self.ensure_connected().await?;
        if self.light(locator, incarnation).is_none() {
            return Err(SessionError::Unavailable);
        }
        let write = self
            .client
            .as_mut()
            .expect("connected client")
            .try_write(command)
            .await;
        self.finish_write(write)
    }

    pub(crate) async fn write_climate(
        &mut self,
        locator: &Locator,
        incarnation: u64,
        value: ClimateWrite,
    ) -> Result<(), SessionError> {
        if self.climate(locator, incarnation).is_none() {
            return Err(SessionError::Unavailable);
        }
        let mut command = empty_climate_command(locator);
        match value {
            ClimateWrite::Mode(mode) => {
                command.has_mode = true;
                command.mode = mode;
            }
            ClimateWrite::SingleTarget(target) => {
                command.has_target_temperature = true;
                command.target_temperature = target;
            }
            ClimateWrite::DualTarget { low, high } => {
                command.has_target_temperature_low = true;
                command.target_temperature_low = low;
                command.has_target_temperature_high = true;
                command.target_temperature_high = high;
            }
            ClimateWrite::Fan(ClimateChoice::Standard(mode)) => {
                command.has_fan_mode = true;
                command.fan_mode = mode;
            }
            ClimateWrite::Fan(ClimateChoice::Custom(mode)) => {
                command.has_custom_fan_mode = true;
                command.custom_fan_mode = mode;
            }
            ClimateWrite::Swing(ClimateChoice::Standard(mode)) => {
                command.has_swing_mode = true;
                command.swing_mode = mode;
            }
            ClimateWrite::Swing(ClimateChoice::Custom(_)) => return Err(SessionError::Unavailable),
            ClimateWrite::Preset(ClimateChoice::Standard(mode)) => {
                command.has_preset = true;
                command.preset = mode;
            }
            ClimateWrite::Preset(ClimateChoice::Custom(mode)) => {
                command.has_custom_preset = true;
                command.custom_preset = mode;
            }
        }
        self.ensure_connected().await?;
        if self.climate(locator, incarnation).is_none() {
            return Err(SessionError::Unavailable);
        }
        let write = self
            .client
            .as_mut()
            .expect("connected client")
            .try_write(command)
            .await;
        self.finish_write(write)
    }

    pub(crate) async fn write_cover(
        &mut self,
        locator: &Locator,
        incarnation: u64,
        value: CoverWrite,
    ) -> Result<(), SessionError> {
        if self.cover(locator, incarnation).is_none() {
            return Err(SessionError::Unavailable);
        }
        let mut command = empty_cover_command(locator);
        match value {
            CoverWrite::OpenLift => {
                command.has_position = true;
                command.position = 1.0;
            }
            CoverWrite::CloseLift => {
                command.has_position = true;
                command.position = 0.0;
            }
            CoverWrite::OpenTilt => {
                command.has_tilt = true;
                command.tilt = 1.0;
            }
            CoverWrite::CloseTilt => {
                command.has_tilt = true;
                command.tilt = 0.0;
            }
            CoverWrite::Stop => command.stop = true,
            CoverWrite::PositionLift(position) => {
                command.has_position = true;
                command.position = position;
            }
            CoverWrite::PositionTilt(position) => {
                command.has_tilt = true;
                command.tilt = position;
            }
        }
        self.ensure_connected().await?;
        if self.cover(locator, incarnation).is_none() {
            return Err(SessionError::Unavailable);
        }
        let write = self
            .client
            .as_mut()
            .expect("connected client")
            .try_write(command)
            .await;
        self.finish_write(write)
    }

    pub(crate) async fn invoke_button(
        &mut self,
        locator: &Locator,
        incarnation: u64,
    ) -> Result<(), SessionError> {
        if self.button(locator, incarnation).is_none() {
            return Err(SessionError::Unavailable);
        }
        self.ensure_connected().await?;
        if self.button(locator, incarnation).is_none() {
            return Err(SessionError::Unavailable);
        }
        let write = self
            .client
            .as_mut()
            .expect("connected client")
            .try_write(ButtonCommandRequest {
                key: locator.key,
                device_id: locator.device_id,
            })
            .await;
        self.finish_write(write)
    }

    async fn ensure_connected(&mut self) -> Result<(), SessionError> {
        if self.client.is_none() {
            self.connect_and_discover()
                .await
                .map_err(|_| SessionError::Unavailable)?;
        }
        Ok(())
    }

    fn finish_write<E>(&mut self, result: Result<(), E>) -> Result<(), SessionError> {
        if result.is_err() {
            self.client = None;
            self.mark_disconnected();
            return Err(SessionError::OutcomeUnknown);
        }
        Ok(())
    }

    async fn connect_and_discover(&mut self) -> Result<(), SessionError> {
        let mut client = EspHomeClient::builder()
            .address(&self.address)
            .key(self.noise_key.as_str())
            .client_info(CLIENT_INFO)
            .timeout(CONNECT_TIMEOUT)
            .connect()
            .await
            .map_err(|_| SessionError::ConnectionFailed)?;
        client
            .try_write(ListEntitiesRequest {})
            .await
            .map_err(|_| SessionError::DiscoveryFailed)?;
        client
            .try_write(SubscribeStatesRequest {})
            .await
            .map_err(|_| SessionError::DiscoveryFailed)?;

        let mut topology = Topology::default();
        let discovery_deadline = tokio::time::Instant::now() + DISCOVERY_TIMEOUT;
        let mut state_grace_deadline = None;
        loop {
            let deadline = state_grace_deadline.unwrap_or(discovery_deadline);
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                if topology.listing_done {
                    break;
                }
                return Err(SessionError::DiscoveryFailed);
            }
            let message = tokio::time::timeout(remaining, client.try_read())
                .await
                .map_err(|_| SessionError::DiscoveryFailed)?
                .map_err(|_| SessionError::DiscoveryFailed)?;
            topology.apply(message);
            if topology.listing_done {
                if topology.observed_count() >= topology.stateful_entity_count() {
                    break;
                }
                state_grace_deadline
                    .get_or_insert_with(|| tokio::time::Instant::now() + INITIAL_STATE_GRACE);
            }
        }
        self.reconcile(topology);
        self.connected = true;
        self.disconnected_at_unix_milliseconds = None;
        self.client = Some(client);
        Ok(())
    }

    fn reconcile(&mut self, topology: Topology) {
        let mut changed = false;
        changed |= reconcile_map(
            &mut self.switches,
            topology.switches,
            &mut self.incarnation_counters,
        );
        changed |= reconcile_map(
            &mut self.binary_sensors,
            topology.binary_sensors,
            &mut self.incarnation_counters,
        );
        changed |= reconcile_map(
            &mut self.sensors,
            topology.sensors,
            &mut self.incarnation_counters,
        );
        changed |= reconcile_map(
            &mut self.lights,
            topology.lights,
            &mut self.incarnation_counters,
        );
        changed |= reconcile_map(
            &mut self.fans,
            topology.fans,
            &mut self.incarnation_counters,
        );
        changed |= reconcile_map(
            &mut self.sirens,
            topology.sirens,
            &mut self.incarnation_counters,
        );
        changed |= reconcile_map(
            &mut self.climates,
            topology.climates,
            &mut self.incarnation_counters,
        );
        changed |= reconcile_map(
            &mut self.covers,
            topology.covers,
            &mut self.incarnation_counters,
        );
        changed |= reconcile_map(
            &mut self.buttons,
            topology.buttons,
            &mut self.incarnation_counters,
        );
        if self.discovered_once && changed {
            self.generation = self.generation.saturating_add(1).max(1);
        }
        self.discovered_once = true;
        self.diagnostics = topology.diagnostics;
    }

    fn apply_message(&mut self, message: EspHomeMessage) {
        match message {
            EspHomeMessage::SwitchStateResponse(state) => {
                let locator = Locator::new(EntityKind::Switch, state.device_id, state.key);
                if let Some(entity) = self.switches.get_mut(&locator) {
                    entity.observation.observe(state.state);
                }
            }
            EspHomeMessage::BinarySensorStateResponse(state) => {
                let locator = Locator::new(EntityKind::BinarySensor, state.device_id, state.key);
                let occurrence = if let Some(entity) = self.binary_sensors.get_mut(&locator) {
                    if state.missing_state {
                        entity.observation.gap();
                        None
                    } else {
                        entity.observation.observe(state.state);
                        Some(state.state)
                    }
                } else {
                    None
                };
                if let Some(value) = occurrence {
                    self.record_boolean_occurrence(&locator, value);
                }
            }
            EspHomeMessage::SensorStateResponse(state) => {
                let locator = Locator::new(EntityKind::Sensor, state.device_id, state.key);
                if let Some(entity) = self.sensors.get_mut(&locator) {
                    observe_sensor(entity, state.state, state.missing_state);
                }
            }
            EspHomeMessage::LightStateResponse(state) => {
                let locator = Locator::new(EntityKind::Light, state.device_id, state.key);
                if let Some(entity) = self.lights.get_mut(&locator) {
                    if let Some(state) = light_state(state) {
                        entity.observation.observe(state);
                    } else {
                        entity.observation.gap();
                    }
                }
            }
            EspHomeMessage::FanStateResponse(state) => {
                let locator = Locator::new(EntityKind::Fan, state.device_id, state.key);
                if let Some(entity) = self.fans.get_mut(&locator) {
                    entity.observation.observe((state.state, state.speed_level));
                }
            }
            EspHomeMessage::SirenStateResponse(state) => {
                let locator = Locator::new(EntityKind::Siren, state.device_id, state.key);
                if let Some(entity) = self.sirens.get_mut(&locator) {
                    entity.observation.observe(state.state);
                }
            }
            EspHomeMessage::ClimateStateResponse(state) => {
                let locator = Locator::new(EntityKind::Climate, state.device_id, state.key);
                if let Some(entity) = self.climates.get_mut(&locator) {
                    if let Some(state) = climate_state(state) {
                        entity.observation.observe(state);
                    } else {
                        entity.observation.gap();
                    }
                }
            }
            EspHomeMessage::CoverStateResponse(state) => {
                let locator = Locator::new(EntityKind::Cover, state.device_id, state.key);
                if let Some(entity) = self.covers.get_mut(&locator) {
                    if let Some(state) = cover_state(state) {
                        entity.observation.observe(state);
                    } else {
                        entity.observation.gap();
                    }
                }
            }
            _ => {}
        }
    }

    fn mark_disconnected(&mut self) {
        self.connected = false;
        self.boolean_occurrence_continuity_lost = true;
        self.disconnected_at_unix_milliseconds
            .get_or_insert_with(now_unix_milliseconds);
    }
}

fn seed<E: EntityRecord>(entity: &E) -> FacetSeed {
    FacetSeed {
        locator: entity.common().locator.clone(),
        incarnation: entity.common().incarnation,
    }
}

fn project<T, U>(
    observation: &Observation<T>,
    assumed: bool,
    map: impl Fn(&T) -> U,
) -> ProjectedObservation<U> {
    ProjectedObservation {
        current: observation.current.as_ref().map(map),
        revision: observation.revision,
        received_at_unix_milliseconds: observation.received_at_unix_milliseconds,
        status: observation.status,
        stale_since_unix_milliseconds: observation.stale_since_unix_milliseconds,
        assumed,
    }
}

trait EntityRecord: Clone {
    fn common(&self) -> &EntityCommon;
    fn common_mut(&mut self) -> &mut EntityCommon;
    fn topology_eq(&self, other: &Self) -> bool;
    fn reconcile_observation(&mut self, previous: &Self);
}

macro_rules! entity_record {
    ($type:ty, $topology:expr, $observation:ident) => {
        impl EntityRecord for $type {
            fn common(&self) -> &EntityCommon {
                &self.common
            }
            fn common_mut(&mut self) -> &mut EntityCommon {
                &mut self.common
            }
            fn topology_eq(&self, other: &Self) -> bool {
                ($topology)(self, other)
            }
            fn reconcile_observation(&mut self, previous: &Self) {
                self.$observation.reconcile(&previous.$observation);
            }
        }
    };
}

entity_record!(
    SwitchEntity,
    |a: &SwitchEntity, b: &SwitchEntity| a.common.label == b.common.label
        && a.assumed_state == b.assumed_state,
    observation
);
entity_record!(
    BinaryEntity,
    |a: &BinaryEntity, b: &BinaryEntity| a.common.label == b.common.label && a.profile == b.profile,
    observation
);
entity_record!(
    SensorEntity,
    |a: &SensorEntity, b: &SensorEntity| a.common.label == b.common.label
        && a.profile == b.profile
        && a.canonical_unit == b.canonical_unit
        && a.native_unit == b.native_unit
        && a.scale == b.scale
        && a.offset == b.offset
        && a.kind == b.kind
        && a.accuracy_decimals == b.accuracy_decimals,
    observation
);
entity_record!(
    LightEntity,
    |a: &LightEntity, b: &LightEntity| a.common.label == b.common.label
        && a.supports_level == b.supports_level
        && a.rgb_mode == b.rgb_mode
        && a.color_temperature_mode == b.color_temperature_mode
        && a.white_mode == b.white_mode
        && a.minimum_mireds == b.minimum_mireds
        && a.maximum_mireds == b.maximum_mireds,
    observation
);
entity_record!(
    FanEntity,
    |a: &FanEntity, b: &FanEntity| a.common.label == b.common.label
        && a.speed_count == b.speed_count,
    observation
);
entity_record!(
    SirenEntity,
    |a: &SirenEntity, b: &SirenEntity| a.common.label == b.common.label,
    observation
);
entity_record!(
    ClimateEntity,
    |a: &ClimateEntity, b: &ClimateEntity| a.common.label == b.common.label
        && a.source_modes == b.source_modes
        && a.dual_target == b.dual_target
        && a.minimum_temperature == b.minimum_temperature
        && a.maximum_temperature == b.maximum_temperature
        && a.target_step == b.target_step
        && a.temperature_unit == b.temperature_unit
        && a.fan_choices == b.fan_choices
        && a.swing_choices == b.swing_choices
        && a.preset_choices == b.preset_choices,
    observation
);
entity_record!(
    CoverEntity,
    |a: &CoverEntity, b: &CoverEntity| a.common.label == b.common.label
        && a.assumed_state == b.assumed_state
        && a.supports_position == b.supports_position
        && a.supports_tilt == b.supports_tilt
        && a.supports_stop == b.supports_stop,
    observation
);

impl EntityRecord for ButtonEntity {
    fn common(&self) -> &EntityCommon {
        &self.common
    }
    fn common_mut(&mut self) -> &mut EntityCommon {
        &mut self.common
    }
    fn topology_eq(&self, other: &Self) -> bool {
        self.common.label == other.common.label && self.device_class == other.device_class
    }
    fn reconcile_observation(&mut self, _previous: &Self) {}
}

fn reconcile_map<E: EntityRecord>(
    current: &mut BTreeMap<Locator, E>,
    mut next: BTreeMap<Locator, E>,
    counters: &mut BTreeMap<Locator, u64>,
) -> bool {
    let previous = std::mem::take(current);
    let mut changed = previous.len() != next.len();
    for (locator, entity) in &mut next {
        if let Some(old) = previous.get(locator) {
            counters
                .entry(locator.clone())
                .and_modify(|value| *value = (*value).max(old.common().incarnation))
                .or_insert(old.common().incarnation);
            if entity.topology_eq(old) {
                entity.common_mut().incarnation = old.common().incarnation;
                entity.reconcile_observation(old);
            } else {
                changed = true;
                entity.common_mut().incarnation = next_incarnation(counters, locator);
            }
        } else {
            changed = true;
            entity.common_mut().incarnation = next_incarnation(counters, locator);
        }
    }
    if previous.keys().any(|locator| !next.contains_key(locator)) {
        changed = true;
    }
    *current = next;
    changed
}

fn next_incarnation(counters: &mut BTreeMap<Locator, u64>, locator: &Locator) -> u64 {
    let value = counters.entry(locator.clone()).or_insert(0);
    *value = value.saturating_add(1).max(1);
    *value
}

#[derive(Default)]
struct Topology {
    switches: BTreeMap<Locator, SwitchEntity>,
    binary_sensors: BTreeMap<Locator, BinaryEntity>,
    sensors: BTreeMap<Locator, SensorEntity>,
    lights: BTreeMap<Locator, LightEntity>,
    fans: BTreeMap<Locator, FanEntity>,
    sirens: BTreeMap<Locator, SirenEntity>,
    climates: BTreeMap<Locator, ClimateEntity>,
    covers: BTreeMap<Locator, CoverEntity>,
    buttons: BTreeMap<Locator, ButtonEntity>,
    pending: BTreeMap<Locator, EspHomeMessage>,
    observed: BTreeSet<Locator>,
    diagnostics: Vec<UnsupportedDiagnostic>,
    listing_done: bool,
}

impl Topology {
    fn apply(&mut self, message: EspHomeMessage) {
        match message {
            EspHomeMessage::ListEntitiesSwitchResponse(entity) => self.add_switch(entity),
            EspHomeMessage::ListEntitiesBinarySensorResponse(entity) => self.add_binary(entity),
            EspHomeMessage::ListEntitiesSensorResponse(entity) => self.add_sensor(entity),
            EspHomeMessage::ListEntitiesLightResponse(entity) => self.add_light(entity),
            EspHomeMessage::ListEntitiesFanResponse(entity) => self.add_fan(entity),
            EspHomeMessage::ListEntitiesSirenResponse(entity) => self.add_siren(entity),
            EspHomeMessage::ListEntitiesClimateResponse(entity) => self.add_climate(entity),
            EspHomeMessage::ListEntitiesCoverResponse(entity) => self.add_cover(entity),
            EspHomeMessage::ListEntitiesButtonResponse(entity) => self.add_button(entity),
            EspHomeMessage::SwitchStateResponse(state) => {
                let locator = Locator::new(EntityKind::Switch, state.device_id, state.key);
                self.observed.insert(locator.clone());
                if let Some(entity) = self.switches.get_mut(&locator) {
                    entity.observation.observe(state.state);
                } else {
                    self.pending
                        .insert(locator, EspHomeMessage::SwitchStateResponse(state));
                }
            }
            EspHomeMessage::BinarySensorStateResponse(state) => {
                let locator = Locator::new(EntityKind::BinarySensor, state.device_id, state.key);
                self.observed.insert(locator.clone());
                if let Some(entity) = self.binary_sensors.get_mut(&locator) {
                    if state.missing_state {
                        entity.observation.gap();
                    } else {
                        entity.observation.observe(state.state);
                    }
                } else {
                    self.pending
                        .insert(locator, EspHomeMessage::BinarySensorStateResponse(state));
                }
            }
            EspHomeMessage::SensorStateResponse(state) => {
                let locator = Locator::new(EntityKind::Sensor, state.device_id, state.key);
                self.observed.insert(locator.clone());
                if let Some(entity) = self.sensors.get_mut(&locator) {
                    observe_sensor(entity, state.state, state.missing_state);
                } else {
                    self.pending
                        .insert(locator, EspHomeMessage::SensorStateResponse(state));
                }
            }
            EspHomeMessage::LightStateResponse(state) => {
                let locator = Locator::new(EntityKind::Light, state.device_id, state.key);
                self.observed.insert(locator.clone());
                if let Some(entity) = self.lights.get_mut(&locator) {
                    apply_light_state(entity, state);
                } else {
                    self.pending
                        .insert(locator, EspHomeMessage::LightStateResponse(state));
                }
            }
            EspHomeMessage::FanStateResponse(state) => {
                let locator = Locator::new(EntityKind::Fan, state.device_id, state.key);
                self.observed.insert(locator.clone());
                if let Some(entity) = self.fans.get_mut(&locator) {
                    entity.observation.observe((state.state, state.speed_level));
                } else {
                    self.pending
                        .insert(locator, EspHomeMessage::FanStateResponse(state));
                }
            }
            EspHomeMessage::SirenStateResponse(state) => {
                let locator = Locator::new(EntityKind::Siren, state.device_id, state.key);
                self.observed.insert(locator.clone());
                if let Some(entity) = self.sirens.get_mut(&locator) {
                    entity.observation.observe(state.state);
                } else {
                    self.pending
                        .insert(locator, EspHomeMessage::SirenStateResponse(state));
                }
            }
            EspHomeMessage::ClimateStateResponse(state) => {
                let locator = Locator::new(EntityKind::Climate, state.device_id, state.key);
                self.observed.insert(locator.clone());
                if let Some(entity) = self.climates.get_mut(&locator) {
                    apply_climate_state(entity, state);
                } else {
                    self.pending
                        .insert(locator, EspHomeMessage::ClimateStateResponse(state));
                }
            }
            EspHomeMessage::CoverStateResponse(state) => {
                let locator = Locator::new(EntityKind::Cover, state.device_id, state.key);
                self.observed.insert(locator.clone());
                if let Some(entity) = self.covers.get_mut(&locator) {
                    apply_cover_state(entity, state);
                } else {
                    self.pending
                        .insert(locator, EspHomeMessage::CoverStateResponse(state));
                }
            }
            EspHomeMessage::ListEntitiesTextSensorResponse(entity) => self.unsupported(
                "text-sensor",
                entity.device_id,
                entity.key,
                "no-typed-contract",
            ),
            EspHomeMessage::ListEntitiesServicesResponse(entity) => {
                self.unsupported("service", 0, entity.key, "no-typed-contract")
            }
            EspHomeMessage::ListEntitiesCameraResponse(entity) => {
                self.unsupported("camera", entity.device_id, entity.key, "no-typed-contract")
            }
            EspHomeMessage::ListEntitiesWaterHeaterResponse(entity) => self.unsupported(
                "water-heater",
                entity.device_id,
                entity.key,
                "no-conservative-thermostat-mapping",
            ),
            EspHomeMessage::ListEntitiesNumberResponse(entity) => {
                self.unsupported("number", entity.device_id, entity.key, "no-typed-contract")
            }
            EspHomeMessage::ListEntitiesSelectResponse(entity) => {
                self.unsupported("select", entity.device_id, entity.key, "no-typed-contract")
            }
            EspHomeMessage::ListEntitiesLockResponse(entity) => {
                self.unsupported("lock", entity.device_id, entity.key, "no-lock-contract")
            }
            EspHomeMessage::ListEntitiesMediaPlayerResponse(entity) => self.unsupported(
                "media-player",
                entity.device_id,
                entity.key,
                "no-media-contract",
            ),
            EspHomeMessage::ListEntitiesAlarmControlPanelResponse(entity) => self.unsupported(
                "alarm-control-panel",
                entity.device_id,
                entity.key,
                "no-alarm-contract",
            ),
            EspHomeMessage::ListEntitiesTextResponse(entity) => {
                self.unsupported("text", entity.device_id, entity.key, "no-typed-contract")
            }
            EspHomeMessage::ListEntitiesDateResponse(entity) => {
                self.unsupported("date", entity.device_id, entity.key, "no-typed-contract")
            }
            EspHomeMessage::ListEntitiesTimeResponse(entity) => {
                self.unsupported("time", entity.device_id, entity.key, "no-typed-contract")
            }
            EspHomeMessage::ListEntitiesEventResponse(entity) => {
                self.unsupported("event", entity.device_id, entity.key, "no-event-contract")
            }
            EspHomeMessage::ListEntitiesValveResponse(entity) => self.unsupported(
                "valve",
                entity.device_id,
                entity.key,
                "no-conservative-level-mapping",
            ),
            EspHomeMessage::ListEntitiesDateTimeResponse(entity) => self.unsupported(
                "datetime",
                entity.device_id,
                entity.key,
                "no-typed-contract",
            ),
            EspHomeMessage::ListEntitiesUpdateResponse(entity) => {
                self.unsupported("update", entity.device_id, entity.key, "no-update-contract")
            }
            EspHomeMessage::ListEntitiesInfraredResponse(entity) => self.unsupported(
                "infrared",
                entity.device_id,
                entity.key,
                "payload-required-action",
            ),
            EspHomeMessage::ListEntitiesRadioFrequencyResponse(entity) => self.unsupported(
                "radio-frequency",
                entity.device_id,
                entity.key,
                "payload-required-action",
            ),
            EspHomeMessage::ListEntitiesDoneResponse(_) => self.listing_done = true,
            _ => {}
        }
    }

    fn add_switch(&mut self, entity: ListEntitiesSwitchResponse) {
        let locator = Locator::new(EntityKind::Switch, entity.device_id, entity.key);
        if !self.normal_category("switch", &locator, entity.entity_category) {
            return;
        }
        let mut result = SwitchEntity {
            common: common(locator.clone(), &entity.name, &entity.object_id),
            assumed_state: entity.assumed_state,
            observation: Observation::empty(),
        };
        if let Some(EspHomeMessage::SwitchStateResponse(state)) = self.pending.remove(&locator) {
            result.observation.observe(state.state);
        }
        self.insert_mapped(locator, result, EntityKind::Switch);
    }

    fn add_binary(&mut self, entity: ListEntitiesBinarySensorResponse) {
        let locator = Locator::new(EntityKind::BinarySensor, entity.device_id, entity.key);
        if !self.normal_category("binary-sensor", &locator, entity.entity_category) {
            return;
        }
        let mut result = BinaryEntity {
            common: common(locator.clone(), &entity.name, &entity.object_id),
            profile: boolean_profile(&entity.device_class, entity.is_status_binary_sensor),
            observation: Observation::empty(),
        };
        if let Some(EspHomeMessage::BinarySensorStateResponse(state)) =
            self.pending.remove(&locator)
        {
            if state.missing_state {
                result.observation.gap();
            } else {
                result.observation.observe(state.state);
            }
        }
        self.insert_mapped(locator, result, EntityKind::BinarySensor);
    }

    fn add_sensor(&mut self, entity: ListEntitiesSensorResponse) {
        let locator = Locator::new(EntityKind::Sensor, entity.device_id, entity.key);
        if !self.normal_category("sensor", &locator, entity.entity_category) {
            return;
        }
        let Some(profile) = measurement_profile(&entity.device_class, &entity.unit_of_measurement)
        else {
            self.diagnostic(
                &locator,
                "sensor",
                &format!(
                    "unsupported-unit:{}",
                    bounded_diagnostic_value(&entity.unit_of_measurement)
                ),
            );
            return;
        };
        let Some(kind) = measurement_kind(entity.state_class) else {
            self.diagnostic(&locator, "sensor", "unsupported-state-class");
            return;
        };
        let mut result = SensorEntity {
            common: common(locator.clone(), &entity.name, &entity.object_id),
            profile: profile.profile,
            canonical_unit: profile.canonical_unit,
            native_unit: entity.unit_of_measurement,
            scale: profile.scale,
            offset: profile.offset,
            kind,
            accuracy_decimals: u32::try_from(entity.accuracy_decimals)
                .ok()
                .filter(|value| *value <= 9),
            observation: Observation::empty(),
        };
        if let Some(EspHomeMessage::SensorStateResponse(state)) = self.pending.remove(&locator) {
            observe_sensor(&mut result, state.state, state.missing_state);
        }
        self.insert_mapped(locator, result, EntityKind::Sensor);
    }

    fn add_light(&mut self, entity: ListEntitiesLightResponse) {
        let locator = Locator::new(EntityKind::Light, entity.device_id, entity.key);
        if !self.normal_category("light", &locator, entity.entity_category) {
            return;
        }
        let modes: BTreeSet<i32> = entity.supported_color_modes.into_iter().collect();
        let supports_level = modes.iter().any(|mode| mode & 2 != 0);
        let rgb_mode = modes.contains(&35).then_some(35);
        let color_temperature_mode = modes.contains(&11).then_some(11);
        let white_mode = if modes.contains(&19) {
            Some(19)
        } else if modes.contains(&7) {
            Some(7)
        } else {
            None
        };
        if modes.iter().any(|mode| matches!(*mode, 39 | 47 | 51)) {
            self.diagnostic(
                &locator,
                "light",
                "combined-color-mode-not-losslessly-representable",
            );
        }
        if !entity.effects.is_empty() {
            self.diagnostic(&locator, "light", "effects-not-mapped");
        }
        let (minimum_mireds, maximum_mireds) = if color_temperature_mode.is_some()
            && entity.min_mireds.is_finite()
            && entity.max_mireds.is_finite()
            && entity.min_mireds > 0.0
            && entity.max_mireds >= entity.min_mireds
        {
            (entity.min_mireds, entity.max_mireds)
        } else {
            if color_temperature_mode.is_some() {
                self.diagnostic(&locator, "light", "invalid-color-temperature-range");
            }
            (0.0, 0.0)
        };
        let color_temperature_mode = color_temperature_mode.filter(|_| minimum_mireds > 0.0);
        let mut result = LightEntity {
            common: common(locator.clone(), &entity.name, &entity.object_id),
            supports_level,
            rgb_mode,
            color_temperature_mode,
            white_mode,
            minimum_mireds,
            maximum_mireds,
            observation: Observation::empty(),
        };
        if let Some(EspHomeMessage::LightStateResponse(state)) = self.pending.remove(&locator) {
            apply_light_state(&mut result, state);
        }
        self.insert_mapped(locator, result, EntityKind::Light);
    }

    fn add_fan(&mut self, entity: ListEntitiesFanResponse) {
        let locator = Locator::new(EntityKind::Fan, entity.device_id, entity.key);
        if !self.normal_category("fan", &locator, entity.entity_category) {
            return;
        }
        if entity.supports_oscillation
            || entity.supports_direction
            || !entity.supported_preset_modes.is_empty()
        {
            self.diagnostic(&locator, "fan", "non-level-features-not-mapped");
        }
        let speed_count = entity
            .supports_speed
            .then(|| u32::try_from(entity.supported_speed_count).ok())
            .flatten()
            .filter(|count| *count > 0 && *count <= 100);
        if entity.supports_speed && speed_count.is_none() {
            self.diagnostic(&locator, "fan", "invalid-speed-range");
        }
        let mut result = FanEntity {
            common: common(locator.clone(), &entity.name, &entity.object_id),
            speed_count,
            observation: Observation::empty(),
        };
        if let Some(EspHomeMessage::FanStateResponse(state)) = self.pending.remove(&locator) {
            result.observation.observe((state.state, state.speed_level));
        }
        self.insert_mapped(locator, result, EntityKind::Fan);
    }

    fn add_siren(&mut self, entity: ListEntitiesSirenResponse) {
        let locator = Locator::new(EntityKind::Siren, entity.device_id, entity.key);
        if !self.normal_category("siren", &locator, entity.entity_category) {
            return;
        }
        if entity.supports_duration || entity.supports_volume || !entity.tones.is_empty() {
            self.diagnostic(&locator, "siren", "tone-duration-volume-not-mapped");
        }
        let mut result = SirenEntity {
            common: common(locator.clone(), &entity.name, &entity.object_id),
            observation: Observation::empty(),
        };
        if let Some(EspHomeMessage::SirenStateResponse(state)) = self.pending.remove(&locator) {
            result.observation.observe(state.state);
        }
        self.insert_mapped(locator, result, EntityKind::Siren);
    }

    fn add_climate(&mut self, entity: ListEntitiesClimateResponse) {
        let locator = Locator::new(EntityKind::Climate, entity.device_id, entity.key);
        if !self.normal_category("climate", &locator, entity.entity_category) {
            return;
        }
        if !valid_temperature_descriptor(&entity) {
            self.diagnostic(&locator, "climate", "invalid-temperature-policy");
            return;
        }
        let mut source_modes: Vec<i32> = entity
            .supported_modes
            .into_iter()
            .filter(|mode| (0..=6).contains(mode))
            .collect();
        source_modes.sort_unstable();
        source_modes.dedup();
        if source_modes.is_empty() {
            self.diagnostic(&locator, "climate", "no-supported-mode");
            return;
        }
        let fan_choices = climate_choices(
            entity.supported_fan_modes,
            entity.supported_custom_fan_modes,
            fan_choice_name,
        );
        let swing_choices =
            climate_choices(entity.supported_swing_modes, Vec::new(), swing_choice_name);
        let preset_choices = climate_choices(
            entity.supported_presets,
            entity.supported_custom_presets,
            preset_choice_name,
        );
        if entity.supports_current_humidity || entity.supports_target_humidity {
            self.diagnostic(&locator, "climate", "humidity-not-in-thermostat-contract");
        }
        let mut result = ClimateEntity {
            common: common(locator.clone(), &entity.name, &entity.object_id),
            source_modes,
            dual_target: entity.supports_two_point_target_temperature,
            minimum_temperature: entity.visual_min_temperature,
            maximum_temperature: entity.visual_max_temperature,
            target_step: entity.visual_target_temperature_step,
            temperature_unit: entity.temperature_unit,
            fan_choices,
            swing_choices,
            preset_choices,
            observation: Observation::empty(),
        };
        if let Some(EspHomeMessage::ClimateStateResponse(state)) = self.pending.remove(&locator) {
            apply_climate_state(&mut result, state);
        }
        self.insert_mapped(locator, result, EntityKind::Climate);
    }

    fn add_cover(&mut self, entity: ListEntitiesCoverResponse) {
        let locator = Locator::new(EntityKind::Cover, entity.device_id, entity.key);
        if !self.normal_category("cover", &locator, entity.entity_category) {
            return;
        }
        let mut result = CoverEntity {
            common: common(locator.clone(), &entity.name, &entity.object_id),
            assumed_state: entity.assumed_state,
            supports_position: entity.supports_position,
            supports_tilt: entity.supports_tilt,
            supports_stop: entity.supports_stop,
            observation: Observation::empty(),
        };
        if let Some(EspHomeMessage::CoverStateResponse(state)) = self.pending.remove(&locator) {
            apply_cover_state(&mut result, state);
        }
        self.insert_mapped(locator, result, EntityKind::Cover);
    }

    fn add_button(&mut self, entity: ListEntitiesButtonResponse) {
        let locator = Locator::new(EntityKind::Button, entity.device_id, entity.key);
        if !self.normal_category("button", &locator, entity.entity_category) {
            return;
        }
        let result = ButtonEntity {
            common: common(locator.clone(), &entity.name, &entity.object_id),
            device_class: entity.device_class,
        };
        self.insert_mapped(locator, result, EntityKind::Button);
    }

    fn normal_category(
        &mut self,
        entity_type: &'static str,
        locator: &Locator,
        category: i32,
    ) -> bool {
        if category == 0 {
            true
        } else {
            self.diagnostic(locator, entity_type, "non-primary-entity-category");
            false
        }
    }

    fn insert_mapped<E>(&mut self, locator: Locator, entity: E, kind: EntityKind)
    where
        E: InsertMapped,
    {
        if self.mapped_entity_count() >= MAX_MAPPED_ENTITIES {
            self.diagnostic(&locator, kind.prefix(), "mapped-entity-limit");
            return;
        }
        entity.insert(self, locator);
    }

    fn mapped_entity_count(&self) -> usize {
        self.switches.len()
            + self.binary_sensors.len()
            + self.sensors.len()
            + self.lights.len()
            + self.fans.len()
            + self.sirens.len()
            + self.climates.len()
            + self.covers.len()
            + self.buttons.len()
    }

    fn stateful_entity_count(&self) -> usize {
        self.mapped_entity_count()
            .saturating_sub(self.buttons.len())
    }

    fn observed_count(&self) -> usize {
        self.observed
            .iter()
            .filter(|locator| match locator.kind {
                EntityKind::Switch => self.switches.contains_key(*locator),
                EntityKind::BinarySensor => self.binary_sensors.contains_key(*locator),
                EntityKind::Sensor => self.sensors.contains_key(*locator),
                EntityKind::Light => self.lights.contains_key(*locator),
                EntityKind::Fan => self.fans.contains_key(*locator),
                EntityKind::Siren => self.sirens.contains_key(*locator),
                EntityKind::Climate => self.climates.contains_key(*locator),
                EntityKind::Cover => self.covers.contains_key(*locator),
                EntityKind::Button => false,
            })
            .count()
    }

    fn unsupported(&mut self, entity_type: &'static str, device_id: u32, key: u32, reason: &str) {
        if self.diagnostics.len() >= MAX_DIAGNOSTICS {
            return;
        }
        self.diagnostics.push(UnsupportedDiagnostic {
            source_locator: format!("{entity_type}-{device_id}-{key}"),
            entity_type,
            reason: reason.into(),
        });
    }

    fn diagnostic(&mut self, locator: &Locator, entity_type: &'static str, reason: &str) {
        if self.diagnostics.len() >= MAX_DIAGNOSTICS {
            return;
        }
        self.diagnostics.push(UnsupportedDiagnostic {
            source_locator: locator.facet_key(),
            entity_type,
            reason: reason.into(),
        });
    }
}

trait InsertMapped {
    fn insert(self, topology: &mut Topology, locator: Locator);
}
macro_rules! insert_mapped {
    ($type:ty, $field:ident) => {
        impl InsertMapped for $type {
            fn insert(self, topology: &mut Topology, locator: Locator) {
                topology.$field.insert(locator, self);
            }
        }
    };
}
insert_mapped!(SwitchEntity, switches);
insert_mapped!(BinaryEntity, binary_sensors);
insert_mapped!(SensorEntity, sensors);
insert_mapped!(LightEntity, lights);
insert_mapped!(FanEntity, fans);
insert_mapped!(SirenEntity, sirens);
insert_mapped!(ClimateEntity, climates);
insert_mapped!(CoverEntity, covers);
insert_mapped!(ButtonEntity, buttons);

fn common(locator: Locator, name: &str, object_id: &str) -> EntityCommon {
    EntityCommon {
        locator,
        label: bounded_label(name, object_id),
        incarnation: 0,
    }
}

fn apply_light_state(entity: &mut LightEntity, state: esphome_client::types::LightStateResponse) {
    if let Some(state) = light_state(state) {
        entity.observation.observe(state);
    } else {
        entity.observation.gap();
    }
}

fn light_state(state: esphome_client::types::LightStateResponse) -> Option<LightState> {
    [
        state.brightness,
        state.red,
        state.green,
        state.blue,
        state.white,
        state.color_temperature,
        state.cold_white,
        state.warm_white,
    ]
    .into_iter()
    .all(f32::is_finite)
    .then_some(LightState {
        on: state.state,
        brightness: state.brightness,
        color_mode: state.color_mode,
        red: state.red,
        green: state.green,
        blue: state.blue,
        white: state.white,
        color_temperature_mireds: state.color_temperature,
        cold_white: state.cold_white,
        warm_white: state.warm_white,
    })
}

fn observe_sensor(entity: &mut SensorEntity, state: f32, missing: bool) {
    if missing || !state.is_finite() {
        entity.observation.gap();
        return;
    }
    let canonical = f64::from(state).mul_add(entity.scale, entity.offset);
    if !canonical.is_finite() {
        entity.observation.gap();
        return;
    }
    entity.observation.observe(MeasurementValue {
        canonical_magnitude: decimal(canonical),
        native_magnitude: decimal(f64::from(state)),
    });
}

fn apply_climate_state(
    entity: &mut ClimateEntity,
    state: esphome_client::types::ClimateStateResponse,
) {
    if let Some(state) = climate_state(state) {
        entity.observation.observe(state);
    } else {
        entity.observation.gap();
    }
}

fn climate_state(state: esphome_client::types::ClimateStateResponse) -> Option<ClimateState> {
    [
        state.target_temperature,
        state.target_temperature_low,
        state.target_temperature_high,
    ]
    .into_iter()
    .all(f32::is_finite)
    .then_some(ClimateState {
        mode: state.mode,
        target_temperature: state.target_temperature,
        target_temperature_low: state.target_temperature_low,
        target_temperature_high: state.target_temperature_high,
        action: state.action,
        fan_mode: state.fan_mode,
        swing_mode: state.swing_mode,
        custom_fan_mode: state.custom_fan_mode,
        preset: state.preset,
        custom_preset: state.custom_preset,
    })
}

fn apply_cover_state(entity: &mut CoverEntity, state: esphome_client::types::CoverStateResponse) {
    if let Some(state) = cover_state(state) {
        entity.observation.observe(state);
    } else {
        entity.observation.gap();
    }
}

fn cover_state(state: esphome_client::types::CoverStateResponse) -> Option<CoverState> {
    (state.position.is_finite() && state.tilt.is_finite()).then_some(CoverState {
        position: state.position,
        tilt: state.tilt,
        operation: state.current_operation,
    })
}

#[derive(Clone, Copy)]
struct MeasurementProfile {
    profile: &'static str,
    canonical_unit: &'static str,
    scale: f64,
    offset: f64,
}

fn measurement_profile(device_class: &str, unit: &str) -> Option<MeasurementProfile> {
    let profile = |profile, canonical_unit, scale, offset| MeasurementProfile {
        profile,
        canonical_unit,
        scale,
        offset,
    };
    match (device_class, unit) {
        ("temperature", "°C" | "Cel") => Some(profile(PROFILE_TEMPERATURE, "K", 1.0, 273.15)),
        ("temperature", "°F" | "[degF]") => Some(profile(
            PROFILE_TEMPERATURE,
            "K",
            5.0 / 9.0,
            255.372_222_222_222_2,
        )),
        ("temperature", "K") => Some(profile(PROFILE_TEMPERATURE, "K", 1.0, 0.0)),
        ("humidity", "%") => Some(profile(PROFILE_HUMIDITY, "1", 0.01, 0.0)),
        ("pressure", "Pa") => Some(profile(PROFILE_PRESSURE, "Pa", 1.0, 0.0)),
        ("pressure", "hPa" | "mbar") => Some(profile(PROFILE_PRESSURE, "Pa", 100.0, 0.0)),
        ("pressure", "kPa") => Some(profile(PROFILE_PRESSURE, "Pa", 1_000.0, 0.0)),
        ("pressure", "bar") => Some(profile(PROFILE_PRESSURE, "Pa", 100_000.0, 0.0)),
        ("illuminance", "lx") => Some(profile(PROFILE_ILLUMINANCE, "lx", 1.0, 0.0)),
        ("power", "W") => Some(profile(PROFILE_POWER, "W", 1.0, 0.0)),
        ("power", "kW") => Some(profile(PROFILE_POWER, "W", 1_000.0, 0.0)),
        ("energy", "J") => Some(profile(PROFILE_ENERGY, "J", 1.0, 0.0)),
        ("energy", "Wh" | "W.h") => Some(profile(PROFILE_ENERGY, "J", 3_600.0, 0.0)),
        ("energy", "kWh" | "kW.h") => Some(profile(PROFILE_ENERGY, "J", 3_600_000.0, 0.0)),
        ("voltage", "V") => Some(profile(PROFILE_VOLTAGE, "V", 1.0, 0.0)),
        ("voltage", "mV") => Some(profile(PROFILE_VOLTAGE, "V", 0.001, 0.0)),
        ("current", "A") => Some(profile(PROFILE_CURRENT, "A", 1.0, 0.0)),
        ("current", "mA") => Some(profile(PROFILE_CURRENT, "A", 0.001, 0.0)),
        ("frequency", "Hz") => Some(profile(PROFILE_FREQUENCY, "Hz", 1.0, 0.0)),
        ("frequency", "kHz") => Some(profile(PROFILE_FREQUENCY, "Hz", 1_000.0, 0.0)),
        ("battery", "%") => Some(profile(PROFILE_BATTERY, "1", 0.01, 0.0)),
        ("signal_strength", "dBm" | "[dBm]") => Some(profile(PROFILE_SIGNAL, "[dBm]", 1.0, 0.0)),
        ("carbon_dioxide", "ppm" | "[ppm]") => Some(profile(PROFILE_CO2, "[ppm]", 1.0, 0.0)),
        ("carbon_monoxide", "ppm" | "[ppm]") => Some(profile(PROFILE_CO, "[ppm]", 1.0, 0.0)),
        ("volatile_organic_compounds", "ppm" | "[ppm]") => {
            Some(profile(PROFILE_VOC, "[ppm]", 1.0, 0.0))
        }
        ("pm1", "µg/m³" | "ug/m3") => Some(profile(PROFILE_PM1, "ug/m3", 1.0, 0.0)),
        ("pm2_5", "µg/m³" | "ug/m3") => Some(profile(PROFILE_PM2_5, "ug/m3", 1.0, 0.0)),
        ("pm10", "µg/m³" | "ug/m3") => Some(profile(PROFILE_PM10, "ug/m3", 1.0, 0.0)),
        ("distance", "m") => Some(profile(PROFILE_DISTANCE, "m", 1.0, 0.0)),
        ("distance", "cm") => Some(profile(PROFILE_DISTANCE, "m", 0.01, 0.0)),
        ("distance", "mm") => Some(profile(PROFILE_DISTANCE, "m", 0.001, 0.0)),
        ("distance", "km") => Some(profile(PROFILE_DISTANCE, "m", 1_000.0, 0.0)),
        ("speed", "m/s") => Some(profile(PROFILE_SPEED, "m/s", 1.0, 0.0)),
        ("speed", "km/h") => Some(profile(PROFILE_SPEED, "m/s", 1.0 / 3.6, 0.0)),
        ("volume", "m³" | "m3") => Some(profile(PROFILE_VOLUME, "m3", 1.0, 0.0)),
        ("volume", "L") => Some(profile(PROFILE_VOLUME, "m3", 0.001, 0.0)),
        ("volume", "mL") => Some(profile(PROFILE_VOLUME, "m3", 0.000_001, 0.0)),
        ("volume_flow_rate", "m³/s" | "m3/s") => Some(profile(PROFILE_FLOW, "m3/s", 1.0, 0.0)),
        ("volume_flow_rate", "m³/h" | "m3/h") => {
            Some(profile(PROFILE_FLOW, "m3/s", 1.0 / 3_600.0, 0.0))
        }
        ("volume_flow_rate", "L/min") => Some(profile(PROFILE_FLOW, "m3/s", 1.0 / 60_000.0, 0.0)),
        _ => None,
    }
}

fn measurement_kind(value: i32) -> Option<MeasurementKind> {
    match SensorStateClass::try_from(value).ok()? {
        SensorStateClass::StateClassNone | SensorStateClass::StateClassMeasurement => {
            Some(MeasurementKind::Instantaneous)
        }
        SensorStateClass::StateClassTotalIncreasing => Some(MeasurementKind::TotalIncreasing),
        SensorStateClass::StateClassTotal => Some(MeasurementKind::Total),
        SensorStateClass::StateClassMeasurementAngle => None,
    }
}

fn valid_temperature_descriptor(entity: &ListEntitiesClimateResponse) -> bool {
    entity.visual_min_temperature.is_finite()
        && entity.visual_max_temperature.is_finite()
        && entity.visual_target_temperature_step.is_finite()
        && entity.visual_min_temperature < entity.visual_max_temperature
        && entity.visual_target_temperature_step > 0.0
        && TemperatureUnit::try_from(entity.temperature_unit).is_ok()
}

fn climate_choices(
    standard: Vec<i32>,
    custom: Vec<String>,
    name: fn(i32) -> Option<&'static str>,
) -> BTreeMap<String, ClimateChoice> {
    let mut choices = BTreeMap::new();
    for value in standard.into_iter().take(MAX_CHOICES) {
        if let Some(choice) = name(value) {
            choices.insert(choice.into(), ClimateChoice::Standard(value));
        }
    }
    for value in custom
        .into_iter()
        .take(MAX_CHOICES.saturating_sub(choices.len()))
    {
        if !value.is_empty() && value.len() <= MAX_CHOICE_BYTES && !choices.contains_key(&value) {
            choices.insert(value.clone(), ClimateChoice::Custom(value));
        }
    }
    choices
}

fn fan_choice_name(value: i32) -> Option<&'static str> {
    [
        "on", "off", "auto", "low", "medium", "high", "middle", "focus", "diffuse", "quiet",
    ]
    .get(usize::try_from(value).ok()?)
    .copied()
}

fn swing_choice_name(value: i32) -> Option<&'static str> {
    ["off", "both", "vertical", "horizontal"]
        .get(usize::try_from(value).ok()?)
        .copied()
}

fn preset_choice_name(value: i32) -> Option<&'static str> {
    [
        "none", "home", "away", "boost", "comfort", "eco", "sleep", "activity",
    ]
    .get(usize::try_from(value).ok()?)
    .copied()
}

pub(crate) fn source_temperature_to_kelvin(value: f32, unit: i32) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }
    match TemperatureUnit::try_from(unit).ok()? {
        TemperatureUnit::Celsius => Some(value + 273.15),
        TemperatureUnit::Fahrenheit => Some((value - 32.0) * (5.0 / 9.0) + 273.15),
        TemperatureUnit::Kelvin => Some(value),
    }
}

pub(crate) fn kelvin_to_source_temperature(value: f32, unit: i32) -> Option<f32> {
    if !value.is_finite() {
        return None;
    }
    match TemperatureUnit::try_from(unit).ok()? {
        TemperatureUnit::Celsius => Some(value - 273.15),
        TemperatureUnit::Fahrenheit => Some((value - 273.15) * (9.0 / 5.0) + 32.0),
        TemperatureUnit::Kelvin => Some(value),
    }
}

pub(crate) fn decimal(value: f64) -> String {
    let mut result = format!("{value:.9}");
    while result.contains('.') && result.ends_with('0') {
        result.pop();
    }
    if result.ends_with('.') {
        result.pop();
    }
    if result == "-0" {
        result = "0".into();
    }
    result
}

#[allow(deprecated)]
fn empty_cover_command(locator: &Locator) -> CoverCommandRequest {
    CoverCommandRequest {
        key: locator.key,
        has_legacy_command: false,
        legacy_command: 0,
        has_position: false,
        position: 0.0,
        has_tilt: false,
        tilt: 0.0,
        stop: false,
        device_id: locator.device_id,
    }
}

#[allow(deprecated)]
fn empty_climate_command(locator: &Locator) -> ClimateCommandRequest {
    ClimateCommandRequest {
        key: locator.key,
        has_mode: false,
        mode: 0,
        has_target_temperature: false,
        target_temperature: 0.0,
        has_target_temperature_low: false,
        target_temperature_low: 0.0,
        has_target_temperature_high: false,
        target_temperature_high: 0.0,
        unused_has_legacy_away: false,
        unused_legacy_away: false,
        has_fan_mode: false,
        fan_mode: 0,
        has_swing_mode: false,
        swing_mode: 0,
        has_custom_fan_mode: false,
        custom_fan_mode: String::new(),
        has_preset: false,
        preset: 0,
        has_custom_preset: false,
        custom_preset: String::new(),
        has_target_humidity: false,
        target_humidity: 0.0,
        device_id: locator.device_id,
    }
}

fn empty_light_command(locator: &Locator, state: Option<bool>) -> LightCommandRequest {
    LightCommandRequest {
        key: locator.key,
        has_state: state.is_some(),
        state: state.unwrap_or(false),
        has_brightness: false,
        brightness: 0.0,
        has_color_mode: false,
        color_mode: 0,
        has_color_brightness: false,
        color_brightness: 0.0,
        has_rgb: false,
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        has_white: false,
        white: 0.0,
        has_color_temperature: false,
        color_temperature: 0.0,
        has_cold_white: false,
        cold_white: 0.0,
        has_warm_white: false,
        warm_white: 0.0,
        has_transition_length: false,
        transition_length: 0,
        has_flash_length: false,
        flash_length: 0,
        has_effect: false,
        effect: String::new(),
        device_id: locator.device_id,
    }
}

fn bounded_label(name: &str, object_id: &str) -> String {
    let source = if name.trim().is_empty() {
        object_id
    } else {
        name
    };
    let mut result = String::new();
    for character in source.chars() {
        if result.len() + character.len_utf8() > MAX_LABEL_BYTES {
            break;
        }
        result.push(character);
    }
    if result.is_empty() {
        "Unnamed entity".into()
    } else {
        result
    }
}

fn bounded_diagnostic_value(value: &str) -> String {
    value
        .chars()
        .take(64)
        .filter(|character| !character.is_control())
        .collect()
}

fn boolean_profile(device_class: &str, status: bool) -> BooleanProfile {
    if status {
        return BooleanProfile::Status;
    }
    match device_class {
        "door" | "garage_door" | "opening" | "window" => BooleanProfile::Contact,
        "motion" | "occupancy" | "presence" => BooleanProfile::Occupancy,
        "smoke" => BooleanProfile::Smoke,
        "moisture" => BooleanProfile::Moisture,
        "vibration" => BooleanProfile::Vibration,
        _ => BooleanProfile::Generic,
    }
}

pub(crate) fn now_unix_milliseconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
#[allow(
    deprecated,
    reason = "API 1.14 compatibility fixtures must populate retained fields"
)]
mod tests {
    use super::*;
    use esphome_client::types::{
        BinarySensorStateResponse, ClimateStateResponse, ColorMode, CoverStateResponse,
        LightStateResponse, ListEntitiesTextSensorResponse, SwitchStateResponse,
    };

    const VALID_KEY: &str = "AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=";

    #[test]
    fn config_requires_literal_socket_and_exact_noise_key_length() {
        let mut valid = format!(r#"{{"address":"192.0.2.10:6053","noise_key":"{VALID_KEY}"}}"#);
        let session = NodeSession::from_json(&mut valid).unwrap();
        assert_eq!(session.address, "192.0.2.10:6053");
        assert!(valid.chars().all(|character| character == '\0'));

        let mut hostname = format!(r#"{{"address":"node.local:6053","noise_key":"{VALID_KEY}"}}"#);
        assert_eq!(
            NodeSession::from_json(&mut hostname).err().unwrap(),
            SessionError::InvalidConfiguration
        );
        let mut short_key = r#"{"address":"192.0.2.10:6053","noise_key":"AA=="}"#.to_owned();
        assert_eq!(
            NodeSession::from_json(&mut short_key).err().unwrap(),
            SessionError::InvalidConfiguration
        );
    }

    #[test]
    fn source_locators_are_stable_and_group_all_light_facets() {
        let locator = Locator::new(EntityKind::Light, 7, 42);
        assert_eq!(locator.facet_key(), "light-7-42");
    }

    #[test]
    fn boolean_occurrence_queue_is_bounded_and_reports_overflow() {
        let mut config = format!(r#"{{"address":"192.0.2.10:6053","noise_key":"{VALID_KEY}"}}"#);
        let mut session = NodeSession::from_json(&mut config).unwrap();
        let locator = Locator::new(EntityKind::BinarySensor, 7, 42);
        session.binary_sensors.insert(
            locator.clone(),
            BinaryEntity {
                common: EntityCommon {
                    locator: locator.clone(),
                    label: "Motion".to_owned(),
                    incarnation: 1,
                },
                profile: BooleanProfile::Occupancy,
                observation: Observation::empty(),
            },
        );
        for index in 0..260 {
            session.apply_message(EspHomeMessage::BinarySensorStateResponse(
                BinarySensorStateResponse {
                    key: 42,
                    state: index % 2 == 0,
                    missing_state: false,
                    device_id: 7,
                },
            ));
        }

        let (first, dropped) = session.drain_boolean_occurrences(64);
        assert_eq!(first.len(), 64);
        assert_eq!(dropped, 4);
        assert_eq!(first[0].sequence, 5);
        let (remaining, dropped) = session.drain_boolean_occurrences(256);
        assert_eq!(remaining.len(), 192);
        assert_eq!(dropped, 0);
    }

    #[test]
    fn occurrence_refresh_reports_lost_transport_continuity() {
        let mut config = format!(r#"{{"address":"192.0.2.10:6053","noise_key":"{VALID_KEY}"}}"#);
        let mut session = NodeSession::from_json(&mut config).unwrap();
        session.mark_disconnected();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        assert_eq!(
            runtime.block_on(session.refresh_occurrences()),
            Err(SessionError::ConnectionFailed)
        );
    }

    #[test]
    fn exact_measurement_whitelist_normalizes_to_catalog_profiles() {
        let temperature = measurement_profile("temperature", "°C").unwrap();
        assert_eq!(temperature.profile, PROFILE_TEMPERATURE);
        assert_eq!(temperature.canonical_unit, "K");
        assert_eq!(
            decimal(25.0_f64.mul_add(temperature.scale, temperature.offset)),
            "298.15"
        );
        let humidity = measurement_profile("humidity", "%").unwrap();
        assert_eq!(humidity.profile, PROFILE_HUMIDITY);
        assert_eq!(humidity.canonical_unit, "1");
        assert!(measurement_profile("duration", "s").is_none());
        assert!(measurement_profile("unknown", "°C").is_none());
    }

    #[test]
    fn topology_maps_all_supported_families_and_bounds_unsupported_entities() {
        let mut topology = Topology::default();
        topology.apply(EspHomeMessage::ListEntitiesSwitchResponse(
            ListEntitiesSwitchResponse {
                object_id: "relay".into(),
                key: 1,
                name: "Relay".into(),
                icon: String::new(),
                assumed_state: false,
                disabled_by_default: false,
                entity_category: 0,
                device_class: "switch".into(),
                device_id: 1,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesBinarySensorResponse(
            ListEntitiesBinarySensorResponse {
                object_id: "door".into(),
                key: 2,
                name: "Door".into(),
                device_class: "door".into(),
                is_status_binary_sensor: false,
                disabled_by_default: false,
                icon: String::new(),
                entity_category: 0,
                device_id: 1,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesSensorResponse(
            ListEntitiesSensorResponse {
                object_id: "temperature".into(),
                key: 3,
                name: "Temperature".into(),
                icon: String::new(),
                unit_of_measurement: "°C".into(),
                accuracy_decimals: 1,
                force_update: false,
                device_class: "temperature".into(),
                state_class: 1,
                legacy_last_reset_type: 0,
                disabled_by_default: false,
                entity_category: 0,
                device_id: 1,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesLightResponse(
            ListEntitiesLightResponse {
                object_id: "lamp".into(),
                key: 4,
                name: "Lamp".into(),
                supported_color_modes: vec![
                    ColorMode::Brightness as i32,
                    ColorMode::Rgb as i32,
                    ColorMode::ColorTemperature as i32,
                ],
                legacy_supports_brightness: false,
                legacy_supports_rgb: false,
                legacy_supports_white_value: false,
                legacy_supports_color_temperature: false,
                min_mireds: 153.0,
                max_mireds: 500.0,
                effects: vec![],
                disabled_by_default: false,
                icon: String::new(),
                entity_category: 0,
                device_id: 1,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesFanResponse(
            ListEntitiesFanResponse {
                object_id: "fan".into(),
                key: 5,
                name: "Fan".into(),
                supports_oscillation: false,
                supports_speed: true,
                supports_direction: false,
                supported_speed_count: 3,
                disabled_by_default: false,
                icon: String::new(),
                entity_category: 0,
                supported_preset_modes: vec![],
                device_id: 1,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesSirenResponse(
            ListEntitiesSirenResponse {
                object_id: "siren".into(),
                key: 6,
                name: "Siren".into(),
                icon: String::new(),
                disabled_by_default: false,
                tones: vec![],
                supports_duration: false,
                supports_volume: false,
                entity_category: 0,
                device_id: 1,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesClimateResponse(
            ListEntitiesClimateResponse {
                object_id: "hvac".into(),
                key: 7,
                name: "HVAC".into(),
                supports_current_temperature: true,
                supports_two_point_target_temperature: true,
                supported_modes: vec![0, 1, 2, 3],
                visual_min_temperature: 10.0,
                visual_max_temperature: 30.0,
                visual_target_temperature_step: 0.5,
                legacy_supports_away: false,
                supports_action: true,
                supported_fan_modes: vec![2, 5],
                supported_swing_modes: vec![0, 2],
                supported_custom_fan_modes: vec![],
                supported_presets: vec![0, 5],
                supported_custom_presets: vec![],
                disabled_by_default: false,
                icon: String::new(),
                entity_category: 0,
                visual_current_temperature_step: 0.1,
                supports_current_humidity: false,
                supports_target_humidity: false,
                visual_min_humidity: 0.0,
                visual_max_humidity: 0.0,
                device_id: 1,
                feature_flags: 0,
                temperature_unit: TemperatureUnit::Celsius as i32,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesCoverResponse(
            ListEntitiesCoverResponse {
                object_id: "blind".into(),
                key: 8,
                name: "Blind".into(),
                assumed_state: false,
                supports_position: true,
                supports_tilt: true,
                device_class: "blind".into(),
                disabled_by_default: false,
                icon: String::new(),
                entity_category: 0,
                supports_stop: true,
                device_id: 1,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesButtonResponse(
            ListEntitiesButtonResponse {
                object_id: "identify".into(),
                key: 9,
                name: "Identify".into(),
                icon: String::new(),
                disabled_by_default: false,
                entity_category: 0,
                device_class: "identify".into(),
                device_id: 1,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesTextSensorResponse(
            ListEntitiesTextSensorResponse {
                object_id: "version".into(),
                key: 10,
                name: "Version".into(),
                icon: String::new(),
                disabled_by_default: false,
                entity_category: 0,
                device_class: String::new(),
                device_id: 1,
            },
        ));
        topology.apply(EspHomeMessage::ListEntitiesSensorResponse(
            ListEntitiesSensorResponse {
                object_id: "unsupported".into(),
                key: 11,
                name: "Unsupported".into(),
                icon: String::new(),
                unit_of_measurement: "furlong".into(),
                accuracy_decimals: 0,
                force_update: false,
                device_class: "distance".into(),
                state_class: 1,
                legacy_last_reset_type: 0,
                disabled_by_default: false,
                entity_category: 0,
                device_id: 1,
            },
        ));
        assert_eq!(topology.switches.len(), 1);
        assert_eq!(topology.binary_sensors.len(), 1);
        assert_eq!(topology.sensors.len(), 1);
        assert_eq!(topology.lights.len(), 1);
        assert_eq!(topology.fans.len(), 1);
        assert_eq!(topology.sirens.len(), 1);
        assert_eq!(topology.climates.len(), 1);
        assert_eq!(topology.covers.len(), 1);
        assert_eq!(topology.buttons.len(), 1);
        let light = topology.lights.values().next().unwrap();
        assert!(light.supports_level);
        assert_eq!(light.rgb_mode, Some(ColorMode::Rgb as i32));
        assert_eq!(
            light.color_temperature_mode,
            Some(ColorMode::ColorTemperature as i32)
        );
        let climate = topology.climates.values().next().unwrap();
        assert!(climate.dual_target);
        assert_eq!(climate.fan_choices.len(), 2);
        assert_eq!(climate.swing_choices.len(), 2);
        assert_eq!(climate.preset_choices.len(), 2);
        let cover = topology.covers.values().next().unwrap();
        assert!(cover.supports_position && cover.supports_tilt && cover.supports_stop);
        assert_eq!(topology.fans.values().next().unwrap().speed_count, Some(3));
        assert!(
            topology
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.entity_type == "text-sensor")
        );
        assert!(topology.diagnostics.iter().any(|diagnostic| {
            diagnostic.source_locator == "sensor-1-11"
                && diagnostic.reason == "unsupported-unit:furlong"
        }));
    }

    #[test]
    fn states_before_descriptors_hydrate_without_gap() {
        let mut topology = Topology::default();
        topology.apply(EspHomeMessage::SwitchStateResponse(SwitchStateResponse {
            key: 1,
            state: true,
            device_id: 2,
        }));
        topology.apply(EspHomeMessage::ListEntitiesSwitchResponse(
            ListEntitiesSwitchResponse {
                object_id: "relay".into(),
                key: 1,
                name: "Relay".into(),
                icon: String::new(),
                assumed_state: false,
                disabled_by_default: false,
                entity_category: 0,
                device_class: String::new(),
                device_id: 2,
            },
        ));
        let observation = &topology.switches.values().next().unwrap().observation;
        assert_eq!(observation.current, Some(true));
        assert_eq!(observation.status, HydrationStatus::Hydrated);
    }

    #[test]
    fn reconnect_gap_preserves_cache_and_topology_changes_reincarnate() {
        let mut current = BTreeMap::new();
        let locator = Locator::new(EntityKind::Switch, 1, 1);
        let mut old = SwitchEntity {
            common: EntityCommon {
                locator: locator.clone(),
                label: "Relay".into(),
                incarnation: 1,
            },
            assumed_state: false,
            observation: Observation::empty(),
        };
        old.observation.observe(true);
        current.insert(locator.clone(), old);
        let mut counters = BTreeMap::from([(locator.clone(), 1)]);
        let same = SwitchEntity {
            common: EntityCommon {
                locator: locator.clone(),
                label: "Relay".into(),
                incarnation: 0,
            },
            assumed_state: false,
            observation: Observation::empty(),
        };
        assert!(!reconcile_map(
            &mut current,
            BTreeMap::from([(locator.clone(), same)]),
            &mut counters
        ));
        assert_eq!(current[&locator].observation.current, Some(true));
        assert_eq!(current[&locator].observation.status, HydrationStatus::Gap);
        let changed = SwitchEntity {
            common: EntityCommon {
                locator: locator.clone(),
                label: "Renamed".into(),
                incarnation: 0,
            },
            assumed_state: false,
            observation: Observation::empty(),
        };
        assert!(reconcile_map(
            &mut current,
            BTreeMap::from([(locator.clone(), changed)]),
            &mut counters
        ));
        assert_eq!(current[&locator].common.incarnation, 2);
        assert_eq!(current[&locator].observation.current, None);
    }

    #[test]
    fn profile_mapping_is_conservative() {
        assert_eq!(boolean_profile("door", false), BooleanProfile::Contact);
        assert_eq!(boolean_profile("motion", false), BooleanProfile::Occupancy);
        assert_eq!(boolean_profile("unknown", true), BooleanProfile::Status);
        assert_eq!(boolean_profile("unknown", false), BooleanProfile::Generic);
    }

    #[test]
    fn labels_are_nonempty_and_utf8_bounded() {
        assert_eq!(bounded_label("", "relay_1"), "relay_1");
        assert_eq!(bounded_label("", ""), "Unnamed entity");
        let label = bounded_label(&"é".repeat(100), "fallback");
        assert!(label.len() <= MAX_LABEL_BYTES);
        assert!(label.is_char_boundary(label.len()));
    }

    #[test]
    fn state_helpers_reject_non_finite_values() {
        assert!(
            light_state(LightStateResponse {
                key: 1,
                state: true,
                brightness: f32::NAN,
                color_mode: 1,
                color_brightness: 0.0,
                red: 0.0,
                green: 0.0,
                blue: 0.0,
                white: 0.0,
                color_temperature: 0.0,
                cold_white: 0.0,
                warm_white: 0.0,
                effect: String::new(),
                device_id: 1
            })
            .is_none()
        );
        assert!(
            cover_state(CoverStateResponse {
                key: 1,
                legacy_state: 0,
                position: f32::INFINITY,
                tilt: 0.0,
                current_operation: 0,
                device_id: 1
            })
            .is_none()
        );
        assert!(
            climate_state(ClimateStateResponse {
                key: 1,
                mode: 0,
                current_temperature: 20.0,
                target_temperature: f32::NAN,
                target_temperature_low: 18.0,
                target_temperature_high: 24.0,
                unused_legacy_away: false,
                action: 0,
                fan_mode: 0,
                swing_mode: 0,
                custom_fan_mode: String::new(),
                preset: 0,
                custom_preset: String::new(),
                current_humidity: 0.0,
                target_humidity: 0.0,
                device_id: 1
            })
            .is_none()
        );
    }
}
