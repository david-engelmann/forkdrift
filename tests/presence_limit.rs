//! FD-16 pins, the interplay of presence and the exclusion
//! rules, ported test for test from the Python corpus file
//! of the same name.

mod common;

use common::{artifact, compare};

const DEFINING: &str = "interface shard-store\n\nreplicas = 3\ntimeout = 30\n";
const DUAL_DEFINING: &str =
    "interface shard-store\n\nreplicas = 3\ntimeout = 30\noperation timeout = ack\n";
const EXCLUDING: &str = "interface shard-store\n\nreplicas = 3\nexcludes = timeout\n";
const BARE: &str = "interface shard-store\n\nreplicas = 3\n";

#[test]
fn excluded_one_sided_name_raises_only_the_exclusion_contradiction() {
    let left = artifact(DEFINING, "run-a");
    let right = artifact(EXCLUDING, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "timeout");
    assert_eq!(found[0].kind, "excluded-but-defined");
}

#[test]
fn exclusion_contradiction_fires_in_the_other_direction_too() {
    let left = artifact(EXCLUDING, "run-a");
    let right = artifact(DEFINING, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "timeout");
    assert_eq!(found[0].kind, "excluded-but-defined");
}

#[test]
fn excluded_name_defined_both_ways_still_raises_one_divergence() {
    let left = artifact(DUAL_DEFINING, "run-a");
    let right = artifact(EXCLUDING, "run-b");
    let found = compare(&left, &right);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "timeout");
    assert_eq!(found[0].kind, "excluded-but-defined");
}

#[test]
fn excluded_name_defined_by_neither_side_raises_nothing() {
    let left = artifact(EXCLUDING, "run-a");
    let right = artifact(BARE, "run-b");
    assert_eq!(compare(&left, &right), Vec::new());
}

#[test]
fn name_excluded_by_both_sides_raises_nothing() {
    let left = artifact(EXCLUDING, "run-a");
    let right = artifact(EXCLUDING, "run-b");
    assert_eq!(compare(&left, &right), Vec::new());
}
