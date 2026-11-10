//! Command line entry point, the forkdrift binary.
//!
//! Subcommands
//!   compare    compare two recorded artifacts
//!   sweep      compare one baseline artifact against several
//!              others
//!   reconcile  stage two recorded branch states over a
//!              divergence record and hand back the sized
//!              reconciliation tasks
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
//! refusals (FD-30). The reconcile subcommand carries the
//! print discipline of its own surface (XC-29), so its
//! documents print through their own print function, and
//! its separator distribution and its list answer stand as
//! the corrections to XC-12 and XC-22 record them. Errors
//! in main ride on anyhow. The library's error types derive
//! thiserror.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, CommandFactory, Parser, Subcommand};

use forkdrift::artifacts::{Artifact, escape_source, parse_artifact};
use forkdrift::divergence::{CompareError, compare_artifacts, format_report, format_sweep};
use forkdrift::pytext;
use forkdrift::reconcile::records::{Divergence, bundle_divergences, parse_divergence_record};
use forkdrift::reconcile::staging::{cross_apply, format_stage_report};
use forkdrift::reconcile::states::{BranchState, parse_branch_state};
use forkdrift::reconcile::tasks::{build_tasks, format_tasks};

#[derive(Parser)]
#[command(
    about = "Tells whether two recorded artifacts that state the same interface actually diverge, and reconciles the divergences over recorded branch states. Everything works from recorded files, and nothing is executed.",
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
    /// Reconcile two recorded branch states over a divergence record.
    #[command(infer_long_args = true, args_override_self = true)]
    Reconcile(ReconcileArgs),
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

#[derive(Args)]
#[command(
    subcommand_required = true,
    infer_long_args = true,
    disable_help_subcommand = true
)]
struct ReconcileArgs {
    #[command(subcommand)]
    form: ReconcileForm,
}

#[derive(Subcommand)]
enum ReconcileForm {
    /// Print the cross-apply report for two branch states.
    #[command(infer_long_args = true, args_override_self = true)]
    Stage(StageArgs),
    /// Print one sized reconciliation task per divergence.
    #[command(infer_long_args = true, args_override_self = true)]
    Handback(HandbackArgs),
}

#[derive(Args)]
struct StageArgs {
    /// Path to the left recorded branch state.
    #[arg(value_parser = PathValueParser)]
    left: PathBuf,
    /// Path to the right recorded branch state.
    #[arg(value_parser = PathValueParser)]
    right: PathBuf,
    /// Path to the recorded divergence record.
    #[arg(value_parser = PathValueParser)]
    divergences: PathBuf,
}

