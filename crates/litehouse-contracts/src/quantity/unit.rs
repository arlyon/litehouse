use super::{
    decimal::Rational,
    error::{ErrorCode, QuantityError, Result},
};

/// Maximum accepted unit-expression length in UTF-8 bytes.
pub const MAX_UNIT_INPUT_BYTES: usize = 256;
/// Maximum lexical tokens in a unit expression.
pub const MAX_UNIT_TOKENS: u8 = 64;
/// Maximum parenthesis nesting depth.
pub const MAX_UNIT_NESTING: u8 = 8;
/// Maximum absolute unit exponent.
pub const MAX_UNIT_EXPONENT: i16 = 12;

// SI base order: length, mass, time, electric current, temperature,
// luminous intensity, and a deliberately non-linear signal-strength axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Dimension(pub(crate) [i16; 7]);

impl Dimension {
    pub(crate) const DIMENSIONLESS: Self = Self([0; 7]);
    pub(crate) const LENGTH: Self = Self([1, 0, 0, 0, 0, 0, 0]);
    pub(crate) const MASS: Self = Self([0, 1, 0, 0, 0, 0, 0]);
    pub(crate) const TIME: Self = Self([0, 0, 1, 0, 0, 0, 0]);
    pub(crate) const CURRENT: Self = Self([0, 0, 0, 1, 0, 0, 0]);
    pub(crate) const TEMPERATURE: Self = Self([0, 0, 0, 0, 1, 0, 0]);
    pub(crate) const SIGNAL_STRENGTH: Self = Self([0, 0, 0, 0, 0, 0, 1]);

    pub(crate) const fn new(parts: [i16; 7]) -> Self {
        Self(parts)
    }

    pub(crate) fn mul(self, rhs: Self) -> Result<Self> {
        let mut result = [0i16; 7];
        for (index, value) in result.iter_mut().enumerate() {
            *value = self.0[index]
                .checked_add(rhs.0[index])
                .ok_or_else(|| QuantityError::new(ErrorCode::ExponentOutOfRange))?;
        }
        Ok(Self(result))
    }

    pub(crate) fn div(self, rhs: Self) -> Result<Self> {
        let mut result = [0i16; 7];
        for (index, value) in result.iter_mut().enumerate() {
            *value = self.0[index]
                .checked_sub(rhs.0[index])
                .ok_or_else(|| QuantityError::new(ErrorCode::ExponentOutOfRange))?;
        }
        Ok(Self(result))
    }

