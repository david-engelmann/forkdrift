//! The command line pins, ported test for test from the
//! Python corpus file of the same name. The pins after the
//! ported five are the port's own, for the source byte forms
//! and the folded read refusal name the probe corrections
//! to FD-2 and FD-10 record.

mod common;

use common::{run_cli, run_cli_bytes, temp_dir, write};

const AGREEING: &str = "interface shard-store\n\ntimeout = 30\n";
const DIVERGENT: &str = "interface shard-store\n\ntimeout = 45\n";
const MALFORMED: &str = "timeout = 30\n";
const OTHER_INTERFACE: &str = "interface shard-cache\n\ntimeout = 30\n";

#[test]
fn cli_agreeing_pair_exits_0_with_empty_output() {
    let dir = temp_dir("cli-agree");
    let left = write(&dir, "a.tape", AGREEING);
    let right = write(&dir, "b.tape", AGREEING);
    let result = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 0);
    assert_eq!(result.stdout, Vec::<u8>::new());
}

#[test]
fn cli_divergent_pair_exits_1_and_quotes_both_definitions() {
    let dir = temp_dir("cli-diverge");
    let left = write(&dir, "a.tape", AGREEING);
    let right = write(&dir, "b.tape", DIVERGENT);
    let result = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 1);
    let stdout = result.stdout_text();
    assert!(stdout.contains("timeout = 30"), "{stdout}");
    assert!(stdout.contains("timeout = 45"), "{stdout}");
}

#[test]
fn cli_malformed_input_exits_2_without_traceback() {
    let dir = temp_dir("cli-malformed");
    let left = write(&dir, "a.tape", MALFORMED);
    let right = write(&dir, "b.tape", AGREEING);
    let result = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 2);
    let stderr = result.stderr_text();
    assert!(stderr.starts_with("error:"), "{stderr}");
    assert!(stderr.contains("interface"), "{stderr}");
    assert!(!stderr.contains("Traceback"), "{stderr}");
}

#[test]
fn cli_missing_file_exits_2_without_traceback() {
    let dir = temp_dir("cli-missing");
    let missing = dir.join("nope.tape").to_string_lossy().into_owned();
    let right = write(&dir, "b.tape", AGREEING);
    let result = run_cli(&["compare", &missing, &right], &dir);
    assert_eq!(result.code, 2);
    assert!(!result.stderr_text().contains("Traceback"));
}

#[test]
fn cli_different_interfaces_exit_2() {
    let dir = temp_dir("cli-interfaces");
    let left = write(&dir, "a.tape", AGREEING);
    let right = write(&dir, "b.tape", OTHER_INTERFACE);
    let result = run_cli(&["compare", &left, &right], &dir);
    assert_eq!(result.code, 2);
    assert!(!result.stderr_text().contains("Traceback"));
}

#[test]
fn cli_undecodable_content_refused_as_not_utf8_text() {
    let dir = temp_dir("cli-badutf");
    std::fs::write(
        dir.join("bad.tape"),
        b"interface shard-store\n\nnote = \xff\xfe\n",
    )
    .expect("file written");
    write(&dir, "b.tape", AGREEING);
    let result = run_cli(&["compare", "bad.tape", "b.tape"], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(
        result.stderr,
        b"error: bad.tape is not readable as utf-8 text\n".to_vec()
    );
}

#[test]
fn cli_empty_path_argument_names_the_current_directory() {
    let dir = temp_dir("cli-empty-path");
    write(&dir, "b.tape", AGREEING);
    let result = run_cli(&["compare", "", "b.tape"], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(
        result.stderr,
        b"error: cannot read ., Is a directory\n".to_vec()
    );
}

#[test]
fn cli_directory_refusal_names_the_folded_path() {
    let dir = temp_dir("cli-dir");
    std::fs::create_dir(dir.join("adir")).expect("directory created");
    write(&dir, "b.tape", AGREEING);
    let result = run_cli(&["compare", "adir/", "b.tape"], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(
        result.stderr,
        b"error: cannot read adir, Is a directory\n".to_vec()
    );
}

#[test]
fn cli_read_refusal_names_the_folded_path() {
    let dir = temp_dir("cli-folded");
    write(&dir, "b.tape", AGREEING);
    let result = run_cli(&["compare", "./missing.tape", "b.tape"], &dir);
    assert_eq!(result.code, 2);
    assert_eq!(
        result.stderr,
        b"error: cannot read missing.tape, No such file or directory\n".to_vec()
    );
    write(&dir, "a.tape", AGREEING);
    let right = write(&dir, "c.tape", DIVERGENT);
    let named = run_cli(&["compare", "./a.tape", &right], &dir);
    assert_eq!(named.code, 1);
    assert!(
        named.stdout_text().contains("left (./a.tape) timeout = 30"),
        "{}",
        named.stdout_text()
    );
}

#[test]
fn cli_non_utf8_path_prints_raw_in_the_report_and_escaped_in_the_refusal() {
    use std::os::unix::ffi::OsStrExt;
    let dir = temp_dir("cli-bytes");
    let raw_name: &[u8] = b"\xff.tape";
    let raw_path = dir.join(std::ffi::OsStr::from_bytes(raw_name));
    std::fs::write(&raw_path, DIVERGENT).expect("file written");
    write(&dir, "a.tape", AGREEING);
    let result = run_cli_bytes(&[b"compare", b"a.tape", raw_name], &dir);
    assert_eq!(result.code, 1);
    let mut expected: Vec<u8> = Vec::new();
    expected.extend_from_slice(
        b"divergence shard-store timeout term-value\n  left (a.tape) timeout = 30\n  right (",
    );
    expected.extend_from_slice(raw_name);
    expected.extend_from_slice(b") timeout = 45\n");
    assert_eq!(result.stdout, expected);
    let missing = run_cli_bytes(&[b"compare", b"\xfe.tape", b"a.tape"], &dir);
    assert_eq!(missing.code, 2);
    assert_eq!(
        missing.stderr,
        b"error: cannot read \\udcfe.tape, No such file or directory\n".to_vec()
    );
}
