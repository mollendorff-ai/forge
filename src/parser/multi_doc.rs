//! Multi-document YAML parsing for Forge models (v4.4.2)
//!
//! Handles parsing of YAML files with multiple documents (--- separators).

use crate::error::{ForgeError, ForgeResult};
use crate::types::ParsedModel;
use std::collections::HashSet;
use std::path::Path;
use yaml_serde::Value;

use super::includes::{resolve_includes_from, FileSource, IncludeSource};
use super::model::parse_v1_model;

/// Detect if content is a multi-document YAML file
/// A multi-document file has at least two document separators (---) on their own lines
#[must_use]
pub fn detect_multi_document(content: &str) -> bool {
    let mut separator_count = 0;
    for line in content.lines() {
        let trimmed = line.trim();
        // Document separator is "---" optionally followed by whitespace
        if trimmed == "---" || trimmed.starts_with("--- ") {
            separator_count += 1;
            if separator_count >= 2 {
                return true;
            }
        }
    }
    false
}

/// Parse a single YAML document
///
/// # Errors
///
/// Returns an error if the YAML content is invalid or fails schema validation.
pub fn parse_single_document_yaml(content: &str, path: &Path) -> ForgeResult<ParsedModel> {
    parse_single_document_from(&FileSource, content, path)
}

/// [`parse_single_document_yaml`], with included files read from `source`.
pub(super) fn parse_single_document_from(
    source: &dyn IncludeSource,
    content: &str,
    path: &Path,
) -> ForgeResult<ParsedModel> {
    // Strip leading document marker if present
    let content = content.trim_start();
    let content = content.strip_prefix("---").map_or(content, str::trim_start);

    let yaml: Value = yaml_serde::from_str(content)?;

    let mut model = parse_v1_model(&yaml)?;

    // Resolve includes if any (v4.0)
    if !model.includes.is_empty() {
        resolve_includes_from(source, &mut model, path, &mut HashSet::new())?;
    }

    Ok(model)
}

/// Parse a multi-document YAML file (ADR-052).
///
/// The documents are one model laid out in parts: their tables, scalars and
/// scenarios share one namespace, so a formula in one document refers to a
/// table or scalar in another exactly as it would in a single file
/// (`=SUM(sales.amount)`). A name defined in two documents is an error that
/// names both. `_name` labels a document (it is not part of any name);
/// documents without one are labelled `doc1`, `doc2`, ...
///
/// # Errors
///
/// Returns an error if any document is invalid YAML or fails schema
/// validation, or if two documents define the same table, scalar or scenario.
pub fn parse_multi_document_yaml(content: &str, path: &Path) -> ForgeResult<ParsedModel> {
    parse_multi_document_from(&FileSource, content, path)
}

/// [`parse_multi_document_yaml`], with included files read from `source`.
pub(super) fn parse_multi_document_from(
    source: &dyn IncludeSource,
    content: &str,
    path: &Path,
) -> ForgeResult<ParsedModel> {
    let mut merged_model = ParsedModel::new();
    // Which document defined each name, for the collision message
    let mut defined_in: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut doc_index = 0;

    for doc_content in split_yaml_documents(content) {
        let doc_content = doc_content.trim();
        let has_content = doc_content
            .lines()
            .any(|line| !line.trim().starts_with('#') && !line.trim().is_empty());
        if !has_content {
            continue;
        }
        doc_index += 1;

        let yaml: Value = yaml_serde::from_str(doc_content)
            .map_err(|e| ForgeError::Parse(format!("Failed to parse document {doc_index}: {e}")))?;
        let doc_model = parse_v1_model(&yaml)?;
        let doc_name = if let Some(Value::String(name)) = yaml.get("_name") {
            name.clone()
        } else {
            format!("doc{doc_index}")
        };

        let mut claim = |kind: &str, name: &str| -> ForgeResult<()> {
            let key = format!("{kind}:{name}");
            if let Some(first) = defined_in.get(&key) {
                return Err(ForgeError::Validation(format!(
                    "{kind} '{name}' is defined in document '{first}' and again in '{doc_name}'. \
                     The documents of a file share one set of names; rename one, or put the \
                     two in separate files (and compare them with forge variance)"
                )));
            }
            defined_in.insert(key, doc_name.clone());
            Ok(())
        };

        for (table_name, table) in doc_model.tables {
            claim("Table", &table_name)?;
            merged_model.tables.insert(table_name, table);
        }
        for (scalar_name, scalar) in doc_model.scalars {
            claim("Scalar", &scalar_name)?;
            merged_model.scalars.insert(scalar_name, scalar);
        }
        for (scenario_name, scenario) in doc_model.scenarios {
            claim("Scenario", &scenario_name)?;
            merged_model.scenarios.insert(scenario_name, scenario);
        }
        merged_model.includes.extend(doc_model.includes);
        merged_model.documents.push(doc_name);
    }

    // A group of scalars and a table with the same name would read the same
    // `name.x` two ways
    for table in merged_model.tables.keys() {
        let prefix = format!("{table}.");
        if let Some(scalar) = merged_model.scalars.keys().find(|s| s.starts_with(&prefix)) {
            return Err(ForgeError::Validation(format!(
                "'{table}' is both a table and the group of scalar '{scalar}'; rename one"
            )));
        }
    }

    check_document_labels_in_formulas(&merged_model)?;

    if !merged_model.includes.is_empty() {
        resolve_includes_from(source, &mut merged_model, path, &mut HashSet::new())?;
    }

    Ok(merged_model)
}

