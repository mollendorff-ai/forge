//! No skipping (the maintainer, 2026-11-16).
//!
//! 100% coverage, pedantic linting and formatting, with no exceptions. A lint switched off by an attribute
//! or in a Cargo.toml, a lint group a crate leaves out, code compiled only
//! for (or only without) the coverage run, an ignored test and a formatting
//! skip are each a way round a rule, and each is counted here. So are the
//! two ways tests left the code they test: a file in a `tests/` folder, and a
//! test module in a file of its own (tests are inline, in the module they
//! test; a file over 1,500 lines is split into modules, not emptied of its
//! tests).
//!
//! The counts may only fall, to zero (roadmap T1-NO-SKIPPING). This test
//! fails when a count rises, so no new way round a rule gets in, and when
//! it falls, so the budget below is lowered in the same change that removes
//! one. Raising a budget is not allowed: fix the code instead.

use crate::{root, rust_files, without_comment};
use std::collections::BTreeMap;

/// What is left of each kind on 2026-11-16, when the rule was made. Lower a
/// number when a change removes some; never raise one.
pub const BUDGET: [(&str, usize); 8] = [
    ("lint allow or expect attributes", 37),
    ("lints allowed in a Cargo.toml", 13),
    ("lint groups a crate does not enable", 2),
    ("code compiled only for or only without coverage", 0),
    ("ignored tests", 0),
    ("formatting skips", 0),
    ("test files in a tests folder", 0),
    ("test modules in a file of their own", 0),
];

/// The crates: their manifest, and the folders of their Rust code
pub const CRATES: [(&str, &[&str]); 5] = [
    ("Cargo.toml", &["src"]),
    ("forge-wasm/Cargo.toml", &["forge-wasm"]),
    ("validation/gnumeric/Cargo.toml", &["validation/gnumeric"]),
    ("validation/r/Cargo.toml", &["validation/r"]),
    ("validation/repo/Cargo.toml", &["validation/repo"]),
];

/// The lint groups every crate turns on, as the main crate does
pub const GROUPS: [&str; 3] = ["pedantic", "nursery", "cargo"];

