use crate::error::{ForgeError, ForgeResult};
use crate::types::{
    Column, ColumnValue, Include, Metadata, ParsedModel, ResolvedInclude, Scenario, Table, Variable,
};
use serde_yaml_ng::Value;
use std::collections::HashSet;
use std::path::Path;

/// Parse a Forge model file (v1.0.0 array format) and return a ParsedModel.
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
/// # Example
/// ```no_run
/// use royalbit_forge::parser::parse_model;
/// use std::path::Path;
///
/// let model = parse_model(Path::new("model.yaml"))?;
/// println!("Tables: {}", model.tables.len());
/// # Ok::<(), royalbit_forge::error::ForgeError>(())
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

/// Detect if content is a multi-document YAML file
/// A multi-document file has at least two document separators (---) on their own lines
fn detect_multi_document(content: &str) -> bool {
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
fn parse_single_document_yaml(content: &str, path: &Path) -> ForgeResult<ParsedModel> {
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
fn parse_multi_document_yaml(content: &str, path: &Path) -> ForgeResult<ParsedModel> {
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
                    "Failed to parse document {}: {}",
                    doc_index, e
                )));
            }
        };

        let doc_model = parse_v1_model(&yaml)?;

        // Get document name from _name field or generate one
        let doc_name = if let Some(Value::String(name)) = yaml.get("_name") {
            name.clone()
        } else {
            format!("doc{}", doc_index)
        };

        // Merge tables with document prefix
        for (table_name, table) in doc_model.tables {
            let prefixed_name = format!("{}.{}", doc_name, table_name);
            let mut prefixed_table = table;
            prefixed_table.name = prefixed_name.clone();
            merged_model.tables.insert(prefixed_name, prefixed_table);
        }

        // Merge scalars with document prefix
        for (scalar_name, mut scalar) in doc_model.scalars {
            let prefixed_name = format!("{}.{}", doc_name, scalar_name);
            scalar.path = prefixed_name.clone();
            merged_model.scalars.insert(prefixed_name, scalar);
        }

        // Merge includes (keep original, they'll be resolved with proper paths)
        for include in doc_model.includes {
            merged_model.includes.push(include);
        }

        // Merge scenarios
        for (scenario_name, scenario) in doc_model.scenarios {
            let prefixed_name = format!("{}.{}", doc_name, scenario_name);
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
fn split_yaml_documents(content: &str) -> Vec<String> {
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

/// Resolve all includes in a model, loading and parsing referenced files.
/// Detects circular dependencies.
fn resolve_includes(
    model: &mut ParsedModel,
    base_path: &Path,
    visited: &mut HashSet<std::path::PathBuf>,
) -> ForgeResult<()> {
    let base_dir = base_path.parent().unwrap_or_else(|| Path::new("."));

    // Check for circular dependency
    let canonical = base_path
        .canonicalize()
        .unwrap_or_else(|_| base_path.to_path_buf());
    if visited.contains(&canonical) {
        return Err(ForgeError::Parse(format!(
            "Circular dependency detected: {} is already included",
            base_path.display()
        )));
    }
    visited.insert(canonical);

    // Process each include
    for include in model.includes.clone() {
        let include_path = base_dir.join(&include.file);

        if !include_path.exists() {
            return Err(ForgeError::Parse(format!(
                "Included file not found: {} (referenced as '{}')",
                include_path.display(),
                include.file
            )));
        }

        // Parse the included file
        let content = std::fs::read_to_string(&include_path)?;
        let yaml: Value = serde_yaml_ng::from_str(&content)?;
        let mut included_model = parse_v1_model(&yaml)?;

        // Recursively resolve includes in the included file
        if !included_model.includes.is_empty() {
            resolve_includes(&mut included_model, &include_path, visited)?;
        }

        // Store resolved include
        let resolved = ResolvedInclude {
            include: include.clone(),
            resolved_path: include_path.canonicalize().unwrap_or(include_path),
            model: included_model,
        };
        model
            .resolved_includes
            .insert(include.namespace.clone(), resolved);
    }

    Ok(())
}

/// Parse v1.0.0 array model
fn parse_v1_model(yaml: &Value) -> ForgeResult<ParsedModel> {
    // Validate against JSON Schema - this is mandatory
    validate_against_schema(yaml)?;

    let mut model = ParsedModel::new();

    // Parse each top-level key as either a table or scalar
    if let Value::Mapping(map) = yaml {
        for (key, value) in map {
            let key_str = key
                .as_str()
                .ok_or_else(|| ForgeError::Parse("Table name must be a string".to_string()))?;

            // Skip special keys
            if key_str == "_forge_version" || key_str == "_name" || key_str == "monte_carlo" {
                continue;
            }

            // Parse _includes section (v4.0 cross-file references)
            if key_str == "_includes" {
                if let Value::Sequence(includes_seq) = value {
                    parse_includes(includes_seq, &mut model)?;
                }
                continue;
            }

            // Parse scenarios section - but only if it looks like scenario overrides
            // (mapping of scenario_name -> {variable: value}), not a table (mapping of column_name -> array)
            if key_str == "scenarios" {
                if let Value::Mapping(scenarios_map) = value {
                    // Check if this is actually a scenarios section or a table named "scenarios"
                    // Scenarios section has nested mappings with numeric values
                    // Tables have arrays (sequences) as column values
                    let is_scenarios_section = scenarios_map
                        .iter()
                        .all(|(_, v)| matches!(v, Value::Mapping(_)))
                        && scenarios_map.iter().any(|(_, v)| {
                            if let Value::Mapping(m) = v {
                                m.iter().any(|(_, vv)| matches!(vv, Value::Number(_)))
                            } else {
                                false
                            }
                        });

                    if is_scenarios_section {
                        parse_scenarios(scenarios_map, &mut model)?;
                        continue;
                    }
                    // Otherwise fall through to parse as table
                }
            }

            // Check if this is a table (mapping with arrays) or scalar (mapping with value/formula)
            if let Value::Mapping(inner_map) = value {
                // Check if it has {value, formula} pattern (scalar)
                if inner_map.contains_key("value") || inner_map.contains_key("formula") {
                    // This is a scalar variable
                    let variable = parse_scalar_variable(value, key_str)?;
                    model.add_scalar(key_str.to_string(), variable);
                } else if is_nested_scalar_section(inner_map) {
                    // This is a section containing nested scalars (e.g., summary.total)
                    parse_nested_scalars(key_str, inner_map, &mut model)?;
                } else {
                    // This is a table - parse it
                    let table = parse_table(key_str, inner_map)?;
                    model.add_table(table);
                }
            }
        }
    }

    // Validate all tables
    for (name, table) in &model.tables {
        table
            .validate_lengths()
            .map_err(|e| ForgeError::Validation(format!("Table '{}': {}", name, e)))?;
    }

    Ok(model)
}

/// Validate YAML against the appropriate Forge JSON Schema based on _forge_version
fn validate_against_schema(yaml: &Value) -> ForgeResult<()> {
    // Extract the _forge_version to determine which schema to use
    let version = yaml
        .get("_forge_version")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            ForgeError::Validation(
                "Missing required field: _forge_version. Must be \"1.0.0\" or \"5.0.0\""
                    .to_string(),
            )
        })?;

    // Load the appropriate schema based on version
    let schema_str = match version {
        "1.0.0" => include_str!("../../schema/forge-v1.0.0.schema.json"),
        "5.0.0" => include_str!("../../schema/forge-v5.0.0.schema.json"),
        _ => {
            return Err(ForgeError::Validation(format!(
                "Unsupported _forge_version: '{}'. Supported versions: 1.0.0 (scalar-only for forge-demo), 5.0.0 (arrays/tables for enterprise)",
                version
            )));
        }
    };

    let schema_value: serde_json::Value = serde_json::from_str(schema_str)
        .map_err(|e| ForgeError::Validation(format!("Failed to parse schema: {}", e)))?;

    // Convert YAML to JSON for validation
    let json_value: serde_json::Value = serde_json::to_value(yaml)
        .map_err(|e| ForgeError::Validation(format!("Failed to convert YAML to JSON: {}", e)))?;

    // Build the validator
    let validator = jsonschema::validator_for(&schema_value)
        .map_err(|e| ForgeError::Validation(format!("Failed to compile schema: {}", e)))?;

    // Validate
    if let Err(_error) = validator.validate(&json_value) {
        let error_messages: Vec<String> = validator
            .iter_errors(&json_value)
            .map(|e| format!("  - {}", e))
            .collect();
        return Err(ForgeError::Validation(format!(
            "Schema validation failed:\n{}",
            error_messages.join("\n")
        )));
    }

    // Additional runtime check for v1.0.0: NO tables/arrays allowed
    if version == "1.0.0" {
        validate_v1_0_0_no_tables(yaml)?;
    }

    Ok(())
}

