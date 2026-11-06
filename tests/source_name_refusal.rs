//! The source name line-break refusal pins (FD-33 to FD-35).
//! A source name that carries a character of the FD-28 line
//! boundary set is refused by the comparison ahead of every
//! other check, at the pairwise and sweep surfaces, in the
//! library and on the command line, and a name with no
//! boundary character prints exactly as it printed before.

mod common;

use common::{artifact, compare, compare_err, run_cli, temp_dir, write};
use forkdrift::divergence::{CompareError, sweep_artifacts};
use forkdrift::pytext::carries_line_break;
use forkdrift::{ForkdriftError, parse_artifact};

const AGREEING: &str = "interface shard-store\n\ntimeout = 30\n";
const DIVERGENT: &str = "interface shard-store\n\ntimeout = 45\n";
const OTHER_INTERFACE: &str = "interface shard-cache\n\ntimeout = 30\n";
const EXCLUDES: &str = "interface shard-store\n\nexcludes = legacy-mode\n";
const DEFINES_EXCLUDED: &str =
    "interface shard-store\n\nlegacy-mode = on\noperation legacy-mode = record\n";

/// The ten characters of the FD-28 line boundary set.
const BOUNDARIES: [char; 10] = [
    '\n', '\r', '\u{b}', '\u{c}', '\u{1c}', '\u{1d}', '\u{1e}', '\u{85}', '\u{2028}', '\u{2029}',
];

fn named(text: &str, boundary: char, side: &str) -> forkdrift::Artifact {
    artifact(text, &format!("run{boundary}{side}"))
}

fn assert_name_refusal(error: &CompareError, side: &str) {
    assert!(
        matches!(error, CompareError::Refusal(ForkdriftError::SourceName(_))),
        "{error}"
    );
    assert_eq!(
        error.to_string(),
        format!("the {side} source name carries a line break")
    );
}

#[test]
fn comparison_refuses_every_boundary_character_in_the_left_name() {
    let right = artifact(DIVERGENT, "run-b");
    for boundary in BOUNDARIES {
        let left = named(AGREEING, boundary, "a");
        assert_name_refusal(&compare_err(&left, &right), "left");
    }
}

#[test]
fn comparison_refuses_every_boundary_character_in_the_right_name() {
    let left = artifact(AGREEING, "run-a");
    for boundary in BOUNDARIES {
        let right = named(DIVERGENT, boundary, "b");
        assert_name_refusal(&compare_err(&left, &right), "right");
    }
}

#[test]
fn the_left_name_is_read_before_the_right() {
    let left = named(AGREEING, '\n', "a");
    let right = named(DIVERGENT, '\r', "b");
    assert_name_refusal(&compare_err(&left, &right), "left");
}

#[test]
fn the_refusal_stands_ahead_of_the_interface_check() {
    let clean_left = artifact(AGREEING, "run-a");
    let boundary_right = named(OTHER_INTERFACE, '\n', "b");
    assert_name_refusal(&compare_err(&clean_left, &boundary_right), "right");
    let boundary_left = named(AGREEING, '\n', "a");
    let clean_right = artifact(OTHER_INTERFACE, "run-b");
    assert_name_refusal(&compare_err(&boundary_left, &clean_right), "left");
}

#[test]
fn the_refusal_stands_ahead_of_the_worth_computation() {
    let digits = "9".repeat(5000);
    let text = format!("interface shard-store\n\ntimeout = 1e{digits}\n");
    let left = artifact(&text, "run-a");
    let right = named(&text, '\n', "b");
    assert_name_refusal(&compare_err(&left, &right), "right");
}

#[test]
fn the_exclusion_rows_refuse_a_boundary_name() {
    let left = artifact(EXCLUDES, "run-a");
    for boundary in BOUNDARIES {
        let right = named(DEFINES_EXCLUDED, boundary, "b");
        assert_name_refusal(&compare_err(&left, &right), "right");
    }
}

#[test]
fn sweep_refuses_a_boundary_name_in_the_baseline() {
    let baseline = named(AGREEING, '\n', "a");
    let others = vec![artifact(DIVERGENT, "one.tape")];
    let error = sweep_artifacts(&baseline, &others).expect_err("sweep refuses");
    assert_name_refusal(&error, "left");
}

#[test]
fn sweep_refuses_a_boundary_name_in_a_later_pair() {
    let baseline = artifact(AGREEING, "base.tape");
    let others = vec![
        artifact(DIVERGENT, "one.tape"),
        named(DIVERGENT, '\u{2028}', "two.tape"),
    ];
    let error = sweep_artifacts(&baseline, &others).expect_err("sweep refuses");
    assert_name_refusal(&error, "right");
}

