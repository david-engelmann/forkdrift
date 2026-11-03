//! The artifact reader and the worth comparison.
//!
//! An artifact is a recorded text that one run left behind
//! for one interface. The reader parses it into its interface
//! line, its term statements, its operation statements, its
//! label statements, and its excludes line, and refuses a
//! malformed artifact with the refusal texts of FD-2. The
//! worth comparison decides when two differently written
//! values state the same value, exactly, on the number's own
//! digits and exponent (FD-6 and FD-19).

use std::collections::{BTreeSet, HashMap};

use crate::errors::{ArtifactError, ForkdriftError};
use crate::pytext;

/// The refusal the worth comparison raises when an exponent
/// is written with more digits than the conversion the oracle
/// performs accepts. The oracle converts the exponent text to
/// an integer before it checks anything else about the
/// number, its runtime refuses a conversion past 4300 digits,
/// and the refusal escapes the comparison uncaught, so the
/// port carries it as its own error kind beside the refusal
/// family (the probe correction to FD-19).
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("the exponent is written with {digits} digits, past the 4300 digit conversion limit")]
pub struct WorthError {
    digits: usize,
}

/// One statement of an artifact, its name, its normalized
/// value, and the full line it stands on, which the report
/// quotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    /// The name the statement defines.
    pub name: String,
    /// The statement's value after normalization.
    pub value: String,
    /// The full statement line, stripped, as written.
    pub line: String,
}

/// A parsed artifact.
#[derive(Clone, Debug)]
pub struct Artifact {
    /// The source the artifact was read under, as the raw
    /// bytes it was given in.
    pub source: Vec<u8>,
    /// The interface the artifact names.
    pub interface: String,
    /// The interface line, as written.
    pub interface_line: String,
    /// The term statements by name.
    pub terms: HashMap<String, Statement>,
    /// The operation statements by name.
    pub operations: HashMap<String, Statement>,
    /// The label statements by name.
    pub labels: HashMap<String, Statement>,
    /// The names the excludes line excludes.
    pub excludes: BTreeSet<String>,
    /// The excludes line, as written, when the artifact
    /// carries one.
    pub excludes_line: Option<String>,
}

/// Render a source for a refusal line.
///
/// A report prints a source with its bytes unaltered, but a
/// refusal line is text the error stream carries, and the
/// oracle's error stream replaces every byte it cannot
/// decode with the escaped form for that byte (the probe
/// correction to FD-2). This rendering decodes the source as
/// UTF-8 in the pieces where it is valid and writes each
/// undecodable byte as a backslash, the letters u, d, and c,
/// and the byte's two lowercase hex digits.
pub fn escape_source(source: &[u8]) -> String {
    let mut out = String::new();
    let mut rest = source;
    while !rest.is_empty() {
        match std::str::from_utf8(rest) {
            Ok(valid) => {
                out.push_str(valid);
                break;
            }
            Err(error) => {
                let valid = error.valid_up_to();
                out.push_str(
                    std::str::from_utf8(&rest[..valid]).expect("the prefix up to the error"),
                );
                out.push_str(&format!("\\udc{:02x}", rest[valid]));
                rest = &rest[valid + 1..];
            }
        }
    }
    out
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// Whether a value matches the number grammar of FD-19, an
/// optional sign, digits with an optional fraction or a
/// fraction standing alone, and an optional exponent, over
/// the ASCII digits alone.
pub fn is_number(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
        index += 1;
    }
    let whole_start = index;
    while matches!(bytes.get(index), Some(b) if b.is_ascii_digit()) {
        index += 1;
    }
    let whole = index - whole_start;
    let mut fraction = 0usize;
    if matches!(bytes.get(index), Some(b'.')) {
        index += 1;
        let fraction_start = index;
        while matches!(bytes.get(index), Some(b) if b.is_ascii_digit()) {
            index += 1;
        }
        fraction = index - fraction_start;
    }
    if whole == 0 && fraction == 0 {
        return false;
    }
    if matches!(bytes.get(index), Some(b'e') | Some(b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
            index += 1;
        }
        let exponent_start = index;
        while matches!(bytes.get(index), Some(b) if b.is_ascii_digit()) {
            index += 1;
        }
        if index == exponent_start {
            return false;
        }
    }
    index == bytes.len()
}