/// Before ADR-052 names were stored as `document.name`. A formula written that
/// way (`=SUM(data.sales.amount)` for document `data`) gets a clear message.
fn check_document_labels_in_formulas(model: &ParsedModel) -> ForgeResult<()> {
    let is_group = |name: &str| {
        model.tables.contains_key(name)
            || model.scalars.contains_key(name)
            || model
                .scalars
                .keys()
                .any(|s| s.starts_with(&format!("{name}.")))
    };
    let formulas = model
        .scalars
        .iter()
        .filter_map(|(n, v)| v.formula.as_deref().map(|f| (n.clone(), f)))
        .chain(model.tables.iter().flat_map(|(t, table)| {
            table
                .row_formulas
                .iter()
                .map(move |(c, f)| (format!("{t}.{c}"), f.as_str()))
        }));
    for (owner, formula) in formulas {
        for word in formula.split(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '.'))) {
            let Some((first, rest)) = word.split_once('.') else {
                continue;
            };
            if !rest.is_empty() && model.documents.iter().any(|d| d == first) && !is_group(first) {
                return Err(ForgeError::Validation(format!(
                    "{owner}: '{word}' starts with the document label '{first}'. The documents \
                     of a file share one set of names, so write '{rest}'"
                )));
            }
        }
    }
    Ok(())
}

