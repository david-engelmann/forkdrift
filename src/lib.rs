//! Tells whether two recorded artifacts that state the same interface actually diverge. Both files are read, compared structurally, and reported without executing either.
//!
//! Two runs of one system leave recorded artifacts behind,
//! and the artifacts can disagree in what they state for the
//! interface they share, in a term's value, in an operation's
//! result type, in a name one side defines and the other
//! excludes, or in a name only one side defines at all.
//! Forkdrift reads two recorded artifacts, or one baseline
//! against several others, and reports every such divergence
//! with the lines that disagree quoted. It detects semantic
//! divergence only. The pairs it catches are reconciled by
//! the reconcile subcommand, which cross-applies two
//! recorded branch states over the records the detector
//! prints and hands the divergences back as sized
//! reconciliation tasks, and the detector itself reconciles
//! nothing.

pub mod artifacts;
pub mod divergence;
pub mod errors;
pub mod pytext;
pub mod reconcile;

pub use artifacts::{Artifact, Statement, parse_artifact};
pub use divergence::{Divergence, compare_artifacts, format_report, format_sweep, sweep_artifacts};
pub use errors::{ArtifactError, ForkdriftError, InterfaceMismatchError, SourceNameError};
