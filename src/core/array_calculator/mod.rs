mod dates;
pub mod evaluator;
mod math;
pub mod parser;
mod text;
pub mod tokenizer;

use crate::error::{ForgeError, ForgeResult};
use crate::types::{Column, ColumnValue, ParsedModel, Table};

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
                // Parse table.column reference
                let parts: Vec<&str> = word.splitn(2, '.').collect();
                if parts.len() == 2 {
                    let table_name = parts[0].to_string();
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

        // Validate all columns have the same length
        if let Err(e) = working_table.validate_lengths() {
            return Err(ForgeError::Eval(format!("Table '{}': {}", table_name, e)));
        }

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
                    // Row-wise: returns an array (v5.2.0 AST evaluator)
                    let result = self.evaluate_rowwise_formula_ast(&working_table, &formula)?;
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

    /// Extract column references from a formula
    fn extract_column_references(&self, formula: &str) -> ForgeResult<Vec<String>> {
        let mut refs = Vec::new();
        for word in formula.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.') {
            let word = word.trim();
            if word.is_empty() {
                continue;
            }
            // Skip numbers and known functions
            if word.parse::<f64>().is_ok() {
                continue;
            }
            let upper = word.to_uppercase();
            if [
                "SUM",
                "AVERAGE",
                "AVG",
                "MAX",
                "MIN",
                "COUNT",
                "IF",
                "AND",
                "OR",
                "NOT",
                "ROUND",
                "ROUNDUP",
                "ROUNDDOWN",
                "ABS",
                "SQRT",
                "POWER",
                "MOD",
                "FLOOR",
                "CEILING",
                "EXP",
                "LN",
                "LOG",
                "INT",
                "TODAY",
                "YEAR",
                "MONTH",
                "DAY",
                "DATE",
                "EDATE",
                "EOMONTH",
                "CONCAT",
                "UPPER",
                "LOWER",
                "TRIM",
                "LEN",
                "LEFT",
                "RIGHT",
                "MID",
                "INDEX",
                "MATCH",
                "XLOOKUP",
                "VLOOKUP",
                "PMT",
                "FV",
                "PV",
                "NPV",
                "IRR",
                "IFERROR",
                "CHOOSE",
                "SWITCH",
                "LET",
                "MEDIAN",
                "STDEV",
                "VAR",
                "COUNTA",
                "COUNTBLANK",
                "SUMIF",
                "COUNTIF",
                "AVERAGEIF",
                "SUMIFS",
                "COUNTIFS",
                "AVERAGEIFS",
                "PERCENTILE",
                "QUARTILE",
                "CORREL",
                "TRUE",
                "FALSE",
            ]
            .contains(&upper.as_str())
            {
                continue;
            }
            refs.push(word.to_string());
        }
        Ok(refs)
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

    // ═══════════════════════════════════════════════════════════════════════════
    // AST-BASED FORMULA EVALUATION (v5.2.0 - Parser Architecture Refactor)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Build an evaluation context from the model state for a given table
    fn build_eval_context(&self, table: &Table) -> evaluator::EvalContext {
        use std::collections::HashMap;
        let mut ctx = evaluator::EvalContext::new();

        // Add all scalars to context
        for (name, scalar) in &self.model.scalars {
            if let Some(value) = scalar.value {
                ctx.scalars
                    .insert(name.clone(), evaluator::Value::Number(value));
                // Also add short name (e.g., "price" from "summary.price")
                // so formulas can reference without prefix
                if let Some(short_name) = name.split('.').next_back() {
                    if short_name != name {
                        ctx.scalars
                            .insert(short_name.to_string(), evaluator::Value::Number(value));
                    }
                }
            }
        }

        // Add current table columns
        for (col_name, col) in &table.columns {
            let values: Vec<evaluator::Value> = match &col.values {
                ColumnValue::Number(nums) => {
                    nums.iter().map(|n| evaluator::Value::Number(*n)).collect()
                }
                ColumnValue::Text(texts) => texts
                    .iter()
                    .map(|s| evaluator::Value::Text(s.clone()))
                    .collect(),
                ColumnValue::Boolean(bools) => bools
                    .iter()
                    .map(|b| evaluator::Value::Boolean(*b))
                    .collect(),
                ColumnValue::Date(dates) => dates
                    .iter()
                    .map(|d| evaluator::Value::Text(d.clone()))
                    .collect(),
            };
            ctx.scalars
                .insert(col_name.clone(), evaluator::Value::Array(values));
        }

        // Add all tables to context
        for (table_name, tbl) in &self.model.tables {
            let mut table_data: HashMap<String, Vec<evaluator::Value>> = HashMap::new();
            for (col_name, col) in &tbl.columns {
                let values: Vec<evaluator::Value> = match &col.values {
                    ColumnValue::Number(nums) => {
                        nums.iter().map(|n| evaluator::Value::Number(*n)).collect()
                    }
                    ColumnValue::Text(texts) => texts
                        .iter()
                        .map(|s| evaluator::Value::Text(s.clone()))
                        .collect(),
                    ColumnValue::Boolean(bools) => bools
                        .iter()
                        .map(|b| evaluator::Value::Boolean(*b))
                        .collect(),
                    ColumnValue::Date(dates) => dates
                        .iter()
                        .map(|d| evaluator::Value::Text(d.clone()))
                        .collect(),
                };
                table_data.insert(col_name.clone(), values);
            }
            ctx.tables.insert(table_name.clone(), table_data);
        }

        // Add scenarios to context
        for (scenario_name, scenario) in &self.model.scenarios {
            let mut overrides = HashMap::new();
            for (var_name, value) in &scenario.overrides {
                overrides.insert(var_name.clone(), *value);
            }
            ctx.scenarios.insert(scenario_name.clone(), overrides);
        }

        ctx.row_count = Some(table.row_count());
        ctx
    }

    /// Evaluate a row-wise formula using the AST evaluator
    fn evaluate_rowwise_formula_ast(
        &self,
        table: &Table,
        formula: &str,
    ) -> ForgeResult<ColumnValue> {
        let formula_str = formula.trim_start_matches('=').trim();
        let tokens = tokenizer::tokenize(formula_str)
            .map_err(|e| ForgeError::Eval(format!("Tokenize: {}", e.message)))?;
        let ast =
            parser::parse(tokens).map_err(|e| ForgeError::Eval(format!("Parse: {}", e.message)))?;

        let base_ctx = self.build_eval_context(table);
        let row_count = table.row_count();
        if row_count == 0 {
            return Err(ForgeError::Eval(
                "Cannot evaluate on empty table".to_string(),
            ));
        }

        // Evaluate first row to determine result type
        let first_ctx = base_ctx.clone().with_row(0, row_count);
        let first_result = evaluator::evaluate(&ast, &first_ctx)
            .map_err(|e| ForgeError::Eval(format!("Row 0: {}", e)))?;

        // Determine column type from first result and evaluate all rows
        match &first_result {
            evaluator::Value::Text(_) => {
                let mut results: Vec<String> = Vec::with_capacity(row_count);
                results.push(first_result.as_text());
                for row_idx in 1..row_count {
                    let row_ctx = base_ctx.clone().with_row(row_idx, row_count);
                    let result = evaluator::evaluate(&ast, &row_ctx)
                        .map_err(|e| ForgeError::Eval(format!("Row {}: {}", row_idx, e)))?;
                    results.push(result.as_text());
                }
                Ok(ColumnValue::Text(results))
            }
            evaluator::Value::Boolean(_) => {
                let mut results: Vec<bool> = Vec::with_capacity(row_count);
                results.push(first_result.as_bool().unwrap_or(false));
                for row_idx in 1..row_count {
                    let row_ctx = base_ctx.clone().with_row(row_idx, row_count);
                    let result = evaluator::evaluate(&ast, &row_ctx)
                        .map_err(|e| ForgeError::Eval(format!("Row {}: {}", row_idx, e)))?;
                    results.push(result.as_bool().unwrap_or(false));
                }
                Ok(ColumnValue::Boolean(results))
            }
            _ => {
                // Default to numeric
                let mut results: Vec<f64> = Vec::with_capacity(row_count);
                let first_num = first_result
                    .as_number()
                    .ok_or_else(|| ForgeError::Eval("Row 0 not a number".to_string()))?;
                results.push(first_num);
                for row_idx in 1..row_count {
                    let row_ctx = base_ctx.clone().with_row(row_idx, row_count);
                    let result = evaluator::evaluate(&ast, &row_ctx)
                        .map_err(|e| ForgeError::Eval(format!("Row {}: {}", row_idx, e)))?;
                    let value = result
                        .as_number()
                        .ok_or_else(|| ForgeError::Eval(format!("Row {} not a number", row_idx)))?;
                    results.push(value);
                }
                Ok(ColumnValue::Number(results))
            }
        }
    }

    /// Evaluate a scalar formula using the AST evaluator
    fn evaluate_scalar_formula_ast(&self, formula: &str) -> ForgeResult<f64> {
        let formula_str = formula.trim_start_matches('=').trim();
        let tokens = tokenizer::tokenize(formula_str)
            .map_err(|e| ForgeError::Eval(format!("Tokenize: {}", e.message)))?;
        let ast =
            parser::parse(tokens).map_err(|e| ForgeError::Eval(format!("Parse: {}", e.message)))?;

        let empty_table = Table::new("_scalar_context".to_string());
        let ctx = self.build_eval_context(&empty_table);
        let result = evaluator::evaluate(&ast, &ctx)
            .map_err(|e| ForgeError::Eval(format!("Eval: {}", e)))?;
        result
            .as_number()
            .ok_or_else(|| ForgeError::Eval("Scalar result not a number".to_string()))
    }

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
                // v5.2.0 AST evaluator
                let value = self.evaluate_scalar_formula_ast(&formula)?;

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
}

#[cfg(test)]
mod tests;
