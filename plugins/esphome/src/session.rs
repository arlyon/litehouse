use base64::Engine as _;
use esphome_client::{
    EspHomeClient,
    types::{
        EspHomeMessage, ListEntitiesBinarySensorResponse, ListEntitiesRequest,
        ListEntitiesSwitchResponse, SubscribeStatesRequest, SwitchCommandRequest,
    },
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectorConfig {
    address: String,
    noise_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Locator {
    pub(crate) device_id: u32,
    pub(crate) key: u32,
}

impl Locator {
    pub(crate) fn switch_key(&self) -> String {
        format!("switch-{}-{}", self.device_id, self.key)
    }

    pub(crate) fn binary_key(&self) -> String {
        format!("binary-{}-{}", self.device_id, self.key)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SwitchEntity {
    pub(crate) locator: Locator,
    pub(crate) label: String,
    pub(crate) assumed_state: bool,
    pub(crate) current: Option<bool>,
    pub(crate) revision: u64,
    pub(crate) received_at_unix_milliseconds: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct BinaryEntity {
    pub(crate) locator: Locator,
    pub(crate) label: String,
    pub(crate) profile: BooleanProfile,
    pub(crate) current: Option<bool>,
    pub(crate) revision: u64,
    pub(crate) received_at_unix_milliseconds: u64,
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
pub(crate) enum SessionError {
    InvalidConfiguration,
    ConnectionFailed,
    DiscoveryFailed,
    Unavailable,
    OutcomeUnknown,
}

pub(crate) struct NodeSession {
    address: String,
    noise_key: Zeroizing<String>,
    client: Option<EspHomeClient>,
    switches: BTreeMap<Locator, SwitchEntity>,
    binary_sensors: BTreeMap<Locator, BinaryEntity>,
    connected: bool,
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
            connected: false,
        })
    }

    pub(crate) async fn start(&mut self) -> Result<(), SessionError> {
        self.connect_and_discover().await
    }

    pub(crate) fn switches(&self) -> impl Iterator<Item = &SwitchEntity> {
        self.switches.values()
    }

    pub(crate) fn binary_sensors(&self) -> impl Iterator<Item = &BinaryEntity> {
        self.binary_sensors.values()
    }

    pub(crate) fn switch(&self, locator: &Locator) -> Option<&SwitchEntity> {
        self.switches.get(locator)
    }

    pub(crate) fn binary_sensor(&self, locator: &Locator) -> Option<&BinaryEntity> {
        self.binary_sensors.get(locator)
    }

    pub(crate) fn connected(&self) -> bool {
        self.connected
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

    pub(crate) async fn set_switch(
        &mut self,
        locator: &Locator,
        value: bool,
    ) -> Result<(), SessionError> {
        if !self.switches.contains_key(locator) {
            return Err(SessionError::Unavailable);
        }
        if self.client.is_none() {
            self.connect_and_discover()
                .await
                .map_err(|_| SessionError::Unavailable)?;
        }
        let Some(client) = self.client.as_mut() else {
            return Err(SessionError::Unavailable);
        };
        if client
            .try_write(SwitchCommandRequest {
                key: locator.key,
                state: value,
                device_id: locator.device_id,
            })
            .await
            .is_err()
        {
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
                if topology.observed_count() >= topology.entity_count() {
                    break;
                }
                state_grace_deadline
                    .get_or_insert_with(|| tokio::time::Instant::now() + INITIAL_STATE_GRACE);
            }
        }
        self.reconcile(topology);
        self.connected = true;
        self.client = Some(client);
        Ok(())
    }

    fn reconcile(&mut self, mut topology: Topology) {
        for (locator, entity) in &mut topology.switches {
            if entity.current.is_none()
                && let Some(previous) = self.switches.get(locator)
            {
                entity.current = previous.current;
                entity.revision = previous.revision;
                entity.received_at_unix_milliseconds = previous.received_at_unix_milliseconds;
            }
        }
        for (locator, entity) in &mut topology.binary_sensors {
            if entity.current.is_none()
                && let Some(previous) = self.binary_sensors.get(locator)
            {
                entity.current = previous.current;
                entity.revision = previous.revision;
                entity.received_at_unix_milliseconds = previous.received_at_unix_milliseconds;
            }
        }
        for (locator, previous) in &self.switches {
            topology
                .switches
                .entry(locator.clone())
                .or_insert_with(|| previous.clone());
        }
        for (locator, previous) in &self.binary_sensors {
            topology
                .binary_sensors
                .entry(locator.clone())
                .or_insert_with(|| previous.clone());
        }
        self.switches = topology.switches;
        self.binary_sensors = topology.binary_sensors;
    }

    fn apply_message(&mut self, message: EspHomeMessage) {
        match message {
            EspHomeMessage::SwitchStateResponse(state) => {
                let locator = Locator {
                    device_id: state.device_id,
                    key: state.key,
                };
                if let Some(entity) = self.switches.get_mut(&locator) {
                    observe(
                        &mut entity.current,
                        &mut entity.revision,
                        &mut entity.received_at_unix_milliseconds,
                        state.state,
                    );
                }
            }
            EspHomeMessage::BinarySensorStateResponse(state) if !state.missing_state => {
                let locator = Locator {
                    device_id: state.device_id,
                    key: state.key,
                };
                if let Some(entity) = self.binary_sensors.get_mut(&locator) {
                    observe(
                        &mut entity.current,
                        &mut entity.revision,
                        &mut entity.received_at_unix_milliseconds,
                        state.state,
                    );
                }
            }
            _ => {}
        }
    }

    fn mark_disconnected(&mut self) {
        self.connected = false;
    }
}

#[derive(Default)]
struct Topology {
    switches: BTreeMap<Locator, SwitchEntity>,
    binary_sensors: BTreeMap<Locator, BinaryEntity>,
    pending_switch_states: BTreeMap<Locator, bool>,
    pending_binary_states: BTreeMap<Locator, bool>,
    listing_done: bool,
}

impl Topology {
    fn apply(&mut self, message: EspHomeMessage) {
        match message {
            EspHomeMessage::ListEntitiesSwitchResponse(entity) if entity.entity_category == 0 => {
                self.add_switch(entity);
            }
            EspHomeMessage::ListEntitiesBinarySensorResponse(entity)
                if entity.entity_category == 0 =>
            {
                self.add_binary(entity);
            }
            EspHomeMessage::SwitchStateResponse(state) => {
                let locator = Locator {
                    device_id: state.device_id,
                    key: state.key,
                };
                if let Some(entity) = self.switches.get_mut(&locator) {
                    observe(
                        &mut entity.current,
                        &mut entity.revision,
                        &mut entity.received_at_unix_milliseconds,
                        state.state,
                    );
                } else {
                    self.pending_switch_states.insert(locator, state.state);
                }
            }
            EspHomeMessage::BinarySensorStateResponse(state) if !state.missing_state => {
                let locator = Locator {
                    device_id: state.device_id,
                    key: state.key,
                };
                if let Some(entity) = self.binary_sensors.get_mut(&locator) {
                    observe(
                        &mut entity.current,
                        &mut entity.revision,
                        &mut entity.received_at_unix_milliseconds,
                        state.state,
                    );
                } else {
                    self.pending_binary_states.insert(locator, state.state);
                }
            }
            EspHomeMessage::ListEntitiesDoneResponse(_) => self.listing_done = true,
            _ => {}
        }
    }

    fn add_switch(&mut self, entity: ListEntitiesSwitchResponse) {
        let locator = Locator {
            device_id: entity.device_id,
            key: entity.key,
        };
        let mut result = SwitchEntity {
            locator: locator.clone(),
            label: bounded_label(&entity.name, &entity.object_id),
            assumed_state: entity.assumed_state,
            current: None,
            revision: 0,
            received_at_unix_milliseconds: now_unix_milliseconds(),
        };
        if let Some(value) = self.pending_switch_states.remove(&locator) {
            observe(
                &mut result.current,
                &mut result.revision,
                &mut result.received_at_unix_milliseconds,
                value,
            );
        }
        self.switches.insert(locator, result);
    }

    fn add_binary(&mut self, entity: ListEntitiesBinarySensorResponse) {
        let locator = Locator {
            device_id: entity.device_id,
            key: entity.key,
        };
        let mut result = BinaryEntity {
            locator: locator.clone(),
            label: bounded_label(&entity.name, &entity.object_id),
            profile: boolean_profile(&entity.device_class, entity.is_status_binary_sensor),
            current: None,
            revision: 0,
            received_at_unix_milliseconds: now_unix_milliseconds(),
        };
        if let Some(value) = self.pending_binary_states.remove(&locator) {
            observe(
                &mut result.current,
                &mut result.revision,
                &mut result.received_at_unix_milliseconds,
                value,
            );
        }
        self.binary_sensors.insert(locator, result);
    }

    fn entity_count(&self) -> usize {
        self.switches.len() + self.binary_sensors.len()
    }

    fn observed_count(&self) -> usize {
        self.switches
            .values()
            .filter(|entity| entity.current.is_some())
            .count()
            + self
                .binary_sensors
                .values()
                .filter(|entity| entity.current.is_some())
                .count()
    }
}

fn observe(current: &mut Option<bool>, revision: &mut u64, received_at: &mut u64, value: bool) {
    *current = Some(value);
    *revision = revision.saturating_add(1);
    *received_at = now_unix_milliseconds();
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
mod tests {
    use super::*;

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
    fn source_locators_are_stable_and_management_safe() {
        let locator = Locator {
            device_id: 7,
            key: 42,
        };
        assert_eq!(locator.switch_key(), "switch-7-42");
        assert_eq!(locator.binary_key(), "binary-7-42");
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
}
