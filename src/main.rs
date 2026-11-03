//! Command line entry point, the forkdrift binary.
//!
//! Subcommands
//!   compare  compare two recorded artifacts
//!   sweep    compare one baseline artifact against several
//!            others
//!
//! The argument parsing is clap, adopted under the
//! maintainer's tool choice ruling. Before clap sees the
//! arguments, a preprocess walks them the way the Python
//! parser's own scan walked them. It fires help where that
//! scan fired it and gathers the positional values in the
//! order they stand, so the functional outcomes of the two
//! command lines agree, the same subcommands and arguments,
//! misuse at exit status 2, and help at exit status 0. One
//! reading is the port's own correction of the walk it
//! follows. A separator standing before any subcommand is
//! itself the candidate the Python parser reads for the
//! subcommand slot, so it blocks the walk the way an invalid
//! candidate does, where the walk's model treats a separator
//! as a separator wherever it stands (the probe correction
//! to FD-11). Help text and malformed invocation error text
//! are clap's own forms. The documents the tool emits and
//! the one error line discipline for its own refusals are
//! unchanged. A failure the Python tool lets escape uncaught
//! answers at status 1, and a standard output failure that
//! surfaces only at its exit flush answers at status 120,
//! while one that surfaces during the write answers at
//! status 1, because the Python command line holds the
//! report write outside the handlers that catch its own
//! refusals (FD-30). Errors in main ride on anyhow. The
//! library's error types derive thiserror.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand};

use forkdrift::artifacts::{Artifact, escape_source, parse_artifact};
use forkdrift::divergence::{CompareError, compare_artifacts, format_report, format_sweep};
use forkdrift::pytext;

#[derive(Parser)]
#[command(
    about = "Detect semantic divergence between two recorded artifacts for one interface.",
    subcommand_required = true,
    infer_long_args = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compare two recorded artifacts.
    #[command(infer_long_args = true, args_override_self = true)]
    Compare(CompareArgs),
    /// Compare one baseline artifact against several others.
    #[command(infer_long_args = true, args_override_self = true)]
    Sweep(SweepArgs),
}

/// The value parser of a positional path argument, reading
/// the value as the path it stands for, the empty string
/// included. The path parser clap derives refuses an empty
/// value in its own voice, ahead of the tool, whose path
/// discipline answers an empty path in the path library's
/// form, as the current directory. A positional path is
/// therefore read by this parser, so the empty value reaches
/// the tool's own handling in every positional slot, and a
/// value whose bytes are not text reaches it unchanged, as
/// it reaches the Python parser.
#[derive(Clone, Debug)]
struct PathValueParser;

impl clap::builder::TypedValueParser for PathValueParser {
    type Value = PathBuf;

