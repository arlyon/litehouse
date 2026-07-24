use core::{fmt, str::FromStr};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

use super::error::{ErrorCode, QuantityError, Result};

/// Maximum accepted decimal input length in UTF-8 bytes.
pub const MAX_DECIMAL_INPUT_BYTES: usize = 96;
/// Maximum significant digits retained by the decimal coefficient.
pub const MAX_SIGNIFICANT_DIGITS: usize = 38;
/// Maximum number of fractional decimal places.
pub const MAX_SCALE: u8 = 18;

/// A canonical, finite base-10 decimal backed by a checked `i128` coefficient.
///
/// The value is `coefficient * 10^-scale`. Zero has scale zero and non-zero
/// values never retain fractional trailing zeroes. Serialization is always a
/// JSON string; JSON numbers and binary floating-point values are not accepted.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Decimal {
    coefficient: i128,
    scale: u8,
}

impl Decimal {
    pub const ZERO: Self = Self {
        coefficient: 0,
        scale: 0,
    };
    pub const ONE: Self = Self {
        coefficient: 1,
        scale: 0,
    };

    /// Constructs and canonicalizes a checked coefficient/scale pair.
    pub fn from_coefficient_scale(mut coefficient: i128, mut scale: u8) -> Result<Self> {
        if coefficient == 0 {
            return Ok(Self::ZERO);
        }
        while scale > 0 && coefficient % 10 == 0 {
            coefficient /= 10;
            scale -= 1;
        }
        if scale > MAX_SCALE {
            return Err(QuantityError::new(ErrorCode::ScaleOutOfRange));
        }
        if decimal_digits(coefficient) > MAX_SIGNIFICANT_DIGITS {
            return Err(QuantityError::new(ErrorCode::PrecisionOverflow));
        }
        Ok(Self { coefficient, scale })
    }

    pub const fn coefficient(self) -> i128 {
        self.coefficient
    }

    pub const fn scale(self) -> u8 {
        self.scale
    }

