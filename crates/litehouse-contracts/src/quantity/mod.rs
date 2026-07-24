//! Deterministic, bounded quantity normalization and arithmetic.
//!
//! Inputs use a deliberately small case-sensitive UCUM subset, but canonical
//! wire values always contain a validated built-in semantic profile, a decimal
//! string, and that profile's cataloged canonical unit. All arithmetic is
//! checked integer/rational arithmetic; there is no floating-point path or
//! implicit rounding.

mod catalog;
mod decimal;
mod error;
mod unit;

use core::{cmp::Ordering, str::FromStr};
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeStruct};

pub use catalog::{
    BUILTIN_PROFILES, CATALOG_ID, Profile, ProfileId, lookup as lookup_profile, profile_ids,
};
pub use decimal::{
    Decimal, MAX_DECIMAL_INPUT_BYTES, MAX_SCALE, MAX_SIGNIFICANT_DIGITS, QuantizationAdjustment,
    QuantizationAudit, RoundingMode,
};
pub use error::{ErrorCode, QuantityError, SourceSpan};
pub use unit::{MAX_UNIT_EXPONENT, MAX_UNIT_INPUT_BYTES, MAX_UNIT_NESTING, MAX_UNIT_TOKENS};

use catalog::unique_for_dimension;
use decimal::Rational;
use error::{ErrorCode as Code, Result};

/// A canonical quantity locked to one immutable built-in semantic profile.
///
/// Serde representation:
///
/// ```json
/// {"profile":"litehouse:quantity/power@1","magnitude":"1","unit":"W"}
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quantity {
    profile: ProfileId,
    magnitude: Decimal,
}

impl Quantity {
    /// Normalizes a decimal magnitude and bounded UCUM unit under the explicitly
    /// expected profile. This path is exact and never rounds.
    pub fn parse(profile: &str, magnitude: &str, unit: &str) -> Result<Self> {
        let magnitude = Decimal::from_str(magnitude)?;
        Self::normalize_decimal(profile, magnitude, unit, None).map(|value| value.quantity)
    }

    /// Normalizes and explicitly quantizes a conversion to `scale` using
    /// bankers' (round-half-even) rounding, returning mandatory audit metadata.
    pub fn parse_quantized(
        profile: &str,
        magnitude: &str,
        unit: &str,
        scale: u8,
    ) -> Result<QuantizedQuantity> {
        let magnitude = Decimal::from_str(magnitude)?;
        Self::normalize_decimal(profile, magnitude, unit, Some(scale))
    }

    /// Constructs a quantity already expressed in its profile's canonical unit
    /// while preserving profile invariants.
    fn from_canonical(profile: ProfileId, magnitude: Decimal) -> Result<Self> {
        let quantity = Self { profile, magnitude };
        if quantity.profile().is_bounded_ratio()
            && !quantity
                .magnitude
                .to_rational()?
                .is_in_closed_unit_interval()
        {
            return Err(QuantityError::new(Code::SemanticMismatch));
        }
        if quantity.profile().is_wmo_weather_code()
            && (quantity.magnitude.scale() != 0
                || !(0..=99).contains(&quantity.magnitude.coefficient()))
        {
            return Err(QuantityError::new(Code::SemanticMismatch));
        }
        Ok(quantity)
    }

    pub fn profile_id(&self) -> &ProfileId {
        &self.profile
    }

