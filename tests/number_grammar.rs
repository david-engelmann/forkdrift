//! FD-19 pins. The number grammar of this format, stated as
//! divergences, ported test for test from the Python corpus
//! file of the same name. The last three pins are the port's
//! own, for the conversion limit the probe correction to
//! FD-19 records.

mod common;

use common::{artifact, compare};
use forkdrift::Divergence;
use forkdrift::artifacts::values_equal;
use forkdrift::divergence::CompareError;

fn pair(left_value: &str, right_value: &str) -> Vec<Divergence> {
    let left = artifact(
        &format!("interface shard-store\n\nreplicas = {left_value}\n"),
        "run-a",
    );
    let right = artifact(
        &format!("interface shard-store\n\nreplicas = {right_value}\n"),
        "run-b",
    );
    compare(&left, &right)
}

#[test]
fn underscored_number_against_plain_number_diverges() {
    let found = pair("1_000", "1000");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
    assert_eq!(found[0].name, "replicas");
}

#[test]
fn short_underscored_number_against_plain_number_diverges() {
    let found = pair("1_0", "10");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
}

#[test]
fn underscored_number_against_itself_agrees() {
    assert_eq!(pair("1_000", "1_000"), Vec::new());
}

#[test]
fn exponent_form_agrees_with_plain_integer() {
    assert_eq!(pair("1e3", "1000"), Vec::new());
}

#[test]
fn signed_form_agrees_with_plain_integer() {
    assert_eq!(pair("+3", "3"), Vec::new());
}

#[test]
fn leading_zero_form_agrees_with_plain_integer() {
    assert_eq!(pair("03", "3"), Vec::new());
}

#[test]
fn leading_fraction_agrees_with_plain_fraction() {
    assert_eq!(pair(".5", "0.5"), Vec::new());
}

#[test]
fn trailing_point_agrees_with_plain_integer() {
    assert_eq!(pair("5.", "5"), Vec::new());
}

#[test]
fn negative_zero_agrees_with_zero() {
    assert_eq!(pair("-0", "0"), Vec::new());
}

#[test]
fn huge_exponent_against_itself_agrees() {
    assert_eq!(pair("1e999", "1e999"), Vec::new());
}

#[test]
fn huge_exponent_against_infinity_spelling_diverges() {
    let found = pair("1e999", "inf");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
}

#[test]
fn hex_form_against_decimal_diverges() {
    let found = pair("0x10", "16");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
}

#[test]
fn arbitrary_length_integers_compare_exactly() {
    let found = pair("10000000000000000000000001", "10000000000000000000000002");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
}

#[test]
fn arbitrary_length_integer_agrees_across_forms() {
    assert_eq!(
        pair(
            "10000000000000000000000001",
            "1.0000000000000000000000001e25"
        ),
        Vec::new()
    );
}

#[test]
fn exact_decimal_fractions_compare_exactly() {
    let found = pair("0.30000000000000004", "0.3");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
}

#[test]
fn underscored_text_against_itself_agrees() {
    assert_eq!(pair("node_1", "node_1"), Vec::new());
}

#[test]
fn underscored_text_against_other_text_diverges() {
    let found = pair("node_1", "node_2");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
}

#[test]
fn an_extreme_exponent_compares_by_worth_not_by_crash() {
    let found = pair("1e9999999999999999999", "2");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
}

#[test]
fn two_spellings_of_one_extreme_number_agree() {
    assert_eq!(
        pair("1e9999999999999999999", "+1e9999999999999999999"),
        Vec::new()
    );
    assert_eq!(
        pair("25e9999999999999999998", "2.5e9999999999999999999"),
        Vec::new()
    );
}

#[test]
fn an_extreme_exponent_against_its_own_text_agrees() {
    assert_eq!(
        pair("1e9999999999999999999", "1e9999999999999999999"),
        Vec::new()
    );
}

#[test]
fn a_digit_from_another_script_is_text_not_a_number() {
    let found = pair("\u{663}", "3");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
    assert_eq!(pair("\u{663}", "\u{663}"), Vec::new());
}

#[test]
fn trailing_point_with_exponent_agrees_with_plain_integer() {
    assert_eq!(pair("5.e3", "5000"), Vec::new());
}

#[test]
fn standing_fraction_with_exponent_agrees_with_plain_integer() {
    assert_eq!(pair(".5e2", "50"), Vec::new());
}

#[test]
fn capital_exponent_marker_agrees_with_plain_integer() {
    assert_eq!(pair("1E3", "1000"), Vec::new());
}

#[test]
fn exponent_with_leading_zeros_agrees_with_plain_integer() {
    assert_eq!(pair("1e007", "10000000"), Vec::new());
}

#[test]
fn exponent_at_the_conversion_limit_still_compares() {
    let huge = format!("1e{}", "9".repeat(4300));
    assert_eq!(values_equal(&huge, "2"), Ok(false));
    assert_eq!(values_equal(&huge, &huge), Ok(true));
    let shifted = format!("2e{}", "9".repeat(4300));
    assert_eq!(values_equal(&huge, &shifted), Ok(false));
}

#[test]
fn exponent_past_the_conversion_limit_refuses_the_comparison() {
    let huge = format!("1e{}", "9".repeat(4301));
    let error = values_equal(&huge, "2").expect_err("the conversion refuses");
    assert!(error.to_string().contains("4301"), "{error}");
    let left = artifact(
        &format!("interface shard-store\n\nreplicas = {huge}\n"),
        "run-a",
    );
    let right = artifact("interface shard-store\n\nreplicas = 2\n", "run-b");
    let failure = common::compare_err(&left, &right);
    assert!(matches!(failure, CompareError::Worth(_)));
}

#[test]
fn identical_text_past_the_limit_agrees_without_converting() {
    let huge = format!("1e{}", "9".repeat(4301));
    assert_eq!(values_equal(&huge, &huge), Ok(true));
    let zeros = format!("1e{}", "0".repeat(5000));
    assert!(values_equal(&zeros, "2").is_err());
    let zero_mantissa = format!("0e{}", "9".repeat(5000));
    assert!(values_equal(&zero_mantissa, "0").is_err());
}