    pub(crate) fn powi(self, exponent: i8) -> Result<Self> {
        let mut result = [0i16; 7];
        for (index, value) in result.iter_mut().enumerate() {
            *value = self.0[index]
                .checked_mul(exponent as i16)
                .ok_or_else(|| QuantityError::new(ErrorCode::ExponentOutOfRange))?;
        }
        Ok(Self(result))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParsedUnit {
    pub(crate) dimension: Dimension,
    /// Multiplicative conversion from this unit to the internal SI basis.
    pub(crate) factor: Rational,
    /// Offset in the SI basis. It is valid only for a lone affine atom.
    pub(crate) offset: Option<Rational>,
}

impl ParsedUnit {
    fn atom(dimension: Dimension, numerator: i128, denominator: i128) -> Result<Self> {
        Ok(Self {
            dimension,
            factor: Rational::new(numerator, denominator)?,
            offset: None,
        })
    }

    fn affine(
        dimension: Dimension,
        numerator: i128,
        denominator: i128,
        offset_numerator: i128,
        offset_denominator: i128,
    ) -> Result<Self> {
        Ok(Self {
            dimension,
            factor: Rational::new(numerator, denominator)?,
            offset: Some(Rational::new(offset_numerator, offset_denominator)?),
        })
    }

    fn multiply(self, rhs: Self) -> Result<Self> {
        if self.offset.is_some() || rhs.offset.is_some() {
            return Err(QuantityError::new(ErrorCode::AffineMisuse));
        }
        Ok(Self {
            dimension: self.dimension.mul(rhs.dimension)?,
            factor: self.factor.mul(rhs.factor)?,
            offset: None,
        })
    }

    fn divide(self, rhs: Self) -> Result<Self> {
        if self.offset.is_some() || rhs.offset.is_some() {
            return Err(QuantityError::new(ErrorCode::AffineMisuse));
        }
        Ok(Self {
            dimension: self.dimension.div(rhs.dimension)?,
            factor: self.factor.div(rhs.factor)?,
            offset: None,
        })
    }

    fn powi(self, exponent: i8) -> Result<Self> {
        if self.offset.is_some() && exponent != 1 {
            return Err(QuantityError::new(ErrorCode::AffineMisuse));
        }
        Ok(Self {
            dimension: self.dimension.powi(exponent)?,
            factor: self.factor.powi(exponent)?,
            offset: self.offset,
        })
    }
}

/// Parses Litehouse's bounded, case-sensitive UCUM input subset.
pub(crate) fn parse(input: &str) -> Result<ParsedUnit> {
    if input.len() > MAX_UNIT_INPUT_BYTES {
        return Err(QuantityError::at(
            ErrorCode::UnitInputTooLong,
            MAX_UNIT_INPUT_BYTES,
            (MAX_UNIT_INPUT_BYTES + 1).min(input.len()),
        ));
    }
    if input.is_empty() {
        return Err(QuantityError::at(ErrorCode::InvalidUnitSyntax, 0, 0));
    }
    let mut parser = Parser {
        input,
        cursor: 0,
        tokens: 0,
    };
    let parsed = parser.expression(0)?;
    if parser.cursor != input.len() {
        return Err(QuantityError::at(
            ErrorCode::InvalidUnitSyntax,
            parser.cursor,
            (parser.cursor + 1).min(input.len()),
        ));
    }
    Ok(parsed)
}

struct Parser<'a> {
    input: &'a str,
    cursor: usize,
    tokens: u8,
}

impl Parser<'_> {
    fn expression(&mut self, depth: u8) -> Result<ParsedUnit> {
        let mut left = self.factor(depth)?;
        loop {
            let Some(byte) = self.peek() else {
                break;
            };
            if byte == b')' {
                break;
            }
            if byte != b'.' && byte != b'/' {
                return Err(QuantityError::at(
                    ErrorCode::InvalidUnitSyntax,
                    self.cursor,
                    self.cursor + 1,
                ));
            }
            let operator_start = self.cursor;
            self.cursor += 1;
            self.token(operator_start, self.cursor)?;
            let right = self.factor(depth)?;
            left = if byte == b'.' {
                left.multiply(right)?
            } else {
                left.divide(right)?
            };
        }
        Ok(left)
    }

    fn factor(&mut self, depth: u8) -> Result<ParsedUnit> {
        let Some(byte) = self.peek() else {
            return Err(QuantityError::at(
                ErrorCode::InvalidUnitSyntax,
                self.cursor,
                self.cursor,
            ));
        };
        let mut value = if byte == b'(' {
            if depth >= MAX_UNIT_NESTING {
                return Err(QuantityError::at(
                    ErrorCode::NestingTooDeep,
                    self.cursor,
                    self.cursor + 1,
                ));
            }
            let open = self.cursor;
            self.cursor += 1;
            self.token(open, self.cursor)?;
            if self.peek() == Some(b')') {
                return Err(QuantityError::at(
                    ErrorCode::InvalidUnitSyntax,
                    self.cursor,
                    self.cursor + 1,
                ));
            }
            let inner = self.expression(depth + 1)?;
            if self.peek() != Some(b')') {
                return Err(QuantityError::at(
                    ErrorCode::InvalidUnitSyntax,
                    self.cursor,
                    self.cursor,
                ));
            }
            let close = self.cursor;
            self.cursor += 1;
            self.token(close, self.cursor)?;
            inner
        } else {
            self.atom()?
        };

        if let Some(exponent) = self.exponent()? {
            value = value.powi(exponent)?;
        }
        Ok(value)
    }

