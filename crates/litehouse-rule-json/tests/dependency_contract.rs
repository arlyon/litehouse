use cel::parser::Parser;
use jiff::{
    civil::date,
    tz::{Disambiguation, TimeZone},
};
use serde_json::json;
use serde_json_canonicalizer::to_string as to_jcs;

#[test]
fn bounded_cel_parser_accepts_the_condition_leaf_shape() {
    let expression = Parser::new()
        .max_recursion_depth(32)
        .parse("enabled && count > 2")
        .expect("bounded CEL condition should parse");

    assert!(format!("{expression:?}").contains("enabled"));
    assert!(
        Parser::new()
            .max_recursion_depth(2)
            .parse("((((enabled))))")
            .is_err()
    );
}

#[test]
fn rfc_8785_reference_numbers_and_strings_are_stable() {
    let canonical = to_jcs(&json!({
        "numbers": [333_333_333.333_333_3, 1e30, 4.50, 2e-3, 1e-27],
        "string": "€$\u{000f}\nA'B\"\\\"/",
    }))
    .expect("reference value should canonicalize");

    assert_eq!(
        canonical,
        r#"{"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\"/"}"#
    );
}

#[test]
fn bundled_timezone_data_exposes_explicit_gap_and_fold_policy() {
    assert_eq!(jiff_tzdb::VERSION, Some("2026c"));
    let timezone = TimeZone::get("America/New_York").expect("bundled timezone should exist");

    let gap = date(2025, 3, 9).at(2, 30, 0, 0);
    assert!(
        timezone
            .to_ambiguous_zoned(gap)
            .disambiguate(Disambiguation::Reject)
            .is_err()
    );

    let fold = date(2025, 11, 2).at(1, 30, 0, 0);
    assert!(
        timezone
            .to_ambiguous_zoned(fold)
            .disambiguate(Disambiguation::Reject)
            .is_err()
    );
}
