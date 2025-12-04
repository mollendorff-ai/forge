use crate::error::{ForgeError, ForgeResult};
use crate::types::{
    Column, ColumnValue, Include, Metadata, ParsedModel, ResolvedInclude, Scenario, Table, Variable,
};
use jsonschema::JSONSchema;
use serde_yaml::Value;
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

    // Handle multi-document YAML files (leading ---)
    // Forge supports single-document files, so we take the first document
    // Strip leading document marker if present
    let content = content.trim_start();
    let content = if let Some(remaining) = content.strip_prefix("---") {
        // Find the end of the first document (next --- or end of file)
        let remaining = remaining.trim_start();
        if let Some(next_doc) = remaining.find("\n---") {
            // Multiple documents - take only the first one
            &remaining[..next_doc]
        } else {
            // Single document with leading ---
            remaining
        }
    } else {
        content
    };

    let yaml: Value = serde_yaml::from_str(content)?;

    let mut model = parse_v1_model(&yaml)?;

    // Resolve includes if any (v4.0)
    if !model.includes.is_empty() {
        resolve_includes(&mut model, path, &mut HashSet::new())?;
    }

    Ok(model)
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
        let yaml: Value = serde_yaml::from_str(&content)?;
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
            if key_str == "_forge_version" {
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

/// Validate YAML against the Forge v1.0.0 JSON Schema
fn validate_against_schema(yaml: &Value) -> ForgeResult<()> {
    // Load the JSON Schema from the embedded schema file
    let schema_str = include_str!("../../schema/forge-v1.0.schema.json");
    let schema_value: serde_json::Value = serde_json::from_str(schema_str)
        .map_err(|e| ForgeError::Validation(format!("Failed to parse schema: {}", e)))?;

    // Compile the schema
    let compiled_schema = JSONSchema::compile(&schema_value)
        .map_err(|e| ForgeError::Validation(format!("Failed to compile schema: {}", e)))?;

    // Convert YAML to JSON for validation
    let json_value: serde_json::Value = serde_json::to_value(yaml)
        .map_err(|e| ForgeError::Validation(format!("Failed to convert YAML to JSON: {}", e)))?;

    // Validate
    if let Err(errors) = compiled_schema.validate(&json_value) {
        let error_messages: Vec<String> = errors.map(|e| format!("  - {}", e)).collect();
        return Err(ForgeError::Validation(format!(
            "Schema validation failed:\n{}",
            error_messages.join("\n")
        )));
    }

    Ok(())
}

/// Check if a mapping contains nested scalar sections (e.g., summary.total)
/// Returns false for v4.0 rich table columns (where value is an array)
fn is_nested_scalar_section(map: &serde_yaml::Mapping) -> bool {
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
    map: &serde_yaml::Mapping,
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
fn parse_table(name: &str, map: &serde_yaml::Mapping) -> ForgeResult<Table> {
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
fn parse_metadata(map: &serde_yaml::Mapping) -> Metadata {
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
    scenarios_map: &serde_yaml::Mapping,
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
mod tests {
    use super::*;

    // =========================================================================
    // v1.0.0 Array Model Tests
    // =========================================================================

    #[test]
    fn test_parse_number_array() {
        let yaml_seq: Vec<Value> = vec![
            Value::Number(1.into()),
            Value::Number(2.into()),
            Value::Number(3.into()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums, vec![1.0, 2.0, 3.0]);
            }
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_parse_text_array() {
        let yaml_seq: Vec<Value> = vec![
            Value::String("A".to_string()),
            Value::String("B".to_string()),
            Value::String("C".to_string()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Text(texts) => {
                assert_eq!(texts, vec!["A", "B", "C"]);
            }
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_parse_date_array() {
        let yaml_seq: Vec<Value> = vec![
            Value::String("2025-01".to_string()),
            Value::String("2025-02".to_string()),
            Value::String("2025-03".to_string()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Date(dates) => {
                assert_eq!(dates, vec!["2025-01", "2025-02", "2025-03"]);
            }
            _ => panic!("Expected Date array"),
        }
    }

    #[test]
    fn test_parse_boolean_array() {
        let yaml_seq: Vec<Value> = vec![Value::Bool(true), Value::Bool(false), Value::Bool(true)];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Boolean(bools) => {
                assert_eq!(bools, vec![true, false, true]);
            }
            _ => panic!("Expected Boolean array"),
        }
    }

    #[test]
    fn test_mixed_type_array_error() {
        let yaml_seq: Vec<Value> = vec![Value::Number(1.into()), Value::String("text".to_string())];
        let result = parse_array_value("test_col", &yaml_seq);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Expected Number, found String"));
    }

    #[test]
    fn test_empty_array_error() {
        let yaml_seq: Vec<Value> = vec![];
        let result = parse_array_value("test_col", &yaml_seq);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("cannot be empty"));
    }

    #[test]
    fn test_invalid_date_format_error() {
        // Mix valid date format with invalid - should fail
        let yaml_seq: Vec<Value> = vec![
            Value::String("2025-01".to_string()), // Valid date
            Value::String("2025-1".to_string()),  // Invalid: needs zero padding
        ];
        let result = parse_array_value("test_col", &yaml_seq);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid date format"));
    }

    #[test]
    fn test_valid_date_formats() {
        // YYYY-MM format
        assert!(is_valid_date_format("2025-01"));
        assert!(is_valid_date_format("2025-12"));

        // YYYY-MM-DD format
        assert!(is_valid_date_format("2025-01-15"));
        assert!(is_valid_date_format("2025-12-31"));

        // Invalid formats
        assert!(!is_valid_date_format("2025-1"));
        assert!(!is_valid_date_format("2025-1-1"));
        assert!(!is_valid_date_format("25-01-01"));
        assert!(!is_valid_date_format("not-a-date"));
    }

    #[test]
    fn test_parse_table_with_arrays() {
        let yaml = r"
        month: ['Jan', 'Feb', 'Mar']
        revenue: [100, 200, 300]
        ";
        let parsed: Value = serde_yaml::from_str(yaml).unwrap();

        if let Value::Mapping(map) = parsed {
            let table = parse_table("test_table", &map).unwrap();

            assert_eq!(table.name, "test_table");
            assert_eq!(table.columns.len(), 2);
            assert!(table.columns.contains_key("month"));
            assert!(table.columns.contains_key("revenue"));
            assert_eq!(table.row_count(), 3);
        } else {
            panic!("Expected mapping");
        }
    }

    #[test]
    fn test_parse_table_with_formula() {
        let yaml = r"
        revenue: [100, 200, 300]
        expenses: [50, 100, 150]
        profit: '=revenue - expenses'
        ";
        let parsed: Value = serde_yaml::from_str(yaml).unwrap();

        if let Value::Mapping(map) = parsed {
            let table = parse_table("test_table", &map).unwrap();

            assert_eq!(table.columns.len(), 2); // Only data columns
            assert_eq!(table.row_formulas.len(), 1); // One formula
            assert!(table.row_formulas.contains_key("profit"));
            assert_eq!(
                table.row_formulas.get("profit").unwrap(),
                "=revenue - expenses"
            );
        } else {
            panic!("Expected mapping");
        }
    }

    #[test]
    fn test_table_validate_lengths_ok() {
        let mut table = Table::new("test".to_string());
        table.add_column(Column::new(
            "col1".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "col2".to_string(),
            ColumnValue::Number(vec![4.0, 5.0, 6.0]),
        ));

        assert!(table.validate_lengths().is_ok());
    }

    #[test]
    fn test_table_validate_lengths_error() {
        let mut table = Table::new("test".to_string());
        table.add_column(Column::new(
            "col1".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "col2".to_string(),
            ColumnValue::Number(vec![4.0, 5.0]), // Different length!
        ));

        let result = table.validate_lengths();
        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        // HashMap iteration order is not guaranteed, so error could mention either column
        assert!(err_msg.contains("col1") || err_msg.contains("col2"));
        assert!(err_msg.contains("2 rows"));
        assert!(err_msg.contains("3 rows"));
    }

    #[test]
    fn test_parse_v1_model_simple() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml_content = r#"
_forge_version: "1.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
  profit: "=revenue * 0.2"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert!(result.tables.contains_key("sales"));

        let sales_table = result.tables.get("sales").unwrap();
        assert_eq!(sales_table.columns.len(), 2); // month, revenue
        assert_eq!(sales_table.row_formulas.len(), 1); // profit
    }

    #[test]
    fn test_parse_v1_model_with_scalars() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml_content = r#"
_forge_version: "1.0.0"

data:
  values: [1, 2, 3]

summary:
  total:
    value: null
    formula: "=SUM(data.values)"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.scalars.len(), 1);
        assert!(result.scalars.contains_key("summary.total"));
    }

    #[test]
    fn test_column_value_type_name() {
        let num_col = ColumnValue::Number(vec![1.0]);
        let text_col = ColumnValue::Text(vec!["A".to_string()]);
        let date_col = ColumnValue::Date(vec!["2025-01".to_string()]);
        let bool_col = ColumnValue::Boolean(vec![true]);

        assert_eq!(num_col.type_name(), "Number");
        assert_eq!(text_col.type_name(), "Text");
        assert_eq!(date_col.type_name(), "Date");
        assert_eq!(bool_col.type_name(), "Boolean");
    }

    #[test]
    fn test_column_value_len() {
        let col = ColumnValue::Number(vec![1.0, 2.0, 3.0]);
        assert_eq!(col.len(), 3);
        assert!(!col.is_empty());

        let empty_col = ColumnValue::Number(vec![]);
        assert_eq!(empty_col.len(), 0);
        assert!(empty_col.is_empty());
    }

    #[test]
    fn test_parse_scenarios() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml_content = r#"
_forge_version: "1.0.0"

growth_rate:
  value: 0.05
  formula: null

scenarios:
  base:
    growth_rate: 0.05
  optimistic:
    growth_rate: 0.12
  pessimistic:
    growth_rate: 0.02
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        // Check scenarios were parsed
        assert_eq!(result.scenarios.len(), 3);
        assert!(result.scenarios.contains_key("base"));
        assert!(result.scenarios.contains_key("optimistic"));
        assert!(result.scenarios.contains_key("pessimistic"));

        // Check override values
        let base = result.scenarios.get("base").unwrap();
        assert_eq!(base.overrides.get("growth_rate"), Some(&0.05));

        let optimistic = result.scenarios.get("optimistic").unwrap();
        assert_eq!(optimistic.overrides.get("growth_rate"), Some(&0.12));

        let pessimistic = result.scenarios.get("pessimistic").unwrap();
        assert_eq!(pessimistic.overrides.get("growth_rate"), Some(&0.02));
    }

    #[test]
    fn test_multi_document_yaml_with_leading_separator() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // YAML files with leading --- should parse correctly
        let yaml_content = r#"---
_forge_version: "1.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        let sales = result.tables.get("sales").unwrap();
        assert_eq!(sales.row_count(), 3);
    }

    #[test]
    fn test_null_in_numeric_array_error() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // [1000, null] should fail with a clear error message
        let yaml_content = r#"
_forge_version: "1.0.0"
data:
  values: [1000, null, 2000]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path());
        assert!(result.is_err());

        let err = result.unwrap_err().to_string();
        assert!(err.contains("null values not allowed"));
        assert!(err.contains("Use 0 or remove the row"));
    }

    #[test]
    fn test_null_first_element_error() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Array starting with null should fail with clear error
        let yaml_content = r#"
_forge_version: "1.0.0"
data:
  values: [null, 1000, 2000]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path());
        assert!(result.is_err());

        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot start with null"));
    }

    #[test]
    fn test_table_named_scenarios() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // A table named "scenarios" should be parsed as a table, not as scenario overrides
        let yaml_content = r#"
