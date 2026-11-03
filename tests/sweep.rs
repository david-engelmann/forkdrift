//! Pins for the sweep obligations FD-20 to FD-23, ported
//! test for test from the Python corpus file of the same
//! name. One assertion moves house in the port. The Python
//! pin for the no-other-argument usage error names the
//! Python parser's usage line, and that wording is the Rust
//! parser's presentation under FD-32, so the pin here holds
//! the functional outcome and the generated command line
//! corpus pins the streams.

mod common;

use common::{artifact, compare, report_text, root, run_cli, run_cli_at_root, temp_dir, write};
use forkdrift::divergence::CompareError;
use forkdrift::{ForkdriftError, format_sweep, sweep_artifacts};

const BASELINE: &str = "interface shard-store\n\ntimeout = 30\n";
const DIVERGENT_TIMEOUT: &str = "interface shard-store\n\ntimeout = 45\n";
const DIVERGENT_STORE: &str = "interface shard-store\n\ntimeout = 30\noperation store = ack\n";
const MALFORMED: &str = "timeout = 30\n";
const OTHER_INTERFACE: &str = "interface shard-cache\n\ntimeout = 30\n";

#[test]
fn sweep_returns_one_list_per_other_in_the_order_given() {
    let baseline = artifact(BASELINE, "base.tape");
    let others = vec![
        artifact(DIVERGENT_TIMEOUT, "one.tape"),
        artifact(BASELINE, "two.tape"),
        artifact(DIVERGENT_STORE, "three.tape"),
    ];
    let sweep = sweep_artifacts(&baseline, &others).expect("sweep runs");
    assert_eq!(sweep.len(), 3);
    let names0: Vec<&str> = sweep[0].iter().map(|d| d.name.as_str()).collect();
    let kinds0: Vec<&str> = sweep[0].iter().map(|d| d.kind.as_str()).collect();
    assert_eq!(names0, ["timeout"]);
    assert_eq!(kinds0, ["term-value"]);
    assert_eq!(sweep[1], Vec::new());
    let names2: Vec<&str> = sweep[2].iter().map(|d| d.name.as_str()).collect();
    let kinds2: Vec<&str> = sweep[2].iter().map(|d| d.kind.as_str()).collect();
    assert_eq!(names2, ["store"]);
    assert_eq!(kinds2, ["presence"]);
}

#[test]
fn sweep_lists_are_exactly_the_pairwise_comparisons() {
    let baseline = artifact(BASELINE, "base.tape");
    let others = vec![
        artifact(DIVERGENT_TIMEOUT, "one.tape"),
        artifact(DIVERGENT_STORE, "two.tape"),
    ];
    let sweep = sweep_artifacts(&baseline, &others).expect("sweep runs");
    for (other, divergences) in others.iter().zip(sweep.iter()) {
        assert_eq!(*divergences, compare(&baseline, other));
    }
}

#[test]
fn sweep_applies_the_interface_refusal_pair_by_pair() {
    let baseline = artifact(BASELINE, "base.tape");
    let others = vec![
        artifact(BASELINE, "one.tape"),
        artifact(OTHER_INTERFACE, "two.tape"),
    ];
    let error = sweep_artifacts(&baseline, &others).expect_err("sweep refuses");
    assert!(matches!(
        error,
        CompareError::Refusal(ForkdriftError::InterfaceMismatch(_))
    ));
    let text = error.to_string();
    assert!(text.contains("shard-store"), "{text}");
    assert!(text.contains("shard-cache"), "{text}");
}

#[test]
fn format_sweep_attributes_an_agreeing_pair_with_its_header_alone() {
    let baseline = artifact(BASELINE, "base.tape");
    let others = vec![
        artifact(DIVERGENT_TIMEOUT, "one.tape"),
        artifact(BASELINE, "two.tape"),
    ];
    let sweep = sweep_artifacts(&baseline, &others).expect("sweep runs");
    let document =
        String::from_utf8(format_sweep(&baseline, &others, &sweep)).expect("document is text");
    let expected = "pair left (base.tape) right (one.tape)\n".to_string()
        + &report_text(&compare(&baseline, &others[0]))
        + "pair left (base.tape) right (two.tape)\n";
    assert_eq!(document, expected);
}

#[test]
fn format_sweep_all_agreeing_is_exactly_the_header_lines() {
    let baseline = artifact(BASELINE, "base.tape");
    let others = vec![
        artifact(BASELINE, "one.tape"),
        artifact(BASELINE, "two.tape"),
    ];
    let sweep = sweep_artifacts(&baseline, &others).expect("sweep runs");
    let document =
        String::from_utf8(format_sweep(&baseline, &others, &sweep)).expect("document is text");
    assert_eq!(
        document,
        "pair left (base.tape) right (one.tape)\npair left (base.tape) right (two.tape)\n"
    );
}

#[test]
fn cli_sweep_all_agreeing_exits_0_with_only_the_header_lines() {
    let dir = temp_dir("sweep-agree");
    let baseline = write(&dir, "base.tape", BASELINE);
    let one = write(&dir, "one.tape", BASELINE);
    let two = write(&dir, "two.tape", BASELINE);
    let result = run_cli(&["sweep", &baseline, &one, &two], &dir);
    assert_eq!(result.code, 0);
    assert_eq!(
        result.stdout_text(),
        format!("pair left ({baseline}) right ({one})\npair left ({baseline}) right ({two})\n")
    );
    assert_eq!(result.stderr, Vec::<u8>::new());
}