/// Split YAML content into separate documents by "---" separator lines
#[must_use]
pub fn split_yaml_documents(content: &str) -> Vec<String> {
    let mut documents = Vec::new();
    let mut current_doc = String::new();
    let mut in_document = false;

    for line in content.lines() {
        let trimmed = line.trim();
        // Check if this is a document separator
        if trimmed == "---" || trimmed.starts_with("--- ") {
            if in_document && !current_doc.trim().is_empty() {
                documents.push(std::mem::take(&mut current_doc));
            }
            in_document = true;
            current_doc.clear();
        } else {
            // Add line to current document
            if !current_doc.is_empty() {
                current_doc.push('\n');
            }
            current_doc.push_str(line);
        }
    }

    // Don't forget the last document
    if !current_doc.trim().is_empty() {
        documents.push(current_doc);
    }

    documents
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_detect_multi_document_true() {
        let content = "---\nfirst: 1\n---\nsecond: 2\n";
        assert!(detect_multi_document(content));
    }

    #[test]
    fn test_detect_multi_document_false_single_separator() {
        let content = "---\nfirst: 1\n";
        assert!(!detect_multi_document(content));
    }

    #[test]
    fn test_detect_multi_document_false_no_separator() {
        let content = "first: 1\nsecond: 2\n";
        assert!(!detect_multi_document(content));
    }

    #[test]
    fn test_detect_multi_document_with_trailing_content() {
        let content = "--- first doc\nfirst: 1\n--- second\nsecond: 2\n";
        assert!(detect_multi_document(content));
    }

    #[test]
    fn test_split_yaml_documents() {
        let content = "---\nfirst: 1\n---\nsecond: 2\n";
        let docs = split_yaml_documents(content);
        assert_eq!(docs.len(), 2);
        assert!(docs[0].contains("first: 1"));
        assert!(docs[1].contains("second: 2"));
    }

    #[test]
    fn test_split_yaml_documents_empty() {
        let content = "";
        let docs = split_yaml_documents(content);
        assert_eq!(docs.len(), 0);
    }

    #[test]
    fn test_split_yaml_documents_single() {
        let content = "---\nfirst: 1\n";
        let docs = split_yaml_documents(content);
        assert_eq!(docs.len(), 1);
    }

    #[test]
    fn test_parse_multi_doc_with_names() {
        let yaml_content = r#"---
_forge_version: "5.0.0"
_name: "revenue"
data:
  values: [100, 200, 300]
---
_forge_version: "5.0.0"
_name: "costs"
expenses:
  amounts: [50, 100, 150]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let result = parse_multi_document_yaml(&content, temp_file.path()).unwrap();

        assert!(result.tables.contains_key("data"));
        assert!(result.tables.contains_key("expenses"));
        assert_eq!(result.documents.len(), 2);
        assert!(result.documents.contains(&"revenue".to_string()));
        assert!(result.documents.contains(&"costs".to_string()));
    }

    #[test]
    fn test_parse_multi_doc_auto_names() {
        let yaml_content = r#"---
_forge_version: "5.0.0"
data1:
  values: [1, 2, 3]
---
_forge_version: "5.0.0"
data2:
  values: [4, 5, 6]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let result = parse_multi_document_yaml(&content, temp_file.path()).unwrap();

        assert!(result.tables.contains_key("data1"));
        assert!(result.tables.contains_key("data2"));
        assert_eq!(result.documents, ["doc1", "doc2"]);
    }

    #[test]
    fn test_parse_multi_doc_with_scalars() {
        let yaml_content = r#"---
_forge_version: "5.0.0"
_name: "config"
rate:
  value: 0.05
  formula: null
---
_forge_version: "5.0.0"
_name: "data"
values:
  items: [1, 2, 3]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let result = parse_multi_document_yaml(&content, temp_file.path()).unwrap();

        assert!(result.scalars.contains_key("rate"));
        assert!(result.tables.contains_key("values"));
    }

    #[test]
    fn test_parse_multi_doc_skip_comments() {
        let yaml_content = r#"---
# This is a comment-only document
# No actual content
---
_forge_version: "5.0.0"
data:
  values: [1, 2, 3]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let result = parse_multi_document_yaml(&content, temp_file.path()).unwrap();
        assert!(!result.tables.is_empty());
    }

    #[test]
    fn test_parse_multi_doc_with_empty_doc() {
        let yaml_content = r#"---
_forge_version: "5.0.0"
data:
  values: [1, 2, 3]
---

---
_forge_version: "5.0.0"
data2:
  values: [4, 5, 6]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let result = parse_multi_document_yaml(&content, temp_file.path()).unwrap();
        assert_eq!(result.tables.len(), 2);
    }

    #[test]
    fn test_parse_multi_doc_invalid_yaml_error() {
        let yaml_content = r#"---
_forge_version: "5.0.0"
data:
  values: [1, 2, 3]
---
invalid: yaml: [[[
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let result = parse_multi_document_yaml(&content, temp_file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse document"));
    }

    #[test]
    fn test_parse_multi_doc_with_scenarios() {
        let yaml_content = r#"---
_name: doc1
_forge_version: "5.0.0"
budget:
  revenue: [1000, 2000]
scenarios:
  optimistic:
    growth: 1.2
---
_name: doc2
_forge_version: "5.0.0"
spending:
  costs: [500, 600]
scenarios:
  pessimistic:
    growth: 0.8
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let result = parse_multi_document_yaml(&content, temp_file.path()).unwrap();
        assert!(result.scenarios.contains_key("optimistic"));
        assert!(result.scenarios.contains_key("pessimistic"));
    }

    #[test]
    fn test_multi_document_yaml_with_leading_separator() {
        let yaml_content = r#"---
_forge_version: "5.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        // Single doc with leading separator should be parsed as single doc
        let result = parse_single_document_yaml(&content, temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        let sales = result.tables.get("sales").unwrap();
        assert_eq!(sales.row_count(), 3);
    }

    fn parse_str(text: &str) -> ForgeResult<ParsedModel> {
        let mut f = NamedTempFile::with_suffix(".yaml").unwrap();
        f.write_all(text.as_bytes()).unwrap();
        crate::parser::parse_model(f.path())
    }

    /// ADR-052: one namespace; a formula in one document uses another's names
    #[test]
    fn formulas_refer_across_documents() {
        let model = parse_str(
            "---\n_forge_version: \"5.1.0\"\n_name: data\nsales:\n  amount: [100, 200]\n  tax: \"=amount * 0.1\"\n---\n_forge_version: \"5.1.0\"\n_name: summary\ntotals:\n  tax:\n    formula: \"=SUM(sales.tax)\"\n  double:\n    formula: \"=totals.tax * 2\"\n",
        )
        .unwrap();
        assert_eq!(model.documents, ["data", "summary"]);
        let calc = crate::core::ArrayCalculator::new(model)
            .calculate_all()
            .unwrap();
        assert_eq!(calc.scalars["totals.tax"].value, Some(30.0));
        assert_eq!(calc.scalars["totals.double"].value, Some(60.0));
    }

    #[test]
    fn a_name_in_two_documents_is_an_error() {
        let err = parse_str(
            "---\n_forge_version: \"5.1.0\"\n_name: budget\nrevenue:\n  total: 100\n---\n_forge_version: \"5.1.0\"\n_name: actual\nrevenue:\n  total: 90\n",
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("'revenue.total' is defined in document 'budget' and again in 'actual'"),
            "{err}"
        );
    }

    #[test]
    fn a_document_label_in_a_formula_is_explained() {
        let err = parse_str(
            "---\n_forge_version: \"5.1.0\"\n_name: data\nsales:\n  amount: [1, 2]\n---\n_forge_version: \"5.1.0\"\n_name: summary\nt:\n  s:\n    formula: \"=SUM(data.sales.amount)\"\n",
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("so write 'sales.amount'"), "{err}");
    }
}