_forge_version: "1.0.0"

scenarios:
  name: ["Base", "Optimistic", "Pessimistic"]
  probability: [0.3, 0.5, 0.2]
  revenue: [100000, 150000, 80000]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        // Should be parsed as a table, not scenario overrides
        assert_eq!(result.scenarios.len(), 0);
        assert_eq!(result.tables.len(), 1);

        let scenarios_table = result.tables.get("scenarios").unwrap();
        assert_eq!(scenarios_table.columns.len(), 3);
        assert!(scenarios_table.columns.contains_key("name"));
        assert!(scenarios_table.columns.contains_key("probability"));
        assert!(scenarios_table.columns.contains_key("revenue"));
        assert_eq!(scenarios_table.row_count(), 3);
    }

    // =========================================================================
    // v4.0 Rich Metadata Tests
    // =========================================================================

    #[test]
    fn test_parse_v4_scalar_with_metadata() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml_content = r#"
_forge_version: "4.0.0"

price:
  value: 100
  formula: null
  unit: "CAD"
  notes: "Base price per unit"
  source: "market_research.yaml"
  validation_status: "VALIDATED"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.scalars.len(), 1);
        let price = result.scalars.get("price").unwrap();
        assert_eq!(price.value, Some(100.0));
        assert!(price.formula.is_none());
        assert_eq!(price.metadata.unit, Some("CAD".to_string()));
        assert_eq!(
            price.metadata.notes,
            Some("Base price per unit".to_string())
        );
        assert_eq!(
            price.metadata.source,
            Some("market_research.yaml".to_string())
        );
        assert_eq!(
            price.metadata.validation_status,
            Some("VALIDATED".to_string())
        );
    }

    #[test]
    fn test_parse_v4_table_column_with_metadata() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let yaml_content = r#"
