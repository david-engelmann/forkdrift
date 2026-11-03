//! The result type divergence pins, ported test for test
//! from the Python corpus file of the same name.

mod common;

use common::{artifact, compare, report_text};

const LEFT: &str = "interface shard-store\n\noperation fetch = record\noperation store = ack\n";
const RIGHT: &str =
    "interface shard-store\n\noperation fetch = record\noperation store = receipt\n";

#[test]
fn different_result_type_alarms_once() {
    let left = artifact(LEFT, "run-a");
    let right = artifact(RIGHT, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    let d = &found[0];
    assert_eq!(d.name, "store");
    assert_eq!(d.kind, "result-type");
}

#[test]
fn result_type_alarm_quotes_both_operation_lines() {
    let left = artifact(LEFT, "run-a");
    let right = artifact(RIGHT, "run-b");
    let report = report_text(&compare(&left, &right));
    assert!(report.contains("operation store = ack"), "{report}");
    assert!(report.contains("operation store = receipt"), "{report}");
    assert!(report.contains("run-a"), "{report}");
    assert!(report.contains("run-b"), "{report}");
}

#[test]
fn same_result_type_raises_no_alarm() {
    let left = artifact(LEFT, "run-a");
    let right = artifact(LEFT, "run-b");
    assert_eq!(compare(&left, &right), Vec::new());
}