    fn atom(&mut self) -> Result<ParsedUnit> {
        let start = self.cursor;
        let symbol = match self.peek() {
            Some(b'%') => {
                self.cursor += 1;
                &self.input[start..self.cursor]
            }
            Some(b'1') => {
                self.cursor += 1;
                if self.peek().is_some_and(|next| next.is_ascii_digit()) {
                    return Err(QuantityError::at(
                        ErrorCode::InvalidUnitSyntax,
                        start,
                        self.cursor + 1,
                    ));
                }
                &self.input[start..self.cursor]
            }
            Some(b'[') => {
                self.cursor += 1;
                while self.peek().is_some_and(|next| next != b']') {
                    if !self
                        .peek()
                        .is_some_and(|next| next.is_ascii_alphanumeric() || next == b'_')
                    {
                        return Err(QuantityError::at(
                            ErrorCode::InvalidUnitSyntax,
                            self.cursor,
                            self.cursor + 1,
                        ));
                    }
                    self.cursor += 1;
                }
                if self.peek() != Some(b']') {
                    return Err(QuantityError::at(
                        ErrorCode::InvalidUnitSyntax,
                        start,
                        self.cursor,
                    ));
                }
                self.cursor += 1;
                &self.input[start..self.cursor]
            }
            Some(byte) if byte.is_ascii_alphabetic() => {
                self.cursor += 1;
                while self.peek().is_some_and(|next| next.is_ascii_alphabetic()) {
                    self.cursor += 1;
                }
                &self.input[start..self.cursor]
            }
            _ => {
                return Err(QuantityError::at(
                    ErrorCode::InvalidUnitSyntax,
                    start,
                    (start + 1).min(self.input.len()),
                ));
            }
        };
        self.token(start, self.cursor)?;
        unit_atom(symbol).map_err(|mut error| {
            error.span = Some(super::error::SourceSpan::new(start, self.cursor));
            error
        })
    }

    fn exponent(&mut self) -> Result<Option<i8>> {
        let start = self.cursor;
        let mut negative = false;
        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            negative = self.peek() == Some(b'-');
            self.cursor += 1;
            if !self.peek().is_some_and(|next| next.is_ascii_digit()) {
                self.cursor = start;
                return Ok(None);
            }
        } else if !self.peek().is_some_and(|next| next.is_ascii_digit()) {
            return Ok(None);
        }

        let digits_start = self.cursor;
        let mut magnitude = 0i16;
        while self.peek().is_some_and(|next| next.is_ascii_digit()) {
            magnitude = magnitude
                .saturating_mul(10)
                .saturating_add((self.input.as_bytes()[self.cursor] - b'0') as i16);
            self.cursor += 1;
        }
        if self.cursor == digits_start {
            self.cursor = start;
            return Ok(None);
        }
        self.token(start, self.cursor)?;
        if magnitude > MAX_UNIT_EXPONENT {
            return Err(QuantityError::at(
                ErrorCode::ExponentOutOfRange,
                start,
                self.cursor,
            ));
        }
        let exponent = if negative { -magnitude } else { magnitude };
        Ok(Some(exponent as i8))
    }

    fn token(&mut self, start: usize, end: usize) -> Result<()> {
        self.tokens = self.tokens.saturating_add(1);
        if self.tokens > MAX_UNIT_TOKENS {
            return Err(QuantityError::at(ErrorCode::TooManyTokens, start, end));
        }
        Ok(())
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.cursor).copied()
    }
}

