//! Reverse formula translation - Excel formulas → YAML syntax
//!
//! Converts Excel formulas like "=B2-C2" to YAML formulas like "=revenue - cogs"

use crate::error::{ForgeError, ForgeResult};
use regex::Regex;
use std::collections::HashMap;

/// Translates Excel formulas to YAML syntax
pub struct ReverseFormulaTranslator {
    /// Maps Excel column letters to YAML column names (A → revenue, B → cogs)
    column_map: HashMap<String, String>,
}

impl ReverseFormulaTranslator {
    /// Create a new reverse formula translator
    pub fn new(column_map: HashMap<String, String>) -> Self {
        Self { column_map }
    }

    /// Translate an Excel formula to YAML syntax
    ///
    /// Example:
    /// - Input: `=B2-C2`, column_map: {A → revenue, B → cogs}
    /// - Output: `=revenue - cogs`
    pub fn translate(&self, excel_formula: &str) -> ForgeResult<String> {
        // Remove leading = if present
        let formula_body = excel_formula.strip_prefix('=').unwrap_or(excel_formula);

        // Handle different formula patterns
        let result = self.translate_formula_body(formula_body)?;

        Ok(format!("={result}"))
    }

    /// Translate formula body (without leading =)
    fn translate_formula_body(&self, formula: &str) -> ForgeResult<String> {
        let mut result = formula.to_string();

        // 1. Handle cross-sheet references: Sheet!A1 → table.column
        result = self.translate_sheet_references(&result)?;

        // 2. Handle range references: SUM(A:A) → SUM(revenue)
        result = self.translate_range_references(&result)?;

        // 3. Handle cell references: B2 → revenue (without row number)
        result = self.translate_cell_references(&result)?;

        Ok(result)
    }

    /// Translate cross-sheet references: Sheet!A1 or Sheet!column2 → table.column
    fn translate_sheet_references(&self, formula: &str) -> ForgeResult<String> {
        // Pattern: SheetName!A1 or 'Sheet Name'!A1 or Sheet!columnName2 (with row number)
        let sheet_ref_pattern = Regex::new(r"('[^']+'|[\w]+)!([\w]+)\d+")
            .map_err(|e| ForgeError::Import(format!("Regex error: {e}")))?;

        let mut result = formula.to_string();

        // Find all matches in reverse order
        let matches: Vec<_> = sheet_ref_pattern.find_iter(formula).collect();

        for match_obj in matches.iter().rev() {
            let full_match = match_obj.as_str();

            // Parse sheet name and column
            if let Some(captures) = sheet_ref_pattern.captures(full_match) {
                let sheet_name = captures.get(1).unwrap().as_str();
                let col_ref = captures.get(2).unwrap().as_str();

                // Remove quotes from sheet name if present
                let clean_sheet = sheet_name.trim_matches('\'');

                // Sanitize sheet name (same as export logic)
                let table_name = self.sanitize_name(clean_sheet);

                // Check if col_ref is a column letter (A, B, AA) or column name
                let col_name = if col_ref.chars().all(|c| c.is_ascii_uppercase()) {
                    // It's a column letter - map it
                    self.column_map
                        .get(col_ref)
                        .map_or(col_ref, std::string::String::as_str)
                } else {
                    // It's already a column name - use as is
                    col_ref
                };

                // Replace with table.column
                let yaml_ref = format!("{table_name}.{col_name}");
                result.replace_range(match_obj.range(), &yaml_ref);
            }
        }

        Ok(result)
    }

