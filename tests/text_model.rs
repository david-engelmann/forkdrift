//! FD-28 pins for the reader's text model. The character
//! tables stand here a second time, written from the probe
//! output, and the implementation's tables are checked
//! against them over every code point, so deleting any
//! entry from either table in the implementation breaks a
//! pin. The behaviour pins then run each boundary character
//! and each whitespace range edge through the reader
//! itself.

mod common;

use common::artifact_err;
use forkdrift::parse_artifact;
use forkdrift::pytext::{is_decimal, is_space};

const EXPECTED_WHITESPACE: &[(u32, u32)] = &[
    (0x0009, 0x000D),
    (0x001C, 0x0020),
    (0x0085, 0x0085),
    (0x00A0, 0x00A0),
    (0x1680, 0x1680),
    (0x2000, 0x200A),
    (0x2028, 0x2029),
    (0x202F, 0x202F),
    (0x205F, 0x205F),
    (0x3000, 0x3000),
];

const EXPECTED_DECIMAL: &[(u32, u32)] = &[
    (0x0030, 0x0039),
    (0x0660, 0x0669),
    (0x06F0, 0x06F9),
    (0x07C0, 0x07C9),
    (0x0966, 0x096F),
    (0x09E6, 0x09EF),
    (0x0A66, 0x0A6F),
    (0x0AE6, 0x0AEF),
    (0x0B66, 0x0B6F),
    (0x0BE6, 0x0BEF),
    (0x0C66, 0x0C6F),
    (0x0CE6, 0x0CEF),
    (0x0D66, 0x0D6F),
    (0x0DE6, 0x0DEF),
    (0x0E50, 0x0E59),
    (0x0ED0, 0x0ED9),
    (0x0F20, 0x0F29),
    (0x1040, 0x1049),
    (0x1090, 0x1099),
    (0x17E0, 0x17E9),
    (0x1810, 0x1819),
    (0x1946, 0x194F),
    (0x19D0, 0x19D9),
    (0x1A80, 0x1A89),
    (0x1A90, 0x1A99),
    (0x1B50, 0x1B59),
    (0x1BB0, 0x1BB9),
    (0x1C40, 0x1C49),
    (0x1C50, 0x1C59),
    (0xA620, 0xA629),
    (0xA8D0, 0xA8D9),
    (0xA900, 0xA909),
    (0xA9D0, 0xA9D9),
    (0xA9F0, 0xA9F9),
    (0xAA50, 0xAA59),
    (0xABF0, 0xABF9),
    (0xFF10, 0xFF19),
    (0x104A0, 0x104A9),
    (0x10D30, 0x10D39),
    (0x11066, 0x1106F),
    (0x110F0, 0x110F9),
    (0x11136, 0x1113F),
    (0x111D0, 0x111D9),
    (0x112F0, 0x112F9),
    (0x11450, 0x11459),
    (0x114D0, 0x114D9),
    (0x11650, 0x11659),
    (0x116C0, 0x116C9),
    (0x11730, 0x11739),
    (0x118E0, 0x118E9),
    (0x11950, 0x11959),
    (0x11C50, 0x11C59),
    (0x11D50, 0x11D59),
    (0x11DA0, 0x11DA9),
    (0x11F50, 0x11F59),
    (0x16A60, 0x16A69),
    (0x16AC0, 0x16AC9),
    (0x16B50, 0x16B59),
    (0x1D7CE, 0x1D7FF),
    (0x1E140, 0x1E149),
    (0x1E2F0, 0x1E2F9),
    (0x1E4F0, 0x1E4F9),
    (0x1E950, 0x1E959),
    (0x1FBF0, 0x1FBF9),
];

fn in_ranges(ranges: &[(u32, u32)], point: u32) -> bool {
    ranges
        .iter()
        .any(|(low, high)| point >= *low && point <= *high)
}

#[test]
fn whitespace_table_matches_the_probed_set_over_every_code_point() {
    for point in 0..=0x10FFFFu32 {
        let Some(c) = char::from_u32(point) else {
            continue;
        };
        assert_eq!(
            is_space(c),
            in_ranges(EXPECTED_WHITESPACE, point),
            "whitespace membership differs at U+{point:04X}"
        );
    }
}