    fn parse_ref(
        &self,
        _cmd: &clap::Command,
        _arg: Option<&clap::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::Error> {
        Ok(PathBuf::from(value))
    }
}

#[derive(Args)]
struct CompareArgs {
    /// Path to the left recorded artifact.
    #[arg(value_parser = PathValueParser)]
    left: PathBuf,
    /// Path to the right recorded artifact.
    #[arg(value_parser = PathValueParser)]
    right: PathBuf,
}

#[derive(Args)]
struct SweepArgs {
    /// Path to the baseline recorded artifact.
    #[arg(value_parser = PathValueParser)]
    baseline: PathBuf,
    /// Paths to the other recorded artifacts.
    #[arg(value_parser = PathValueParser, required = true, num_args = 1..)]
    others: Vec<PathBuf>,
}

/// A failure the Python tool lets escape uncaught.
///
/// The binary carries the failure in its own type and main
/// answers it with status 1 and one error line, where every
/// refusal the tool makes on purpose answers with status 2.
#[derive(Debug)]
struct Uncaught(String);

impl std::fmt::Display for Uncaught {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Uncaught {}

/// A standard output failure that surfaces during the
/// write, which the Python tool lets escape uncaught
/// because the write stands outside its refusal handlers.
///
/// The binary carries the failure in its own type and main
/// answers it with status 1 and one error line (FD-30).
#[derive(Debug)]
struct PrintFailed(String);

impl std::fmt::Display for PrintFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PrintFailed {}

/// A standard output failure the Python tool discovers only
/// when its buffer is flushed as the tool exits.
///
/// The Python tool holds its printed output in a buffer of
/// 8192 bytes, so a failure under a smaller output is never
/// discovered during the run. It surfaces at the shutdown
/// flush, where the Python tool reports it as an exception
/// ignored and exits with status 120 and no traceback, so
/// the binary carries the failure in its own type and main
/// answers it with status 120 and one error line (FD-30).
#[derive(Debug)]
struct FailedFlush(String);

impl std::fmt::Display for FailedFlush {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FailedFlush {}

/// The size of the buffer the Python tool holds its printed
/// output in, in bytes. A printed output of this size or
/// more is written through as it is printed, so a failure
/// under it surfaces during the run, where it escapes the
/// Python command line uncaught, and a smaller output is
/// flushed only as the tool exits.
const STDOUT_BUFFER_BYTES: usize = 8192;

fn print_out(text: &[u8]) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(text)
        .and_then(|()| lock.flush())
        .map_err(|error| {
            if text.len() < STDOUT_BUFFER_BYTES {
                anyhow::Error::new(FailedFlush(
                    "forkdrift could not print its report".to_string(),
                ))
            } else {
                anyhow::Error::new(PrintFailed(format!(
                    "forkdrift could not print its report, {error}"
                )))
            }
        })
}

/// The platform error text of a failed read, without the
/// suffix the Rust error rendering adds, so the refusal line
/// carries the same words the Python tool's handler reads
/// off its own error.
fn strerror(error: &std::io::Error) -> String {
    let text = error.to_string();
    match error.raw_os_error() {
        Some(code) => text
            .strip_suffix(&format!(" (os error {code})"))
            .map_or(text.clone(), str::to_string),
        None => text,
    }
}

/// The folded form of a path, the form the oracle's path
/// library opens and names in its read refusals (the probe
/// correction to FD-10). Empty components and single dot
/// components fall away, dot dot components stay, a leading
/// pair of slashes stays a pair while any longer run folds
/// to one, and a path left with no components names a single
/// dot.
fn py_path_folded(path: &[u8]) -> Vec<u8> {
    if path.is_empty() {
        return b".".to_vec();
    }
    let slashes = path.iter().take_while(|byte| **byte == b'/').count();
    let anchor: &[u8] = if slashes == 2 {
        b"//"
    } else if slashes >= 1 {
        b"/"
    } else {
        b""
    };
    let mut parts: Vec<&[u8]> = Vec::new();
    for part in path[slashes..].split(|byte| *byte == b'/') {
        if part.is_empty() || part == b"." {
            continue;
        }
        parts.push(part);
    }
    if parts.is_empty() {
        return if anchor.is_empty() {
            b".".to_vec()
        } else {
            anchor.to_vec()
        };
    }
    let mut out = anchor.to_vec();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            out.push(b'/');
        }
        out.extend_from_slice(part);
    }
    out
}

/// Read one recorded artifact under the name it was given.
///
/// The file is opened under the folded form of its path and
/// a failed read names that folded form, while the artifact
/// itself stands under the path as written, so reports and
/// parse refusals name it as written (the probe correction
/// to FD-10).
fn read_artifact(path: &Path) -> anyhow::Result<Artifact> {
    use std::os::unix::ffi::OsStrExt;
    let given = path.as_os_str().as_bytes();
    let folded = py_path_folded(given);
    let bytes = std::fs::read(std::ffi::OsStr::from_bytes(&folded)).map_err(|error| {
        anyhow::Error::msg(format!(
            "cannot read {}, {}",
            escape_source(&folded),
            strerror(&error)
        ))
    })?;
    let text = String::from_utf8(bytes).map_err(|_| {
        anyhow::Error::msg(format!(
            "{} is not readable as utf-8 text",
            escape_source(given)
        ))
    })?;
    Ok(parse_artifact(&text, given)?)
}

/// Carry a comparison failure into main, keeping the
/// uncaught kind apart so the exit status follows it.
fn compare_failure(error: CompareError) -> anyhow::Error {
    match error {
        CompareError::Refusal(refusal) => anyhow::Error::new(refusal),
        CompareError::Worth(worth) => anyhow::Error::new(Uncaught(format!(
            "forkdrift could not finish the comparison, {worth}"
        ))),
    }
}

