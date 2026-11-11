//! Writing calculated results back into a model file (ADR-051).
//!
//! A model file holds formulas and their last results, like an Excel cell.
//! `forge calculate` writes each formula scalar's result into its `value:`,
//! and nothing else: the edit is made in the text itself (`in_place`), so
//! comments, quotes, blank lines, key order and inputs stay as typed.

pub mod in_place;

use crate::core::precision::to_15_digits;
use crate::error::{ForgeError, ForgeResult};
use crate::types::ParsedModel;
use std::fs;
use std::path::Path;

/// Write the results of formula scalars into the model file.
///
/// Each result is stored rounded to 15 significant digits, as Excel shows it
/// (`core::precision`): `418.33`, not `418.3299999999999`.
///
/// Returns the number of values that changed. Multi-document files are
/// written document by document (ADR-052). The file is rewritten,
/// and a `.yaml.bak` made, only if at least one value changed.
///
/// # Errors
///
/// Returns an error if the file cannot be read or written, or if the results
/// cannot be written without changing anything else in the file (then the
/// file is left as it was).
pub fn write_calculated_results(path: &Path, result: &ParsedModel) -> ForgeResult<usize> {
    let content = fs::read_to_string(path)?;
    let multi_doc = crate::parser::detect_multi_document(&content);

    let values: Vec<(in_place::ScalarPath, f64)> = result
        .scalars
        .iter()
        .filter(|(_, var)| var.formula.is_some())
        .filter_map(|(name, var)| {
            var.value.map(|v| {
                (
                    name.split('.').map(str::to_string).collect(),
                    to_15_digits(v),
                )
            })
        })
        .collect();

    let edited = if multi_doc {
        in_place::set_values_multi(&content, &values)
    } else {
        in_place::set_values(&content, &values)
    }
    .map_err(|e| {
        ForgeError::Validation(format!(
            "{}: results not written, the file is unchanged: {e}",
            path.display()
        ))
    })?;

    let changed = count_changed(&content, &edited);
    if changed == 0 {
        return Ok(0);
    }

    let backup_path = path.with_extension("yaml.bak");
    fs::copy(path, &backup_path)?;
    fs::write(path, edited)?;
    Ok(changed)
}

/// The number of lines that differ (each changed or added result is one).
fn count_changed(before: &str, after: &str) -> usize {
    let before: Vec<&str> = before.lines().collect();
    let after: Vec<&str> = after.lines().collect();
    // Lines are only replaced or inserted, in order: walk both.
    let (mut i, mut j, mut changed) = (0, 0, 0);
    while j < after.len() {
        if i < before.len() && before[i] == after[j] {
            i += 1;
        } else {
            changed += 1;
            if i < before.len() && after.len() - j == before.len() - i {
                i += 1; // replaced
            }
        }
        j += 1;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Variable;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn model_with(scalars: &[(&str, Option<f64>, Option<&str>)]) -> ParsedModel {
        let mut model = ParsedModel::new();
        for (name, value, formula) in scalars {
            model.scalars.insert(
                (*name).to_string(),
                Variable::new((*name).to_string(), *value, formula.map(str::to_string)),
            );
        }
        model
    }

    fn file_with(text: &str) -> NamedTempFile {
        let mut f = NamedTempFile::with_suffix(".yaml").unwrap();
        f.write_all(text.as_bytes()).unwrap();
        f
    }

    #[test]
    fn writes_only_formula_results_and_keeps_comments() {
        let text = "_forge_version: \"5.0.0\"\n# keep me\ninputs:\n  price:\n    value: 16   # typed by hand\n    formula: null\nout:\n  revenue:\n    value: null\n    formula: \"=inputs.price * 100\"\n";
        let f = file_with(text);
        let model = model_with(&[
            ("inputs.price", Some(16.0), None),
            ("out.revenue", Some(1600.0), Some("=inputs.price * 100")),
        ]);
        assert_eq!(write_calculated_results(f.path(), &model).unwrap(), 1);
        let after = fs::read_to_string(f.path()).unwrap();
        assert_eq!(
            after,
            text.replace("    value: null\n", "    value: 1600\n")
        );
        assert!(f.path().with_extension("yaml.bak").exists());
        let _ = fs::remove_file(f.path().with_extension("yaml.bak"));
    }

    #[test]
    fn unchanged_results_leave_the_file_alone() {
        let text = "out:\n  total:\n    formula: \"=1 + 1\"\n    value: 2\n";
        let f = file_with(text);
        let model = model_with(&[("out.total", Some(2.0), Some("=1 + 1"))]);
        assert_eq!(write_calculated_results(f.path(), &model).unwrap(), 0);
        assert_eq!(fs::read_to_string(f.path()).unwrap(), text);
        assert!(!f.path().with_extension("yaml.bak").exists());
    }

    #[test]
    fn writes_into_the_document_that_has_the_scalar() {
        let text = "---\n_forge_version: \"5.1.0\"\n_name: a\nx:\n  n: 2\n---\n_forge_version: \"5.1.0\"\n_name: b\n# keep\ny:\n  m:\n    formula: \"=x.n * 3\"\n";
        let f = file_with(text);
        let model = model_with(&[
            ("x.n", Some(2.0), None),
            ("y.m", Some(6.0), Some("=x.n * 3")),
        ]);
        assert_eq!(write_calculated_results(f.path(), &model).unwrap(), 1);
        assert_eq!(
            fs::read_to_string(f.path()).unwrap(),
            format!("{text}    value: 6\n")
        );
        let _ = fs::remove_file(f.path().with_extension("yaml.bak"));
    }

    #[test]
    fn counts_replaced_and_inserted_lines() {
        assert_eq!(count_changed("a\nb\nc\n", "a\nB\nc\n"), 1);
        assert_eq!(count_changed("a\nc\n", "a\nb\nc\n"), 1);
        assert_eq!(count_changed("a\nc\ne\n", "a\nb\nc\nd\ne\n"), 2);
        assert_eq!(count_changed("a\n", "a\n"), 0);
    }
}
