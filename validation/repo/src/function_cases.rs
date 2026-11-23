//! Every function Forge lists has a case that checks it against another engine
//! (ADR-045, as it was built).
//!
//! `forge functions --json` is the list. A function is checked when a Gnumeric
//! case calls it (`validation/gnumeric/tests`, which Gnumeric or `LibreOffice`
//! recalculates), or when the R suite builds a formula with it
//! (`validation/r/src/main.rs`: the Monte Carlo distributions, drawn by
//! `forge simulate` and compared with R). A function in [`EXEMPT`] is checked
//! by neither, with the reason; the list may only shrink, so a function whose
//! case is added must leave it.

use std::path::{Path, PathBuf};

/// Functions no other engine checks, each with why. A function here that a
/// case now calls fails the test, so the entry is removed with it.
pub const EXEMPT: [(&str, &str); 1] = [(
    "MC.Discrete",
    "a draw from a list of values with their weights: no spreadsheet or R validator draws it here; \
     its unit tests and the examples run it",
)];

/// Every function `forge functions --json` lists.
///
/// # Panics
///
/// Panics if the command fails or its JSON is not the function catalog.
#[must_use]
pub fn listed() -> Vec<String> {
    let catalog: serde_json::Value =
        serde_json::from_str(&crate::forge(&["functions", "--json"]).printed())
            .expect("the catalog is JSON");
    catalog["categories"]
        .as_array()
        .expect("categories")
        .iter()
        .flat_map(|category| category["functions"].as_array().expect("functions"))
        .map(|function| function["name"].as_str().expect("a name").to_string())
        .collect()
}

/// The YAML files under `dir`, at any depth.
///
/// # Panics
///
/// Panics if `dir` or a folder in it cannot be read.
pub fn yaml_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("a folder of cases reads") {
        let path = entry.expect("a folder entry").path();
        if path.is_dir() {
            yaml_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "yaml") {
            out.push(path);
        }
    }
}

/// Whether `text` calls `name` (`NAME(`, case ignored), not a longer name that
/// ends with it (`FORECAST(` is not `FORECAST.LINEAR(`, `T.DIST(` is not
/// `NORM.S.DIST(`).
#[must_use]
pub fn calls(text: &str, name: &str) -> bool {
    let text = text.to_uppercase();
    let call = format!("{}(", name.to_uppercase());
    text.match_indices(&call).any(|(at, _)| {
        text[..at].chars().next_back().is_none_or(|before| {
            !(before.is_ascii_alphanumeric() || before == '_' || before == '.')
        })
    })
}

/// The functions of `names` that `cases` does not call and `exempt` does not
/// list, and the exempt ones that `cases` now calls.
#[must_use]
pub fn gaps(names: &[String], cases: &str, exempt: &[(&str, &str)]) -> (Vec<String>, Vec<String>) {
    let is_exempt = |name: &str| exempt.iter().any(|(listed, _)| *listed == name);
    let unchecked = names
        .iter()
        .filter(|name| !calls(cases, name) && !is_exempt(name))
        .cloned()
        .collect();
    let stale = exempt
        .iter()
        .filter(|(name, _)| calls(cases, name))
        .map(|(name, _)| (*name).to_string())
        .collect();
    (unchecked, stale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read, root};

    /// What the cases say: the Gnumeric cases and the formulas the R suite builds.
    fn cases() -> String {
        let mut files = Vec::new();
        yaml_files(&root().join("validation/gnumeric/tests"), &mut files);
        let mut text: String = files
            .iter()
            .map(|file| std::fs::read_to_string(file).expect("a case file reads"))
            .collect();
        text.push_str(&read("validation/r/src/main.rs"));
        text
    }

    #[test]
    fn yaml_files_are_found_at_any_depth_and_nothing_else() {
        let dir = tempfile::tempdir().expect("a temporary folder");
        std::fs::create_dir(dir.path().join("deeper")).expect("a folder");
        for file in ["a.yaml", "notes.md", "deeper/b.yaml"] {
            std::fs::write(dir.path().join(file), "x: 1").expect("a file");
        }
        let mut found = Vec::new();
        yaml_files(dir.path(), &mut found);
        let mut names: Vec<String> = found
            .iter()
            .map(|path| {
                path.strip_prefix(dir.path())
                    .expect("inside")
                    .display()
                    .to_string()
            })
            .collect();
        names.sort();
        assert_eq!(names, ["a.yaml", "deeper/b.yaml"]);
    }

    #[test]
    fn a_call_is_the_whole_name_followed_by_a_bracket() {
        assert!(calls("=LEN(TEXTJOIN(\"-\", TRUE, a))", "TEXTJOIN"));
        assert!(calls("=forecast.linear(1, y, x)", "FORECAST.LINEAR"));
        assert!(calls("=MC.Normal(1, 2)", "MC.Normal"));
        assert!(calls("TEXTJOIN(", "TEXTJOIN"));
        assert!(!calls("=FORECAST.LINEAR(1, y, x)", "FORECAST"));
        assert!(!calls("=NORM.S.DIST(0, TRUE)", "S.DIST"));
        assert!(!calls("=MY_SUM(1)", "SUM"));
        assert!(!calls("TEXTJOIN is listed", "TEXTJOIN"));
    }

    #[test]
    fn gaps_are_the_unchecked_and_the_exempt_that_are_now_checked() {
        let names = [
            "SUM".to_string(),
            "ABS".to_string(),
            "MC.Discrete".to_string(),
        ];
        let exempt = [("MC.Discrete", "no engine")];
        assert_eq!(
            gaps(&names, "=SUM(1)", &exempt),
            (vec!["ABS".to_string()], Vec::<String>::new())
        );
        assert_eq!(
            gaps(&names, "=SUM(1) =ABS(1) =MC.Discrete(1)", &exempt),
            (Vec::<String>::new(), vec!["MC.Discrete".to_string()])
        );
    }

    #[test]
    fn every_listed_function_has_a_case_or_a_reason() {
        let names = listed();
        let count = names.len();
        assert!(count > 200, "{count} functions listed");
        let (unchecked, stale) = gaps(&names, &cases(), &EXEMPT);
        assert_eq!(
            unchecked,
            Vec::<String>::new(),
            "no Gnumeric case or R formula calls these: add a case to validation/gnumeric/tests/functions"
        );
        assert_eq!(
            stale,
            Vec::<String>::new(),
            "a case calls these now: remove them from EXEMPT"
        );
        for (name, _) in EXEMPT {
            assert!(
                names.iter().any(|listed| listed == name),
                "{name} is not listed"
            );
        }
    }
}