/// A signed exponent held at arbitrary precision, a sign and
/// a magnitude digit string with no leading zeros, the empty
/// magnitude standing for zero. The worth key adjusts the
/// exponent by the small counts the mantissa contributes, so
/// the arithmetic here is addition and subtraction of a small
/// count against a digit string of any length.
#[derive(Clone, Debug, PartialEq, Eq)]
struct BigExponent {
    negative: bool,
    digits: String,
}

impl BigExponent {
    fn zero() -> Self {
        BigExponent {
            negative: false,
            digits: String::new(),
        }
    }

    fn parse(text: &str) -> Result<Self, WorthError> {
        let (negative, digits) = match text.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, text.strip_prefix('+').unwrap_or(text)),
        };
        if digits.len() > 4300 {
            return Err(WorthError {
                digits: digits.len(),
            });
        }
        let trimmed = digits.trim_start_matches('0');
        if trimmed.is_empty() {
            return Ok(BigExponent::zero());
        }
        Ok(BigExponent {
            negative,
            digits: trimmed.to_string(),
        })
    }

    fn add_small(&mut self, delta: i64) {
        if delta == 0 {
            return;
        }
        let count = delta.unsigned_abs().to_string();
        if self.digits.is_empty() {
            self.negative = delta < 0;
            self.digits = count;
            return;
        }
        if (delta < 0) == self.negative {
            self.digits = add_digit_strings(&self.digits, &count);
        } else {
            match compare_digit_strings(&self.digits, &count) {
                std::cmp::Ordering::Equal => {
                    self.digits.clear();
                    self.negative = false;
                }
                std::cmp::Ordering::Greater => {
                    self.digits = subtract_digit_strings(&self.digits, &count);
                }
                std::cmp::Ordering::Less => {
                    self.digits = subtract_digit_strings(&count, &self.digits);
                    self.negative = delta < 0;
                }
            }
        }
        if self.digits.is_empty() {
            self.negative = false;
        }
    }
}

fn compare_digit_strings(left: &str, right: &str) -> std::cmp::Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn add_digit_strings(left: &str, right: &str) -> String {
    let mut out: Vec<u8> = Vec::new();
    let mut carry = 0u8;
    let mut a = left.as_bytes().iter().rev();
    let mut b = right.as_bytes().iter().rev();
    loop {
        let next_a = a.next();
        let next_b = b.next();
        if next_a.is_none() && next_b.is_none() && carry == 0 {
            break;
        }
        let sum = next_a.map_or(0, |d| d - b'0') + next_b.map_or(0, |d| d - b'0') + carry;
        out.push(b'0' + (sum % 10));
        carry = sum / 10;
    }
    out.reverse();
    String::from_utf8(out).expect("digit bytes are text")
}

/// Subtract the smaller digit string from the larger, both
/// without leading zeros, and trim the result.
fn subtract_digit_strings(left: &str, right: &str) -> String {
    let mut out: Vec<u8> = Vec::new();
    let mut borrow: i16 = 0;
    let mut a = left.as_bytes().iter().rev();
    let mut b = right.as_bytes().iter().rev();
    loop {
        let next_a = a.next();
        let next_b = b.next();
        if next_a.is_none() && next_b.is_none() {
            break;
        }
        let mut digit = next_a.map_or(0, |d| (d - b'0') as i16) - borrow;
        let sub = next_b.map_or(0, |d| (d - b'0') as i16);
        if digit < sub {
            digit += 10;
            borrow = 1;
        } else {
            borrow = 0;
        }
        out.push(b'0' + (digit - sub) as u8);
    }
    while out.len() > 1 && out[out.len() - 1] == b'0' {
        out.pop();
    }
    if out.iter().all(|d| *d == b'0') {
        return String::new();
    }
    out.reverse();
    String::from_utf8(out).expect("digit bytes are text")
}

