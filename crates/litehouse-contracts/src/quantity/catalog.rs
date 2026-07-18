use core::{fmt, str::FromStr};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use super::{
    decimal::Rational,
    error::{ErrorCode, QuantityError, Result},
    unit::Dimension,
};

/// Immutable identifier of this built-in catalog release.
pub const CATALOG_ID: &str = "litehouse:quantity-catalog@1";

/// Stable built-in profile identifiers.
pub mod profile_ids {
    pub const TEMPERATURE_ABSOLUTE: &str = "litehouse:quantity/temperature-absolute@1";
    pub const TEMPERATURE_DIFFERENCE: &str = "litehouse:quantity/temperature-difference@1";
    pub const RELATIVE_HUMIDITY: &str = "litehouse:quantity/relative-humidity@1";
    pub const RATIO: &str = "litehouse:quantity/ratio@1";
    pub const WMO_WEATHER_CODE: &str = "litehouse:quantity/wmo-weather-code@1";
    pub const PRESSURE: &str = "litehouse:quantity/pressure@1";
    pub const ILLUMINANCE: &str = "litehouse:quantity/illuminance@1";
    pub const POWER: &str = "litehouse:quantity/power@1";
    pub const ENERGY: &str = "litehouse:quantity/energy@1";
    pub const VOLTAGE: &str = "litehouse:quantity/voltage@1";
    pub const ELECTRIC_CURRENT: &str = "litehouse:quantity/electric-current@1";
    pub const FREQUENCY: &str = "litehouse:quantity/frequency@1";
    pub const BATTERY_RATIO: &str = "litehouse:quantity/battery-ratio@1";
    pub const SIGNAL_STRENGTH: &str = "litehouse:quantity/signal-strength@1";
    pub const CARBON_DIOXIDE: &str = "litehouse:quantity/carbon-dioxide@1";
    pub const CARBON_MONOXIDE: &str = "litehouse:quantity/carbon-monoxide@1";
    pub const VOLATILE_ORGANIC_COMPOUNDS: &str = "litehouse:quantity/volatile-organic-compounds@1";
    pub const PM1: &str = "litehouse:quantity/particulate-matter-pm1@1";
    pub const PM2_5: &str = "litehouse:quantity/particulate-matter-pm2-5@1";
    pub const PM10: &str = "litehouse:quantity/particulate-matter-pm10@1";
    pub const DISTANCE: &str = "litehouse:quantity/distance@1";
    pub const SPEED: &str = "litehouse:quantity/speed@1";
    pub const VOLUME: &str = "litehouse:quantity/volume@1";
    pub const VOLUMETRIC_FLOW: &str = "litehouse:quantity/volumetric-flow@1";
}

/// Public documentation for an immutable built-in semantic profile.
///
/// `canonical_unit` is the only unit emitted on the canonical wire. Equal
/// physical dimensions do not imply equal profile semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Profile {
    pub id: &'static str,
    pub name: &'static str,
    pub canonical_unit: &'static str,
    pub description: &'static str,
    #[serde(skip)]
    pub(crate) dimension: Dimension,
    #[serde(skip)]
    factor_numerator: i128,
    #[serde(skip)]
    factor_denominator: i128,
}

impl Profile {
    pub(crate) fn canonical_factor(&self) -> Result<Rational> {
        Rational::new(self.factor_numerator, self.factor_denominator)
    }

    pub(crate) fn is_absolute_temperature(&self) -> bool {
        self.id == profile_ids::TEMPERATURE_ABSOLUTE
    }

    pub(crate) fn is_temperature_difference(&self) -> bool {
        self.id == profile_ids::TEMPERATURE_DIFFERENCE
    }

    pub(crate) fn is_bounded_ratio(&self) -> bool {
        matches!(
            self.id,
            profile_ids::RELATIVE_HUMIDITY | profile_ids::RATIO | profile_ids::BATTERY_RATIO
        )
    }

    pub(crate) fn is_wmo_weather_code(&self) -> bool {
        self.id == profile_ids::WMO_WEATHER_CODE
    }

