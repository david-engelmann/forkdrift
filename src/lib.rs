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
//! divergence only. Reconciling the pairs it catches is
//! crosscurrent's work, and forkdrift never reconciles.

pub mod artifacts;
pub mod divergence;
pub mod errors;
pub mod pytext;

pub use artifacts::{Artifact, Statement, parse_artifact};
pub use divergence::{Divergence, compare_artifacts, format_report, format_sweep, sweep_artifacts};
pub use errors::{ArtifactError, ForkdriftError, InterfaceMismatchError, SourceNameError};
