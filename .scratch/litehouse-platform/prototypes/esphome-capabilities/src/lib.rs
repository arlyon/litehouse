use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceEntity {
    Switch { locator: String, group: Option<String> },
    BinarySensor { locator: String, group: Option<String> },
    Sensor {
        locator: String,
        device_class: String,
        unit: String,
        group: Option<String>,
    },
    Light {
        locator: String,
        brightness: bool,
        rgb: bool,
        group: Option<String>,
    },
    Diagnostic { locator: String, kind: String },
}

impl SourceEntity {
    fn locator(&self) -> &str {
        match self {
            Self::Switch { locator, .. }
            | Self::BinarySensor { locator, .. }
            | Self::Sensor { locator, .. }
            | Self::Light { locator, .. }
            | Self::Diagnostic { locator, .. } => locator,
        }
    }

    fn group(&self) -> Option<&str> {
        match self {
            Self::Switch { group, .. }
            | Self::BinarySensor { group, .. }
            | Self::Sensor { group, .. }
            | Self::Light { group, .. } => group.as_deref(),
            Self::Diagnostic { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Capability {
    OnOff,
    BooleanState,
    Level,
    Color,
    Measurement(MeasurementProfile),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasurementProfile {
    pub quantity: &'static str,
    pub canonical_unit: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Quality {
    Unknown,
    Observed,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservedValue {
    Boolean(bool),
    Decimal { value: String, unit: &'static str },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Facet {
    pub stable_id: String,
    pub capabilities: BTreeSet<String>,
    pub group: Option<String>,
    pub incarnation: u64,
    pub quality: Quality,
    pub value: Option<ObservedValue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandReceipt {
    AcceptedForDelivery {
        facet_id: String,
        target: bool,
        confirmed: bool,
    },
    Rejected { facet_id: String, reason: &'static str },
}

#[derive(Debug)]
pub struct ProviderInstance {
    alias: String,
    generation: u64,
    next_identity: u64,
    identities: BTreeMap<String, String>,
    facets: BTreeMap<String, Facet>,
}

impl ProviderInstance {
    pub fn new(alias: &str) -> Self {
        Self {
            alias: alias.into(),
            generation: 1,
            next_identity: 1,
            identities: BTreeMap::new(),
            facets: BTreeMap::new(),
        }
    }

    pub fn alias(&self) -> &str {
        &self.alias
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn facets(&self) -> impl Iterator<Item = &Facet> {
        self.facets.values()
    }

    pub fn apply_topology(&mut self, entities: &[SourceEntity]) {
        let mut next = BTreeMap::new();
        for source in entities {
            let capabilities = map_capabilities(source);
            if capabilities.is_empty() {
                continue;
            }
            let locator = source.locator();
            let stable_id = match self.identities.get(locator) {
                Some(id) => id.clone(),
                None => {
                    let id = format!("{}-facet-{}", self.alias, self.next_identity);
                    self.next_identity += 1;
                    self.identities.insert(locator.into(), id.clone());
                    id
                }
            };
            let previous = self.facets.get(&stable_id);
            next.insert(
                stable_id.clone(),
                Facet {
                    stable_id,
                    capabilities: capabilities
                        .into_iter()
                        .map(capability_name)
                        .map(str::to_owned)
                        .collect(),
                    group: source.group().map(str::to_owned),
                    incarnation: previous.map_or(1, |facet| facet.incarnation),
                    quality: previous.map_or(Quality::Unknown, |facet| facet.quality),
                    value: previous.and_then(|facet| facet.value.clone()),
                },
            );
        }
        self.facets = next;
    }

    pub fn observe_boolean(&mut self, locator: &str, value: bool) -> bool {
        self.update(locator, ObservedValue::Boolean(value))
    }

    pub fn observe_measurement(&mut self, locator: &str, value: f32) -> bool {
        let Some(stable_id) = self.identities.get(locator) else {
            return false;
        };
        let Some(facet) = self.facets.get_mut(stable_id) else {
            return false;
        };
        let Some(unit) = facet.capabilities.iter().find_map(|capability| {
            capability.strip_prefix("measurement:").and_then(|profile| profile.split_once(':')).map(|(_, unit)| unit)
        }) else {
            return false;
        };
        let mut buffer = ryu::Buffer::new();
        facet.value = Some(ObservedValue::Decimal {
            value: buffer.format_finite(value).to_owned(),
            unit: canonical_unit(unit),
        });
        facet.quality = Quality::Observed;
        true
    }

    fn update(&mut self, locator: &str, value: ObservedValue) -> bool {
        let Some(stable_id) = self.identities.get(locator) else {
            return false;
        };
        let Some(facet) = self.facets.get_mut(stable_id) else {
            return false;
        };
        facet.value = Some(value);
        facet.quality = Quality::Observed;
        true
    }

    pub fn accept_absolute_on_off(&self, facet_id: &str, target: bool) -> CommandReceipt {
        let Some(facet) = self.facets.get(facet_id) else {
            return CommandReceipt::Rejected { facet_id: facet_id.into(), reason: "unknown-facet" };
        };
        if !facet.capabilities.contains("on-off") {
            return CommandReceipt::Rejected { facet_id: facet_id.into(), reason: "operation-not-supported" };
        }
        CommandReceipt::AcceptedForDelivery {
            facet_id: facet_id.into(),
            target,
            confirmed: false,
        }
    }

    pub fn disconnect(&mut self) {
        self.generation += 1;
        for facet in self.facets.values_mut() {
            facet.quality = Quality::Unavailable;
        }
    }

    pub fn replace_facet(&mut self, locator: &str) {
        if let Some(stable_id) = self.identities.get(locator)
            && let Some(facet) = self.facets.get_mut(stable_id)
        {
            facet.incarnation += 1;
            facet.quality = Quality::Unknown;
            facet.value = None;
        }
    }
}

pub fn map_capabilities(source: &SourceEntity) -> Vec<Capability> {
    match source {
        SourceEntity::Switch { .. } => vec![Capability::OnOff],
        SourceEntity::BinarySensor { .. } => vec![Capability::BooleanState],
        SourceEntity::Light { brightness, rgb, .. } => {
            let mut result = vec![Capability::OnOff];
            if *brightness {
                result.push(Capability::Level);
            }
            if *rgb {
                result.push(Capability::Color);
            }
            result
        }
        SourceEntity::Sensor { device_class, unit, .. } => measurement_profile(device_class, unit)
            .map(Capability::Measurement)
            .into_iter()
            .collect(),
        SourceEntity::Diagnostic { .. } => vec![],
    }
}

pub fn measurement_profile(device_class: &str, unit: &str) -> Option<MeasurementProfile> {
    match (device_class, unit) {
        ("temperature", "°C" | "Cel") => Some(MeasurementProfile { quantity: "temperature", canonical_unit: "Cel" }),
        ("humidity", "%" | "%RH") => Some(MeasurementProfile { quantity: "relative-humidity", canonical_unit: "%" }),
        ("carbon_dioxide", "ppm") => Some(MeasurementProfile { quantity: "carbon-dioxide", canonical_unit: "[ppm]" }),
        ("pm1" | "pm10" | "pm25", "µg/m³" | "ug/m3") => Some(MeasurementProfile { quantity: "mass-concentration", canonical_unit: "ug/m3" }),
        _ => None,
    }
}

fn capability_name(capability: Capability) -> &'static str {
    match capability {
        Capability::OnOff => "on-off",
        Capability::BooleanState => "boolean-state",
        Capability::Level => "level",
        Capability::Color => "color",
        Capability::Measurement(profile) => match (profile.quantity, profile.canonical_unit) {
            ("temperature", "Cel") => "measurement:temperature:Cel",
            ("relative-humidity", "%") => "measurement:relative-humidity:%",
            ("carbon-dioxide", "[ppm]") => "measurement:carbon-dioxide:[ppm]",
            ("mass-concentration", "ug/m3") => "measurement:mass-concentration:ug/m3",
            _ => unreachable!("only built-in profiles reach capability names"),
        },
    }
}

fn canonical_unit(encoded: &str) -> &'static str {
    match encoded {
        "Cel" => "Cel",
        "%" => "%",
        "[ppm]" => "[ppm]",
        "ug/m3" => "ug/m3",
        _ => unreachable!("only built-in units are encoded"),
    }
}

pub mod fixtures {
    use super::SourceEntity;

    pub fn node_s() -> Vec<SourceEntity> {
        vec![
            SourceEntity::Switch { locator: "switch:11".into(), group: Some("channel-1".into()) },
            SourceEntity::Switch { locator: "switch:22".into(), group: Some("channel-2".into()) },
            SourceEntity::Switch { locator: "switch:33".into(), group: Some("channel-3".into()) },
            SourceEntity::BinarySensor { locator: "binary:41".into(), group: Some("channel-1".into()) },
            SourceEntity::BinarySensor { locator: "binary:42".into(), group: Some("channel-2".into()) },
            SourceEntity::BinarySensor { locator: "binary:43".into(), group: Some("channel-3".into()) },
            SourceEntity::Diagnostic { locator: "text:99".into(), kind: "text".into() },
        ]
    }

    pub fn node_a() -> Vec<SourceEntity> {
        vec![
            sensor("sensor:1", "temperature", "°C"),
            sensor("sensor:2", "humidity", "%"),
            sensor("sensor:3", "carbon_dioxide", "ppm"),
            sensor("sensor:4", "pm25", "µg/m³"),
            sensor("sensor:5", "volatile_organic_compounds", "ppb"),
        ]
    }

    pub fn feature_light() -> Vec<SourceEntity> {
        vec![SourceEntity::Light {
            locator: "light:1".into(),
            brightness: true,
            rgb: true,
            group: Some("fixture-light".into()),
        }]
    }

    fn sensor(locator: &str, device_class: &str, unit: &str) -> SourceEntity {
        SourceEntity::Sensor {
            locator: locator.into(),
            device_class: device_class.into(),
            unit: unit.into(),
            group: Some("air-quality".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_facets_share_one_instance_without_nested_device_identity() {
        let mut provider = ProviderInstance::new("node-s");
        provider.apply_topology(&fixtures::node_s());
        assert_eq!(provider.facets().count(), 6);
        assert_eq!(provider.facets().filter(|facet| facet.capabilities.contains("on-off")).count(), 3);
        assert!(provider.facets().all(|facet| facet.stable_id.starts_with("node-s-facet-")));
    }

    #[test]
    fn identity_survives_reconnect_but_quality_and_generation_change() {
        let mut provider = ProviderInstance::new("node-s");
        let topology = fixtures::node_s();
        provider.apply_topology(&topology);
        let before = provider.facets().map(|facet| facet.stable_id.clone()).collect::<Vec<_>>();
        provider.observe_boolean("switch:11", true);
        provider.disconnect();
        assert_eq!(provider.generation(), 2);
        assert!(provider.facets().all(|facet| facet.quality == Quality::Unavailable));
        provider.apply_topology(&topology);
        let after = provider.facets().map(|facet| facet.stable_id.clone()).collect::<Vec<_>>();
        assert_eq!(before, after);
    }

    #[test]
    fn metadata_groups_do_not_change_binding_identity() {
        let mut provider = ProviderInstance::new("node-s");
        let mut topology = fixtures::node_s();
        provider.apply_topology(&topology);
        let before = provider.facets().map(|facet| facet.stable_id.clone()).collect::<Vec<_>>();
        if let SourceEntity::Switch { group, .. } = &mut topology[0] {
            *group = Some("renamed-group".into());
        }
        provider.apply_topology(&topology);
        let after = provider.facets().map(|facet| facet.stable_id.clone()).collect::<Vec<_>>();
        assert_eq!(before, after);
    }

    #[test]
    fn light_features_are_never_fabricated() -> Result<(), &'static str> {
        let mut provider = ProviderInstance::new("fixture");
        provider.apply_topology(&fixtures::feature_light());
        let facet = provider.facets().next().ok_or("fixture light facet missing")?;
        assert_eq!(facet.capabilities, BTreeSet::from(["color".into(), "level".into(), "on-off".into()]));

        let topology = [SourceEntity::Light { locator: "light:2".into(), brightness: false, rgb: false, group: None }];
        provider.apply_topology(&topology);
        let on_off_only = provider.facets().next().ok_or("on-off-only light facet missing")?;
        assert_eq!(on_off_only.capabilities, BTreeSet::from(["on-off".into()]));
        Ok(())
    }

    #[test]
    fn measurements_map_only_exact_known_profiles() -> Result<(), &'static str> {
        let mut provider = ProviderInstance::new("node-a");
        provider.apply_topology(&fixtures::node_a());
        assert_eq!(provider.facets().count(), 4, "unknown VOC profile remains diagnostic-only");
        assert!(provider.observe_measurement("sensor:1", 21.25));
        let temperature = provider
            .facets()
            .find(|facet| facet.capabilities.contains("measurement:temperature:Cel"))
            .ok_or("temperature fixture facet missing")?;
        assert_eq!(temperature.value, Some(ObservedValue::Decimal { value: "21.25".into(), unit: "Cel" }));
        Ok(())
    }

    #[test]
    fn delivery_receipt_never_claims_confirmation() -> Result<(), &'static str> {
        let mut provider = ProviderInstance::new("node-s");
        provider.apply_topology(&fixtures::node_s());
        let id = provider
            .facets()
            .find(|facet| facet.capabilities.contains("on-off"))
            .ok_or("on-off fixture facet missing")?
            .stable_id
            .clone();
        assert_eq!(provider.accept_absolute_on_off(&id, true), CommandReceipt::AcceptedForDelivery { facet_id: id, target: true, confirmed: false });
        Ok(())
    }

    #[test]
    fn instances_are_isolated() {
        let mut switches = ProviderInstance::new("node-s");
        let mut air = ProviderInstance::new("node-a");
        switches.apply_topology(&fixtures::node_s());
        air.apply_topology(&fixtures::node_a());
        switches.disconnect();
        assert_eq!(switches.generation(), 2);
        assert_eq!(air.generation(), 1);
        assert!(air.facets().all(|facet| facet.quality == Quality::Unknown));
    }
}