    pub fn profile(&self) -> &'static Profile {
        self.profile.profile()
    }

    pub const fn magnitude(&self) -> Decimal {
        self.magnitude
    }

    pub fn unit(&self) -> &'static str {
        self.profile().canonical_unit
    }

    /// Compares quantities only when their semantic profiles are identical.
    pub fn checked_cmp(&self, rhs: &Self) -> Result<Ordering> {
        self.require_same_profile(rhs)?;
        Ok(self.magnitude.cmp(&rhs.magnitude))
    }

    /// Adds values only within the same semantic profile. Adding two absolute
    /// temperatures is intentionally rejected as affine misuse.
    pub fn checked_add(&self, rhs: &Self) -> Result<Self> {
        self.require_same_profile(rhs)?;
        if self.profile().is_absolute_temperature() {
            return Err(QuantityError::new(Code::AffineMisuse));
        }
        if self.profile().is_logarithmic_source() {
            return Err(QuantityError::new(Code::SemanticMismatch));
        }
        Self::from_canonical(
            self.profile.clone(),
            self.magnitude.checked_add(rhs.magnitude)?,
        )
    }

    /// Subtracts values only within the same semantic profile. Absolute minus
    /// absolute temperature produces the distinct temperature-difference
    /// profile.
    pub fn checked_sub(&self, rhs: &Self) -> Result<Self> {
        self.require_same_profile(rhs)?;
        let result_profile = if self.profile().is_absolute_temperature() {
            ProfileId::new(profile_ids::TEMPERATURE_DIFFERENCE)?
        } else {
            self.profile.clone()
        };
        if self.profile().is_logarithmic_source() {
            return Err(QuantityError::new(Code::SemanticMismatch));
        }
        Self::from_canonical(result_profile, self.magnitude.checked_sub(rhs.magnitude)?)
    }

    /// Multiplies exactly and succeeds only when the result dimension maps to
    /// exactly one built-in profile.
    pub fn checked_mul(&self, rhs: &Self) -> Result<Self> {
        self.binary_product(rhs, false, None)
            .map(|value| value.quantity)
    }

    /// Explicitly quantized multiplication with audit metadata.
    pub fn checked_mul_quantized(&self, rhs: &Self, scale: u8) -> Result<QuantizedQuantity> {
        self.binary_product(rhs, false, Some(scale))
    }

    /// Divides exactly and succeeds only when the result dimension maps to
    /// exactly one built-in profile.
    pub fn checked_div(&self, rhs: &Self) -> Result<Self> {
        self.binary_product(rhs, true, None)
            .map(|value| value.quantity)
    }

    /// Explicitly quantized division with audit metadata.
    pub fn checked_div_quantized(&self, rhs: &Self, scale: u8) -> Result<QuantizedQuantity> {
        self.binary_product(rhs, true, Some(scale))
    }

    /// Raises this quantity to a bounded integer power. Power one is identity;
    /// every other result must resolve unambiguously to a built-in profile.
    pub fn checked_powi(&self, exponent: i8) -> Result<Self> {
        self.power(exponent, None).map(|value| value.quantity)
    }

    /// Explicitly quantized integer power with audit metadata.
    pub fn checked_powi_quantized(&self, exponent: i8, scale: u8) -> Result<QuantizedQuantity> {
        self.power(exponent, Some(scale))
    }

    fn normalize_decimal(
        profile_id: &str,
        magnitude: Decimal,
        input_unit: &str,
        quantization_scale: Option<u8>,
    ) -> Result<QuantizedQuantity> {
        let profile_id = ProfileId::new(profile_id)?;
        let profile = profile_id.profile();
        let parsed_unit = unit::parse(input_unit)?;
        if parsed_unit.dimension != profile.dimension {
            return Err(QuantityError::new(Code::DimensionMismatch));
        }

        let mut canonical = magnitude.to_rational()?.mul(parsed_unit.factor)?;
        if let Some(offset) = parsed_unit.offset {
            if profile.is_absolute_temperature() {
                canonical = canonical.add(offset)?;
            } else if !profile.is_temperature_difference() {
                return Err(QuantityError::new(Code::AffineMisuse));
            }
        }
        canonical = canonical.div(profile.canonical_factor()?)?;
        if profile.is_bounded_ratio() && !canonical.is_in_closed_unit_interval() {
            return Err(QuantityError::new(Code::SemanticMismatch));
        }
        finish(profile_id, canonical, quantization_scale)
    }

    fn binary_product(
        &self,
        rhs: &Self,
        division: bool,
        quantization_scale: Option<u8>,
    ) -> Result<QuantizedQuantity> {
        if self.profile().is_absolute_temperature() || rhs.profile().is_absolute_temperature() {
            return Err(QuantityError::new(Code::AffineMisuse));
        }
        if self.profile().is_logarithmic_source() || rhs.profile().is_logarithmic_source() {
            return Err(QuantityError::new(Code::SemanticMismatch));
        }
        let dimension = if division {
            self.profile().dimension.div(rhs.profile().dimension)?
        } else {
            self.profile().dimension.mul(rhs.profile().dimension)?
        };
        let result_profile = unique_for_dimension(dimension)?;

        let left = self
            .magnitude
            .to_rational()?
            .mul(self.profile().canonical_factor()?)?;
        let right = rhs
            .magnitude
            .to_rational()?
            .mul(rhs.profile().canonical_factor()?)?;
        let basis_value = if division {
            left.div(right)?
        } else {
            left.mul(right)?
        };
        let canonical = basis_value.div(result_profile.canonical_factor()?)?;
        finish(
            ProfileId::new(result_profile.id)?,
            canonical,
            quantization_scale,
        )
    }

    fn power(&self, exponent: i8, quantization_scale: Option<u8>) -> Result<QuantizedQuantity> {
        if exponent.unsigned_abs() > MAX_UNIT_EXPONENT as u8 {
            return Err(QuantityError::new(Code::ExponentOutOfRange));
        }
        if self.profile().is_logarithmic_source() {
            return Err(QuantityError::new(Code::SemanticMismatch));
        }
        if exponent == 1 {
            let audit = QuantizationAudit {
                requested_scale: quantization_scale.unwrap_or(self.magnitude.scale()),
                mode: RoundingMode::HalfEven,
                inexact: false,
                tie: false,
                adjustment: QuantizationAdjustment::Exact,
            };
            return if let Some(scale) = quantization_scale {
                let (magnitude, audit) = self.magnitude.quantize(scale)?;
                Ok(QuantizedQuantity {
                    quantity: Self::from_canonical(self.profile.clone(), magnitude)?,
                    audit,
                })
            } else {
                Ok(QuantizedQuantity {
                    quantity: self.clone(),
                    audit,
                })
            };
        }
        if self.profile().is_absolute_temperature() {
            return Err(QuantityError::new(Code::AffineMisuse));
        }
        let dimension = self.profile().dimension.powi(exponent)?;
        let result_profile = unique_for_dimension(dimension)?;
        let basis = self
            .magnitude
            .to_rational()?
            .mul(self.profile().canonical_factor()?)?
            .powi(exponent)?;
        let canonical = basis.div(result_profile.canonical_factor()?)?;
        finish(
            ProfileId::new(result_profile.id)?,
            canonical,
            quantization_scale,
        )
    }

    fn require_same_profile(&self, rhs: &Self) -> Result<()> {
        if self.profile != rhs.profile {
            return Err(QuantityError::new(Code::SemanticMismatch));
        }
        Ok(())
    }
}

