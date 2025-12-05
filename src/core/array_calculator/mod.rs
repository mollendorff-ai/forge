mod dates;
mod math;
mod text;

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, ParsedModel, Table};
use std::collections::HashSet;
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
        // Step 1: Calculate all tables (row-wise formulas) in dependency order
        let table_names: Vec<String> = self.model.tables.keys().cloned().collect();
        let calc_order = self.get_table_calculation_order(&table_names)?;

        for table_name in calc_order {
            let table = self.model.tables.get(&table_name).unwrap().clone();
            let calculated_table = self.calculate_table(&table_name, &table)?;
            self.model.tables.insert(table_name, calculated_table);
        }

        // Step 2: Calculate scalar aggregations and formulas
        self.calculate_scalars()?;

        Ok(self.model)
    }

    /// Get calculation order for tables (topological sort based on cross-table references)
    fn get_table_calculation_order(&self, table_names: &[String]) -> ForgeResult<Vec<String>> {
        use petgraph::algo::toposort;
        use petgraph::graph::DiGraph;
        use std::collections::HashMap;

        let mut graph = DiGraph::new();
        let mut node_indices = HashMap::new();

        // Create nodes for all tables
        for name in table_names {
            let idx = graph.add_node(name.clone());
            node_indices.insert(name.clone(), idx);
        }

        // Add edges for cross-table dependencies
        for name in table_names {
            if let Some(table) = self.model.tables.get(name) {
                // Check all row formulas for cross-table references
                for formula in table.row_formulas.values() {
                    let deps = self.extract_table_dependencies_from_formula(formula)?;
                    for dep_table in deps {
                        // Only add edge if dependency is another table
                        if let Some(&dep_idx) = node_indices.get(&dep_table) {
                            if let Some(&name_idx) = node_indices.get(name) {
                                graph.add_edge(dep_idx, name_idx, ());
                            }
                        }
                    }
                }
            }
        }

        // Topological sort
        let order = toposort(&graph, None).map_err(|_| {
            ForgeError::CircularDependency(
                "Circular dependency detected between tables".to_string(),
            )
        })?;

        let ordered_names: Vec<String> = order
            .iter()
            .filter_map(|idx| graph.node_weight(*idx).cloned())
            .collect();

        Ok(ordered_names)
    }

    /// Extract table names referenced in a formula (e.g., "pl_2025" from "=pl_2025.revenue")
    fn extract_table_dependencies_from_formula(&self, formula: &str) -> ForgeResult<Vec<String>> {
        let mut deps = Vec::new();

        // Look for table.column patterns
        for word in formula.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.') {
            if word.contains('.') {
                // This might be table.column reference
                if let Ok((table_name, _col_name)) = self.parse_table_column_ref(word) {
                    // Check if this table exists
                    if self.model.tables.contains_key(&table_name) && !deps.contains(&table_name) {
                        deps.push(table_name);
                    }
                }
            }
        }

        Ok(deps)
    }

    /// Calculate all formulas in a table
    fn calculate_table(&mut self, table_name: &str, table: &Table) -> ForgeResult<Table> {
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
            ForgeError::CircularDependency(
                "Circular dependency detected in table formulas".to_string(),
            )
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
            || upper.contains("SUMIFS(")
            || upper.contains("COUNTIFS(")
            || upper.contains("AVERAGEIFS(")
            || upper.contains("MAXIFS(")
            || upper.contains("MINIFS(")
            // Statistical functions (v5.0.0)
            || upper.contains("MEDIAN(")
            || upper.contains("VAR(")
            || upper.contains("VAR.S(")
            || upper.contains("VAR.P(")
            || upper.contains("STDEV(")
            || upper.contains("STDEV.S(")
            || upper.contains("STDEV.P(")
            || upper.contains("PERCENTILE(")
            || upper.contains("QUARTILE(")
            || upper.contains("CORREL(")
    }

    /// Check if formula contains custom math functions that need special handling
    fn has_custom_math_function(&self, formula: &str) -> bool {
        let upper = formula.to_uppercase();
        upper.contains("ROUND(")
            || upper.contains("ROUNDUP(")
            || upper.contains("ROUNDDOWN(")
            || upper.contains("CEILING(")
            || upper.contains("FLOOR(")
            || upper.contains("MOD(")
            || upper.contains("SQRT(")
            || upper.contains("POWER(")
    }

    /// Check if formula contains custom text functions that need special handling
    fn has_custom_text_function(&self, formula: &str) -> bool {
        let upper = formula.to_uppercase();
        upper.contains("CONCAT(")
            || upper.contains("CONCATENATE(")
            || upper.contains("TRIM(")
            || upper.contains("UPPER(")
            || upper.contains("LOWER(")
            || upper.contains("LEN(")
            || upper.contains("MID(")
    }

    /// Check if formula contains custom date functions that need special handling
    fn has_custom_date_function(&self, formula: &str) -> bool {
        let upper = formula.to_uppercase();
        upper.contains("TODAY(")
            || upper.contains("DATE(")
            || upper.contains("YEAR(")
            || upper.contains("MONTH(")
            || upper.contains("DAY(")
            || upper.contains("DATEDIF(")
            || upper.contains("EDATE(")
            || upper.contains("EOMONTH(")
            // Additional date functions (v5.0.0)
            || upper.contains("NETWORKDAYS(")
            || upper.contains("WORKDAY(")
            || upper.contains("YEARFRAC(")
    }

    /// Check if formula contains Forge-native FP&A functions (v5.0.0)
    fn has_forge_function(&self, formula: &str) -> bool {
        let upper = formula.to_uppercase();
        upper.contains("VARIANCE(")
            || upper.contains("VARIANCE_PCT(")
            || upper.contains("VARIANCE_STATUS(")
            || upper.contains("BREAKEVEN_UNITS(")
            || upper.contains("BREAKEVEN_REVENUE(")
            || upper.contains("SCENARIO(")
    }

    /// Check if formula contains lookup functions that need special handling
    fn has_lookup_function(&self, formula: &str) -> bool {
        let upper = formula.to_uppercase();
        upper.contains("MATCH(")
            || upper.contains("INDEX(")
            || upper.contains("VLOOKUP(")
            || upper.contains("XLOOKUP(")
            || upper.contains("CHOOSE(")
            || upper.contains("OFFSET(")
            || upper.contains("LET(")
            || upper.contains("SWITCH(")
            || upper.contains("INDIRECT(")
            || upper.contains("LAMBDA(")
    }

    /// Check if formula contains financial functions that need special handling (v1.6.0)
    fn has_financial_function(&self, formula: &str) -> bool {
        let upper = formula.to_uppercase();
        upper.contains("NPV(")
            || upper.contains("IRR(")
            || upper.contains("XNPV(")
            || upper.contains("XIRR(")
            || upper.contains("PMT(")
            || upper.contains("FV(")
            || upper.contains("PV(")
            || upper.contains("RATE(")
            || upper.contains("NPER(")
            || upper.contains("CHOOSE(")
            // Additional financial functions (v5.0.0)
            || upper.contains("MIRR(")
            || upper.contains("SLN(")
            || upper.contains("DB(")
            || upper.contains("DDB(")
    }

    /// Check if formula contains array functions that need special handling (v4.1.0)
    fn has_array_function(&self, formula: &str) -> bool {
        let upper = formula.to_uppercase();
        upper.contains("UNIQUE(")
            || upper.contains("COUNTUNIQUE(")
            || upper.contains("FILTER(")
            || upper.contains("SORT(")
    }

    /// Check if formula contains math functions that need special handling (v4.4.1)
    fn has_math_function(&self, formula: &str) -> bool {
        let upper = formula.to_uppercase();
        upper.contains("ROUND(")
            || upper.contains("ROUNDUP(")
            || upper.contains("ROUNDDOWN(")
            || upper.contains("SQRT(")
            || upper.contains("POWER(")
            || upper.contains("MOD(")
            || upper.contains("CEILING(")
            || upper.contains("FLOOR(")
    }

    /// Evaluate a row-wise formula (element-wise operations)
    /// Example: profit = revenue - expenses
    /// Evaluates: profit[i] = revenue[i] - expenses[i] for all i
    fn evaluate_rowwise_formula(
        &mut self,
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
        // Skip validation for lookup functions - their arrays can be different lengths
        let col_refs = if self.has_lookup_function(&formula_str) {
            Vec::new() // Skip column validation for lookup functions
        } else {
            self.extract_column_references(formula)?
        };

        // Validate all columns exist and have correct length
        for col_ref in &col_refs {
            // Check if this is a cross-table reference (table.column format) or scalar reference
            if col_ref.contains('.') {
                // First check if it's a scalar reference (v4.3.0 fix)
                // Scalars like thresholds.min_value should be skipped in column validation
                if self.model.scalars.contains_key(col_ref) {
                    continue; // Scalar reference - no row count validation needed
                }

                // Cross-table reference - validate it exists
                let parts: Vec<&str> = col_ref.split('.').collect();
                if parts.len() == 2 {
                    let ref_table_name = parts[0];
                    let ref_col_name = parts[1];

                    if let Some(ref_table) = self.model.tables.get(ref_table_name) {
                        if let Some(ref_col) = ref_table.columns.get(ref_col_name) {
                            if ref_col.values.len() != row_count {
                                return Err(ForgeError::Eval(format!(
                                    "Column '{}.{}' has {} rows, expected {}",
                                    ref_table_name,
                                    ref_col_name,
                                    ref_col.values.len(),
                                    row_count
                                )));
                            }
                        } else {
                            return Err(ForgeError::Eval(format!(
                                "Column '{}' not found in table '{}'",
                                ref_col_name, ref_table_name
                            )));
                        }
                    } else {
                        return Err(ForgeError::Eval(format!(
                            "Table '{}' not found",
                            ref_table_name
                        )));
                    }
                } else {
                    return Err(ForgeError::Eval(format!(
                        "Invalid cross-table reference: {}",
                        col_ref
                    )));
                }
            } else if let Some(col) = table.columns.get(col_ref) {
                // Local column reference
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
        let mut number_results = Vec::new();
        let mut text_results = Vec::new();
        let mut bool_results = Vec::new();
        let mut result_type: Option<&str> = None;

        for row_idx in 0..row_count {
            // Preprocess formula to replace scalar references with their values (v4.3.0 fix)
            // This handles references like thresholds.min_value before xlformula_engine parsing
            let formula_with_scalars = self.preprocess_scalar_refs_for_table(&formula_str)?;

            // Preprocess formula for custom functions
            let processed_formula = if self.has_custom_math_function(&formula_with_scalars)
                || self.has_custom_text_function(&formula_with_scalars)
                || self.has_custom_date_function(&formula_with_scalars)
                || self.has_lookup_function(&formula_with_scalars)
                || self.has_financial_function(&formula_with_scalars)
            {
                self.preprocess_custom_functions(&formula_with_scalars, row_idx, table)?
            } else {
                formula_with_scalars.clone()
            };

            // Create a resolver for this specific row
            let resolver = |var_name: String| -> types::Value {
                // Check if this is a cross-table reference (table.column format)
                if var_name.contains('.') {
                    let parts: Vec<&str> = var_name.split('.').collect();
                    if parts.len() == 2 {
                        let ref_table_name = parts[0];
                        let ref_col_name = parts[1];

                        // Try as table.column reference first
                        if let Some(ref_table) = self.model.tables.get(ref_table_name) {
                            if let Some(ref_col) = ref_table.columns.get(ref_col_name) {
                                match &ref_col.values {
                                    ColumnValue::Number(nums) => {
                                        if let Some(&val) = nums.get(row_idx) {
                                            return types::Value::Number(val as f32);
                                        }
                                    }
                                    ColumnValue::Text(texts) => {
                                        if let Some(text) = texts.get(row_idx) {
                                            return types::Value::Text(text.clone());
                                        }
                                    }
                                    ColumnValue::Boolean(bools) => {
                                        if let Some(&val) = bools.get(row_idx) {
                                            return types::Value::Boolean(if val {
                                                types::Boolean::True
                                            } else {
                                                types::Boolean::False
                                            });
                                        }
                                    }
                                    ColumnValue::Date(dates) => {
                                        if let Some(date) = dates.get(row_idx) {
                                            return types::Value::Text(date.clone());
                                        }
                                    }
                                }
                            }
                        }

                        // Try as section.scalar reference (v4.3.0 fix)
                        // This allows table formulas to reference scalars like thresholds.min_value
                        if let Some(scalar) = self.model.scalars.get(&var_name) {
                            if let Some(value) = scalar.value {
                                return types::Value::Number(value as f32);
                            }
                        }
                    }
                    return types::Value::Error(types::Error::Value);
                }

                // Local column reference
                if let Some(col) = table.columns.get(&var_name) {
                    // Get the value at this row index
                    match &col.values {
                        ColumnValue::Number(nums) => {
                            if let Some(&val) = nums.get(row_idx) {
                                return types::Value::Number(val as f32);
                            }
                        }
                        ColumnValue::Text(texts) => {
                            if let Some(text) = texts.get(row_idx) {
                                return types::Value::Text(text.clone());
                            }
                        }
                        ColumnValue::Boolean(bools) => {
                            if let Some(&val) = bools.get(row_idx) {
                                return types::Value::Boolean(if val {
                                    types::Boolean::True
                                } else {
                                    types::Boolean::False
                                });
                            }
                        }
                        ColumnValue::Date(dates) => {
                            if let Some(date) = dates.get(row_idx) {
                                // For dates, return as text (ISO format)
                                return types::Value::Text(date.clone());
                            }
                        }
                    }
                }

                // Scalar variable reference (v4.3.0 fix)
                // Check if this is a scalar variable (for formulas like =IF(remaining_quota > 0, 1, 0))
                if let Some(scalar) = self.model.scalars.get(&var_name) {
                    if let Some(value) = scalar.value {
                        return types::Value::Number(value as f32);
                    }
                }

                types::Value::Error(types::Error::Reference)
            };

            // Parse and calculate for this row
            let parsed = parse_formula::parse_string_to_formula(
                &processed_formula,
                None::<NoCustomFunction>,
            );
            let result = calculate::calculate_formula(parsed, Some(&resolver));

            match result {
                types::Value::Number(n) => {
                    let value = n as f64;
                    let rounded = (value * 1e6).round() / 1e6;
                    number_results.push(rounded);
                    if result_type.is_none() {
                        result_type = Some("number");
                    }
                }
                types::Value::Text(t) => {
                    text_results.push(t);
                    if result_type.is_none() {
                        result_type = Some("text");
                    }
                }
                types::Value::Boolean(b) => {
                    let bool_val = match b {
                        types::Boolean::True => true,
                        types::Boolean::False => false,
                    };
                    bool_results.push(bool_val);
                    if result_type.is_none() {
                        result_type = Some("boolean");
                    }
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

        // Return the appropriate column type based on results
        match result_type {
            Some("number") => Ok(ColumnValue::Number(number_results)),
            Some("text") => Ok(ColumnValue::Text(text_results)),
            Some("boolean") => Ok(ColumnValue::Boolean(bool_results)),
            _ => Err(ForgeError::Eval(
                "Formula did not produce any valid results".to_string(),
            )),
        }
    }

    /// Calculate scalar values and aggregations
    /// Returns updated model with calculated scalars
    fn calculate_scalars(&mut self) -> ForgeResult<()> {
        // Get all scalar variable names that have formulas
        let scalar_names: Vec<String> = self
            .model
            .scalars
            .iter()
            .filter(|(_, var)| var.formula.is_some())
            .map(|(name, _)| name.clone())
            .collect();

        // Build dependency graph and calculate in order
        let calc_order = self.get_scalar_calculation_order(&scalar_names)?;

        // Calculate each scalar in dependency order
        for scalar_name in calc_order {
            let formula = self
                .model
                .scalars
                .get(&scalar_name)
                .and_then(|v| v.formula.clone());

            if let Some(formula) = formula {
                let value = self.evaluate_scalar_formula(&formula, &scalar_name)?;

                // Update the scalar with calculated value
                if let Some(var) = self.model.scalars.get_mut(&scalar_name) {
                    var.value = Some(value);
                }
            }
        }

        Ok(())
    }

    /// Get calculation order for scalars (topological sort)
    fn get_scalar_calculation_order(&self, scalar_names: &[String]) -> ForgeResult<Vec<String>> {
        use petgraph::algo::toposort;
        use petgraph::graph::DiGraph;
        use std::collections::HashMap;

        let mut graph = DiGraph::new();
        let mut node_indices = HashMap::new();

        // Create nodes for all scalars
        for name in scalar_names {
            let idx = graph.add_node(name.clone());
            node_indices.insert(name.clone(), idx);
        }

        // Add edges for dependencies
        for name in scalar_names {
            if let Some(var) = self.model.scalars.get(name) {
                if let Some(formula) = &var.formula {
                    let deps = self.extract_scalar_dependencies(formula, name)?;
                    for dep in deps {
                        // Only add dependency if it's another scalar
                        if let Some(&dep_idx) = node_indices.get(&dep) {
                            if let Some(&name_idx) = node_indices.get(name) {
                                graph.add_edge(dep_idx, name_idx, ());
                            }
                        }
                    }
                }
            }
        }

        // Topological sort
        let order = toposort(&graph, None).map_err(|_| {
            ForgeError::CircularDependency(
                "Circular dependency detected in scalar formulas".to_string(),
            )
        })?;

        let ordered_names: Vec<String> = order
            .iter()
            .filter_map(|idx| graph.node_weight(*idx).cloned())
            .collect();

        Ok(ordered_names)
    }

    /// Extract scalar dependencies from a formula with scoping
    /// Uses same scoping logic as evaluate_scalar_with_resolver
    fn extract_scalar_dependencies(
        &self,
        formula: &str,
        scalar_name: &str,
    ) -> ForgeResult<Vec<String>> {
        let mut deps = Vec::new();

        // Extract parent section from scalar_name (e.g., "annual_2025" from "annual_2025.total_revenue")
        let parent_section = scalar_name
            .rfind('.')
            .map(|dot_pos| &scalar_name[..dot_pos]);

        // Extract all words (variable references) from formula
        for word in formula.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.') {
            if word.is_empty() {
                continue;
            }

            // Strategy 1: Try exact match
            if self.model.scalars.contains_key(word) {
                if !deps.contains(&word.to_string()) {
                    deps.push(word.to_string());
                }
                continue;
            }

            // Strategy 2: If we're in a section and word is simple, try prefixing with parent section
            if let Some(section) = parent_section {
                if !word.contains('.') {
                    let scoped_name = format!("{}.{}", section, word);
                    if self.model.scalars.contains_key(&scoped_name) {
                        if !deps.contains(&scoped_name) {
                            deps.push(scoped_name);
                        }
                        continue;
                    }
                }
            }

            // Strategy 3: Could be a table.column reference, not a scalar dependency
            // Skip it (no dependency edge needed)
        }

        Ok(deps)
    }

    /// Evaluate a scalar formula (aggregations, array indexing, scalar operations)
    fn evaluate_scalar_formula(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        let formula_str = if !formula.starts_with('=') {
            format!("={}", formula.trim())
        } else {
            formula.to_string()
        };

        // Check if this formula contains lookup functions that need preprocessing
        // This includes OFFSET which must be resolved before aggregation
        if self.has_lookup_function(&formula_str) {
            return self.evaluate_lookup_formula(&formula_str, scalar_name);
        }

        // Check if this formula contains array functions (FILTER, SORT) that need preprocessing
        // These must be resolved before aggregation functions can process them
        if self.has_array_function(&formula_str) && self.is_aggregation_formula(&formula_str) {
            return self.evaluate_array_then_aggregation(&formula_str, scalar_name);
        }

        // Check if this formula contains aggregation functions (but not mixed with other operations)
        if self.is_aggregation_formula(&formula_str) && !formula_str.contains('[') {
            self.evaluate_aggregation(&formula_str)
        } else if formula_str.contains('[') && formula_str.contains(']') {
            // Check if it's a pure array indexing formula (just =table.column[index])
            let trimmed = formula_str.trim_start_matches('=').trim();
            if trimmed.matches('[').count() == 1
                && !trimmed.contains('+')
                && !trimmed.contains('-')
                && !trimmed.contains('*')
                && !trimmed.contains('/')
                && !trimmed.contains('^')
                && !trimmed.contains('(')
            {
                // Pure array indexing
                self.evaluate_array_indexing(&formula_str)
            } else {
                // Complex formula with array indexing - preprocess it
                let processed = self.preprocess_array_indexing(&formula_str)?;
                self.evaluate_scalar_with_resolver(&processed, scalar_name)
            }
        } else if self.has_financial_function(&formula_str) {
            // Financial functions - evaluate them specially
            self.evaluate_financial_formula(&formula_str, scalar_name)
        } else if self.has_custom_date_function(&formula_str) {
            // Date functions - evaluate them specially
            self.evaluate_date_formula(&formula_str, scalar_name)
        } else if self.has_array_function(&formula_str) {
            // Array functions (UNIQUE, COUNTUNIQUE) - evaluate them specially (v4.1.0)
            self.evaluate_array_formula(&formula_str, scalar_name)
        } else if self.has_math_function(&formula_str) {
            // Math functions (ROUND, SQRT, etc.) - evaluate them specially (v4.4.1)
            self.evaluate_math_formula(&formula_str, scalar_name)
        } else if self.has_forge_function(&formula_str) {
            // Forge-native FP&A functions (v5.0.0)
            self.evaluate_forge_formula(&formula_str, scalar_name)
        } else {
            // Regular scalar formula - use xlformula_engine
            self.evaluate_scalar_with_resolver(&formula_str, scalar_name)
        }
    }

    /// Evaluate a formula containing financial functions
    fn evaluate_financial_formula(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        // First resolve all scalar references to their values
        let resolved = self.resolve_scalar_references(formula, scalar_name)?;

        // Create an empty table for context (not used for scalar evaluation)
        let empty_table = Table::new("_scalar_context".to_string());

        // Process financial functions
        let processed = self.replace_financial_functions(&resolved, 0, &empty_table)?;

        // If the result is just a number, parse it directly
        let trimmed = processed.trim().trim_start_matches('=');
        if let Ok(value) = trimmed.parse::<f64>() {
            return Ok(value);
        }

        // Otherwise evaluate with xlformula_engine
        self.evaluate_scalar_with_resolver(&processed, scalar_name)
    }

    /// Evaluate a formula containing date functions (for scalar context)
    fn evaluate_date_formula(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        // First resolve all scalar references to their values
        let resolved = self.resolve_scalar_references(formula, scalar_name)?;

        // Create an empty table for context (not used for scalar evaluation)
        let empty_table = Table::new("_scalar_context".to_string());

        // Process date functions
        let processed = self.replace_date_functions(&resolved, 0, &empty_table)?;

        // If the result is just a number, parse it directly
        let trimmed = processed.trim().trim_start_matches('=');
        if let Ok(value) = trimmed.parse::<f64>() {
            return Ok(value);
        }

        // Otherwise evaluate with xlformula_engine
        self.evaluate_scalar_with_resolver(&processed, scalar_name)
    }

    /// Evaluate a formula containing Forge-native FP&A functions (v5.0.0)
    /// These are functions unique to Forge that don't exist in Excel
    fn evaluate_forge_formula(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        let upper = formula.to_uppercase();

        // VARIANCE(actual, budget) - returns actual - budget
        if upper.contains("VARIANCE_PCT(") {
            return self.evaluate_variance_pct(formula, scalar_name);
        } else if upper.contains("VARIANCE_STATUS(") {
            return self.evaluate_variance_status(formula, scalar_name);
        } else if upper.contains("VARIANCE(") {
            return self.evaluate_variance(formula, scalar_name);
        } else if upper.contains("BREAKEVEN_UNITS(") {
            return self.evaluate_breakeven_units(formula, scalar_name);
        } else if upper.contains("BREAKEVEN_REVENUE(") {
            return self.evaluate_breakeven_revenue(formula, scalar_name);
        } else if upper.contains("SCENARIO(") {
            return self.evaluate_scenario(formula, scalar_name);
        }

        Err(ForgeError::Eval(format!(
            "Unknown Forge function in formula: {}",
            formula
        )))
    }

    /// VARIANCE(actual, budget) - returns actual - budget
    /// Positive = favorable for revenue, negative = unfavorable
    fn evaluate_variance(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        let args = self.extract_forge_function_args(formula, "VARIANCE")?;
        if args.len() != 2 {
            return Err(ForgeError::Eval(
                "VARIANCE requires exactly 2 arguments: actual, budget".to_string(),
            ));
        }

        let actual = self.resolve_scalar_value(&args[0], scalar_name)?;
        let budget = self.resolve_scalar_value(&args[1], scalar_name)?;

        Ok(actual - budget)
    }

    /// VARIANCE_PCT(actual, budget) - returns (actual - budget) / budget
    fn evaluate_variance_pct(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        let args = self.extract_forge_function_args(formula, "VARIANCE_PCT")?;
        if args.len() != 2 {
            return Err(ForgeError::Eval(
                "VARIANCE_PCT requires exactly 2 arguments: actual, budget".to_string(),
            ));
        }

        let actual = self.resolve_scalar_value(&args[0], scalar_name)?;
        let budget = self.resolve_scalar_value(&args[1], scalar_name)?;

        if budget == 0.0 {
            return Err(ForgeError::Eval(
                "VARIANCE_PCT: budget cannot be zero".to_string(),
            ));
        }

        Ok((actual - budget) / budget)
    }

    /// VARIANCE_STATUS(actual, budget, [type]) - returns numeric status code
    /// Returns: 1 = favorable, 0 = on-target, -1 = unfavorable
    /// type='cost' inverts logic (lower actual = favorable)
    fn evaluate_variance_status(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        let args = self.extract_forge_function_args(formula, "VARIANCE_STATUS")?;
        if args.len() < 2 || args.len() > 3 {
            return Err(ForgeError::Eval(
                "VARIANCE_STATUS requires 2-3 arguments: actual, budget, [type]".to_string(),
            ));
        }

        let actual = self.resolve_scalar_value(&args[0], scalar_name)?;
        let budget = self.resolve_scalar_value(&args[1], scalar_name)?;

        // Check if type is 'cost' (invert logic)
        let is_cost = args.len() == 3 && args[2].trim().trim_matches('"').to_lowercase() == "cost";

        let variance = actual - budget;

        // Define threshold for "on-target" (within 0.1%)
        let threshold = budget.abs() * 0.001;

        if variance.abs() <= threshold {
            Ok(0.0) // On-target
        } else if is_cost {
            // For costs: lower actual is favorable (positive)
            if variance < 0.0 {
                Ok(1.0)
            } else {
                Ok(-1.0)
            }
        } else {
            // For revenue: higher actual is favorable (positive)
            if variance > 0.0 {
                Ok(1.0)
            } else {
                Ok(-1.0)
            }
        }
    }

    /// BREAKEVEN_UNITS(fixed_costs, unit_price, variable_cost_per_unit)
    /// Returns the number of units needed to break even
    fn evaluate_breakeven_units(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        let args = self.extract_forge_function_args(formula, "BREAKEVEN_UNITS")?;
        if args.len() != 3 {
            return Err(ForgeError::Eval(
                "BREAKEVEN_UNITS requires exactly 3 arguments: fixed_costs, unit_price, variable_cost_per_unit".to_string(),
            ));
        }

        let fixed_costs = self.resolve_scalar_value(&args[0], scalar_name)?;
        let unit_price = self.resolve_scalar_value(&args[1], scalar_name)?;
        let variable_cost = self.resolve_scalar_value(&args[2], scalar_name)?;

        let contribution_margin = unit_price - variable_cost;
        if contribution_margin <= 0.0 {
            return Err(ForgeError::Eval(
                "BREAKEVEN_UNITS: unit_price must be greater than variable_cost".to_string(),
            ));
        }

        Ok(fixed_costs / contribution_margin)
    }

    /// BREAKEVEN_REVENUE(fixed_costs, contribution_margin_pct)
    /// Returns the revenue needed to break even
    fn evaluate_breakeven_revenue(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        let args = self.extract_forge_function_args(formula, "BREAKEVEN_REVENUE")?;
        if args.len() != 2 {
            return Err(ForgeError::Eval(
                "BREAKEVEN_REVENUE requires exactly 2 arguments: fixed_costs, contribution_margin_pct".to_string(),
            ));
        }

        let fixed_costs = self.resolve_scalar_value(&args[0], scalar_name)?;
        let margin_pct = self.resolve_scalar_value(&args[1], scalar_name)?;

        if margin_pct <= 0.0 || margin_pct > 1.0 {
            return Err(ForgeError::Eval(
                "BREAKEVEN_REVENUE: contribution_margin_pct must be between 0 and 1".to_string(),
            ));
        }

        Ok(fixed_costs / margin_pct)
    }

    /// SCENARIO(scenario_name, variable) - Get variable value from a specific scenario
    /// Enables side-by-side scenario comparison in a single calculation pass
    fn evaluate_scenario(&self, formula: &str, _scalar_name: &str) -> ForgeResult<f64> {
        let args = self.extract_forge_function_args(formula, "SCENARIO")?;
        if args.len() != 2 {
            return Err(ForgeError::Eval(
                "SCENARIO requires exactly 2 arguments: scenario_name, variable".to_string(),
            ));
        }

        // Parse scenario name (strip quotes)
        let scenario_name = args[0].trim().trim_matches('"').trim_matches('\'');
        let variable_name = args[1].trim().trim_matches('"').trim_matches('\'');

        // Look up scenario in model
        let scenario = self.model.scenarios.get(scenario_name).ok_or_else(|| {
            let available: Vec<&String> = self.model.scenarios.keys().collect();
            ForgeError::Eval(format!(
                "SCENARIO: scenario '{}' not found. Available: {:?}",
                scenario_name, available
            ))
        })?;

        // Look up variable in scenario overrides
        let value = scenario.overrides.get(variable_name).ok_or_else(|| {
            let available: Vec<&String> = scenario.overrides.keys().collect();
            ForgeError::Eval(format!(
                "SCENARIO: variable '{}' not found in scenario '{}'. Available: {:?}",
                variable_name, scenario_name, available
            ))
        })?;

        Ok(*value)
    }

    /// Helper to extract function arguments for Forge functions
    fn extract_forge_function_args(
        &self,
        formula: &str,
        func_name: &str,
    ) -> ForgeResult<Vec<String>> {
        let upper = formula.to_uppercase();
        let pattern = format!("{}(", func_name);
        let start = upper
            .find(&pattern)
            .ok_or_else(|| ForgeError::Eval(format!("{} function not found", func_name)))?
            + pattern.len();

        // Find matching closing parenthesis
        let rest = &formula[start..];
        let mut depth = 1;
        let mut end = 0;
        for (i, c) in rest.chars().enumerate() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = i;
                        break;
                    }
                }
                _ => {}
            }
        }

        let args_str = &rest[..end];

        // Split by comma, respecting parentheses
        let mut args = Vec::new();
        let mut current = String::new();
        let mut paren_depth = 0;

        for c in args_str.chars() {
            match c {
                '(' => {
                    paren_depth += 1;
                    current.push(c);
                }
                ')' => {
                    paren_depth -= 1;
                    current.push(c);
                }
                ',' if paren_depth == 0 => {
                    args.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(c),
            }
        }
        if !current.is_empty() {
            args.push(current.trim().to_string());
        }

        Ok(args)
    }

    /// Resolve a scalar value from a scalar reference or literal number
    fn resolve_scalar_value(&self, arg: &str, current_scalar: &str) -> ForgeResult<f64> {
        let trimmed = arg.trim();

        // Try parsing as a literal number first
        if let Ok(value) = trimmed.parse::<f64>() {
            return Ok(value);
        }

        // If it contains a dot, it might be a scalar reference like "budget.revenue"
        if trimmed.contains('.') {
            // Try to get from model scalars
            if let Some(scalar) = self.model.scalars.get(trimmed) {
                if let Some(value) = scalar.value {
                    return Ok(value);
                }
            }
        }

        // Try as a simple scalar name
        if let Some(scalar) = self.model.scalars.get(trimmed) {
            if let Some(value) = scalar.value {
                return Ok(value);
            }
        }

        // Try evaluating as a formula
        self.evaluate_scalar_with_resolver(&format!("={}", trimmed), current_scalar)
    }

    /// Evaluate a formula containing array functions (for scalar context) (v4.1.0)
    fn evaluate_array_formula(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        // First resolve all scalar references to their values
        let resolved = self.resolve_scalar_references(formula, scalar_name)?;

        // Create an empty table for context (we'll use self.model for cross-table lookups)
        let empty_table = Table::new("_scalar_context".to_string());

        // Process array functions
        let processed = self.replace_array_functions(&resolved, 0, &empty_table)?;

        // If the result is just a number, parse it directly
        let trimmed = processed.trim().trim_start_matches('=');
        if let Ok(value) = trimmed.parse::<f64>() {
            return Ok(value);
        }

        // Otherwise evaluate with xlformula_engine
        self.evaluate_scalar_with_resolver(&processed, scalar_name)
    }

    /// Evaluate a formula containing math functions (for scalar context) (v4.4.1)
    /// Handles: ROUND, ROUNDUP, ROUNDDOWN, SQRT, POWER, MOD, CEILING, FLOOR
    fn evaluate_math_formula(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        // First resolve all scalar references to their values
        let resolved = self.resolve_scalar_references(formula, scalar_name)?;

        // Create an empty table for context
        let empty_table = Table::new("_scalar_context".to_string());

        // Process math functions
        let processed = self.replace_math_functions(&resolved, 0, &empty_table)?;

        // If the result is just a number, parse it directly
        let trimmed = processed.trim().trim_start_matches('=');
        if let Ok(value) = trimmed.parse::<f64>() {
            return Ok(value);
        }

        // Otherwise evaluate with xlformula_engine
        self.evaluate_scalar_with_resolver(&processed, scalar_name)
    }

    /// Evaluate a formula containing lookup functions (MATCH, INDEX, CHOOSE, OFFSET, etc.)
    /// These must be resolved before aggregation functions can process them
    fn evaluate_lookup_formula(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        // First resolve all scalar references to their values
        let resolved = self.resolve_scalar_references(formula, scalar_name)?;

        // Create an empty table for context (we'll use self.model for cross-table lookups)
        let empty_table = Table::new("_scalar_context".to_string());

        // Process lookup functions (OFFSET, CHOOSE, etc. will be replaced with values)
        let processed = self.replace_lookup_functions(&resolved, 0, &empty_table)?;

        // If the result is just a number, parse it directly
        let trimmed = processed.trim().trim_start_matches('=');
        if let Ok(value) = trimmed.parse::<f64>() {
            return Ok(value);
        }

        // Check if the processed formula is now an aggregation formula
        // e.g., =SUM(100, 200, 300) after OFFSET(array, 0, 3) was resolved
        if self.is_aggregation_formula(&processed) {
            return self.evaluate_aggregation(&processed);
        }

        // Otherwise evaluate with xlformula_engine
        self.evaluate_scalar_with_resolver(&processed, scalar_name)
    }

    /// Evaluate a formula containing array functions followed by aggregation
    /// Example: =SUM(FILTER(values, include)) or =SUM(SORT(values))
    fn evaluate_array_then_aggregation(
        &self,
        formula: &str,
        scalar_name: &str,
    ) -> ForgeResult<f64> {
        // First resolve all scalar references to their values
        let resolved = self.resolve_scalar_references(formula, scalar_name)?;

        // Create an empty table for context (we'll use self.model for cross-table lookups)
        let empty_table = Table::new("_scalar_context".to_string());

        // Process array functions (FILTER, SORT will be replaced with comma-separated values)
        let processed = self.replace_array_functions(&resolved, 0, &empty_table)?;

        // If the result is just a number, parse it directly
        let trimmed = processed.trim().trim_start_matches('=');
        if let Ok(value) = trimmed.parse::<f64>() {
            return Ok(value);
        }

        // Now evaluate the aggregation on the processed formula
        // e.g., =SUM(100, 200, 300) after SORT was resolved
        if self.is_aggregation_formula(&processed) {
            return self.evaluate_aggregation(&processed);
        }

        // Otherwise evaluate with xlformula_engine
        self.evaluate_scalar_with_resolver(&processed, scalar_name)
    }

    /// Resolve scalar variable references in a formula
    fn resolve_scalar_references(
        &self,
        formula: &str,
        current_scalar: &str,
    ) -> ForgeResult<String> {
        let mut result = formula.to_string();

        // Find all word boundaries and check if they're scalar references
        // Keep dots to preserve fully-qualified scalar names like "inputs.current_usage_pct"
        let words: Vec<&str> = formula
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .filter(|s| !s.is_empty())
            .collect();

        // Helper to check if a word appears inside a quoted string
        let is_inside_quotes = |word: &str, formula: &str| -> bool {
            // Find all occurrences of the word and check if they're inside quotes
            let mut in_double_quote = false;
            let mut in_single_quote = false;
            let mut i = 0;
            let chars: Vec<char> = formula.chars().collect();

            while i < chars.len() {
                // Track quote state
                if chars[i] == '"' && !in_single_quote {
                    in_double_quote = !in_double_quote;
                } else if chars[i] == '\'' && !in_double_quote {
                    in_single_quote = !in_single_quote;
                }

                // Check if this position starts the word
                if i + word.len() <= chars.len() {
                    let slice: String = chars[i..i + word.len()].iter().collect();
                    if slice == word {
                        // Check word boundaries
                        let before_ok =
                            i == 0 || (!chars[i - 1].is_alphanumeric() && chars[i - 1] != '_');
                        let after_ok = i + word.len() >= chars.len()
                            || (!chars[i + word.len()].is_alphanumeric()
                                && chars[i + word.len()] != '_');

                        if before_ok && after_ok && (in_double_quote || in_single_quote) {
                            return true;
                        }
                    }
                }
                i += 1;
            }
            false
        };

        for word in words {
            // Skip function names and numbers
            if word.chars().next().is_none_or(|c| c.is_numeric()) {
                continue;
            }

            // Skip the current scalar (avoid self-reference) and common function names
            if word == current_scalar {
                continue;
            }

            // Skip words that appear inside quoted strings (for INDIRECT, etc.)
            if is_inside_quotes(word, formula) {
                continue;
            }

            let upper = word.to_uppercase();
            let is_function = matches!(
                upper.as_str(),
                "PMT"
                    | "FV"
                    | "PV"
                    | "NPV"
                    | "IRR"
                    | "NPER"
                    | "RATE"
                    | "XNPV"
                    | "XIRR"
                    | "CHOOSE"
                    | "SUM"
                    | "AVERAGE"
                    | "COUNT"
                    | "MAX"
                    | "MIN"
                    | "IF"
                    | "AND"
                    | "OR"
                    | "NOT"
                    | "TRUE"
                    | "FALSE"
                    | "ROUND"
                    | "ROUNDUP"
                    | "ROUNDDOWN"
                    | "ABS"
                    | "SQRT"
                    | "POWER"
                    | "MOD"
                    | "DATEDIF"
                    | "EDATE"
                    | "EOMONTH"
                    | "DATE"
                    | "YEAR"
                    | "MONTH"
                    | "DAY"
                    | "TODAY"
                    | "NOW"
            );

            if is_function {
                continue;
            }

            // Check if it's a scalar variable
            if let Some(scalar) = self.model.scalars.get(word) {
                if let Some(value) = scalar.value {
                    // Replace the scalar reference with its value
                    // Use word boundary matching to avoid partial replacements
                    let pattern = format!(r"\b{}\b", regex::escape(word));
                    if let Ok(re) = regex::Regex::new(&pattern) {
                        result = re.replace_all(&result, value.to_string()).to_string();
                    }
                }
            }
        }

        Ok(result)
    }

    /// Preprocess formula to replace array indexing with actual values
    /// Converts: =(pl_2025.revenue[3] / pl_2025.revenue[0]) ^ (1/3) - 1
    /// To: =(1800000 / 1000000) ^ (1/3) - 1
    fn preprocess_array_indexing(&self, formula: &str) -> ForgeResult<String> {
        use regex::Regex;

        // Pattern: table_name.column_name[index]
        // Match word characters (including _), a dot, more word characters, then [number]
        let re = Regex::new(r"(\w+)\.(\w+)\[(\d+)\]")
            .map_err(|e| ForgeError::Eval(format!("Regex error: {}", e)))?;

        let mut result = formula.to_string();

        // Find all matches and replace them with their values
        let captures: Vec<_> = re.captures_iter(formula).collect();
        for cap in captures {
            let full_match = cap.get(0).unwrap().as_str();
            let table_name = cap.get(1).unwrap().as_str();
            let col_name = cap.get(2).unwrap().as_str();
            let index_str = cap.get(3).unwrap().as_str();

            let index = index_str
                .parse::<usize>()
                .map_err(|_| ForgeError::Eval(format!("Invalid index: {}", index_str)))?;

            // Get the actual value
            let table = self
                .model
                .tables
                .get(table_name)
                .ok_or_else(|| ForgeError::Eval(format!("Table '{}' not found", table_name)))?;

            let column = table.columns.get(col_name).ok_or_else(|| {
                ForgeError::Eval(format!(
                    "Column '{}' not found in table '{}'",
                    col_name, table_name
                ))
            })?;

            let value = match &column.values {
                ColumnValue::Number(nums) => nums
                    .get(index)
                    .copied()
                    .ok_or_else(|| ForgeError::Eval(format!("Index {} out of bounds", index)))?,
                _ => {
                    return Err(ForgeError::Eval(format!(
                        "Array indexing requires numeric column, got {}",
                        column.values.type_name()
                    )))
                }
            };

            // Replace the array indexing with the actual value
            result = result.replace(full_match, &value.to_string());
        }

        Ok(result)
    }

    /// Evaluate aggregation formula (SUM, AVERAGE, MAX, MIN, conditional variants)
    fn evaluate_aggregation(&self, formula: &str) -> ForgeResult<f64> {
        let upper = formula.to_uppercase();

        // Check for conditional aggregations first (SUMIF, COUNTIF, etc.)
        if upper.contains("SUMIF(") {
            return self.evaluate_conditional_aggregation(formula, "SUMIF");
        } else if upper.contains("COUNTIF(") {
            return self.evaluate_conditional_aggregation(formula, "COUNTIF");
        } else if upper.contains("AVERAGEIF(") {
            return self.evaluate_conditional_aggregation(formula, "AVERAGEIF");
        } else if upper.contains("SUMIFS(") {
            return self.evaluate_conditional_aggregation(formula, "SUMIFS");
        } else if upper.contains("COUNTIFS(") {
            return self.evaluate_conditional_aggregation(formula, "COUNTIFS");
        } else if upper.contains("AVERAGEIFS(") {
            return self.evaluate_conditional_aggregation(formula, "AVERAGEIFS");
        } else if upper.contains("MAXIFS(") {
            return self.evaluate_conditional_aggregation(formula, "MAXIFS");
        } else if upper.contains("MINIFS(") {
            return self.evaluate_conditional_aggregation(formula, "MINIFS");
        }

        // Extract function name and argument for simple aggregations
        let (func_name, arg) = if let Some(start) = upper.find("SUM(") {
            ("SUM", self.extract_function_arg(formula, start + 4)?)
        } else if let Some(start) = upper.find("AVERAGE(") {
            ("AVERAGE", self.extract_function_arg(formula, start + 8)?)
        } else if let Some(start) = upper.find("AVG(") {
            ("AVG", self.extract_function_arg(formula, start + 4)?)
        } else if let Some(start) = upper.find("MAX(") {
            ("MAX", self.extract_function_arg(formula, start + 4)?)
        } else if let Some(start) = upper.find("MIN(") {
            ("MIN", self.extract_function_arg(formula, start + 4)?)
        } else if let Some(start) = upper.find("COUNT(") {
            ("COUNT", self.extract_function_arg(formula, start + 6)?)
        // Statistical functions (v5.0.0)
        } else if let Some(start) = upper.find("MEDIAN(") {
            ("MEDIAN", self.extract_function_arg(formula, start + 7)?)
        } else if let Some(start) = upper.find("VAR.P(") {
            ("VAR.P", self.extract_function_arg(formula, start + 6)?)
        } else if let Some(start) = upper.find("VAR.S(") {
            ("VAR.S", self.extract_function_arg(formula, start + 6)?)
        } else if let Some(start) = upper.find("VAR(") {
            ("VAR", self.extract_function_arg(formula, start + 4)?)
        } else if let Some(start) = upper.find("STDEV.P(") {
            ("STDEV.P", self.extract_function_arg(formula, start + 8)?)
        } else if let Some(start) = upper.find("STDEV.S(") {
            ("STDEV.S", self.extract_function_arg(formula, start + 8)?)
        } else if let Some(start) = upper.find("STDEV(") {
            ("STDEV", self.extract_function_arg(formula, start + 6)?)
        } else if let Some(start) = upper.find("PERCENTILE(") {
            // PERCENTILE has two arguments: array, k
            return self.evaluate_percentile(formula, start + 11);
        } else if let Some(start) = upper.find("QUARTILE(") {
            // QUARTILE has two arguments: array, quart
            return self.evaluate_quartile(formula, start + 9);
        } else if let Some(start) = upper.find("CORREL(") {
            // CORREL has two arguments: array1, array2
            return self.evaluate_correl(formula, start + 7);
        } else {
            return Err(ForgeError::Eval("Unknown aggregation function".to_string()));
        };

        // Check if the argument is already a list of comma-separated values
        // (e.g., from OFFSET resolution: "300, 400, 500")
        if arg.contains(',') && !arg.contains('.') {
            // Parse comma-separated numeric values
            let nums: Result<Vec<f64>, _> =
                arg.split(',').map(|s| s.trim().parse::<f64>()).collect();

            if let Ok(nums) = nums {
                let result = match func_name {
                    "SUM" => nums.iter().sum(),
                    "AVERAGE" | "AVG" => {
                        if nums.is_empty() {
                            0.0
                        } else {
                            nums.iter().sum::<f64>() / nums.len() as f64
                        }
                    }
                    "MAX" => nums.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    "MIN" => nums.iter().copied().fold(f64::INFINITY, f64::min),
                    "COUNT" => nums.len() as f64,
                    // Statistical functions (v5.0.0)
                    "MEDIAN" => Self::calculate_median(&nums),
                    "VAR" | "VAR.S" => Self::calculate_variance(&nums, true), // Sample variance
                    "VAR.P" => Self::calculate_variance(&nums, false),        // Population variance
                    "STDEV" | "STDEV.S" => Self::calculate_stdev(&nums, true), // Sample stdev
                    "STDEV.P" => Self::calculate_stdev(&nums, false),         // Population stdev
                    _ => {
                        return Err(ForgeError::Eval(format!(
                            "Unsupported aggregation function: {}",
                            func_name
                        )))
                    }
                };
                return Ok(result);
            }
        }

        // Parse table.column reference
        let (table_name, col_name) = self.parse_table_column_ref(&arg)?;

        // Get the column
        let table = self
            .model
            .tables
            .get(&table_name)
            .ok_or_else(|| ForgeError::Eval(format!("Table '{}' not found", table_name)))?;

        let column = table.columns.get(&col_name).ok_or_else(|| {
            ForgeError::Eval(format!(
                "Column '{}' not found in table '{}'",
                col_name, table_name
            ))
        })?;

        // Apply aggregation function
        // COUNT works on any column type - it just counts rows
        if func_name == "COUNT" {
            return Ok(column.values.len() as f64);
        }

        // Other aggregations require numeric columns
        match &column.values {
            ColumnValue::Number(nums) => {
                let result = match func_name {
                    "SUM" => nums.iter().sum(),
                    "AVERAGE" | "AVG" => {
                        if nums.is_empty() {
                            0.0
                        } else {
                            nums.iter().sum::<f64>() / nums.len() as f64
                        }
                    }
                    "MAX" => nums.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    "MIN" => nums.iter().copied().fold(f64::INFINITY, f64::min),
                    // Statistical functions (v5.0.0)
                    "MEDIAN" => Self::calculate_median(nums),
                    "VAR" | "VAR.S" => Self::calculate_variance(nums, true),
                    "VAR.P" => Self::calculate_variance(nums, false),
                    "STDEV" | "STDEV.S" => Self::calculate_stdev(nums, true),
                    "STDEV.P" => Self::calculate_stdev(nums, false),
                    _ => {
                        return Err(ForgeError::Eval(format!(
                            "Unsupported aggregation function: {}",
                            func_name
                        )))
                    }
                };
                Ok(result)
            }
            _ => Err(ForgeError::Eval(format!(
                "Aggregation functions require numeric columns, got {}",
                column.values.type_name()
            ))),
        }
    }

    /// Calculate median of a slice of numbers
    fn calculate_median(nums: &[f64]) -> f64 {
        if nums.is_empty() {
            return 0.0;
        }
        let mut sorted = nums.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        if sorted.len().is_multiple_of(2) {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }

    /// Calculate variance (sample or population)
    fn calculate_variance(nums: &[f64], sample: bool) -> f64 {
        if nums.is_empty() || (sample && nums.len() < 2) {
            return 0.0;
        }
        let mean = nums.iter().sum::<f64>() / nums.len() as f64;
        let sum_sq: f64 = nums.iter().map(|x| (x - mean).powi(2)).sum();
        let divisor = if sample {
            (nums.len() - 1) as f64
        } else {
            nums.len() as f64
        };
        sum_sq / divisor
    }

    /// Calculate standard deviation (sample or population)
    fn calculate_stdev(nums: &[f64], sample: bool) -> f64 {
        Self::calculate_variance(nums, sample).sqrt()
    }

    /// Calculate percentile value
    /// k should be between 0 and 1 (e.g., 0.25 for 25th percentile)
    fn calculate_percentile(nums: &[f64], k: f64) -> f64 {
        if nums.is_empty() {
            return 0.0;
        }
        let mut sorted = nums.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = sorted.len() as f64;
        let rank = k * (n - 1.0);
        let lower = rank.floor() as usize;
        let upper = rank.ceil() as usize;
        let frac = rank - rank.floor();

        if lower == upper || upper >= sorted.len() {
            sorted[lower.min(sorted.len() - 1)]
        } else {
            sorted[lower] + frac * (sorted[upper] - sorted[lower])
        }
    }

    /// Evaluate PERCENTILE function: PERCENTILE(array, k)
    fn evaluate_percentile(&self, formula: &str, start: usize) -> ForgeResult<f64> {
        // Extract arguments from PERCENTILE(array, k)
        let rest = &formula[start..];
        let end = rest.find(')').ok_or_else(|| {
            ForgeError::Eval("Missing closing parenthesis in PERCENTILE".to_string())
        })?;
        let args = &rest[..end];
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() != 2 {
            return Err(ForgeError::Eval(
                "PERCENTILE requires exactly 2 arguments: array, k".to_string(),
            ));
        }

        let array_ref = parts[0].trim();
        let k: f64 = parts[1].trim().parse().map_err(|_| {
            ForgeError::Eval("PERCENTILE k must be a number between 0 and 1".to_string())
        })?;

        if !(0.0..=1.0).contains(&k) {
            return Err(ForgeError::Eval(
                "PERCENTILE k must be between 0 and 1".to_string(),
            ));
        }

        // Get the array values
        let nums = self.get_numeric_array(array_ref)?;
        Ok(Self::calculate_percentile(&nums, k))
    }

    /// Evaluate QUARTILE function: QUARTILE(array, quart)
    fn evaluate_quartile(&self, formula: &str, start: usize) -> ForgeResult<f64> {
        // Extract arguments from QUARTILE(array, quart)
        let rest = &formula[start..];
        let end = rest.find(')').ok_or_else(|| {
            ForgeError::Eval("Missing closing parenthesis in QUARTILE".to_string())
        })?;
        let args = &rest[..end];
        let parts: Vec<&str> = args.split(',').collect();
        if parts.len() != 2 {
            return Err(ForgeError::Eval(
                "QUARTILE requires exactly 2 arguments: array, quart".to_string(),
            ));
        }

        let array_ref = parts[0].trim();
        let quart: i32 = parts[1]
            .trim()
            .parse()
            .map_err(|_| ForgeError::Eval("QUARTILE quart must be an integer 0-4".to_string()))?;

        if !(0..=4).contains(&quart) {
            return Err(ForgeError::Eval(
                "QUARTILE quart must be between 0 and 4".to_string(),
            ));
        }

        // Get the array values
        let nums = self.get_numeric_array(array_ref)?;

        // QUARTILE(data, 0) = MIN, QUARTILE(data, 4) = MAX
        let k = quart as f64 / 4.0;
        Ok(Self::calculate_percentile(&nums, k))
    }

    /// Helper: Get numeric array from a table.column reference or comma-separated values
    fn get_numeric_array(&self, array_ref: &str) -> ForgeResult<Vec<f64>> {
        // Check if it's comma-separated values
        if array_ref.contains(',') && !array_ref.contains('.') {
            let nums: Result<Vec<f64>, _> = array_ref
                .split(',')
                .map(|s| s.trim().parse::<f64>())
                .collect();
            return nums.map_err(|_| ForgeError::Eval("Invalid numeric values".to_string()));
        }

        // Parse table.column reference
        let (table_name, col_name) = self.parse_table_column_ref(array_ref)?;

        let table = self
            .model
            .tables
            .get(&table_name)
            .ok_or_else(|| ForgeError::Eval(format!("Table '{}' not found", table_name)))?;

        let column = table.columns.get(&col_name).ok_or_else(|| {
            ForgeError::Eval(format!(
                "Column '{}' not found in table '{}'",
                col_name, table_name
            ))
        })?;

        match &column.values {
            ColumnValue::Number(nums) => Ok(nums.clone()),
            _ => Err(ForgeError::Eval(format!(
                "PERCENTILE/QUARTILE require numeric columns, got {}",
                column.values.type_name()
            ))),
        }
    }

    /// Evaluate CORREL function: CORREL(array1, array2)
    /// Returns the correlation coefficient between two arrays
    fn evaluate_correl(&self, formula: &str, start: usize) -> ForgeResult<f64> {
        let rest = &formula[start..];
        let end = rest
            .find(')')
            .ok_or_else(|| ForgeError::Eval("Missing closing parenthesis in CORREL".to_string()))?;
        let args = &rest[..end];

        // Split on comma, but handle table.column references
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        if parts.len() != 2 {
            return Err(ForgeError::Eval(
                "CORREL requires exactly 2 arguments: array1, array2".to_string(),
            ));
        }

        let array1 = self.get_numeric_array(parts[0].trim())?;
        let array2 = self.get_numeric_array(parts[1].trim())?;

        if array1.len() != array2.len() {
            return Err(ForgeError::Eval(
                "CORREL arrays must have the same length".to_string(),
            ));
        }

        if array1.is_empty() {
            return Ok(0.0);
        }

        // Calculate means
        let mean1 = array1.iter().sum::<f64>() / array1.len() as f64;
        let mean2 = array2.iter().sum::<f64>() / array2.len() as f64;

        // Calculate correlation coefficient
        let mut cov = 0.0;
        let mut var1 = 0.0;
        let mut var2 = 0.0;

        for (x, y) in array1.iter().zip(array2.iter()) {
            let dx = x - mean1;
            let dy = y - mean2;
            cov += dx * dy;
            var1 += dx * dx;
            var2 += dy * dy;
        }

        let denominator = (var1 * var2).sqrt();
        if denominator == 0.0 {
            return Ok(0.0); // No variance = no correlation
        }

        Ok(cov / denominator)
    }

    /// Evaluate conditional aggregation (SUMIF, COUNTIF, AVERAGEIF, etc.)
    /// Syntax examples:
    /// - SUMIF(range, criteria, sum_range)
    /// - COUNTIF(range, criteria)
    /// - AVERAGEIF(range, criteria, avg_range)
    /// - SUMIFS(sum_range, criteria_range1, criteria1, criteria_range2, criteria2, ...)
    /// - MAXIFS(max_range, criteria_range1, criteria1, ...)
    fn evaluate_conditional_aggregation(&self, formula: &str, func_name: &str) -> ForgeResult<f64> {
        let upper = formula.to_uppercase();
        let func_pattern = format!("{}(", func_name);

        let start = upper
            .find(&func_pattern)
            .ok_or_else(|| ForgeError::Eval(format!("Function {} not found", func_name)))?
            + func_pattern.len();

        let args_str = self.extract_function_arg(formula, start)?;
        let args = self.parse_function_args(&args_str)?;

        match func_name {
            "SUMIF" | "COUNTIF" | "AVERAGEIF" => {
                self.evaluate_single_criteria_aggregation(func_name, &args)
            }
            "SUMIFS" | "COUNTIFS" | "AVERAGEIFS" | "MAXIFS" | "MINIFS" => {
                self.evaluate_multiple_criteria_aggregation(func_name, &args)
            }
            _ => Err(ForgeError::Eval(format!(
                "Unknown conditional aggregation: {}",
                func_name
            ))),
        }
    }

    /// Evaluate single-criteria conditional aggregations (SUMIF, COUNTIF, AVERAGEIF)
    fn evaluate_single_criteria_aggregation(
        &self,
        func_name: &str,
        args: &[String],
    ) -> ForgeResult<f64> {
        // Validate argument count
        let expected_args = if func_name == "COUNTIF" { 2 } else { 3 };
        if args.len() != expected_args {
            return Err(ForgeError::Eval(format!(
                "{} requires {} arguments, got {}",
                func_name,
                expected_args,
                args.len()
            )));
        }

        // Parse the criteria range
        let (criteria_table, criteria_col) = self.parse_table_column_ref(args[0].trim())?;
        let criteria_str = args[1].trim();

        // Get criteria column
        let table = self
            .model
            .tables
            .get(&criteria_table)
            .ok_or_else(|| ForgeError::Eval(format!("Table '{}' not found", criteria_table)))?;

        let criteria_column = table.columns.get(&criteria_col).ok_or_else(|| {
            ForgeError::Eval(format!(
                "Column '{}' not found in table '{}'",
                criteria_col, criteria_table
            ))
        })?;

        // Get the sum/average range (if applicable)
        let value_column = if func_name == "COUNTIF" {
            criteria_column
        } else {
            let (value_table_name, value_col) = self.parse_table_column_ref(args[2].trim())?;
            let value_table = self.model.tables.get(&value_table_name).ok_or_else(|| {
                ForgeError::Eval(format!("Table '{}' not found", value_table_name))
            })?;

            value_table.columns.get(&value_col).ok_or_else(|| {
                ForgeError::Eval(format!(
                    "Column '{}' not found in table '{}'",
                    value_col, value_table_name
                ))
            })?
        };

        // Apply the criteria and aggregate
        match (&criteria_column.values, &value_column.values) {
            (ColumnValue::Number(criteria_nums), ColumnValue::Number(value_nums)) => {
                if func_name != "COUNTIF" && criteria_nums.len() != value_nums.len() {
                    return Err(ForgeError::Eval(format!(
                        "Criteria range and value range must have same length: {} vs {}",
                        criteria_nums.len(),
                        value_nums.len()
                    )));
                }

                let matches: Vec<f64> = criteria_nums
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &crit_val)| {
                        if self.matches_criteria(crit_val, criteria_str).unwrap_or(false) {
                            if func_name == "COUNTIF" {
                                Some(1.0)
                            } else {
                                value_nums.get(i).copied()
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                let result = match func_name {
                    "SUMIF" | "COUNTIF" => matches.iter().sum(),
                    "AVERAGEIF" => {
                        if matches.is_empty() {
                            0.0
                        } else {
                            matches.iter().sum::<f64>() / matches.len() as f64
                        }
                    }
                    _ => {
                        return Err(ForgeError::Eval(format!(
                            "Unsupported function: {}",
                            func_name
                        )))
                    }
                };

                Ok(result)
            }
            (ColumnValue::Text(criteria_text), ColumnValue::Number(value_nums)) => {
                if func_name != "COUNTIF" && criteria_text.len() != value_nums.len() {
                    return Err(ForgeError::Eval(format!(
                        "Criteria range and value range must have same length: {} vs {}",
                        criteria_text.len(),
                        value_nums.len()
                    )));
                }

                let matches: Vec<f64> = criteria_text
                    .iter()
                    .enumerate()
                    .filter_map(|(i, crit_val)| {
                        if self.matches_text_criteria(crit_val, criteria_str).unwrap_or(false) {
                            if func_name == "COUNTIF" {
                                Some(1.0)
                            } else {
                                value_nums.get(i).copied()
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                let result = match func_name {
                    "SUMIF" | "COUNTIF" => matches.iter().sum(),
                    "AVERAGEIF" => {
                        if matches.is_empty() {
                            0.0
                        } else {
                            matches.iter().sum::<f64>() / matches.len() as f64
                        }
                    }
                    _ => {
                        return Err(ForgeError::Eval(format!(
                            "Unsupported function: {}",
                            func_name
                        )))
                    }
                };

                Ok(result)
            }
            // COUNTIF with text criteria (range and criteria only)
            (ColumnValue::Text(criteria_text), ColumnValue::Text(_)) if func_name == "COUNTIF" => {
                let count = criteria_text
                    .iter()
                    .filter(|crit_val| {
                        self.matches_text_criteria(crit_val, criteria_str).unwrap_or(false)
                    })
                    .count() as f64;

                Ok(count)
            }
            _ => Err(ForgeError::Eval(format!(
                "{} requires compatible column types (numeric criteria with numeric values, or text with text for COUNTIF)",
                func_name
            ))),
        }
    }

    /// Evaluate multiple-criteria conditional aggregations (SUMIFS, COUNTIFS, AVERAGEIFS, MAXIFS, MINIFS)
    fn evaluate_multiple_criteria_aggregation(
        &self,
        func_name: &str,
        args: &[String],
    ) -> ForgeResult<f64> {
        // COUNTIFS has a different arg structure: pairs of criteria_range/criteria only
        // SUMIFS/AVERAGEIFS/MAXIFS/MINIFS: value_range, criteria_range1, criteria1, ...

        let (value_table, value_col, criteria_start_idx) = if func_name == "COUNTIFS" {
            // COUNTIFS: just pairs of criteria_range/criteria
            if args.is_empty() || !args.len().is_multiple_of(2) {
                return Err(ForgeError::Eval(
                    "COUNTIFS requires even number of arguments (criteria_range1, criteria1, ...)"
                        .to_string(),
                ));
            }
            // Use first criteria range as the value range for counting
            let (table, col) = self.parse_table_column_ref(args[0].trim())?;
            (table, col, 0)
        } else {
            // Other *IFS functions: value_range + pairs
            if args.is_empty() || (args.len() % 2) != 1 {
                return Err(ForgeError::Eval(format!(
                    "{} requires odd number of arguments (value_range, criteria_range1, criteria1, ...)",
                    func_name
                )));
            }
            let (table, col) = self.parse_table_column_ref(args[0].trim())?;
            (table, col, 1)
        };

        // Parse the value range
        let (value_table, value_col) = (value_table, value_col);

        let table = self
            .model
            .tables
            .get(&value_table)
            .ok_or_else(|| ForgeError::Eval(format!("Table '{}' not found", value_table)))?;

        let value_column = table.columns.get(&value_col).ok_or_else(|| {
            ForgeError::Eval(format!(
                "Column '{}' not found in table '{}'",
                value_col, value_table
            ))
        })?;

        // Get value numbers (or row count for COUNTIFS)
        let row_count = if func_name == "COUNTIFS" {
            // For COUNTIFS, we just need the row count from any column
            match &value_column.values {
                ColumnValue::Number(nums) => nums.len(),
                ColumnValue::Text(texts) => texts.len(),
                ColumnValue::Date(dates) => dates.len(),
                ColumnValue::Boolean(bools) => bools.len(),
            }
        } else {
            // For other *IFS functions, we need numeric values
            match &value_column.values {
                ColumnValue::Number(nums) => nums.len(),
                _ => {
                    return Err(ForgeError::Eval(format!(
                        "{} requires numeric value range",
                        func_name
                    )))
                }
            }
        };

        let value_nums = if func_name != "COUNTIFS" {
            match &value_column.values {
                ColumnValue::Number(nums) => Some(nums),
                _ => None,
            }
        } else {
            None
        };

        // Build a mask of which rows match ALL criteria
        let mut matching_rows: Vec<bool> = vec![true; row_count];

        // Process each criteria pair
        for i in (criteria_start_idx..args.len()).step_by(2) {
            let (criteria_table, criteria_col) = self.parse_table_column_ref(args[i].trim())?;
            let criteria_str = args[i + 1].trim();

            let table =
                self.model.tables.get(&criteria_table).ok_or_else(|| {
                    ForgeError::Eval(format!("Table '{}' not found", criteria_table))
                })?;

            let criteria_column = table.columns.get(&criteria_col).ok_or_else(|| {
                ForgeError::Eval(format!(
                    "Column '{}' not found in table '{}'",
                    criteria_col, criteria_table
                ))
            })?;

            // Apply this criteria to the mask
            match &criteria_column.values {
                ColumnValue::Number(criteria_nums) => {
                    if criteria_nums.len() != row_count {
                        return Err(ForgeError::Eval(format!(
                            "All ranges must have same length: {} vs {}",
                            criteria_nums.len(),
                            row_count
                        )));
                    }

                    for (j, &crit_val) in criteria_nums.iter().enumerate() {
                        if !self
                            .matches_criteria(crit_val, criteria_str)
                            .unwrap_or(false)
                        {
                            matching_rows[j] = false;
                        }
                    }
                }
                ColumnValue::Text(criteria_text) => {
                    if criteria_text.len() != row_count {
                        return Err(ForgeError::Eval(format!(
                            "All ranges must have same length: {} vs {}",
                            criteria_text.len(),
                            row_count
                        )));
                    }

                    for (j, crit_val) in criteria_text.iter().enumerate() {
                        if !self
                            .matches_text_criteria(crit_val, criteria_str)
                            .unwrap_or(false)
                        {
                            matching_rows[j] = false;
                        }
                    }
                }
                _ => {
                    return Err(ForgeError::Eval(format!(
                        "{} criteria must be numeric or text",
                        func_name
                    )))
                }
            }
        }

        // Aggregate the matching values
        let result = if func_name == "COUNTIFS" {
            // COUNTIFS just counts matching rows
            matching_rows.iter().filter(|&&m| m).count() as f64
        } else {
            // Other *IFS functions need to aggregate numeric values
            let nums = value_nums.ok_or_else(|| {
                ForgeError::Eval(format!("{} requires numeric value range", func_name))
            })?;

            let matched_values: Vec<f64> = nums
                .iter()
                .enumerate()
                .filter_map(|(i, &val)| if matching_rows[i] { Some(val) } else { None })
                .collect();

            match func_name {
                "SUMIFS" => matched_values.iter().sum(),
                "AVERAGEIFS" => {
                    if matched_values.is_empty() {
                        0.0
                    } else {
                        matched_values.iter().sum::<f64>() / matched_values.len() as f64
                    }
                }
                "MAXIFS" => {
                    if matched_values.is_empty() {
                        f64::NEG_INFINITY
                    } else {
                        matched_values
                            .iter()
                            .copied()
                            .fold(f64::NEG_INFINITY, f64::max)
                    }
                }
                "MINIFS" => {
                    if matched_values.is_empty() {
                        f64::INFINITY
                    } else {
                        matched_values.iter().copied().fold(f64::INFINITY, f64::min)
                    }
                }
                _ => {
                    return Err(ForgeError::Eval(format!(
                        "Unsupported function: {}",
                        func_name
                    )))
                }
            }
        };

        Ok(result)
    }

    /// Check if a numeric value matches a criteria string
    /// Supports: =value, >value, <value, >=value, <=value, <>value
    fn matches_criteria(&self, value: f64, criteria: &str) -> ForgeResult<bool> {
        let criteria = criteria.trim();

        // Remove quotes if present
        let criteria = criteria.trim_matches('"').trim_matches('\'');

        if let Some(stripped) = criteria.strip_prefix(">=") {
            let threshold = stripped
                .trim()
                .parse::<f64>()
                .map_err(|_| ForgeError::Eval(format!("Invalid criteria: {}", criteria)))?;
            Ok(value >= threshold)
        } else if let Some(stripped) = criteria.strip_prefix("<=") {
            let threshold = stripped
                .trim()
                .parse::<f64>()
                .map_err(|_| ForgeError::Eval(format!("Invalid criteria: {}", criteria)))?;
            Ok(value <= threshold)
        } else if let Some(stripped) = criteria.strip_prefix("<>") {
            let threshold = stripped
                .trim()
                .parse::<f64>()
                .map_err(|_| ForgeError::Eval(format!("Invalid criteria: {}", criteria)))?;
            Ok((value - threshold).abs() > 1e-10)
        } else if let Some(stripped) = criteria.strip_prefix('>') {
            let threshold = stripped
                .trim()
                .parse::<f64>()
                .map_err(|_| ForgeError::Eval(format!("Invalid criteria: {}", criteria)))?;
            Ok(value > threshold)
        } else if let Some(stripped) = criteria.strip_prefix('<') {
            let threshold = stripped
                .trim()
                .parse::<f64>()
                .map_err(|_| ForgeError::Eval(format!("Invalid criteria: {}", criteria)))?;
            Ok(value < threshold)
        } else if let Some(stripped) = criteria.strip_prefix('=') {
            let threshold = stripped
                .trim()
                .parse::<f64>()
                .map_err(|_| ForgeError::Eval(format!("Invalid criteria: {}", criteria)))?;
            Ok((value - threshold).abs() < 1e-10)
        } else {
            // No operator - assume equality
            let threshold = criteria
                .parse::<f64>()
                .map_err(|_| ForgeError::Eval(format!("Invalid criteria: {}", criteria)))?;
            Ok((value - threshold).abs() < 1e-10)
        }
    }

    /// Check if a text value matches a criteria string
    /// Supports: =text, <>text, or simple text match
    fn matches_text_criteria(&self, value: &str, criteria: &str) -> ForgeResult<bool> {
        let criteria = criteria.trim();

        // Remove quotes if present
        let criteria = criteria.trim_matches('"').trim_matches('\'');

        if let Some(stripped) = criteria.strip_prefix("<>") {
            let text = stripped.trim();
            Ok(value != text)
        } else if let Some(stripped) = criteria.strip_prefix('=') {
            let text = stripped.trim();
            Ok(value == text)
        } else {
            // Simple equality check
            Ok(value == criteria)
        }
    }

    /// Parse comma-separated function arguments
    /// Handles nested parentheses and quoted strings
    fn parse_function_args(&self, args_str: &str) -> ForgeResult<Vec<String>> {
        let mut args = Vec::new();
        let mut current_arg = String::new();
        let mut paren_depth = 0;
        let mut in_quotes = false;
        let mut quote_char = ' ';

        for ch in args_str.chars() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                    current_arg.push(ch);
                }
                '"' | '\'' if in_quotes && ch == quote_char => {
                    in_quotes = false;
                    current_arg.push(ch);
                }
                '(' if !in_quotes => {
                    paren_depth += 1;
                    current_arg.push(ch);
                }
                ')' if !in_quotes => {
                    paren_depth -= 1;
                    current_arg.push(ch);
                }
                ',' if !in_quotes && paren_depth == 0 => {
                    args.push(current_arg.trim().to_string());
                    current_arg.clear();
                }
                _ => {
                    current_arg.push(ch);
                }
            }
        }

        if !current_arg.trim().is_empty() {
            args.push(current_arg.trim().to_string());
        }

        Ok(args)
    }

    /// Evaluate array indexing (e.g., table.column[3])
    fn evaluate_array_indexing(&self, formula: &str) -> ForgeResult<f64> {
        // Find the array reference pattern: table.column[index]
        let formula = formula.trim_start_matches('=').trim();

        let bracket_pos = formula
            .find('[')
            .ok_or_else(|| ForgeError::Eval("Missing '[' in array index".to_string()))?;

        let table_col = &formula[..bracket_pos];
        let index_part = &formula[bracket_pos + 1..];

        let index_end = index_part
            .find(']')
            .ok_or_else(|| ForgeError::Eval("Missing ']' in array index".to_string()))?;

        let index_str = &index_part[..index_end];
        let index = index_str
            .parse::<usize>()
            .map_err(|_| ForgeError::Eval(format!("Invalid array index: {}", index_str)))?;

        // Parse table.column reference
        let (table_name, col_name) = self.parse_table_column_ref(table_col)?;

        // Get the column value at index
        let table = self
            .model
            .tables
            .get(&table_name)
            .ok_or_else(|| ForgeError::Eval(format!("Table '{}' not found", table_name)))?;

        let column = table
            .columns
            .get(&col_name)
            .ok_or_else(|| ForgeError::Eval(format!("Column '{}' not found", col_name)))?;

        match &column.values {
            ColumnValue::Number(nums) => nums
                .get(index)
                .copied()
                .ok_or_else(|| ForgeError::Eval(format!("Index {} out of bounds", index))),
            _ => Err(ForgeError::Eval(format!(
                "Array indexing requires numeric column, got {}",
                column.values.type_name()
            ))),
        }
    }

    /// Preprocess formula to resolve @namespace.field references (v4.0 cross-file refs)
    fn preprocess_namespace_refs(&self, formula: &str) -> String {
        let mut result = formula.to_string();

        // Find and replace @namespace.field patterns
        // Pattern: @word.word(.word)*
        let mut i = 0;
        while let Some(at_pos) = result[i..].find('@') {
            let start = i + at_pos;

            // Extract the full reference (everything until a non-identifier char)
            let ref_end = result[start + 1..]
                .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                .map(|pos| start + 1 + pos)
                .unwrap_or(result.len());

            let reference = &result[start..ref_end];

            // Try to resolve the reference
            if let Some(value) = self.model.resolve_namespace_ref(reference) {
                // Replace with the actual value
                result = format!("{}{}{}", &result[..start], value, &result[ref_end..]);
                i = start + value.to_string().len();
            } else {
                // Skip this @ and continue looking
                i = start + 1;
            }
        }

        result
    }

    /// Evaluate scalar formula with variable resolver
    fn evaluate_scalar_with_resolver(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        // Preprocess to resolve @namespace.field references (v4.0)
        let formula = self.preprocess_namespace_refs(formula);

        // Preprocess to resolve fully-qualified scalar references (v4.3.0)
        // This handles "inputs.current_usage_pct" style references before xlformula_engine parsing
        let formula = self.resolve_scalar_references(&formula, scalar_name)?;

        // Extract parent section from scalar_name (e.g., "annual_2025" from "annual_2025.total_revenue")
        let parent_section = scalar_name
            .rfind('.')
            .map(|dot_pos| scalar_name[..dot_pos].to_string());

        let resolver = move |var_name: String| -> types::Value {
            // Strategy 1: Try exact match
            if let Some(var) = self.model.scalars.get(&var_name) {
                if let Some(value) = var.value {
                    return types::Value::Number(value as f32);
                }
            }

            // Strategy 2: If we're in a section and var_name is simple, try prefixing with parent section
            if let Some(ref section) = parent_section {
                if !var_name.contains('.') {
                    // Simple name - try prefixing with parent section
                    let scoped_name = format!("{}.{}", section, var_name);
                    if let Some(var) = self.model.scalars.get(&scoped_name) {
                        if let Some(value) = var.value {
                            return types::Value::Number(value as f32);
                        }
                    }
                }
            }

            // Strategy 3: Try table.column reference (returns first value)
            if var_name.contains('.') {
                if let Ok((table_name, col_name)) = self.parse_table_column_ref(&var_name) {
                    if let Some(table) = self.model.tables.get(&table_name) {
                        if let Some(column) = table.columns.get(&col_name) {
                            if let ColumnValue::Number(nums) = &column.values {
                                if let Some(&first) = nums.first() {
                                    return types::Value::Number(first as f32);
                                }
                            }
                        }
                    }
                }
            }

            types::Value::Error(types::Error::Value)
        };

        let parsed = parse_formula::parse_string_to_formula(&formula, None::<NoCustomFunction>);
        let result = calculate::calculate_formula(parsed, Some(&resolver));

        match result {
            types::Value::Number(n) => Ok(n as f64),
            types::Value::Error(e) => Err(ForgeError::Eval(format!(
                "Formula '{}' returned error: {:?}",
                &formula, e
            ))),
            other => Err(ForgeError::Eval(format!(
                "Formula '{}' returned unexpected type: {:?}",
                &formula, other
            ))),
        }
    }

    /// Parse table.column reference
    fn parse_table_column_ref(&self, ref_str: &str) -> ForgeResult<(String, String)> {
        let parts: Vec<&str> = ref_str.trim().split('.').collect();
        if parts.len() == 2 {
            Ok((parts[0].to_string(), parts[1].to_string()))
        } else {
            Err(ForgeError::Eval(format!(
                "Invalid table.column reference: {}",
                ref_str
            )))
        }
    }

    /// Extract function argument from formula
    fn extract_function_arg(&self, formula: &str, start: usize) -> ForgeResult<String> {
        let rest = &formula[start..];
        let end = rest
            .find(')')
            .ok_or_else(|| ForgeError::Eval("Missing closing parenthesis".to_string()))?;

        Ok(rest[..end].trim().to_string())
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
                    "SUM"
                        | "AVERAGE"
                        | "AVG"
                        | "MAX"
                        | "MIN"
                        | "COUNT"
                        | "SUMIF"
                        | "COUNTIF"
                        | "AVERAGEIF"
                        | "SUMIFS"
                        | "COUNTIFS"
                        | "AVERAGEIFS"
                        | "MAXIFS"
                        | "MINIFS"
                        | "IF"
                        | "AND"
                        | "OR"
                        | "NOT"
                        | "ABS"
                        | "ROUND"
                        | "ROUNDUP"
                        | "ROUNDDOWN"
                        | "CEILING"
                        | "FLOOR"
                        | "MOD"
                        | "POWER"
                        | "SQRT"
                        | "POW"
                        | "EXP"
                        | "LN"
                        | "LOG"
                        | "CONCAT"
                        | "TRIM"
                        | "UPPER"
                        | "LOWER"
                        | "LEN"
                        | "MID"
                        | "LEFT"
                        | "RIGHT"
                        | "TODAY"
                        | "NOW"
                        | "DATE"
                        | "YEAR"
                        | "MONTH"
                        | "DAY"
                        | "DATEDIF"
                        | "EDATE"
                        | "EOMONTH"
                        | "MATCH"
                        | "INDEX"
                        | "VLOOKUP"
                        | "XLOOKUP"
                ) && !refs.contains(&word.to_string())
                {
                    refs.push(word.to_string());
                }
            }
        }

        Ok(refs)
    }

    // ============================================================================
    // Custom Function Preprocessing (v1.1.0)
    // ============================================================================

    /// Preprocess formula to replace fully-qualified scalar references with their values (v4.3.0)
    /// This is called before xlformula_engine evaluation for row-wise table formulas
    /// to handle references like `thresholds.min_value` in formulas like `=IF(amount > thresholds.min_value, 1, 0)`
    fn preprocess_scalar_refs_for_table(&self, formula: &str) -> ForgeResult<String> {
        let mut result = formula.to_string();

        // Sort scalars by key length (longest first) to avoid partial replacements
        // e.g., "inputs.tax_rate" should be replaced before "inputs.tax"
        let mut scalar_keys: Vec<_> = self.model.scalars.keys().collect();
        scalar_keys.sort_by_key(|k| std::cmp::Reverse(k.len()));

        for key in scalar_keys {
            // Only preprocess fully-qualified references (section.name)
            if key.contains('.') {
                if let Some(scalar) = self.model.scalars.get(key) {
                    if let Some(value) = scalar.value {
                        // Replace the scalar reference with its value
                        // Use word boundary matching to avoid partial replacements
                        let pattern = format!(r"\b{}\b", regex::escape(key));
                        if let Ok(re) = regex::Regex::new(&pattern) {
                            result = re.replace_all(&result, value.to_string()).to_string();
                        }
                    }
                }
            }
        }

        // Fix xlformula_engine bug: wrap IF conditions with parentheses (v4.3.0)
        // =IF(x > 50, ...) fails, but =IF((x > 50), ...) works
        result = self.fix_if_conditions(&result);

        Ok(result)
    }

    /// Fix xlformula_engine parsing bug with IF expressions (v4.3.0)
    /// The engine fails to parse IF conditions and expressions containing operators
    /// This wraps IF parts with parentheses to work around the bug
    fn fix_if_conditions(&self, formula: &str) -> String {
        use regex::Regex;

        let mut result = formula.to_string();

        // Step 1: Wrap IF conditions containing comparison operators
        // =IF(x > 50, ...) -> =IF((x > 50), ...)
        let cond_pattern = Regex::new(r"(?i)IF\s*\(\s*([^(),]+\s*(?:>=?|<=?|<>|=)\s*[^(),]+)\s*,")
            .expect("Invalid IF condition regex");

        loop {
            if let Some(caps) = cond_pattern.captures(&result) {
                let condition = &caps[1];
                let trimmed = condition.trim();
                if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
                    let full_match = caps.get(0).unwrap().as_str();
                    let replacement = format!("IF(({}),", condition);
                    result = result.replacen(full_match, &replacement, 1);
                    continue;
                }
            }
            break;
        }

        // Step 2: Wrap IF then/else expressions containing operators
        // =IF((cond), x * 2, y) -> =IF((cond), (x * 2), y)
        // Pattern: Match IF with parenthesized condition, then capture the expression parts
        let expr_pattern =
            Regex::new(r"(?i)IF\s*\(\s*\([^)]+\)\s*,\s*([^,()]+\s*[+\-*/]\s*[^,()]+)\s*,")
                .expect("Invalid IF then expr regex");

        loop {
            if let Some(caps) = expr_pattern.captures(&result) {
                let expr = &caps[1];
                let trimmed = expr.trim();
                if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
                    let full_match = caps.get(0).unwrap().as_str();
                    // Reconstruct with parentheses around the expression
                    let prefix = &full_match[..full_match.rfind(expr).unwrap()];
                    let suffix = &full_match[full_match.rfind(expr).unwrap() + expr.len()..];
                    let replacement = format!("{}({}){}", prefix, expr, suffix);
                    result = result.replacen(full_match, &replacement, 1);
                    continue;
                }
            }
            break;
        }

        // Step 3: Wrap IF else expressions containing operators
        // =IF((cond), (then), x * 2) -> =IF((cond), (then), (x * 2))
        let else_pattern = Regex::new(
            r"(?i)IF\s*\([^)]+\)\s*,\s*\([^)]+\)\s*,\s*([^()]+\s*[+\-*/]\s*[^()]+)\s*\)",
        )
        .expect("Invalid IF else expr regex");

        loop {
            if let Some(caps) = else_pattern.captures(&result) {
                let expr = &caps[1];
                let trimmed = expr.trim();
                if !trimmed.starts_with('(') || !trimmed.ends_with(')') {
                    let full_match = caps.get(0).unwrap().as_str();
                    // Reconstruct with parentheses around the expression
                    let prefix = &full_match[..full_match.rfind(expr).unwrap()];
                    let suffix = &full_match[full_match.rfind(expr).unwrap() + expr.len()..];
                    let replacement = format!("{}({}){}", prefix, expr, suffix);
                    result = result.replacen(full_match, &replacement, 1);
                    continue;
                }
            }
            break;
        }

        result
    }

    /// Preprocess formula to handle custom functions
    /// This is called before xlformula_engine evaluation for row-wise formulas
    fn preprocess_custom_functions(
        &self,
        formula: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        let mut result = formula.to_string();

        // Phase 2: Math functions
        if self.has_custom_math_function(formula) {
            result = self.replace_math_functions(&result, row_idx, table)?;
        }

        // Phase 3: Text functions
        if self.has_custom_text_function(formula) {
            result = self.replace_text_functions(&result, row_idx, table)?;
        }

        // Phase 4: Date functions
        if self.has_custom_date_function(formula) {
            result = self.replace_date_functions(&result, row_idx, table)?;
        }

        // Phase 5: Lookup functions (v1.2.0)
        if self.has_lookup_function(formula) {
            result = self.replace_lookup_functions(&result, row_idx, table)?;
        }

        // Phase 6: Financial functions (v1.6.0)
        if self.has_financial_function(formula) {
            result = self.replace_financial_functions(&result, row_idx, table)?;
        }

        // Phase 7: Array functions (v4.1.0)
        if self.has_array_function(formula) {
            result = self.replace_array_functions(&result, row_idx, table)?;
        }

        Ok(result)
    }

    /// Replace math functions with evaluated results
    /// Process from innermost to outermost for nested functions
    fn replace_math_functions(
        &self,
        formula: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        use regex::Regex;
        let mut result = formula.to_string();
        let mut prev_result = String::new();

        // Create all regex patterns once outside the loop
        let re_sqrt = Regex::new(r"SQRT\(([^)]+)\)").unwrap();
        let re_round = Regex::new(r"ROUND\(([^,]+),\s*([^)]+)\)").unwrap();
        let re_roundup = Regex::new(r"ROUNDUP\(([^,]+),\s*([^)]+)\)").unwrap();
        let re_rounddown = Regex::new(r"ROUNDDOWN\(([^,]+),\s*([^)]+)\)").unwrap();
        let re_ceiling = Regex::new(r"CEILING\(([^,]+),\s*([^)]+)\)").unwrap();
        let re_floor = Regex::new(r"FLOOR\(([^,]+),\s*([^)]+)\)").unwrap();
        let re_mod = Regex::new(r"MOD\(([^,]+),\s*([^)]+)\)").unwrap();
        let re_power = Regex::new(r"POWER\(([^,]+),\s*([^)]+)\)").unwrap();

        // Keep processing until no more changes (handles nested functions)
        // Process innermost (simpler) functions first
        while result != prev_result {
            prev_result = result.clone();

            // SQRT(number) - process single-arg functions first
            for cap in re_sqrt.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let num_expr = cap.get(1).unwrap().as_str();

                let num = self.eval_expression(num_expr, row_idx, table)?;
                let sqrt = self.eval_sqrt(num)?;

                result = result.replace(full, &sqrt.to_string());
            }

            // ROUND(number, digits)
            for cap in re_round.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let num_expr = cap.get(1).unwrap().as_str();
                let digits_expr = cap.get(2).unwrap().as_str();

                let num = self.eval_expression(num_expr, row_idx, table)?;
                let digits = self.eval_expression(digits_expr, row_idx, table)? as i32;
                let rounded = self.eval_round(num, digits);

                result = result.replace(full, &rounded.to_string());
            }

            // ROUNDUP(number, digits)
            for cap in re_roundup
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let num_expr = cap.get(1).unwrap().as_str();
                let digits_expr = cap.get(2).unwrap().as_str();

                let num = self.eval_expression(num_expr, row_idx, table)?;
                let digits = self.eval_expression(digits_expr, row_idx, table)? as i32;
                let rounded = self.eval_roundup(num, digits);

                result = result.replace(full, &rounded.to_string());
            }

            // ROUNDDOWN(number, digits)
            for cap in re_rounddown
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let num_expr = cap.get(1).unwrap().as_str();
                let digits_expr = cap.get(2).unwrap().as_str();

                let num = self.eval_expression(num_expr, row_idx, table)?;
                let digits = self.eval_expression(digits_expr, row_idx, table)? as i32;
                let rounded = self.eval_rounddown(num, digits);

                result = result.replace(full, &rounded.to_string());
            }

            // CEILING(number, significance)
            for cap in re_ceiling
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let num_expr = cap.get(1).unwrap().as_str();
                let sig_expr = cap.get(2).unwrap().as_str();

                let num = self.eval_expression(num_expr, row_idx, table)?;
                let sig = self.eval_expression(sig_expr, row_idx, table)?;
                let ceiling = self.eval_ceiling(num, sig);

                result = result.replace(full, &ceiling.to_string());
            }

            // FLOOR(number, significance)
            for cap in re_floor.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let num_expr = cap.get(1).unwrap().as_str();
                let sig_expr = cap.get(2).unwrap().as_str();

                let num = self.eval_expression(num_expr, row_idx, table)?;
                let sig = self.eval_expression(sig_expr, row_idx, table)?;
                let floor = self.eval_floor(num, sig);

                result = result.replace(full, &floor.to_string());
            }

            // MOD(number, divisor)
            for cap in re_mod.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let num_expr = cap.get(1).unwrap().as_str();
                let div_expr = cap.get(2).unwrap().as_str();

                let num = self.eval_expression(num_expr, row_idx, table)?;
                let div = self.eval_expression(div_expr, row_idx, table)?;
                let modulo = self.eval_mod(num, div)?;

                result = result.replace(full, &modulo.to_string());
            }

            // POWER(base, exponent)
            for cap in re_power.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let base_expr = cap.get(1).unwrap().as_str();
                let exp_expr = cap.get(2).unwrap().as_str();

                let base = self.eval_expression(base_expr, row_idx, table)?;
                let exp = self.eval_expression(exp_expr, row_idx, table)?;
                let power = self.eval_power(base, exp);

                result = result.replace(full, &power.to_string());
            }
        }

        Ok(result)
    }

    /// Replace text functions with evaluated results (returns quoted strings for xlformula_engine)
    /// Process from innermost to outermost for nested functions
    fn replace_text_functions(
        &self,
        formula: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        use regex::Regex;
        let mut result = formula.to_string();
        let mut prev_result = String::new();

        // Create all regex patterns once outside the loop
        let re_concat = Regex::new(r"(?:CONCAT|CONCATENATE)\(([^)]+)\)").unwrap();
        let re_trim = Regex::new(r"TRIM\(([^)]+)\)").unwrap();
        let re_upper = Regex::new(r"UPPER\(([^)]+)\)").unwrap();
        let re_lower = Regex::new(r"LOWER\(([^)]+)\)").unwrap();
        let re_len = Regex::new(r"LEN\(([^)]+)\)").unwrap();
        let re_mid = Regex::new(r"MID\(([^,]+),\s*([^,]+),\s*([^)]+)\)").unwrap();

        // Keep processing until no more changes (handles nested functions)
        while result != prev_result {
            prev_result = result.clone();

            // CONCAT/CONCATENATE - variable arguments
            for cap in re_concat.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let args_str = cap.get(1).unwrap().as_str();
                let args = self.parse_function_args(args_str)?;

                let mut texts = Vec::new();
                for arg in args {
                    let text = self.eval_text_expression(&arg, row_idx, table)?;
                    texts.push(text);
                }

                let concatenated = self.eval_concat(texts);
                result = result.replace(full, &format!("\"{}\"", concatenated));
            }

            // TRIM(text)
            for cap in re_trim.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let text_expr = cap.get(1).unwrap().as_str();

                let text = self.eval_text_expression(text_expr, row_idx, table)?;
                let trimmed = self.eval_trim(&text);

                result = result.replace(full, &format!("\"{}\"", trimmed));
            }

            // UPPER(text)
            for cap in re_upper.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let text_expr = cap.get(1).unwrap().as_str();

                let text = self.eval_text_expression(text_expr, row_idx, table)?;
                let upper = self.eval_upper(&text);

                result = result.replace(full, &format!("\"{}\"", upper));
            }

            // LOWER(text)
            for cap in re_lower.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let text_expr = cap.get(1).unwrap().as_str();

                let text = self.eval_text_expression(text_expr, row_idx, table)?;
                let lower = self.eval_lower(&text);

                result = result.replace(full, &format!("\"{}\"", lower));
            }

            // LEN(text)
            for cap in re_len.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let text_expr = cap.get(1).unwrap().as_str();

                let text = self.eval_text_expression(text_expr, row_idx, table)?;
                let len = self.eval_len(&text);

                result = result.replace(full, &len.to_string());
            }

            // MID(text, start, length)
            for cap in re_mid.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let text_expr = cap.get(1).unwrap().as_str();
                let start_expr = cap.get(2).unwrap().as_str();
                let len_expr = cap.get(3).unwrap().as_str();

                let text = self.eval_text_expression(text_expr, row_idx, table)?;
                let start = self.eval_expression(start_expr, row_idx, table)? as usize;
                let length = self.eval_expression(len_expr, row_idx, table)? as usize;
                let mid = self.eval_mid(&text, start, length);

                result = result.replace(full, &format!("\"{}\"", mid));
            }
        }

        Ok(result)
    }

    /// Replace date functions with evaluated results
    /// Process from innermost to outermost for nested functions
    fn replace_date_functions(
        &self,
        formula: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        use regex::Regex;
        let mut result = formula.to_string();
        let mut prev_result = String::new();

        // Create all regex patterns once outside the loop
        let re_today = Regex::new(r"TODAY\(\)").unwrap();
        let re_year = Regex::new(r"\bYEAR\(([^)]+)\)").unwrap();
        let re_month = Regex::new(r"\bMONTH\(([^)]+)\)").unwrap();
        let re_day = Regex::new(r"\bDAY\(([^)]+)\)").unwrap();
        let re_date = Regex::new(r"DATE\(([^,]+),\s*([^,]+),\s*([^)]+)\)").unwrap();
        let re_datedif = Regex::new(r#"DATEDIF\(([^,]+),\s*([^,]+),\s*"?([YMD])"?\)"#).unwrap();
        let re_edate = Regex::new(r"EDATE\(([^,]+),\s*([^)]+)\)").unwrap();
        let re_eomonth = Regex::new(r"EOMONTH\(([^,]+),\s*([^)]+)\)").unwrap();
        // Additional date functions (v5.0.0)
        let re_networkdays =
            Regex::new(r"NETWORKDAYS\(([^,]+),\s*([^,\)]+)(?:,\s*([^\)]+))?\)").unwrap();
        let re_workday = Regex::new(r"WORKDAY\(([^,]+),\s*([^,\)]+)(?:,\s*([^\)]+))?\)").unwrap();
        let re_yearfrac = Regex::new(r"YEARFRAC\(([^,]+),\s*([^,\)]+)(?:,\s*([^\)]+))?\)").unwrap();

        // Keep processing until no more changes (handles nested functions)
        // Process simpler (single-arg) functions first
        while result != prev_result {
            prev_result = result.clone();

            // TODAY()
            if re_today.is_match(&result) {
                let today = self.eval_today();
                result = result.replace("TODAY()", &format!("\"{}\"", today));
            }

            // YEAR(date) - process single-arg functions first
            for cap in re_year.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let date_expr = cap.get(1).unwrap().as_str();

                let date = self.eval_text_expression(date_expr, row_idx, table)?;
                let year = self.eval_year(&date)?;

                result = result.replace(full, &year.to_string());
            }

            // MONTH(date)
            for cap in re_month.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let date_expr = cap.get(1).unwrap().as_str();

                let date = self.eval_text_expression(date_expr, row_idx, table)?;
                let month = self.eval_month(&date)?;

                result = result.replace(full, &month.to_string());
            }

            // DAY(date)
            for cap in re_day.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let date_expr = cap.get(1).unwrap().as_str();

                let date = self.eval_text_expression(date_expr, row_idx, table)?;
                let day = self.eval_day(&date)?;

                result = result.replace(full, &day.to_string());
            }

            // DATE(year, month, day)
            for cap in re_date.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let year_expr = cap.get(1).unwrap().as_str();
                let month_expr = cap.get(2).unwrap().as_str();
                let day_expr = cap.get(3).unwrap().as_str();

                let year = self.eval_expression(year_expr, row_idx, table)? as i32;
                let month = self.eval_expression(month_expr, row_idx, table)? as i32;
                let day = self.eval_expression(day_expr, row_idx, table)? as i32;
                let date = self.eval_date(year, month, day)?;

                result = result.replace(full, &format!("\"{}\"", date));
            }

            // DATEDIF(start_date, end_date, unit) - Difference between dates
            for cap in re_datedif
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let start_expr = cap.get(1).unwrap().as_str();
                let end_expr = cap.get(2).unwrap().as_str();
                let unit = cap.get(3).unwrap().as_str().to_uppercase();

                let start_date = self.eval_text_expression(start_expr, row_idx, table)?;
                let end_date = self.eval_text_expression(end_expr, row_idx, table)?;
                let diff = self.eval_datedif(&start_date, &end_date, &unit)?;

                result = result.replace(full, &diff.to_string());
            }

            // EDATE(start_date, months) - Add months to date
            for cap in re_edate.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let start_expr = cap.get(1).unwrap().as_str();
                let months_expr = cap.get(2).unwrap().as_str();

                let start_date = self.eval_text_expression(start_expr, row_idx, table)?;
                let months = self.eval_expression(months_expr, row_idx, table)? as i32;
                let new_date = self.eval_edate(&start_date, months)?;

                result = result.replace(full, &format!("\"{}\"", new_date));
            }

            // EOMONTH(start_date, months) - End of month after adding months
            for cap in re_eomonth
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let start_expr = cap.get(1).unwrap().as_str();
                let months_expr = cap.get(2).unwrap().as_str();

                let start_date = self.eval_text_expression(start_expr, row_idx, table)?;
                let months = self.eval_expression(months_expr, row_idx, table)? as i32;
                let new_date = self.eval_eomonth(&start_date, months)?;

                result = result.replace(full, &format!("\"{}\"", new_date));
            }

            // NETWORKDAYS(start_date, end_date, [holidays]) - Working days between dates
            for cap in re_networkdays
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let start_expr = cap.get(1).unwrap().as_str();
                let end_expr = cap.get(2).unwrap().as_str();

                let start_date = self.eval_text_expression(start_expr, row_idx, table)?;
                let end_date = self.eval_text_expression(end_expr, row_idx, table)?;

                let days = self.eval_networkdays(&start_date, &end_date)?;
                result = result.replace(full, &days.to_string());
            }

            // WORKDAY(start_date, days, [holidays]) - Date after N working days
            for cap in re_workday
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let start_expr = cap.get(1).unwrap().as_str();
                let days_expr = cap.get(2).unwrap().as_str();

                let start_date = self.eval_text_expression(start_expr, row_idx, table)?;
                let days = self.eval_expression(days_expr, row_idx, table)? as i32;

                let new_date = self.eval_workday(&start_date, days)?;
                result = result.replace(full, &format!("\"{}\"", new_date));
            }

            // YEARFRAC(start_date, end_date, [basis]) - Fraction of year between dates
            for cap in re_yearfrac
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let start_expr = cap.get(1).unwrap().as_str();
                let end_expr = cap.get(2).unwrap().as_str();
                let basis = if let Some(basis_cap) = cap.get(3) {
                    self.eval_expression(basis_cap.as_str(), row_idx, table)? as i32
                } else {
                    0 // Default: US (NASD) 30/360
                };

                let start_date = self.eval_text_expression(start_expr, row_idx, table)?;
                let end_date = self.eval_text_expression(end_expr, row_idx, table)?;

                let frac = self.eval_yearfrac(&start_date, &end_date, basis)?;
                result = result.replace(full, &frac.to_string());
            }
        }

        Ok(result)
    }

    /// Evaluate a simple expression (column reference, literal, or simple arithmetic) to get a numeric value
    fn eval_expression(&self, expr: &str, row_idx: usize, table: &Table) -> ForgeResult<f64> {
        let expr = expr.trim();

        // Check if it's a quoted string (shouldn't be in numeric context, but handle gracefully)
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            let unquoted = &expr[1..expr.len() - 1];
            if let Ok(num) = unquoted.parse::<f64>() {
                return Ok(num);
            }
            return Err(ForgeError::Eval(format!(
                "Cannot convert '{}' to number",
                unquoted
            )));
        }

        // Try parsing as literal number
        if let Ok(num) = expr.parse::<f64>() {
            return Ok(num);
        }

        // Try as column reference
        if let Some(col) = table.columns.get(expr) {
            match &col.values {
                ColumnValue::Number(nums) => {
                    return nums.get(row_idx).copied().ok_or_else(|| {
                        ForgeError::Eval(format!(
                            "Index {} out of bounds for column '{}'",
                            row_idx, expr
                        ))
                    });
                }
                _ => {
                    return Err(ForgeError::Eval(format!(
                        "Column '{}' is not numeric",
                        expr
                    )));
                }
            }
        }

        // Try using xlformula_engine for simple expressions (like "6 + 1")
        let formula = format!("={}", expr);
        let parsed = parse_formula::parse_string_to_formula(&formula, None::<NoCustomFunction>);
        let result = calculate::calculate_formula(
            parsed,
            Some(&|_: String| types::Value::Error(types::Error::Reference)),
        );

        match result {
            types::Value::Number(n) => Ok(n as f64),
            _ => Err(ForgeError::Eval(format!(
                "Cannot evaluate expression '{}'",
                expr
            ))),
        }
    }

    /// Evaluate a simple expression to get a text value
    fn eval_text_expression(
        &self,
        expr: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        let expr = expr.trim();

        // Try parsing as literal string (quoted)
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            return Ok(expr[1..expr.len() - 1].to_string());
        }

        // Try as column reference
        if let Some(col) = table.columns.get(expr) {
            match &col.values {
                ColumnValue::Text(texts) => {
                    return texts.get(row_idx).cloned().ok_or_else(|| {
                        ForgeError::Eval(format!(
                            "Index {} out of bounds for column '{}'",
                            row_idx, expr
                        ))
                    });
                }
                ColumnValue::Date(dates) => {
                    return dates.get(row_idx).cloned().ok_or_else(|| {
                        ForgeError::Eval(format!(
                            "Index {} out of bounds for column '{}'",
                            row_idx, expr
                        ))
                    });
                }
                _ => {
                    return Err(ForgeError::Eval(format!(
                        "Column '{}' is not text or date",
                        expr
                    )));
                }
            }
        }

        // Return as-is if it looks like a plain identifier
        Ok(expr.to_string())
    }

    // ============================================================================
    // PHASE 5: Lookup Functions (v1.2.0)
    // ============================================================================

    /// Replace lookup functions with evaluated results
    /// Process MATCH first, then INDEX (since INDEX may contain MATCH), then VLOOKUP, then XLOOKUP
    fn replace_lookup_functions(
        &self,
        formula: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        use regex::Regex;
        let mut result = formula.to_string();
        let mut prev_result = String::new();

        // Create regex patterns once (outside the loop to avoid clippy warning)
        let re_match = Regex::new(r"MATCH\(([^,]+),\s*([^,]+)(?:,\s*([^)]+))?\)").unwrap();
        let re_index = Regex::new(r"INDEX\(([^,]+),\s*([^)]+)\)").unwrap();
        let re_vlookup =
            Regex::new(r"VLOOKUP\(([^,]+),\s*([^,]+),\s*([^,]+)(?:,\s*([^)]+))?\)").unwrap();
        let re_xlookup = Regex::new(r"XLOOKUP\(([^,]+),\s*([^,]+),\s*([^,]+)(?:,\s*([^,]+))?(?:,\s*([^,]+))?(?:,\s*([^)]+))?\)").unwrap();
        let re_choose = Regex::new(r"CHOOSE\(([^)]+)\)").unwrap();
        // SWITCH(expression, value1, result1, [value2, result2, ...], [default])
        let re_switch = Regex::new(r"SWITCH\(([^)]+)\)").unwrap();
        // INDIRECT(ref_text) - converts text string to reference
        let re_indirect = Regex::new(r"INDIRECT\(([^)]+)\)").unwrap();
        // OFFSET(array, rows, [height]) - returns subset of array starting from rows with optional height
        let re_offset = Regex::new(r"OFFSET\(([^,]+),\s*([^,)]+)(?:,\s*([^)]+))?\)").unwrap();

        // Keep processing until no more changes (handles nested functions)
        // Process innermost (MATCH) first, then INDEX, then convenience functions
        while result != prev_result {
            prev_result = result.clone();

            // MATCH(lookup_value, lookup_array, [match_type])
            for cap in re_match.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let lookup_value_expr = cap.get(1).unwrap().as_str();
                let lookup_array_expr = cap.get(2).unwrap().as_str();
                let match_type_expr = cap.get(3).map(|m| m.as_str()).unwrap_or("0");

                let match_result = self.eval_match(
                    lookup_value_expr,
                    lookup_array_expr,
                    match_type_expr,
                    row_idx,
                    table,
                )?;
                result = result.replace(full, &match_result.to_string());
            }

            // INDEX(array, row_num)
            for cap in re_index.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let array_expr = cap.get(1).unwrap().as_str();
                let row_num_expr = cap.get(2).unwrap().as_str();

                let index_result = self.eval_index(array_expr, row_num_expr, row_idx, table)?;
                result = result.replace(full, &index_result);
            }

            // VLOOKUP(lookup_value, table_array, col_index_num, [range_lookup])
            for cap in re_vlookup
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let lookup_value_expr = cap.get(1).unwrap().as_str();
                let table_array_expr = cap.get(2).unwrap().as_str();
                let col_index_expr = cap.get(3).unwrap().as_str();
                let range_lookup_expr = cap.get(4).map(|m| m.as_str()).unwrap_or("FALSE");

                let vlookup_result = self.eval_vlookup(
                    lookup_value_expr,
                    table_array_expr,
                    col_index_expr,
                    range_lookup_expr,
                    row_idx,
                    table,
                )?;
                result = result.replace(full, &vlookup_result);
            }

            // XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])
            for cap in re_xlookup
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let lookup_value_expr = cap.get(1).unwrap().as_str();
                let lookup_array_expr = cap.get(2).unwrap().as_str();
                let return_array_expr = cap.get(3).unwrap().as_str();
                let if_not_found_expr = cap.get(4).map(|m| m.as_str());
                let match_mode_expr = cap.get(5).map(|m| m.as_str()).unwrap_or("0");
                let search_mode_expr = cap.get(6).map(|m| m.as_str()).unwrap_or("1");

                let xlookup_result = self.eval_xlookup(
                    lookup_value_expr,
                    lookup_array_expr,
                    return_array_expr,
                    if_not_found_expr,
                    match_mode_expr,
                    search_mode_expr,
                    row_idx,
                    table,
                )?;
                result = result.replace(full, &xlookup_result);
            }

            // CHOOSE(index, value1, value2, ...) - Returns the value at position index (1-based)
            for cap in re_choose.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let args_str = cap.get(1).unwrap().as_str();
                let choose_result = self.eval_choose(args_str, row_idx, table)?;
                result = result.replace(full, &choose_result);
            }

            // SWITCH(expression, value1, result1, [value2, result2, ...], [default])
            // Returns result corresponding to first matching value, or default if no match
            for cap in re_switch.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let args_str = cap.get(1).unwrap().as_str();
                let switch_result = self.eval_switch(args_str, row_idx, table)?;
                result = result.replace(full, &switch_result);
            }

            // INDIRECT(ref_text) - Converts text string to a reference
            // Useful for dynamic column references: INDIRECT("sales.revenue") or INDIRECT(col_name)
            for cap in re_indirect
                .captures_iter(&result.clone())
                .collect::<Vec<_>>()
            {
                let full = cap.get(0).unwrap().as_str();
                let ref_expr = cap.get(1).unwrap().as_str();
                let indirect_result = self.eval_indirect(ref_expr, row_idx, table)?;
                result = result.replace(full, &indirect_result);
            }

            // LAMBDA(params...)(args...) - Anonymous functions
            // Process LAMBDA expressions (special handling for curried syntax)
            if result.to_uppercase().contains("LAMBDA(") {
                result = self.eval_lambda(&result, row_idx, table)?;
            }

            // OFFSET(array, rows, [height]) - Returns subset of array
            // Used to get a slice of an array starting from a given offset
            // Often combined with SUM, AVERAGE, etc.: =SUM(OFFSET(sales.revenue, 2, 5))
            for cap in re_offset.captures_iter(&result.clone()).collect::<Vec<_>>() {
                let full = cap.get(0).unwrap().as_str();
                let array_expr = cap.get(1).unwrap().as_str();
                let rows_expr = cap.get(2).unwrap().as_str();
                let height_expr = cap.get(3).map(|m| m.as_str());
                let offset_result =
                    self.eval_offset(array_expr, rows_expr, height_expr, row_idx, table)?;
                result = result.replace(full, &offset_result);
            }

            // LET(name1, value1, [name2, value2, ...], calculation) - Named variables in formulas
            // Process LET functions to substitute named variables
            if result.to_uppercase().contains("LET(") {
                result = self.eval_let(&result, row_idx, table)?;
            }
        }

        Ok(result)
    }

    /// Evaluate LET function: LET(name1, value1, [name2, value2, ...], calculation)
    /// Creates named variables for use in the final calculation expression
    /// Example: =LET(x, 10, y, 20, x + y) → 30
    fn eval_let(&self, formula: &str, row_idx: usize, table: &Table) -> ForgeResult<String> {
        // Find LET( and its matching closing parenthesis
        let upper = formula.to_uppercase();
        if let Some(let_start) = upper.find("LET(") {
            let start_idx = let_start + 4; // Position after "LET("

            // Find the matching closing parenthesis
            let chars: Vec<char> = formula.chars().collect();
            let mut depth = 1;
            let mut end_idx = start_idx;

            for (i, &c) in chars.iter().enumerate().skip(start_idx) {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end_idx = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if depth != 0 {
                return Err(ForgeError::Eval("LET: Unmatched parentheses".to_string()));
            }

            // Extract the full LET expression and its arguments
            let full_let = &formula[let_start..=end_idx];
            let args_str = &formula[start_idx..end_idx];

            // Parse the arguments - this is complex because args can contain nested functions
            let args = self.parse_let_args(args_str)?;

            if args.len() < 3 {
                return Err(ForgeError::Eval(
                    "LET requires at least 3 arguments: name, value, and calculation".to_string(),
                ));
            }

            // Must have odd number of args: name1, val1, name2, val2, ..., calculation
            // So: name-value pairs + 1 calculation = 2n + 1 (odd)
            if args.len() % 2 == 0 {
                return Err(ForgeError::Eval(
                    "LET requires name-value pairs followed by a calculation expression"
                        .to_string(),
                ));
            }

            // Build variable substitution map
            let mut variables: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            let num_pairs = (args.len() - 1) / 2;

            for i in 0..num_pairs {
                let name = args[i * 2].trim().to_string();
                let value_expr = args[i * 2 + 1].trim();

                // Validate variable name (must be alphanumeric starting with letter)
                if name.is_empty() || !name.chars().next().unwrap().is_alphabetic() {
                    return Err(ForgeError::Eval(format!(
                        "LET: Invalid variable name '{}'",
                        name
                    )));
                }

                // Evaluate the value expression, substituting any previously defined variables
                let mut resolved_value = value_expr.to_string();
                for (var_name, var_value) in &variables {
                    // Replace variable references (whole word only)
                    resolved_value = self.replace_variable(&resolved_value, var_name, var_value);
                }

                // Try to evaluate as numeric expression
                // First try aggregation functions (SUM, AVERAGE, etc.)
                let evaluated = if self.is_aggregation_formula(&format!("={}", resolved_value)) {
                    if let Ok(num) = self.evaluate_aggregation(&format!("={}", resolved_value)) {
                        format!("{}", num)
                    } else {
                        resolved_value
                    }
                } else if let Ok(num) = self.eval_expression(&resolved_value, row_idx, table) {
                    format!("{}", num)
                } else {
                    // Keep as-is for text or complex expressions
                    resolved_value
                };

                variables.insert(name, evaluated);
            }

            // Get the final calculation expression (last argument)
            let mut calculation = args.last().unwrap().trim().to_string();

            // Substitute all variables into the calculation
            for (name, value) in &variables {
                calculation = self.replace_variable(&calculation, name, value);
            }

            // Replace the full LET(...) with the evaluated calculation
            let result = formula.replace(full_let, &calculation);
            Ok(result)
        } else {
            Ok(formula.to_string())
        }
    }

    /// Parse LET function arguments, handling nested parentheses
    fn parse_let_args(&self, args_str: &str) -> ForgeResult<Vec<String>> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut depth = 0;
        let mut in_string = false;
        let mut string_char = '"';

        for c in args_str.chars() {
            match c {
                '"' | '\'' if !in_string => {
                    in_string = true;
                    string_char = c;
                    current.push(c);
                }
                c if in_string && c == string_char => {
                    in_string = false;
                    current.push(c);
                }
                '(' if !in_string => {
                    depth += 1;
                    current.push(c);
                }
                ')' if !in_string => {
                    depth -= 1;
                    current.push(c);
                }
                ',' if depth == 0 && !in_string => {
                    args.push(current.trim().to_string());
                    current = String::new();
                }
                _ => {
                    current.push(c);
                }
            }
        }

        // Don't forget the last argument
        if !current.trim().is_empty() {
            args.push(current.trim().to_string());
        }

        Ok(args)
    }

    /// Replace a variable name with its value in an expression (whole word only)
    fn replace_variable(&self, expr: &str, name: &str, value: &str) -> String {
        // Use word boundary matching to avoid partial replacements
        // e.g., replacing "x" shouldn't affect "max" or "x1"
        let mut result = String::new();
        let chars: Vec<char> = expr.chars().collect();
        let name_chars: Vec<char> = name.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Check if this position starts the variable name
            if i + name_chars.len() <= chars.len() {
                let slice: String = chars[i..i + name_chars.len()].iter().collect();
                if slice == name {
                    // Check word boundaries
                    let before_ok =
                        i == 0 || !chars[i - 1].is_alphanumeric() && chars[i - 1] != '_';
                    let after_ok = i + name_chars.len() >= chars.len()
                        || !chars[i + name_chars.len()].is_alphanumeric()
                            && chars[i + name_chars.len()] != '_';

                    if before_ok && after_ok {
                        result.push_str(value);
                        i += name_chars.len();
                        continue;
                    }
                }
            }
            result.push(chars[i]);
            i += 1;
        }

        result
    }

    /// Evaluate OFFSET function: OFFSET(array, rows, [height])
    /// Returns a subset of the array as a comma-separated list of values
    /// - array: column reference like sales.revenue
    /// - rows: number of rows to skip from the start (0-indexed)
    /// - height: optional number of rows to include (default: all remaining)
    fn eval_offset(
        &self,
        array_expr: &str,
        rows_expr: &str,
        height_expr: Option<&str>,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        // Evaluate the rows offset
        let rows_offset = self.eval_expression(rows_expr, row_idx, table)? as usize;

        // Get the array values
        let values = self.get_values_from_arg(array_expr.trim(), row_idx, table)?;

        if values.is_empty() {
            return Err(ForgeError::Eval(format!(
                "OFFSET: array '{}' is empty or not found",
                array_expr
            )));
        }

        if rows_offset >= values.len() {
            return Err(ForgeError::Eval(format!(
                "OFFSET: rows offset {} exceeds array length {}",
                rows_offset,
                values.len()
            )));
        }

        // Determine the height (number of values to return)
        let height = if let Some(h_expr) = height_expr {
            let h = self.eval_expression(h_expr, row_idx, table)? as usize;
            if h == 0 {
                return Err(ForgeError::Eval("OFFSET: height cannot be 0".to_string()));
            }
            // Cap at remaining values
            std::cmp::min(h, values.len() - rows_offset)
        } else {
            // Default: all remaining values
            values.len() - rows_offset
        };

        // Extract the subset
        let subset: Vec<f64> = values
            .iter()
            .skip(rows_offset)
            .take(height)
            .copied()
            .collect();

        // Return as comma-separated values (for use with SUM, AVERAGE, etc.)
        let result = subset
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(", ");

        Ok(result)
    }

    /// Evaluate CHOOSE function: CHOOSE(index, value1, value2, ...)
    /// Returns the value at position index (1-based)
    fn eval_choose(&self, args_str: &str, row_idx: usize, table: &Table) -> ForgeResult<String> {
        let args = self.parse_function_args(args_str)?;

        if args.len() < 2 {
            return Err(ForgeError::Eval(
                "CHOOSE requires at least 2 arguments: index and at least one value".to_string(),
            ));
        }

        // Evaluate the index (first argument)
        let index = self.eval_expression(&args[0], row_idx, table)? as usize;

        if index < 1 {
            return Err(ForgeError::Eval(
                "CHOOSE index must be at least 1".to_string(),
            ));
        }

        if index > args.len() - 1 {
            return Err(ForgeError::Eval(format!(
                "CHOOSE index {} is out of range (max: {})",
                index,
                args.len() - 1
            )));
        }

        // Get the value at the index position (1-based, so args[index] is correct)
        let value_expr = &args[index];

        // Try to evaluate as a number first
        if let Ok(num_value) = self.eval_expression(value_expr, row_idx, table) {
            return Ok(format!("{}", num_value));
        }

        // Try to evaluate as text
        if let Ok(text_value) = self.eval_text_expression(value_expr, row_idx, table) {
            // Return quoted string for text values
            return Ok(format!("\"{}\"", text_value));
        }

        // Return as-is if we can't evaluate it
        Ok(value_expr.trim().to_string())
    }

    /// Evaluate SWITCH function: SWITCH(expression, value1, result1, [value2, result2, ...], [default])
    /// Returns the result corresponding to the first value that matches the expression
    /// If no match and odd number of args after expression, last arg is default
    fn eval_switch(&self, args_str: &str, row_idx: usize, table: &Table) -> ForgeResult<String> {
        let args = self.parse_function_args(args_str)?;

        if args.len() < 3 {
            return Err(ForgeError::Eval(
                "SWITCH requires at least 3 arguments: expression, value, and result".to_string(),
            ));
        }

        // First argument is the expression to match against
        let expr_to_match = args[0].trim();

        // Try to evaluate expression as number first, then as text
        let match_value = if let Ok(num) = self.eval_expression(expr_to_match, row_idx, table) {
            SwitchValue::Number(num)
        } else if let Ok(text) = self.eval_text_expression(expr_to_match, row_idx, table) {
            SwitchValue::Text(text)
        } else {
            SwitchValue::Text(expr_to_match.to_string())
        };

        // Remaining args are value-result pairs, possibly with a default at the end
        let remaining = &args[1..];

        // If odd number of remaining args, last one is default
        let (pairs, default) = if remaining.len() % 2 == 1 {
            (
                &remaining[..remaining.len() - 1],
                Some(&remaining[remaining.len() - 1]),
            )
        } else {
            (remaining, None)
        };

        // Check each value-result pair
        for chunk in pairs.chunks(2) {
            let value_expr = chunk[0].trim();
            let result_expr = chunk[1].trim();

            // Evaluate the value to compare
            let compare_value = if let Ok(num) = self.eval_expression(value_expr, row_idx, table) {
                SwitchValue::Number(num)
            } else if let Ok(text) = self.eval_text_expression(value_expr, row_idx, table) {
                SwitchValue::Text(text)
            } else {
                SwitchValue::Text(value_expr.to_string())
            };

            // Check if values match
            let matches = match (&match_value, &compare_value) {
                (SwitchValue::Number(a), SwitchValue::Number(b)) => (a - b).abs() < 1e-10,
                (SwitchValue::Text(a), SwitchValue::Text(b)) => a == b,
                _ => false,
            };

            if matches {
                // Return the result for this match
                if let Ok(num) = self.eval_expression(result_expr, row_idx, table) {
                    return Ok(format!("{}", num));
                }
                if let Ok(text) = self.eval_text_expression(result_expr, row_idx, table) {
                    return Ok(format!("\"{}\"", text));
                }
                return Ok(result_expr.to_string());
            }
        }

        // No match found - return default if provided
        if let Some(default_expr) = default {
            let default_expr = default_expr.trim();
            if let Ok(num) = self.eval_expression(default_expr, row_idx, table) {
                return Ok(format!("{}", num));
            }
            if let Ok(text) = self.eval_text_expression(default_expr, row_idx, table) {
                return Ok(format!("\"{}\"", text));
            }
            return Ok(default_expr.to_string());
        }

        Err(ForgeError::Eval(
            "SWITCH: No matching value found and no default provided".to_string(),
        ))
    }

    /// Evaluate INDIRECT function: INDIRECT(ref_text)
    /// Converts a text string to a reference and returns the value
    /// - For scalars: INDIRECT("inputs.rate") → value of inputs.rate
    /// - For columns: INDIRECT("sales.revenue") → column values (for use with SUM, etc.)
    fn eval_indirect(&self, ref_expr: &str, row_idx: usize, table: &Table) -> ForgeResult<String> {
        let ref_expr = ref_expr.trim();

        // First, evaluate the expression to get the reference string
        let ref_string = if ref_expr.starts_with('"') && ref_expr.ends_with('"') {
            // Literal string: INDIRECT("sales.revenue")
            ref_expr[1..ref_expr.len() - 1].to_string()
        } else if ref_expr.starts_with('\'') && ref_expr.ends_with('\'') {
            // Single-quoted string
            ref_expr[1..ref_expr.len() - 1].to_string()
        } else if let Ok(text) = self.eval_text_expression(ref_expr, row_idx, table) {
            // Expression that evaluates to text: INDIRECT(CONCAT("sales.", col_name))
            text
        } else {
            // Try as-is
            ref_expr.to_string()
        };

        // Now resolve the reference string
        let ref_string = ref_string.trim();

        // Try to resolve as a scalar reference first
        if let Some(scalar) = self.model.scalars.get(ref_string) {
            if let Some(value) = scalar.value {
                return Ok(format!("{}", value));
            }
        }

        // Try as table.column reference for aggregation context
        if ref_string.contains('.') {
            let parts: Vec<&str> = ref_string.splitn(2, '.').collect();
            if parts.len() == 2 {
                let table_name = parts[0];
                let col_name = parts[1];

                if let Some(table) = self.model.tables.get(table_name) {
                    if let Some(col) = table.columns.get(col_name) {
                        match &col.values {
                            crate::types::ColumnValue::Number(nums) => {
                                // Return as comma-separated for use with aggregation functions
                                let result = nums
                                    .iter()
                                    .map(|v| format!("{}", v))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                return Ok(result);
                            }
                            crate::types::ColumnValue::Text(texts) => {
                                let result = texts
                                    .iter()
                                    .map(|s| format!("\"{}\"", s))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                return Ok(result);
                            }
                            crate::types::ColumnValue::Date(dates) => {
                                let result = dates
                                    .iter()
                                    .map(|s| format!("\"{}\"", s))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                return Ok(result);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Err(ForgeError::Eval(format!(
            "INDIRECT: Cannot resolve reference '{}'",
            ref_string
        )))
    }

    /// Evaluate LAMBDA function: LAMBDA(param1, param2, ..., calculation)(arg1, arg2, ...)
    /// Creates an anonymous function and immediately invokes it with arguments
    /// Example: =LAMBDA(x, x * 2)(5) → 10
    /// Example: =LAMBDA(x, y, x + y)(3, 4) → 7
    fn eval_lambda(&self, formula: &str, row_idx: usize, table: &Table) -> ForgeResult<String> {
        let upper = formula.to_uppercase();
        if let Some(lambda_start) = upper.find("LAMBDA(") {
            // Find the closing parenthesis of LAMBDA definition
            let chars: Vec<char> = formula.chars().collect();
            let def_start = lambda_start + 7; // Position after "LAMBDA("

            // Find the matching closing parenthesis for the definition
            let mut depth = 1;
            let mut def_end = def_start;

            for (i, &c) in chars.iter().enumerate().skip(def_start) {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            def_end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if depth != 0 {
                return Err(ForgeError::Eval(
                    "LAMBDA: Unmatched parentheses in definition".to_string(),
                ));
            }

            // Extract the LAMBDA definition (params and calculation)
            let def_content = &formula[def_start..def_end];

            // Check if there's an invocation after the definition: )(args)
            let invocation_start = def_end + 1;
            if invocation_start >= chars.len() || chars[invocation_start] != '(' {
                return Err(ForgeError::Eval(
                    "LAMBDA: Missing invocation arguments. Use LAMBDA(params, calc)(args)"
                        .to_string(),
                ));
            }

            // Find the closing parenthesis of invocation
            let args_start = invocation_start + 1;
            let mut depth = 1;
            let mut args_end = args_start;

            for (i, &c) in chars.iter().enumerate().skip(args_start) {
                match c {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            args_end = i;
                            break;
                        }
                    }
                    _ => {}
                }
            }

            if depth != 0 {
                return Err(ForgeError::Eval(
                    "LAMBDA: Unmatched parentheses in invocation".to_string(),
                ));
            }

            // Extract the invocation arguments
            let args_content = &formula[args_start..args_end];

            // Parse definition: everything before the last comma is params, last item is calculation
            let def_parts = self.parse_function_args(def_content)?;
            if def_parts.len() < 2 {
                return Err(ForgeError::Eval(
                    "LAMBDA requires at least one parameter and a calculation".to_string(),
                ));
            }

            // Last part is the calculation, everything else is parameters
            let params: Vec<String> = def_parts[..def_parts.len() - 1]
                .iter()
                .map(|s| s.trim().to_string())
                .collect();
            let calculation = def_parts.last().unwrap().trim();

            // Parse invocation arguments
            let args = self.parse_function_args(args_content)?;
            if args.len() != params.len() {
                return Err(ForgeError::Eval(format!(
                    "LAMBDA: Expected {} arguments, got {}",
                    params.len(),
                    args.len()
                )));
            }

            // Evaluate each argument and build substitution map
            let mut substitutions: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for (param, arg) in params.iter().zip(args.iter()) {
                let evaluated = if let Ok(num) = self.eval_expression(arg.trim(), row_idx, table) {
                    format!("{}", num)
                } else {
                    arg.trim().to_string()
                };
                substitutions.insert(param.clone(), evaluated);
            }

            // Substitute parameters in calculation
            let mut result_calc = calculation.to_string();
            for (param, value) in &substitutions {
                result_calc = self.replace_variable(&result_calc, param, value);
            }

            // Replace the entire LAMBDA(...)(...) with the substituted calculation
            let full_lambda = &formula[lambda_start..=args_end];
            let result = formula.replace(full_lambda, &result_calc);
            Ok(result)
        } else {
            Ok(formula.to_string())
        }
    }

    /// Evaluate MATCH function: MATCH(lookup_value, lookup_array, [match_type])
    /// Returns 1-based index of matched value
    /// match_type: 0 = exact match (default), 1 = less than or equal, -1 = greater than or equal
    fn eval_match(
        &self,
        lookup_value_expr: &str,
        lookup_array_expr: &str,
        match_type_expr: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<f64> {
        // Parse match_type (default to 0 for exact match - safest option)
        let match_type = self.eval_expression(match_type_expr, row_idx, table)? as i32;

        // Get the lookup value
        let lookup_value = self.get_lookup_value(lookup_value_expr, row_idx, table)?;

        // Get the lookup array (table.column reference)
        let lookup_array = self.get_column_array(lookup_array_expr)?;

        // Perform the match based on match_type
        match match_type {
            0 => {
                // Exact match
                for (i, val) in lookup_array.iter().enumerate() {
                    if self.values_match(&lookup_value, val) {
                        return Ok((i + 1) as f64); // 1-based index
                    }
                }
                Err(ForgeError::Eval(format!(
                    "MATCH: Value '{}' not found in array",
                    self.format_lookup_value(&lookup_value)
                )))
            }
            1 => {
                // Find largest value less than or equal to lookup_value
                // Array should be sorted in ascending order (Excel behavior)
                let mut best_match: Option<usize> = None;

                for (i, val) in lookup_array.iter().enumerate() {
                    if let (LookupValue::Number(lookup_num), LookupValue::Number(val_num)) =
                        (&lookup_value, val)
                    {
                        if val_num <= lookup_num {
                            best_match = Some(i);
                        } else {
                            break; // Array is sorted, no need to continue
                        }
                    }
                }

                best_match.map(|i| (i + 1) as f64).ok_or_else(|| {
                    ForgeError::Eval(format!(
                        "MATCH: No value less than or equal to '{}' found",
                        self.format_lookup_value(&lookup_value)
                    ))
                })
            }
            -1 => {
                // Find smallest value greater than or equal to lookup_value
                // Array should be sorted in descending order (Excel behavior)
                let mut best_match: Option<usize> = None;

                for (i, val) in lookup_array.iter().enumerate() {
                    if let (LookupValue::Number(lookup_num), LookupValue::Number(val_num)) =
                        (&lookup_value, val)
                    {
                        if val_num >= lookup_num {
                            best_match = Some(i);
                        } else {
                            break; // Array is sorted descending, no need to continue
                        }
                    }
                }

                best_match.map(|i| (i + 1) as f64).ok_or_else(|| {
                    ForgeError::Eval(format!(
                        "MATCH: No value greater than or equal to '{}' found",
                        self.format_lookup_value(&lookup_value)
                    ))
                })
            }
            _ => Err(ForgeError::Eval(format!(
                "MATCH: Invalid match_type '{}' (must be -1, 0, or 1)",
                match_type
            ))),
        }
    }

    /// Evaluate INDEX function: INDEX(array, row_num)
    /// Returns value at position row_num (1-based)
    fn eval_index(
        &self,
        array_expr: &str,
        row_num_expr: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        // Get the row number (1-based)
        let row_num = self.eval_expression(row_num_expr, row_idx, table)? as usize;

        if row_num == 0 {
            return Err(ForgeError::Eval(
                "INDEX: row_num must be >= 1 (1-based indexing)".to_string(),
            ));
        }

        // Get the array (table.column reference)
        let array = self.get_column_array(array_expr)?;

        // Get value at position (convert from 1-based to 0-based)
        let zero_based_index = row_num - 1;

        if zero_based_index >= array.len() {
            return Err(ForgeError::Eval(format!(
                "INDEX: row_num {} out of bounds (array has {} elements)",
                row_num,
                array.len()
            )));
        }

        // Return the value as a string that can be embedded in the formula
        Ok(self.format_lookup_value(&array[zero_based_index]))
    }

    /// Evaluate VLOOKUP function: VLOOKUP(lookup_value, table_array, col_index_num, [range_lookup])
    /// Convenience wrapper around INDEX/MATCH
    fn eval_vlookup(
        &self,
        lookup_value_expr: &str,
        table_array_expr: &str,
        col_index_expr: &str,
        range_lookup_expr: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        // Parse col_index_num
        let col_index = self.eval_expression(col_index_expr, row_idx, table)? as usize;

        if col_index == 0 {
            return Err(ForgeError::Eval(
                "VLOOKUP: col_index_num must be >= 1".to_string(),
            ));
        }

        // Parse range_lookup (FALSE/0 = exact, TRUE/1 = approximate)
        let range_lookup = self.parse_boolean(range_lookup_expr, row_idx, table)?;
        let match_type = if range_lookup { 1 } else { 0 }; // 1 for approximate (<=), 0 for exact

        // Parse table_array_expr to get table name and columns
        // Expected format: "table_name" or "table_name.first_col:table_name.last_col"
        let (table_name, first_col_name, num_columns) = self.parse_table_array(table_array_expr)?;

        if col_index > num_columns {
            return Err(ForgeError::Eval(format!(
                "VLOOKUP: col_index_num {} exceeds number of columns {}",
                col_index, num_columns
            )));
        }

        // Get the lookup column (first column of table array)
        let lookup_col_ref = format!("{}.{}", table_name, first_col_name);
        let lookup_array = self.get_column_array(&lookup_col_ref)?;

        // Get the return column (col_index position in table array)
        let return_col_name =
            self.get_column_name_at_offset(&table_name, &first_col_name, col_index - 1)?;
        let return_col_ref = format!("{}.{}", table_name, return_col_name);
        let return_array = self.get_column_array(&return_col_ref)?;

        // Get lookup value
        let lookup_value = self.get_lookup_value(lookup_value_expr, row_idx, table)?;

        // Perform the lookup
        let match_index = if match_type == 0 {
            // Exact match
            lookup_array
                .iter()
                .position(|val| self.values_match(&lookup_value, val))
                .ok_or_else(|| {
                    ForgeError::Eval(format!(
                        "VLOOKUP: Value '{}' not found",
                        self.format_lookup_value(&lookup_value)
                    ))
                })?
        } else {
            // Approximate match - find largest value <= lookup_value
            let mut best_match: Option<usize> = None;

            for (i, val) in lookup_array.iter().enumerate() {
                if let (LookupValue::Number(lookup_num), LookupValue::Number(val_num)) =
                    (&lookup_value, val)
                {
                    if val_num <= lookup_num {
                        best_match = Some(i);
                    } else {
                        break;
                    }
                }
            }

            best_match.ok_or_else(|| {
                ForgeError::Eval(format!(
                    "VLOOKUP: No value <= '{}' found",
                    self.format_lookup_value(&lookup_value)
                ))
            })?
        };

        // Return value from return array
        Ok(self.format_lookup_value(&return_array[match_index]))
    }

    /// Evaluate XLOOKUP function: XLOOKUP(lookup_value, lookup_array, return_array, [if_not_found], [match_mode], [search_mode])
    /// Enhanced lookup with more options than VLOOKUP
    #[allow(clippy::too_many_arguments)]
    fn eval_xlookup(
        &self,
        lookup_value_expr: &str,
        lookup_array_expr: &str,
        return_array_expr: &str,
        if_not_found_expr: Option<&str>,
        match_mode_expr: &str,
        _search_mode_expr: &str, // TODO: Implement search_mode (1=first-to-last, -1=last-to-first, 2=binary asc, -2=binary desc)
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        // Parse match_mode (0=exact, -1=exact or next smallest, 1=exact or next largest, 2=wildcard - not implemented yet)
        let match_mode = self.eval_expression(match_mode_expr, row_idx, table)? as i32;

        // Get lookup value
        let lookup_value = self.get_lookup_value(lookup_value_expr, row_idx, table)?;

        // Get lookup and return arrays
        let lookup_array = self.get_column_array(lookup_array_expr)?;
        let return_array = self.get_column_array(return_array_expr)?;

        if lookup_array.len() != return_array.len() {
            return Err(ForgeError::Eval(format!(
                "XLOOKUP: lookup_array ({} elements) and return_array ({} elements) must have same length",
                lookup_array.len(),
                return_array.len()
            )));
        }

        // Perform the lookup based on match_mode
        let match_index = match match_mode {
            0 => {
                // Exact match
                lookup_array
                    .iter()
                    .position(|val| self.values_match(&lookup_value, val))
            }
            1 => {
                // Exact or next largest
                let mut best_match: Option<usize> = None;

                for (i, val) in lookup_array.iter().enumerate() {
                    if self.values_match(&lookup_value, val) {
                        return Ok(self.format_lookup_value(&return_array[i]));
                    }

                    if let (LookupValue::Number(lookup_num), LookupValue::Number(val_num)) =
                        (&lookup_value, val)
                    {
                        if *val_num >= *lookup_num
                            && (best_match.is_none()
                                || *val_num
                                    < self.get_number_value(&lookup_array[best_match.unwrap()]))
                        {
                            best_match = Some(i);
                        }
                    }
                }

                best_match
            }
            -1 => {
                // Exact or next smallest
                let mut best_match: Option<usize> = None;

                for (i, val) in lookup_array.iter().enumerate() {
                    if self.values_match(&lookup_value, val) {
                        return Ok(self.format_lookup_value(&return_array[i]));
                    }

                    if let (LookupValue::Number(lookup_num), LookupValue::Number(val_num)) =
                        (&lookup_value, val)
                    {
                        if *val_num <= *lookup_num
                            && (best_match.is_none()
                                || *val_num
                                    > self.get_number_value(&lookup_array[best_match.unwrap()]))
                        {
                            best_match = Some(i);
                        }
                    }
                }

                best_match
            }
            _ => {
                return Err(ForgeError::Eval(format!(
                    "XLOOKUP: match_mode {} not supported (use 0, 1, or -1)",
                    match_mode
                )))
            }
        };

        // Return result or if_not_found value
        match match_index {
            Some(i) => Ok(self.format_lookup_value(&return_array[i])),
            None => {
                if let Some(not_found_expr) = if_not_found_expr {
                    // Return the if_not_found value as-is (it's already properly formatted)
                    Ok(not_found_expr.to_string())
                } else {
                    Err(ForgeError::Eval(format!(
                        "XLOOKUP: Value '{}' not found",
                        self.format_lookup_value(&lookup_value)
                    )))
                }
            }
        }
    }

    // ============================================================================
    // Helper Methods for Lookup Functions
    // ============================================================================

    /// Get lookup value from expression (can be column reference or literal)
    fn get_lookup_value(
        &self,
        expr: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<LookupValue> {
        let expr = expr.trim();

        // Try as literal number
        if let Ok(num) = expr.parse::<f64>() {
            return Ok(LookupValue::Number(num));
        }

        // Try as literal string (quoted)
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            return Ok(LookupValue::Text(expr[1..expr.len() - 1].to_string()));
        }

        // Try as column reference (get value at current row)
        if let Some(col) = table.columns.get(expr) {
            return self.column_value_to_lookup_value(&col.values, row_idx);
        }

        // Try as table.column reference
        if expr.contains('.') {
            if let Ok((table_name, col_name)) = self.parse_table_column_ref(expr) {
                if let Some(ref_table) = self.model.tables.get(&table_name) {
                    if let Some(ref_col) = ref_table.columns.get(&col_name) {
                        return self.column_value_to_lookup_value(&ref_col.values, row_idx);
                    }
                }
            }
        }

        // Default to text value
        Ok(LookupValue::Text(expr.to_string()))
    }

    /// Get column array as LookupValue vector
    fn get_column_array(&self, col_ref: &str) -> ForgeResult<Vec<LookupValue>> {
        let col_ref = col_ref.trim();

        // Parse table.column reference
        let (table_name, col_name) = self.parse_table_column_ref(col_ref)?;

        let table = self
            .model
            .tables
            .get(&table_name)
            .ok_or_else(|| ForgeError::Eval(format!("Table '{}' not found", table_name)))?;

        let column = table.columns.get(&col_name).ok_or_else(|| {
            ForgeError::Eval(format!(
                "Column '{}' not found in table '{}'",
                col_name, table_name
            ))
        })?;

        // Convert ColumnValue to Vec<LookupValue>
        match &column.values {
            ColumnValue::Number(nums) => Ok(nums.iter().map(|&n| LookupValue::Number(n)).collect()),
            ColumnValue::Text(texts) => {
                Ok(texts.iter().map(|s| LookupValue::Text(s.clone())).collect())
            }
            ColumnValue::Date(dates) => {
                Ok(dates.iter().map(|s| LookupValue::Text(s.clone())).collect())
            }
            ColumnValue::Boolean(bools) => {
                Ok(bools.iter().map(|&b| LookupValue::Boolean(b)).collect())
            }
        }
    }

    /// Convert ColumnValue at specific index to LookupValue
    fn column_value_to_lookup_value(
        &self,
        col_val: &ColumnValue,
        index: usize,
    ) -> ForgeResult<LookupValue> {
        match col_val {
            ColumnValue::Number(nums) => nums
                .get(index)
                .copied()
                .map(LookupValue::Number)
                .ok_or_else(|| ForgeError::Eval(format!("Index {} out of bounds", index))),
            ColumnValue::Text(texts) => texts
                .get(index)
                .cloned()
                .map(LookupValue::Text)
                .ok_or_else(|| ForgeError::Eval(format!("Index {} out of bounds", index))),
            ColumnValue::Date(dates) => dates
                .get(index)
                .cloned()
                .map(LookupValue::Text)
                .ok_or_else(|| ForgeError::Eval(format!("Index {} out of bounds", index))),
            ColumnValue::Boolean(bools) => bools
                .get(index)
                .copied()
                .map(LookupValue::Boolean)
                .ok_or_else(|| ForgeError::Eval(format!("Index {} out of bounds", index))),
        }
    }

    /// Check if two LookupValues match
    fn values_match(&self, a: &LookupValue, b: &LookupValue) -> bool {
        match (a, b) {
            (LookupValue::Number(n1), LookupValue::Number(n2)) => (n1 - n2).abs() < 1e-10,
            (LookupValue::Text(s1), LookupValue::Text(s2)) => s1 == s2,
            (LookupValue::Boolean(b1), LookupValue::Boolean(b2)) => b1 == b2,
            _ => false,
        }
    }

    /// Format LookupValue for insertion into formula
    fn format_lookup_value(&self, val: &LookupValue) -> String {
        match val {
            LookupValue::Number(n) => n.to_string(),
            LookupValue::Text(s) => format!("\"{}\"", s),
            LookupValue::Boolean(b) => {
                if *b {
                    "TRUE".to_string()
                } else {
                    "FALSE".to_string()
                }
            }
        }
    }

    /// Get number value from LookupValue
    fn get_number_value(&self, val: &LookupValue) -> f64 {
        match val {
            LookupValue::Number(n) => *n,
            _ => 0.0,
        }
    }

    /// Parse table_array expression for VLOOKUP
    /// Returns (table_name, first_column_name, number_of_columns)
    fn parse_table_array(&self, expr: &str) -> ForgeResult<(String, String, usize)> {
        let expr = expr.trim();

        // Check if it's a simple table name (use all columns)
        if !expr.contains('.') && !expr.contains(':') {
            // Just a table name - use all columns
            if let Some(table) = self.model.tables.get(expr) {
                if table.columns.is_empty() {
                    return Err(ForgeError::Eval(format!("Table '{}' has no columns", expr)));
                }

                // Get first column name (tables maintain insertion order via LinkedHashMap-like behavior)
                let first_col_name = table.columns.keys().next().unwrap().clone();

                return Ok((expr.to_string(), first_col_name, table.columns.len()));
            }
        }

        // Check for range notation: table.col1:table.col2
        if expr.contains(':') {
            let parts: Vec<&str> = expr.split(':').collect();
            if parts.len() == 2 {
                let (table1, col1) = self.parse_table_column_ref(parts[0])?;
                let (table2, col2) = self.parse_table_column_ref(parts[1])?;

                if table1 != table2 {
                    return Err(ForgeError::Eval(format!(
                        "Table array range must be within same table: {} vs {}",
                        table1, table2
                    )));
                }

                // Count columns from col1 to col2
                if let Some(table) = self.model.tables.get(&table1) {
                    let col_names: Vec<String> = table.columns.keys().cloned().collect();
                    let start_idx = col_names.iter().position(|c| c == &col1).ok_or_else(|| {
                        ForgeError::Eval(format!(
                            "Column '{}' not found in table '{}'",
                            col1, table1
                        ))
                    })?;
                    let end_idx = col_names.iter().position(|c| c == &col2).ok_or_else(|| {
                        ForgeError::Eval(format!(
                            "Column '{}' not found in table '{}'",
                            col2, table1
                        ))
                    })?;

                    if start_idx > end_idx {
                        return Err(ForgeError::Eval(format!(
                            "Invalid column range: '{}' comes after '{}'",
                            col1, col2
                        )));
                    }

                    let num_cols = end_idx - start_idx + 1;
                    return Ok((table1, col1, num_cols));
                }
            }
        }

        Err(ForgeError::Eval(format!(
            "Invalid table_array expression: '{}'. Expected 'table_name' or 'table.col1:table.col2'",
            expr
        )))
    }

    /// Get column name at offset from starting column
    fn get_column_name_at_offset(
        &self,
        table_name: &str,
        start_col: &str,
        offset: usize,
    ) -> ForgeResult<String> {
        let table = self
            .model
            .tables
            .get(table_name)
            .ok_or_else(|| ForgeError::Eval(format!("Table '{}' not found", table_name)))?;

        let col_names: Vec<String> = table.columns.keys().cloned().collect();
        let start_idx = col_names
            .iter()
            .position(|c| c == start_col)
            .ok_or_else(|| {
                ForgeError::Eval(format!(
                    "Column '{}' not found in table '{}'",
                    start_col, table_name
                ))
            })?;

        let target_idx = start_idx + offset;

        col_names.get(target_idx).cloned().ok_or_else(|| {
            ForgeError::Eval(format!(
                "Column offset {} from '{}' exceeds table bounds",
                offset, start_col
            ))
        })
    }

    /// Parse boolean expression (TRUE/FALSE/0/1)
    fn parse_boolean(&self, expr: &str, row_idx: usize, table: &Table) -> ForgeResult<bool> {
        let upper = expr.trim().to_uppercase();

        match upper.as_str() {
            "TRUE" | "1" => Ok(true),
            "FALSE" | "0" => Ok(false),
            _ => {
                // Try evaluating as expression
                let num = self.eval_expression(expr, row_idx, table)?;
                Ok(num != 0.0)
            }
        }
    }

    // ============================================================================
    // Financial Functions (v1.6.0)
    // NPV, IRR, PMT, FV, PV - Essential for DCF and financial modeling
    // ============================================================================

    /// Replace financial functions with evaluated results
    fn replace_financial_functions(
        &self,
        formula: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        use regex::Regex;
        let mut result = formula.to_string();

        // NPV(rate, cash_flow1, cash_flow2, ...) - Net Present Value
        // Use \b word boundary to avoid matching XNPV
        let re_npv = Regex::new(r"\bNPV\(([^)]+)\)").unwrap();
        for caps in re_npv.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 2 {
                return Err(ForgeError::Eval(
                    "NPV requires at least 2 arguments: rate and at least one cash flow"
                        .to_string(),
                ));
            }

            let rate = self.eval_expression(&args[0], row_idx, table)?;
            let mut npv = 0.0;

            for (i, arg) in args.iter().skip(1).enumerate() {
                // Check if it's a column reference
                let values = self.get_values_from_arg(arg, row_idx, table)?;
                for (j, cf) in values.iter().enumerate() {
                    let period = (i * values.len() + j + 1) as f64;
                    npv += cf / (1.0 + rate).powf(period);
                }
            }

            result = result.replace(full, &format!("{}", npv));
        }

        // PMT(rate, nper, pv, [fv], [type]) - Payment for a loan
        let re_pmt = Regex::new(r"PMT\(([^)]+)\)").unwrap();
        for caps in re_pmt.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 3 {
                return Err(ForgeError::Eval(
                    "PMT requires at least 3 arguments: rate, nper, pv".to_string(),
                ));
            }

            let rate = self.eval_expression(&args[0], row_idx, table)?;
            let nper = self.eval_expression(&args[1], row_idx, table)?;
            let pv = self.eval_expression(&args[2], row_idx, table)?;
            let fv = if args.len() > 3 {
                self.eval_expression(&args[3], row_idx, table)?
            } else {
                0.0
            };
            let pmt_type = if args.len() > 4 {
                self.eval_expression(&args[4], row_idx, table)? as i32
            } else {
                0
            };

            let pmt = if rate == 0.0 {
                -(pv + fv) / nper
            } else {
                let pvif = (1.0 + rate).powf(nper);
                let pmt = rate * (pv * pvif + fv) / (pvif - 1.0);
                if pmt_type == 1 {
                    -pmt / (1.0 + rate)
                } else {
                    -pmt
                }
            };

            result = result.replace(full, &format!("{}", pmt));
        }

        // FV(rate, nper, pmt, [pv], [type]) - Future Value
        // Use \b word boundary to avoid matching inside XFVP etc.
        let re_fv = Regex::new(r"\bFV\(([^)]+)\)").unwrap();
        for caps in re_fv.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 3 {
                return Err(ForgeError::Eval(
                    "FV requires at least 3 arguments: rate, nper, pmt".to_string(),
                ));
            }

            let rate = self.eval_expression(&args[0], row_idx, table)?;
            let nper = self.eval_expression(&args[1], row_idx, table)?;
            let pmt = self.eval_expression(&args[2], row_idx, table)?;
            let pv = if args.len() > 3 {
                self.eval_expression(&args[3], row_idx, table)?
            } else {
                0.0
            };
            let fv_type = if args.len() > 4 {
                self.eval_expression(&args[4], row_idx, table)? as i32
            } else {
                0
            };

            let fv = if rate == 0.0 {
                -(pv + pmt * nper)
            } else {
                let pvif = (1.0 + rate).powf(nper);
                let fvifa = (pvif - 1.0) / rate;
                let fv_pv = -pv * pvif;
                let fv_pmt = if fv_type == 1 {
                    -pmt * fvifa * (1.0 + rate)
                } else {
                    -pmt * fvifa
                };
                fv_pv + fv_pmt
            };

            result = result.replace(full, &format!("{}", fv));
        }

        // PV(rate, nper, pmt, [fv], [type]) - Present Value
        // Use \b word boundary to avoid matching inside XNPV
        let re_pv = Regex::new(r"\bPV\(([^)]+)\)").unwrap();
        for caps in re_pv.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 3 {
                return Err(ForgeError::Eval(
                    "PV requires at least 3 arguments: rate, nper, pmt".to_string(),
                ));
            }

            let rate = self.eval_expression(&args[0], row_idx, table)?;
            let nper = self.eval_expression(&args[1], row_idx, table)?;
            let pmt = self.eval_expression(&args[2], row_idx, table)?;
            let fv = if args.len() > 3 {
                self.eval_expression(&args[3], row_idx, table)?
            } else {
                0.0
            };
            let pv_type = if args.len() > 4 {
                self.eval_expression(&args[4], row_idx, table)? as i32
            } else {
                0
            };

            let pv = if rate == 0.0 {
                -(fv + pmt * nper)
            } else {
                let pvif = (1.0 + rate).powf(nper);
                let pvifa = (pvif - 1.0) / (rate * pvif);
                let pv_fv = -fv / pvif;
                let pv_pmt = if pv_type == 1 {
                    -pmt * pvifa * (1.0 + rate)
                } else {
                    -pmt * pvifa
                };
                pv_fv + pv_pmt
            };

            result = result.replace(full, &format!("{}", pv));
        }

        // IRR(values, [guess]) - Internal Rate of Return using Newton-Raphson
        // Use \b word boundary to avoid matching XIRR
        let re_irr = Regex::new(r"\bIRR\(([^)]+)\)").unwrap();
        for caps in re_irr.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.is_empty() {
                return Err(ForgeError::Eval(
                    "IRR requires at least one argument: values array".to_string(),
                ));
            }

            let values = self.get_values_from_arg(&args[0], row_idx, table)?;
            let guess = if args.len() > 1 {
                self.eval_expression(&args[1], row_idx, table)?
            } else {
                0.1
            };

            let irr = self.calculate_irr(&values, guess)?;
            result = result.replace(full, &format!("{}", irr));
        }

        // NPER(rate, pmt, pv, [fv], [type]) - Number of periods
        let re_nper = Regex::new(r"NPER\(([^)]+)\)").unwrap();
        for caps in re_nper.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 3 {
                return Err(ForgeError::Eval(
                    "NPER requires at least 3 arguments: rate, pmt, pv".to_string(),
                ));
            }

            let rate = self.eval_expression(&args[0], row_idx, table)?;
            let pmt = self.eval_expression(&args[1], row_idx, table)?;
            let pv = self.eval_expression(&args[2], row_idx, table)?;
            let fv = if args.len() > 3 {
                self.eval_expression(&args[3], row_idx, table)?
            } else {
                0.0
            };
            let nper_type = if args.len() > 4 {
                self.eval_expression(&args[4], row_idx, table)? as i32
            } else {
                0
            };

            let nper = if rate == 0.0 {
                -(pv + fv) / pmt
            } else {
                let pmt_adj = if nper_type == 1 {
                    pmt * (1.0 + rate)
                } else {
                    pmt
                };
                let numerator = pmt_adj - fv * rate;
                let denominator = pv * rate + pmt_adj;
                if denominator == 0.0 || numerator / denominator <= 0.0 {
                    return Err(ForgeError::Eval(
                        "NPER: Cannot calculate number of periods".to_string(),
                    ));
                }
                (numerator / denominator).ln() / (1.0 + rate).ln()
            };

            result = result.replace(full, &format!("{}", nper));
        }

        // RATE(nper, pmt, pv, [fv], [type], [guess]) - Interest rate using Newton-Raphson
        let re_rate = Regex::new(r"RATE\(([^)]+)\)").unwrap();
        for caps in re_rate.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 3 {
                return Err(ForgeError::Eval(
                    "RATE requires at least 3 arguments: nper, pmt, pv".to_string(),
                ));
            }

            let nper = self.eval_expression(&args[0], row_idx, table)?;
            let pmt = self.eval_expression(&args[1], row_idx, table)?;
            let pv = self.eval_expression(&args[2], row_idx, table)?;
            let fv = if args.len() > 3 {
                self.eval_expression(&args[3], row_idx, table)?
            } else {
                0.0
            };
            let rate_type = if args.len() > 4 {
                self.eval_expression(&args[4], row_idx, table)? as i32
            } else {
                0
            };
            let guess = if args.len() > 5 {
                self.eval_expression(&args[5], row_idx, table)?
            } else {
                0.1
            };

            let rate = self.calculate_rate(nper, pmt, pv, fv, rate_type, guess)?;
            result = result.replace(full, &format!("{}", rate));
        }

        // XNPV(rate, values, dates) - Net Present Value with irregular dates
        let re_xnpv = Regex::new(r"XNPV\(([^)]+)\)").unwrap();
        for caps in re_xnpv.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 3 {
                return Err(ForgeError::Eval(
                    "XNPV requires 3 arguments: rate, values, dates".to_string(),
                ));
            }

            let rate = self.eval_expression(&args[0], row_idx, table)?;
            let values = self.get_values_from_arg(&args[1], row_idx, table)?;
            let dates = self.get_dates_from_arg(&args[2], row_idx, table)?;

            if values.len() != dates.len() {
                return Err(ForgeError::Eval(format!(
                    "XNPV: values ({}) and dates ({}) must have same length",
                    values.len(),
                    dates.len()
                )));
            }

            let xnpv = self.calculate_xnpv(rate, &values, &dates)?;
            result = result.replace(full, &format!("{}", xnpv));
        }

        // XIRR(values, dates, [guess]) - Internal Rate of Return with irregular dates
        let re_xirr = Regex::new(r"XIRR\(([^)]+)\)").unwrap();
        for caps in re_xirr.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 2 {
                return Err(ForgeError::Eval(
                    "XIRR requires at least 2 arguments: values, dates".to_string(),
                ));
            }

            let values = self.get_values_from_arg(&args[0], row_idx, table)?;
            let dates = self.get_dates_from_arg(&args[1], row_idx, table)?;
            let guess = if args.len() > 2 {
                self.eval_expression(&args[2], row_idx, table)?
            } else {
                0.1
            };

            if values.len() != dates.len() {
                return Err(ForgeError::Eval(format!(
                    "XIRR: values ({}) and dates ({}) must have same length",
                    values.len(),
                    dates.len()
                )));
            }

            let xirr = self.calculate_xirr(&values, &dates, guess)?;
            result = result.replace(full, &format!("{}", xirr));
        }

        // CHOOSE(index, value1, value2, ...) - Select value by index
        let re_choose = Regex::new(r"CHOOSE\(([^)]+)\)").unwrap();
        for caps in re_choose.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 2 {
                return Err(ForgeError::Eval(
                    "CHOOSE requires at least 2 arguments: index and at least one value"
                        .to_string(),
                ));
            }

            let index = self.eval_expression(&args[0], row_idx, table)? as usize;

            if index == 0 || index > args.len() - 1 {
                return Err(ForgeError::Eval(format!(
                    "CHOOSE: index {} out of range (1 to {})",
                    index,
                    args.len() - 1
                )));
            }

            // index is 1-based in Excel, so args[index] is the correct value
            let chosen_value = self.eval_expression(&args[index], row_idx, table)?;
            result = result.replace(full, &format!("{}", chosen_value));
        }

        // MIRR(values, finance_rate, reinvest_rate) - Modified Internal Rate of Return
        let re_mirr = Regex::new(r"MIRR\(([^)]+)\)").unwrap();
        for caps in re_mirr.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() != 3 {
                return Err(ForgeError::Eval(
                    "MIRR requires exactly 3 arguments: values, finance_rate, reinvest_rate"
                        .to_string(),
                ));
            }

            let values = self.get_values_from_arg(&args[0], row_idx, table)?;
            let finance_rate = self.eval_expression(&args[1], row_idx, table)?;
            let reinvest_rate = self.eval_expression(&args[2], row_idx, table)?;

            let mirr = self.calculate_mirr(&values, finance_rate, reinvest_rate)?;
            result = result.replace(full, &format!("{}", mirr));
        }

        // SLN(cost, salvage, life) - Straight-line depreciation
        let re_sln = Regex::new(r"SLN\(([^)]+)\)").unwrap();
        for caps in re_sln.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() != 3 {
                return Err(ForgeError::Eval(
                    "SLN requires exactly 3 arguments: cost, salvage, life".to_string(),
                ));
            }

            let cost = self.eval_expression(&args[0], row_idx, table)?;
            let salvage = self.eval_expression(&args[1], row_idx, table)?;
            let life = self.eval_expression(&args[2], row_idx, table)?;

            if life == 0.0 {
                return Err(ForgeError::Eval("SLN: life cannot be zero".to_string()));
            }

            let sln = (cost - salvage) / life;
            result = result.replace(full, &format!("{}", sln));
        }

        // DB(cost, salvage, life, period, [month]) - Declining balance depreciation
        let re_db = Regex::new(r"\bDB\(([^)]+)\)").unwrap();
        for caps in re_db.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 4 {
                return Err(ForgeError::Eval(
                    "DB requires at least 4 arguments: cost, salvage, life, period".to_string(),
                ));
            }

            let cost = self.eval_expression(&args[0], row_idx, table)?;
            let salvage = self.eval_expression(&args[1], row_idx, table)?;
            let life = self.eval_expression(&args[2], row_idx, table)?;
            let period = self.eval_expression(&args[3], row_idx, table)?;
            let month = if args.len() > 4 {
                self.eval_expression(&args[4], row_idx, table)?
            } else {
                12.0
            };

            let db = self.calculate_db(cost, salvage, life, period, month)?;
            result = result.replace(full, &format!("{}", db));
        }

        // DDB(cost, salvage, life, period, [factor]) - Double declining balance depreciation
        let re_ddb = Regex::new(r"DDB\(([^)]+)\)").unwrap();
        for caps in re_ddb.captures_iter(formula) {
            let full = caps.get(0).unwrap().as_str();
            let args_str = caps.get(1).unwrap().as_str();
            let args = self.parse_function_args(args_str)?;

            if args.len() < 4 {
                return Err(ForgeError::Eval(
                    "DDB requires at least 4 arguments: cost, salvage, life, period".to_string(),
                ));
            }

            let cost = self.eval_expression(&args[0], row_idx, table)?;
            let salvage = self.eval_expression(&args[1], row_idx, table)?;
            let life = self.eval_expression(&args[2], row_idx, table)?;
            let period = self.eval_expression(&args[3], row_idx, table)?;
            let factor = if args.len() > 4 {
                self.eval_expression(&args[4], row_idx, table)?
            } else {
                2.0 // Default factor for double declining balance
            };

            let ddb = self.calculate_ddb(cost, salvage, life, period, factor)?;
            result = result.replace(full, &format!("{}", ddb));
        }

        Ok(result)
    }

    /// Calculate Modified Internal Rate of Return (MIRR)
    fn calculate_mirr(
        &self,
        values: &[f64],
        finance_rate: f64,
        reinvest_rate: f64,
    ) -> ForgeResult<f64> {
        let n = values.len() as f64;
        if n < 2.0 {
            return Err(ForgeError::Eval(
                "MIRR: values must have at least 2 elements".to_string(),
            ));
        }

        // Present value of negative cash flows (costs) at finance rate
        let mut pv_neg = 0.0;
        // Future value of positive cash flows (returns) at reinvest rate
        let mut fv_pos = 0.0;

        for (i, &cf) in values.iter().enumerate() {
            let t = i as f64;
            if cf < 0.0 {
                pv_neg += cf / (1.0 + finance_rate).powf(t);
            } else {
                fv_pos += cf * (1.0 + reinvest_rate).powf(n - 1.0 - t);
            }
        }

        if pv_neg >= 0.0 {
            return Err(ForgeError::Eval(
                "MIRR: values must contain at least one negative cash flow".to_string(),
            ));
        }
        if fv_pos <= 0.0 {
            return Err(ForgeError::Eval(
                "MIRR: values must contain at least one positive cash flow".to_string(),
            ));
        }

        // MIRR formula: (-fv_pos / pv_neg)^(1/(n-1)) - 1
        let mirr = (-fv_pos / pv_neg).powf(1.0 / (n - 1.0)) - 1.0;
        Ok(mirr)
    }

    /// Calculate Declining Balance depreciation (DB)
    fn calculate_db(
        &self,
        cost: f64,
        salvage: f64,
        life: f64,
        period: f64,
        month: f64,
    ) -> ForgeResult<f64> {
        if life <= 0.0 {
            return Err(ForgeError::Eval("DB: life must be positive".to_string()));
        }
        if period < 1.0 || period > life + 1.0 {
            return Err(ForgeError::Eval(
                "DB: period must be between 1 and life".to_string(),
            ));
        }

        // Calculate rate (Excel rounds to 3 decimal places)
        let rate = (1.0 - (salvage / cost).powf(1.0 / life) * 1000.0).floor() / 1000.0;
        let rate = rate.clamp(0.0, 1.0);

        let mut total_depreciation = 0.0;
        let mut book_value = cost;

        for p in 1..=(period as i32) {
            let depreciation = if p == 1 {
                // First period: prorate by months
                cost * rate * month / 12.0
            } else if p as f64 == (life + 1.0).floor() {
                // Last period: remaining months
                book_value * rate * (12.0 - month) / 12.0
            } else {
                book_value * rate
            };

            total_depreciation = depreciation;
            book_value -= depreciation;
            book_value = book_value.max(salvage);
        }

        Ok(total_depreciation)
    }

    /// Calculate Double Declining Balance depreciation (DDB)
    fn calculate_ddb(
        &self,
        cost: f64,
        salvage: f64,
        life: f64,
        period: f64,
        factor: f64,
    ) -> ForgeResult<f64> {
        if life <= 0.0 {
            return Err(ForgeError::Eval("DDB: life must be positive".to_string()));
        }
        if period < 1.0 || period > life {
            return Err(ForgeError::Eval(
                "DDB: period must be between 1 and life".to_string(),
            ));
        }

        let rate = factor / life;
        let mut book_value = cost;

        for p in 1..=(period as i32) {
            let depreciation = book_value * rate;
            // Don't depreciate below salvage value
            let max_depreciation = (book_value - salvage).max(0.0);
            let actual_depreciation = depreciation.min(max_depreciation);

            if p == period as i32 {
                return Ok(actual_depreciation);
            }

            book_value -= actual_depreciation;
        }

        Ok(0.0)
    }

    /// Replace array functions with evaluated results (v4.1.0)
    /// Supports: UNIQUE, COUNTUNIQUE
    fn replace_array_functions(
        &self,
        formula: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        use regex::Regex;
        let mut result = formula.to_string();

        // COUNTUNIQUE(array) - Count unique values in array
        let re_countunique = Regex::new(r"COUNTUNIQUE\(([^)]+)\)").unwrap();
        for cap in re_countunique
            .captures_iter(&result.clone())
            .collect::<Vec<_>>()
        {
            let full = cap.get(0).unwrap().as_str();
            let array_arg = cap.get(1).unwrap().as_str().trim();

            let unique_count = self.eval_countunique(array_arg, row_idx, table)?;
            result = result.replace(full, &format!("{}", unique_count));
        }

        // UNIQUE(array) - Returns count of unique values (scalar context)
        // In row-wise context, UNIQUE returns the count since we can't return arrays
        // For actual unique values, use in aggregation context
        let re_unique = Regex::new(r"UNIQUE\(([^)]+)\)").unwrap();
        for cap in re_unique.captures_iter(&result.clone()).collect::<Vec<_>>() {
            let full = cap.get(0).unwrap().as_str();
            let array_arg = cap.get(1).unwrap().as_str().trim();

            // In scalar context, UNIQUE returns the count of unique values
            let unique_count = self.eval_countunique(array_arg, row_idx, table)?;
            result = result.replace(full, &format!("{}", unique_count));
        }

        // FILTER(array, include) - Returns values where include is truthy (non-zero)
        // Returns comma-separated values for use with aggregations
        let re_filter = Regex::new(r"FILTER\(([^,]+),\s*([^)]+)\)").unwrap();
        for cap in re_filter.captures_iter(&result.clone()).collect::<Vec<_>>() {
            let full = cap.get(0).unwrap().as_str();
            let array_arg = cap.get(1).unwrap().as_str().trim();
            let include_arg = cap.get(2).unwrap().as_str().trim();

            let filter_result = self.eval_filter(array_arg, include_arg, row_idx, table)?;
            result = result.replace(full, &filter_result);
        }

        // SORT(array, [order]) - Returns sorted values
        // order: 1 = ascending (default), -1 = descending
        let re_sort = Regex::new(r"SORT\(([^,)]+)(?:,\s*([^)]+))?\)").unwrap();
        for cap in re_sort.captures_iter(&result.clone()).collect::<Vec<_>>() {
            let full = cap.get(0).unwrap().as_str();
            let array_arg = cap.get(1).unwrap().as_str().trim();
            let order_arg = cap.get(2).map(|m| m.as_str().trim());

            let sort_result = self.eval_sort(array_arg, order_arg, row_idx, table)?;
            result = result.replace(full, &sort_result);
        }

        Ok(result)
    }

    /// Evaluate FILTER function - returns values where include array is truthy
    fn eval_filter(
        &self,
        array_arg: &str,
        include_arg: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        // Get values from the array
        let values = self.get_values_from_arg(array_arg, row_idx, table)?;

        // Get include criteria (must be same length as values)
        let include = self.get_values_from_arg(include_arg, row_idx, table)?;

        if values.len() != include.len() {
            return Err(ForgeError::Eval(format!(
                "FILTER: array ({} rows) and include ({} rows) must have same length",
                values.len(),
                include.len()
            )));
        }

        // Filter values where include is truthy (non-zero)
        let filtered: Vec<f64> = values
            .iter()
            .zip(include.iter())
            .filter(|(_, inc)| **inc != 0.0)
            .map(|(val, _)| *val)
            .collect();

        if filtered.is_empty() {
            return Err(ForgeError::Eval(
                "FILTER: No values match the criteria".to_string(),
            ));
        }

        // Return as comma-separated values
        Ok(filtered
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(", "))
    }

    /// Evaluate SORT function - returns sorted values
    fn eval_sort(
        &self,
        array_arg: &str,
        order_arg: Option<&str>,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<String> {
        // Get values from the array
        let mut values = self.get_values_from_arg(array_arg, row_idx, table)?;

        // Determine sort order (1 = asc, -1 = desc)
        let descending = if let Some(order) = order_arg {
            let order_val = self.eval_expression(order, row_idx, table)?;
            order_val < 0.0
        } else {
            false // Default: ascending
        };

        // Sort values
        values.sort_by(|a, b| {
            if descending {
                b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        // Return as comma-separated values
        Ok(values
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(", "))
    }

    /// Evaluate COUNTUNIQUE - count unique values in a column/array
    fn eval_countunique(
        &self,
        array_arg: &str,
        _row_idx: usize,
        table: &Table,
    ) -> ForgeResult<usize> {
        let array_arg = array_arg.trim();

        // Check if it's a cross-table reference (table.column)
        if array_arg.contains('.') {
            let parts: Vec<&str> = array_arg.split('.').collect();
            if parts.len() == 2 {
                let table_name = parts[0];
                let col_name = parts[1];

                if let Some(ref_table) = self.model.tables.get(table_name) {
                    if let Some(col) = ref_table.columns.get(col_name) {
                        return self.count_unique_in_column(col);
                    }
                }
                return Err(ForgeError::Eval(format!(
                    "COUNTUNIQUE: Column '{}' not found in table '{}'",
                    col_name, table_name
                )));
            }
        }

        // Check if it's a local column reference
        if let Some(col) = table.columns.get(array_arg) {
            return self.count_unique_in_column(col);
        }

        Err(ForgeError::Eval(format!(
            "COUNTUNIQUE: '{}' is not a valid column reference. Use 'column_name' or 'table.column'",
            array_arg
        )))
    }

    /// Count unique values in a column
    fn count_unique_in_column(&self, col: &Column) -> ForgeResult<usize> {
        match &col.values {
            ColumnValue::Number(v) => {
                let mut seen: HashSet<String> = HashSet::new();
                for val in v {
                    // Use string representation to handle floating point comparison
                    seen.insert(format!("{:.10}", val));
                }
                Ok(seen.len())
            }
            ColumnValue::Text(v) => {
                let seen: HashSet<&String> = v.iter().collect();
                Ok(seen.len())
            }
            ColumnValue::Boolean(v) => {
                let seen: HashSet<&bool> = v.iter().collect();
                Ok(seen.len())
            }
            ColumnValue::Date(v) => {
                let seen: HashSet<&String> = v.iter().collect();
                Ok(seen.len())
            }
        }
    }

    /// Get values from an argument - handles both single values and column references
    fn get_values_from_arg(
        &self,
        arg: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<Vec<f64>> {
        let arg = arg.trim();

        // Check if it's a column reference (table.column)
        if arg.contains('.') {
            let parts: Vec<&str> = arg.split('.').collect();
            if parts.len() == 2 {
                let table_name = parts[0];
                let col_name = parts[1];

                if let Some(ref_table) = self.model.tables.get(table_name) {
                    if let Some(col) = ref_table.columns.get(col_name) {
                        return self.column_to_f64_vec(col);
                    }
                }
            }
        }

        // Check if it's a local column reference
        if let Some(col) = table.columns.get(arg) {
            return self.column_to_f64_vec(col);
        }

        // Try to evaluate as a single expression
        let value = self.eval_expression(arg, row_idx, table)?;
        Ok(vec![value])
    }

    /// Convert a Column to a Vec<f64> for financial functions
    fn column_to_f64_vec(&self, col: &Column) -> ForgeResult<Vec<f64>> {
        match &col.values {
            ColumnValue::Number(v) => Ok(v.clone()),
            ColumnValue::Boolean(v) => Ok(v.iter().map(|&b| if b { 1.0 } else { 0.0 }).collect()),
            ColumnValue::Text(_) => Err(ForgeError::Eval(format!(
                "Cannot use text column '{}' in financial function",
                col.name
            ))),
            ColumnValue::Date(_) => Err(ForgeError::Eval(format!(
                "Cannot use date column '{}' in financial function",
                col.name
            ))),
        }
    }

    /// Calculate IRR using Newton-Raphson method
    fn calculate_irr(&self, values: &[f64], guess: f64) -> ForgeResult<f64> {
        const MAX_ITERATIONS: usize = 100;
        const TOLERANCE: f64 = 1e-10;

        let mut rate = guess;

        for _ in 0..MAX_ITERATIONS {
            let mut npv = 0.0;
            let mut d_npv = 0.0; // Derivative of NPV

            for (i, &cf) in values.iter().enumerate() {
                let t = i as f64;
                let factor = (1.0 + rate).powf(t);
                npv += cf / factor;
                if i > 0 {
                    d_npv -= t * cf / ((1.0 + rate).powf(t + 1.0));
                }
            }

            if d_npv.abs() < TOLERANCE {
                return Err(ForgeError::Eval("IRR: Derivative too small".to_string()));
            }

            let new_rate = rate - npv / d_npv;

            if (new_rate - rate).abs() < TOLERANCE {
                return Ok(new_rate);
            }

            rate = new_rate;
        }

        Err(ForgeError::Eval("IRR: Did not converge".to_string()))
    }

    /// Calculate RATE using Newton-Raphson method
    fn calculate_rate(
        &self,
        nper: f64,
        pmt: f64,
        pv: f64,
        fv: f64,
        rate_type: i32,
        guess: f64,
    ) -> ForgeResult<f64> {
        const MAX_ITERATIONS: usize = 100;
        const TOLERANCE: f64 = 1e-10;

        let mut rate = guess;

        for _ in 0..MAX_ITERATIONS {
            // Calculate f(rate) = pv * (1+rate)^n + pmt * (1+rate*type) * ((1+rate)^n - 1) / rate + fv
            let pvif = (1.0 + rate).powf(nper);
            let pmt_factor = if rate_type == 1 { 1.0 + rate } else { 1.0 };

            let f = if rate.abs() < TOLERANCE {
                pv + pmt * nper * pmt_factor + fv
            } else {
                pv * pvif + pmt * pmt_factor * (pvif - 1.0) / rate + fv
            };

            // Calculate f'(rate) - derivative
            let f_prime = if rate.abs() < TOLERANCE {
                nper * pv + pmt * nper * (nper - 1.0) / 2.0
            } else {
                nper * pv * (1.0 + rate).powf(nper - 1.0)
                    + pmt
                        * pmt_factor
                        * (nper * (1.0 + rate).powf(nper - 1.0) * rate - (pvif - 1.0))
                        / (rate * rate)
            };

            if f_prime.abs() < TOLERANCE {
                return Err(ForgeError::Eval("RATE: Derivative too small".to_string()));
            }

            let new_rate = rate - f / f_prime;

            if (new_rate - rate).abs() < TOLERANCE {
                return Ok(new_rate);
            }

            rate = new_rate;
        }

        Err(ForgeError::Eval("RATE: Did not converge".to_string()))
    }

    /// Get dates from an argument - handles both single values and column references
    fn get_dates_from_arg(
        &self,
        arg: &str,
        row_idx: usize,
        table: &Table,
    ) -> ForgeResult<Vec<f64>> {
        let arg = arg.trim();

        // Check if it's a column reference (table.column)
        if arg.contains('.') {
            let parts: Vec<&str> = arg.split('.').collect();
            if parts.len() == 2 {
                let table_name = parts[0];
                let col_name = parts[1];

                if let Some(ref_table) = self.model.tables.get(table_name) {
                    if let Some(col) = ref_table.columns.get(col_name) {
                        return self.column_to_date_serial_vec(col);
                    }
                }
            }
        }

        // Check if it's a local column reference
        if let Some(col) = table.columns.get(arg) {
            return self.column_to_date_serial_vec(col);
        }

        // Try to evaluate as a single date string
        let date_str = self.eval_text_expression(arg, row_idx, table)?;
        let serial = self.date_string_to_serial(&date_str)?;
        Ok(vec![serial])
    }

    /// Calculate XNPV (Net Present Value with irregular dates)
    fn calculate_xnpv(&self, rate: f64, values: &[f64], dates: &[f64]) -> ForgeResult<f64> {
        if values.is_empty() || dates.is_empty() {
            return Err(ForgeError::Eval(
                "XNPV: values and dates cannot be empty".to_string(),
            ));
        }

        let first_date = dates[0];
        let mut xnpv = 0.0;

        for (value, &date) in values.iter().zip(dates.iter()) {
            let years = (date - first_date) / 365.0;
            xnpv += value / (1.0 + rate).powf(years);
        }

        Ok(xnpv)
    }

    /// Calculate XIRR (Internal Rate of Return with irregular dates) using Newton-Raphson
    fn calculate_xirr(&self, values: &[f64], dates: &[f64], guess: f64) -> ForgeResult<f64> {
        const MAX_ITERATIONS: usize = 100;
        const TOLERANCE: f64 = 1e-10;

        if values.is_empty() || dates.is_empty() {
            return Err(ForgeError::Eval(
                "XIRR: values and dates cannot be empty".to_string(),
            ));
        }

        // Check that there's at least one positive and one negative value
        let has_positive = values.iter().any(|&v| v > 0.0);
        let has_negative = values.iter().any(|&v| v < 0.0);
        if !has_positive || !has_negative {
            return Err(ForgeError::Eval(
                "XIRR: values must contain at least one positive and one negative value"
                    .to_string(),
            ));
        }

        let first_date = dates[0];
        let mut rate = guess;

        for _ in 0..MAX_ITERATIONS {
            let mut xnpv = 0.0;
            let mut d_xnpv = 0.0; // Derivative of XNPV

            for (i, &value) in values.iter().enumerate() {
                let years = (dates[i] - first_date) / 365.0;
                let factor = (1.0 + rate).powf(years);
                xnpv += value / factor;
                if years != 0.0 {
                    d_xnpv -= years * value / ((1.0 + rate).powf(years + 1.0));
                }
            }

            if d_xnpv.abs() < TOLERANCE {
                // Try a different guess
                if rate != 0.0 {
                    rate = 0.0;
                    continue;
                }
                return Err(ForgeError::Eval("XIRR: Derivative too small".to_string()));
            }

            let new_rate = rate - xnpv / d_xnpv;

            // Bound the rate to prevent overflow
            let new_rate = new_rate.clamp(-0.99, 10.0);

            if (new_rate - rate).abs() < TOLERANCE {
                return Ok(new_rate);
            }

            rate = new_rate;
        }

        Err(ForgeError::Eval("XIRR: Did not converge".to_string()))
    }
}

/// Lookup value type (supports numbers, text, and booleans)
#[derive(Debug, Clone)]
enum LookupValue {
    Number(f64),
    Text(String),
    Boolean(bool),
}

/// Switch value type for SWITCH function comparison
#[derive(Debug, Clone)]
enum SwitchValue {
    Number(f64),
    Text(String),
}

#[cfg(test)]
mod tests;