    pub const fn is_zero(self) -> bool {
        self.coefficient == 0
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self> {
        let scale = self.scale.max(rhs.scale);
        let left = checked_scale(self.coefficient, scale - self.scale)?;
        let right = checked_scale(rhs.coefficient, scale - rhs.scale)?;
        let coefficient = left
            .checked_add(right)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        Self::from_coefficient_scale(coefficient, scale)
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self> {
        let negated = rhs
            .coefficient
            .checked_neg()
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        self.checked_add(Self {
            coefficient: negated,
            scale: rhs.scale,
        })
    }

    pub fn checked_mul(self, rhs: Self) -> Result<Self> {
        let coefficient = self
            .coefficient
            .checked_mul(rhs.coefficient)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        let scale = self
            .scale
            .checked_add(rhs.scale)
            .ok_or_else(|| QuantityError::new(ErrorCode::ScaleOutOfRange))?;
        Self::from_coefficient_scale(coefficient, scale)
    }

    /// Divides exactly. A recurring result, or a terminating result beyond the
    /// supported scale, is rejected rather than rounded implicitly.
    pub fn checked_div(self, rhs: Self) -> Result<Self> {
        Rational::from_decimal(self)?
            .div(Rational::from_decimal(rhs)?)?
            .to_decimal_exact()
    }

    /// Divides and explicitly quantizes to `scale` using round-half-to-even.
    pub fn checked_div_quantized(self, rhs: Self, scale: u8) -> Result<(Self, QuantizationAudit)> {
        Rational::from_decimal(self)?
            .div(Rational::from_decimal(rhs)?)?
            .quantize(scale)
    }

    /// Explicitly quantizes this value to `scale` using round-half-to-even.
    pub fn quantize(self, scale: u8) -> Result<(Self, QuantizationAudit)> {
        Rational::from_decimal(self)?.quantize(scale)
    }

    pub(crate) fn to_rational(self) -> Result<Rational> {
        Rational::from_decimal(self)
    }
}

impl FromStr for Decimal {
    type Err = QuantityError;

    fn from_str(input: &str) -> Result<Self> {
        if input.len() > MAX_DECIMAL_INPUT_BYTES {
            return Err(QuantityError::at(
                ErrorCode::DecimalInputTooLong,
                MAX_DECIMAL_INPUT_BYTES,
                (MAX_DECIMAL_INPUT_BYTES + 1).min(input.len()),
            ));
        }
        if input.is_empty() {
            return Err(QuantityError::at(ErrorCode::InvalidDecimal, 0, 0));
        }

        let bytes = input.as_bytes();
        let mut cursor = 0usize;
        let negative = bytes[0] == b'-';
        if negative {
            cursor = 1;
            if cursor == bytes.len() {
                return Err(QuantityError::at(ErrorCode::InvalidDecimal, 0, 1));
            }
        }

        let integer_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == integer_start {
            return Err(QuantityError::at(
                ErrorCode::InvalidDecimal,
                cursor,
                (cursor + 1).min(bytes.len()),
            ));
        }
        let integer_end = cursor;

        let (fraction_start, fraction_end) = if cursor < bytes.len() && bytes[cursor] == b'.' {
            cursor += 1;
            let start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
                cursor += 1;
            }
            if cursor == start {
                return Err(QuantityError::at(
                    ErrorCode::InvalidDecimal,
                    cursor.saturating_sub(1),
                    cursor,
                ));
            }
            (start, cursor)
        } else {
            (cursor, cursor)
        };

        if cursor != bytes.len() {
            return Err(QuantityError::at(
                ErrorCode::InvalidDecimal,
                cursor,
                cursor + 1,
            ));
        }

        let mut fractional_end = fraction_end;
        while fractional_end > fraction_start && bytes[fractional_end - 1] == b'0' {
            fractional_end -= 1;
        }
        let scale = fractional_end - fraction_start;
        if scale > MAX_SCALE as usize {
            return Err(QuantityError::at(
                ErrorCode::ScaleOutOfRange,
                fraction_start + MAX_SCALE as usize,
                fractional_end,
            ));
        }

        let mut first_nonzero = integer_start;
        while first_nonzero < integer_end && bytes[first_nonzero] == b'0' {
            first_nonzero += 1;
        }
        if first_nonzero == integer_end {
            first_nonzero = fraction_start;
            while first_nonzero < fractional_end && bytes[first_nonzero] == b'0' {
                first_nonzero += 1;
            }
        }
        if first_nonzero == fractional_end && integer_start == integer_end {
            return Ok(Self::ZERO);
        }

        let integer_significant = if first_nonzero < integer_end {
            integer_end - first_nonzero
        } else {
            0
        };
        let fractional_significant = if first_nonzero < integer_end {
            scale
        } else {
            fractional_end.saturating_sub(first_nonzero)
        };
        let significant_digits = integer_significant + fractional_significant;
        if significant_digits == 0 {
            return Ok(Self::ZERO);
        }
        if significant_digits > MAX_SIGNIFICANT_DIGITS {
            let start = if first_nonzero < integer_end {
                first_nonzero
            } else {
                first_nonzero
            };
            return Err(QuantityError::at(
                ErrorCode::PrecisionOverflow,
                start,
                bytes.len(),
            ));
        }

        let mut coefficient = 0i128;
        for &digit in &bytes[integer_start..integer_end] {
            coefficient = coefficient
                .checked_mul(10)
                .and_then(|value| value.checked_add((digit - b'0') as i128))
                .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        }
        for &digit in &bytes[fraction_start..fractional_end] {
            coefficient = coefficient
                .checked_mul(10)
                .and_then(|value| value.checked_add((digit - b'0') as i128))
                .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        }
        if negative {
            coefficient = coefficient
                .checked_neg()
                .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        }
        Self::from_coefficient_scale(coefficient, scale as u8)
    }
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.coefficient == 0 {
            return f.write_str("0");
        }
        let negative = self.coefficient < 0;
        let digits = self.coefficient.unsigned_abs().to_string();
        if negative {
            f.write_str("-")?;
        }
        if self.scale == 0 {
            return f.write_str(&digits);
        }
        let scale = self.scale as usize;
        if digits.len() <= scale {
            f.write_str("0.")?;
            for _ in 0..(scale - digits.len()) {
                f.write_str("0")?;
            }
            f.write_str(&digits)
        } else {
            let split = digits.len() - scale;
            write!(f, "{}.{}", &digits[..split], &digits[split..])
        }
    }
}

