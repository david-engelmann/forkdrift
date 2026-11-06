//! FD-30 pins for the print failure family. The oracle
//! holds its document in an 8192 byte buffer, so a dead
//! standard output under a smaller document answers at
//! status 120 from the exit flush, and under a document of
//! 8192 bytes or more it answers at status 1 from inside
//! the write. These pins run the binary against a pipe
//! whose reading end is already closed. The error stream
//! family stands beside them under the probe correction to
//! FD-30, because a line that cannot be written to the
//! error stream rides the same exit flush shape in the
//! oracle and answers 120 there too.

mod common;

use common::{
    run_cli, run_cli_on_closed_stderr, run_cli_on_closed_stdout, run_cli_on_closed_streams,
    temp_dir, write,
};

fn diverging_pair(dir: &std::path::Path, terms: usize) -> (String, String) {
    let mut left = String::from("interface shard-store\n\n");
    let mut right = String::from("interface shard-store\n\n");
    for index in 0..terms {
        left.push_str(&format!("term{index:05} = {index}\n"));
        right.push_str(&format!("term{index:05} = {}\n", index + 100000));
    }
    (write(dir, "a.tape", &left), write(dir, "b.tape", &right))
}

#[test]
fn small_report_to_a_dead_output_answers_120() {
    let dir = temp_dir("pipe-small");
    let (left, right) = diverging_pair(&dir, 10);
    let sized = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(sized.code, 1);
    assert!(
        sized.stdout.len() < 8192,
        "report is {} bytes",
        sized.stdout.len()
    );
    let result = run_cli_on_closed_stdout(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 120);
    assert_eq!(
        result.stderr,
        b"error: forkdrift could not print its report\n".to_vec()
    );
}

#[test]
fn large_report_to_a_dead_output_answers_1() {
    let dir = temp_dir("pipe-large");
    let (left, right) = diverging_pair(&dir, 90);
    let sized = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(sized.code, 1);
    assert!(
        sized.stdout.len() >= 8192,
        "report is {} bytes",
        sized.stdout.len()
    );
    let result = run_cli_on_closed_stdout(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 1);
    let stderr = result.stderr_text();
    assert!(
        stderr.starts_with("error: forkdrift could not print its report, "),
        "{stderr}"
    );
    assert!(stderr.contains("Broken pipe"), "{stderr}");
}

/// The pin the FD-30 port note carried into this round. The
/// large document's print failure answers 1 when its error
/// line lands, and when the error stream is dead as well the
/// line has nowhere to land and the answer is 120.
#[test]
fn large_report_with_both_streams_dead_answers_120() {
    let dir = temp_dir("output-large-both-dead");
    let (left, right) = diverging_pair(&dir, 90);
    let result = run_cli_on_closed_streams(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 120);
}

#[test]
fn help_to_a_dead_output_answers_120() {
    let dir = temp_dir("pipe-help");
    let result = run_cli_on_closed_stdout(&["--help"], &dir);
    assert_eq!(result.code, 120);
    assert_eq!(
        result.stderr,
        b"error: forkdrift could not print its report\n".to_vec()
    );
}

#[test]
fn refusal_to_a_dead_error_stream_answers_120() {
    let dir = temp_dir("pipe-refusal-dead-err");
    let bad = write(
        &dir,
        "bad.tape",
        "interface shard-store\n\nreplicas three\n",
    );
    let good = write(&dir, "good.tape", "interface shard-store\n\nreplicas = 3\n");
    let control = run_cli(&["compare", &bad, &good], &dir);
    assert_eq!(control.code, 2);
    assert_eq!(
        control.stderr,
        format!("error: {bad} is malformed, line 3 matches no statement form, replicas three\n")
            .into_bytes()
    );
    let result = run_cli_on_closed_stderr(&["compare", &bad, &good], &dir);
    assert_eq!(result.code, 120);
    assert_eq!(result.stdout, Vec::<u8>::new());
    let missing = dir.join("missing.tape").to_string_lossy().into_owned();
    let control = run_cli(&["compare", &missing, &good], &dir);
    assert_eq!(control.code, 2);
    assert_eq!(
        control.stderr,
        format!("error: cannot read {missing}, No such file or directory\n").into_bytes()
    );
    let result = run_cli_on_closed_stderr(&["compare", &missing, &good], &dir);
    assert_eq!(result.code, 120);
}

#[test]
fn usage_error_to_a_dead_error_stream_answers_120() {
    let dir = temp_dir("pipe-usage-dead-err");
    let good = write(&dir, "good.tape", "interface shard-store\n\nreplicas = 3\n");
    let control = run_cli(&["compare", &good], &dir);
    assert_eq!(control.code, 2);
    assert_eq!(
        control.stderr,
        b"error: the following required arguments were not provided:\n  <RIGHT>\n\nUsage: forkdrift compare <LEFT> <RIGHT>\n\nFor more information, try '--help'.\n"
            .to_vec()
    );
    let result = run_cli_on_closed_stderr(&["compare", &good], &dir);
    assert_eq!(result.code, 120);
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn small_report_with_both_streams_dead_answers_120() {
    let dir = temp_dir("pipe-report-both-dead");
    let (left, right) = diverging_pair(&dir, 10);
    let result = run_cli_on_closed_streams(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 120);
}

#[test]
fn refusal_with_both_streams_dead_answers_120() {
    let dir = temp_dir("pipe-refusal-both-dead");
    let bad = write(
        &dir,
        "bad.tape",
        "interface shard-store\n\nreplicas three\n",
    );
    let good = write(&dir, "good.tape", "interface shard-store\n\nreplicas = 3\n");
    let result = run_cli_on_closed_streams(&["compare", &bad, &good], &dir);
    assert_eq!(result.code, 120);
}

#[test]
fn help_with_both_streams_dead_answers_120() {
    let dir = temp_dir("pipe-help-both-dead");
    let result = run_cli_on_closed_streams(&["--help"], &dir);
    assert_eq!(result.code, 120);
}

#[test]
fn exponent_refusal_to_a_dead_output_still_answers_1() {
    let dir = temp_dir("pipe-exponent");
    let huge = format!("interface s\n\nt = 1e{}\n", "9".repeat(4301));
    let left = write(&dir, "a.tape", &huge);
    let right = write(&dir, "b.tape", "interface s\n\nt = 2\n");
    let result = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 1);
    assert_eq!(result.stdout, Vec::<u8>::new());
    let stderr = result.stderr_text();
    assert!(
        stderr.starts_with("error: forkdrift could not finish the comparison, "),
        "{stderr}"
    );
    assert!(stderr.contains("4301"), "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
}
