//! Formula translation from YAML to Excel syntax

use crate::error::{ForgeError, ForgeResult};
use regex::Regex;
use std::collections::HashMap;

/// Translates YAML row formulas to Excel cell formulas
pub struct FormulaTranslator {
    /// Maps column names to Excel column letters (revenue → A, cogs → B, etc.)
    column_map: HashMap<String, String>,
    /// Global mapping: `table_name` -> (`column_name` -> `column_letter`)
    table_column_maps: HashMap<String, HashMap<String, String>>,
    /// Global mapping: `table_name` -> `row_count`
    table_row_counts: HashMap<String, usize>,
}

/// The column letters, by their place in the alphabet
const LETTERS: &[u8; 26] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// The function names the translator leaves as they are, in byte order, so
/// that a lookup can bisect. A test keeps the order.
const EXCEL_FUNCTIONS: [&str; 193] = [
    "ABS",
    "ACCRINT",
    "ACOS",
    "ADDRESS",
    "AND",
    "ASIN",
    "ATAN",
    "AVERAGE",
    "AVERAGEIF",
    "AVERAGEIFS",
    "AVG",
    "CEILING",
    "CHOOSE",
    "COLUMN",
    "COLUMNS",
    "CONCAT",
    "CONCATENATE",
    "CORREL",
    "COS",
    "COSH",
    "COUNT",
    "COUNTA",
    "COUNTBLANK",
    "COUNTIF",
    "COUNTIFS",
    "COUNTUNIQUE",
    "COVARIANCE.P",
    "COVARIANCE.S",
    "CUMIPMT",
    "CUMPRINC",
    "DATE",
    "DATEDIF",
    "DAY",
    "DAYS",
    "DB",
    "DDB",
    "DEGREES",
    "E",
    "EDATE",
    "EFFECT",
    "EOMONTH",
    "EXP",
    "FALSE",
    "FILTER",
    "FIND",
    "FLOOR",
    "FORECAST",
    "FORECAST.LINEAR",
    "FV",
    "GEOMEAN",
    "HLOOKUP",
    "HOUR",
    "IF",
    "IFERROR",
    "IFNA",
    "IFS",
    "INDEX",
    "INDIRECT",
    "INT",
    "INTERCEPT",
    "IPMT",
    "IRR",
    "ISBLANK",
    "ISERROR",
    "ISEVEN",
    "ISFORMULA",
    "ISLOGICAL",
    "ISNA",
    "ISNUMBER",
    "ISODD",
    "ISOWEEKNUM",
    "ISREF",
    "ISTEXT",
    "LAMBDA",
    "LARGE",
    "LEFT",
    "LEN",
    "LET",
    "LN",
    "LOG",
    "LOG10",
    "LOWER",
    "MATCH",
    "MAX",
    "MAXIFS",
    "MEDIAN",
    "MID",
    "MIN",
    "MINIFS",
    "MINUTE",
    "MIRR",
    "MOD",
    "MODE",
    "MODE.SNGL",
    "MONTH",
    "MROUND",
    "N",
    "NA",
    "NETWORKDAYS",
    "NOMINAL",
    "NORM.DIST",
    "NORM.INV",
    "NORM.S.DIST",
    "NORM.S.INV",
    "NOT",
    "NOW",
    "NPER",
    "NPV",
    "OFFSET",
    "OR",
    "PERCENTILE",
    "PERCENTILE.EXC",
    "PERCENTILE.INC",
    "PI",
    "PMT",
    "POW",
    "POWER",
    "PPMT",
    "PRICEDISC",
    "PRODUCT",
    "PV",
    "QUARTILE",
    "QUARTILE.EXC",
    "QUARTILE.INC",
    "RADIANS",
    "RAND",
    "RANDARRAY",
    "RANDBETWEEN",
    "RANK",
    "RANK.EQ",
    "RATE",
    "REPLACE",
    "REPT",
    "RIGHT",
    "ROUND",
    "ROUNDDOWN",
    "ROUNDUP",
    "ROW",
    "ROWS",
    "RRI",
    "RSQ",
    "SEARCH",
    "SECOND",
    "SEQUENCE",
    "SIGN",
    "SIN",
    "SINH",
    "SLN",
    "SLOPE",
    "SMALL",
    "SORT",
    "SQRT",
    "STDEV",
    "STDEV.P",
    "STDEV.S",
    "STDEVP",
    "SUBSTITUTE",
    "SUM",
    "SUMIF",
    "SUMIFS",
    "SUMPRODUCT",
    "SWITCH",
    "SYD",
    "TAN",
    "TANH",
    "TEXT",
    "TEXTJOIN",
    "TIME",
    "TODAY",
    "TRIM",
    "TRUE",
    "TRUNC",
    "TYPE",
    "UNIQUE",
    "UPPER",
    "VALUE",
    "VAR",
    "VAR.P",
    "VAR.S",
    "VARP",
    "VDB",
    "VLOOKUP",
    "WEEKDAY",
    "WEEKNUM",
    "WORKDAY",
    "XIRR",
    "XLOOKUP",
    "XMATCH",
    "XNPV",
    "XOR",
    "YEAR",
    "YEARFRAC",
    "YIELDDISC",
];

