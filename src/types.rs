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

/// A column in a table (v4.0 enhanced with metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub values: ColumnValue,
    /// Rich metadata (v4.0) - unit, notes, source, validation_status
    #[serde(default)]
    pub metadata: Metadata,
}

impl Column {
    pub fn new(name: String, values: ColumnValue) -> Self {
        Self {
            name,
            values,
            metadata: Metadata::default(),
        }
    }

    pub fn with_metadata(name: String, values: ColumnValue, metadata: Metadata) -> Self {
        Self {
            name,
            values,
            metadata,
        }
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

/// Rich metadata for enterprise financial models (v4.0)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    /// Unit of measurement (CAD, USD, %, count, days, ratio)
    pub unit: Option<String>,
    /// Human-readable explanation
    pub notes: Option<String>,
    /// Source reference (file:field or URL)
    pub source: Option<String>,
    /// Validation status (VALIDATED, PROJECTED, ESTIMATED)
    pub validation_status: Option<String>,
    /// Last updated timestamp
    pub last_updated: Option<String>,
}

impl Metadata {
    pub fn is_empty(&self) -> bool {
        self.unit.is_none()
            && self.notes.is_none()
            && self.source.is_none()
            && self.validation_status.is_none()
            && self.last_updated.is_none()
    }
}

/// A scalar variable with optional formula and metadata (v4.0 enhanced)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub path: String,
    pub value: Option<f64>,
    pub formula: Option<String>,
    /// Rich metadata (v4.0) - unit, notes, source, validation_status
    #[serde(default)]
    pub metadata: Metadata,
}

impl Variable {
    /// Create a new variable with default (empty) metadata
    pub fn new(path: String, value: Option<f64>, formula: Option<String>) -> Self {
        Self {
            path,
            value,
            formula,
            metadata: Metadata::default(),
        }
    }

    /// Create a new variable with metadata (v4.0)
    pub fn with_metadata(
        path: String,
        value: Option<f64>,
        formula: Option<String>,
        metadata: Metadata,
    ) -> Self {
        Self {
            path,
            value,
            formula,
            metadata,
        }
    }
}

//==============================================================================
// Cross-File References (v4.0)
//==============================================================================

/// An include directive for cross-file references (v4.0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Include {
    /// Path to the included file (relative to current file)
    pub file: String,
    /// Namespace alias for referencing (e.g., "sources" for @sources.field)
    pub namespace: String,
}

impl Include {
    pub fn new(file: String, namespace: String) -> Self {
        Self { file, namespace }
    }
}

/// Resolved include with parsed model data
#[derive(Debug, Clone)]
pub struct ResolvedInclude {
    /// The include directive
    pub include: Include,
    /// Resolved absolute path
    pub resolved_path: std::path::PathBuf,
    /// Parsed model from the included file
    pub model: ParsedModel,
}

//==============================================================================
// Scenarios (for multi-scenario modeling)
//==============================================================================

/// A named scenario with variable overrides
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Scenario {
    /// Variable overrides for this scenario (variable_name -> value)
    pub overrides: HashMap<String, f64>,
}

impl Scenario {
    pub fn new() -> Self {
        Self {
            overrides: HashMap::new(),
        }
    }

    pub fn add_override(&mut self, name: String, value: f64) {
        self.overrides.insert(name, value);
    }
}

//==============================================================================
// Parsed Model
//==============================================================================

/// Parsed Forge model (v4.0 with cross-file references)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedModel {
    /// Tables with column arrays
    pub tables: HashMap<String, Table>,

    /// Scalar variables (for summary values and aggregations)
    pub scalars: HashMap<String, Variable>,

    /// Aggregation formulas (formulas that reduce columns to scalars)
    pub aggregations: HashMap<String, String>,

    /// Named scenarios with variable overrides
    pub scenarios: HashMap<String, Scenario>,

    /// Cross-file includes (v4.0) - parsed but not yet resolved
    #[serde(default)]
    pub includes: Vec<Include>,

    /// Resolved includes with loaded models (populated after resolution)
    #[serde(skip)]
    pub resolved_includes: HashMap<String, ResolvedInclude>,

    /// Document names from multi-document YAML files (v4.4.2)
    /// Empty for single-document files
    #[serde(default)]
    pub documents: Vec<String>,
}

