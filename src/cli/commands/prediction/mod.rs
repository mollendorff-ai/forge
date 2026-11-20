//! Prediction Methods CLI Commands
//!
//! CLI handlers for advanced forecasting methods:
//! - scenarios: Run scenario analysis (Base/Bull/Bear)
//! - `decision_tree`: Analyze decision trees with backward induction
//! - `real_options`: Value managerial flexibility (defer/expand/abandon)
//! - tornado: Generate sensitivity tornado diagrams
//! - bootstrap: Non-parametric confidence intervals via resampling
//! - bayesian: Bayesian network inference and queries

mod bayesian;
mod bootstrap;
mod decision_tree;
mod real_options;
mod scenarios;
mod tornado;

pub use bayesian::{bayesian, bayesian_core};
pub use bootstrap::{bootstrap, bootstrap_core};
pub use decision_tree::{decision_tree, decision_tree_core};
pub use real_options::{real_options, real_options_core};
pub use scenarios::{scenarios, scenarios_core};
pub use tornado::{tornado, tornado_core};

use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::precision::Rounded;
use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;
use std::path::Path;

/// Serialises an analytics result for `--output`: JSON for a `.json` path,
/// YAML otherwise, with numbers to 15 significant digits (`core::precision`).
/// (This used to write a Rust debug dump, which no tool can read.)
pub(super) fn serialize_for_file<T: serde::Serialize>(
    value: &T,
    path: &Path,
) -> ForgeResult<String> {
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        serde_json::to_string_pretty(&Rounded(value))
            .map_err(|e| ForgeError::Validation(format!("JSON error: {e}")))
    } else {
        yaml_serde::to_string(&Rounded(value))
            .map_err(|e| ForgeError::Validation(format!("YAML error: {e}")))
    }
}

/// Write an analysis to the `--output` file, as [`serialize_for_file`] gives it,
/// and say so.
///
/// # Errors
///
/// Returns an error if the result cannot be serialised or the file cannot be written.
pub(super) fn write_result<T: serde::Serialize>(
    c: &mut Console<'_>,
    value: &T,
    path: &Path,
) -> ForgeResult<()> {
    let text = serialize_for_file(value, path)?;
    std::fs::write(path, text).map_err(ForgeError::Io)?;
    outln!(
        c,
        "{}",
        format!("💾 Results written to {}", path.display())
            .bold()
            .green()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{serialize_for_file, write_result};
    use crate::cli::Console;
    use std::path::Path;

    /// A value that cannot be written as JSON (a map with keys that are not text) or as YAML
    struct Unwritable;

    impl serde::Serialize for Unwritable {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("no way to write this"))
        }
    }

    #[test]
    fn a_result_is_json_for_a_json_path_and_yaml_for_any_other() {
        let value = std::collections::BTreeMap::from([("rate", 0.1 + 0.2)]);
        assert_eq!(
            serialize_for_file(&value, Path::new("r.json")).unwrap(),
            "{\n  \"rate\": 0.3\n}"
        );
        assert_eq!(
            serialize_for_file(&value, Path::new("r.yaml")).unwrap(),
            "rate: 0.3\n"
        );
    }

    #[test]
    fn a_result_that_cannot_be_written_is_refused_in_either_format() {
        for (path, want) in [
            (
                "r.json",
                "Validation error: JSON error: no way to write this",
            ),
            (
                "r.yaml",
                "Validation error: YAML error: no way to write this",
            ),
        ] {
            let err = serialize_for_file(&Unwritable, Path::new(path)).unwrap_err();
            assert_eq!(err.to_string(), want);
            let mut out = Vec::new();
            let mut err_out = Vec::new();
            let mut input = std::io::empty();
            let mut console = Console::new(&mut out, &mut err_out, &mut input);
            let err = write_result(&mut console, &Unwritable, Path::new(path)).unwrap_err();
            assert_eq!(err.to_string(), want);
            assert_eq!(out, b"");
        }
    }
}
