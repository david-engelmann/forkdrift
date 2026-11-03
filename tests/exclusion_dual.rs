//! FD-12 and FD-14 regression pins from the Round 1 review
//! fixes, ported test for test from the Python corpus file
//! of the same name.

mod common;

use common::{artifact, compare, report_text};
use forkdrift::divergence::EXCLUDED_BUT_DEFINED;

#[test]
fn excluded_name_defined_as_term_and_operation_is_one_divergence() {
    let left = artifact("interface shard-store\nexcludes = dual\n", "a.tape");
    let right = artifact(
        "interface shard-store\ndual = 1\noperation dual = ack\n",
        "b.tape",
    );
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    let only = &found[0];
    assert_eq!(only.kind, EXCLUDED_BUT_DEFINED);
    assert_eq!(only.name, "dual");
    let report = report_text(&found);
    assert!(report.contains("excludes = dual"), "{report}");
    assert!(report.contains("dual = 1"), "{report}");
    assert!(report.contains("operation dual = ack"), "{report}");
}

#[test]
fn excluded_name_defined_as_term_and_operation_reverse_direction() {
    let left = artifact(
        "interface shard-store\ndual = 1\noperation dual = ack\n",
        "a.tape",
    );
    let right = artifact("interface shard-store\nexcludes = dual\n", "b.tape");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    let report = report_text(&found);
    assert!(report.contains("dual = 1"), "{report}");
    assert!(report.contains("operation dual = ack"), "{report}");
}

#[test]
fn label_only_name_may_be_excluded_by_its_own_artifact() {
    let parsed = artifact(
        "interface shard-store\nlabel legacy = node-a\nexcludes = legacy\n",
        "a.tape",
    );
    assert!(parsed.labels.contains_key("legacy"));
}
