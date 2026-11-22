//! Parser module for Forge YAML models
//!
//! This module provides functionality for parsing Forge YAML files into
//! structured `ParsedModel` objects that can be calculated and analyzed.
//!
//! # Submodules
//! - `arrays`: Array/column type parsing (Number, Text, Date, Boolean)
//! - `schema`: validation against the schema of the model's format
//! - `schema_reader`: the model formats' JSON schemas, checked by `jsonschema` (ADR-072)
//! - `schema_errors`: what does not fit a schema, in words
//! - `multi_doc`: Multi-document YAML parsing (v4.4.2)
//! - `includes`: Cross-file include resolution (v4.0)
//! - `variables`: Table and scalar variable parsing
//! - `model`: Core model parsing logic
//! - `expect`: what a model states its results should be (`expect:`, ADR-074)

mod arrays;
mod expect;
mod finite;
mod includes;
mod memory;
mod merge_keys;
pub(crate) mod model;
mod multi_doc;
pub(crate) mod periods;
mod schema;
mod schema_errors;
mod schema_reader;
mod variables;

// Re-export commonly used functions
pub use arrays::{detect_array_type, is_valid_date_format, parse_array_value, type_name};
#[cfg(feature = "cli")]
pub(crate) use includes::normalize;
pub use includes::{parse_includes, resolve_includes};
pub use memory::{files_in_memory, read_model_text, with_files_in_memory};
pub use merge_keys::read_yaml;
pub use model::{parse_nested_scalars, parse_scenarios, parse_v1_model};
pub use multi_doc::{
    detect_multi_document, parse_multi_document_yaml, parse_single_document_yaml,
    split_yaml_documents,
};
pub use schema::{format_at_least, validate_against_schema, validate_v1_0_0_no_tables};
pub use variables::{
    is_nested_scalar_section, looks_like_date, parse_metadata, parse_scalar_variable, parse_table,
    scalar_date,
};

use crate::error::ForgeResult;
use crate::types::ParsedModel;

/// Parse a Forge model file (v1.0.0 array format) and return a `ParsedModel`.
///
/// This is the main entry point for parsing Forge YAML files.
///
/// # Arguments
/// * `path` - Path to the Forge YAML file
///
/// # Returns
/// * `Ok(ParsedModel)` - Successfully parsed model with tables and scalars
/// * `Err(ForgeError)` - Parse error with detailed context
///
/// # Errors
///
/// Returns an error if the file cannot be read, contains invalid YAML, or fails
/// schema validation.
///
/// # Example
/// ```no_run
/// use mollendorff_forge::parser::parse_model;
/// use std::path::Path;
///
/// let model = parse_model(Path::new("model.yaml"))?;
/// println!("Tables: {}", model.tables.len());
/// # Ok::<(), mollendorff_forge::error::ForgeError>(())
/// ```
pub fn parse_model(path: &std::path::Path) -> ForgeResult<ParsedModel> {
    let content = read_model_text(path)?;
    // Inside `with_files_in_memory` the includes come from memory too
    let mut model = memory::with_current(|files| {
        files.map_or_else(
            || parse_content(&includes::FileSource, &content, path),
            |files| parse_content(&includes::MemorySource { files }, &content, path),
        )
    })?;
    model.source = Some(path.to_path_buf());
    Ok(model)
}

/// Parse a model held in memory, reading nothing from the file system (ADR-055).
///
/// `text` is the main model and `name` its file name, used in messages and as
/// the place its includes are relative to. `files` holds the text of every
/// file it includes, directly or through another include, by the path written
/// in `_includes` (`costs.yaml`), relative to the main model as on disk. The
/// model's `source` is `None`: there is no file to write results back to.
///
/// # Errors
///
/// Returns the errors [`parse_model`] returns for the same files, including
/// an included file that is not in `files`.
pub fn parse_model_text<S: std::hash::BuildHasher>(
    name: &str,
    text: &str,
    files: &std::collections::HashMap<String, String, S>,
) -> ForgeResult<ParsedModel> {
    let source = includes::MemorySource { files };
    parse_content(&source, text, std::path::Path::new(name))
}