    /// Translate range references: SUM(A:A) → SUM(revenue)
    fn translate_range_references(&self, formula: &str) -> ForgeResult<String> {
        // Pattern: A:A (column range) or A1:A10 (cell range with same column)
        // Note: Rust regex doesn't support backreferences, so we match generally and validate in code
        let range_pattern = Regex::new(r"\b([A-Z]+):([A-Z]+)\b|\b([A-Z]+)(\d+):([A-Z]+)(\d+)\b")
            .map_err(|e| ForgeError::Import(format!("Regex error: {e}")))?;

        let mut result = formula.to_string();

        // Find all matches in reverse order
        let matches: Vec<_> = range_pattern.find_iter(formula).collect();

        for match_obj in matches.iter().rev() {
            let full_match = match_obj.as_str();

            if let Some(captures) = range_pattern.captures(full_match) {
                // Check if it's a column range (A:A) or cell range (A1:A10)
                let col_letter = if let Some(col1) = captures.get(1) {
                    // Column range: A:A
                    let col1_str = col1.as_str();
                    let col2_str = captures.get(2).unwrap().as_str();

                    // Verify both sides are the same column
                    if col1_str != col2_str {
                        continue; // Skip if different columns (not a valid same-column range)
                    }
                    col1_str
                } else if let Some(col1) = captures.get(3) {
                    // Cell range: A1:A10
                    let col1_str = col1.as_str();
                    let col2_str = captures.get(5).unwrap().as_str();

                    // Verify both sides are the same column
                    if col1_str != col2_str {
                        continue; // Skip if different columns (not a valid same-column range)
                    }
                    col1_str
                } else {
                    continue;
                };

                // Map to column name
                let col_name = self
                    .column_map
                    .get(col_letter)
                    .map_or(col_letter, std::string::String::as_str);

                result.replace_range(match_obj.range(), col_name);
            }
        }

        Ok(result)
    }

    /// Translate cell references: B2 → revenue (remove row numbers)
    fn translate_cell_references(&self, formula: &str) -> ForgeResult<String> {
        // Pattern: Column letter followed by row number (A1, B2, AA10, etc.)
        let cell_ref_pattern = Regex::new(r"\b([A-Z]+)(\d+)\b")
            .map_err(|e| ForgeError::Import(format!("Regex error: {e}")))?;

        let mut result = formula.to_string();

        // Find all matches in reverse order
        let matches: Vec<_> = cell_ref_pattern.find_iter(formula).collect();

        for match_obj in matches.iter().rev() {
            if let Some(captures) = cell_ref_pattern.captures(match_obj.as_str()) {
                let col_letter = captures.get(1).unwrap().as_str();

                // Skip if it's an Excel function (like IF, AND, OR, MAX, etc.)
                if self.is_excel_function(col_letter) {
                    continue;
                }

                // Map to column name
                let col_name = self
                    .column_map
                    .get(col_letter)
                    .map_or(col_letter, std::string::String::as_str);

                result.replace_range(match_obj.range(), col_name);
            }
        }

        Ok(result)
    }

    /// Check if a word is an Excel function (don't translate these!)
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

    /// Sanitize table/sheet name to valid YAML key
    fn sanitize_name(&self, name: &str) -> String {
        name.to_lowercase()
            .replace(' ', "_")
            .replace('&', "and")
            .replace('-', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_translator() -> ReverseFormulaTranslator {
        let mut column_map = HashMap::new();
        column_map.insert("A".to_string(), "revenue".to_string());
        column_map.insert("B".to_string(), "cogs".to_string());
        column_map.insert("C".to_string(), "gross_profit".to_string());
        ReverseFormulaTranslator::new(column_map)
    }

    #[test]
    fn test_simple_cell_reference() {
        let translator = create_test_translator();
        let result = translator.translate("=B2-A2").unwrap();
        assert_eq!(result, "=cogs-revenue");
    }

    #[test]
    fn test_multiple_cell_references() {
        let translator = create_test_translator();
        let result = translator.translate("=A2+B2+C2").unwrap();
        assert_eq!(result, "=revenue+cogs+gross_profit");
    }

    #[test]
    fn test_range_reference() {
        let translator = create_test_translator();
        let result = translator.translate("=SUM(A:A)").unwrap();
        assert_eq!(result, "=SUM(revenue)");
    }

    #[test]
    fn test_cross_sheet_reference() {
        let translator = create_test_translator();
        let result = translator.translate("=Sheet1!A2").unwrap();
        assert_eq!(result, "=sheet1.revenue");
    }

    #[test]
    fn test_if_function() {
        let translator = create_test_translator();
        let result = translator.translate("=IF(A2>0,B2,C2)").unwrap();
        assert_eq!(result, "=IF(revenue>0,cogs,gross_profit)");
    }
}