#[test]
fn sweep_refusal_order_follows_the_pair_order() {
    let baseline = artifact(AGREEING, "base.tape");
    let mismatch_first = vec![
        artifact(OTHER_INTERFACE, "one.tape"),
        named(DIVERGENT, '\n', "two.tape"),
    ];
    let error = sweep_artifacts(&baseline, &mismatch_first).expect_err("sweep refuses");
    assert!(
        matches!(
            error,
            CompareError::Refusal(ForkdriftError::InterfaceMismatch(_))
        ),
        "{error}"
    );
    let name_first = vec![
        named(DIVERGENT, '\n', "one.tape"),
        artifact(OTHER_INTERFACE, "two.tape"),
    ];
    let error = sweep_artifacts(&baseline, &name_first).expect_err("sweep refuses");
    assert_name_refusal(&error, "right");
}

#[test]
fn cli_refuses_a_newline_in_a_file_name() {
    let dir = temp_dir("name-refusal-newline");
    let left = write(&dir, "evil\nname.tape", AGREEING);
    let right = write(&dir, "b.tape", DIVERGENT);
    let result = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert_eq!(
        result.stderr_text(),
        "error: the left source name carries a line break\n"
    );
}

#[test]
fn cli_refuses_a_carriage_return_in_a_file_name() {
    let dir = temp_dir("name-refusal-return");
    let left = write(&dir, "a.tape", AGREEING);
    let right = write(&dir, "evil\rname.tape", DIVERGENT);
    let result = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert_eq!(
        result.stderr_text(),
        "error: the right source name carries a line break\n"
    );
}

#[test]
fn cli_sweep_with_a_boundary_name_prints_no_partial_document() {
    let dir = temp_dir("name-refusal-sweep");
    let baseline = write(&dir, "base.tape", AGREEING);
    let first = write(&dir, "one.tape", DIVERGENT);
    let forged = write(&dir, "evil\nname.tape", DIVERGENT);
    let result = run_cli(&["sweep", &baseline, &first, &forged], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert_eq!(
        result.stderr_text(),
        "error: the right source name carries a line break\n"
    );
}

#[test]
fn cli_sweep_refuses_at_the_first_pair_ahead_of_a_later_read() {
    let dir = temp_dir("name-refusal-sweep-first");
    let baseline = write(&dir, "evil\nbase.tape", AGREEING);
    let good = write(&dir, "one.tape", DIVERGENT);
    let malformed = write(&dir, "bad.tape", "timeout = 30\n");
    let result = run_cli(&["sweep", &baseline, &good, &malformed], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(result.stdout, Vec::<u8>::new());
    assert_eq!(
        result.stderr_text(),
        "error: the left source name carries a line break\n"
    );
}

#[test]
fn names_without_a_boundary_character_print_byte_for_byte() {
    let plain = compare(&artifact(AGREEING, "plain"), &artifact(DIVERGENT, "plain"));
    let plain_report = String::from_utf8(forkdrift::format_report(&plain)).expect("report text");
    for name in [
        "with space",
        "with\ttab",
        "with 'quote'",
        "with \"quotes\"",
        "...",
        "name (with parens)",
        "trailing space ",
    ] {
        let found = compare(&artifact(AGREEING, name), &artifact(DIVERGENT, name));
        let report = String::from_utf8(forkdrift::format_report(&found)).expect("report text");
        assert!(report.contains(name), "{name}");
        assert_eq!(report.replace(name, "plain"), plain_report, "{name}");
    }
}

#[test]
fn byte_forms_that_are_not_boundaries_do_not_refuse() {
    let left = parse_artifact(AGREEING, b"run-a").expect("left parses");
    for source in [
        &b"run\x85b"[..],
        &b"run\xc2"[..],
        &b"run\xe2\x80\xa7b"[..],
        &b"run\xffb"[..],
    ] {
        let right = parse_artifact(DIVERGENT, source).expect("right parses");
        assert!(!carries_line_break(source));
        let found = compare(&left, &right);
        assert_eq!(found.len(), 1);
        let report = forkdrift::format_report(&found);
        assert!(report.windows(source.len()).any(|w| w == source));
    }
}

#[test]
fn carries_line_break_reads_the_boundary_set_and_nothing_wider() {
    for boundary in BOUNDARIES {
        let name = format!("run{boundary}b");
        assert!(carries_line_break(name.as_bytes()), "{boundary:?}");
    }
    assert!(carries_line_break(b"run\xc2\x85b"));
    assert!(carries_line_break(b"run\xe2\x80\xa8b"));
    assert!(carries_line_break(b"run\xe2\x80\xa9b"));
    assert!(!carries_line_break(b""));
    assert!(!carries_line_break(b"plain-name.tape"));
    assert!(!carries_line_break(b"with\ttab"));
    assert!(!carries_line_break("with space".as_bytes()));
}
