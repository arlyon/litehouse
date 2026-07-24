use litehouse_contracts::quantity as core;

wit_bindgen::generate!({
    path: "../litehouse-contracts/packages/quantity/1.0.0/wit",
    world: "quantity-engine",
});

use exports::litehouse::quantity::engine::{
    ErrorCode, Guest, QuantityError, QuantizationAudit, QuantizedQuantity, SourceSpan,
};
use litehouse::quantity::types::Quantity;

struct QuantityEngine;

impl Guest for QuantityEngine {
    fn normalize(
        profile: String,
        magnitude: String,
        unit: String,
    ) -> Result<Quantity, QuantityError> {
        core::Quantity::parse(&profile, &magnitude, &unit)
            .map(to_wire)
            .map_err(to_error)
    }

    fn normalize_quantized(
        profile: String,
        magnitude: String,
        unit: String,
        scale: u8,
    ) -> Result<QuantizedQuantity, QuantityError> {
        core::Quantity::parse_quantized(&profile, &magnitude, &unit, scale)
            .map(|result| QuantizedQuantity {
                value: to_wire(result.quantity),
                audit: to_audit(result.audit),
            })
            .map_err(to_error)
    }

    fn add(left: Quantity, right: Quantity) -> Result<Quantity, QuantityError> {
        binary(left, right, core::Quantity::checked_add)
    }

    fn subtract(left: Quantity, right: Quantity) -> Result<Quantity, QuantityError> {
        binary(left, right, core::Quantity::checked_sub)
    }

    fn multiply(left: Quantity, right: Quantity) -> Result<Quantity, QuantityError> {
        binary(left, right, core::Quantity::checked_mul)
    }

    fn multiply_quantized(
        left: Quantity,
        right: Quantity,
        scale: u8,
    ) -> Result<QuantizedQuantity, QuantityError> {
        let left = from_wire(left)?;
        let right = from_wire(right)?;
        left.checked_mul_quantized(&right, scale)
            .map(to_quantized)
            .map_err(to_error)
    }

    fn divide(left: Quantity, right: Quantity) -> Result<Quantity, QuantityError> {
        binary(left, right, core::Quantity::checked_div)
    }

    fn divide_quantized(
        left: Quantity,
        right: Quantity,
        scale: u8,
    ) -> Result<QuantizedQuantity, QuantityError> {
        let left = from_wire(left)?;
        let right = from_wire(right)?;
        left.checked_div_quantized(&right, scale)
            .map(to_quantized)
            .map_err(to_error)
    }

    fn power(value: Quantity, exponent: i8) -> Result<Quantity, QuantityError> {
        from_wire(value)?
            .checked_powi(exponent)
            .map(to_wire)
            .map_err(to_error)
    }

    fn power_quantized(
        value: Quantity,
        exponent: i8,
        scale: u8,
    ) -> Result<QuantizedQuantity, QuantityError> {
        from_wire(value)?
            .checked_powi_quantized(exponent, scale)
            .map(to_quantized)
            .map_err(to_error)
    }
}

fn binary(
    left: Quantity,
    right: Quantity,
    operation: fn(&core::Quantity, &core::Quantity) -> Result<core::Quantity, core::QuantityError>,
) -> Result<Quantity, QuantityError> {
    operation(&from_wire(left)?, &from_wire(right)?)
        .map(to_wire)
        .map_err(to_error)
}

fn from_wire(value: Quantity) -> Result<core::Quantity, QuantityError> {
    let quantity =
        core::Quantity::parse(&value.profile, &value.magnitude, &value.unit).map_err(to_error)?;
    if quantity.magnitude().to_string() != value.magnitude || quantity.unit() != value.unit {
        return Err(QuantityError {
            code: ErrorCode::InvalidDecimal,
            span: None,
        });
    }
    Ok(quantity)
}

fn to_wire(value: core::Quantity) -> Quantity {
    Quantity {
        profile: value.profile_id().as_str().to_string(),
        magnitude: value.magnitude().to_string(),
        unit: value.unit().to_string(),
    }
}

fn to_quantized(result: core::QuantizedQuantity) -> QuantizedQuantity {
    QuantizedQuantity {
        value: to_wire(result.quantity),
        audit: to_audit(result.audit),
    }
}

fn to_audit(audit: core::QuantizationAudit) -> QuantizationAudit {
    QuantizationAudit {
        requested_scale: audit.requested_scale,
        inexact: audit.inexact,
        tie: audit.tie,
        adjustment: match audit.adjustment {
            core::QuantizationAdjustment::Exact => "exact",
            core::QuantizationAdjustment::TowardZero => "toward-zero",
            core::QuantizationAdjustment::AwayFromZero => "away-from-zero",
        }
        .to_string(),
    }
}

fn to_error(error: core::QuantityError) -> QuantityError {
    QuantityError {
        code: match error.code {
            core::ErrorCode::DecimalInputTooLong => ErrorCode::DecimalInputTooLong,
            core::ErrorCode::UnitInputTooLong => ErrorCode::UnitInputTooLong,
            core::ErrorCode::TooManyTokens => ErrorCode::TooManyTokens,
            core::ErrorCode::NestingTooDeep => ErrorCode::NestingTooDeep,
            core::ErrorCode::ExponentOutOfRange => ErrorCode::ExponentOutOfRange,
            core::ErrorCode::InvalidDecimal => ErrorCode::InvalidDecimal,
            core::ErrorCode::InvalidUnitSyntax => ErrorCode::InvalidUnitSyntax,
            core::ErrorCode::UnknownUnit => ErrorCode::UnknownUnit,
            core::ErrorCode::ScaleOutOfRange => ErrorCode::ScaleOutOfRange,
            core::ErrorCode::PrecisionOverflow => ErrorCode::PrecisionOverflow,
            core::ErrorCode::ArithmeticOverflow => ErrorCode::ArithmeticOverflow,
            core::ErrorCode::DivisionByZero => ErrorCode::DivisionByZero,
            core::ErrorCode::InexactResult => ErrorCode::InexactResult,
            core::ErrorCode::UnknownProfile => ErrorCode::UnknownProfile,
            core::ErrorCode::DimensionMismatch => ErrorCode::DimensionMismatch,
            core::ErrorCode::SemanticMismatch => ErrorCode::SemanticMismatch,
            core::ErrorCode::AffineMisuse => ErrorCode::AffineMisuse,
            core::ErrorCode::AmbiguousResultProfile => ErrorCode::AmbiguousResultProfile,
            core::ErrorCode::UnknownResultProfile => ErrorCode::UnknownResultProfile,
            core::ErrorCode::InvalidQuantizationScale => ErrorCode::InvalidQuantizationScale,
        },
        span: error.span.map(|span| SourceSpan {
            start: span.start,
            end: span.end,
        }),
    }
}

export!(QuantityEngine);