fn unit_atom(symbol: &str) -> Result<ParsedUnit> {
    use Dimension as D;

    let value = match symbol {
        "1" => ParsedUnit::atom(D::DIMENSIONLESS, 1, 1),
        "%" => ParsedUnit::atom(D::DIMENSIONLESS, 1, 100),
        "[ppm]" => ParsedUnit::atom(D::DIMENSIONLESS, 1, 1_000_000),

        "m" => ParsedUnit::atom(D::LENGTH, 1, 1),
        "[ft_i]" => ParsedUnit::atom(D::LENGTH, 381, 1_250),
        "[in_i]" => ParsedUnit::atom(D::LENGTH, 127, 5_000),

        "s" => ParsedUnit::atom(D::TIME, 1, 1),
        "min" => ParsedUnit::atom(D::TIME, 60, 1),
        "h" => ParsedUnit::atom(D::TIME, 3_600, 1),

        "kg" => ParsedUnit::atom(D::MASS, 1, 1),
        "g" => ParsedUnit::atom(D::MASS, 1, 1_000),
        "mg" => ParsedUnit::atom(D::MASS, 1, 1_000_000),
        "ug" => ParsedUnit::atom(D::MASS, 1, 1_000_000_000),

        "K" => ParsedUnit::atom(D::TEMPERATURE, 1, 1),
        "Cel" => ParsedUnit::affine(D::TEMPERATURE, 1, 1, 27_315, 100),

        "Pa" => ParsedUnit::atom(D::new([-1, 1, -2, 0, 0, 0, 0]), 1, 1),
        "hPa" => ParsedUnit::atom(D::new([-1, 1, -2, 0, 0, 0, 0]), 100, 1),
        "kPa" => ParsedUnit::atom(D::new([-1, 1, -2, 0, 0, 0, 0]), 1_000, 1),
        "bar" => ParsedUnit::atom(D::new([-1, 1, -2, 0, 0, 0, 0]), 100_000, 1),

        "lx" => ParsedUnit::atom(D::new([-2, 0, 0, 0, 0, 1, 0]), 1, 1),
        "W" => ParsedUnit::atom(D::new([2, 1, -3, 0, 0, 0, 0]), 1, 1),
        "mW" => ParsedUnit::atom(D::new([2, 1, -3, 0, 0, 0, 0]), 1, 1_000),
        "kW" => ParsedUnit::atom(D::new([2, 1, -3, 0, 0, 0, 0]), 1_000, 1),
        "J" => ParsedUnit::atom(D::new([2, 1, -2, 0, 0, 0, 0]), 1, 1),
        "kJ" => ParsedUnit::atom(D::new([2, 1, -2, 0, 0, 0, 0]), 1_000, 1),
        "Wh" => ParsedUnit::atom(D::new([2, 1, -2, 0, 0, 0, 0]), 3_600, 1),
        "kWh" => ParsedUnit::atom(D::new([2, 1, -2, 0, 0, 0, 0]), 3_600_000, 1),

        "V" => ParsedUnit::atom(D::new([2, 1, -3, -1, 0, 0, 0]), 1, 1),
        "mV" => ParsedUnit::atom(D::new([2, 1, -3, -1, 0, 0, 0]), 1, 1_000),
        "kV" => ParsedUnit::atom(D::new([2, 1, -3, -1, 0, 0, 0]), 1_000, 1),
        "A" => ParsedUnit::atom(D::CURRENT, 1, 1),
        "mA" => ParsedUnit::atom(D::CURRENT, 1, 1_000),
        "Hz" => ParsedUnit::atom(D::new([0, 0, -1, 0, 0, 0, 0]), 1, 1),
        "kHz" => ParsedUnit::atom(D::new([0, 0, -1, 0, 0, 0, 0]), 1_000, 1),
        "MHz" => ParsedUnit::atom(D::new([0, 0, -1, 0, 0, 0, 0]), 1_000_000, 1),

        "L" => ParsedUnit::atom(D::new([3, 0, 0, 0, 0, 0, 0]), 1, 1_000),
        "mL" => ParsedUnit::atom(D::new([3, 0, 0, 0, 0, 0, 0]), 1, 1_000_000),
        "[dBm]" | "dBm" => ParsedUnit::atom(D::SIGNAL_STRENGTH, 1, 1),
        _ => Err(QuantityError::new(ErrorCode::UnknownUnit)),
    }?;
    Ok(value)
}
