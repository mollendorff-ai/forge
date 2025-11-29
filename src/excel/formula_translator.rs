//! Formula translation from YAML to Excel syntax

use crate::error::{ForgeError, ForgeResult};
use regex::Regex;
use std::collections::HashMap;

/// Translates YAML row formulas to Excel cell formulas
pub struct FormulaTranslator {
    /// Maps column names to Excel column letters (revenue → A, cogs → B, etc.)
    column_map: HashMap<String, String>,
    /// Global mapping: table_name -> (column_name -> column_letter)
    table_column_maps: HashMap<String, HashMap<String, String>>,
    /// Global mapping: table_name -> row_count
    table_row_counts: HashMap<String, usize>,
}

impl FormulaTranslator {
    /// Create a new formula translator with column mappings (legacy, for backwards compat)
    pub fn new(column_map: HashMap<String, String>) -> Self {
        Self {
            column_map,
            table_column_maps: HashMap::new(),
            table_row_counts: HashMap::new(),
        }
    }

    /// Create a new formula translator with full table knowledge
    pub fn new_with_tables(
        column_map: HashMap<String, String>,
        table_column_maps: HashMap<String, HashMap<String, String>>,
        table_row_counts: HashMap<String, usize>,
    ) -> Self {
        Self {
            column_map,
            table_column_maps,
            table_row_counts,
        }
    }