fn cmd_compare(args: &CompareArgs) -> anyhow::Result<i32> {
    let left = read_artifact(&args.left)?;
    let right = read_artifact(&args.right)?;
    let divergences = compare_artifacts(&left, &right).map_err(compare_failure)?;
    print_out(&format_report(&divergences))?;
    Ok(i32::from(!divergences.is_empty()))
}

fn cmd_sweep(args: &SweepArgs) -> anyhow::Result<i32> {
    let baseline = read_artifact(&args.baseline)?;
    let mut others = Vec::new();
    let mut results = Vec::new();
    for path in &args.others {
        let other = read_artifact(path)?;
        results.push(compare_artifacts(&baseline, &other).map_err(compare_failure)?);
        others.push(other);
    }
    let diverged = results.iter().any(|pair| !pair.is_empty());
    print_out(&format_sweep(&baseline, &others, &results))?;
    Ok(i32::from(diverged))
}

/// The exit status an error from a command answers with.
///
/// A failure the Python tool lets escape uncaught answers
/// at status 1, and a failed print during the write is one
/// of those. A standard output failure discovered only at
/// the exit flush answers with status 120, the status the
/// Python tool exits with when it reports that failure as
/// an exception ignored. Every other refusal the tool makes
/// answers with status 2.
fn refusal_status(error: &anyhow::Error) -> u8 {
    if error.downcast_ref::<FailedFlush>().is_some() {
        return 120;
    }
    if error.downcast_ref::<Uncaught>().is_some() || error.downcast_ref::<PrintFailed>().is_some() {
        return 1;
    }
    2
}

/// Whether the token after its dash reads as a negative
/// number under the argparse pattern, one run of decimal
/// digits, or a run of digits, a point, and another run.
/// The digit set is the full Unicode decimal set, as it is
/// in a Python text pattern.
fn is_negative_number(token: &str) -> bool {
    let Some(body) = token.strip_prefix('-') else {
        return false;
    };
    let digit = |ch: char| pytext::is_decimal(ch);
    if !body.is_empty() && body.chars().all(digit) {
        return true;
    }
    match body.split_once('.') {
        Some((whole, fraction)) => {
            whole.chars().all(digit) && !fraction.is_empty() && fraction.chars().all(digit)
        }
        None => false,
    }
}

/// Whether the negative number pattern matches the token,
/// where the pattern's end anchor also matches just before
/// one trailing newline, as a Python text pattern's does.
fn negative_number_matches(token: &str) -> bool {
    is_negative_number(token) || token.strip_suffix('\n').is_some_and(is_negative_number)
}

/// Whether a token reads as a positional string rather than
/// as an option, the classification the Python parser makes
/// when it meets the token. A bare word and a lone dash are
/// positional. A dash token carrying a space reads as
/// positional whatever its prefix, the one exception being
/// the short help family, whose letter after the dash
/// decides ahead of the space rule, as the Python parser
/// refuses a help token with a trailing word in a value
/// position rather than consuming it. A dash token without
/// a space is positional only when it matches the negative
/// number pattern. Any long token without a space is an
/// option.
fn reads_as_positional(token: &str) -> bool {
    if token == "--" {
        return false;
    }
    if !token.starts_with('-') || token == "-" {
        return true;
    }
    if token.chars().nth(1) == Some('h') {
        return false;
    }
    if token.contains(' ') {
        return true;
    }
    if token.starts_with("--") {
        return false;
    }
    negative_number_matches(token)
}

/// One subcommand of the command line, with the long flags
/// the derive model declares for it.
///
/// The flag tables of the preprocess come from clap's own
/// model of the derive structs, read through the command
/// factory, so the derive model stays the one owner of the
/// flag set and this layer restates none of it. Forkdrift
/// declares no long flags of its own, so the tables hold the
/// help flag alone, and the walk's work is the positional
/// and help discipline.
struct SubcommandSpec {
    name: String,
    longs: Vec<String>,
}