    pub(crate) fn is_logarithmic_source(&self) -> bool {
        self.id == profile_ids::SIGNAL_STRENGTH
    }
}

macro_rules! profile {
    ($id:expr, $name:expr, $unit:expr, $description:expr, $dimension:expr) => {
        Profile {
            id: $id,
            name: $name,
            canonical_unit: $unit,
            description: $description,
            dimension: $dimension,
            factor_numerator: 1,
            factor_denominator: 1,
        }
    };
    ($id:expr, $name:expr, $unit:expr, $description:expr, $dimension:expr, $num:expr, $den:expr) => {
        Profile {
            id: $id,
            name: $name,
            canonical_unit: $unit,
            description: $description,
            dimension: $dimension,
            factor_numerator: $num,
            factor_denominator: $den,
        }
    };
}

/// The complete immutable v1 catalog. Canonical units are documented here and
/// are presentation-independent UCUM codes.
pub static BUILTIN_PROFILES: &[Profile] = &[
    profile!(
        profile_ids::TEMPERATURE_ABSOLUTE,
        "absolute temperature",
        "K",
        "Thermodynamic absolute temperature; Celsius input applies its affine offset.",
        Dimension::new([0, 0, 0, 0, 1, 0, 0])
    ),
    profile!(
        profile_ids::TEMPERATURE_DIFFERENCE,
        "temperature difference",
        "K",
        "Temperature interval; Celsius input is multiplicative and never applies an offset.",
        Dimension::new([0, 0, 0, 0, 1, 0, 0])
    ),
    profile!(
        profile_ids::RELATIVE_HUMIDITY,
        "relative humidity",
        "1",
        "Relative-humidity ratio in the closed interval expected by the provider contract.",
        Dimension::DIMENSIONLESS
    ),
    profile!(
        profile_ids::RATIO,
        "generic ratio",
        "1",
        "Dimensionless ratio in the closed interval [0, 1] for levels and positions.",
        Dimension::DIMENSIONLESS
    ),
    profile!(
        profile_ids::WMO_WEATHER_CODE,
        "WMO weather interpretation code",
        "1",
        "Discrete WMO weather interpretation code in the closed integer interval [0, 99].",
        Dimension::DIMENSIONLESS
    ),
    profile!(
        profile_ids::PRESSURE,
        "pressure",
        "Pa",
        "Pressure in pascals.",
        Dimension::new([-1, 1, -2, 0, 0, 0, 0])
    ),
    profile!(
        profile_ids::ILLUMINANCE,
        "illuminance",
        "lx",
        "Illuminance in lux.",
        Dimension::new([-2, 0, 0, 0, 0, 1, 0])
    ),
    profile!(
        profile_ids::POWER,
        "power",
        "W",
        "Power in watts.",
        Dimension::new([2, 1, -3, 0, 0, 0, 0])
    ),
    profile!(
        profile_ids::ENERGY,
        "energy",
        "J",
        "Energy in joules.",
        Dimension::new([2, 1, -2, 0, 0, 0, 0])
    ),
    profile!(
        profile_ids::VOLTAGE,
        "voltage",
        "V",
        "Electric potential difference in volts.",
        Dimension::new([2, 1, -3, -1, 0, 0, 0])
    ),
    profile!(
        profile_ids::ELECTRIC_CURRENT,
        "electric current",
        "A",
        "Electric current in amperes.",
        Dimension::CURRENT
    ),
    profile!(
        profile_ids::FREQUENCY,
        "frequency",
        "Hz",
        "Frequency in hertz.",
        Dimension::new([0, 0, -1, 0, 0, 0, 0])
    ),
    profile!(
        profile_ids::BATTERY_RATIO,
        "battery ratio",
        "1",
        "Battery state-of-charge ratio; semantically distinct from other ratios.",
        Dimension::DIMENSIONLESS
    ),
    profile!(
        profile_ids::SIGNAL_STRENGTH,
        "signal strength",
        "[dBm]",
        "Signal strength in dBm, kept on a non-linear semantic axis.",
        Dimension::SIGNAL_STRENGTH
    ),
    profile!(
        profile_ids::CARBON_DIOXIDE,
        "carbon dioxide concentration",
        "[ppm]",
        "Carbon-dioxide amount fraction in parts per million.",
        Dimension::DIMENSIONLESS,
        1,
        1_000_000
    ),
    profile!(
        profile_ids::CARBON_MONOXIDE,
        "carbon monoxide concentration",
        "[ppm]",
        "Carbon-monoxide amount fraction in parts per million.",
        Dimension::DIMENSIONLESS,
        1,
        1_000_000
    ),
    profile!(
        profile_ids::VOLATILE_ORGANIC_COMPOUNDS,
        "volatile organic compound concentration",
        "[ppm]",
        "Aggregate VOC amount fraction in parts per million.",
        Dimension::DIMENSIONLESS,
        1,
        1_000_000
    ),
    profile!(
        profile_ids::PM1,
        "PM1 mass concentration",
        "ug/m3",
        "PM1 mass concentration in micrograms per cubic metre.",
        Dimension::new([-3, 1, 0, 0, 0, 0, 0]),
        1,
        1_000_000_000
    ),
    profile!(
        profile_ids::PM2_5,
        "PM2.5 mass concentration",
        "ug/m3",
        "PM2.5 mass concentration in micrograms per cubic metre.",
        Dimension::new([-3, 1, 0, 0, 0, 0, 0]),
        1,
        1_000_000_000
    ),
    profile!(
        profile_ids::PM10,
        "PM10 mass concentration",
        "ug/m3",
        "PM10 mass concentration in micrograms per cubic metre.",
        Dimension::new([-3, 1, 0, 0, 0, 0, 0]),
        1,
        1_000_000_000
    ),
    profile!(
        profile_ids::DISTANCE,
        "distance",
        "m",
        "Distance in metres.",
        Dimension::LENGTH
    ),
    profile!(
        profile_ids::SPEED,
        "speed",
        "m/s",
        "Speed in metres per second.",
        Dimension::new([1, 0, -1, 0, 0, 0, 0])
    ),
    profile!(
        profile_ids::VOLUME,
        "volume",
        "m3",
        "Volume in cubic metres.",
        Dimension::new([3, 0, 0, 0, 0, 0, 0])
    ),
    profile!(
        profile_ids::VOLUMETRIC_FLOW,
        "volumetric flow",
        "m3/s",
        "Volumetric flow in cubic metres per second.",
        Dimension::new([3, 0, -1, 0, 0, 0, 0])
    ),
];