impl ParsedModel {
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            scalars: HashMap::new(),
            aggregations: HashMap::new(),
            scenarios: HashMap::new(),
            includes: Vec::new(),
            resolved_includes: HashMap::new(),
            documents: Vec::new(),
        }
    }

    /// Add an include directive (v4.0)
    pub fn add_include(&mut self, include: Include) {
        self.includes.push(include);
    }

    /// Check if model has unresolved includes
    pub fn has_unresolved_includes(&self) -> bool {
        !self.includes.is_empty() && self.resolved_includes.is_empty()
    }

    /// Get a value from a resolved include by namespace reference
    /// e.g., "@sources.pricing.unit_price" -> lookup in sources namespace
    pub fn resolve_namespace_ref(&self, reference: &str) -> Option<f64> {
        // Parse @namespace.path format
        if !reference.starts_with('@') {
            return None;
        }

        let ref_path = &reference[1..]; // Remove @
        let parts: Vec<&str> = ref_path.splitn(2, '.').collect();
        if parts.len() != 2 {
            return None;
        }

        let namespace = parts[0];
        let field_path = parts[1];

        // Look up in resolved includes
        if let Some(resolved) = self.resolved_includes.get(namespace) {
            // Try to find the value in the included model's scalars
            if let Some(var) = resolved.model.scalars.get(field_path) {
                return var.value;
            }
            // Try nested path (e.g., "pricing.unit_price" -> scalars["pricing.unit_price"])
            for (key, var) in &resolved.model.scalars {
                if key == field_path || key.ends_with(&format!(".{}", field_path)) {
                    return var.value;
                }
            }
        }

        None
    }

    pub fn add_scenario(&mut self, name: String, scenario: Scenario) {
        self.scenarios.insert(name, scenario);
    }

    /// Get available scenario names
    pub fn scenario_names(&self) -> Vec<&String> {
        self.scenarios.keys().collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Metadata Tests (v4.0)
    // =========================================================================

    #[test]
    fn test_metadata_default_is_empty() {
        let metadata = Metadata::default();
        assert!(metadata.is_empty());
        assert!(metadata.unit.is_none());
        assert!(metadata.notes.is_none());
        assert!(metadata.source.is_none());
        assert!(metadata.validation_status.is_none());
        assert!(metadata.last_updated.is_none());
    }

    #[test]
    fn test_metadata_with_unit_not_empty() {
        let metadata = Metadata {
            unit: Some("CAD".to_string()),
            ..Default::default()
        };
        assert!(!metadata.is_empty());
    }

    #[test]
    fn test_metadata_with_notes_not_empty() {
        let metadata = Metadata {
            notes: Some("Test notes".to_string()),
            ..Default::default()
        };
        assert!(!metadata.is_empty());
    }

    #[test]
    fn test_metadata_with_source_not_empty() {
        let metadata = Metadata {
            source: Some("data.yaml".to_string()),
            ..Default::default()
        };
        assert!(!metadata.is_empty());
    }

    #[test]
    fn test_metadata_with_validation_status_not_empty() {
        let metadata = Metadata {
            validation_status: Some("VALIDATED".to_string()),
            ..Default::default()
        };
        assert!(!metadata.is_empty());
    }

    #[test]
    fn test_metadata_with_last_updated_not_empty() {
        let metadata = Metadata {
            last_updated: Some("2025-11-26".to_string()),
            ..Default::default()
        };
        assert!(!metadata.is_empty());
    }

    #[test]
    fn test_metadata_full() {
        let metadata = Metadata {
            unit: Some("CAD".to_string()),
            notes: Some("Revenue projection".to_string()),
            source: Some("market_research.yaml".to_string()),
            validation_status: Some("PROJECTED".to_string()),
            last_updated: Some("2025-11-26".to_string()),
        };
        assert!(!metadata.is_empty());
        assert_eq!(metadata.unit, Some("CAD".to_string()));
        assert_eq!(metadata.notes, Some("Revenue projection".to_string()));
        assert_eq!(metadata.source, Some("market_research.yaml".to_string()));
        assert_eq!(metadata.validation_status, Some("PROJECTED".to_string()));
        assert_eq!(metadata.last_updated, Some("2025-11-26".to_string()));
    }

    // =========================================================================
    // Variable Tests (v4.0)
    // =========================================================================

    #[test]
    fn test_variable_new_with_value() {
        let var = Variable::new("price".to_string(), Some(100.0), None);
        assert_eq!(var.path, "price");
        assert_eq!(var.value, Some(100.0));
        assert!(var.formula.is_none());
        assert!(var.metadata.is_empty());
    }

    #[test]
    fn test_variable_new_with_formula() {
        let var = Variable::new(
            "profit".to_string(),
            None,
            Some("=revenue - costs".to_string()),
        );
        assert_eq!(var.path, "profit");
        assert!(var.value.is_none());
        assert_eq!(var.formula, Some("=revenue - costs".to_string()));
        assert!(var.metadata.is_empty());
    }

    #[test]
    fn test_variable_with_metadata() {
        let metadata = Metadata {
            unit: Some("CAD".to_string()),
            notes: Some("Total revenue".to_string()),
            ..Default::default()
        };
        let var = Variable::with_metadata("total".to_string(), Some(1000.0), None, metadata);
        assert_eq!(var.path, "total");
        assert_eq!(var.value, Some(1000.0));
        assert!(!var.metadata.is_empty());
        assert_eq!(var.metadata.unit, Some("CAD".to_string()));
    }

    // =========================================================================
    // Column Tests (v4.0)
    // =========================================================================

    #[test]
    fn test_column_new_no_metadata() {
        let col = Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        );
        assert_eq!(col.name, "revenue");
        assert_eq!(col.len(), 2);
        assert!(col.metadata.is_empty());
    }

    #[test]
    fn test_column_with_metadata() {
        let metadata = Metadata {
            unit: Some("CAD".to_string()),
            validation_status: Some("VALIDATED".to_string()),
            ..Default::default()
        };
        let col = Column::with_metadata(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
            metadata,
        );
        assert_eq!(col.name, "revenue");
        assert_eq!(col.len(), 3);
        assert!(!col.metadata.is_empty());
        assert_eq!(col.metadata.unit, Some("CAD".to_string()));
        assert_eq!(
            col.metadata.validation_status,
            Some("VALIDATED".to_string())
        );
    }

    #[test]
    fn test_column_value_types_with_metadata() {
        // Number column
        let num_col = Column::with_metadata(
            "amounts".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
            Metadata {
                unit: Some("%".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(num_col.values.type_name(), "Number");

        // Text column
        let text_col = Column::with_metadata(
            "labels".to_string(),
            ColumnValue::Text(vec!["A".to_string(), "B".to_string()]),
            Metadata {
                notes: Some("Labels".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(text_col.values.type_name(), "Text");

        // Date column
        let date_col = Column::with_metadata(
            "months".to_string(),
            ColumnValue::Date(vec!["2025-01".to_string()]),
            Metadata {
                unit: Some("month".to_string()),
                ..Default::default()
            },
        );
        assert_eq!(date_col.values.type_name(), "Date");

        // Boolean column
        let bool_col = Column::with_metadata(
            "flags".to_string(),
            ColumnValue::Boolean(vec![true, false]),
            Metadata::default(),
        );
        assert_eq!(bool_col.values.type_name(), "Boolean");
    }

    // =========================================================================
    // Table Tests
    // =========================================================================

    #[test]
    fn test_table_new() {
        let table = Table::new("sales".to_string());
        assert_eq!(table.name, "sales");
        assert!(table.columns.is_empty());
        assert!(table.row_formulas.is_empty());
    }

    #[test]
    fn test_table_add_column() {
        let mut table = Table::new("sales".to_string());
        let col = Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        );
        table.add_column(col);
        assert_eq!(table.columns.len(), 1);
        assert!(table.columns.contains_key("revenue"));
    }

    #[test]
    fn test_table_add_row_formula() {
        let mut table = Table::new("sales".to_string());
        table.add_row_formula("profit".to_string(), "=revenue - costs".to_string());
        assert_eq!(table.row_formulas.len(), 1);
        assert_eq!(
            table.row_formulas.get("profit"),
            Some(&"=revenue - costs".to_string())
        );
    }

    #[test]
    fn test_table_row_count_empty() {
        let table = Table::new("empty".to_string());
        assert_eq!(table.row_count(), 0);
    }

    #[test]
    fn test_table_row_count_with_data() {
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        assert_eq!(table.row_count(), 3);
    }

    #[test]
    fn test_column_is_empty() {
        let empty_col = Column::new("empty".to_string(), ColumnValue::Number(vec![]));
        assert!(empty_col.is_empty());

        let non_empty_col = Column::new("nums".to_string(), ColumnValue::Number(vec![1.0]));
        assert!(!non_empty_col.is_empty());
    }

    // =========================================================================
    // Include Tests
    // =========================================================================

    #[test]
    fn test_include_new() {
        let include = Include::new("data.yaml".to_string(), "data".to_string());
        assert_eq!(include.file, "data.yaml");
        assert_eq!(include.namespace, "data");
    }

    // =========================================================================
    // Scenario Tests
    // =========================================================================

    #[test]
    fn test_scenario_new() {
        let scenario = Scenario::new();
        assert!(scenario.overrides.is_empty());
    }

    #[test]
    fn test_scenario_default() {
        let scenario = Scenario::default();
        assert!(scenario.overrides.is_empty());
    }

    #[test]
    fn test_scenario_add_override() {
        let mut scenario = Scenario::new();
        scenario.add_override("growth_rate".to_string(), 0.15);
        assert_eq!(scenario.overrides.get("growth_rate"), Some(&0.15));
    }

    // =========================================================================
    // ParsedModel Tests
    // =========================================================================

    #[test]
    fn test_parsed_model_new() {
        let model = ParsedModel::new();
        assert!(model.tables.is_empty());
        assert!(model.scalars.is_empty());
        assert!(model.aggregations.is_empty());
        assert!(model.scenarios.is_empty());
        assert!(model.includes.is_empty());
        assert!(model.resolved_includes.is_empty());
        assert!(model.documents.is_empty());
    }

    #[test]
    fn test_parsed_model_default() {
        let model = ParsedModel::default();
        assert!(model.tables.is_empty());
    }

    #[test]
    fn test_parsed_model_add_table() {
        let mut model = ParsedModel::new();
        let table = Table::new("sales".to_string());
        model.add_table(table);
        assert!(model.tables.contains_key("sales"));
    }

    #[test]
    fn test_parsed_model_add_scalar() {
        let mut model = ParsedModel::new();
        let var = Variable::new("profit".to_string(), Some(100.0), None);
        model.add_scalar("profit".to_string(), var);
        assert!(model.scalars.contains_key("profit"));
    }

    #[test]
    fn test_parsed_model_add_aggregation() {
        let mut model = ParsedModel::new();
        model.add_aggregation("total".to_string(), "=SUM(sales.revenue)".to_string());
        assert_eq!(
            model.aggregations.get("total"),
            Some(&"=SUM(sales.revenue)".to_string())
        );
    }

    #[test]
    fn test_parsed_model_add_scenario() {
        let mut model = ParsedModel::new();
        let mut scenario = Scenario::new();
        scenario.add_override("rate".to_string(), 0.10);
        model.add_scenario("optimistic".to_string(), scenario);
        assert!(model.scenarios.contains_key("optimistic"));
    }

    #[test]
    fn test_parsed_model_scenario_names() {
        let mut model = ParsedModel::new();
        model.add_scenario("base".to_string(), Scenario::new());
        model.add_scenario("optimistic".to_string(), Scenario::new());
        let names = model.scenario_names();
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_parsed_model_add_include() {
        let mut model = ParsedModel::new();
        let include = Include::new("external.yaml".to_string(), "ext".to_string());
        model.add_include(include);
        assert_eq!(model.includes.len(), 1);
    }

    #[test]
    fn test_parsed_model_has_unresolved_includes_empty() {
        let model = ParsedModel::new();
        assert!(!model.has_unresolved_includes());
    }

    #[test]
    fn test_parsed_model_has_unresolved_includes_true() {
        let mut model = ParsedModel::new();
        model.add_include(Include::new("external.yaml".to_string(), "ext".to_string()));
        assert!(model.has_unresolved_includes());
    }

    #[test]
    fn test_parsed_model_has_unresolved_includes_resolved() {
        let mut model = ParsedModel::new();
        model.add_include(Include::new("external.yaml".to_string(), "ext".to_string()));

        // Add a resolved include
        let resolved = ResolvedInclude {
            include: Include::new("external.yaml".to_string(), "ext".to_string()),
            resolved_path: std::path::PathBuf::from("/tmp/external.yaml"),
            model: ParsedModel::new(),
        };
        model.resolved_includes.insert("ext".to_string(), resolved);

        assert!(!model.has_unresolved_includes());
    }

    #[test]
    fn test_parsed_model_resolve_namespace_ref_invalid_format() {
        let model = ParsedModel::new();

        // Missing @ prefix
        assert_eq!(model.resolve_namespace_ref("ext.value"), None);

        // No dot in path
        assert_eq!(model.resolve_namespace_ref("@ext"), None);
    }

    #[test]
    fn test_parsed_model_resolve_namespace_ref_not_found() {
        let model = ParsedModel::new();
        assert_eq!(model.resolve_namespace_ref("@ext.value"), None);
    }

    #[test]
    fn test_parsed_model_resolve_namespace_ref_found() {
        let mut model = ParsedModel::new();

        // Create an included model with a scalar
        let mut included_model = ParsedModel::new();
        let var = Variable::new("price".to_string(), Some(99.99), None);
        included_model.add_scalar("price".to_string(), var);

        let resolved = ResolvedInclude {
            include: Include::new("pricing.yaml".to_string(), "pricing".to_string()),
            resolved_path: std::path::PathBuf::from("/tmp/pricing.yaml"),
            model: included_model,
        };
        model
            .resolved_includes
            .insert("pricing".to_string(), resolved);

        assert_eq!(model.resolve_namespace_ref("@pricing.price"), Some(99.99));
    }

    #[test]
    fn test_parsed_model_resolve_namespace_ref_nested_path() {
        let mut model = ParsedModel::new();

        // Create an included model with a nested scalar path
        let mut included_model = ParsedModel::new();
        let var = Variable::new("products.item_price".to_string(), Some(50.0), None);
        included_model.add_scalar("products.item_price".to_string(), var);

        let resolved = ResolvedInclude {
            include: Include::new("data.yaml".to_string(), "data".to_string()),
            resolved_path: std::path::PathBuf::from("/tmp/data.yaml"),
            model: included_model,
        };
        model.resolved_includes.insert("data".to_string(), resolved);

        // Should find via ends_with match
        assert_eq!(model.resolve_namespace_ref("@data.item_price"), Some(50.0));
    }

    // =========================================================================
    // ColumnValue Tests
    // =========================================================================

    #[test]
    fn test_column_value_equality() {
        let a = ColumnValue::Number(vec![1.0, 2.0]);
        let b = ColumnValue::Number(vec![1.0, 2.0]);
        let c = ColumnValue::Number(vec![1.0, 3.0]);

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_column_value_text_equality() {
        let a = ColumnValue::Text(vec!["a".to_string(), "b".to_string()]);
        let b = ColumnValue::Text(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(a, b);
    }

    #[test]
    fn test_column_value_date_equality() {
        let a = ColumnValue::Date(vec!["2025-01".to_string()]);
        let b = ColumnValue::Date(vec!["2025-01".to_string()]);
        assert_eq!(a, b);
    }

    #[test]
    fn test_column_value_boolean_equality() {
        let a = ColumnValue::Boolean(vec![true, false]);
        let b = ColumnValue::Boolean(vec![true, false]);
        assert_eq!(a, b);
    }
}
