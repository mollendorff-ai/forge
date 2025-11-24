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
                    if self.model.tables.contains_key(&table_name) {
                        if !deps.contains(&table_name) {
                            deps.push(table_name);
                        }
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
    }

    /// Evaluate a row-wise formula (element-wise operations)
    /// Example: profit = revenue - expenses
    /// Evaluates: profit[i] = revenue[i] - expenses[i] for all i
    fn evaluate_rowwise_formula(&mut self, table: &Table, formula: &str) -> ForgeResult<ColumnValue> {
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
            // Check if this is a cross-table reference (table.column format)
            if col_ref.contains('.') {
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
        let mut results = Vec::new();
        for row_idx in 0..row_count {
            // Create a resolver for this specific row
            let resolver = |var_name: String| -> types::Value {
                // Check if this is a cross-table reference (table.column format)
                if var_name.contains('.') {
                    let parts: Vec<&str> = var_name.split('.').collect();
                    if parts.len() == 2 {
                        let ref_table_name = parts[0];
                        let ref_col_name = parts[1];

                        if let Some(ref_table) = self.model.tables.get(ref_table_name) {
                            if let Some(ref_col) = ref_table.columns.get(ref_col_name) {
                                match &ref_col.values {
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
    fn extract_scalar_dependencies(&self, formula: &str, scalar_name: &str) -> ForgeResult<Vec<String>> {
        let mut deps = Vec::new();

        // Extract parent section from scalar_name (e.g., "annual_2025" from "annual_2025.total_revenue")
        let parent_section = if let Some(dot_pos) = scalar_name.rfind('.') {
            Some(&scalar_name[..dot_pos])
        } else {
            None
        };

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
        } else {
            // Regular scalar formula - use xlformula_engine
            self.evaluate_scalar_with_resolver(&formula_str, scalar_name)
        }
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
                ForgeError::Eval(format!("Column '{}' not found in table '{}'", col_name, table_name))
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

    /// Evaluate aggregation formula (SUM, AVERAGE, MAX, MIN)
    fn evaluate_aggregation(&self, formula: &str) -> ForgeResult<f64> {
        let upper = formula.to_uppercase();

        // Extract function name and argument
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
        } else {
            return Err(ForgeError::Eval("Unknown aggregation function".to_string()));
        };

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

    /// Evaluate scalar formula with variable resolver
    fn evaluate_scalar_with_resolver(&self, formula: &str, scalar_name: &str) -> ForgeResult<f64> {
        // Extract parent section from scalar_name (e.g., "annual_2025" from "annual_2025.total_revenue")
        let parent_section = if let Some(dot_pos) = scalar_name.rfind('.') {
            Some(scalar_name[..dot_pos].to_string())
        } else {
            None
        };

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

        let parsed = parse_formula::parse_string_to_formula(formula, None::<NoCustomFunction>);
        let result = calculate::calculate_formula(parsed, Some(&resolver));

        match result {
            types::Value::Number(n) => Ok(n as f64),
            types::Value::Error(e) => Err(ForgeError::Eval(format!(
                "Formula '{}' returned error: {:?}",
                formula, e
            ))),
            other => Err(ForgeError::Eval(format!(
                "Formula '{}' returned unexpected type: {:?}",
                formula, other
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
                        | "IF"
                        | "AND"
                        | "OR"
                        | "NOT"
                        | "ABS"
                        | "ROUND"
                        | "POWER"
                        | "SQRT"
                ) && !refs.contains(&word.to_string())
                {
                    refs.push(word.to_string());
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

        let refs = calc
            .extract_column_references("=revenue - expenses")
            .unwrap();
        assert_eq!(refs.len(), 2);
        assert!(refs.contains(&"revenue".to_string()));
        assert!(refs.contains(&"expenses".to_string()));

        let refs2 = calc
            .extract_column_references("=revenue * 0.3 + fixed_cost")
            .unwrap();
        assert!(refs2.contains(&"revenue".to_string()));
        assert!(refs2.contains(&"fixed_cost".to_string()));
    }

    #[test]
    fn test_aggregation_sum() {
        use crate::types::Variable;

        let mut model = ParsedModel::new(ForgeVersion::V1_0_0);

        // Create a table with revenue column
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0, 400.0]),
        ));
        model.add_table(table);

        // Add scalar with SUM formula
        let total_revenue = Variable {
            path: "total_revenue".to_string(),
            value: None,
            formula: Some("=SUM(sales.revenue)".to_string()),
            alias: None,
        };
        model.add_scalar("total_revenue".to_string(), total_revenue);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let total = result.scalars.get("total_revenue").unwrap();
        assert_eq!(total.value, Some(1000.0));
    }

    #[test]
    fn test_aggregation_average() {
        use crate::types::Variable;

        let mut model = ParsedModel::new(ForgeVersion::V1_0_0);

        let mut table = Table::new("metrics".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        model.add_table(table);

        let avg_value = Variable {
            path: "avg_value".to_string(),
            value: None,
            formula: Some("=AVERAGE(metrics.values)".to_string()),
            alias: None,
        };
        model.add_scalar("avg_value".to_string(), avg_value);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        let avg = result.scalars.get("avg_value").unwrap();
        assert_eq!(avg.value, Some(25.0));
    }

    #[test]
    fn test_array_indexing() {
        use crate::types::Variable;

        let mut model = ParsedModel::new(ForgeVersion::V1_0_0);

        let mut table = Table::new("quarterly".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![1000.0, 1200.0, 1500.0, 1800.0]),
        ));
        model.add_table(table);

        let q1_revenue = Variable {
            path: "q1_revenue".to_string(),
            value: None,
            formula: Some("=quarterly.revenue[0]".to_string()),
            alias: None,
        };
        model.add_scalar("q1_revenue".to_string(), q1_revenue);

        let q4_revenue = Variable {
            path: "q4_revenue".to_string(),
            value: None,
            formula: Some("=quarterly.revenue[3]".to_string()),
            alias: None,
        };
        model.add_scalar("q4_revenue".to_string(), q4_revenue);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(
            result.scalars.get("q1_revenue").unwrap().value,
            Some(1000.0)
        );
        assert_eq!(
            result.scalars.get("q4_revenue").unwrap().value,
            Some(1800.0)
        );
    }

    #[test]
    fn test_scalar_dependencies() {
        use crate::types::Variable;

        let mut model = ParsedModel::new(ForgeVersion::V1_0_0);

        let mut table = Table::new("pl".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![1000.0, 1200.0]),
        ));
        table.add_column(Column::new(
            "cogs".to_string(),
            ColumnValue::Number(vec![300.0, 360.0]),
        ));
        model.add_table(table);

        // total_revenue depends on table
        let total_revenue = Variable {
            path: "total_revenue".to_string(),
            value: None,
            formula: Some("=SUM(pl.revenue)".to_string()),
            alias: None,
        };
        model.add_scalar("total_revenue".to_string(), total_revenue);

        // total_cogs depends on table
        let total_cogs = Variable {
            path: "total_cogs".to_string(),
            value: None,
            formula: Some("=SUM(pl.cogs)".to_string()),
            alias: None,
        };
        model.add_scalar("total_cogs".to_string(), total_cogs);

        // gross_profit depends on total_revenue and total_cogs
        let gross_profit = Variable {
            path: "gross_profit".to_string(),
            value: None,
            formula: Some("=total_revenue - total_cogs".to_string()),
            alias: None,
        };
        model.add_scalar("gross_profit".to_string(), gross_profit);

        // gross_margin depends on gross_profit and total_revenue
        let gross_margin = Variable {
            path: "gross_margin".to_string(),
            value: None,
            formula: Some("=gross_profit / total_revenue".to_string()),
            alias: None,
        };
        model.add_scalar("gross_margin".to_string(), gross_margin);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(
            result.scalars.get("total_revenue").unwrap().value,
            Some(2200.0)
        );
        assert_eq!(result.scalars.get("total_cogs").unwrap().value, Some(660.0));
        assert_eq!(
            result.scalars.get("gross_profit").unwrap().value,
            Some(1540.0)
        );
        assert!((result.scalars.get("gross_margin").unwrap().value.unwrap() - 0.7).abs() < 0.0001);
    }

    #[test]
    fn test_aggregation_max_min() {
        use crate::types::Variable;

        let mut model = ParsedModel::new(ForgeVersion::V1_0_0);

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![15.0, 42.0, 8.0, 23.0]),
        ));
        model.add_table(table);

        let max_value = Variable {
            path: "max_value".to_string(),
            value: None,
            formula: Some("=MAX(data.values)".to_string()),
            alias: None,
        };
        model.add_scalar("max_value".to_string(), max_value);

        let min_value = Variable {
            path: "min_value".to_string(),
            value: None,
            formula: Some("=MIN(data.values)".to_string()),
            alias: None,
        };
        model.add_scalar("min_value".to_string(), min_value);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();

        assert_eq!(result.scalars.get("max_value").unwrap().value, Some(42.0));
        assert_eq!(result.scalars.get("min_value").unwrap().value, Some(8.0));
    }
}