#[test]
fn decimal_table_matches_the_probed_set_over_every_code_point() {
    for point in 0..=0x10FFFFu32 {
        let Some(c) = char::from_u32(point) else {
            continue;
        };
        assert_eq!(
            is_decimal(c),
            in_ranges(EXPECTED_DECIMAL, point),
            "decimal membership differs at U+{point:04X}"
        );
    }
}

const BOUNDARIES: [char; 10] = [
    '\n', '\r', '\u{b}', '\u{c}', '\u{1c}', '\u{1d}', '\u{1e}', '\u{85}', '\u{2028}', '\u{2029}',
];

#[test]
fn every_boundary_character_splits_lines_and_counts_line_numbers() {
    for boundary in BOUNDARIES {
        let text = format!("interface s{boundary}timeout = 30{boundary}");
        let parsed = parse_artifact(&text, b"probe")
            .unwrap_or_else(|e| panic!("boundary U+{:04X} splits, {e}", boundary as u32));
        assert_eq!(parsed.terms["timeout"].value, "30");
        let refused = format!("interface s{boundary}timeout = 30{boundary}bogus line{boundary}");
        let error = artifact_err(&refused, "probe");
        assert!(
            error.to_string().contains("line 3"),
            "boundary U+{:04X} counts lines, {error}",
            boundary as u32
        );
    }
}

#[test]
fn carriage_return_line_feed_pair_counts_as_one_boundary() {
    let error = artifact_err("interface s\r\nbogus line\r\n", "probe");
    assert!(
        error.to_string().contains("line 2"),
        "the pair is one boundary, {error}"
    );
    let parsed = parse_artifact("interface s\r\ntimeout = 30\r\n", b"probe").expect("parses");
    assert_eq!(parsed.terms["timeout"].value, "30");
}

#[test]
fn file_separator_without_boundary_status_does_not_split() {
    // U+001F is whitespace in the oracle's set but not a
    // line boundary, so the line stays whole and the
    // interface line refuses.
    let error = artifact_err("interface s\u{1f}timeout = 30\n", "probe");
    assert!(
        error.to_string().contains("does not name an interface"),
        "{error}"
    );
}

#[test]
fn every_whitespace_range_edge_strips_and_collapses() {
    let mut edges: Vec<char> = Vec::new();
    for (low, high) in EXPECTED_WHITESPACE {
        for point in [*low, *high] {
            edges.push(char::from_u32(point).expect("edge is a char"));
        }
    }
    for edge in edges {
        // Padded at the ends of the statement line. A
        // boundary character among the edges divides the
        // text there instead of padding it, and the
        // statement between the two divisions is the same
        // statement either way.
        let padded = format!("interface s\n\n{edge}timeout = 30{edge}\n");
        let parsed = parse_artifact(&padded, b"probe")
            .unwrap_or_else(|e| panic!("whitespace U+{:04X} strips, {e}", edge as u32));
        assert_eq!(parsed.terms["timeout"].value, "30");
        if !BOUNDARIES.contains(&edge) {
            let joined = format!("interface s\n\nnote = a{edge}b\n");
            let parsed = parse_artifact(&joined, b"probe").expect("parses");
            assert_eq!(
                parsed.terms["note"].value, "a b",
                "whitespace U+{:04X} collapses",
                edge as u32
            );
        }
        let blank = format!("interface s\n{edge}\ntimeout = 30\n");
        let parsed = parse_artifact(&blank, b"probe").expect("parses");
        assert_eq!(parsed.terms["timeout"].value, "30");
    }
}

#[test]
fn zero_width_space_is_not_whitespace() {
    let parsed = parse_artifact("interface s\n\nnote = a\u{200b}b\n", b"probe").expect("parses");
    assert_eq!(parsed.terms["note"].value, "a\u{200b}b");
}

#[test]
fn non_breaking_space_inside_a_value_collapses_to_a_space() {
    let left = parse_artifact("interface s\n\nnote = a\u{a0}b\n", b"L").expect("parses");
    let right = parse_artifact("interface s\n\nnote = a b\n", b"R").expect("parses");
    assert_eq!(common::compare(&left, &right), Vec::new());
}