impl FormulaTranslator {
    /// Create a new formula translator with column mappings (legacy, for backwards compat)
    #[must_use]
    pub fn new(column_map: HashMap<String, String>) -> Self {
        Self {
            column_map,
            table_column_maps: HashMap::new(),
            table_row_counts: HashMap::new(),
        }
    }

    /// Create a new formula translator with full table knowledge
    #[must_use]
    pub const fn new_with_tables(
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
    /// - Input: `=revenue - cogs`, `row_idx` 0, `excel_row` 2
    /// - Output: `=A2-B2`
    ///
    /// # Errors
    ///
    /// Returns an error if a column reference cannot be resolved.
    ///
    /// # Panics
    ///
    /// Panics if a regex capture group 0 is missing (should never occur).
    pub fn translate_row_formula(&self, formula: &str, excel_row: u32) -> ForgeResult<String> {
        // Remove leading = if present
        let formula_body = formula.strip_prefix('=').unwrap_or(formula);

        let mut result = formula_body.to_string();

        // Aggregations first, then other tables, then names without a table, then columns
        self.replace_aggregate_ranges(&mut result)?;
        self.replace_table_ranges(&mut result)?;
        Self::replace_unknown_table_cells(&mut result, excel_row)?;
        self.replace_column_references(&mut result, excel_row)?;

        Ok(format!("={result}"))
    }

    /// Check if a word is an Excel function
    pub(crate) fn is_excel_function(word: &str) -> bool {
        EXCEL_FUNCTIONS
            .binary_search(&word.to_uppercase().as_str())
            .is_ok()
    }

    /// Translate a scalar formula to an Excel formula
    ///
    /// Examples:
    /// - `=SUM(table.column)` → `=SUM(table!A2:A4)`
    /// - `=table.column[0]` → `=table!A2`
    /// - `=scalar_name / 100` → `=B3 / 100`
    ///
    /// # Errors
    ///
    /// Returns an error if a regex compilation fails.
    ///
    /// # Panics
    ///
    /// Panics if a regex capture group 0 is missing (should never occur).
    pub fn translate_scalar_formula(
        &self,
        formula: &str,
        scalar_row_map: &HashMap<String, u32>,
    ) -> ForgeResult<String> {
        // Remove leading = if present
        let formula_body = formula.strip_prefix('=').unwrap_or(formula);

        let mut result = formula_body.to_string();

        // Indexed references first, then ranges, then single cells, then scalars
        self.replace_indexed_cells(&mut result)?;
        self.replace_aggregate_ranges(&mut result)?;
        self.replace_table_ranges(&mut result)?;
        self.replace_first_row_cells(&mut result, scalar_row_map)?;
        Self::replace_scalar_cells(&mut result, scalar_row_map)?;

        Ok(format!("={result}"))
    }

    /// Rewrite `SUM(table.column)` and the like as a range: `SUM('table'!A2:A4)`.
    fn replace_aggregate_ranges(&self, result: &mut String) -> ForgeResult<()> {
        // FIRST: Handle table.column references in aggregation functions that need ranges
        // Pattern for simple aggregations: SUM(table.column) → SUM(table!A2:A4)
        let agg_pattern = Regex::new(r"(SUM|AVERAGE|MAX|MIN|COUNT|COUNTA|PRODUCT)\(([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)\)")
            .map_err(|e| ForgeError::Export(format!("Regex error: {e}")))?;

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
                let end_row = row_count + 1;
                let replacement =
                    format!("{func_name}('{table_name}'!{col_letter}2:{col_letter}{end_row})");
                (full_match.range(), replacement)
            })
            .collect();