impl fmt::Debug for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Decimal(\"{self}\")")
    }
}

impl Serialize for Decimal {
    fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DecimalVisitor;

        impl de::Visitor<'_> for DecimalVisitor {
            type Value = Decimal;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a finite decimal encoded as a JSON string")
            }

            fn visit_str<E>(self, value: &str) -> core::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(DecimalVisitor)
    }
}

/// The only rounding mode supported by the v1 quantity contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoundingMode {
    HalfEven,
}

/// How explicit quantization changed the unrounded coefficient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QuantizationAdjustment {
    Exact,
    TowardZero,
    AwayFromZero,
}

/// Deterministic audit data returned whenever a caller requests quantization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantizationAudit {
    pub requested_scale: u8,
    pub mode: RoundingMode,
    pub inexact: bool,
    pub tie: bool,
    pub adjustment: QuantizationAdjustment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rational {
    num: i128,
    den: i128,
}

impl Rational {
    pub(crate) const ZERO: Self = Self { num: 0, den: 1 };

    pub(crate) fn new(mut num: i128, mut den: i128) -> Result<Self> {
        if den == 0 {
            return Err(QuantityError::new(ErrorCode::DivisionByZero));
        }
        if num == 0 {
            return Ok(Self::ZERO);
        }
        if den < 0 {
            num = num
                .checked_neg()
                .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
            den = den
                .checked_neg()
                .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        }
        let divisor = gcd(num.unsigned_abs(), den as u128) as i128;
        Ok(Self {
            num: num / divisor,
            den: den / divisor,
        })
    }

    pub(crate) fn from_decimal(value: Decimal) -> Result<Self> {
        Self::new(value.coefficient, pow10(value.scale)?)
    }

    pub(crate) fn add(self, rhs: Self) -> Result<Self> {
        let shared = gcd(self.den as u128, rhs.den as u128) as i128;
        let left_factor = rhs.den / shared;
        let right_factor = self.den / shared;
        let left = self
            .num
            .checked_mul(left_factor)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        let right = rhs
            .num
            .checked_mul(right_factor)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        let num = left
            .checked_add(right)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        let den = self
            .den
            .checked_mul(left_factor)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        Self::new(num, den)
    }

    pub(crate) fn mul(self, rhs: Self) -> Result<Self> {
        let cross_left = gcd(self.num.unsigned_abs(), rhs.den as u128) as i128;
        let cross_right = gcd(rhs.num.unsigned_abs(), self.den as u128) as i128;
        let left_num = self.num / cross_left;
        let right_den = rhs.den / cross_left;
        let right_num = rhs.num / cross_right;
        let left_den = self.den / cross_right;
        let num = left_num
            .checked_mul(right_num)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        let den = left_den
            .checked_mul(right_den)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        Self::new(num, den)
    }

    pub(crate) fn div(self, rhs: Self) -> Result<Self> {
        if rhs.num == 0 {
            return Err(QuantityError::new(ErrorCode::DivisionByZero));
        }
        self.mul(Self::new(rhs.den, rhs.num)?)
    }

    pub(crate) fn is_in_closed_unit_interval(self) -> bool {
        self.num >= 0 && self.num <= self.den
    }

