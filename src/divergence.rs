//! The comparison and the two documents it prints.
//!
//! The comparison runs on two recorded artifacts for one
//! interface and raises divergences of four kinds, term
//! value, result type, excluded but defined, and presence,
//! each quoting the lines that disagree (FD-3 to FD-18). The
//! sweep compares one baseline against several others and
//! prints one document that attributes every divergence to
//! its pair (FD-20 to FD-23). Forkdrift detects divergence
//! and nothing here reconciles it.

use std::collections::{HashMap, HashSet};

use crate::artifacts::{Artifact, Statement, WorthError, escape_source, values_equal};
use crate::errors::{ForkdriftError, InterfaceMismatchError};

/// The kind of a divergence over a term's value.
pub const TERM_VALUE: &str = "term-value";
/// The kind of a divergence over an operation's result type.
pub const RESULT_TYPE: &str = "result-type";
/// The kind of a divergence where one side excludes a name
/// the other side defines.
pub const EXCLUDED_BUT_DEFINED: &str = "excluded-but-defined";
/// The kind of a divergence over a name only one side
/// defines.
pub const PRESENCE: &str = "presence";

/// One divergence, the interface and name it stands over and
/// the lines each side states, with the further line of
/// FD-12 when a side defines the name both ways.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Divergence {
    /// The interface both artifacts name.
    pub interface: String,
    /// The name that diverged.
    pub name: String,
    /// The kind of the divergence.
    pub kind: String,
    /// The source of the left artifact, as raw bytes.
    pub left_source: Vec<u8>,
    /// The line the left side states.
    pub left_line: String,
    /// The source of the right artifact, as raw bytes.
    pub right_source: Vec<u8>,
    /// The line the right side states.
    pub right_line: String,
    /// The source and line of the further row, when a side
    /// defines the name both ways.
    pub extra: Option<(Vec<u8>, String)>,
}

/// The failure of a comparison, either the interface refusal
/// of the refusal family or the worth conversion refusal
/// that escapes uncaught (the probe correction to FD-19).
#[derive(Debug, thiserror::Error)]
pub enum CompareError {
    /// The interface mismatch refusal.
    #[error("{0}")]
    Refusal(#[from] ForkdriftError),
    /// The worth conversion refusal.
    #[error("{0}")]
    Worth(#[from] WorthError),
}

fn one_sided_lines<'a>(
    defining: &'a HashMap<String, Statement>,
    other: &'a HashMap<String, Statement>,
    name: &str,
) -> Vec<&'a Statement> {
    let mut lines = Vec::new();
    if let Some(statement) = defining.get(name)
        && !other.contains_key(name)
    {
        lines.push(statement);
    }
    lines
}

fn exclusion_divergences(
    excluding: &Artifact,
    defining: &Artifact,
    excluding_is_left: bool,
    found: &mut Vec<Divergence>,
) {
    let Some(excludes_line) = excluding.excludes_line.as_deref() else {
        return;
    };
    for name in &excluding.excludes {
        let mut defining_lines: Vec<&Statement> = Vec::new();
        if let Some(statement) = defining.terms.get(name) {
            defining_lines.push(statement);
        }
        if let Some(statement) = defining.operations.get(name) {
            defining_lines.push(statement);
        }
        if defining_lines.is_empty() {
            continue;
        }
        let (left_source, left_line, right_source, right_line);
        if excluding_is_left {
            left_source = &excluding.source;
            left_line = excludes_line;
            right_source = &defining.source;
            right_line = defining_lines[0].line.as_str();
        } else {
            left_source = &defining.source;
            left_line = defining_lines[0].line.as_str();
            right_source = &excluding.source;
            right_line = excludes_line;
        }
        let extra = if defining_lines.len() > 1 {
            Some((defining.source.clone(), defining_lines[1].line.clone()))
        } else {
            None
        };
        found.push(Divergence {
            interface: excluding.interface.clone(),
            name: name.clone(),
            kind: EXCLUDED_BUT_DEFINED.to_string(),
            left_source: left_source.clone(),
            left_line: left_line.to_string(),
            right_source: right_source.clone(),
            right_line: right_line.to_string(),
            extra,
        });
    }
}

fn presence_divergences(left: &Artifact, right: &Artifact, found: &mut Vec<Divergence>) {
    // The names collect once each in the order they first
    // stand, with a set carrying the membership question so
    // a wide artifact costs a pass and not a square.
    let mut seen: HashSet<&str> = HashSet::new();
    let mut names: Vec<&str> = Vec::new();
    for name in left
        .terms
        .keys()
        .chain(left.operations.keys())
        .chain(right.terms.keys())
        .chain(right.operations.keys())
    {
        if seen.insert(name.as_str()) {
            names.push(name.as_str());
        }
    }
    names.sort_unstable();
    for name in names {
        if left.excludes.contains(name) || right.excludes.contains(name) {
            continue;
        }
        let mut left_lines = one_sided_lines(&left.terms, &right.terms, name);
        left_lines.extend(one_sided_lines(&left.operations, &right.operations, name));
        let mut right_lines = one_sided_lines(&right.terms, &left.terms, name);
        right_lines.extend(one_sided_lines(&right.operations, &left.operations, name));
        if left_lines.is_empty() && right_lines.is_empty() {
            continue;
        }
        let mut divergence = Divergence {
            interface: left.interface.clone(),
            name: name.to_string(),
            kind: PRESENCE.to_string(),
            left_source: left.source.clone(),
            left_line: String::new(),
            right_source: right.source.clone(),
            right_line: String::new(),
            extra: None,
        };
        if !left_lines.is_empty() && !right_lines.is_empty() {
            divergence.left_line = left_lines[0].line.clone();
            divergence.right_line = right_lines[0].line.clone();
        } else if !left_lines.is_empty() {
            divergence.left_line = left_lines[0].line.clone();
            divergence.right_line = right.interface_line.clone();
            if left_lines.len() > 1 {
                divergence.extra = Some((left.source.clone(), left_lines[1].line.clone()));
            }
        } else {
            divergence.left_line = left.interface_line.clone();
            divergence.right_line = right_lines[0].line.clone();
            if right_lines.len() > 1 {
                divergence.extra = Some((right.source.clone(), right_lines[1].line.clone()));
            }
        }
        found.push(divergence);
    }
}

