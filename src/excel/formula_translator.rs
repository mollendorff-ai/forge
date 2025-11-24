//! Formula translation from YAML to Excel syntax

use crate::error::{ForgeError, ForgeResult};
use regex::Regex;
use std::collections::HashMap;

/// Translates YAML row formulas to Excel cell formulas
pub struct FormulaTranslator {
    /// Maps column names to Excel column letters (revenue → A, cogs → B, etc.)
    column_map: HashMap<String, String>,
}

impl FormulaTranslator {
    /// Create a new formula translator with column mappings
    pub fn new(column_map: HashMap<String, String>) -> Self {
        Self { column_map }
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
                // Math functions
                | "ABS"
                | "ROUND"
                | "ROUNDUP"
                | "ROUNDDOWN"
                | "SQRT"
                | "POW"
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
                // Lookup functions
                | "VLOOKUP"
                | "HLOOKUP"
                | "XLOOKUP"
                | "INDEX"
                | "MATCH"
        )
    }

    /// Translate table.column reference to Excel sheet reference
    ///
    /// Example:
    /// - Input: `pl_2025.revenue`, row 2
    /// - Output: `pl_2025!A2`
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

        // For cross-table references, we need to look up the column in the target table
        // For now, assume alphabetical column ordering (matching export_table logic)
        // TODO: This should be passed in or computed from the target table
        // For MVP, we'll just use the column name as-is and let Excel handle it

        Ok(format!("{}!{}{}", table_name, col_name, excel_row))
    }

    /// Translate a scalar formula to an Excel formula
    ///
    /// Example:
    /// - Input: `=SUM(pl_2025.revenue)`
    /// - Output: `=SUM(pl_2025!A:A)`
    pub fn translate_scalar_formula(&self, _formula: &str) -> ForgeResult<String> {
        // TODO: Implement in Phase 3.4
        Err(ForgeError::Export(
            "Scalar formula translation not yet implemented".to_string(),
        ))
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
        assert_eq!(result, "=pl_2025!revenue2");
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
}