/// Runtime validation: v1.0.0 models must NOT contain tables (arrays)
/// This provides a clear error message when users try to use enterprise features in forge-demo
fn validate_v1_0_0_no_tables(yaml: &Value) -> ForgeResult<()> {
    if let Value::Mapping(map) = yaml {
        for (key, value) in map {
            let key_str = key.as_str().unwrap_or("");

            // Skip special keys (but error on enterprise features in v1.0.0)
            if key_str == "_forge_version" || key_str == "_name" || key_str == "scenarios" {
                continue;
            }

            // Block monte_carlo in v1.0.0 (enterprise feature)
            if key_str == "monte_carlo" {
                return Err(ForgeError::Validation(
                    "monte_carlo requires Forge Enterprise (v5.0.0+). \
                     This feature is not available in forge-demo. \
                     Upgrade to _forge_version: \"5.0.0\" to use Monte Carlo simulation."
                        .to_string(),
                ));
            }

            // Check if this is a table (mapping with arrays)
            if let Value::Mapping(inner_map) = value {
                // Skip if this is a scalar (has value/formula keys)
                if inner_map.contains_key("value") || inner_map.contains_key("formula") {
                    continue;
                }

                // Check if any child contains arrays (indicates a table)
                for (col_key, col_value) in inner_map {
                    let col_key_str = col_key.as_str().unwrap_or("");

                    // Check for direct array values (table columns)
                    if matches!(col_value, Value::Sequence(_)) {
                        return Err(ForgeError::Validation(format!(
                            "v1.0.0 models do not support tables/arrays. Found table '{}' with array column '{}'.\n\
                            \n\
                            v1.0.0 is for forge-demo and only supports scalar values.\n\
                            To use tables/arrays, upgrade to v5.0.0 (enterprise):\n\
                            \n\
                            _forge_version: \"5.0.0\"\n\
                            \n\
                            Or convert your table to scalars using dot notation:\n\
                            {}.{}: {{ value: ..., formula: null }}\n\
                            {}.{}: {{ value: ..., formula: null }}",
                            key_str,
                            col_key_str,
                            key_str,
                            col_key_str,
                            key_str,
                            col_key_str
                        )));
                    }

                    // Check for rich column format with array value
                    if let Value::Mapping(col_map) = col_value {
                        if let Some(Value::Sequence(_)) = col_map.get("value") {
                            return Err(ForgeError::Validation(format!(
                                "v1.0.0 models do not support tables/arrays. Found table '{}' with array column '{}' (rich format).\n\
                                \n\
                                v1.0.0 is for forge-demo and only supports scalar values.\n\
                                To use tables/arrays, upgrade to v5.0.0 (enterprise):\n\
                                \n\
                                _forge_version: \"5.0.0\"",
                                key_str,
                                col_key_str
                            )));
                        }
                    }

                    // Check for row formulas (string starting with =)
                    if let Value::String(s) = col_value {
                        if s.starts_with('=') {
                            return Err(ForgeError::Validation(format!(
                                "v1.0.0 models do not support tables/arrays. Found table '{}' with formula column '{}'.\n\
                                \n\
                                v1.0.0 is for forge-demo and only supports scalar values.\n\
                                To use tables/arrays, upgrade to v5.0.0 (enterprise):\n\
                                \n\
                                _forge_version: \"5.0.0\"",
                                key_str,
                                col_key_str
                            )));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Check if a mapping contains nested scalar sections (e.g., summary.total)
/// Returns false for v4.0 rich table columns (where value is an array)
fn is_nested_scalar_section(map: &serde_yaml_ng::Mapping) -> bool {
    // Check if children are mappings with {value, formula} pattern where value is a scalar (not array)
    for (_key, value) in map {
        if let Value::Mapping(child_map) = value {
            // Check if this child has value or formula keys
            if child_map.contains_key("value") || child_map.contains_key("formula") {
                // If value is an array, this is a v4.0 rich table column, not a scalar
                if let Some(val) = child_map.get("value") {
                    if matches!(val, Value::Sequence(_)) {
                        return false; // v4.0 rich table column
                    }
                }
                return true; // Nested scalar section
            }
        }
    }
    false
}

/// Parse nested scalar variables (e.g., summary.total, summary.average)
fn parse_nested_scalars(
    parent_key: &str,
    map: &serde_yaml_ng::Mapping,
    model: &mut ParsedModel,
) -> ForgeResult<()> {
    for (key, value) in map {
        let key_str = key
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Scalar name must be a string".to_string()))?;

        if let Value::Mapping(child_map) = value {
            if child_map.contains_key("value") || child_map.contains_key("formula") {
                // This is a scalar variable
                let full_path = format!("{}.{}", parent_key, key_str);
                let variable = parse_scalar_variable(value, &full_path)?;
                model.add_scalar(full_path.clone(), variable);
            }
        }
    }
    Ok(())
}

/// Parse a table from a YAML mapping (v4.0 enhanced with metadata)
fn parse_table(name: &str, map: &serde_yaml_ng::Mapping) -> ForgeResult<Table> {
    let mut table = Table::new(name.to_string());

    for (key, value) in map {
        let col_name = key
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Column name must be a string".to_string()))?;

        // Skip _metadata table-level metadata (v4.0)
        if col_name == "_metadata" {
            continue;
        }

        // Check if this is a formula (string starting with =)
        if let Value::String(s) = value {
            if s.starts_with('=') {
                // This is a row-wise formula
                table.add_row_formula(col_name.to_string(), s.clone());
                continue;
            }
        }

        // Check for v4.0 rich column format: { value: [...], unit: "...", notes: "..." }
        if let Value::Mapping(col_map) = value {
            // Check if it has a 'value' key with an array (v4.0 rich format)
            if let Some(Value::Sequence(seq)) = col_map.get("value") {
                let column_value = parse_array_value(col_name, seq)?;
                let metadata = parse_metadata(col_map);
                let column = Column::with_metadata(col_name.to_string(), column_value, metadata);
                table.add_column(column);
                continue;
            }
            // Check if it has a 'formula' key (v4.0 rich formula format)
            if let Some(formula_val) = col_map.get("formula") {
                if let Some(formula_str) = formula_val.as_str() {
                    if formula_str.starts_with('=') {
                        // This is a row-wise formula with metadata
                        // TODO: Store formula metadata when we add formula metadata support
                        table.add_row_formula(col_name.to_string(), formula_str.to_string());
                        continue;
                    }
                }
            }
        }

        // Otherwise, it's a simple data column (array) - v1.0 format
        if let Value::Sequence(seq) = value {
            let column_value = parse_array_value(col_name, seq)?;
            let column = Column::new(col_name.to_string(), column_value);
            table.add_column(column);
        } else {
            return Err(ForgeError::Parse(format!(
                "Column '{}' in table '{}' must be an array or formula",
                col_name, name
            )));
        }
    }

    Ok(table)
}

/// Parse a scalar variable (v4.0 enhanced with metadata)
fn parse_scalar_variable(value: &Value, path: &str) -> ForgeResult<Variable> {
    if let Value::Mapping(map) = value {
        let val = map.get("value").and_then(|v| v.as_f64());
        let formula = map
            .get("formula")
            .and_then(|f| f.as_str().map(std::string::ToString::to_string));

        // Extract v4.0 metadata fields
        let metadata = parse_metadata(map);

        Ok(Variable {
            path: path.to_string(),
            value: val,
            formula,
            metadata,
        })
    } else {
        Err(ForgeError::Parse(format!(
            "Expected mapping for scalar variable '{}'",
            path
        )))
    }
}

/// Extract metadata fields from a YAML mapping (v4.0)
fn parse_metadata(map: &serde_yaml_ng::Mapping) -> Metadata {
    Metadata {
        unit: map
            .get("unit")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        notes: map
            .get("notes")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        source: map
            .get("source")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        validation_status: map
            .get("validation_status")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
        last_updated: map
            .get("last_updated")
            .and_then(|v| v.as_str().map(std::string::ToString::to_string)),
    }
}

/// Parse scenarios section from YAML
///
/// Expected format:
/// ```yaml
/// scenarios:
///   base:
///     growth_rate: 0.05
///     churn_rate: 0.02
///   optimistic:
///     growth_rate: 0.12
///     churn_rate: 0.01
/// ```
fn parse_scenarios(
    scenarios_map: &serde_yaml_ng::Mapping,
    model: &mut ParsedModel,
) -> ForgeResult<()> {
    for (scenario_name, scenario_value) in scenarios_map {
        let name = scenario_name
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Scenario name must be a string".to_string()))?;

        if let Value::Mapping(overrides_map) = scenario_value {
            let mut scenario = Scenario::new();

            for (var_name, var_value) in overrides_map {
                let var_name_str = var_name.as_str().ok_or_else(|| {
                    ForgeError::Parse("Variable name must be a string".to_string())
                })?;

                let value = match var_value {
                    Value::Number(n) => n.as_f64().ok_or_else(|| {
                        ForgeError::Parse(format!(
                            "Scenario '{}': Variable '{}' must be a number",
                            name, var_name_str
                        ))
                    })?,
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Scenario '{}': Variable '{}' must be a number",
                            name, var_name_str
                        )));
                    }
                };

                scenario.add_override(var_name_str.to_string(), value);
            }

            model.add_scenario(name.to_string(), scenario);
        } else {
            return Err(ForgeError::Parse(format!(
                "Scenario '{}' must be a mapping of variable overrides",
                name
            )));
        }
    }

    Ok(())
}

/// Parse _includes section from YAML (v4.0 cross-file references)
///
/// Expected format:
/// ```yaml
/// _includes:
///   - file: "data_sources.yaml"
///     as: "sources"
///   - file: "pricing.yaml"
///     as: "pricing"
/// ```
fn parse_includes(includes_seq: &[Value], model: &mut ParsedModel) -> ForgeResult<()> {
    for include_val in includes_seq {
        if let Value::Mapping(include_map) = include_val {
            // Extract 'file' field (required)
            let file = include_map
                .get("file")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ForgeError::Parse("Include must have a 'file' field".to_string()))?
                .to_string();

            // Extract 'as' field (required - the namespace alias)
            let namespace = include_map
                .get("as")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ForgeError::Parse(format!(
                        "Include '{}' must have an 'as' field for the namespace",
                        file
                    ))
                })?
                .to_string();

            model.add_include(Include::new(file, namespace));
        } else {
            return Err(ForgeError::Parse(
                "Each include must be a mapping with 'file' and 'as' fields".to_string(),
            ));
        }
    }
    Ok(())
}

