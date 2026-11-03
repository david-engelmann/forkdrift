//! FD-30 pins for the print failure family. The oracle
//! holds its document in an 8192 byte buffer, so a dead
//! standard output under a smaller document answers at
//! status 120 from the exit flush, and under a document of
//! 8192 bytes or more it answers at status 1 from inside
//! the write. These pins run the binary against a pipe
//! whose reading end is already closed.

mod common;

use common::{run_cli, run_cli_on_closed_stdout, temp_dir, write};

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