/// Compare two artifacts for one interface and return the
/// divergences between them, sorted for the report (FD-10).
pub fn compare_artifacts(
    left: &Artifact,
    right: &Artifact,
) -> Result<Vec<Divergence>, CompareError> {
    if left.interface != right.interface {
        return Err(CompareError::Refusal(
            InterfaceMismatchError::new(format!(
                "artifacts name different interfaces, {} names {} and {} names {}",
                escape_source(&left.source),
                left.interface,
                escape_source(&right.source),
                right.interface
            ))
            .into(),
        ));
    }
    let mut found: Vec<Divergence> = Vec::new();
    for (kind, pick_terms) in [(TERM_VALUE, true), (RESULT_TYPE, false)] {
        let (left_table, right_table) = if pick_terms {
            (&left.terms, &right.terms)
        } else {
            (&left.operations, &right.operations)
        };
        let mut shared: Vec<&String> = left_table
            .keys()
            .filter(|name| right_table.contains_key(*name))
            .collect();
        shared.sort_unstable();
        for name in shared {
            let left_statement = &left_table[name];
            let right_statement = &right_table[name];
            if !values_equal(&left_statement.value, &right_statement.value)? {
                found.push(Divergence {
                    interface: left.interface.clone(),
                    name: name.clone(),
                    kind: kind.to_string(),
                    left_source: left.source.clone(),
                    left_line: left_statement.line.clone(),
                    right_source: right.source.clone(),
                    right_line: right_statement.line.clone(),
                    extra: None,
                });
            }
        }
    }
    exclusion_divergences(left, right, true, &mut found);
    exclusion_divergences(right, left, false, &mut found);
    presence_divergences(left, right, &mut found);
    found.sort_by(|a, b| {
        a.interface
            .cmp(&b.interface)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    Ok(found)
}

/// Render one divergence block as bytes. The sources splice
/// in as their raw bytes, so a source that is not valid
/// UTF-8 stands in the document unaltered (the probe
/// correction to FD-2).
fn render_block(divergence: &Divergence, out: &mut Vec<u8>) {
    out.extend_from_slice(
        format!(
            "divergence {} {} {}\n  left (",
            divergence.interface, divergence.name, divergence.kind
        )
        .as_bytes(),
    );
    out.extend_from_slice(&divergence.left_source);
    out.extend_from_slice(format!(") {}\n  right (", divergence.left_line).as_bytes());
    out.extend_from_slice(&divergence.right_source);
    out.extend_from_slice(format!(") {}", divergence.right_line).as_bytes());
    if let Some((extra_source, extra_line)) = &divergence.extra {
        let side = if *extra_source == divergence.left_source {
            "left"
        } else {
            "right"
        };
        out.extend_from_slice(format!("\n  {side} (").as_bytes());
        out.extend_from_slice(extra_source);
        out.extend_from_slice(format!(") {extra_line}").as_bytes());
    }
}

/// Print a comparison as a pairwise report (FD-2). An empty
/// comparison prints the empty document.
pub fn format_report(divergences: &[Divergence]) -> Vec<u8> {
    let mut out = Vec::new();
    for divergence in divergences {
        render_block(divergence, &mut out);
        out.push(b'\n');
    }
    out
}

/// Compare one baseline artifact against each of several
/// others in turn, returning one divergence list per other
/// artifact in the order given (FD-20). A pair that refuses
/// refuses the whole sweep.
pub fn sweep_artifacts(
    baseline: &Artifact,
    others: &[Artifact],
) -> Result<Vec<Vec<Divergence>>, CompareError> {
    let mut results = Vec::new();
    for other in others {
        results.push(compare_artifacts(baseline, other)?);
    }
    Ok(results)
}

/// Print a sweep as one document, a header line per pair and
/// then that pair's report (FD-21).
pub fn format_sweep(
    baseline: &Artifact,
    others: &[Artifact],
    results: &[Vec<Divergence>],
) -> Vec<u8> {
    let mut out = Vec::new();
    for (other, divergences) in others.iter().zip(results.iter()) {
        out.extend_from_slice(b"pair left (");
        out.extend_from_slice(&baseline.source);
        out.extend_from_slice(b") right (");
        out.extend_from_slice(&other.source);
        out.extend_from_slice(b")\n");
        out.extend_from_slice(&format_report(divergences));
    }
    out
}