#[derive(Args)]
struct HandbackArgs {
    /// Path to the left recorded branch state.
    #[arg(value_parser = PathValueParser)]
    left: PathBuf,
    /// Path to the right recorded branch state.
    #[arg(value_parser = PathValueParser)]
    right: PathBuf,
    /// Paths to the recorded divergence records of the
    /// bundle.
    #[arg(value_parser = PathValueParser, required = true, num_args = 1..)]
    divergences: Vec<PathBuf>,
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

/// A standard output failure that surfaces during the
/// write on the reconcile surface, which the Python tool
/// behind that surface catches in its own handler because
/// the write stands inside the handler's reach.
///
/// The binary carries the failure in its own type and main
/// answers it with status 2 and the handler's own line, the
/// cannot read line the handler prints for it, under the
/// print discipline of XC-29 the reconcile surface carries
/// rather than the discipline of FD-30.
#[derive(Debug)]
struct PrintCaught(String);

impl std::fmt::Display for PrintCaught {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PrintCaught {}

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

/// Print a document of the reconcile surface, under the
/// print discipline of XC-29 rather than the discipline of
/// FD-30.
///
/// A failure under the buffer size surfaces only at the
/// exit flush and answers at status 120, and one at or
/// over it surfaces during the write, where the handler of
/// that surface catches it and answers at status 2 with its
/// cannot read line. The flush line is the carried text
/// of XC-29 with its name amended under FD-60, the line
/// naming forkdrift from Round 13.
fn print_out_reconcile(text: &[u8]) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    lock.write_all(text)
        .and_then(|()| lock.flush())
        .map_err(|_error| {
            if text.len() < STDOUT_BUFFER_BYTES {
                anyhow::Error::new(FailedFlush(
                    "forkdrift could not print its report".to_string(),
                ))
            } else {
                anyhow::Error::new(PrintCaught("cannot read None, Broken pipe".to_string()))
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

/// Read one recorded text of the reconcile surface under
/// the name it was given.
///
/// The file is opened under the folded form of its path and
/// a failed read names that folded form, while the text
/// itself is parsed under the path as written, so parse
/// refusals name it as written (the probe correction to
/// XC-12). The read applies the universal newline
/// translation of XC-27, so a carriage return and line feed
/// pair and a lone carriage return both reach the parsers
/// as a line feed.
fn read_text(path: &Path) -> anyhow::Result<String> {
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
    Ok(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// Read and parse one branch state under the name it was
/// given.
fn read_state(path: &Path) -> anyhow::Result<BranchState> {
    use std::os::unix::ffi::OsStrExt;
    let text = read_text(path)?;
    let source = escape_source(path.as_os_str().as_bytes());
    Ok(parse_branch_state(&text, &source)?)
}

/// Read and parse one divergence record under the name it
/// was given, returning the name with the divergences.
fn read_record(path: &Path) -> anyhow::Result<(String, Vec<Divergence>)> {
    use std::os::unix::ffi::OsStrExt;
    let text = read_text(path)?;
    let source = escape_source(path.as_os_str().as_bytes());
    let divergences = parse_divergence_record(&text, &source)?;
    Ok((source, divergences))
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

fn cmd_stage(args: &StageArgs) -> anyhow::Result<i32> {
    let left = read_state(&args.left)?;
    let right = read_state(&args.right)?;
    let (_, divergences) = read_record(&args.divergences)?;
    finish_stage(&left, &right, &divergences)
}

/// Cross-apply the two branch states over the divergences
/// and print the staging report.
fn finish_stage(
    left: &BranchState,
    right: &BranchState,
    divergences: &[Divergence],
) -> anyhow::Result<i32> {
    let staging = cross_apply(left, right, divergences)?;
    print_out_reconcile(format_stage_report(&staging).as_bytes())?;
    Ok(0)
}

fn cmd_handback(args: &HandbackArgs) -> anyhow::Result<i32> {
    let left = read_state(&args.left)?;
    let right = read_state(&args.right)?;
    let mut records = Vec::new();
    for path in &args.divergences {
        records.push(read_record(path)?);
    }
    finish_handback(&left, &right, &records)
}

/// Bundle the records, cross-apply the two branch states
/// over the union, build the tasks, and print the handback
/// document.
fn finish_handback(
    left: &BranchState,
    right: &BranchState,
    records: &[(String, Vec<Divergence>)],
) -> anyhow::Result<i32> {
    let divergences = bundle_divergences(records)?;
    let staging = cross_apply(left, right, &divergences)?;
    let tasks = build_tasks(left, right, &divergences, &staging)?;
    print_out_reconcile(format_tasks(&tasks).as_bytes())?;
    Ok(0)
}

/// One positional slot of a reconcile form after the
/// separator distribution the probe correction to XC-12
/// and XC-22 records.
enum Slot {
    /// The slot received the one path token of its share.
    One(OsString),
    /// The slot received a list in place of a path, because
    /// the deletion of its share's separator token left the
    /// share holding no token. The text names the slot the
    /// way the uncaught line names it.
    Listed(&'static str),
}

/// A reconcile form whose arguments the separator
/// distribution assigned in full, ready to run without
/// clap.
enum Assigned {
    /// The stage form and its three slots in read order.
    Stage {
        /// The left branch state slot.
        left: Slot,
        /// The right branch state slot.
        right: Slot,
        /// The divergence record slot.
        record: Slot,
    },
    /// The handback form, its two branch state slots in
    /// read order, and the record paths of its bundle
    /// share, which may be empty.
    Handback {
        /// The left branch state slot.
        left: Slot,
        /// The right branch state slot.
        right: Slot,
        /// The record paths of the bundle share.
        records: Vec<OsString>,
    },
}

/// The path a distributed slot hands to its read, or the
/// uncaught answer when the slot received a list in place
/// of a path. The line is the carried text of the
/// correction to XC-12 with its name amended under FD-60,
/// the line naming forkdrift from Round 13.
fn slot_path(slot: &Slot) -> anyhow::Result<PathBuf> {
    match slot {
        Slot::One(path) => Ok(PathBuf::from(path)),
        Slot::Listed(name) => Err(anyhow::Error::new(Uncaught(format!(
            "forkdrift could not finish the reconciliation, {name} was given as a list"
        )))),
    }
}

/// Run a reconcile form whose arguments the separator
/// distribution assigned, without clap.
///
/// The slots resolve in read order, so a list in one slot
/// is met only when the read of that slot is reached, after
/// the slots before it have read and parsed under their
/// standing refusals.
fn cmd_assigned(assigned: &Assigned) -> anyhow::Result<i32> {
    match assigned {
        Assigned::Stage {
            left,
            right,
            record,
        } => {
            let left = read_state(&slot_path(left)?)?;
            let right = read_state(&slot_path(right)?)?;
            let (_, divergences) = read_record(&slot_path(record)?)?;
            finish_stage(&left, &right, &divergences)
        }
        Assigned::Handback {
            left,
            right,
            records,
        } => {
            let left = read_state(&slot_path(left)?)?;
            let right = read_state(&slot_path(right)?)?;
            let mut named = Vec::new();
            for path in records {
                named.push(read_record(&PathBuf::from(path))?);
            }
            finish_handback(&left, &right, &named)
        }
    }
}

/// The exit status an error from a command answers with.
///
/// A failure the Python tool lets escape uncaught answers
/// at status 1, and a failed print during the write is one
/// of those. A standard output failure discovered only at
/// the exit flush answers with status 120, the status the
/// Python tool exits with when it reports that failure as
/// an exception ignored. Every other refusal the tool makes
/// answers with status 2. The reconcile surface's failures
/// land on the same statuses under XC-29, its flush failure
/// at 120, its list answer at 1 as the uncaught failure it
/// is, and its caught print failure at 2 with the refusals.
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

/// The forms of the reconcile subcommand, with the long
/// flags the derive model declares for each.
///
/// The flag tables come from clap's own model in the
/// manner of subcommand_specs, read off the reconcile
/// command of the built tree, so the derive model stays
/// the one owner of the flag set at this level too.
fn form_specs() -> Vec<SubcommandSpec> {
    let command = Cli::command();
    let Some(reconcile) = command
        .get_subcommands()
        .find(|sub| sub.get_name() == "reconcile")
    else {
        return Vec::new();
    };
    reconcile
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
    /// Print the help of the parser the command path names,
    /// or of the top level parser when the path is empty,
    /// at status 0.
    FireHelp(Vec<String>),
    /// Run the reconcile form the separator distribution
    /// assigned, without clap.
    Assigned(Assigned),
}

/// The hand set the preprocess walk reads help tokens
/// against on the root, holding the help long alone
/// because the root command declares no other long
/// (FD-39). The set stands in one function so the pin
/// of FD-39 can read the same table the walk reads
/// (FD-46).
fn root_longs() -> Vec<String> {
    vec!["--help".to_string()]
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
/// token itself. The walk reaches one level deeper on the
/// reconcile subcommand, where the token after it is read
/// as the candidate for its form slot the way the
/// subcommand token is read at the top level, and a
/// separator standing there blocks the walk the same way.
/// When the walk collects no option token on any level,
/// the tokens behind a form go through the separator
/// distribution the probe correction to XC-12 and XC-22
/// records instead of the gathered hand off, which assigns
/// them across the form's positional actions the way the
/// oracle parser's own matching does, a second separator
/// among the values included, and a complete distribution
/// runs the form itself, because the list a single value
/// action can receive under the distribution has no form
/// the gathered hand off can carry. Every other shape on
/// that surface keeps the gathered hand off.
fn preprocess(tokens: &[OsString]) -> Preprocess {
    let specs = subcommand_specs();
    let forms = form_specs();
    let root_longs: Vec<String> = root_longs();
    let reconcile = specs.iter().position(|spec| spec.name == "reconcile");
    let mut raw: Vec<OsString> = Vec::new();
    let mut out: Vec<OsString> = Vec::new();
    let mut values: Vec<OsString> = Vec::new();
    let mut current: Option<usize> = None;
    let mut form: Option<usize> = None;
    let mut blocked = false;
    let mut separated = false;
    let mut root_option = false;
    let mut mid_option = false;
    let mut form_option = false;
    let mut form_start: Option<usize> = None;
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
            if current.is_none() || (current == reconcile && form.is_none()) {
                blocked = true;
            } else {
                separated = true;
            }
            continue;
        }
        let longs: &[String] = match (current, form) {
            (Some(_), Some(form_index)) => &forms[form_index].longs,
            (Some(index), None) => &specs[index].longs,
            (None, _) => &root_longs,
        };
        match help_read(&text, longs) {
            HelpRead::Fire => {
                let mut path: Vec<String> = Vec::new();
                if let Some(index) = current {
                    path.push(specs[index].name.clone());
                    if let Some(form_index) = form {
                        path.push(forms[form_index].name.clone());
                    }
                }
                return Preprocess::FireHelp(path);
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
                root_option = true;
                out.push(token.clone());
            }
            continue;
        }
        if current == reconcile && form.is_none() {
            if let Some(form_index) = forms.iter().position(|spec| spec.name == text) {
                form = Some(form_index);
                form_start = Some(raw.len() - 1);
                out.push(token.clone());
            } else if reads_as_positional(&text) {
                blocked = true;
            } else {
                // An option token on the reconcile command
                // is only collected, and clap refuses it in
                // the stream.
                mid_option = true;
                out.push(token.clone());
            }
            continue;
        }
        if reads_as_positional(&text) {
            values.push(token.clone());
        } else {
            // An option this parser does not declare is only
            // collected, and clap refuses it in the stream.
            if form.is_some() {
                form_option = true;
            }
            out.push(token.clone());
        }
    }
    if blocked {
        return Preprocess::Run(raw);
    }
    if form.is_some()
        && !root_option
        && !mid_option
        && !form_option
        && let (Some(form_index), Some(start)) = (form, form_start)
    {
        let handback = forms[form_index].name == "handback";
        if let Some(assigned) = distribute(handback, &raw[start + 1..]) {
            return Preprocess::Assigned(assigned);
        }
    }
    if !values.is_empty() {
        out.push(OsString::from("--"));
        out.extend(values);
    }
    Preprocess::Run(out)
}

/// Whether a token reads as a plain argument token in the
/// oracle parser's pattern scan, which is a token that does
/// not open with a dash, or a lone dash, or a token carrying
/// a space, or a token naming a negative number in the
/// pattern. A token the parser would read as an option
/// stands outside the separator distribution, so its
/// presence leaves the whole invocation to the gathered
/// hand off.
fn reads_as_plain_argument(token: &OsString) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Some(text) = token.to_str() else {
        return !token.as_os_str().as_bytes().starts_with(b"-");
    };
    if !text.starts_with('-') || text == "-" {
        return true;
    }
    if text.contains(' ') {
        return true;
    }
    negative_number_matches(text)
}

/// Whether a token is the separator token, two dashes and
/// nothing else, compared on bytes so a token whose bytes
/// are not text never reads as one.
fn is_separator_token(token: &OsString) -> bool {
    use std::os::unix::ffi::OsStrExt;
    token.as_os_str().as_bytes() == b"--"
}

/// Distribute the tokens after a reconcile form across the
/// form's positional actions the way the oracle parser's
/// pattern matching does, under the probe correction to
/// XC-12 and XC-22.
///
/// Every token before the first separator token must read
/// as a plain argument, and the assignment must be
/// complete, three argument tokens for the stage form and
/// at least three for the handback form, one to each single
/// value action in order and the rest to the handback record
/// action. The separator token joins the share of the
/// argument token just before it, or the first share when
/// it leads, and the first separator token of each share is
/// then deleted from that share. A share an action cannot
/// fill, or a token no action takes, leaves the invocation
/// to the gathered hand off, where the usage refusal answers
/// at status 2 before any file is read, as it does in the
/// oracle.
fn distribute(handback: bool, tokens: &[OsString]) -> Option<Assigned> {
    let separator = tokens.iter().position(is_separator_token);
    let head = separator.unwrap_or(tokens.len());
    if tokens[..head]
        .iter()
        .any(|token| !reads_as_plain_argument(token))
    {
        return None;
    }
    let arguments = tokens.len() - usize::from(separator.is_some());
    if arguments < 3 || (!handback && arguments != 3) {
        return None;
    }
    let group_of = |count: usize| if handback { count.min(2) } else { count };
    let mut shares: [Vec<OsString>; 3] = Default::default();
    let mut seen = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        if Some(index) == separator {
            let target = if seen == 0 { 0 } else { group_of(seen - 1) };
            shares[target].push(token.clone());
        } else {
            shares[group_of(seen)].push(token.clone());
            seen += 1;
        }
    }
    for share in &mut shares {
        if let Some(at) = share.iter().position(is_separator_token) {
            share.remove(at);
        }
    }
    let single = |share: Vec<OsString>, name: &'static str| {
        if share.len() == 1 {
            Slot::One(share.into_iter().next().unwrap())
        } else {
            Slot::Listed(name)
        }
    };
    let [left, right, rest] = shares;
    if handback {
        Some(Assigned::Handback {
            left: single(left, "the left branch state"),
            right: single(right, "the right branch state"),
            records: rest,
        })
    } else {
        Some(Assigned::Stage {
            left: single(left, "the left branch state"),
            right: single(right, "the right branch state"),
            record: single(rest, "the divergence record"),
        })
    }
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

/// Answer a clap error on the reconcile surface the way
/// that surface answers it.
///
/// The discipline is the one answer_clap_error states,
/// with the one change the surface carries. Its help text
/// prints through the reconcile print function, so a
/// standard output that cannot take the bytes answers at
/// the status the failure family of XC-29 carries, 120
/// under a small output and status 2 under a larger one,
/// with that surface's one error line.
fn answer_reconcile_clap_error(error: clap::Error) -> ExitCode {
    if error.use_stderr() {
        let code = error.exit_code();
        let text = error.render().to_string();
        return answer_error_line(&text, code as u8);
    }
    let text = error.render().to_string();
    match print_out_reconcile(text.as_bytes()) {
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
        Preprocess::Assigned(assigned) => {
            return match cmd_assigned(&assigned) {
                Ok(code) => ExitCode::from(code as u8),
                Err(error) => {
                    answer_error_line(&format!("error: {error}\n"), refusal_status(&error))
                }
            };
        }
        Preprocess::FireHelp(path) => {
            let mut argv = vec![program];
            for name in &path {
                argv.push(OsString::from(name));
            }
            argv.push(OsString::from("--help"));
            let reconcile_path = path.first().map(String::as_str) == Some("reconcile");
            return match Cli::try_parse_from(argv) {
                Ok(_) => ExitCode::from(0),
                Err(error) => {
                    if reconcile_path {
                        answer_reconcile_clap_error(error)
                    } else {
                        answer_clap_error(error)
                    }
                }
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
        Command::Reconcile(args) => match &args.form {
            ReconcileForm::Stage(stage) => cmd_stage(stage),
            ReconcileForm::Handback(handback) => cmd_handback(handback),
        },
    };
    match outcome {
        Ok(code) => ExitCode::from(code as u8),
        Err(error) => answer_error_line(&format!("error: {error}\n"), refusal_status(&error)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Derive the root long set from the built command in
    /// the manner subcommand_specs derives the long set of
    /// a subcommand, collecting the long of every argument
    /// the root command carries and completing the set with
    /// the help long where it stands absent.
    fn derived_root_longs() -> Vec<String> {
        let mut command = Cli::command();
        command.build();
        let mut longs: Vec<String> = Vec::new();
        for arg in command.get_arguments() {
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
        longs
    }

    /// FD-39. The hand set the preprocess walk reads help
    /// tokens against on the root holds the help long alone,
    /// and it holds it because the root command declares no
    /// other long. This pin derives the root long set from
    /// the built command and asserts the derived set is the
    /// one help long, so a long added to the root command,
    /// declared as an argument or raised by an attribute,
    /// fails here in the open instead of leaving the hand
    /// set standing behind the parser in silence.
    ///
    /// FD-46 adds the second assertion, that the table
    /// root_longs returns equals the derived set, so an
    /// edit to the table alone fails here as well.
    #[test]
    fn root_long_set_is_the_one_help_long() {
        assert_eq!(derived_root_longs(), vec!["--help".to_string()]);
        assert_eq!(root_longs(), derived_root_longs());
    }

    /// Derive the short of the help argument of the root
    /// command from the built command, finding the argument
    /// by its help long in the manner derived_root_longs
    /// collects the longs of the root command, and standing
    /// absent where the built command carries no argument
    /// with the help long.
    fn derived_root_help_short() -> Option<char> {
        let mut command = Cli::command();
        command.build();
        for arg in command.get_arguments() {
            if arg.get_long() == Some("help") {
                return arg.get_short();
            }
        }
        None
    }

    /// FD-42. The two reads that test the second character
    /// of a token against the letter h, the short help test
    /// of reads_as_positional and the short help branch of
    /// help_read, rest on the help argument of the root
    /// command carrying the short h. This pin derives that
    /// short from the built command and asserts it is the
    /// letter h, so a help short customized to another
    /// letter, or a root command left with no help argument
    /// at all, fails here in the open instead of leaving
    /// the two reads standing behind the parser in silence.
    #[test]
    fn root_help_short_is_the_letter_h() {
        assert_eq!(derived_root_help_short(), Some('h'));
    }

    /// Derive the short of the help argument of every
    /// command in the tree, the root command first and
    /// then its subcommands in their own order, finding
    /// each argument by its help long in the manner
    /// derived_root_longs collects the longs of the root
    /// command, and standing absent in a command's place
    /// where that command carries no argument with the
    /// help long.
    fn derived_help_shorts() -> Vec<Option<char>> {
        fn help_short(command: &clap::Command) -> Option<char> {
            for arg in command.get_arguments() {
                if arg.get_long() == Some("help") {
                    return arg.get_short();
                }
            }
            None
        }
        fn collect(command: &clap::Command, shorts: &mut Vec<Option<char>>) {
            shorts.push(help_short(command));
            for sub in command.get_subcommands() {
                collect(sub, shorts);
            }
        }
        let mut command = Cli::command();
        command.build();
        let mut shorts = Vec::new();
        collect(&command, &mut shorts);
        shorts
    }

    /// FD-45. The two reads that test the second character
    /// of a token against the letter h, the short help test
    /// of reads_as_positional and the short help branch of
    /// help_read, test that character in every context the
    /// walk reaches, on the root and behind a subcommand
    /// alike, and they rest on the help argument of every
    /// command in the tree carrying the short h. This pin
    /// derives the help short of every command from the
    /// built command and asserts each one is the letter h,
    /// so a help short customized to another letter on any
    /// command, or a command left with no help argument at
    /// all, fails here in the open instead of leaving the
    /// two reads standing behind the parser in silence.
    ///
    /// FD-56 amends the collected count this pin asserts
    /// from three to six, the count of the commands in the
    /// tree as it stands with the reconcile subcommand and
    /// its two forms joined.
    #[test]
    fn every_help_short_is_the_letter_h() {
        let shorts = derived_help_shorts();
        assert_eq!(shorts.len(), 6);
        assert!(shorts.iter().all(|short| *short == Some('h')));
    }

    /// Build the token stream a walk pin names from its
    /// spellings.
    fn walk_tokens(spellings: &[&str]) -> Vec<OsString> {
        spellings.iter().map(OsString::from).collect()
    }

    /// Assert the walk hands the tokens over to the parser
    /// in exactly the stream the pin names, token for token.
    fn assert_walk_run(spellings: &[&str], expected: &[&str]) {
        match preprocess(&walk_tokens(spellings)) {
            Preprocess::Run(stream) => {
                let stream: Vec<String> = stream
                    .iter()
                    .map(|token| token.to_string_lossy().into_owned())
                    .collect();
                let expected: Vec<String> =
                    expected.iter().map(|token| token.to_string()).collect();
                assert_eq!(stream, expected);
            }
            Preprocess::FireHelp(path) => {
                panic!("expected a handed over stream, the walk fired help for {path:?}")
            }
            Preprocess::Assigned(_) => {
                panic!("expected a handed over stream, the walk assigned a reconcile form")
            }
        }
    }

    /// Assert the walk fires the help of the parser the
    /// named command path stands on.
    fn assert_walk_help(spellings: &[&str], expected: &[&str]) {
        match preprocess(&walk_tokens(spellings)) {
            Preprocess::FireHelp(path) => {
                let expected: Vec<String> = expected.iter().map(|name| name.to_string()).collect();
                assert_eq!(path, expected);
            }
            Preprocess::Run(_) => {
                panic!("expected a fired help, the walk handed the tokens over")
            }
            Preprocess::Assigned(_) => {
                panic!("expected a fired help, the walk assigned a reconcile form")
            }
        }
    }

    /// Render one distributed slot in the words a walk pin
    /// asserts against.
    fn render_slot(slot: &Slot) -> String {
        match slot {
            Slot::One(path) => format!("one {}", path.to_string_lossy()),
            Slot::Listed(name) => format!("listed {name}"),
        }
    }

    /// Assert the walk assigns the tokens behind a form to
    /// the distribution, with the slots and the bundle the
    /// pin names.
    fn assert_walk_assigned(spellings: &[&str], expected: &str) {
        match preprocess(&walk_tokens(spellings)) {
            Preprocess::Assigned(assigned) => {
                let rendered = match &assigned {
                    Assigned::Stage {
                        left,
                        right,
                        record,
                    } => format!(
                        "stage {} {} {}",
                        render_slot(left),
                        render_slot(right),
                        render_slot(record)
                    ),
                    Assigned::Handback {
                        left,
                        right,
                        records,
                    } => format!(
                        "handback {} {} [{}]",
                        render_slot(left),
                        render_slot(right),
                        records
                            .iter()
                            .map(|record| record.to_string_lossy().into_owned())
                            .collect::<Vec<String>>()
                            .join(", ")
                    ),
                };
                assert_eq!(rendered, expected);
            }
            Preprocess::Run(_) => {
                panic!("expected an assigned form, the walk handed the tokens over")
            }
            Preprocess::FireHelp(_) => {
                panic!("expected an assigned form, the walk fired help")
            }
        }
    }

    /// FD-70, the help reads. The long help token fires the
    /// root help, because the probe correction to FD-11
    /// records that a help token fires where the scan
    /// reaches it.
    #[test]
    fn walk_help_long_fires_at_the_root() {
        assert_walk_help(&["--help"], &[]);
    }

    /// FD-70, the help reads. A prefix of the help long
    /// fires the root help, because the long arm matches by
    /// unique prefix against the table and the root table
    /// holds the help long alone.
    #[test]
    fn walk_help_long_prefix_fires_at_the_root() {
        assert_walk_help(&["--hel"], &[]);
    }

    /// FD-70, the help reads. A help long carrying a value
    /// refuses and blocks the walk, so the help token after
    /// it cannot fire. The stream stands as the refusal
    /// leaves it, the token itself and the blocked token
    /// behind it, where a plain no help reading of the
    /// first token would let the second one fire.
    #[test]
    fn walk_help_long_carrying_a_value_refuses_at_the_root() {
        assert_walk_run(&["--help=x", "--help"], &["--help=x", "--help"]);
    }

    /// FD-70, the help reads. A prefix of the help long
    /// carrying a value refuses on the same terms as the
    /// full spelling, the match being by prefix before the
    /// value is read.
    #[test]
    fn walk_help_long_prefix_carrying_a_value_refuses_at_the_root() {
        assert_walk_run(&["--hel=x", "--help"], &["--hel=x", "--help"]);
    }

    /// FD-70, the help reads. The short help token fires
    /// the root help.
    #[test]
    fn walk_help_short_fires_at_the_root() {
        assert_walk_help(&["-h"], &[]);
    }

    /// FD-70, the help reads. A short help token whose
    /// bundle continues with another letter fires, the two
    /// continuations that read as a value being the equals
    /// sign and the dash alone.
    #[test]
    fn walk_help_short_bundle_fires_at_the_root() {
        assert_walk_help(&["-hx"], &[]);
    }

    /// FD-70, the help reads. A short help token carrying
    /// a value refuses, and the walk hands the parser the
    /// rewritten long form of the refusal, so the stream
    /// carries the long spelling the token never wore. The
    /// following help token stands blocked behind the
    /// refusal, where a plain no help reading of the first
    /// token would let it fire.
    #[test]
    fn walk_help_short_carrying_a_value_refuses_at_the_root() {
        assert_walk_run(&["-h=x", "--help"], &["--help=x", "--help"]);
    }

    /// FD-70, the help reads. A short help token continuing
    /// with a dash refuses as a token carrying a value, and
    /// the rewritten form hands the parser everything
    /// behind the short letter as that value.
    #[test]
    fn walk_help_short_dash_continuation_refuses_at_the_root() {
        assert_walk_run(&["-h-x", "--help"], &["--help=-x", "--help"]);
    }

    /// FD-70, the help reads. A help token behind compare
    /// fires the help of compare, the path naming the
    /// command the walk stands on.
    #[test]
    fn walk_help_fires_behind_compare() {
        assert_walk_help(&["compare", "--help"], &["compare"]);
    }

    /// FD-70, the help reads. The short help token behind
    /// sweep fires the help of sweep.
    #[test]
    fn walk_help_short_fires_behind_sweep() {
        assert_walk_help(&["sweep", "-h"], &["sweep"]);
    }

    /// FD-70, the help reads. A help token behind reconcile
    /// before any form stands fires the help of reconcile
    /// itself, the mid level reading help against the
    /// reconcile table.
    #[test]
    fn walk_help_fires_behind_reconcile() {
        assert_walk_help(&["reconcile", "--help"], &["reconcile"]);
    }

    /// FD-70, the help reads. A help token behind the stage
    /// form fires the help of the form, the path naming
    /// both levels.
    #[test]
    fn walk_help_fires_behind_stage() {
        assert_walk_help(&["reconcile", "stage", "--help"], &["reconcile", "stage"]);
    }

    /// FD-70, the help reads. The short help token behind
    /// the handback form fires the help of the form.
    #[test]
    fn walk_help_short_fires_behind_handback() {
        assert_walk_help(&["reconcile", "handback", "-h"], &["reconcile", "handback"]);
    }

    /// FD-70, the help reads. A help token fires ahead of a
    /// value the walk has already gathered, as the probe
    /// correction to FD-11 records, so the gathered value
    /// never reaches a handoff.
    #[test]
    fn walk_help_fires_ahead_of_a_gathered_value() {
        assert_walk_help(&["compare", "a", "--help"], &["compare"]);
    }

    /// FD-70, the help reads. An unknown long at the root
    /// is only collected, so a help token after it still
    /// fires.
    #[test]
    fn walk_unknown_long_at_the_root_leaves_help_free_to_fire() {
        assert_walk_help(&["--bogus", "--help"], &[]);
    }

    /// FD-70, the slot recognition. A word no subcommand
    /// answers to blocks the walk at the root, and every
    /// token after it stands in the stream as it stood, the
    /// help token behind it blocked with the rest.
    #[test]
    fn walk_unknown_word_at_the_root_blocks_the_walk() {
        assert_walk_run(&["bogus", "--help"], &["bogus", "--help"]);
    }

    /// FD-70, the slot recognition. A mere prefix of a
    /// command name blocks the walk, because the slot match
    /// is exact and the abbreviation reading belongs to the
    /// flags and never to the command names.
    #[test]
    fn walk_subcommand_prefix_at_the_root_blocks_the_walk() {
        assert_walk_run(&["comp", "--help"], &["comp", "--help"]);
    }

    /// FD-70, the slot recognition. A form name at the root
    /// blocks the walk, because no form slot stands at the
    /// top level and the name reads as a positional
    /// candidate for the subcommand slot.
    #[test]
    fn walk_form_name_at_the_root_blocks_the_walk() {
        assert_walk_run(&["stage", "--help"], &["stage", "--help"]);
    }

    /// FD-70, the slot recognition. A word the form slot
    /// cannot take blocks the walk behind reconcile, on the
    /// root slot's own terms one level down.
    #[test]
    fn walk_unknown_word_behind_reconcile_blocks_the_walk() {
        assert_walk_run(
            &["reconcile", "bogus", "--help"],
            &["reconcile", "bogus", "--help"],
        );
    }

    /// FD-70, the slot recognition. The reconcile name
    /// repeated behind itself blocks the walk, the second
    /// standing as a positional candidate the form slot
    /// cannot take.
    #[test]
    fn walk_reconcile_repeated_behind_itself_blocks_the_walk() {
        assert_walk_run(
            &["reconcile", "reconcile", "--help"],
            &["reconcile", "reconcile", "--help"],
        );
    }

    /// FD-70, the slot recognition. An option token at the
    /// root is collected and the walk goes on, the command
    /// after it filling its slot and the value after that
    /// gathering behind the inserted separator.
    #[test]
    fn walk_option_at_the_root_is_collected_and_the_walk_goes_on() {
        assert_walk_run(
            &["--bogus", "compare", "a"],
            &["--bogus", "compare", "--", "a"],
        );
    }

    /// FD-70, the slot recognition. An option token behind
    /// reconcile is collected at the mid level, so a help
    /// token after it fires the help of reconcile.
    #[test]
    fn walk_option_behind_reconcile_leaves_help_free_to_fire() {
        assert_walk_help(&["reconcile", "--bogus", "--help"], &["reconcile"]);
    }

    /// FD-70, the classification reads. A token carrying a
    /// space reads as a positional value whatever its
    /// prefix, as the probe correction to FD-11 records, so
    /// behind compare it gathers as a value.
    #[test]
    fn walk_space_token_reads_as_a_value_behind_compare() {
        assert_walk_run(
            &["compare", "-- a.tape", "b"],
            &["compare", "--", "-- a.tape", "b"],
        );
    }

    /// FD-70, the classification reads. The same space
    /// token at the root reads as a positional candidate
    /// and blocks the walk, the help token behind it
    /// blocked with the rest.
    #[test]
    fn walk_space_token_at_the_root_blocks_the_walk() {
        assert_walk_run(&["-- a.tape", "--help"], &["-- a.tape", "--help"]);
    }

    /// FD-70, the classification reads. A dash token
    /// carrying a space reads as a value on the same rule,
    /// its dash opening no option reading.
    #[test]
    fn walk_dash_space_token_reads_as_a_value_behind_compare() {
        assert_walk_run(&["compare", "-x y", "b"], &["compare", "--", "-x y", "b"]);
    }

    /// FD-70, the classification reads. A token naming a
    /// negative number in the pattern reads as a value, as
    /// the probe correction to FD-11 records.
    #[test]
    fn walk_negative_number_reads_as_a_value_behind_compare() {
        assert_walk_run(&["compare", "-1", "b"], &["compare", "--", "-1", "b"]);
    }

    /// FD-70, the classification reads. A negative fraction
    /// with digits on both sides of the point reads as a
    /// value on the same pattern.
    #[test]
    fn walk_negative_fraction_reads_as_a_value_behind_compare() {
        assert_walk_run(&["compare", "-1.5", "b"], &["compare", "--", "-1.5", "b"]);
    }

    /// FD-70, the classification reads. A negative fraction
    /// with no digit before the point reads as a value,
    /// because the digits before the point may stand empty
    /// in the pattern while those after it may not.
    #[test]
    fn walk_negative_leading_dot_reads_as_a_value_behind_compare() {
        assert_walk_run(&["compare", "-.5", "b"], &["compare", "--", "-.5", "b"]);
    }

    /// FD-70, the classification reads. A negative number
    /// carrying one trailing newline reads as a value,
    /// because the pattern's end anchor forgives exactly
    /// one trailing newline.
    #[test]
    fn walk_negative_number_with_a_trailing_newline_reads_as_a_value() {
        assert_walk_run(&["compare", "-1\n", "b"], &["compare", "--", "-1\n", "b"]);
    }

    /// FD-70, the classification reads. A negative spelling
    /// ending at the point reads as an option and not as a
    /// value, because the fraction the pattern admits
    /// after digits and a point still needs a digit of its
    /// own when the point is read as the fraction's start,
    /// and the spelling lands in the collected stream
    /// ahead of the gathered value.
    #[test]
    fn walk_negative_trailing_dot_reads_as_an_option_behind_compare() {
        assert_walk_run(&["compare", "-1.", "b"], &["compare", "-1.", "--", "b"]);
    }

    /// FD-70, the classification reads. A negative spelling
    /// carrying an exponent reads as an option, because
    /// the pattern admits no exponent.
    #[test]
    fn walk_negative_exponent_spelling_reads_as_an_option_behind_compare() {
        assert_walk_run(&["compare", "-1e3", "b"], &["compare", "-1e3", "--", "b"]);
    }

    /// FD-70, the classification reads. A lone dash reads
    /// as a value, the one dash token the classification
    /// names positional outright.
    #[test]
    fn walk_lone_dash_reads_as_a_value_behind_compare() {
        assert_walk_run(&["compare", "-", "b"], &["compare", "--", "-", "b"]);
    }

    /// FD-70, the classification reads. A token whose long
    /// name is the two dashes alone, written with an equals
    /// sign, reads as an option, because the long arm asks
    /// for a name longer than the dashes before it matches
    /// anything, and the classification reads a long token
    /// without a space as an option.
    #[test]
    fn walk_double_dash_equals_token_reads_as_an_option_behind_compare() {
        assert_walk_run(&["compare", "--=x", "a"], &["compare", "--=x", "--", "a"]);
    }

    /// FD-70, the classification reads. The help family
    /// letter stands ahead of the space rule in the
    /// classification read, so a token opening with that
    /// letter never reads as a positional even when it
    /// carries a space, while a token opening with any
    /// other letter reads as one on the space rule. The
    /// pair brackets the precedence the walk cannot isolate
    /// through its own outcomes, because the help read
    /// answers such a token before the classification is
    /// consulted.
    #[test]
    fn positional_read_h_family_stands_ahead_of_the_space_rule() {
        assert!(!reads_as_positional("-h x"));
        assert!(reads_as_positional("-x y"));
    }

    /// FD-70, the classification reads. The decimal digits
    /// of another script read as digits of the negative
    /// number pattern, because the pattern's digit set is
    /// the full Unicode decimal set the oracle's text
    /// pattern carries.
    #[test]
    fn positional_read_unicode_decimal_digits_read_as_a_negative_number() {
        assert!(reads_as_positional("-\u{661}\u{662}"));
    }

    /// FD-70, the separator arms. A separator standing
    /// before any subcommand is read as the subcommand
    /// candidate itself, as the probe correction to FD-11
    /// records, and it blocks the walk, the help token
    /// behind it blocked with the rest.
    #[test]
    fn walk_separator_at_the_root_blocks_the_walk() {
        assert_walk_run(&["--", "--help"], &["--", "--help"]);
    }

    /// FD-70, the separator arms. A separator behind
    /// reconcile before any form stands blocks the walk on
    /// the same candidate reading one level down, and the
    /// form name behind it never routes.
    #[test]
    fn walk_separator_behind_reconcile_blocks_the_walk() {
        assert_walk_run(
            &["reconcile", "--", "stage", "a"],
            &["reconcile", "--", "stage", "a"],
        );
    }

    /// FD-70, the separator arms. A separator standing
    /// last at the root, its tail empty, blocks the walk
    /// and stands in the handed over stream. The gathered
    /// handoff inserts no separator of its own where no
    /// value stands, so a reading that handed the tokens
    /// after a separator over as values would drop this
    /// separator and answer the empty stream, and the two
    /// arms separate on the empty tail though they
    /// coincide on every tail that carries a token.
    #[test]
    fn walk_separator_at_the_root_with_an_empty_tail_blocks_the_walk() {
        assert_walk_run(&["--"], &["--"]);
    }

    /// FD-70, the separator arms. The same empty tail
    /// separation behind a collected option at the root,
    /// the separator standing after it and last.
    #[test]
    fn walk_separator_at_the_root_after_an_option_with_an_empty_tail_blocks() {
        assert_walk_run(&["--bogus", "--"], &["--bogus", "--"]);
    }

    /// FD-70, the separator arms. A separator standing
    /// last behind reconcile before any form stands, its
    /// tail empty, blocks the walk on the same separation
    /// one level down.
    #[test]
    fn walk_separator_behind_reconcile_with_an_empty_tail_blocks_the_walk() {
        assert_walk_run(&["reconcile", "--"], &["reconcile", "--"]);
    }

    /// FD-70, the separator arms. The same empty tail
    /// separation behind reconcile behind a collected
    /// option, the separator standing after it and last.
    #[test]
    fn walk_separator_behind_reconcile_after_an_option_with_an_empty_tail_blocks() {
        assert_walk_run(
            &["reconcile", "--bogus", "--"],
            &["reconcile", "--bogus", "--"],
        );
    }

    /// FD-70, the separator arms. A separator behind a
    /// command is consumed where it stands and hands the
    /// tokens after it over as values, so the value before
    /// it and the value after it gather together behind
    /// the one inserted separator of the handoff.
    #[test]
    fn walk_separator_behind_compare_hands_the_rest_over_as_values() {
        assert_walk_run(&["compare", "a", "--", "b"], &["compare", "--", "a", "b"]);
    }

    /// FD-70, the separator arms. A separator behind
    /// sweep hands the tokens after it over as values on
    /// the same arm as behind compare, and the value
    /// ahead of it separates the readings, because the
    /// blocked stream keeps the separator where it stands
    /// and the gathered stream moves it ahead of the
    /// values it joins.
    #[test]
    fn walk_separator_behind_sweep_hands_the_rest_over_as_values() {
        assert_walk_run(&["sweep", "a", "--", "b"], &["sweep", "--", "a", "b"]);
    }

    /// FD-70, the separator arms. A second separator behind
    /// a command is itself a value, as the probe
    /// correction to FD-11 records, and it stands in the
    /// gathered values behind the inserted separator. The
    /// value ahead of the first separator separates the
    /// readings, so the pin proves the second separator
    /// stands as a gathered value and not merely where it
    /// stood in the raw stream.
    #[test]
    fn walk_second_separator_behind_compare_is_itself_a_value() {
        assert_walk_run(&["compare", "a", "--", "--"], &["compare", "--", "a", "--"]);
    }

    /// FD-70, the gathered handoff. Values gather in order
    /// behind the one separator the handoff inserts, and a
    /// collected option stands ahead of that separator in
    /// the stream.
    #[test]
    fn walk_values_gather_behind_the_inserted_separator() {
        assert_walk_run(
            &["compare", "--bogus", "a", "b"],
            &["compare", "--bogus", "--", "a", "b"],
        );
    }

    /// FD-70, the gathered handoff. A stream with no
    /// gathered value carries no separator into the
    /// handoff, the insertion standing only where a value
    /// stands.
    #[test]
    fn walk_no_values_carry_no_separator_into_the_handoff() {
        assert_walk_run(&["compare", "--bogus"], &["compare", "--bogus"]);
    }

    /// FD-70, the gathered handoff. The empty stream runs
    /// as an empty handoff.
    #[test]
    fn walk_empty_stream_runs_as_an_empty_handoff() {
        assert_walk_run(&[], &[]);
    }

    /// FD-70, the gathered handoff. Values keep their order
    /// across an option collected between them, the option
    /// moving ahead of the inserted separator and the
    /// values standing in the order they were gathered.
    #[test]
    fn walk_values_keep_their_order_across_a_collected_option() {
        assert_walk_run(
            &["sweep", "a", "--bogus", "b"],
            &["sweep", "--bogus", "--", "a", "b"],
        );
    }

    /// FD-70, the reconcile routing. The stage form with
    /// its three values and no option collected on any
    /// level goes through the distribution, each value
    /// filling its own slot.
    #[test]
    fn walk_stage_form_distributes_its_three_values() {
        assert_walk_assigned(
            &["reconcile", "stage", "a", "b", "c"],
            "stage one a one b one c",
        );
    }

    /// FD-70, the reconcile routing. The handback form
    /// distributes its values with the first two filling
    /// the branch state slots and the rest standing in the
    /// bundle.
    #[test]
    fn walk_handback_form_distributes_its_values() {
        assert_walk_assigned(
            &["reconcile", "handback", "a", "b", "c", "d"],
            "handback one a one b [c, d]",
        );
    }

    /// FD-70, the reconcile routing. An option token
    /// collected at the root vetoes the distribution, so
    /// the same three values behind the form gather into
    /// the handoff instead, the veto standing at the gate
    /// because the distribution never sees the root token.
    #[test]
    fn walk_root_option_vetoes_the_distribution() {
        assert_walk_run(
            &["--bogus", "reconcile", "stage", "a", "b", "c"],
            &["--bogus", "reconcile", "stage", "--", "a", "b", "c"],
        );
    }

    /// FD-70, the reconcile routing. An option token
    /// collected behind reconcile vetoes the distribution
    /// on the same terms, the token standing ahead of the
    /// form token in the stream the distribution reads
    /// from.
    #[test]
    fn walk_reconcile_option_vetoes_the_distribution() {
        assert_walk_run(
            &["reconcile", "--bogus", "stage", "a", "b", "c"],
            &["reconcile", "--bogus", "stage", "--", "a", "b", "c"],
        );
    }

    /// FD-70, the reconcile routing. An option token
    /// collected behind the form keeps the gathered
    /// handoff. The gate's veto and the distribution's own
    /// plain argument head check both refuse this shape,
    /// the joint coverage the census of FD-69 records, and
    /// the gate mutation of FD-71 demonstrates the overlap
    /// by leaving this pin standing.
    #[test]
    fn walk_form_option_keeps_the_gathered_handoff() {
        assert_walk_run(
            &["reconcile", "stage", "a", "--bogus", "b", "c"],
            &["reconcile", "stage", "--bogus", "--", "a", "b", "c"],
        );
    }

    /// FD-70, the reconcile routing. A form asked for too
    /// few values keeps the gathered handoff, because the
    /// distribution's assignment must be complete and two
    /// values fill no three slots.
    #[test]
    fn walk_too_few_values_behind_a_form_keep_the_gathered_handoff() {
        assert_walk_run(
            &["reconcile", "stage", "a", "b"],
            &["reconcile", "stage", "--", "a", "b"],
        );
    }

    /// FD-70, the reconcile routing. A stage form asked for
    /// one value too many keeps the gathered handoff,
    /// because the stage assignment takes exactly three
    /// values and the fourth has no slot.
    #[test]
    fn walk_extra_value_behind_stage_keeps_the_gathered_handoff() {
        assert_walk_run(
            &["reconcile", "stage", "a", "b", "c", "d"],
            &["reconcile", "stage", "--", "a", "b", "c", "d"],
        );
    }

    /// FD-70, the distribution. A separator after the first
    /// value joins the share of that value and is deleted
    /// from it, so each value still fills its own slot.
    #[test]
    fn walk_distribution_separator_after_the_first_value() {
        assert_walk_assigned(
            &["reconcile", "stage", "a", "--", "b", "c"],
            "stage one a one b one c",
        );
    }

    /// FD-70, the distribution. A separator leading the
    /// values joins the first share and is deleted from
    /// it, so the assignment stands as it stands with no
    /// separator at all.
    #[test]
    fn walk_distribution_separator_leading() {
        assert_walk_assigned(
            &["reconcile", "stage", "--", "a", "b", "c"],
            "stage one a one b one c",
        );
    }

    /// FD-70, the distribution. A separator after the last
    /// value joins the share of that value and is deleted
    /// from it, on the same arm at the stream's end.
    #[test]
    fn walk_distribution_separator_after_the_last_value() {
        assert_walk_assigned(
            &["reconcile", "stage", "a", "b", "c", "--"],
            "stage one a one b one c",
        );
    }

    /// FD-70, the distribution. A second separator behind
    /// the first is itself a value of the share it lands
    /// in, and the deletion of that share's first separator
    /// takes the value with it, leaving the right branch
    /// state slot of a stage form holding a list.
    #[test]
    fn walk_distribution_second_separator_lists_the_right_state_in_a_stage() {
        assert_walk_assigned(
            &["reconcile", "stage", "a", "--", "--", "b"],
            "stage one a listed the right branch state one b",
        );
    }

    /// FD-70, the distribution. The same second separator
    /// landing in the last share leaves the divergence
    /// record slot of a stage form holding a list.
    #[test]
    fn walk_distribution_second_separator_lists_the_record_in_a_stage() {
        assert_walk_assigned(
            &["reconcile", "stage", "a", "b", "--", "--"],
            "stage one a one b listed the divergence record",
        );
    }

    /// FD-70, the distribution. The second separator leaves
    /// the right branch state slot of a handback form
    /// holding a list on the same arm, the bundle behind
    /// it standing whole.
    #[test]
    fn walk_distribution_second_separator_lists_the_right_state_in_a_handback() {
        assert_walk_assigned(
            &["reconcile", "handback", "a", "--", "--", "b", "c"],
            "handback one a listed the right branch state [b, c]",
        );
    }

    /// FD-70, the distribution. Two trailing separators
    /// leave the handback bundle empty, the first joining
    /// the share of the value before it and the second
    /// standing as the bundle share's only token and
    /// falling to that share's deletion.
    #[test]
    fn walk_distribution_trailing_separators_leave_the_bundle_empty() {
        assert_walk_assigned(
            &["reconcile", "handback", "a", "b", "--", "--"],
            "handback one a one b []",
        );
    }

    /// FD-70, the distribution. A separator inside the
    /// bundle's span joins the share of the branch state
    /// before it under the handback grouping, and the
    /// bundle behind it stands whole.
    #[test]
    fn walk_distribution_separator_inside_the_bundle_share() {
        assert_walk_assigned(
            &["reconcile", "handback", "a", "b", "--", "c", "d"],
            "handback one a one b [c, d]",
        );
    }

    /// FD-70, the distribution. Two leading separators
    /// leave one separator standing as the left value,
    /// because the deletion takes only the first separator
    /// of the share and the second stands as a value token
    /// the share received.
    #[test]
    fn walk_distribution_leading_separators_leave_one_separator_as_the_left_value() {
        assert_walk_assigned(
            &["reconcile", "handback", "--", "--", "a", "b", "c"],
            "handback one -- one a [b, c]",
        );
    }

    /// FD-70, the distribution. A token carrying a space
    /// fills a slot as a plain argument, the distribution
    /// reading it under its own space rule where the walk
    /// gathered it as a value.
    #[test]
    fn walk_distribution_space_token_fills_a_slot() {
        assert_walk_assigned(
            &["reconcile", "stage", "-x y", "a", "b"],
            "stage one -x y one a one b",
        );
    }

    /// FD-70, the distribution. A negative number fills a
    /// slot as a plain argument on the same terms, the
    /// distribution's plain argument read resting on the
    /// same negative number pattern as the walk's
    /// classification.
    #[test]
    fn walk_distribution_negative_number_fills_a_slot() {
        assert_walk_assigned(
            &["reconcile", "stage", "-1", "a", "b"],
            "stage one -1 one a one b",
        );
    }

    /// FD-70, the distribution. Two separators behind a
    /// stage form with values between them make the count
    /// one the stage assignment cannot take, so the whole
    /// stream keeps the gathered handoff, the separators
    /// standing among the gathered values.
    #[test]
    fn walk_two_separators_behind_a_stage_keep_the_gathered_handoff() {
        assert_walk_run(
            &["reconcile", "stage", "a", "--", "b", "--", "c"],
            &["reconcile", "stage", "--", "a", "b", "--", "c"],
        );
    }
}
