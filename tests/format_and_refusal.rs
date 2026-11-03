//! The format and refusal pins, ported test for test from
//! the Python corpus file of the same name.

mod common;

use common::{artifact, artifact_err, compare_err};
use forkdrift::divergence::CompareError;
use forkdrift::{ForkdriftError, parse_artifact};

const VALID: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 30\noperation fetch = record\nlabel primary = node-a\nexcludes = legacy-mode, retired-flag\n";

#[test]
fn valid_artifact_parses_every_statement_form() {
    let art = artifact(VALID, "run-a");
    assert_eq!(art.interface, "shard-store");
    assert_eq!(art.terms["replicas"].value, "3");
    assert_eq!(art.terms["timeout"].value, "30");
    assert_eq!(art.operations["fetch"].value, "record");
    assert_eq!(art.labels["primary"].value, "node-a");
    let expected: std::collections::BTreeSet<String> = ["legacy-mode", "retired-flag"]
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(art.excludes, expected);
    assert_eq!(
        art.excludes_line.as_deref(),
        Some("excludes = legacy-mode, retired-flag")
    );
}

#[test]
fn missing_interface_line_refused_by_name() {
    let error = artifact_err("replicas = 3\n", "run-a");
    assert!(matches!(error, ForkdriftError::Artifact(_)));
    let text = error.to_string();
    assert!(text.contains("run-a"), "{text}");
    assert!(text.contains("interface"), "{text}");
}

#[test]
fn empty_artifact_refused_as_missing_interface() {
    let error = artifact_err("# only a comment\n\n", "run-a");
    assert!(error.to_string().contains("interface"));
}

#[test]
fn line_without_equals_sign_refused_by_name() {
    let error = artifact_err("interface shard-store\n\nreplicas 3\n", "run-a");
    let text = error.to_string();
    assert!(text.contains("run-a"), "{text}");
    assert!(text.contains("statement form"), "{text}");
}

#[test]
fn duplicate_term_refused_by_name() {
    let text = "interface shard-store\n\ntimeout = 30\ntimeout = 45\n";
    let error = artifact_err(text, "run-b");
    let message = error.to_string();
    assert!(message.contains("duplicate"), "{message}");
    assert!(message.contains("timeout"), "{message}");
}

#[test]
fn duplicate_operation_refused_by_name() {
    let text = "interface shard-store\n\noperation fetch = record\noperation fetch = receipt\n";
    let error = artifact_err(text, "run-b");
    let message = error.to_string();
    assert!(message.contains("duplicate"), "{message}");
    assert!(message.contains("fetch"), "{message}");
}

#[test]
fn duplicate_label_refused_by_name() {
    let text = "interface shard-store\n\nlabel main = node-a\nlabel main = node-b\n";
    let error = artifact_err(text, "run-b");
    let message = error.to_string();
    assert!(message.contains("duplicate"), "{message}");
    assert!(message.contains("main"), "{message}");
}

#[test]
fn empty_value_refused_by_name() {
    let text = "interface shard-store\n\ntimeout =\n";
    let error = artifact_err(text, "run-b");
    let message = error.to_string();
    assert!(message.contains("empty"), "{message}");
    assert!(message.contains("timeout"), "{message}");
}

#[test]
fn second_excludes_line_refused() {
    let text = "interface shard-store\n\nexcludes = a-mode\nexcludes = b-mode\n";
    assert!(parse_artifact(text, b"run-b").is_err());
}

#[test]
fn artifact_that_defines_what_it_excludes_refused() {
    let text = "interface shard-store\n\nlegacy-mode = on\nexcludes = legacy-mode\n";
    let error = artifact_err(text, "run-b");
    let message = error.to_string();
    assert!(message.contains("legacy-mode"), "{message}");
    assert!(message.contains("excludes"), "{message}");
}

#[test]
fn different_interfaces_refused_by_name() {
    let a = artifact("interface shard-store\n\nreplicas = 3\n", "run-a");
    let b = artifact("interface shard-cache\n\nreplicas = 3\n", "run-b");
    let error = compare_err(&a, &b);
    assert!(matches!(
        error,
        CompareError::Refusal(ForkdriftError::InterfaceMismatch(_))
    ));
    let text = error.to_string();
    assert!(text.contains("shard-store"), "{text}");
    assert!(text.contains("shard-cache"), "{text}");
}