    /// Translate a row formula to an Excel cell formula for a specific row
    ///
    /// Example:
    /// - Input: `=revenue - cogs`, row_idx 0, excel_row 2
    /// - Output: `=A2-B2`
    pub fn translate_row_formula(&self, formula: &str, excel_row: u32) -> ForgeResult<String> {
        // Remove leading = if present
        let formula_body = formula.strip_prefix('=').unwrap_or(formula);

        // Pattern to match variable names (alphanumeric + underscore, but not starting with digit)
        // Also matches table.column references
        let var_pattern = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)?)\b")
            .map_err(|e| ForgeError::Export(format!("Regex error: {}", e)))?;

        let mut result = formula_body.to_string();

        // Find all variable references and replace them
        let matches: Vec<_> = var_pattern.find_iter(formula_body).collect();

        // Replace in reverse order to maintain string positions
        for match_obj in matches.iter().rev() {
            let var_name = match_obj.as_str();

            // Skip Excel functions (SUM, AVERAGE, etc.)
            if self.is_excel_function(var_name) {
                continue;
            }

            // Check if it's a cross-table reference (table.column)
            if var_name.contains('.') {
                let excel_ref = self.translate_table_column_ref(var_name, excel_row)?;
                result.replace_range(match_obj.range(), &excel_ref);
            } else {
                // Simple column reference
                if let Some(col_letter) = self.column_map.get(var_name) {
                    let excel_ref = format!("{}{}", col_letter, excel_row);
                    result.replace_range(match_obj.range(), &excel_ref);
                } else {
                    return Err(ForgeError::Export(format!(
                        "Column '{}' not found in table",
                        var_name
                    )));
                }
            }
        }

        Ok(format!("={}", result))
    }

    /// Check if a word is an Excel function
    fn is_excel_function(&self, word: &str) -> bool {
        let upper = word.to_uppercase();
        matches!(
            upper.as_str(),
            // Aggregation functions
            "SUM"
                | "AVERAGE"
                | "MAX"
                | "MIN"
                | "COUNT"
                | "COUNTA"
                | "PRODUCT"
                // Conditional aggregations
                | "SUMIF"
                | "SUMIFS"
                | "COUNTIF"
                | "COUNTIFS"
                | "AVERAGEIF"
                | "AVERAGEIFS"
                | "MAXIFS"
                | "MINIFS"
                // Logical functions
                | "IF"
                | "AND"
                | "OR"
                | "NOT"
                | "XOR"
                | "TRUE"
                | "FALSE"
                | "IFERROR"
                | "IFNA"
                | "CHOOSE"
                // Math functions
                | "ABS"
                | "ROUND"
                | "ROUNDUP"
                | "ROUNDDOWN"
                | "SQRT"
                | "POW"
                | "POWER"
                | "EXP"
                | "LN"
                | "LOG"
                | "LOG10"
                | "PI"
                | "E"
                | "MOD"
                | "CEILING"
                | "FLOOR"
                // Text functions
                | "CONCATENATE"
                | "CONCAT"
                | "LEFT"
                | "RIGHT"
                | "MID"
                | "LEN"
                | "UPPER"
                | "LOWER"
                | "TRIM"
                // Date functions
                | "TODAY"
                | "NOW"
                | "DATE"
                | "YEAR"
                | "MONTH"
                | "DAY"
                | "DATEDIF"
                | "EDATE"
                | "EOMONTH"
                // Financial functions
                | "NPV"
                | "IRR"
                | "XNPV"
                | "XIRR"
                | "PMT"
                | "FV"
                | "PV"
                | "RATE"
                | "NPER"
                // Lookup functions
                | "VLOOKUP"
                | "HLOOKUP"
                | "XLOOKUP"
                | "INDEX"
                | "MATCH"
                // Array functions (v4.1.0)
                | "UNIQUE"
                | "COUNTUNIQUE"
        )
    }

    /// Translate table.column reference to Excel sheet reference
    ///
    /// Example:
    /// - Input: `pl_2025.revenue`, row 2
    /// - Output: `'pl_2025'!A2` (where A is the column letter for revenue)
    fn translate_table_column_ref(&self, ref_str: &str, excel_row: u32) -> ForgeResult<String> {
        let parts: Vec<&str> = ref_str.split('.').collect();
        if parts.len() != 2 {
            return Err(ForgeError::Export(format!(
                "Invalid table.column reference: {}",
                ref_str
            )));
        }

        let table_name = parts[0];
        let col_name = parts[1];

        // Look up the column letter from the global table mappings
        // Quote sheet names for LibreOffice compatibility
        if let Some(table_cols) = self.table_column_maps.get(table_name) {
            if let Some(col_letter) = table_cols.get(col_name) {
                return Ok(format!("'{}'!{}{}", table_name, col_letter, excel_row));
            }
        }

        // Fallback: use column name directly (won't work in Excel but better than crashing)
        Ok(format!("'{}'!{}{}", table_name, col_name, excel_row))
    }

    /// Translate a scalar formula to an Excel formula
    ///
    /// Examples:
    /// - `=SUM(table.column)` → `=SUM(table!A2:A4)`
    /// - `=table.column[0]` → `=table!A2`
    /// - `=scalar_name / 100` → `=B3 / 100`
    pub fn translate_scalar_formula(
        &self,
        formula: &str,
        scalar_row_map: &HashMap<String, u32>,
    ) -> ForgeResult<String> {
        // Remove leading = if present
        let formula_body = formula.strip_prefix('=').unwrap_or(formula);

        let mut result = formula_body.to_string();

        // Pattern for table.column[index] references (must be processed first)
        let indexed_pattern =
            Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)\[(\d+)\]")
                .map_err(|e| ForgeError::Export(format!("Regex error: {}", e)))?;

        // Replace indexed references: table.column[0] → table!A2
        let indexed_replacements: Vec<(std::ops::Range<usize>, String)> = indexed_pattern
            .captures_iter(&result.clone())
            .map(|cap| {
                let full_match = cap.get(0).unwrap();
                let table_name = &cap[1];
                let col_name = &cap[2];
                let index: usize = cap[3].parse().unwrap_or(0);

                let col_letter = self
                    .table_column_maps
                    .get(table_name)
                    .and_then(|cols| cols.get(col_name))
                    .cloned()
                    .unwrap_or_else(|| col_name.to_string());

                // Excel row = index + 2 (1 for header, 1 for 1-indexing)
                let excel_row = index + 2;
                let replacement = format!("'{}'!{}{}", table_name, col_letter, excel_row);
                (full_match.range(), replacement)
            })
            .collect();

        // Apply indexed replacements in reverse order
        for (range, replacement) in indexed_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        // Pattern for table.column references inside aggregation functions
        // SUM(table.column) → SUM(table!A2:A4)
        let agg_pattern = Regex::new(r"(SUM|AVERAGE|MAX|MIN|COUNT|COUNTA|PRODUCT)\(([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)\)")
            .map_err(|e| ForgeError::Export(format!("Regex error: {}", e)))?;

        let agg_replacements: Vec<(std::ops::Range<usize>, String)> = agg_pattern
            .captures_iter(&result.clone())
            .map(|cap| {
                let full_match = cap.get(0).unwrap();
                let func_name = &cap[1];
                let table_name = &cap[2];
                let col_name = &cap[3];

                let col_letter = self
                    .table_column_maps
                    .get(table_name)
                    .and_then(|cols| cols.get(col_name))
                    .cloned()
                    .unwrap_or_else(|| col_name.to_string());

                let row_count = self.table_row_counts.get(table_name).copied().unwrap_or(1);
                // Range: row 2 to row (row_count + 1) - header is row 1
                let end_row = row_count + 1;
                let replacement = format!(
                    "{}('{}'!{}2:{}{})",
                    func_name, table_name, col_letter, col_letter, end_row
                );
                (full_match.range(), replacement)
            })
            .collect();

        // Apply aggregation replacements in reverse order
        for (range, replacement) in agg_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        // Pattern for simple table.column references (not in aggregations, not indexed)
        // These become references to row 2 (first data row)
        let simple_table_pattern =
            Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)")
                .map_err(|e| ForgeError::Export(format!("Regex error: {}", e)))?;

        let simple_replacements: Vec<(std::ops::Range<usize>, String)> = simple_table_pattern
            .captures_iter(&result.clone())
            .filter_map(|cap| {
                let full_match = cap.get(0).unwrap();
                let table_name = &cap[1];
                let col_name = &cap[2];

                // Skip if this looks like it's already been processed (contains !)
                if result[full_match.range()].contains('!') {
                    return None;
                }

                // Check if this is actually a table reference
                if !self.table_column_maps.contains_key(table_name) {
                    // Could be a scalar like metrics.total_savings
                    // Check if it's a scalar reference
                    let scalar_name = format!("{}.{}", table_name, col_name);
                    if let Some(&row) = scalar_row_map.get(&scalar_name) {
                        return Some((full_match.range(), format!("B{}", row)));
                    }
                    return None;
                }

                let col_letter = self
                    .table_column_maps
                    .get(table_name)
                    .and_then(|cols| cols.get(col_name))
                    .cloned()
                    .unwrap_or_else(|| col_name.to_string());

                // Default to row 2 (first data row)
                let replacement = format!("'{}'!{}2", table_name, col_letter);
                Some((full_match.range(), replacement))
            })
            .collect();

        // Apply simple replacements in reverse order
        for (range, replacement) in simple_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        // Handle scalar-to-scalar references (e.g., metrics.total_savings → B3)
        // Pattern for standalone scalar names
        let scalar_pattern = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*\.[a-zA-Z_][a-zA-Z0-9_]*)\b")
            .map_err(|e| ForgeError::Export(format!("Regex error: {}", e)))?;

        let scalar_replacements: Vec<(std::ops::Range<usize>, String)> = scalar_pattern
            .captures_iter(&result.clone())
            .filter_map(|cap| {
                let full_match = cap.get(0).unwrap();
                let scalar_name = &cap[1];

                // Skip if already processed (contains ! or is a number)
                if result[full_match.range()].contains('!') {
                    return None;
                }

                if let Some(&row) = scalar_row_map.get(scalar_name) {
                    return Some((full_match.range(), format!("B{}", row)));
                }
                None
            })
            .collect();

        // Apply scalar replacements in reverse order
        for (range, replacement) in scalar_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        Ok(format!("={}", result))
    }

    /// Convert a column name to an Excel column letter
    ///
    /// Examples:
    /// - 0 → A
    /// - 1 → B
    /// - 25 → Z
    /// - 26 → AA
    pub fn column_index_to_letter(index: usize) -> String {
        let mut result = String::new();
        let mut idx = index;

        loop {
            let remainder = idx % 26;
            result.insert(0, (b'A' + remainder as u8) as char);
            if idx < 26 {
                break;
            }
            idx = idx / 26 - 1;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_index_to_letter() {
        assert_eq!(FormulaTranslator::column_index_to_letter(0), "A");
        assert_eq!(FormulaTranslator::column_index_to_letter(1), "B");
        assert_eq!(FormulaTranslator::column_index_to_letter(25), "Z");
        assert_eq!(FormulaTranslator::column_index_to_letter(26), "AA");
        assert_eq!(FormulaTranslator::column_index_to_letter(27), "AB");
        assert_eq!(FormulaTranslator::column_index_to_letter(701), "ZZ");
    }

    #[test]
    fn test_simple_formula_translation() {
        let mut column_map = HashMap::new();
        column_map.insert("revenue".to_string(), "A".to_string());
        column_map.insert("cogs".to_string(), "B".to_string());

        let translator = FormulaTranslator::new(column_map);

        // Test simple subtraction (row 2 in Excel)
        let result = translator
            .translate_row_formula("=revenue - cogs", 2)
            .unwrap();
        assert_eq!(result, "=A2 - B2");

        // Test division (row 3 in Excel)
        let result = translator
            .translate_row_formula("=revenue / cogs", 3)
            .unwrap();
        assert_eq!(result, "=A3 / B3");
    }

    #[test]
    fn test_formula_with_multiple_columns() {
        let mut column_map = HashMap::new();
        column_map.insert("sales_marketing".to_string(), "A".to_string());
        column_map.insert("rd".to_string(), "B".to_string());
        column_map.insert("ga".to_string(), "C".to_string());

        let translator = FormulaTranslator::new(column_map);

        let result = translator
            .translate_row_formula("=sales_marketing + rd + ga", 2)
            .unwrap();
        assert_eq!(result, "=A2 + B2 + C2");
    }

    #[test]
    fn test_formula_with_parentheses() {
        let mut column_map = HashMap::new();
        column_map.insert("gross_profit".to_string(), "A".to_string());
        column_map.insert("revenue".to_string(), "B".to_string());

        let translator = FormulaTranslator::new(column_map);

        let result = translator
            .translate_row_formula("=(gross_profit / revenue) * 100", 2)
            .unwrap();
        assert_eq!(result, "=(A2 / B2) * 100");
    }

    #[test]
    fn test_cross_table_reference() {
        let column_map = HashMap::new(); // Empty for this test

        let translator = FormulaTranslator::new(column_map);

        let result = translator
            .translate_row_formula("=pl_2025.revenue", 2)
            .unwrap();
        assert_eq!(result, "='pl_2025'!revenue2");
    }

    #[test]
    fn test_formula_without_leading_equals() {
        let mut column_map = HashMap::new();
        column_map.insert("revenue".to_string(), "A".to_string());
        column_map.insert("cogs".to_string(), "B".to_string());

        let translator = FormulaTranslator::new(column_map);

        // Test formula without leading =
        let result = translator
            .translate_row_formula("revenue - cogs", 2)
            .unwrap();
        assert_eq!(result, "=A2 - B2");
    }

    #[test]
    fn test_financial_functions_preserved() {
        let mut column_map = HashMap::new();
        column_map.insert("cashflow".to_string(), "A".to_string());

        let translator = FormulaTranslator::new(column_map);

        // Test that NPV, IRR, XNPV, XIRR are preserved as functions (use literals for other args)
        let result = translator
            .translate_row_formula("=NPV(0.1, cashflow)", 2)
            .unwrap();
        assert!(result.contains("NPV"));
        assert!(result.contains("A2")); // cashflow translated

        // Test XNPV with literals
        let result = translator
            .translate_row_formula("=XNPV(0.1, cashflow, 45000)", 2)
            .unwrap();
        assert!(result.contains("XNPV"));

        // Test PMT with literals
        let result = translator
            .translate_row_formula("=PMT(0.05, 12, 1000)", 2)
            .unwrap();
        assert!(result.contains("PMT"));

        // Test IRR, PV, FV, RATE, NPER
        let result = translator
            .translate_row_formula("=IRR(cashflow)", 2)
            .unwrap();
        assert!(result.contains("IRR"));

        let result = translator
            .translate_row_formula("=PV(0.1, 10, 100)", 2)
            .unwrap();
        assert!(result.contains("PV"));

        let result = translator
            .translate_row_formula("=FV(0.1, 10, 100)", 2)
            .unwrap();
        assert!(result.contains("FV"));
    }

    #[test]
    fn test_date_functions_preserved() {
        let column_map = HashMap::new();
        let translator = FormulaTranslator::new(column_map);

        // Test DATEDIF, EDATE, EOMONTH are preserved (use numeric literals only)
        let result = translator
            .translate_row_formula("=DATEDIF(45000, 45365, 1)", 2)
            .unwrap();
        assert!(result.contains("DATEDIF"));

        let result = translator
            .translate_row_formula("=EDATE(45000, 3)", 2)
            .unwrap();
        assert!(result.contains("EDATE"));

        let result = translator
            .translate_row_formula("=EOMONTH(45000, 1)", 2)
            .unwrap();
        assert!(result.contains("EOMONTH"));
    }

    #[test]
    fn test_other_new_functions_preserved() {
        let column_map = HashMap::new();
        let translator = FormulaTranslator::new(column_map);

        // Test CHOOSE, MAXIFS, MINIFS, POWER, CONCAT (use numeric literals only)
        let result = translator
            .translate_row_formula("=CHOOSE(1, 10, 20, 30)", 2)
            .unwrap();
        assert!(result.contains("CHOOSE"));

        let result = translator.translate_row_formula("=POWER(2, 8)", 2).unwrap();
        assert!(result.contains("POWER"));

        // CONCAT with numbers to avoid string parsing issues
        let result = translator
            .translate_row_formula("=CONCAT(1, 2)", 2)
            .unwrap();
        assert!(result.contains("CONCAT"));

        let result = translator
            .translate_row_formula("=MAXIFS(1, 2, 3)", 2)
            .unwrap();
        assert!(result.contains("MAXIFS"));

        let result = translator
            .translate_row_formula("=MINIFS(1, 2, 3)", 2)
            .unwrap();
        assert!(result.contains("MINIFS"));
    }
}
