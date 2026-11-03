//! The term value divergence pins, ported test for test
//! from the Python corpus file of the same name.

mod common;

use common::{artifact, compare, report_text};

const LEFT: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 30\n";
const RIGHT: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 45\n";

#[test]
fn incompatible_term_value_alarms_once() {
    let left = artifact(LEFT, "run-a");
    let right = artifact(RIGHT, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "term-value");
}

#[test]
fn alarm_quotes_both_definitions_verbatim_with_sources() {
    let left = artifact(LEFT, "run-a");
    let right = artifact(RIGHT, "run-b");
    let report = report_text(&compare(&left, &right));
    assert!(report.contains("timeout = 30"), "{report}");
    assert!(report.contains("timeout = 45"), "{report}");
    assert!(report.contains("run-a"), "{report}");
    assert!(report.contains("run-b"), "{report}");
}

#[test]
fn divergence_names_the_concept_and_both_runs() {
    let left = artifact(LEFT, "run-a");
    let right = artifact(RIGHT, "run-b");
    let found = compare(&left, &right);
    let d = &found[0];
    assert_eq!(d.interface, "shard-store");
    assert_eq!(d.name, "timeout");
    assert_eq!(d.left_source, b"run-a".to_vec());
    assert_eq!(d.right_source, b"run-b".to_vec());
    assert_eq!(d.left_line, "timeout = 30");
    assert_eq!(d.right_line, "timeout = 45");
}

#[test]
fn agreeing_terms_raise_no_alarm() {
    let left = artifact(LEFT, "run-a");
    let right = artifact(LEFT, "run-b");
    assert_eq!(compare(&left, &right), Vec::new());
}
