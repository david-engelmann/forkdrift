//! FD-15 and FD-17 pins, the presence kind and its report
//! order, ported test for test from the Python corpus file
//! of the same name.

mod common;

use common::{artifact, compare, report_text, run_cli, temp_dir, write};

const TERM_LEFT: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 30\n";
const TERM_RIGHT: &str = "interface shard-store\n\nreplicas = 3\n";
const OPERATION_RIGHT: &str = "interface shard-store\n\nreplicas = 3\noperation purge = ack\n";
const DUAL_LEFT: &str = "interface shard-store\n\ntimeout = 30\noperation timeout = ack\n";
const EMPTY: &str = "interface shard-store\n";
const CROSS_LEFT: &str = "interface shard-store\n\ncache = on\n";
const CROSS_RIGHT: &str = "interface shard-store\n\noperation cache = record\n";

#[test]
fn one_sided_term_raises_one_presence_divergence() {
    let left = artifact(TERM_LEFT, "run-a");
    let right = artifact(TERM_RIGHT, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    let only = &found[0];
    assert_eq!(only.name, "timeout");
    assert_eq!(only.kind, "presence");
    assert_eq!(only.left_source, b"run-a".to_vec());
    assert_eq!(only.left_line, "timeout = 30");
    assert_eq!(only.right_source, b"run-b".to_vec());
    assert_eq!(only.right_line, "interface shard-store");
    assert_eq!(
        report_text(&found),
        "divergence shard-store timeout presence\n  left (run-a) timeout = 30\n  right (run-b) interface shard-store\n"
    );
}

#[test]
fn one_sided_operation_raises_one_presence_divergence() {
    let left = artifact(TERM_RIGHT, "run-a");
    let right = artifact(OPERATION_RIGHT, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    let only = &found[0];
    assert_eq!(only.name, "purge");
    assert_eq!(only.kind, "presence");
    assert_eq!(only.left_line, "interface shard-store");
    assert_eq!(only.right_source, b"run-b".to_vec());
    assert_eq!(only.right_line, "operation purge = ack");
    assert_eq!(
        report_text(&found),
        "divergence shard-store purge presence\n  left (run-a) interface shard-store\n  right (run-b) operation purge = ack\n"
    );
}

#[test]
fn name_defined_both_ways_on_one_side_raises_one_presence_block() {
    let left = artifact(DUAL_LEFT, "run-a");
    let right = artifact(EMPTY, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "timeout");
    assert_eq!(found[0].kind, "presence");
    assert_eq!(
        report_text(&found),
        "divergence shard-store timeout presence\n  left (run-a) timeout = 30\n  right (run-b) interface shard-store\n  left (run-a) operation timeout = ack\n"
    );
}

#[test]
fn term_on_one_side_and_operation_on_the_other_raise_one_block() {
    let left = artifact(CROSS_LEFT, "run-a");
    let right = artifact(CROSS_RIGHT, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    let only = &found[0];
    assert_eq!(only.name, "cache");
    assert_eq!(only.kind, "presence");
    assert_eq!(only.left_line, "cache = on");
    assert_eq!(only.right_line, "operation cache = record");
    assert_eq!(
        report_text(&found),
        "divergence shard-store cache presence\n  left (run-a) cache = on\n  right (run-b) operation cache = record\n"
    );
}

#[test]
fn shared_term_with_different_values_raises_no_presence() {
    let left = artifact(TERM_LEFT, "run-a");
    let right = artifact(
        "interface shard-store\n\nreplicas = 3\ntimeout = 45\n",
        "run-b",
    );
    let found = compare(&left, &right);
    let kinds: Vec<&str> = found.iter().map(|d| d.kind.as_str()).collect();
    assert_eq!(kinds, ["term-value"]);
}

#[test]
fn shared_operation_with_different_types_raises_no_presence() {
    let left = artifact(OPERATION_RIGHT, "run-a");
    let right = artifact(
        "interface shard-store\n\nreplicas = 3\noperation purge = receipt\n",
        "run-b",
    );
    let found = compare(&left, &right);
    let kinds: Vec<&str> = found.iter().map(|d| d.kind.as_str()).collect();
    assert_eq!(kinds, ["result-type"]);
}

#[test]
fn shared_form_differing_only_by_normalization_raises_nothing() {
    let left = artifact(
        "interface shard-store\n\ntimeout = 030\nnote = a  b\n",
        "run-a",
    );
    let right = artifact(
        "interface shard-store\n\ntimeout = 30\nnote = a b\n",
        "run-b",
    );
    assert_eq!(compare(&left, &right), Vec::new());
}

#[test]
fn one_sided_operation_beside_a_shared_term_raises_presence_only() {
    let left = artifact(
        "interface shard-store\n\nreplicas = 3\ntimeout = 30\noperation purge = ack\n",
        "run-a",
    );
    let right = artifact(TERM_LEFT, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    let only = &found[0];
    assert_eq!(only.name, "purge");
    assert_eq!(only.kind, "presence");
    assert_eq!(only.left_line, "operation purge = ack");
    assert_eq!(only.right_line, "interface shard-store");
}

#[test]
fn diverging_term_and_one_sided_operation_raise_both_kinds() {
    let left = artifact(
        "interface shard-store\n\nmode = fast\noperation mode = ack\n",
        "run-a",
    );
    let right = artifact("interface shard-store\n\nmode = slow\n", "run-b");
    let found = compare(&left, &right);
    let names: Vec<&str> = found.iter().map(|d| d.name.as_str()).collect();
    let kinds: Vec<&str> = found.iter().map(|d| d.kind.as_str()).collect();
    assert_eq!(names, ["mode", "mode"]);
    assert_eq!(kinds, ["presence", "term-value"]);
    let presence = &found[0];
    assert_eq!(presence.left_line, "operation mode = ack");
    assert_eq!(presence.right_line, "interface shard-store");
}

#[test]
fn label_stated_on_one_side_only_raises_nothing() {
    let left = artifact(
        "interface shard-store\n\nreplicas = 3\nlabel primary = node-a\n",
        "run-a",
    );
    let right = artifact(TERM_RIGHT, "run-b");
    assert_eq!(compare(&left, &right), Vec::new());
}

#[test]
fn presence_and_value_divergences_sort_by_name_then_kind() {
    let left = artifact(
        "interface shard-store\n\nreplicas = 3\ntimeout = 30\noperation purge = ack\n",
        "run-a",
    );
    let right = artifact(
        "interface shard-store\n\nreplicas = 4\ntimeout = 30\n",
        "run-b",
    );
    let found = compare(&left, &right);
    assert_eq!(
        report_text(&found),
        "divergence shard-store purge presence\n  left (run-a) operation purge = ack\n  right (run-b) interface shard-store\ndivergence shard-store replicas term-value\n  left (run-a) replicas = 3\n  right (run-b) replicas = 4\n"
    );
}

#[test]
fn further_row_is_labelled_for_the_side_it_comes_from() {
    let left = artifact(DUAL_LEFT, "a.tape");
    let right = artifact(EMPTY, "b.tape");
    let report = report_text(&compare(&left, &right));
    let rows: Vec<&str> = report
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    assert!(
        rows.last()
            .expect("rows stand")
            .starts_with("left (a.tape) operation timeout = ack"),
        "{report}"
    );
}

#[test]
fn further_row_label_in_an_exclusion_block_from_the_left() {
    let left = artifact(
        "interface shard-store\n\ndual = 1\noperation dual = run\n",
        "a.tape",
    );
    let right = artifact("interface shard-store\n\nexcludes = dual\n", "b.tape");
    let report = report_text(&compare(&left, &right));
    let rows: Vec<&str> = report
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    assert!(rows[0].contains("excluded-but-defined"), "{report}");
    assert!(
        rows.last()
            .expect("rows stand")
            .starts_with("left (a.tape) operation dual = run"),
        "{report}"
    );
}

#[test]
fn presence_over_a_large_artifact_counts_every_one_sided_name() {
    // The name collection behind the presence pass is set
    // backed, so a wide artifact costs a pass and not a
    // square. This pin holds the count exact at a size
    // where the square would dominate the suite, and the
    // report order stays the FD-10 order at that size.
    let mut left_text = String::from("interface shard-store\n\nshared = 1\n");
    let mut right_text = String::from("interface shard-store\n\nshared = 1\n");
    for index in 0..4000 {
        left_text.push_str(&format!("left-term-{index:05} = {index}\n"));
        right_text.push_str(&format!("right-term-{index:05} = {index}\n"));
    }
    let left = artifact(&left_text, "run-a");
    let right = artifact(&right_text, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 8000);
    assert!(found.iter().all(|d| d.kind == "presence"));
    assert_eq!(found.first().expect("first stands").name, "left-term-00000");
    assert_eq!(found.last().expect("last stands").name, "right-term-03999");
}

#[test]
fn cli_pair_with_only_a_presence_divergence_exits_1() {
    let dir = temp_dir("presence-cli");
    let left = write(&dir, "a.tape", TERM_LEFT);
    let right = write(&dir, "b.tape", TERM_RIGHT);
    let result = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 1);
    assert!(
        result
            .stdout_text()
            .contains("divergence shard-store timeout presence")
    );
    assert_eq!(result.stderr, Vec::<u8>::new());
}