/// The exact worth key of a number, the sign and the
/// significant digit string and the exponent the digits
/// stand at, so two spellings of one number hold equal keys
/// and two different numbers never do (FD-19). A zero holds
/// the zero key whatever its sign and exponent.
#[derive(Clone, Debug, PartialEq, Eq)]
struct WorthKey {
    zero: bool,
    negative: bool,
    digits: String,
    exponent: BigExponent,
}

fn worth_key(text: &str) -> Result<WorthKey, WorthError> {
    let negative = text.starts_with('-');
    let body = text.trim_start_matches(['+', '-']);
    let (mantissa, exponent_text) = match body.find(['e', 'E']) {
        Some(at) => (&body[..at], Some(&body[at + 1..])),
        None => (body, None),
    };
    let mut exponent = match exponent_text {
        Some(part) => BigExponent::parse(part)?,
        None => BigExponent::zero(),
    };
    let (whole, fraction) = match mantissa.split_once('.') {
        Some((whole, fraction)) => (whole, fraction),
        None => (mantissa, ""),
    };
    let mut digits = String::with_capacity(whole.len() + fraction.len());
    digits.push_str(whole);
    digits.push_str(fraction);
    exponent.add_small(-(fraction.len() as i64));
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Ok(WorthKey {
            zero: true,
            negative: false,
            digits: String::new(),
            exponent: BigExponent::zero(),
        });
    }
    let trimmed = digits.trim_end_matches('0');
    exponent.add_small((digits.len() - trimmed.len()) as i64);
    Ok(WorthKey {
        zero: false,
        negative,
        digits: trimmed.to_string(),
        exponent,
    })
}

/// Whether two normalized values state the same value. Two
/// identical texts agree. Two numbers agree when their exact
/// worth keys agree. Anything else compares by its
/// characters (FD-6 and FD-19).
pub fn values_equal(left: &str, right: &str) -> Result<bool, WorthError> {
    if left == right {
        return Ok(true);
    }
    if is_number(left) && is_number(right) {
        return Ok(worth_key(left)? == worth_key(right)?);
    }
    Ok(false)
}

/// Collapse the runs of whitespace in a value to single
/// spaces, over the oracle's whitespace set.
fn normalize_value(value: &str) -> String {
    pytext::split(value).join(" ")
}

fn split_statement(
    text: &str,
    source: &str,
    line_number: usize,
) -> Result<(String, String), ForkdriftError> {
    if !text.contains('=') {
        return Err(ArtifactError::new(format!(
            "{source} is malformed, line {line_number} matches no statement form"
        ))
        .into());
    }
    let (name_part, value_part) = text.split_once('=').expect("an equals sign stands");
    let name = pytext::strip(name_part);
    let value = pytext::strip(value_part);
    if !is_identifier(name) {
        return Err(ArtifactError::new(format!(
            "{source} is malformed, line {line_number} names no valid identifier, {name}"
        ))
        .into());
    }
    if value.is_empty() {
        return Err(ArtifactError::new(format!(
            "{source} is malformed, line {line_number} states an empty value for {name}"
        ))
        .into());
    }
    Ok((name.to_string(), value.to_string()))
}

fn store(
    table: &mut HashMap<String, Statement>,
    kind: &str,
    name: String,
    value: &str,
    line: &str,
    source: &str,
    line_number: usize,
) -> Result<(), ForkdriftError> {
    if table.contains_key(&name) {
        return Err(ArtifactError::new(format!(
            "{source} is malformed, duplicate {kind} name {name} at line {line_number}"
        ))
        .into());
    }
    table.insert(
        name.clone(),
        Statement {
            name,
            value: normalize_value(value),
            line: line.to_string(),
        },
    );
    Ok(())
}