fn subcommand_specs() -> Vec<SubcommandSpec> {
    Cli::command()
        .get_subcommands()
        .map(|sub| {
            let mut longs: Vec<String> = Vec::new();
            for arg in sub.get_arguments() {
                let Some(long) = arg.get_long() else {
                    continue;
                };
                let name = format!("--{long}");
                if !longs.contains(&name) {
                    longs.push(name);
                }
            }
            if !longs.iter().any(|long| long == "--help") {
                longs.push("--help".to_string());
            }
            SubcommandSpec {
                name: sub.get_name().to_string(),
                longs,
            }
        })
        .collect()
}

/// How a token reads as a help token on the current parser.
enum HelpRead {
    /// Not a help token.
    No,
    /// The help action fires.
    Fire,
    /// A help token carrying an explicit value, an error in
    /// both parsers. The scan carries the token in the form
    /// clap refuses in its own voice, and nothing later can
    /// fire help.
    Refuse(OsString),
}

/// Read a token as a help token against the long flags of
/// the parser the scan stands on. The Python parser fires
/// help when its scan reaches the token, ahead of extra
/// tokens it has already collected, and it refuses a help
/// token that carries an explicit value, ahead of any help
/// token that follows. A short help token fires when its
/// bundle continues with anything but an equals sign or a
/// dash, the two continuations the parser reads as an
/// explicit value arriving with the token.
fn help_read(token: &str, longs: &[String]) -> HelpRead {
    if token.starts_with("--") {
        let name = token.split('=').next().unwrap_or(token);
        if name.len() > 2 {
            let matches: Vec<&String> =
                longs.iter().filter(|long| long.starts_with(name)).collect();
            if matches.len() == 1 && matches[0] == "--help" {
                if token.contains('=') {
                    return HelpRead::Refuse(OsString::from(token));
                }
                return HelpRead::Fire;
            }
        }
        return HelpRead::No;
    }
    if token.starts_with('-') && token.chars().nth(1) == Some('h') {
        return match token.chars().nth(2) {
            None => HelpRead::Fire,
            Some('=') => HelpRead::Refuse(OsString::from(format!("--help={}", &token[3..]))),
            Some('-') => HelpRead::Refuse(OsString::from(format!("--help={}", &token[2..]))),
            Some(_) => HelpRead::Fire,
        };
    }
    HelpRead::No
}

/// The outcome of the preprocessed argument walk.
enum Preprocess {
    /// Hand these arguments to clap.
    Run(Vec<OsString>),
    /// Print the help of the named subcommand parser, or of
    /// the top level parser when the name is absent, at
    /// status 0.
    FireHelp(Option<String>),
}

/// Walk the arguments the way the Python parser's scan
/// walked them and prepare them for clap.
///
/// The Python parser consumes its arguments from the left.
/// Positional values are gathered in the order they stand
/// and handed to clap behind a separator, so a value that
/// reads as an option string reaches its slot as a value,
/// and inside a subcommand the separator itself is consumed
/// wherever it stands, one separator only, with every token
/// after it a positional value, a second separator among
/// them. A separator standing before any subcommand is read
/// as the subcommand candidate itself, so it blocks the
/// walk, as do a word the subcommand slot cannot take and a
/// help token carrying an explicit value, and past any of
/// those this walk hands the arguments to clap as they
/// stand, since clap refuses the same invocation in its own
/// voice. An option token the current parser does not
/// declare is only collected by the scan, ahead of the help
/// check on the tokens after it, and it stands in the
/// stream handed to clap, which refuses it there. Help
/// fires where the scan reaches it, ahead of extra tokens
/// already collected, and this walk answers a fired help
/// token itself.
fn preprocess(tokens: &[OsString]) -> Preprocess {
    let specs = subcommand_specs();
    let root_longs: Vec<String> = vec!["--help".to_string()];
    let mut raw: Vec<OsString> = Vec::new();
    let mut out: Vec<OsString> = Vec::new();
    let mut values: Vec<OsString> = Vec::new();
    let mut current: Option<usize> = None;
    let mut blocked = false;
    let mut separated = false;
    for token in tokens {
        if separated {
            raw.push(token.clone());
            values.push(token.clone());
            continue;
        }
        if blocked {
            raw.push(token.clone());
            continue;
        }
        let text = token.to_string_lossy().into_owned();
        if text == "--" {
            raw.push(token.clone());
            if current.is_none() {
                blocked = true;
            } else {
                separated = true;
            }
            continue;
        }
        let longs: &[String] = match current {
            Some(index) => &specs[index].longs,
            None => &root_longs,
        };
        match help_read(&text, longs) {
            HelpRead::Fire => {
                return Preprocess::FireHelp(current.map(|index| specs[index].name.clone()));
            }
            HelpRead::Refuse(replacement) => {
                blocked = true;
                raw.push(replacement);
                continue;
            }
            HelpRead::No => {}
        }
        raw.push(token.clone());
        if current.is_none() {
            if let Some(index) = specs.iter().position(|spec| spec.name == text) {
                current = Some(index);
                out.push(token.clone());
            } else if reads_as_positional(&text) {
                blocked = true;
            } else {
                // An option token at the top level is only
                // collected, and a help token after it still
                // fires.
                out.push(token.clone());
            }
            continue;
        }
        if reads_as_positional(&text) {
            values.push(token.clone());
        } else {
            // An option this parser does not declare is only
            // collected, and clap refuses it in the stream.
            out.push(token.clone());
        }
    }
    if blocked {
        return Preprocess::Run(raw);
    }
    if !values.is_empty() {
        out.push(OsString::from("--"));
        out.extend(values);
    }
    Preprocess::Run(out)
}

