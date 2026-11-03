//! The exclusion contradiction pins, ported test for test
//! from the Python corpus file of the same name.

mod common;

use common::{artifact, compare, report_text};

const LEFT_EXCLUDES: &str = "interface shard-store\n\nreplicas = 3\nexcludes = legacy-mode\n";
const RIGHT_DEFINES_TERM: &str = "interface shard-store\n\nreplicas = 3\nlegacy-mode = on\n";
const RIGHT_DEFINES_OPERATION: &str =
    "interface shard-store\n\nreplicas = 3\noperation legacy-mode = record\n";
const RIGHT_NEUTRAL: &str = "interface shard-store\n\nreplicas = 3\n";

#[test]
fn excluded_name_defined_as_term_alarms() {
    let left = artifact(LEFT_EXCLUDES, "run-a");
    let right = artifact(RIGHT_DEFINES_TERM, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    let d = &found[0];
    assert_eq!(d.name, "legacy-mode");
    assert_eq!(d.kind, "excluded-but-defined");
}

#[test]
fn exclusion_alarm_quotes_both_lines_verbatim() {
    let left = artifact(LEFT_EXCLUDES, "run-a");
    let right = artifact(RIGHT_DEFINES_TERM, "run-b");
    let report = report_text(&compare(&left, &right));
    assert!(report.contains("excludes = legacy-mode"), "{report}");
    assert!(report.contains("legacy-mode = on"), "{report}");
    assert!(report.contains("run-a"), "{report}");
    assert!(report.contains("run-b"), "{report}");
}

#[test]
fn excluded_name_defined_as_operation_alarms() {
    let left = artifact(LEFT_EXCLUDES, "run-a");
    let right = artifact(RIGHT_DEFINES_OPERATION, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, "excluded-but-defined");
}

#[test]
fn excluded_name_not_defined_raises_no_alarm() {
    let left = artifact(LEFT_EXCLUDES, "run-a");
    let right = artifact(RIGHT_NEUTRAL, "run-b");
    assert_eq!(compare(&left, &right), Vec::new());
}

#[test]
fn both_exclude_and_neither_defines_raises_no_alarm() {
    let left = artifact(LEFT_EXCLUDES, "run-a");
    let right = artifact(LEFT_EXCLUDES, "run-b");
    assert_eq!(compare(&left, &right), Vec::new());
}

#[test]
fn exclusion_in_the_other_direction_alarms() {
    let left = artifact(RIGHT_DEFINES_TERM, "run-a");
    let right = artifact(LEFT_EXCLUDES, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    let d = &found[0];
    assert_eq!(d.kind, "excluded-but-defined");
    assert_eq!(d.left_line, "legacy-mode = on");
    assert_eq!(d.right_line, "excludes = legacy-mode");
}
