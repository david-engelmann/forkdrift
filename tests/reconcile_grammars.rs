//! The two worth grammars of FD-57, pinned apart.
//!
//! The staging equality of the reconcile surface computes
//! under the carried grammar of XC-28, which removes every
//! underscore from a written number and reads the full
//! Unicode decimal digit set. The comparison of the
//! detector computes under FD-19, whose grammar admits no
//! underscore and the ASCII digits alone, so a value it
//! cannot read as a number compares as text. The same two
//! writings therefore stage clean and diverge, each side
//! computing under its own grammar, and neither grammar
//! gives way to the other in the one binary.

mod reconcile_common;

use reconcile_common::{run_cli, temp_dir, write};

fn state(author: &str, value: &str) -> String {
    format!("interface shard-store\nauthor {author}\n\ntimeout = {value}\n")
}

fn artifact(value: &str) -> String {
    format!("interface shard-store\n\ntimeout = {value}\n")
}

#[test]
fn staging_reads_an_underscored_value_by_its_worth() {
    let dir = temp_dir("grammar-stage-clean");
    let left = write(&dir, "left.state", &state("engineer-a", "1_0"));
    let right = write(&dir, "right.state", &state("engineer-b", "10"));
    let record = write(&dir, "record.txt", "");
    let result = run_cli(&["reconcile", "stage", &left, &right, &record], &dir);
    assert_eq!(result.code, 0);
    let stdout = result.stdout_text();
    assert!(stdout.contains("clean: timeout"), "{stdout}");
    assert!(!stdout.contains("conflicting: timeout"), "{stdout}");
}

#[test]
fn staging_worth_reading_still_conflicts_on_a_different_worth() {
    let dir = temp_dir("grammar-stage-conflict");
    let left = write(&dir, "left.state", &state("engineer-a", "1_0"));
    let right = write(&dir, "right.state", &state("engineer-b", "11"));
    let record = write(&dir, "record.txt", "");
    let result = run_cli(&["reconcile", "stage", &left, &right, &record], &dir);
    assert_eq!(result.code, 0);
    let stdout = result.stdout_text();
    assert!(stdout.contains("conflicting: timeout"), "{stdout}");
    assert!(!stdout.contains("clean: timeout"), "{stdout}");
}

#[test]
fn compare_reads_the_same_writings_as_text() {
    let dir = temp_dir("grammar-compare");
    let left = write(&dir, "left.tape", &artifact("1_0"));
    let right = write(&dir, "right.tape", &artifact("10"));
    let result = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 1);
    let stdout = result.stdout_text();
    assert!(
        stdout.contains("divergence shard-store timeout term-value"),
        "{stdout}"
    );
    assert!(stdout.contains("timeout = 1_0"), "{stdout}");
    assert!(stdout.contains("timeout = 10"), "{stdout}");
}