#[test]
fn cli_sweep_with_a_divergence_exits_1_and_attributes_the_pair() {
    let dir = temp_dir("sweep-diverge");
    let baseline = write(&dir, "base.tape", BASELINE);
    let agreeing = write(&dir, "one.tape", BASELINE);
    let divergent = write(&dir, "two.tape", DIVERGENT_TIMEOUT);
    let result = run_cli(&["sweep", &baseline, &agreeing, &divergent], &dir);
    assert_eq!(result.code, 1);
    assert_eq!(
        result.stdout_text(),
        format!(
            "pair left ({baseline}) right ({agreeing})\npair left ({baseline}) right ({divergent})\ndivergence shard-store timeout term-value\n  left ({baseline}) timeout = 30\n  right ({divergent}) timeout = 45\n"
        )
    );
}

#[test]
fn cli_sweep_refusal_in_a_late_pair_prints_no_partial_document() {
    let dir = temp_dir("sweep-late-refusal");
    let baseline = write(&dir, "base.tape", BASELINE);
    let divergent = write(&dir, "one.tape", DIVERGENT_TIMEOUT);
    let malformed = write(&dir, "two.tape", MALFORMED);
    let result = run_cli(&["sweep", &baseline, &divergent, &malformed], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(result.stdout, Vec::<u8>::new());
    let stderr = result.stderr_text();
    assert!(stderr.starts_with("error:"), "{stderr}");
    assert_eq!(stderr.matches('\n').count(), 1, "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
}

#[test]
fn cli_sweep_missing_file_exits_2_with_no_document() {
    let dir = temp_dir("sweep-missing");
    let baseline = write(&dir, "base.tape", BASELINE);
    let missing = dir.join("nope.tape").to_string_lossy().into_owned();
    let result = run_cli(&["sweep", &baseline, &missing], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(result.stdout, Vec::<u8>::new());
    let stderr = result.stderr_text();
    assert!(stderr.starts_with("error:"), "{stderr}");
    assert!(stderr.contains("cannot read"), "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
}

#[test]
fn cli_sweep_interface_mismatch_in_a_later_pair_refuses_the_sweep() {
    let dir = temp_dir("sweep-mismatch");
    let baseline = write(&dir, "base.tape", BASELINE);
    let agreeing = write(&dir, "one.tape", BASELINE);
    let other = write(&dir, "two.tape", OTHER_INTERFACE);
    let result = run_cli(&["sweep", &baseline, &agreeing, &other], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(result.stdout, Vec::<u8>::new());
    let stderr = result.stderr_text();
    assert!(stderr.contains("different interfaces"), "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
}

#[test]
fn cli_sweep_reports_the_first_refusal_in_pair_order() {
    let dir = temp_dir("sweep-first-refusal");
    let baseline = write(&dir, "base.tape", BASELINE);
    let mismatched = write(&dir, "one.tape", OTHER_INTERFACE);
    let malformed = write(&dir, "two.tape", MALFORMED);
    let first = run_cli(&["sweep", &baseline, &mismatched, &malformed], &dir);
    assert_eq!(first.code, 2);
    assert!(first.stderr_text().contains("different interfaces"));
    let second = run_cli(&["sweep", &baseline, &malformed, &mismatched], &dir);
    assert_eq!(second.code, 2);
    let stderr = second.stderr_text();
    assert!(!stderr.contains("different interfaces"), "{stderr}");
    assert!(stderr.contains("malformed"), "{stderr}");
}

#[test]
fn cli_sweep_with_no_other_artifact_is_a_usage_error() {
    let dir = temp_dir("sweep-usage");
    let baseline = write(&dir, "base.tape", BASELINE);
    let result = run_cli(&["sweep", &baseline], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert!(!result.stderr.is_empty());
}

#[test]
fn committed_sweep_example_matches_regeneration() {
    let args = &[
        "sweep",
        "examples/run-a.tape",
        "examples/run-b.tape",
        "examples/presence-a.tape",
        "examples/presence-b.tape",
    ];
    let result = run_cli_at_root(args);
    assert_eq!(result.code, 1);
    let committed =
        std::fs::read(root().join("examples").join("sweep-divergences.txt")).expect("committed");
    assert_eq!(result.stdout, committed);
    let second = run_cli_at_root(args);
    assert_eq!(second.stdout, result.stdout);
}

#[test]
fn committed_sweep_example_attributes_all_three_pairs() {
    let text = std::fs::read_to_string(root().join("examples").join("sweep-divergences.txt"))
        .expect("committed reads");
    let headers: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("pair left"))
        .collect();
    assert_eq!(
        headers,
        [
            "pair left (examples/run-a.tape) right (examples/run-b.tape)",
            "pair left (examples/run-a.tape) right (examples/presence-a.tape)",
            "pair left (examples/run-a.tape) right (examples/presence-b.tape)",
        ]
    );
}