    pub(crate) fn powi(self, exponent: i8) -> Result<Self> {
        if exponent == 0 {
            return Self::new(1, 1);
        }
        let mut exponent = exponent;
        let mut base = self;
        if exponent < 0 {
            if base.num == 0 {
                return Err(QuantityError::new(ErrorCode::DivisionByZero));
            }
            base = Self::new(base.den, base.num)?;
            exponent = -exponent;
        }
        let mut result = Self::new(1, 1)?;
        let mut remaining = exponent as u8;
        while remaining > 0 {
            if remaining & 1 == 1 {
                result = result.mul(base)?;
            }
            remaining >>= 1;
            if remaining > 0 {
                base = base.mul(base)?;
            }
        }
        Ok(result)
    }

    pub(crate) fn to_decimal_exact(self) -> Result<Decimal> {
        if self.num == 0 {
            return Ok(Decimal::ZERO);
        }
        let mut denominator = self.den;
        let mut twos = 0u8;
        let mut fives = 0u8;
        while denominator % 2 == 0 {
            denominator /= 2;
            twos += 1;
        }
        while denominator % 5 == 0 {
            denominator /= 5;
            fives += 1;
        }
        if denominator != 1 {
            return Err(QuantityError::new(ErrorCode::InexactResult));
        }
        let scale = twos.max(fives);
        if scale > MAX_SCALE {
            return Err(QuantityError::new(ErrorCode::InexactResult));
        }
        let multiplier_two = checked_pow(2, scale - twos)?;
        let multiplier_five = checked_pow(5, scale - fives)?;
        let coefficient = self
            .num
            .checked_mul(multiplier_two)
            .and_then(|value| value.checked_mul(multiplier_five))
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        Decimal::from_coefficient_scale(coefficient, scale)
    }

    pub(crate) fn quantize(self, scale: u8) -> Result<(Decimal, QuantizationAudit)> {
        if scale > MAX_SCALE {
            return Err(QuantityError::new(ErrorCode::InvalidQuantizationScale));
        }
        let multiplier = pow10(scale)?;
        let common = gcd(multiplier as u128, self.den as u128) as i128;
        let scaled_multiplier = multiplier / common;
        let denominator = self.den / common;
        let scaled = self
            .num
            .checked_mul(scaled_multiplier)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        let mut quotient = scaled / denominator;
        let remainder = (scaled % denominator).unsigned_abs() as i128;

        let mut tie = false;
        let mut increment = false;
        if remainder != 0 {
            match remainder.cmp(&(denominator - remainder)) {
                core::cmp::Ordering::Greater => increment = true,
                core::cmp::Ordering::Equal => {
                    tie = true;
                    increment = quotient.unsigned_abs() % 2 == 1;
                }
                core::cmp::Ordering::Less => {}
            }
        }
        if increment {
            quotient = if scaled < 0 {
                quotient.checked_sub(1)
            } else {
                quotient.checked_add(1)
            }
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
        }
        let adjustment = if remainder == 0 {
            QuantizationAdjustment::Exact
        } else if increment {
            QuantizationAdjustment::AwayFromZero
        } else {
            QuantizationAdjustment::TowardZero
        };
        Ok((
            Decimal::from_coefficient_scale(quotient, scale)?,
            QuantizationAudit {
                requested_scale: scale,
                mode: RoundingMode::HalfEven,
                inexact: remainder != 0,
                tie,
                adjustment,
            },
        ))
    }
}

fn decimal_digits(value: i128) -> usize {
    if value == 0 {
        1
    } else {
        value.unsigned_abs().to_string().len()
    }
}

fn checked_scale(value: i128, places: u8) -> Result<i128> {
    value
        .checked_mul(pow10(places)?)
        .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))
}

fn pow10(scale: u8) -> Result<i128> {
    checked_pow(10, scale)
}

fn checked_pow(base: i128, exponent: u8) -> Result<i128> {
    let mut result = 1i128;
    for _ in 0..exponent {
        result = result
            .checked_mul(base)
            .ok_or_else(|| QuantityError::new(ErrorCode::ArithmeticOverflow))?;
    }
    Ok(result)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}
