use core::fmt;
use serde::{Deserialize, Serialize};

/// A bounded half-open byte span in the rejected input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start: u16,
    pub end: u16,
}

impl SourceSpan {
    pub(crate) fn new(start: usize, end: usize) -> Self {
        // Quantity inputs are capped at 256 bytes, so this conversion is lossless.
        Self {
            start: start as u16,
            end: end as u16,
        }
    }
}

/// Stable machine-readable failures produced by the quantity engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ErrorCode {
    DecimalInputTooLong,
    UnitInputTooLong,
    TooManyTokens,
    NestingTooDeep,
    ExponentOutOfRange,
    InvalidDecimal,
    InvalidUnitSyntax,
    UnknownUnit,
    ScaleOutOfRange,
    PrecisionOverflow,
    ArithmeticOverflow,
    DivisionByZero,
    InexactResult,
    UnknownProfile,
    DimensionMismatch,
    SemanticMismatch,
    AffineMisuse,
    AmbiguousResultProfile,
    UnknownResultProfile,
    InvalidQuantizationScale,
}

impl ErrorCode {
    /// Stable wire spelling for this error code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DecimalInputTooLong => "decimal-input-too-long",
            Self::UnitInputTooLong => "unit-input-too-long",
            Self::TooManyTokens => "too-many-tokens",
            Self::NestingTooDeep => "nesting-too-deep",
            Self::ExponentOutOfRange => "exponent-out-of-range",
            Self::InvalidDecimal => "invalid-decimal",
            Self::InvalidUnitSyntax => "invalid-unit-syntax",
            Self::UnknownUnit => "unknown-unit",
            Self::ScaleOutOfRange => "scale-out-of-range",
            Self::PrecisionOverflow => "precision-overflow",
            Self::ArithmeticOverflow => "arithmetic-overflow",
            Self::DivisionByZero => "division-by-zero",
            Self::InexactResult => "inexact-result",
            Self::UnknownProfile => "unknown-profile",
            Self::DimensionMismatch => "dimension-mismatch",
            Self::SemanticMismatch => "semantic-mismatch",
            Self::AffineMisuse => "affine-misuse",
            Self::AmbiguousResultProfile => "ambiguous-result-profile",
            Self::UnknownResultProfile => "unknown-result-profile",
            Self::InvalidQuantizationScale => "invalid-quantization-scale",
        }
    }
}

/// A stable, bounded quantity diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuantityError {
    pub code: ErrorCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

impl QuantityError {
    pub(crate) const fn new(code: ErrorCode) -> Self {
        Self { code, span: None }
    }

    pub(crate) fn at(code: ErrorCode, start: usize, end: usize) -> Self {
        Self {
            code,
            span: Some(SourceSpan::new(start, end)),
        }
    }
}

impl fmt::Display for QuantityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.span {
            Some(span) => write!(
                f,
                "quantity error {} at bytes {}..{}",
                self.code.as_str(),
                span.start,
                span.end
            ),
            None => write!(f, "quantity error {}", self.code.as_str()),
        }
    }
}

impl std::error::Error for QuantityError {}

pub(crate) type Result<T> = core::result::Result<T, QuantityError>;
