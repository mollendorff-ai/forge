//! No skipping (the maintainer, 2026-11-16): 100% coverage, pedantic linting
//! and formatting, with no exceptions. A lint switched off by an attribute
//! or in a Cargo.toml, a lint group a crate leaves out, code compiled only
//! for (or only without) the coverage run, an ignored test and a formatting
//! skip are each a way round a rule, and each is counted here.
//!
//! The counts may only fall, to zero (roadmap T1-NO-SKIPPING). This test
//! fails when a count rises, so no new way round a rule gets in, and when
//! it falls, so the budget below is lowered in the same change that removes
//! one. Raising a budget is not allowed: fix the code instead.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// What is left of each kind on 2026-11-16, when the rule was made. Lower a
/// number when a change removes some; never raise one.
const BUDGET: [(&str, usize); 6] = [
    ("lint allow or expect attributes", 219),
    ("lints allowed in a Cargo.toml", 16),
    ("lint groups a crate does not enable", 2),
    ("code compiled only for or only without coverage", 31),
    ("ignored tests", 2),
    ("formatting skips", 0),
];

/// The crates: their manifest, and the folders of their Rust code
const CRATES: [(&str, &[&str]); 4] = [
    ("Cargo.toml", &["src", "tests"]),
    ("forge-wasm/Cargo.toml", &["forge-wasm"]),
    ("validation/gnumeric/Cargo.toml", &["validation/gnumeric"]),
    ("validation/r/Cargo.toml", &["validation/r"]),
];

/// The lint groups every crate turns on, as the main crate does
const GROUPS: [&str; 3] = ["pedantic", "nursery", "cargo"];

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap_or_default();
        if path.is_dir() {
            if name != "target" && name != "node_modules" {
                rust_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Each Rust file of every crate, with its text and spaces removed, so that
/// an attribute written with spaces inside it counts as well
fn compact_sources() -> Vec<(String, String)> {
    let mut files = Vec::new();
    for (_, dirs) in CRATES {
        for dir in dirs {
            rust_files(&root().join(dir), &mut files);
        }
    }
    files.sort();
    files.dedup();
    files
        .into_iter()
        .map(|path| {
            let text: String = std::fs::read_to_string(&path)
                .unwrap()
                .split_whitespace()
                .collect();
            let name = path
                .strip_prefix(root())
                .unwrap_or(&path)
                .display()
                .to_string();
            (name, text)
        })
        .collect()
}

/// The patterns are joined at run time, so this file does not count itself
fn joined(parts: &[&str]) -> String {
    parts.concat()
}

/// `allow(` or `expect(` as an attribute, alone or inside a `cfg_attr`
fn lint_attributes(text: &str) -> usize {
    let plain: usize = ["allow(", "expect("]
        .iter()
        .flat_map(|lint| [joined(&["#[", lint]), joined(&["#![", lint])])
        .map(|pattern| text.matches(pattern.as_str()).count())
        .sum();
    let conditional = joined(&["cfg_", "attr("]);
    let inside: usize = text
        .match_indices(conditional.as_str())
        .map(|(start, _)| {
            let attribute = &text[start..];
            let attribute = &attribute[..attribute.find(']').unwrap_or(attribute.len())];
            [",allow(", ",expect("]
                .iter()
                .map(|lint| attribute.matches(lint).count())
                .sum::<usize>()
        })
        .sum();
    plain + inside
}

fn coverage_only(text: &str) -> usize {
    [
        joined(&["cfg(not(", "coverage))"]),
        joined(&["cfg(", "coverage)"]),
        joined(&["cfg_attr(", "coverage"]),
        joined(&["coverage(", "off)"]),
    ]
    .iter()
    .map(|pattern| text.matches(pattern.as_str()).count())
    .sum()
}

fn ignored_tests(text: &str) -> usize {
    let plain = text.matches(joined(&["#[", "ignore"]).as_str()).count();
    let conditional = joined(&["cfg_", "attr("]);
    let inside = text
        .match_indices(conditional.as_str())
        .filter(|(start, _)| {
            let attribute = &text[*start..];
            attribute[..attribute.find(']').unwrap_or(attribute.len())].contains(",ignore")
        })
        .count();
    plain + inside
}

fn formatting_skips(text: &str) -> usize {
    text.matches(joined(&["rustfmt::", "skip"]).as_str())
        .count()
}

/// The `[lints.clippy]` table of a manifest: the lints set to allow, and the
/// lint groups it does not turn on
fn manifest_lints(manifest: &str) -> (Vec<String>, Vec<&'static str>) {
    let text = std::fs::read_to_string(root().join(manifest)).unwrap();
    let start = text
        .find("[lints.clippy]")
        .unwrap_or_else(|| panic!("{manifest} has no [lints.clippy] table"));
    let table = &text[start + "[lints.clippy]".len()..];
    let table = &table[..table.find("\n[").unwrap_or(table.len())];
    let settings: Vec<(&str, &str)> = table
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.trim(), value.trim()))
        .collect();
    let allowed = settings
        .iter()
        .filter(|(_, value)| value.contains("\"allow\""))
        .map(|(name, _)| format!("{manifest}: {name}"))
        .collect();
    let missing = GROUPS
        .into_iter()
        .filter(|group| {
            !settings
                .iter()
                .any(|(name, value)| name == group && value.contains("\"warn\""))
        })
        .collect();
    (allowed, missing)
}

