use esphome_capability_spike::{ProviderInstance, SourceEntity};
use esphome_client::{
    EspHomeClient,
    types::{DeviceInfoRequest, EspHomeMessage, ListEntitiesRequest, SubscribeStatesRequest},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::Read,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::time::{Instant, timeout};
use zeroize::{Zeroize, ZeroizeOnDrop};

const CLIENT_INFO: &str = "litehouse-esphome-proof/0";
const READ_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct CredentialFile {
    node_s: Target,
    node_a: Target,
}

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Target {
    address: String,
    noise_key: String,
}

#[derive(Default, Serialize)]
struct Summary {
    alias: &'static str,
    esphome_version: String,
    switches: usize,
    binary_sensors: usize,
    measurements: usize,
    diagnostic_entities: usize,
    observed_states: usize,
    stable_after_reconnect: bool,
}

struct Snapshot {
    version: String,
    topology: Vec<SourceEntity>,
    switches: usize,
    binary_sensors: usize,
    diagnostic_entities: usize,
    observations: Vec<(String, Observation)>,
}

enum Observation {
    Boolean(bool),
    Measurement(f32),
}

#[tokio::main]
async fn main() {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "fixtures".into());
    let target = args.next();
    let result = match command.as_str() {
        "fixtures" => {
            println!("fixture model is validated by `cargo test`");
            Ok(())
        }
        "preflight" => preflight(target.as_deref()).await,
        _ => Err("usage: esphome-capability-spike [fixtures|preflight [node-s|node-a]]".into()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

async fn preflight(selected: Option<&str>) -> Result<(), String> {
    let path = credential_path()?;
    let mut credentials = read_credentials(&path)?;

    let mut summaries = Vec::new();
    match selected {
        None => {
            summaries.push(preflight_target("node-s", &credentials.node_s).await?);
            summaries.push(preflight_target("node-a", &credentials.node_a).await?);
        }
        Some("node-s") => summaries.push(preflight_target("node-s", &credentials.node_s).await?),
        Some("node-a") => summaries.push(preflight_target("node-a", &credentials.node_a).await?),
        Some(_) => return Err("preflight target must be node-s or node-a".into()),
    }

    credentials.zeroize();
    println!(
        "{}",
        serde_json::to_string_pretty(&summaries)
            .map_err(|_| "could not serialize redacted preflight summary")?
    );
    Ok(())
}

async fn preflight_target(alias: &'static str, target: &Target) -> Result<Summary, String> {
    let first = read_snapshot(alias, target).await?;
    let mut provider = ProviderInstance::new(alias);
    provider.apply_topology(&first.topology);
    apply_observations(&mut provider, &first.observations);
    let first_ids = provider.facets().map(|facet| facet.stable_id.clone()).collect::<Vec<_>>();
    provider.disconnect();

    let second = read_snapshot(alias, target).await?;
    provider.apply_topology(&second.topology);
    apply_observations(&mut provider, &second.observations);
    let second_ids = provider.facets().map(|facet| facet.stable_id.clone()).collect::<Vec<_>>();

    Ok(Summary {
        alias,
        esphome_version: second.version,
        switches: second.switches,
        binary_sensors: second.binary_sensors,
        measurements: provider
            .facets()
            .filter(|facet| facet.capabilities.iter().any(|capability| capability.starts_with("measurement:")))
            .count(),
        diagnostic_entities: second.diagnostic_entities,
        observed_states: second.observations.len(),
        stable_after_reconnect: first_ids == second_ids,
    })
}

async fn read_snapshot(alias: &str, target: &Target) -> Result<Snapshot, String> {
    let mut client = EspHomeClient::builder()
        .address(&target.address)
        .key(&target.noise_key)
        .client_info(CLIENT_INFO)
        .timeout(Duration::from_secs(5))
        .connect()
        .await
        .map_err(|_| format!("{alias}: encrypted connection failed"))?;

    client
        .try_write(DeviceInfoRequest {})
        .await
        .map_err(|_| format!("{alias}: device-info request failed"))?;
    client
        .try_write(ListEntitiesRequest {})
        .await
        .map_err(|_| format!("{alias}: entity-list request failed"))?;
    client
        .try_write(SubscribeStatesRequest {})
        .await
        .map_err(|_| format!("{alias}: state subscription failed"))?;

    let mut version = None;
    let mut topology = Vec::new();
    let mut locator_by_key = BTreeMap::new();
    let mut switches = 0;
    let mut binary_sensors = 0;
    let mut diagnostic_entities = 0;
    let mut observations = Vec::new();
    let mut listing_done = false;
    let deadline = Instant::now() + READ_TIMEOUT;

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let message = timeout(remaining, client.try_read())
            .await
            .map_err(|_| format!("{alias}: preflight timed out"))?
            .map_err(|_| format!("{alias}: protocol read failed"))?;
        match message {
            EspHomeMessage::DeviceInfoResponse(info) => version = Some(info.esphome_version),
            EspHomeMessage::ListEntitiesSwitchResponse(entity) => {
                if entity.entity_category != 0 {
                    diagnostic_entities += 1;
                    continue;
                }
                switches += 1;
                let locator = format!("switch:{}:{}", entity.device_id, entity.key);
                locator_by_key.insert(("switch", entity.device_id, entity.key), locator.clone());
                topology.push(SourceEntity::Switch { locator, group: None });
            }
            EspHomeMessage::ListEntitiesBinarySensorResponse(entity) => {
                if entity.entity_category != 0 {
                    diagnostic_entities += 1;
                    continue;
                }
                binary_sensors += 1;
                let locator = format!("binary:{}:{}", entity.device_id, entity.key);
                locator_by_key.insert(("binary", entity.device_id, entity.key), locator.clone());
                topology.push(SourceEntity::BinarySensor { locator, group: None });
            }
            EspHomeMessage::ListEntitiesSensorResponse(entity) => {
                if entity.entity_category != 0 {
                    diagnostic_entities += 1;
                    continue;
                }
                let locator = format!("sensor:{}:{}", entity.device_id, entity.key);
                locator_by_key.insert(("sensor", entity.device_id, entity.key), locator.clone());
                topology.push(SourceEntity::Sensor {
                    locator,
                    device_class: entity.device_class,
                    unit: entity.unit_of_measurement,
                    group: None,
                });
            }
            EspHomeMessage::ListEntitiesDoneResponse(_) => {
                listing_done = true;
                if version.is_some() && !observations.is_empty() {
                    break;
                }
            }
            EspHomeMessage::SwitchStateResponse(state) => {
                if let Some(locator) = locator_by_key.get(&("switch", state.device_id, state.key)) {
                    observations.push((locator.clone(), Observation::Boolean(state.state)));
                }
            }
            EspHomeMessage::BinarySensorStateResponse(state) => {
                if !state.missing_state
                    && let Some(locator) = locator_by_key.get(&("binary", state.device_id, state.key))
                {
                    observations.push((locator.clone(), Observation::Boolean(state.state)));
                }
            }
            EspHomeMessage::SensorStateResponse(state) => {
                if !state.missing_state
                    && let Some(locator) = locator_by_key.get(&("sensor", state.device_id, state.key))
                {
                    observations.push((locator.clone(), Observation::Measurement(state.state)));
                }
            }
            EspHomeMessage::ListEntitiesLightResponse(_)
            | EspHomeMessage::ListEntitiesFanResponse(_)
            | EspHomeMessage::ListEntitiesCoverResponse(_)
            | EspHomeMessage::ListEntitiesNumberResponse(_)
            | EspHomeMessage::ListEntitiesSelectResponse(_)
            | EspHomeMessage::ListEntitiesButtonResponse(_)
            | EspHomeMessage::ListEntitiesTextResponse(_) => diagnostic_entities += 1,
            _ => {}
        }
        if listing_done && version.is_some() && observations.len() >= switches + binary_sensors {
            break;
        }
    }

    let _ = client.close().await;
    let version = version.ok_or_else(|| format!("{alias}: no device version received"))?;
    if !listing_done {
        return Err(format!("{alias}: entity listing did not finish"));
    }
    Ok(Snapshot {
        version,
        topology,
        switches,
        binary_sensors,
        diagnostic_entities,
        observations,
    })
}

fn apply_observations(provider: &mut ProviderInstance, observations: &[(String, Observation)]) {
    for (locator, observation) in observations {
        match observation {
            Observation::Boolean(value) => {
                provider.observe_boolean(locator, *value);
            }
            Observation::Measurement(value) => {
                provider.observe_measurement(locator, *value);
            }
        }
    }
}

fn credential_path() -> Result<PathBuf, String> {
    let runtime = env::var_os("XDG_RUNTIME_DIR").ok_or("XDG_RUNTIME_DIR is not set")?;
    Ok(PathBuf::from(runtime).join("litehouse-esphome-proof/targets.json"))
}

fn read_credentials(path: &Path) -> Result<CredentialFile, String> {
    let link_metadata = fs::symlink_metadata(path).map_err(|_| "credential file is unavailable")?;
    if link_metadata.file_type().is_symlink() || !link_metadata.file_type().is_file() {
        return Err("credential path must be a regular non-symlink file".into());
    }
    if link_metadata.mode() & 0o777 != 0o600 {
        return Err("credential file mode must be exactly 0600".into());
    }

    let file = File::open(path).map_err(|_| "credential file could not be opened")?;
    let opened = file.metadata().map_err(|_| "credential metadata is unavailable")?;
    if opened.dev() != link_metadata.dev() || opened.ino() != link_metadata.ino() {
        return Err("credential file changed during open".into());
    }
    let mut json = String::new();
    file.take(16 * 1024)
        .read_to_string(&mut json)
        .map_err(|_| "credential file could not be read")?;
    let result = serde_json::from_str(&json).map_err(|_| "credential file is invalid".to_owned());
    json.zeroize();
    result
}
