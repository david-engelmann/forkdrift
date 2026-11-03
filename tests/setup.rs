//! The setup invariants in the crate's own form (FD-26),
//! ported from the Python corpus file of the same name. The
//! dash scan runs over the bytes of every file the walk
//! reaches rather than a suffix list, which is the stronger
//! form the port states, and it asserts the walk reached the
//! records it exists to cover so it cannot pass vacuously.

mod common;

use common::root;
use sha2::{Digest, Sha256};

const CANONICAL_PREFIX_SHA256: &str =
    "63d2f04005dc8eb111d3efca1be55c0911f9ad46bc7cf3c74d9f9908f4a86f11";

const EXPECTED_OBLIGATIONS: [&str; 5] = [
    "FD-S1. Two runs that define the same interface in incompatible ways raise an alarm, and an alarm that does not quote both conflicting definitions is a defect the tests must catch.",
    "FD-S2. Two runs whose definitions of a shared interface agree raise no alarm, and a false alarm on agreeing runs is a defect the tests must catch.",
    "FD-S3. Every alarm names the concept that diverged and the runs that diverged on it, and an alarm missing either is rejected rather than reported incomplete.",
    "FD-S4. Forkdrift detects and reports only, and it never edits a run, a branch, or a definition in order to reconcile a divergence it found.",
    "FD-S5. Every fixture and example in this repo is synthetic and built for this repo, and no real trace or session data appears anywhere in the tree.",
];

fn read_repo(rel: &str) -> String {
    std::fs::read_to_string(root().join(rel)).expect("repo file reads")
}

#[test]
fn contract_stub_obligations_exist() {
    let text = read_repo("CONTRACT.md");
    let mut found: Vec<(String, u32, &str)> = Vec::new();
    for line in text.lines() {
        let bytes = line.as_bytes();
        if bytes.len() > 6
            && bytes[0].is_ascii_uppercase()
            && bytes[1].is_ascii_uppercase()
            && bytes[2] == b'-'
            && bytes[3] == b'S'
            && bytes[4].is_ascii_digit()
            && bytes[5] == b'.'
        {
            found.push((line[..2].to_string(), u32::from(bytes[4] - b'0'), line));
        }
    }
    assert!(
        found.len() >= 5,
        "expected at least 5 stub obligations, found {}",
        found.len()
    );
    let texts: Vec<&str> = found.iter().map(|(_, _, line)| *line).collect();
    assert_eq!(
        texts, EXPECTED_OBLIGATIONS,
        "stub obligation text has changed without a contract amendment"
    );
    let mut numbers: Vec<u32> = found.iter().map(|(_, number, _)| *number).collect();
    numbers.sort_unstable();
    assert_eq!(numbers, [1, 2, 3, 4, 5]);
    for (prefix, _, line) in &found {
        assert_eq!(prefix, "FD", "obligation is not an FD obligation");
        let words = forkdrift::pytext::split(line).len();
        assert!(words >= 20, "obligation too thin at {words} words");
    }
}

#[test]
fn agents_canonical_block_present() {
    let text = read_repo("AGENTS.md");
    assert!(text.contains("repo annex below this line"));
    let at = text.find("## Repo annex").expect("annex heading stands");
    let prefix = &text[..at];
    let digest = Sha256::digest(prefix.as_bytes());
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    assert_eq!(
        hex, CANONICAL_PREFIX_SHA256,
        "canonical block above the annex line has drifted"
    );
    let annex = &text[at..];
    assert!(annex.contains("CONTRACT.md"));
}

#[test]
fn readme_status_reports_rounds() {
    let text = read_repo("README.md");
    let mut counts: Vec<String> = Vec::new();
    let mut search = text.as_str();
    while let Some(at) = search.find("Closed rounds ") {
        let rest = &search[at + "Closed rounds ".len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        let after = &rest[digits.len()..];
        if !digits.is_empty()
            && after.starts_with(" of 100")
            && !after[" of 100".len()..].starts_with(|c: char| c.is_ascii_digit())
        {
            counts.push(digits);
        }
        search = &search[at + 1..];
    }
    assert_eq!(
        counts,
        ["4"],
        "README status must report exactly one closed round count"
    );
}

const ANY_DEPTH_SKIP: [&str; 6] = [
    ".git",
    "target",
    ".pytest_cache",
    "__pycache__",
    ".venv",
    "venv",
];
const ROOT_SKIP: [&str; 2] = ["node_modules", ".tox"];

fn collect_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("directory readable") {
        let path = entry.expect("entry readable").path();
        let name = path
            .file_name()
            .expect("name stands")
            .to_string_lossy()
            .into_owned();
        if ANY_DEPTH_SKIP.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

#[test]
fn no_em_or_en_dash_in_text_files() {
    let base = root();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&base).expect("root readable") {
        let path = entry.expect("entry readable").path();
        let name = path
            .file_name()
            .expect("name stands")
            .to_string_lossy()
            .into_owned();
        if ROOT_SKIP.contains(&name.as_str()) || ANY_DEPTH_SKIP.contains(&name.as_str()) {
            continue;
        }
        if path.is_dir() {
            collect_files(&path, &mut files);
        } else if path.is_file() {
            files.push(path);
        }
    }
    let anchors = [
        "CONTRACT.md",
        "README.md",
        "AGENTS.md",
        "Cargo.toml",
        "Cargo.lock",
        "src/lib.rs",
        "examples/divergences.txt",
        "tests/cli_corpus.json",
    ];
    let rel_names: Vec<String> = files
        .iter()
        .map(|path| {
            path.strip_prefix(&base)
                .expect("under root")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    for anchor in anchors {
        assert!(
            rel_names.iter().any(|name| name == anchor),
            "the scan never reached {anchor}"
        );
    }
    let em_dash: [u8; 3] = [0xE2, 0x80, 0x94];
    let en_dash: [u8; 3] = [0xE2, 0x80, 0x93];
    let mut bad = Vec::new();
    for path in &files {
        let bytes = std::fs::read(path).expect("file readable");
        if bytes.windows(3).any(|w| w == em_dash || w == en_dash) {
            bad.push(
                path.strip_prefix(&base)
                    .expect("under root")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    assert!(bad.is_empty(), "em or en dash found in {bad:?}");
}
