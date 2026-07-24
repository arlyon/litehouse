use litehouse_contracts::quantity::{
    BUILTIN_PROFILES, CATALOG_ID, Decimal, ErrorCode, Quantity, QuantizationAdjustment, profile_ids,
};
use std::str::FromStr;

#[test]
fn canonicalizes_compound_power_expressions() {
    for unit in ["W", "J/s", "kg.m2/s3"] {
        let quantity = Quantity::parse(profile_ids::POWER, "1", unit).unwrap();
        assert_eq!(quantity.magnitude().to_string(), "1");
        assert_eq!(quantity.unit(), "W");
    }
}

#[test]
fn canonicalizes_percent_to_profile_ratio() {
    let humidity = Quantity::parse(profile_ids::RELATIVE_HUMIDITY, "50", "%").unwrap();
    assert_eq!(humidity.magnitude().to_string(), "0.5");
    assert_eq!(humidity.unit(), "1");
}

#[test]
fn wmo_weather_code_is_a_bounded_discrete_profile() {
    for value in ["0", "99"] {
        let code = Quantity::parse(profile_ids::WMO_WEATHER_CODE, value, "1").unwrap();
        assert_eq!(code.magnitude().to_string(), value);
    }
    for invalid in ["-1", "1.5", "100"] {
        assert_eq!(
            Quantity::parse(profile_ids::WMO_WEATHER_CODE, invalid, "1")
                .unwrap_err()
                .code,
            ErrorCode::SemanticMismatch
        );
    }
}

#[test]
fn keeps_absolute_temperature_and_difference_distinct() {
    let freezing = Quantity::parse(profile_ids::TEMPERATURE_ABSOLUTE, "0", "Cel").unwrap();
    let boiling = Quantity::parse(profile_ids::TEMPERATURE_ABSOLUTE, "100", "Cel").unwrap();
    assert_eq!(freezing.magnitude().to_string(), "273.15");
    assert_eq!(boiling.magnitude().to_string(), "373.15");

    let difference = boiling.checked_sub(&freezing).unwrap();
    assert_eq!(
        difference.profile_id().as_str(),
        profile_ids::TEMPERATURE_DIFFERENCE
    );
    assert_eq!(difference.magnitude().to_string(), "100");
    assert_eq!(
        freezing.checked_add(&boiling).unwrap_err().code,
        ErrorCode::AffineMisuse
    );

    let interval = Quantity::parse(profile_ids::TEMPERATURE_DIFFERENCE, "10", "Cel").unwrap();
    assert_eq!(interval.magnitude().to_string(), "10");
}

#[test]
fn semantic_profiles_do_not_collapse_on_equal_dimensions() {
    let humidity = Quantity::parse(profile_ids::RELATIVE_HUMIDITY, "0.5", "1").unwrap();
    let battery = Quantity::parse(profile_ids::BATTERY_RATIO, "0.5", "1").unwrap();
    assert_eq!(
        humidity.checked_add(&battery).unwrap_err().code,
        ErrorCode::SemanticMismatch
    );
}

#[test]
fn multiplication_and_division_require_known_unambiguous_profiles() {
    let voltage = Quantity::parse(profile_ids::VOLTAGE, "230", "V").unwrap();
    let current = Quantity::parse(profile_ids::ELECTRIC_CURRENT, "2", "A").unwrap();
    let power = voltage.checked_mul(&current).unwrap();
    assert_eq!(power.profile_id().as_str(), profile_ids::POWER);
    assert_eq!(power.magnitude().to_string(), "460");

    let recovered = power.checked_div(&voltage).unwrap();
    assert_eq!(
        recovered.profile_id().as_str(),
        profile_ids::ELECTRIC_CURRENT
    );
    assert_eq!(recovered.magnitude().to_string(), "2");
}

#[test]
fn half_even_quantization_is_explicit_and_audited() {
    let (even, even_audit) = Decimal::from_str("1.005").unwrap().quantize(2).unwrap();
    assert_eq!(even.to_string(), "1");
    assert!(even_audit.tie);
    assert_eq!(even_audit.adjustment, QuantizationAdjustment::TowardZero);

    let (odd, odd_audit) = Decimal::from_str("1.015").unwrap().quantize(2).unwrap();
    assert_eq!(odd.to_string(), "1.02");
    assert!(odd_audit.tie);
    assert_eq!(odd_audit.adjustment, QuantizationAdjustment::AwayFromZero);
}