        for (range, replacement) in agg_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        Ok(())
    }

    /// Rewrite any other `table.column` of a known table as a range, for `SUMIFS`,
    /// `PERCENTILE`, `RANK`, `CORREL` and the rest that the aggregate pass leaves.
    fn replace_table_ranges(&self, result: &mut String) -> ForgeResult<()> {
        // SECOND: Handle general table.column references that need ranges (for functions like SUMIFS, PERCENTILE, etc.)
        let general_table_pattern =
            Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)\b")
                .map_err(|e| ForgeError::Export(format!("Regex error: {e}")))?;

        let general_replacements: Vec<(std::ops::Range<usize>, String)> = general_table_pattern
            .captures_iter(&result.clone())
            .filter_map(|cap| {
                let full_match = cap.get(0).unwrap();
                let table_name = &cap[1];
                let col_name = &cap[2];

                // Skip if already processed (would contain !)
                if result[..full_match.start()].ends_with('\'')
                    || result[full_match.end()..].starts_with('!')
                {
                    return None;
                }

                // Only convert if this is a known table
                if !self.table_column_maps.contains_key(table_name) {
                    return None;
                }

                let col_letter = self
                    .table_column_maps
                    .get(table_name)
                    .and_then(|cols| cols.get(col_name))
                    .cloned()
                    .unwrap_or_else(|| col_name.to_string());

                let row_count = self.table_row_counts.get(table_name).copied().unwrap_or(1);
                let end_row = row_count + 1;
                let replacement = format!("'{table_name}'!{col_letter}2:{col_letter}{end_row}");
                Some((full_match.range(), replacement))
            })
            .collect();

        for (range, replacement) in general_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        Ok(())
    }

    /// Rewrite a `table.column` of no known table as a cell of this row.
    fn replace_unknown_table_cells(result: &mut String, excel_row: u32) -> ForgeResult<()> {
        // THIRD: Handle remaining table.column references (not in known tables, use fallback)
        let remaining_table_pattern =
            Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)")
                .map_err(|e| ForgeError::Export(format!("Regex error: {e}")))?;

        let remaining_replacements: Vec<(std::ops::Range<usize>, String)> = remaining_table_pattern
            .captures_iter(&result.clone())
            .filter_map(|cap| {
                let full_match = cap.get(0).unwrap();
                let table_name = &cap[1];
                let col_name = &cap[2];

                // Skip if already processed
                if result[..full_match.start()].ends_with('\'')
                    || result[full_match.end()..].starts_with('!')
                {
                    return None;
                }

                // Fallback: convert to single cell reference with table name and column name
                let replacement = format!("'{table_name}'!{col_name}{excel_row}");
                Some((full_match.range(), replacement))
            })
            .collect();

        for (range, replacement) in remaining_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        Ok(())
    }

    /// Rewrite each name of this table as the cell of its column on this row; a
    /// name that is neither a column, a function, a cell nor a number is an error.
    fn replace_column_references(&self, result: &mut String, excel_row: u32) -> ForgeResult<()> {
        // FOURTH: Handle simple column references in the current table
        let var_pattern = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\b")
            .map_err(|e| ForgeError::Export(format!("Regex error: {e}")))?;

        let result_clone = result.clone();
        let matches: Vec<_> = var_pattern.find_iter(&result_clone).collect();

        for match_obj in matches.iter().rev() {
            let var_name = match_obj.as_str();

            // Skip Excel functions
            if Self::is_excel_function(var_name) {
                continue;
            }

            // Skip if already contains ! (already processed as table reference)
            let start = match_obj.start();
            let end = match_obj.end();
            if start > 0 && result[..start].ends_with('!') {
                continue;
            }

            // Skip if inside single quotes (already processed as sheet name)
            if start > 0 && result[..start].ends_with('\'') {
                continue;
            }
            if end < result.len() && result[end..].starts_with('!') {
                continue;
            }

            // Simple column reference in current table
            if let Some(col_letter) = self.column_map.get(var_name) {
                let excel_ref = format!("{col_letter}{excel_row}");
                result.replace_range(match_obj.range(), &excel_ref);
            } else {
                // Check if this looks like an Excel cell reference (e.g., A6, B2)
                let looks_like_cell_ref = var_name.len() >= 2
                    && var_name.chars().all(char::is_alphanumeric)
                    && var_name.chars().any(char::is_alphabetic)
                    && var_name.chars().any(char::is_numeric);

                // Check if it's a number
                let is_number = var_name.parse::<f64>().is_ok();

                if !looks_like_cell_ref && !is_number {
                    // Column not found and it's not a cell reference or number - this is an error
                    return Err(ForgeError::Export(format!(
                        "Column '{var_name}' not found in table"
                    )));
                }
                // If it's a cell reference or number, leave it as is
            }
        }

        Ok(())
    }

    /// Rewrite `table.column[index]` as one cell of the table: `table!A2`.
    fn replace_indexed_cells(&self, result: &mut String) -> ForgeResult<()> {
        // Pattern for table.column[index] references (must be processed first)
        let indexed_pattern =
            Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)\[(\d+)\]")
                .map_err(|e| ForgeError::Export(format!("Regex error: {e}")))?;

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
                let replacement = format!("'{table_name}'!{col_letter}{excel_row}");
                (full_match.range(), replacement)
            })
            .collect();

        // Apply indexed replacements in reverse order
        for (range, replacement) in indexed_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        Ok(())
    }

    /// Rewrite a `table.column` that is none of the above as the first data row of
    /// the table, or as a scalar's cell when the name is a scalar.
    fn replace_first_row_cells(
        &self,
        result: &mut String,
        scalar_row_map: &HashMap<String, u32>,
    ) -> ForgeResult<()> {
        // Pattern for simple table.column references (not in aggregations, not indexed)
        // These become references to row 2 (first data row)
        let simple_table_pattern =
            Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)\.([a-zA-Z_][a-zA-Z0-9_]*)")
                .map_err(|e| ForgeError::Export(format!("Regex error: {e}")))?;

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
                    let scalar_name = format!("{table_name}.{col_name}");
                    if let Some(&row) = scalar_row_map.get(&scalar_name) {
                        return Some((full_match.range(), format!("B{row}")));
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
                let replacement = format!("'{table_name}'!{col_letter}2");
                Some((full_match.range(), replacement))
            })
            .collect();

        // Apply simple replacements in reverse order
        for (range, replacement) in simple_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        Ok(())
    }

    /// Rewrite a scalar's name, `metrics.total_savings`, as its cell: `B3`.
    fn replace_scalar_cells(
        result: &mut String,
        scalar_row_map: &HashMap<String, u32>,
    ) -> ForgeResult<()> {
        // Handle scalar-to-scalar references (e.g., metrics.total_savings → B3)
        // Pattern for standalone scalar names
        let scalar_pattern = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*\.[a-zA-Z_][a-zA-Z0-9_]*)\b")
            .map_err(|e| ForgeError::Export(format!("Regex error: {e}")))?;

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
                    return Some((full_match.range(), format!("B{row}")));
                }
                None
            })
            .collect();

        // Apply scalar replacements in reverse order
        for (range, replacement) in scalar_replacements.into_iter().rev() {
            result.replace_range(range, &replacement);
        }

        Ok(())
    }

    /// Convert a column name to an Excel column letter
    ///
    /// Examples:
    /// - 0 → A
    /// - 1 → B
    /// - 25 → Z
    /// - 26 → AA
    #[must_use]
    pub fn column_index_to_letter(index: usize) -> String {
        let mut result = String::new();
        let mut idx = index;

        loop {
            result.insert(0, char::from(LETTERS[idx % 26]));
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

    // ═══════════════════════════════════════════════════════════════════════════
    // Additional coverage tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_new_with_tables() {
        let column_map = HashMap::new();
        let mut table_column_maps = HashMap::new();
        let mut sales_cols = HashMap::new();
        sales_cols.insert("revenue".to_string(), "A".to_string());
        sales_cols.insert("cost".to_string(), "B".to_string());
        table_column_maps.insert("sales".to_string(), sales_cols);

        let mut table_row_counts = HashMap::new();
        table_row_counts.insert("sales".to_string(), 5);

        let translator =
            FormulaTranslator::new_with_tables(column_map, table_column_maps, table_row_counts);

        // Test cross-table reference with full knowledge
        let result = translator
            .translate_row_formula("=sales.revenue", 2)
            .unwrap();
        assert!(result.contains("sales"));
    }

    #[test]
    fn test_translate_scalar_formula_basic() {
        let mut table_column_maps = HashMap::new();
        let mut sales_cols = HashMap::new();
        sales_cols.insert("amount".to_string(), "A".to_string());
        table_column_maps.insert("sales".to_string(), sales_cols);

        let mut table_row_counts = HashMap::new();
        table_row_counts.insert("sales".to_string(), 3);

        let translator =
            FormulaTranslator::new_with_tables(HashMap::new(), table_column_maps, table_row_counts);

        let scalar_row_map = HashMap::new();

        // Test SUM aggregation
        let result = translator
            .translate_scalar_formula("=SUM(sales.amount)", &scalar_row_map)
            .unwrap();
        assert!(result.contains("SUM"));
        assert!(result.contains("sales"));
    }

    #[test]
    fn test_translate_scalar_formula_with_index() {
        let mut table_column_maps = HashMap::new();
        let mut data_cols = HashMap::new();
        data_cols.insert("value".to_string(), "C".to_string());
        table_column_maps.insert("data".to_string(), data_cols);

        let mut table_row_counts = HashMap::new();
        table_row_counts.insert("data".to_string(), 10);

        let translator =
            FormulaTranslator::new_with_tables(HashMap::new(), table_column_maps, table_row_counts);

        let scalar_row_map = HashMap::new();

        // Test indexed reference: data.value[0]
        let result = translator
            .translate_scalar_formula("=data.value[0]", &scalar_row_map)
            .unwrap();
        assert!(result.contains("data"));
        assert!(result.contains("C2")); // index 0 = row 2
    }

    #[test]
    fn test_translate_scalar_formula_with_scalar_ref() {
        let translator = FormulaTranslator::new(HashMap::new());

        let mut scalar_row_map = HashMap::new();
        scalar_row_map.insert("metrics.total".to_string(), 5);

        // Test scalar-to-scalar reference
        let result = translator
            .translate_scalar_formula("=metrics.total * 2", &scalar_row_map)
            .unwrap();
        assert!(result.contains("B5")); // scalar at row 5
    }

    #[test]
    fn test_column_not_found_error() {
        let column_map = HashMap::new(); // Empty
        let translator = FormulaTranslator::new(column_map);

        // Should error on unknown column
        let result = translator.translate_row_formula("=unknown_column + 1", 2);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"));
    }

    #[test]
    fn every_listed_excel_function_is_known_to_the_export() {
        // A function in the list (`forge functions`) that the export does not
        // know would be read as a column name and stop the export. Forge's own
        // functions and the Monte Carlo ones have no Excel name and are left out.
        use crate::functions::registry::{Category, FUNCTIONS};
        let own = [
            "SCENARIO",
            "PREV",
            "VARIANCE",
            "VARIANCE_PCT",
            "VARIANCE_STATUS",
        ];
        let missing: Vec<&str> = FUNCTIONS
            .iter()
            .filter(|f| !matches!(f.category, Category::MonteCarlo | Category::ForgeNative))
            .map(|f| f.name)
            .filter(|n| !own.contains(n))
            .filter(|n| !FormulaTranslator::is_excel_function(n))
            .collect();
        assert!(missing.is_empty(), "not known to the export: {missing:?}");
    }

    #[test]
    fn test_more_excel_functions_preserved() {
        let column_map = HashMap::new();
        let translator = FormulaTranslator::new(column_map);

        // Test more functions
        let functions = [
            "ABS", "ROUND", "CEILING", "FLOOR", "MOD", "SQRT", "EXP", "LN", "LOG", "LOG10",
        ];

        for func in functions {
            let formula = format!("={func}(10)");
            let result = translator.translate_row_formula(&formula, 2).unwrap();
            assert!(
                result.to_uppercase().contains(func),
                "Function {func} should be preserved"
            );
        }
    }

    #[test]
    fn test_text_functions_preserved() {
        let column_map = HashMap::new();
        let translator = FormulaTranslator::new(column_map);

        let functions = ["UPPER", "LOWER", "TRIM", "LEN", "LEFT", "RIGHT", "MID"];

        for func in functions {
            let formula = format!("={func}(1)"); // Using number to avoid string parsing
            let result = translator.translate_row_formula(&formula, 2).unwrap();
            assert!(
                result.to_uppercase().contains(func),
                "Function {func} should be preserved"
            );
        }
    }

    #[test]
    fn test_logical_functions_preserved() {
        let column_map = HashMap::new();
        let translator = FormulaTranslator::new(column_map);

        let functions = ["IF", "AND", "OR", "NOT", "TRUE", "FALSE", "IFERROR"];

        for func in functions {
            let formula = format!("={func}(1, 2, 3)");
            let result = translator.translate_row_formula(&formula, 2).unwrap();
            assert!(
                result.to_uppercase().contains(func),
                "Function {func} should be preserved"
            );
        }
    }

    #[test]
    fn test_column_index_to_letter_extended() {
        // Test more column letters
        assert_eq!(FormulaTranslator::column_index_to_letter(702), "AAA");
        assert_eq!(FormulaTranslator::column_index_to_letter(52), "BA");
    }

    /// The list is sorted and has no repeats, or the bisection would miss a name
    #[test]
    fn the_excel_function_names_are_sorted_and_distinct() {
        assert!(EXCEL_FUNCTIONS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn test_is_excel_function() {
        // Test various function names
        assert!(FormulaTranslator::is_excel_function("SUM"));
        assert!(FormulaTranslator::is_excel_function("sum")); // lowercase
        assert!(FormulaTranslator::is_excel_function("AVERAGE"));
        assert!(FormulaTranslator::is_excel_function("NPV"));
        assert!(FormulaTranslator::is_excel_function("XLOOKUP"));
        assert!(!FormulaTranslator::is_excel_function("revenue")); // not a function
        assert!(!FormulaTranslator::is_excel_function("my_column")); // not a function
    }

    #[test]
    fn test_translate_sumifs_formula() {
        let mut table_column_maps = HashMap::new();
        let mut sales_cols = HashMap::new();
        sales_cols.insert("amount".to_string(), "A".to_string());
        sales_cols.insert("region".to_string(), "B".to_string());
        sales_cols.insert("product".to_string(), "C".to_string());
        table_column_maps.insert("sales_data".to_string(), sales_cols);

        let mut table_row_counts = HashMap::new();
        table_row_counts.insert("sales_data".to_string(), 6);

        let translator =
            FormulaTranslator::new_with_tables(HashMap::new(), table_column_maps, table_row_counts);

        let scalar_row_map = HashMap::new();

        // Test SUMIFS with multiple criteria
        let result = translator
            .translate_scalar_formula(
                "=SUMIFS(sales_data.amount, sales_data.region, 1, sales_data.product, 1)",
                &scalar_row_map,
            )
            .unwrap();

        println!("SUMIFS result: {result}");
        assert!(result.contains("'sales_data'!A2:A7"));
        assert!(result.contains("'sales_data'!B2:B7"));
        assert!(result.contains("'sales_data'!C2:C7"));
    }

    #[test]
    fn test_translate_percentile_formula() {
        let mut table_column_maps = HashMap::new();
        let mut dataset_cols = HashMap::new();
        dataset_cols.insert("values".to_string(), "A".to_string());
        table_column_maps.insert("dataset".to_string(), dataset_cols);

        let mut table_row_counts = HashMap::new();
        table_row_counts.insert("dataset".to_string(), 10);

        let translator =
            FormulaTranslator::new_with_tables(HashMap::new(), table_column_maps, table_row_counts);

        let scalar_row_map = HashMap::new();

        // Test PERCENTILE
        let result = translator
            .translate_scalar_formula("=PERCENTILE(dataset.values, 0.5)", &scalar_row_map)
            .unwrap();

        println!("PERCENTILE result: {result}");
        assert!(result.contains("'dataset'!A2:A11"));
    }

    #[test]
    fn test_translate_correl_formula() {
        let mut table_column_maps = HashMap::new();
        let mut data_cols = HashMap::new();
        data_cols.insert("advertising".to_string(), "A".to_string());
        data_cols.insert("sales".to_string(), "B".to_string());
        table_column_maps.insert("data_series".to_string(), data_cols);

        let mut table_row_counts = HashMap::new();
        table_row_counts.insert("data_series".to_string(), 5);

        let translator =
            FormulaTranslator::new_with_tables(HashMap::new(), table_column_maps, table_row_counts);

        let scalar_row_map = HashMap::new();

        // Test CORREL
        let result = translator
            .translate_scalar_formula(
                "=CORREL(data_series.advertising, data_series.sales)",
                &scalar_row_map,
            )
            .unwrap();

        println!("CORREL result: {result}");
        assert!(result.contains("'data_series'!A2:A6"));
        assert!(result.contains("'data_series'!B2:B6"));
    }
}