/// The count of each kind, and where each was found
fn found() -> BTreeMap<&'static str, (usize, Vec<String>)> {
    let mut found = BTreeMap::new();
    let mut add = |kind: &'static str, count: usize, place: String| {
        let entry: &mut (usize, Vec<String>) = found.entry(kind).or_default();
        if count > 0 {
            entry.0 += count;
            entry.1.push(format!("{place} ({count})"));
        }
    };
    for (file, text) in compact_sources() {
        add(BUDGET[0].0, lint_attributes(&text), file.clone());
        add(BUDGET[3].0, coverage_only(&text), file.clone());
        add(BUDGET[4].0, ignored_tests(&text), file.clone());
        add(BUDGET[5].0, formatting_skips(&text), file);
    }
    for (manifest, _) in CRATES {
        let (allowed, missing) = manifest_lints(manifest);
        for lint in allowed {
            add(BUDGET[1].0, 1, lint);
        }
        for group in missing {
            add(BUDGET[2].0, 1, format!("{manifest}: {group}"));
        }
    }
    let rustfmt = std::fs::read_to_string(root().join("rustfmt.toml")).unwrap();
    for key in ["ignore", "disable_all_formatting"] {
        let set = rustfmt.lines().any(|line| {
            line.split_once('=')
                .is_some_and(|(name, _)| name.trim() == key)
        });
        add(
            BUDGET[5].0,
            usize::from(set),
            format!("rustfmt.toml: {key}"),
        );
    }
    found
}

#[test]
fn the_ways_round_a_rule_only_ever_get_fewer() {
    let found = found();
    let mut wrong = Vec::new();
    for (kind, budget) in BUDGET {
        let (count, places) = found.get(kind).cloned().unwrap_or_default();
        if count > budget {
            wrong.push(format!(
                "{kind}: {count}, more than the {budget} left. Fix the code rather than \
                 switch the rule off; the budget is never raised. Where: {places:#?}"
            ));
        } else if count < budget {
            wrong.push(format!(
                "{kind}: {count}, fewer than the {budget} in BUDGET. Good: lower it to \
                 {count} in tests/no_skipping.rs, in this change"
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n\n"));
}

#[test]
fn every_kind_is_counted() {
    // A kind that the scan never reports would pass at any budget
    let found = found();
    for (kind, _) in BUDGET {
        assert!(found.contains_key(kind), "{kind} is not counted");
    }
}