/// Check every file the model at `path` includes, before reading it.
///
/// Each path the parser would open, directly or through another include, is
/// given to `check` first (`forge mcp --root`). Errors other than a refused
/// path are left to the command that reads the model.
///
/// # Errors
///
/// Returns the first refusal of `check`.
pub fn check_included_files(
    path: &std::path::Path,
    check: &dyn Fn(&std::path::Path) -> Result<(), String>,
) -> Result<(), String> {
    let Ok(content) = read_model_text(path) else {
        return Ok(());
    };
    let source = includes::CheckedFileSource {
        check,
        refused: std::cell::RefCell::new(None),
    };
    // Only the refusal matters here; the command reports any other error
    let _ = parse_content(&source, &content, path);
    source.refused.into_inner().map_or(Ok(()), Err)
}

/// One file's text, of one document or several (v4.4.2: a multi-document
/// file has at least two `---` lines), with its includes read from `source`.
fn parse_content(
    source: &dyn includes::IncludeSource,
    content: &str,
    path: &std::path::Path,
) -> ForgeResult<ParsedModel> {
    if detect_multi_document(content) {
        multi_doc::parse_multi_document_from(source, content, path)
    } else {
        multi_doc::parse_single_document_from(source, content, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::Path;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_file_not_found() {
        let result = parse_model(Path::new("/nonexistent/path/file.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let yaml_content = "not: valid: yaml: [[[";

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_table_column_not_array() {
        let yaml_content = r#"
_forge_version: "1.0.0"
data:
  values: 123
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path());
        assert!(result.is_err());
    }

    mod memory {
        //! A model held in memory (`parse_model_text`, ADR-055) parses and calculates
        //! exactly as the same files read from disk.

        use super::{parse_model, parse_model_text};
        use crate::core::ArrayCalculator;
        use crate::types::ParsedModel;
        use std::collections::HashMap;
        use std::path::PathBuf;

        fn example(path: &str) -> PathBuf {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("examples")
                .join(path)
        }

        fn text(path: &str) -> String {
            std::fs::read_to_string(example(path)).unwrap()
        }

        /// What two parses of one model must share: everything but where it came from.
        fn same_model(from_disk: &ParsedModel, in_memory: &ParsedModel) {
            assert_eq!(
                format!("{:?}", from_disk.tables),
                format!("{:?}", in_memory.tables)
            );
            assert_eq!(
                format!("{:?}", from_disk.scalars),
                format!("{:?}", in_memory.scalars)
            );
            // Scenarios and their overrides are HashMaps, in no fixed order
            let scenarios = |m: &ParsedModel| {
                m.scenarios
                    .iter()
                    .map(|(name, s)| {
                        let overrides = s
                            .overrides
                            .iter()
                            .map(|(k, v)| (k.clone(), v.to_bits()))
                            .collect::<std::collections::BTreeMap<_, _>>();
                        (name.clone(), overrides)
                    })
                    .collect::<std::collections::BTreeMap<_, _>>()
            };
            assert_eq!(scenarios(from_disk), scenarios(in_memory));
            let namespaces =
                |m: &ParsedModel| m.resolved_includes.keys().cloned().collect::<Vec<_>>();
            assert_eq!(namespaces(from_disk), namespaces(in_memory));
            for (namespace, include) in &from_disk.resolved_includes {
                let other = &in_memory.resolved_includes[namespace];
                same_model(&include.model, &other.model);
            }
        }

        fn calculated(model: ParsedModel) -> String {
            let result = ArrayCalculator::new(model).calculate_all().unwrap();
            format!("{:?}{:?}", result.scalars, result.tables)
        }

        #[test]
        fn a_model_with_an_include_calculates_as_on_disk() {
            let files = HashMap::from([("costs.yaml".to_string(), text("bagel/costs.yaml"))]);
            let in_memory =
                parse_model_text("bagel.yaml", &text("bagel/bagel.yaml"), &files).unwrap();
            let from_disk = parse_model(&example("bagel/bagel.yaml")).unwrap();

            assert!(in_memory.source.is_none());
            same_model(&from_disk, &in_memory);
            assert_eq!(calculated(from_disk), calculated(in_memory));
        }

        #[test]
        fn an_include_of_an_include_is_read_from_memory_too() {
            // bagel_simulate.yaml includes costs_risk.yaml, which includes costs.yaml
            let files = HashMap::from([
                ("costs_risk.yaml".to_string(), text("bagel/costs_risk.yaml")),
                ("costs.yaml".to_string(), text("bagel/costs.yaml")),
            ]);
            let in_memory = parse_model_text(
                "bagel_simulate.yaml",
                &text("bagel/bagel_simulate.yaml"),
                &files,
            )
            .unwrap();
            let from_disk = parse_model(&example("bagel/bagel_simulate.yaml")).unwrap();

            same_model(&from_disk, &in_memory);
            assert!(in_memory.resolved_includes["costs"]
                .model
                .resolved_includes
                .contains_key("base"));
        }

        #[test]
        fn a_model_without_includes_needs_no_files() {
            for path in [
                "mortgage/mortgage.yaml",
                "periods/monthly_plan.yaml",
                "starter.yaml",
            ] {
                let name = path.rsplit('/').next().unwrap();
                let in_memory = parse_model_text(name, &text(path), &HashMap::new()).unwrap();
                let from_disk = parse_model(&example(path)).unwrap();
                same_model(&from_disk, &in_memory);
                assert_eq!(calculated(from_disk), calculated(in_memory), "{path}");
            }
        }

        #[test]
        fn a_missing_include_is_named_as_on_disk() {
            let error = parse_model_text("bagel.yaml", &text("bagel/bagel.yaml"), &HashMap::new())
                .unwrap_err()
                .to_string();
            assert!(
                error.contains("Included file not found: costs.yaml (referenced as 'costs.yaml')"),
                "{error}"
            );
        }

        #[test]
        fn include_paths_are_relative_to_the_including_file() {
            // models/main.yaml includes ../shared/rates.yaml, which includes fx.yaml
            // next to it: the keys are paths relative to the main model's folder
            let main = "_forge_version: \"5.1.0\"\n\
                _includes:\n  - file: ../shared/rates.yaml\n    as: rates\n\
                plan:\n  amount: 100\n  in_euros: {formula: \"=amount * @rates.fx.eur\"}\n";
            let rates = "_forge_version: \"5.1.0\"\n\
                 _includes:\n  - file: fx.yaml\n    as: market\n\
                 fx:\n  eur: {formula: \"=@market.quotes.eur\"}\n";
            let fx = "_forge_version: \"5.1.0\"\nquotes:\n  eur: 0.92\n";
            let files = HashMap::from([
                ("shared/rates.yaml".to_string(), rates.to_string()),
                ("shared/fx.yaml".to_string(), fx.to_string()),
            ]);
            let model = parse_model_text("models/main.yaml", main, &files).unwrap();
            let result = ArrayCalculator::new(model).calculate_all().unwrap();
            let in_euros = result.scalars["plan.in_euros"].value.unwrap();
            assert!((in_euros - 92.0).abs() < 1e-9, "{in_euros}");
        }

        #[test]
        fn a_circular_include_is_refused() {
            let a =
                "_forge_version: \"5.1.0\"\n_includes:\n  - file: b.yaml\n    as: b\nx:\n  v: 1\n";
            let b =
                "_forge_version: \"5.1.0\"\n_includes:\n  - file: a.yaml\n    as: a\ny:\n  w: 2\n";
            let files = HashMap::from([
                ("a.yaml".to_string(), a.to_string()),
                ("b.yaml".to_string(), b.to_string()),
            ]);
            let error = parse_model_text("a.yaml", a, &files)
                .unwrap_err()
                .to_string();
            assert!(error.contains("Circular dependency detected"), "{error}");
        }
    }

    #[cfg(feature = "cli")]
    mod short_form {
        //! Model format 5.1.0: the scalar short form (ADR-051)

        use crate::parser::parse_model;
        use std::io::Write;

        fn parse(text: &str) -> crate::error::ForgeResult<crate::types::ParsedModel> {
            let mut f = tempfile::NamedTempFile::with_suffix(".yaml").unwrap();
            f.write_all(text.as_bytes()).unwrap();
            parse_model(f.path())
        }

        #[test]
        fn plain_numbers_and_formula_only_scalars() {
            let m = parse(
                "_forge_version: \"5.1.0\"\ntotal_budget: 1000\nassumptions:\n  price: 16\n  rate: 0.03\nyear:\n  revenue:\n    formula: \"=assumptions.price * 100\"\n    unit: CAD\n",
            )
            .unwrap();
            assert_eq!(m.scalars["assumptions.price"].value, Some(16.0));
            assert_eq!(m.scalars["assumptions.price"].formula, None);
            assert_eq!(m.scalars["total_budget"].value, Some(1000.0));
            let revenue = &m.scalars["year.revenue"];
            assert_eq!(revenue.value, None);
            assert_eq!(revenue.formula.as_deref(), Some("=assumptions.price * 100"));
            assert_eq!(revenue.metadata.unit.as_deref(), Some("CAD"));
        }

        #[test]
        fn short_and_long_forms_mean_the_same() {
            let short = parse(
                "_forge_version: \"5.1.0\"\na:\n  x: 2\nb:\n  y:\n    formula: \"=a.x * 3\"\n",
            )
            .unwrap();
            let long = parse(
                "_forge_version: \"5.0.0\"\na:\n  x:\n    value: 2\n    formula: null\nb:\n  y:\n    value: null\n    formula: \"=a.x * 3\"\n",
            )
            .unwrap();
            for name in ["a.x", "b.y"] {
                assert_eq!(short.scalars[name].value, long.scalars[name].value);
                assert_eq!(short.scalars[name].formula, long.scalars[name].formula);
            }
        }

        #[test]
        fn table_with_rich_formula_column_before_data_column_is_a_table() {
            // Order used to decide: a rich formula column first made it a scalar group
            let m = parse(
                "_forge_version: \"5.1.0\"\nt:\n  y:\n    formula: \"=x * 2\"\n    unit: CAD\n  x: [1, 2]\n",
            )
            .unwrap();
            assert!(m.tables.contains_key("t"), "{:?}", m.tables.keys());
            assert!(m.scalars.is_empty(), "{:?}", m.scalars.keys());
        }

        #[test]
        fn short_form_in_a_5_0_0_file_says_which_format_to_use() {
            let err = parse("_forge_version: \"5.0.0\"\na:\n  x: 2\n")
                .unwrap_err()
                .to_string();
            assert!(err.contains("'a.x' is a plain number"), "{err}");
            assert!(err.contains("_forge_version: \"5.1.0\""), "{err}");
        }

        #[test]
        fn bare_formula_string_in_a_scalar_group_is_explained() {
            let err = parse("_forge_version: \"5.1.0\"\na:\n  x: 2\n  y: \"=a.x * 3\"\n")
                .unwrap_err()
                .to_string();
            assert!(err.contains("`y: {formula: \"=a.x * 3\"}`"), "{err}");
        }

        #[test]
        fn calculate_writes_results_next_to_formulas_only() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            let text = "_forge_version: \"5.1.0\"\n# plan\na:\n  x: 2   # typed\nb:\n  y:\n    formula: \"=a.x * 3\"\n";
            std::fs::write(&path, text).unwrap();
            let result = crate::cli::calculate_core(&path, false, None).unwrap();
            assert!(result.file_updated);
            let after = std::fs::read_to_string(&path).unwrap();
            assert_eq!(after, format!("{text}    value: 6\n"));
            // A second run finds nothing to change
            let again = crate::cli::calculate_core(&path, false, None).unwrap();
            assert!(!again.file_updated);
        }

        #[test]
        fn upgrade_converts_and_the_result_calculates_the_same() {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            let long = "_forge_version: \"5.0.0\"\n# keep\na:\n  x:\n    value: 2\n    formula: null\nb:\n  y:\n    value: null\n    formula: \"=a.x * 3\"\n";
            std::fs::write(&path, long).unwrap();
            let before = crate::cli::calculate_core(&path, true, None).unwrap();
            crate::cli::console::capture(|c| crate::cli::upgrade(c, &path, false, "5.1.0", false))
                .0
                .unwrap();
            let converted = std::fs::read_to_string(&path).unwrap();
            assert_eq!(
                converted,
                "_forge_version: \"5.1.0\"\n# keep\na:\n  x: 2\nb:\n  y:\n    formula: \"=a.x * 3\"\n"
            );
            let after = crate::cli::calculate_core(&path, true, None).unwrap();
            assert_eq!(before.scalars, after.scalars);
        }
    }
}