/// Write text to the error stream in one plain write and
/// answer with the status the text carries.
///
/// The write is handled and never panicking, because a
/// panicking write answers with the panic status, which is
/// neither the status the text carries nor the status a
/// failed write answers with. When the write lands, the
/// status the caller passes stands. When it cannot land,
/// the answer is status 120, the exit flush shape the probe
/// correction to FD-30 records for the oracle's own lines
/// on a dead error stream. Every line the tool writes to
/// its error stream goes through this write.
fn answer_error_line(text: &str, status: u8) -> ExitCode {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    match lock.write_all(text.as_bytes()) {
        Ok(()) => ExitCode::from(status),
        Err(_) => ExitCode::from(120),
    }
}

/// Answer a clap error the way the command line answers it.
///
/// An error clap prints to standard error, a malformed
/// invocation, is printed there and answered with its own
/// exit code, and it goes through the same handled error
/// stream write as the tool's own lines, so an error stream
/// that cannot take the text answers with status 120 under
/// the probe correction to FD-30, where the parser's own
/// print would have kept the code over a write that never
/// landed. An error clap prints to standard output is
/// the help text, and help output takes the failed write
/// discipline of every document the tool prints, because
/// the Python tool's help goes to the same buffered
/// standard output its documents do. The help is rendered
/// and passed through print_out, so a standard output that
/// cannot take the bytes answers at the status the failure
/// family carries, 120 under a small output and status 1
/// under a larger one, with the tool's one error line,
/// rather than exiting 0 over a print whose failure was
/// discarded.
fn answer_clap_error(error: clap::Error) -> ExitCode {
    if error.use_stderr() {
        let code = error.exit_code();
        let text = error.render().to_string();
        return answer_error_line(&text, code as u8);
    }
    let text = error.render().to_string();
    match print_out(text.as_bytes()) {
        Ok(()) => ExitCode::from(error.exit_code() as u8),
        Err(failure) => answer_error_line(&format!("error: {failure}\n"), refusal_status(&failure)),
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let program = args.next().unwrap_or_else(|| OsString::from("forkdrift"));
    let rest: Vec<OsString> = args.collect();
    let joined = match preprocess(&rest) {
        Preprocess::Run(joined) => joined,
        Preprocess::FireHelp(subcommand) => {
            let mut argv = vec![program];
            if let Some(name) = subcommand {
                argv.push(OsString::from(name));
            }
            argv.push(OsString::from("--help"));
            return match Cli::try_parse_from(argv) {
                Ok(_) => ExitCode::from(0),
                Err(error) => answer_clap_error(error),
            };
        }
    };
    let cli = match Cli::try_parse_from(std::iter::once(program).chain(joined)) {
        Ok(cli) => cli,
        Err(error) => return answer_clap_error(error),
    };
    let outcome = match &cli.command {
        Command::Compare(args) => cmd_compare(args),
        Command::Sweep(args) => cmd_sweep(args),
    };
    match outcome {
        Ok(code) => ExitCode::from(code as u8),
        Err(error) => answer_error_line(&format!("error: {error}\n"), refusal_status(&error)),
    }
}