fn parse_excludes(
    raw_value: &str,
    source: &str,
    line_number: usize,
) -> Result<BTreeSet<String>, ForkdriftError> {
    let mut names = BTreeSet::new();
    for part in raw_value.split(',') {
        let name = pytext::strip(part);
        if !is_identifier(name) {
            return Err(ArtifactError::new(format!(
                "{source} is malformed, line {line_number} excludes line names no valid identifier"
            ))
            .into());
        }
        names.insert(name.to_string());
    }
    Ok(names)
}

/// Read a recorded text into an artifact, or refuse it.
///
/// The source is the name the artifact stands under in
/// reports and refusals, carried as the raw bytes it was
/// given in, so a source that is not valid UTF-8 reaches the
/// report with its bytes unaltered and the refusal line with
/// its bytes escaped (the probe correction to FD-2).
pub fn parse_artifact(text: &str, source: &[u8]) -> Result<Artifact, ForkdriftError> {
    let named = escape_source(source);
    let mut meaningful: Vec<(usize, &str)> = Vec::new();
    for (index, raw) in pytext::splitlines(text).into_iter().enumerate() {
        let stripped = pytext::strip(raw);
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }
        meaningful.push((index + 1, stripped));
    }
    let Some(&(first_number, first)) = meaningful.first() else {
        return Err(ArtifactError::new(format!(
            "{named} is malformed, missing interface line, the artifact holds no meaningful line"
        ))
        .into());
    };
    let head = pytext::split(first);
    if !(head.len() == 2 && head[0] == "interface" && is_identifier(head[1])) {
        return Err(ArtifactError::new(format!(
            "{named} is malformed, missing interface line, line {first_number} does not name an interface"
        ))
        .into());
    }
    let mut artifact = Artifact {
        source: source.to_vec(),
        interface: head[1].to_string(),
        interface_line: first.to_string(),
        terms: HashMap::new(),
        operations: HashMap::new(),
        labels: HashMap::new(),
        excludes: BTreeSet::new(),
        excludes_line: None,
    };
    for &(line_number, line) in &meaningful[1..] {
        if let Some(rest) = line.strip_prefix("label ") {
            let (name, value) = split_statement(rest, &named, line_number)?;
            store(
                &mut artifact.labels,
                "label",
                name,
                &value,
                line,
                &named,
                line_number,
            )?;
        } else if let Some(rest) = line.strip_prefix("operation ") {
            let (name, value) = split_statement(rest, &named, line_number)?;
            store(
                &mut artifact.operations,
                "operation",
                name,
                &value,
                line,
                &named,
                line_number,
            )?;
        } else if line.contains('=')
            && pytext::strip(line.split('=').next().expect("an equals sign stands")) == "excludes"
        {
            if artifact.excludes_line.is_some() {
                return Err(ArtifactError::new(format!(
                    "{named} is malformed, more than one excludes line, second one at line {line_number}"
                ))
                .into());
            }
            artifact.excludes = parse_excludes(
                line.split_once('=').expect("an equals sign stands").1,
                &named,
                line_number,
            )?;
            artifact.excludes_line = Some(line.to_string());
        } else if line.contains('=') {
            let (name, value) = split_statement(line, &named, line_number)?;
            store(
                &mut artifact.terms,
                "term",
                name,
                &value,
                line,
                &named,
                line_number,
            )?;
        } else {
            return Err(ArtifactError::new(format!(
                "{named} is malformed, line {line_number} matches no statement form, {line}"
            ))
            .into());
        }
    }
    for name in &artifact.excludes {
        if artifact.terms.contains_key(name) || artifact.operations.contains_key(name) {
            return Err(ArtifactError::new(format!(
                "{named} is malformed, it defines {name} and also excludes it"
            ))
            .into());
        }
    }
    Ok(artifact)
}