/// Each Rust file of every crate, with its comments and spaces removed, so
/// that an attribute written with spaces inside it counts as well
///
/// # Panics
///
/// Panics if a file cannot be read.
#[must_use]
pub fn compact_sources() -> Vec<(String, String)> {
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
                .lines()
                .map(without_comment)
                .flat_map(str::split_whitespace)
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
#[must_use]
pub fn joined(parts: &[&str]) -> String {
    parts.concat()
}

/// `allow(` or `expect(` as an attribute, alone or inside a `cfg_attr`
#[must_use]
pub fn lint_attributes(text: &str) -> usize {
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

/// Code compiled only for, or only without, the coverage run.
///
/// `coverage` named in any `cfg(...)`, `cfg_attr(...)` or `cfg!(...)`
/// (`all(not(coverage), ...)` and `any(not(coverage), test)` too, not only
/// `cfg(coverage)`), and the `coverage(off)` attribute.
#[must_use]
pub fn coverage_only(text: &str) -> usize {
    let conditions = ["cfg(", "cfg_attr(", "cfg!("]
        .iter()
        .flat_map(|opening| text.match_indices(opening).map(|(start, _)| start))
        .filter(|start| {
            let rest = &text[*start..];
            let condition = &rest[..rest.find([']', ';']).unwrap_or(rest.len())];
            names_coverage(condition)
        })
        .count();
    conditions
        + text
            .matches(joined(&["coverage(", "off)"]).as_str())
            .count()
}

/// Whether `coverage` appears as a name in `condition`, not inside a longer one
/// (`coverage_report`, `no_coverage`)
fn names_coverage(condition: &str) -> bool {
    condition.match_indices("coverage").any(|(at, word)| {
        let before = condition[..at].chars().next_back();
        let after = condition[at + word.len()..].chars().next();
        let part_of_name = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        !part_of_name(before) && !part_of_name(after)
    })
}

#[must_use]
pub fn ignored_tests(text: &str) -> usize {
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

#[must_use]
pub fn formatting_skips(text: &str) -> usize {
    text.matches(joined(&["rustfmt::", "skip"]).as_str())
        .count()
}

/// A `mod name;` with no body, under a `cfg` that names `test`: a test
/// module kept in a file of its own
#[must_use]
pub fn test_modules_elsewhere(text: &str) -> usize {
    let cfg = joined(&["#[", "cfg("]);
    text.match_indices(cfg.as_str())
        .filter(|(start, _)| {
            let attribute = &text[*start..];
            let end = attribute.find(']').unwrap_or(attribute.len());
            let names_test = ["(test)", "(test,", ",test)", ",test,"]
                .iter()
                .any(|form| attribute[..end].contains(form));
            let mut rest = attribute.get(end + 1..).unwrap_or_default();
            while rest.starts_with("#[") {
                rest = rest
                    .get(rest.find(']').map_or(rest.len(), |e| e + 1)..)
                    .unwrap_or_default();
            }
            for visibility in ["pub(crate)", "pub(super)", "pub"] {
                if let Some(after) = rest.strip_prefix(visibility) {
                    rest = after;
                    break;
                }
            }
            names_test
                && rest.strip_prefix("mod").is_some_and(|after| {
                    let name_end = after
                        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .unwrap_or(after.len());
                    name_end > 0 && after[name_end..].starts_with(';')
                })
        })
        .count()
}

/// The Rust files in each crate's `tests` folder
#[must_use]
pub fn test_folder_files() -> Vec<String> {
    let mut files = Vec::new();
    for (manifest, _) in CRATES {
        let folder = root()
            .join(manifest)
            .parent()
            .map(|crate_root| crate_root.join("tests"))
            .unwrap_or_default();
        if folder.is_dir() {
            rust_files(&folder, &mut files);
        }
    }
    files.sort();
    files
        .into_iter()
        .map(|path| {
            path.strip_prefix(root())
                .unwrap_or(&path)
                .display()
                .to_string()
        })
        .collect()
}

/// The `[lints.clippy]` table of a manifest: the lints set to allow, and the
/// lint groups it does not turn on
///
/// # Panics
///
/// Panics if the manifest cannot be read or has no `[lints.clippy]` table.
#[must_use]
pub fn manifest_lints(manifest: &str) -> (Vec<String>, Vec<&'static str>) {
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
///
/// # Panics
///
/// Panics if a file cannot be read.
#[must_use]
pub fn found() -> BTreeMap<&'static str, (usize, Vec<String>)> {
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
        add(BUDGET[5].0, formatting_skips(&text), file.clone());
        add(BUDGET[7].0, test_modules_elsewhere(&text), file);
    }
    // Counted even when there are none, so the kind is known to be scanned
    let files = test_folder_files();
    add(BUDGET[6].0, files.len(), files.join(", "));
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

#[cfg(test)]
mod tests {
    use super::*;

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
                 {count} in validation/repo/src/no_skipping.rs, in this change"
                ));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n\n"));
    }

    #[test]
    fn coverage_only_code_is_counted_in_every_form() {
        // "COV" stands for the word, which this file must not spell next to a cfg
        let count = |text: &str| coverage_only(&text.replace("COV", &joined(&["cover", "age"])));
        for condition in [
            "COV",
            "not(COV)",
            "all(not(COV),feature=\"cli\")",
            "all(feature=\"cli\",any(not(COV),test))",
        ] {
            let attribute = format!("#[{}{condition})]fnf(){{}}", joined(&["cf", "g("]));
            assert_eq!(count(&attribute), 1, "{condition}");
        }
        assert_eq!(count("#![cfg_attr(COV,derive(Debug))]"), 1);
        assert_eq!(count("if!cfg!(COV){run();}"), 1);
        assert_eq!(count("#[COV(off)]fnf(){}"), 1);
        // Two conditions, two counts; a longer name, or no condition, none
        assert_eq!(count("#[cfg(COV)]fna(){}#[cfg(not(COV))]fnb(){}"), 2);
        assert_eq!(count("#[cfg(feature=\"COV_report\")]fna(){}"), 0);
        assert_eq!(count("#[cfg(no_COV)]fna(){}"), 0);
        assert_eq!(count("#[cfg(unix)]fna(){}"), 0);
        assert_eq!(count("letCOV=1;"), 0);
    }

    #[test]
    fn every_kind_is_counted() {
        // A kind that the scan never reports would pass at any budget
        let found = found();
        for (kind, _) in BUDGET {
            assert!(found.contains_key(kind), "{kind} is not counted");
        }
    }
}