/// A validated built-in profile ID. The string representation remains open to
/// additive future catalog releases without making the wire an enum.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileId(String);

impl ProfileId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if lookup(&value).is_none() {
            return Err(QuantityError::new(ErrorCode::UnknownProfile));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn profile(&self) -> &'static Profile {
        // Construction and deserialization both validate this invariant.
        lookup(&self.0).expect("validated profile ID")
    }
}

impl fmt::Debug for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ProfileId(\"{}\")", self.0)
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ProfileId {
    type Err = QuantityError;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value)
    }
}

impl Serialize for ProfileId {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ProfileId {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ProfileIdVisitor;

        impl de::Visitor<'_> for ProfileIdVisitor {
            type Value = ProfileId;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a built-in Litehouse quantity profile ID")
            }

            fn visit_str<E>(self, value: &str) -> core::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                ProfileId::new(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_str(ProfileIdVisitor)
    }
}

pub fn lookup(id: &str) -> Option<&'static Profile> {
    BUILTIN_PROFILES.iter().find(|profile| profile.id == id)
}

pub(crate) fn unique_for_dimension(dimension: Dimension) -> Result<&'static Profile> {
    let mut matches = BUILTIN_PROFILES
        .iter()
        .filter(|profile| profile.dimension == dimension);
    let Some(first) = matches.next() else {
        return Err(QuantityError::new(ErrorCode::UnknownResultProfile));
    };
    if matches.next().is_some() {
        return Err(QuantityError::new(ErrorCode::AmbiguousResultProfile));
    }
    Ok(first)
}
