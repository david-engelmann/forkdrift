//! The reconcile surface of the crate, the session work
//! the fold carries into forkdrift under FD-54.
//!
//! Detection hands the subcommand two recorded branch
//! states and the divergence records the detector printed
//! over one pair of sources. The staging cross-applies the
//! two states over the records, and the handback hands
//! every divergence back to the two authors as one sized
//! reconciliation task. The subcommand reconciles only the
//! pairs it is handed. Detecting divergence across the
//! fleet is the detector's work, and the subcommand takes
//! over none of it.

pub mod errors;
pub mod pynumber;
pub mod records;
pub mod staging;
pub mod states;
pub mod tasks;

pub use errors::{CrosscurrentError, InterfaceMismatchError, RecordError, StateError, TaskError};
pub use records::{Divergence, QuotedLine, bundle_divergences, parse_divergence_record};
pub use staging::{CrossApply, cross_apply, format_stage_report};
pub use states::{BranchState, Statement, parse_branch_state, values_equal};
pub use tasks::{Task, build_tasks, build_tasks_bundle, format_tasks};