#[test]
fn semantic_ratio_bounds_and_logarithmic_sources_are_enforced() {
    let ratio = Quantity::parse(profile_ids::RATIO, "50", "%").unwrap();
    assert_eq!(ratio.magnitude().to_string(), "0.5");
    assert_eq!(
        Quantity::parse(profile_ids::RATIO, "1.1", "1")
            .unwrap_err()
            .code,
        ErrorCode::SemanticMismatch
    );
    assert!(
        serde_json::from_str::<Quantity>(
            r#"{"profile":"litehouse:quantity/ratio@1","magnitude":"1.1","unit":"1"}"#,
        )
        .is_err()
    );
    let high = Quantity::parse(profile_ids::RATIO, "0.8", "1").unwrap();
    let low = Quantity::parse(profile_ids::RATIO, "0.4", "1").unwrap();
    assert_eq!(
        high.checked_add(&low).unwrap_err().code,
        ErrorCode::SemanticMismatch
    );
    assert_eq!(
        low.checked_sub(&high).unwrap_err().code,
        ErrorCode::SemanticMismatch
    );

    let signal = Quantity::parse(profile_ids::SIGNAL_STRENGTH, "-42", "[dBm]").unwrap();
    assert_eq!(signal.unit(), "[dBm]");
    assert_eq!(
        signal.checked_add(&signal).unwrap_err().code,
        ErrorCode::SemanticMismatch
    );
    assert_eq!(
        signal.checked_sub(&signal).unwrap_err().code,
        ErrorCode::SemanticMismatch
    );
    assert_eq!(
        signal.checked_mul(&ratio).unwrap_err().code,
        ErrorCode::SemanticMismatch
    );
    assert_eq!(
        signal.checked_div(&ratio).unwrap_err().code,
        ErrorCode::SemanticMismatch
    );
    assert_eq!(
        signal.checked_powi(1).unwrap_err().code,
        ErrorCode::SemanticMismatch
    );
}

#[test]
fn malformed_and_unbounded_inputs_fail_with_stable_codes() {
    assert_eq!(
        Decimal::from_str(&"1".repeat(97)).unwrap_err().code,
        ErrorCode::DecimalInputTooLong
    );
    assert_eq!(
        Quantity::parse(profile_ids::POWER, "1", "not-a-unit")
            .unwrap_err()
            .code,
        ErrorCode::UnknownUnit
    );
    assert_eq!(
        Quantity::parse(profile_ids::POWER, "1", "m")
            .unwrap_err()
            .code,
        ErrorCode::DimensionMismatch
    );
    assert_eq!(
        Quantity::parse(profile_ids::POWER, "NaN", "W")
            .unwrap_err()
            .code,
        ErrorCode::InvalidDecimal
    );
}

#[test]
fn checked_profile_manifest_matches_the_runtime_catalog() {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../packages/quantity/1.0.0/quantity/profiles.json"
    ))
    .unwrap();
    assert_eq!(manifest["catalog"], CATALOG_ID);
    let profiles = manifest["profiles"].as_array().unwrap();
    assert_eq!(profiles.len(), BUILTIN_PROFILES.len());
    for profile in BUILTIN_PROFILES {
        let source = profiles
            .iter()
            .find(|candidate| candidate["id"] == profile.id)
            .unwrap_or_else(|| panic!("missing source profile {}", profile.id));
        assert_eq!(source["canonicalUnit"], profile.canonical_unit);
    }
}

#[test]
fn serde_wire_form_is_canonical_and_revalidates() {
    let quantity = Quantity::parse(profile_ids::ENERGY, "1", "kWh").unwrap();
    let json = serde_json::to_string(&quantity).unwrap();
    assert_eq!(
        json,
        r#"{"profile":"litehouse:quantity/energy@1","magnitude":"3600000","unit":"J"}"#
    );
    assert_eq!(serde_json::from_str::<Quantity>(&json).unwrap(), quantity);

    let wrong_unit = json.replace("\"J\"", "\"kWh\"");
    assert!(serde_json::from_str::<Quantity>(&wrong_unit).is_err());
}
