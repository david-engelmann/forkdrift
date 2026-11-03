//! The textual difference pins, ported test for test from
//! the Python corpus file of the same name.

mod common;

use common::{artifact, compare};

const BASE: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 30\noperation fetch = list of records\nlabel primary = node-a\n";
const REORDERED: &str = "interface shard-store\n\nlabel primary = node-a\noperation fetch = list of records\ntimeout = 30\nreplicas = 3\n";
const WHITESPACE: &str = "interface shard-store\n\nreplicas   =   3\ntimeout=30\noperation fetch = list  of   records\nlabel primary = node-a\n";
const NUMERIC_FORMAT: &str = "interface shard-store\n\nreplicas = 3.00\ntimeout = 30.0\noperation fetch = list of records\nlabel primary = node-a\n";
const LABEL_RENAMED: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 30\noperation fetch = list of records\nlabel main = node-a\n";
const LABEL_REVALUED: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 30\noperation fetch = list of records\nlabel primary = node-b\n";
const NUMERIC_WORTH_DIFFERS: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 31\noperation fetch = list of records\nlabel primary = node-a\n";
const WORD_DIFFERS: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 30\noperation fetch = list of record\nlabel primary = node-a\n";

fn assert_agree(variant: &str) {
    let left = artifact(BASE, "run-a");
    let right = artifact(variant, "run-b");
    assert_eq!(compare(&left, &right), Vec::new());
}

#[test]
fn reordered_lines_raise_no_alarm() {
    assert_agree(REORDERED);
}

#[test]
fn whitespace_differences_raise_no_alarm() {
    assert_agree(WHITESPACE);
}

#[test]
fn numeric_formatting_raises_no_alarm() {
    assert_agree(NUMERIC_FORMAT);
}

#[test]
fn renamed_label_raises_no_alarm() {
    assert_agree(LABEL_RENAMED);
}

#[test]
fn revalued_label_raises_no_alarm() {
    assert_agree(LABEL_REVALUED);
}

#[test]
fn numeric_worth_difference_still_alarms() {
    let left = artifact(BASE, "run-a");
    let right = artifact(NUMERIC_WORTH_DIFFERS, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "timeout");
    assert_eq!(found[0].kind, "term-value");
}

#[test]
fn one_word_difference_in_a_type_still_alarms() {
    let left = artifact(BASE, "run-a");
    let right = artifact(WORD_DIFFERS, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "fetch");
    assert_eq!(found[0].kind, "result-type");
}
