//! The report order pins, ported test for test from the
//! Python corpus file of the same name.

mod common;

use common::{artifact, compare, report_text};
use forkdrift::format_report;

const LEFT: &str = "interface shard-store\n\ntimeout = 30\nbackoff = fixed\n";
const RIGHT: &str = "interface shard-store\n\ntimeout = 45\nbackoff = exponential\n";

#[test]
fn two_divergences_are_both_reported_in_sorted_order() {
    let left = artifact(LEFT, "run-a");
    let right = artifact(RIGHT, "run-b");
    let found = compare(&left, &right);
    let names: Vec<&str> = found.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["backoff", "timeout"]);
    let report = report_text(&found);
    let backoff_at = report.find("backoff").expect("backoff stands");
    let timeout_at = report.find("timeout").expect("timeout stands");
    assert!(backoff_at < timeout_at, "{report}");
}

#[test]
fn repeated_comparison_is_byte_identical() {
    let left = artifact(LEFT, "run-a");
    let right = artifact(RIGHT, "run-b");
    let first = format_report(&compare(&left, &right));
    let second = format_report(&compare(&left, &right));
    assert_eq!(first, second);
}
