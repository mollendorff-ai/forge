//! Parser module for Forge YAML models
//!
//! This module provides functionality for parsing Forge YAML files into
//! structured `ParsedModel` objects that can be calculated and analyzed.
//!
//! # Submodules
//! - `arrays`: Array/column type parsing (Number, Text, Date, Boolean)
//! - `schema`: JSON Schema validation (v1.0.0 and v5.0.0)
//! - `multi_doc`: Multi-document YAML parsing (v4.4.2)
//! - `includes`: Cross-file include resolution (v4.0)
//! - `variables`: Table and scalar variable parsing
//! - `model`: Core model parsing logic

mod arrays;
mod includes;
mod model;
mod multi_doc;
mod schema;
mod variables;

// Re-export commonly used functions
pub use arrays::{detect_array_type, is_valid_date_format, parse_array_value, type_name};
pub use includes::{parse_includes, resolve_includes};
pub use model::{parse_nested_scalars, parse_scenarios, parse_v1_model};
pub use multi_doc::{
    detect_multi_document, parse_multi_document_yaml, parse_single_document_yaml,
    split_yaml_documents,
};
pub use schema::{validate_against_schema, validate_v1_0_0_no_tables};
pub use variables::{is_nested_scalar_section, parse_metadata, parse_scalar_variable, parse_table};

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
    let content = std::fs::read_to_string(path)?;

    // Check if this is a multi-document YAML file (v4.4.2)
    // Multi-doc files have at least two document separators (---) on their own lines
    // We need to skip comments and whitespace when detecting
    let is_multi_doc = detect_multi_document(&content);

    if is_multi_doc {
        // Parse all documents and merge (v4.4.2)
        parse_multi_document_yaml(&content, path)
    } else {
        // Single document parsing (original behavior)
        parse_single_document_yaml(&content, path)
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
}
