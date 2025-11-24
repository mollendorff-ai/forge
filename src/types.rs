use serde::{Deserialize, Serialize};
use std::collections::HashMap;

//==============================================================================
// Forge v1.0.0 Array Model Types
//==============================================================================

/// Column value types (homogeneous arrays)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// A column in a table
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// A table with column arrays
#[derive(Debug, Clone, Serialize, Deserialize)]
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
// Scalar Variables (for aggregations and summary values)
//==============================================================================

/// A scalar variable with optional formula
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub path: String,
    pub value: Option<f64>,
    pub formula: Option<String>,
}

//==============================================================================
// Parsed Model
//==============================================================================

/// Parsed Forge model (v1.0.0 array format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedModel {
    /// Tables with column arrays
    pub tables: HashMap<String, Table>,

    /// Scalar variables (for summary values and aggregations)
    pub scalars: HashMap<String, Variable>,

    /// Aggregation formulas (formulas that reduce columns to scalars)
    pub aggregations: HashMap<String, String>,
}

impl ParsedModel {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            scalars: HashMap::new(),
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

impl Default for ParsedModel {
    fn default() -> Self {
        Self::new()
    }
}