fn finish(
    profile: ProfileId,
    value: Rational,
    quantization_scale: Option<u8>,
) -> Result<QuantizedQuantity> {
    let (magnitude, audit) = if let Some(scale) = quantization_scale {
        value.quantize(scale)?
    } else {
        let magnitude = value.to_decimal_exact()?;
        let audit = QuantizationAudit {
            requested_scale: magnitude.scale(),
            mode: RoundingMode::HalfEven,
            inexact: false,
            tie: false,
            adjustment: QuantizationAdjustment::Exact,
        };
        (magnitude, audit)
    };
    Ok(QuantizedQuantity {
        quantity: Quantity::from_canonical(profile, magnitude)?,
        audit,
    })
}

impl Serialize for Quantity {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Quantity", 3)?;
        state.serialize_field("profile", &self.profile)?;
        state.serialize_field("magnitude", &self.magnitude)?;
        state.serialize_field("unit", self.unit())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Quantity {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct WireQuantity {
            profile: ProfileId,
            magnitude: String,
            unit: String,
        }

        let wire = WireQuantity::deserialize(deserializer)?;
        let profile = wire.profile.profile();
        if wire.unit != profile.canonical_unit {
            return Err(serde::de::Error::custom(
                "quantity wire unit must match the profile's canonical unit",
            ));
        }
        let magnitude = Decimal::from_str(&wire.magnitude).map_err(serde::de::Error::custom)?;
        if magnitude.to_string() != wire.magnitude {
            return Err(serde::de::Error::custom(
                "quantity wire magnitude must be a canonical decimal",
            ));
        }
        Quantity::from_canonical(wire.profile, magnitude).map_err(serde::de::Error::custom)
    }
}

/// A canonical quantity paired with the mandatory explicit-rounding audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantizedQuantity {
    pub quantity: Quantity,
    pub audit: QuantizationAudit,
}
