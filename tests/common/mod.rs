//! Shared helpers for the ported test corpus.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use forkdrift::divergence::CompareError;
use forkdrift::{Artifact, Divergence, ForkdriftError, compare_artifacts, format_report};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Parse an artifact that must parse.
pub fn artifact(text: &str, source: &str) -> Artifact {
    forkdrift::parse_artifact(text, source.as_bytes()).expect("artifact parses")
}

/// Parse an artifact that must refuse, and return the
/// refusal.
pub fn artifact_err(text: &str, source: &str) -> ForkdriftError {
    forkdrift::parse_artifact(text, source.as_bytes()).expect_err("artifact refuses")
}

/// Compare two artifacts that must compare.
pub fn compare(left: &Artifact, right: &Artifact) -> Vec<Divergence> {
    compare_artifacts(left, right).expect("comparison runs")
}

/// Compare two artifacts and return the failure.
pub fn compare_err(left: &Artifact, right: &Artifact) -> CompareError {
    compare_artifacts(left, right).expect_err("comparison refuses")
}

/// Render a report as text, for the tests whose sources are
/// all text.
pub fn report_text(found: &[Divergence]) -> String {
    String::from_utf8(format_report(found)).expect("report is text")
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub fn temp_dir(tag: &str) -> PathBuf {
    let number = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "forkdrift-test-{}-{}-{}",
        std::process::id(),
        tag,
        number
    ));
    // The name rests on the process id and a counter that
    // starts again in every process, so a directory a dead
    // process left behind can stand where a later process
    // with a recycled id asks for the same name. Clear any
    // such leftover before reuse, so no test inherits files
    // it did not write.
    if dir.exists() {
        std::fs::remove_dir_all(&dir).expect("stale temp dir cleared");
    }
    std::fs::create_dir_all(&dir).expect("temp dir created");
    dir
}

pub struct CliOutput {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl CliOutput {
    pub fn stdout_text(&self) -> String {
        String::from_utf8(self.stdout.clone()).expect("stdout is UTF-8")
    }

    pub fn stderr_text(&self) -> String {
        String::from_utf8(self.stderr.clone()).expect("stderr is UTF-8")
    }
}

pub fn run_cli(args: &[&str], cwd: &std::path::Path) -> CliOutput {
    let output = Command::new(env!("CARGO_BIN_EXE_forkdrift"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("binary runs");
    CliOutput {
        code: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

/// Run the binary with argument bytes that are not text, the
/// shape a path whose bytes are not valid UTF-8 arrives in.
pub fn run_cli_bytes(args: &[&[u8]], cwd: &std::path::Path) -> CliOutput {
    use std::os::unix::ffi::OsStrExt;
    let output = Command::new(env!("CARGO_BIN_EXE_forkdrift"))
        .args(
            args.iter()
                .map(|arg| std::ffi::OsStr::from_bytes(arg))
                .collect::<Vec<_>>(),
        )
        .current_dir(cwd)
        .output()
        .expect("binary runs");
    CliOutput {
        code: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
        stderr: output.stderr,
    }
}

/// Run the binary at the repo root, the working directory
/// the example tests use.
pub fn run_cli_at_root(args: &[&str]) -> CliOutput {
    run_cli(args, &root())
}

/// Run the binary with its standard output standing on a
/// pipe no one reads, so every byte it prints meets a dead
/// output, and return its exit code and standard error. The
/// reading end is dropped before the binary starts, so the
/// pipe is dead from the first byte and never raced, and a
/// byte count the output reaches is the only variable.
pub fn run_cli_on_closed_stdout(args: &[&str], cwd: &std::path::Path) -> CliOutput {
    let (reader, writer) = std::io::pipe().expect("pipe stands");
    drop(reader);
    let output = Command::new(env!("CARGO_BIN_EXE_forkdrift"))
        .args(args)
        .current_dir(cwd)
        .stdout(writer)
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("binary runs");
    CliOutput {
        code: output.status.code().unwrap_or(-1),
        stdout: Vec::new(),
        stderr: output.stderr,
    }
}

/// Run the binary with its error stream standing on a pipe
/// no one reads, so every line it writes there meets a dead
/// stream, and return its exit code and standard output.
/// The reading end is dropped before the binary starts, so
/// the pipe is dead from the first byte and never raced.
pub fn run_cli_on_closed_stderr(args: &[&str], cwd: &std::path::Path) -> CliOutput {
    let (reader, writer) = std::io::pipe().expect("pipe stands");
    drop(reader);
    let output = Command::new(env!("CARGO_BIN_EXE_forkdrift"))
        .args(args)
        .current_dir(cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(writer)
        .output()
        .expect("binary runs");
    CliOutput {
        code: output.status.code().unwrap_or(-1),
        stdout: output.stdout,
        stderr: Vec::new(),
    }
}

/// Run the binary with both streams standing on pipes no
/// one reads, so nothing it writes anywhere can land, and
/// return its exit code alone, the only observable a dead
/// pair of streams leaves.
pub fn run_cli_on_closed_streams(args: &[&str], cwd: &std::path::Path) -> CliOutput {
    let (out_reader, out_writer) = std::io::pipe().expect("pipe stands");
    drop(out_reader);
    let (err_reader, err_writer) = std::io::pipe().expect("pipe stands");
    drop(err_reader);
    let status = Command::new(env!("CARGO_BIN_EXE_forkdrift"))
        .args(args)
        .current_dir(cwd)
        .stdout(out_writer)
        .stderr(err_writer)
        .status()
        .expect("binary runs");
    CliOutput {
        code: status.code().unwrap_or(-1),
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

/// Write a text file inside a directory and return its path
/// as a string, the shape the command line tests pass.
pub fn write(dir: &std::path::Path, name: &str, text: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("file written");
    path.to_string_lossy().into_owned()
}