/// Parse a YAML array into a typed ColumnValue
fn parse_array_value(col_name: &str, seq: &[Value]) -> ForgeResult<ColumnValue> {
    if seq.is_empty() {
        return Err(ForgeError::Parse(format!(
            "Column '{}' cannot be empty",
            col_name
        )));
    }

    // Detect the type from the first element
    let array_type = detect_array_type(&seq[0])?;

    match array_type {
        "Number" => {
            let mut numbers = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::Number(n) => {
                        if let Some(f) = n.as_f64() {
                            numbers.push(f);
                        } else {
                            return Err(ForgeError::Parse(format!(
                                "Column '{}' row {}: Invalid number format",
                                col_name, i
                            )));
                        }
                    }
                    Value::Null => {
                        // Provide clear error for null values in numeric arrays
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: null values not allowed in numeric arrays. \
                            Use 0 or remove the row if the value is missing.",
                            col_name, i
                        )));
                    }
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Number, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    }
                }
            }
            Ok(ColumnValue::Number(numbers))
        }
        "Text" => {
            let mut texts = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::String(s) => texts.push(s.clone()),
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Text, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    }
                }
            }
            Ok(ColumnValue::Text(texts))
        }
        "Date" => {
            let mut dates = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::String(s) => {
                        // Validate date format (YYYY-MM or YYYY-MM-DD)
                        if !is_valid_date_format(s) {
                            return Err(ForgeError::Parse(format!(
                                "Column '{}' row {}: Invalid date format '{}' (expected YYYY-MM or YYYY-MM-DD)",
                                col_name, i, s
                            )));
                        }
                        dates.push(s.clone());
                    }
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Date, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    }
                }
            }
            Ok(ColumnValue::Date(dates))
        }
        "Boolean" => {
            let mut bools = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::Bool(b) => bools.push(*b),
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Boolean, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    }
                }
            }
            Ok(ColumnValue::Boolean(bools))
        }
        _ => Err(ForgeError::Parse(format!(
            "Column '{}': Unsupported array type '{}'",
            col_name, array_type
        ))),
    }
}

