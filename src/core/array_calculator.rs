use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, ParsedModel, Table};
use xlformula_engine::{calculate, parse_formula, types, NoCustomFunction};

/// Array-aware calculator for v1.0.0 models
/// Handles both row-wise (element-wise) and aggregation formulas
pub struct ArrayCalculator {
    model: ParsedModel,
}

impl ArrayCalculator {
    pub fn new(model: ParsedModel) -> Self {
        Self { model }
    }

    /// Calculate all formulas in the model
    /// Returns updated model with calculated values
    pub fn calculate_all(mut self) -> ForgeResult<ParsedModel> {
        // For now, just process each table independently
        // TODO: Add dependency resolution for cross-table references

        // Clone table names to avoid borrow checker issues
        let table_names: Vec<String> = self.model.tables.keys().cloned().collect();

        for table_name in table_names {
            let table = self.model.tables.get(&table_name).unwrap().clone();
            let calculated_table = self.calculate_table(&table_name, &table)?;
            self.model.tables.insert(table_name, calculated_table);
        }

        Ok(self.model)
    }

    /// Calculate all formulas in a table
    fn calculate_table(&self, table_name: &str, table: &Table) -> ForgeResult<Table> {
        let mut working_table = table.clone();

        // Build dependency order for formulas
        let formula_order = self.get_formula_calculation_order(&working_table)?;

        // Calculate formulas in dependency order
        for col_name in formula_order {
            if let Some(formula) = working_table.row_formulas.get(&col_name) {
                let formula = formula.clone();

                // Determine if this is a row-wise or aggregation formula
                if self.is_aggregation_formula(&formula) {
                    // Aggregation: returns a scalar
                    // For now, we'll skip aggregations in tables (they belong in scalars section)
                    return Err(ForgeError::Eval(format!(
                        "Table '{}': Column '{}' uses aggregation formula - aggregations should be in scalars section",
                        table_name, col_name
                    )));
                } else {
                    // Row-wise: returns an array
                    let result = self.evaluate_rowwise_formula(&working_table, &formula)?;
                    working_table.add_column(Column::new(col_name.clone(), result));
                }
            }
        }

        Ok(working_table)
    }

    /// Get the order in which formulas should be calculated (dependency order)
    fn get_formula_calculation_order(&self, table: &Table) -> ForgeResult<Vec<String>> {
        use petgraph::algo::toposort;
        use petgraph::graph::DiGraph;
        use std::collections::HashMap;

        let mut graph = DiGraph::new();
        let mut node_indices = HashMap::new();

        // Create nodes for all formula columns
        for col_name in table.row_formulas.keys() {
            let idx = graph.add_node(col_name.clone());
            node_indices.insert(col_name.clone(), idx);
        }

        // Add edges for dependencies
        for (col_name, formula) in &table.row_formulas {
            let deps = self.extract_column_references(formula)?;
            for dep in deps {
                // Only add dependency if it's another formula column
                if let Some(&dep_idx) = node_indices.get(&dep) {
                    if let Some(&col_idx) = node_indices.get(col_name) {
                        graph.add_edge(dep_idx, col_idx, ());
                    }
                }
            }
        }

        // Topological sort
        let order = toposort(&graph, None).map_err(|_| {
            ForgeError::CircularDependency(format!(
                "Circular dependency detected in table formulas"
            ))
        })?;

        let ordered_names: Vec<String> = order
            .iter()
            .filter_map(|idx| graph.node_weight(*idx).cloned())
            .collect();

        Ok(ordered_names)
    }

    /// Check if a formula is an aggregation (returns scalar)
    fn is_aggregation_formula(&self, formula: &str) -> bool {
        let upper = formula.to_uppercase();
        upper.contains("SUM(")
            || upper.contains("AVERAGE(")
            || upper.contains("AVG(")
            || upper.contains("MAX(")
            || upper.contains("MIN(")
            || upper.contains("COUNT(")
            || upper.contains("SUMIF(")
            || upper.contains("COUNTIF(")
            || upper.contains("AVERAGEIF(")
    }

