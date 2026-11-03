//! The committed example pins, ported test for test from
//! the Python corpus file of the same name.

mod common;

use common::{artifact, compare, root, run_cli_at_root};

fn read_example(name: &str) -> String {
    std::fs::read_to_string(root().join("examples").join(name)).expect("example reads")
}

fn read_example_bytes(name: &str) -> Vec<u8> {
    std::fs::read(root().join("examples").join(name)).expect("example reads")
}

#[test]
fn example_pair_carries_exactly_two_divergences() {
    let left = artifact(&read_example("run-a.tape"), "examples/run-a.tape");
    let right = artifact(&read_example("run-b.tape"), "examples/run-b.tape");
    let found = compare(&left, &right);
    let names: Vec<&str> = found.iter().map(|d| d.name.as_str()).collect();
    let kinds: Vec<&str> = found.iter().map(|d| d.kind.as_str()).collect();
    assert_eq!(names, ["store", "timeout"]);
    assert_eq!(kinds, ["result-type", "term-value"]);
}

#[test]
fn committed_example_output_matches_regeneration() {
    let args = &["compare", "examples/run-a.tape", "examples/run-b.tape"];
    let result = run_cli_at_root(args);
    assert_eq!(result.code, 1);
    assert_eq!(result.stdout, read_example_bytes("divergences.txt"));
    let second = run_cli_at_root(args);
    assert_eq!(second.stdout, result.stdout);
}

#[test]
fn presence_example_pair_carries_a_mixed_report_in_order() {
    let left = artifact(&read_example("presence-a.tape"), "examples/presence-a.tape");
    let right = artifact(&read_example("presence-b.tape"), "examples/presence-b.tape");
    let found = compare(&left, &right);
    let names: Vec<&str> = found.iter().map(|d| d.name.as_str()).collect();
    let kinds: Vec<&str> = found.iter().map(|d| d.kind.as_str()).collect();
    assert_eq!(names, ["purge", "replicas"]);
    assert_eq!(kinds, ["presence", "term-value"]);
}

#[test]
fn committed_presence_example_output_matches_regeneration() {
    let args = &[
        "compare",
        "examples/presence-a.tape",
        "examples/presence-b.tape",
    ];
    let result = run_cli_at_root(args);
    assert_eq!(result.code, 1);
    assert_eq!(
        result.stdout,
        read_example_bytes("presence-divergences.txt")
    );
    let second = run_cli_at_root(args);
    assert_eq!(second.stdout, result.stdout);
}
