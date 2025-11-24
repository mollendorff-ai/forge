use serde::{Deserialize, Serialize};
use std::collections::HashMap;

//==============================================================================
// Forge Version Detection
//==============================================================================

/// Forge model version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeVersion {
    /// v0.2.0 - Scalar model (discrete values with {value, formula})
    V0_2_0,
    /// v1.0.0 - Array model (column arrays for Excel compatibility)
    V1_0_0,
}

impl ForgeVersion {
    /// Detect version from YAML content
    pub fn detect(yaml: &serde_yaml::Value) -> Self {
        // Check for explicit version marker
        if let Some(version_val) = yaml.get("_forge_version") {
            if let Some(version_str) = version_val.as_str() {
                if version_str.starts_with("1.0") {
                    return ForgeVersion::V1_0_0;
                }
            }
        }

        // Check for v0.2.0 specific features FIRST
        // includes: field indicates v0.2.0 cross-file references
        if yaml.get("includes").is_some() {
            return ForgeVersion::V0_2_0;
        }

        // Check for v1.0.0 specific features
        // tables: field with columns indicates v1.0.0 array model
        if let Some(tables_val) = yaml.get("tables") {
            if let Some(tables_map) = tables_val.as_mapping() {
                // Check if any table has a "columns" field
                for (_table_name, table_val) in tables_map {
                    if let Some(table_map) = table_val.as_mapping() {
                        if table_map.contains_key("columns") {
                            return ForgeVersion::V1_0_0;
                        }
                    }
                }
            }
        }

        // Fallback: Check for array pattern (v1.0.0 indicator)
        // This catches v1.0.0 files that use scalars with arrays
        if Self::has_array_values(yaml) {
            return ForgeVersion::V1_0_0;
        }

        // Default to v0.2.0 for backwards compatibility
        ForgeVersion::V0_2_0
    }

    /// Check if YAML contains array values (v1.0.0 pattern)
    fn has_array_values(yaml: &serde_yaml::Value) -> bool {
        match yaml {
            serde_yaml::Value::Mapping(map) => {
                for (_key, value) in map {
                    if value.is_sequence() {
                        return true;
                    }
                    if Self::has_array_values(value) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}

//==============================================================================
// v1.0.0 Array Model Types
//==============================================================================

/// Column value types (homogeneous arrays)
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnValue {
    /// Array of numbers (f64)
    Number(Vec<f64>),
    /// Array of text strings
    Text(Vec<String>),
    /// Array of ISO date strings (YYYY-MM or YYYY-MM-DD)
    Date(Vec<String>),
    /// Array of booleans
    Boolean(Vec<bool>),
}

impl ColumnValue {
    /// Get the length of the array
    pub fn len(&self) -> usize {
        match self {
            ColumnValue::Number(v) => v.len(),
            ColumnValue::Text(v) => v.len(),
            ColumnValue::Date(v) => v.len(),
            ColumnValue::Boolean(v) => v.len(),
        }
    }

    /// Check if array is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the type name as a string
    pub fn type_name(&self) -> &'static str {
        match self {
            ColumnValue::Number(_) => "Number",
            ColumnValue::Text(_) => "Text",
            ColumnValue::Date(_) => "Date",
            ColumnValue::Boolean(_) => "Boolean",
        }
    }
}

/// A column in a table (v1.0.0)
#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub values: ColumnValue,
}

impl Column {
    pub fn new(name: String, values: ColumnValue) -> Self {
        Self { name, values }
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// A table with column arrays (v1.0.0)
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub columns: HashMap<String, Column>,
    /// Row-wise formulas (e.g., "profit: =revenue - expenses")
    pub row_formulas: HashMap<String, String>,
}

impl Table {
    pub fn new(name: String) -> Self {
        Self {
            name,
            columns: HashMap::new(),
            row_formulas: HashMap::new(),
        }
    }

    pub fn add_column(&mut self, column: Column) {
        self.columns.insert(column.name.clone(), column);
    }

    pub fn add_row_formula(&mut self, name: String, formula: String) {
        self.row_formulas.insert(name, formula);
    }

    /// Get the number of rows (length of first column, all should be same)
    pub fn row_count(&self) -> usize {
        self.columns.values().next().map_or(0, |col| col.len())
    }

    /// Validate all columns have the same length
    pub fn validate_lengths(&self) -> Result<(), String> {
        let row_count = self.row_count();
        for (name, column) in &self.columns {
            if column.len() != row_count {
                return Err(format!(
                    "Column '{}' has {} rows, expected {} rows",
                    name,
                    column.len(),
                    row_count
                ));
            }
        }
        Ok(())
    }
}

//==============================================================================
// v0.2.0 Scalar Model Types (Backwards Compatible)
//==============================================================================

/// Represents an included YAML file with an alias
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Include {
    pub file: String,
    pub r#as: String,
}

/// A variable in the YAML file
#[derive(Debug, Clone)]
pub struct Variable {
    pub path: String,
    pub value: Option<f64>,
    pub formula: Option<String>,
    /// The alias of the file this variable came from (None for main file)
    pub alias: Option<String>,
}

/// Parsed YAML data with includes (v0.2.0 - kept for backwards compatibility)
#[derive(Debug)]
pub struct ParsedYaml {
    pub includes: Vec<Include>,
    pub variables: HashMap<String, Variable>,
}

//==============================================================================
// Unified Model (v0.2.0 + v1.0.0)
//==============================================================================

/// Parsed model that supports both v0.2.0 and v1.0.0 formats
#[derive(Debug)]
pub struct ParsedModel {
    /// Model version detected
    pub version: ForgeVersion,

    /// Tables (v1.0.0 only)
    pub tables: HashMap<String, Table>,

    /// Scalar variables (v0.2.0 compatible, also used in v1.0.0 for summary values)
    pub scalars: HashMap<String, Variable>,

    /// Includes (both versions)
    pub includes: Vec<Include>,

    /// Aggregation formulas (v1.0.0 - formulas that reduce columns to scalars)
    pub aggregations: HashMap<String, String>,
}

impl ParsedModel {
    pub fn new(version: ForgeVersion) -> Self {
        Self {
            version,
            tables: HashMap::new(),
            scalars: HashMap::new(),
            includes: Vec::new(),
            aggregations: HashMap::new(),
        }
    }

    pub fn add_table(&mut self, table: Table) {
        self.tables.insert(table.name.clone(), table);
    }

    pub fn add_scalar(&mut self, name: String, variable: Variable) {
        self.scalars.insert(name, variable);
    }

    pub fn add_aggregation(&mut self, name: String, formula: String) {
        self.aggregations.insert(name, formula);
    }
}

/// Context for formula evaluation
pub struct EvalContext {
    pub variables: HashMap<String, f64>,
}

impl Default for EvalContext {
    fn default() -> Self {
        Self::new()
    }
}

impl EvalContext {
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn set(&mut self, name: String, value: f64) {
        self.variables.insert(name, value);
    }
}