    /// Evaluate a row-wise formula (element-wise operations)
    /// Example: profit = revenue - expenses
    /// Evaluates: profit[i] = revenue[i] - expenses[i] for all i
    fn evaluate_rowwise_formula(
        &self,
        table: &Table,
        formula: &str,
    ) -> ForgeResult<ColumnValue> {
        let formula_str = if !formula.starts_with('=') {
            format!("={}", formula.trim())
        } else {
            formula.to_string()
        };

        // Get the row count from the table
        let row_count = table.row_count();
        if row_count == 0 {
            return Err(ForgeError::Eval(
                "Cannot evaluate row-wise formula on empty table".to_string(),
            ));
        }

        // Extract column references from the formula
        let col_refs = self.extract_column_references(formula)?;

        // Validate all columns exist and have correct length
        for col_ref in &col_refs {
            if let Some(col) = table.columns.get(col_ref) {
                if col.values.len() != row_count {
                    return Err(ForgeError::Eval(format!(
                        "Column '{}' has {} rows, expected {}",
                        col_ref,
                        col.values.len(),
                        row_count
                    )));
                }
            } else {
                return Err(ForgeError::Eval(format!(
                    "Column '{}' not found in table",
                    col_ref
                )));
            }
        }

        // Evaluate formula for each row
        let mut results = Vec::new();
        for row_idx in 0..row_count {
            // Create a resolver for this specific row
            let resolver = |var_name: String| -> types::Value {
                // Check if this is a column reference
                if let Some(col) = table.columns.get(&var_name) {
                    // Get the value at this row index
                    match &col.values {
                        ColumnValue::Number(nums) => {
                            if let Some(&val) = nums.get(row_idx) {
                                return types::Value::Number(val as f32);
                            }
                        }
                        _ => {
                            return types::Value::Error(types::Error::Value);
                        }
                    }
                }
                types::Value::Error(types::Error::Value)
            };

            // Parse and calculate for this row
            let parsed =
                parse_formula::parse_string_to_formula(&formula_str, None::<NoCustomFunction>);
            let result = calculate::calculate_formula(parsed, Some(&resolver));

            match result {
                types::Value::Number(n) => {
                    let value = n as f64;
                    let rounded = (value * 1e6).round() / 1e6;
                    results.push(rounded);
                }
                types::Value::Error(e) => {
                    return Err(ForgeError::Eval(format!(
                        "Formula '{}' at row {} returned error: {:?}",
                        formula_str, row_idx, e
                    )));
                }
                other => {
                    return Err(ForgeError::Eval(format!(
                        "Formula '{}' at row {} returned unexpected type: {:?}",
                        formula_str, row_idx, other
                    )));
                }
            }
        }

        Ok(ColumnValue::Number(results))
    }

    /// Extract column names referenced in a formula
    /// Simple implementation - looks for words that match column names
    fn extract_column_references(&self, formula: &str) -> ForgeResult<Vec<String>> {
        let formula = formula.trim_start_matches('=');
        let mut refs = Vec::new();

        // Extract all words (column names)
        for word in formula.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.') {
            if !word.is_empty() && !word.chars().next().unwrap().is_numeric() {
                // Don't include function names
                let upper = word.to_uppercase();
                if !matches!(
                    upper.as_str(),
                    "SUM" | "AVERAGE"
                        | "AVG"
                        | "MAX"
                        | "MIN"
                        | "COUNT"
                        | "IF"
                        | "AND"
                        | "OR"
                        | "NOT"
                        | "ABS"
                        | "ROUND"
                        | "POWER"
                        | "SQRT"
                ) {
                    if !refs.contains(&word.to_string()) {
                        refs.push(word.to_string());
                    }
                }
            }
        }

        Ok(refs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ForgeVersion;

    #[test]
    fn test_simple_rowwise_formula() {
        let mut model = ParsedModel::new(ForgeVersion::V1_0_0);

        let mut table = Table::new("test".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        table.add_column(Column::new(
            "expenses".to_string(),
            ColumnValue::Number(vec![60.0, 120.0, 180.0]),
        ));
        table.add_row_formula("profit".to_string(), "=revenue - expenses".to_string());

        model.add_table(table);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        let result_table = result.tables.get("test").unwrap();
        let profit_col = result_table.columns.get("profit").unwrap();

        match &profit_col.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums.len(), 3);
                assert_eq!(nums[0], 40.0);
                assert_eq!(nums[1], 80.0);
                assert_eq!(nums[2], 120.0);
            }
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_is_aggregation_formula() {
        let model = ParsedModel::new(ForgeVersion::V1_0_0);
        let calc = ArrayCalculator::new(model);

        assert!(calc.is_aggregation_formula("=SUM(revenue)"));
        assert!(calc.is_aggregation_formula("=AVERAGE(profit)"));
        assert!(calc.is_aggregation_formula("=sum(revenue)")); // case insensitive
        assert!(!calc.is_aggregation_formula("=revenue - expenses"));
        assert!(!calc.is_aggregation_formula("=revenue * 0.3"));
    }

    #[test]
    fn test_extract_column_references() {
        let model = ParsedModel::new(ForgeVersion::V1_0_0);
        let calc = ArrayCalculator::new(model);

        let refs = calc.extract_column_references("=revenue - expenses").unwrap();
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"revenue".to_string()));
        assert!(refs.contains(&"expenses".to_string()));

        let refs2 = calc
            .extract_column_references("=revenue * 0.3 + fixed_cost")
            .unwrap();
        assert!(refs2.contains(&"revenue".to_string()));
        assert!(refs2.contains(&"fixed_cost".to_string()));
    }
}