_forge_version: "4.0.0"

sales:
  month:
    value: ["Jan", "Feb", "Mar"]
    unit: "month"
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
    notes: "Monthly revenue projection"
    validation_status: "PROJECTED"
  profit:
    formula: "=revenue * 0.3"
    unit: "CAD"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        let sales = result.tables.get("sales").unwrap();

        // Check month column metadata
        let month = sales.columns.get("month").unwrap();
        assert_eq!(month.metadata.unit, Some("month".to_string()));

        // Check revenue column metadata
        let revenue = sales.columns.get("revenue").unwrap();
        assert_eq!(revenue.metadata.unit, Some("CAD".to_string()));
        assert_eq!(
            revenue.metadata.notes,
            Some("Monthly revenue projection".to_string())
        );
        assert_eq!(
            revenue.metadata.validation_status,
            Some("PROJECTED".to_string())
        );

        // Check profit formula was parsed
        assert!(sales.row_formulas.contains_key("profit"));
        assert_eq!(sales.row_formulas.get("profit").unwrap(), "=revenue * 0.3");
    }

    #[test]
    fn test_parse_v4_backward_compatible_with_v1() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // v1.0 format should still work
        let yaml_content = r#"
_forge_version: "1.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
  profit: "=revenue * 0.3"

summary:
  total:
    value: null
    formula: "=SUM(sales.revenue)"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        // Tables should parse as before
        assert_eq!(result.tables.len(), 1);
        let sales = result.tables.get("sales").unwrap();
        assert_eq!(sales.columns.len(), 2); // month, revenue
        assert_eq!(sales.row_formulas.len(), 1); // profit

        // Scalars should parse as before
        assert_eq!(result.scalars.len(), 1);
        let total = result.scalars.get("summary.total").unwrap();
        assert_eq!(total.formula, Some("=SUM(sales.revenue)".to_string()));

        // Metadata should be empty for v1.0 models
        assert!(sales.columns.get("revenue").unwrap().metadata.is_empty());
        assert!(total.metadata.is_empty());
    }

    #[test]
    fn test_parse_v4_mixed_formats() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Mix of v1.0 simple and v4.0 rich formats
        let yaml_content = r#"
_forge_version: "4.0.0"
sales:
  month: ["Jan", "Feb", "Mar"]
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
    notes: "Rich format column"
  expenses: [50, 100, 150]
  profit: "=revenue - expenses"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        let sales = result.tables.get("sales").unwrap();

        // month and expenses should have no metadata
        assert!(sales.columns.get("month").unwrap().metadata.is_empty());
        assert!(sales.columns.get("expenses").unwrap().metadata.is_empty());

        // revenue should have rich metadata
        let revenue = sales.columns.get("revenue").unwrap();
        assert_eq!(revenue.metadata.unit, Some("CAD".to_string()));
        assert_eq!(
            revenue.metadata.notes,
            Some("Rich format column".to_string())
        );
    }
}
