//! Multi-document YAML parsing for Forge models (v4.4.2)
//!
//! Handles parsing of YAML files with multiple documents (--- separators).

use crate::error::{ForgeError, ForgeResult};
use crate::types::ParsedModel;
use serde_yaml_ng::Value;
use std::collections::HashSet;
use std::path::Path;

use super::includes::resolve_includes;
use super::model::parse_v1_model;

/// Detect if content is a multi-document YAML file
/// A multi-document file has at least two document separators (---) on their own lines
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
pub fn parse_single_document_yaml(content: &str, path: &Path) -> ForgeResult<ParsedModel> {
    // Strip leading document marker if present
    let content = content.trim_start();
    let content = if let Some(remaining) = content.strip_prefix("---") {
        remaining.trim_start()
    } else {
        content
    };

    let yaml: Value = serde_yaml_ng::from_str(content)?;

    let mut model = parse_v1_model(&yaml)?;

    // Resolve includes if any (v4.0)
    if !model.includes.is_empty() {
        resolve_includes(&mut model, path, &mut HashSet::new())?;
    }

    Ok(model)
}

/// Parse a multi-document YAML file (v4.4.2)
/// Each document is parsed and merged into a single model.
/// Document names come from _name field or are auto-generated as "doc1", "doc2", etc.
pub fn parse_multi_document_yaml(content: &str, path: &Path) -> ForgeResult<ParsedModel> {
    let mut merged_model = ParsedModel::new();
    let mut doc_index = 0;

    // Split by document separator lines (--- on its own line)
    let docs = split_yaml_documents(content);

    for doc_content in docs {
        let doc_content = doc_content.trim();
        if doc_content.is_empty() {
            continue;
        }

        // Skip if it's just comments
        let non_comment_content: String = doc_content
            .lines()
            .filter(|line| !line.trim().starts_with('#') && !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        if non_comment_content.is_empty() {
            continue;
        }

        doc_index += 1;

        // Parse the document
        let yaml: Value = match serde_yaml_ng::from_str(doc_content) {
            Ok(v) => v,
            Err(e) => {
                return Err(ForgeError::Parse(format!(
                    "Failed to parse document {doc_index}: {e}"
                )));
            },
        };

        let doc_model = parse_v1_model(&yaml)?;

        // Get document name from _name field or generate one
        let doc_name = if let Some(Value::String(name)) = yaml.get("_name") {
            name.clone()
        } else {
            format!("doc{doc_index}")
        };

        // Merge tables with document prefix
        for (table_name, table) in doc_model.tables {
            let prefixed_name = format!("{doc_name}.{table_name}");
            let mut prefixed_table = table;
            prefixed_table.name = prefixed_name.clone();
            merged_model.tables.insert(prefixed_name, prefixed_table);
        }

        // Merge scalars with document prefix
        for (scalar_name, mut scalar) in doc_model.scalars {
            let prefixed_name = format!("{doc_name}.{scalar_name}");
            scalar.path = prefixed_name.clone();
            merged_model.scalars.insert(prefixed_name, scalar);
        }

        // Merge includes (keep original, they'll be resolved with proper paths)
        for include in doc_model.includes {
            merged_model.includes.push(include);
        }

        // Merge scenarios
        for (scenario_name, scenario) in doc_model.scenarios {
            let prefixed_name = format!("{doc_name}.{scenario_name}");
            merged_model.scenarios.insert(prefixed_name, scenario);
        }

        // Store document metadata
        merged_model.documents.push(doc_name);
    }

    // Resolve includes if any (v4.0)
    if !merged_model.includes.is_empty() {
        resolve_includes(&mut merged_model, path, &mut HashSet::new())?;
    }

    Ok(merged_model)
}

/// Split YAML content into separate documents by "---" separator lines
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
        assert!(docs.is_empty());
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

        assert!(result.tables.contains_key("revenue.data"));
        assert!(result.tables.contains_key("costs.expenses"));
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

        assert!(result.tables.contains_key("doc1.data1"));
        assert!(result.tables.contains_key("doc2.data2"));
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

        assert!(result.scalars.contains_key("config.rate"));
        assert!(result.tables.contains_key("data.values"));
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
budget:
  costs: [500, 600]
scenarios:
  pessimistic:
    growth: 0.8
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let content = std::fs::read_to_string(temp_file.path()).unwrap();
        let result = parse_multi_document_yaml(&content, temp_file.path()).unwrap();
        assert!(result.scenarios.contains_key("doc1.optimistic"));
        assert!(result.scenarios.contains_key("doc2.pessimistic"));
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
}