/// Detect the type of a YAML value
fn detect_array_type(val: &Value) -> ForgeResult<&'static str> {
    match val {
        Value::Number(_) => Ok("Number"),
        Value::String(s) => {
            // Check if it's a date string
            if is_valid_date_format(s) {
                Ok("Date")
            } else {
                Ok("Text")
            }
        }
        Value::Bool(_) => Ok("Boolean"),
        Value::Null => Err(ForgeError::Parse(
            "Array cannot start with null. First element must be a valid value to determine column type.".to_string()
        )),
        _ => Err(ForgeError::Parse(format!(
            "Unsupported array element type: {}",
            type_name(val)
        ))),
    }
}

/// Check if a string is a valid date format (YYYY-MM or YYYY-MM-DD)
fn is_valid_date_format(s: &str) -> bool {
    // YYYY-MM format
    if s.len() == 7 {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 2 {
            return parts[0].len() == 4
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].len() == 2
                && parts[1].chars().all(|c| c.is_ascii_digit());
        }
    }
    // YYYY-MM-DD format
    if s.len() == 10 {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 3 {
            return parts[0].len() == 4
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].len() == 2
                && parts[1].chars().all(|c| c.is_ascii_digit())
                && parts[2].len() == 2
                && parts[2].chars().all(|c| c.is_ascii_digit());
        }
    }
    false
}

/// Get the type name of a YAML value for error messages
fn type_name(val: &Value) -> &'static str {
    match val {
        Value::Null => "Null",
        Value::Bool(_) => "Boolean",
        Value::Number(_) => "Number",
        Value::String(_) => "String",
        Value::Sequence(_) => "Array",
        Value::Mapping(_) => "Mapping",
        Value::Tagged(_) => "Tagged",
    }
}

#[cfg(test)]
mod tests;
