//! Reverse formula translation - Excel formulas → YAML syntax
//!
//! Converts Excel formulas like "=B2-C2" to YAML formulas like "=revenue - cogs"

use crate::error::ForgeResult;
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::sync::LazyLock;

/// `Sheet!A1`, `'Sheet Name'!A1` or `Sheet!columnName2`: a sheet, a column and a row
static SHEET_REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"('[^']+'|[\w]+)!([\w]+)\d+").expect("the sheet reference pattern is valid")
});
/// `A:A`
static COLUMN_RANGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([A-Z]+):([A-Z]+)\b").expect("the column range pattern is valid")
});
/// `A1:A10`
static CELL_RANGE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([A-Z]+)\d+:([A-Z]+)\d+\b").expect("the cell range pattern is valid")
});
/// `A1`, `B2`, `AA10`
static CELL_REFERENCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b([A-Z]+)(\d+)\b").expect("the cell reference pattern is valid")
});

/// Translates Excel formulas to YAML syntax
pub struct ReverseFormulaTranslator {
    /// Maps Excel column letters to YAML column names (A → revenue, B → cogs)
    column_map: HashMap<String, String>,
}

impl ReverseFormulaTranslator {
    /// Create a new reverse formula translator
    #[must_use]
    pub const fn new(column_map: HashMap<String, String>) -> Self {
        Self { column_map }
    }

    /// Translate an Excel formula to YAML syntax
    ///
    /// Example:
    /// - Input: `=B2-C2`, `column_map`: {A → revenue, B → cogs}
    /// - Output: `=revenue - cogs`
    ///
    /// # Errors
    ///
    /// Returns no error today: every step is a replacement that cannot fail.
    /// The result type is kept so callers need not change.
    pub fn translate(&self, excel_formula: &str) -> ForgeResult<String> {
        // Remove leading = if present
        let formula_body = excel_formula.strip_prefix('=').unwrap_or(excel_formula);

        // 1. Cross-sheet references: Sheet!A1 → table.column
        let result = self.translate_sheet_references(formula_body);
        // 2. Range references: SUM(A:A) → SUM(revenue)
        let result = self.translate_range_references(&result);
        // 3. Cell references: B2 → revenue (without row number)
        let result = self.translate_cell_references(&result);

        Ok(format!("={result}"))
    }

    /// The name of the column in a column letter: the letter itself when the
    /// map does not have it.
    fn column_name<'a>(&'a self, letter: &'a str) -> &'a str {
        self.column_map.get(letter).map_or(letter, String::as_str)
    }

    /// Translate cross-sheet references: Sheet!A1 or Sheet!column2 → table.column
    fn translate_sheet_references(&self, formula: &str) -> String {
        SHEET_REFERENCE
            .replace_all(formula, |captures: &Captures<'_>| {
                let sheet = captures[1].trim_matches('\'');
                let column = &captures[2];
                // A column letter (A, B, AA) is mapped; a column name is used as is
                let column = if column.chars().all(|c| c.is_ascii_uppercase()) {
                    self.column_name(column)
                } else {
                    column
                };
                format!("{}.{column}", Self::sanitize_name(sheet))
            })
            .into_owned()
    }

    /// Translate range references: SUM(A:A) or SUM(A1:A10) → SUM(revenue).
    /// A range from one column to another is left as it is.
    fn translate_range_references(&self, formula: &str) -> String {
        let same_column = |captures: &Captures<'_>| {
            if captures[1] == captures[2] {
                self.column_name(&captures[1]).to_string()
            } else {
                captures[0].to_string()
            }
        };
        let columns = COLUMN_RANGE.replace_all(formula, same_column);
        CELL_RANGE.replace_all(&columns, same_column).into_owned()
    }

    /// Translate cell references: B2 → revenue (remove row numbers)
    fn translate_cell_references(&self, formula: &str) -> String {
        CELL_REFERENCE
            .replace_all(formula, |captures: &Captures<'_>| {
                // A function name with digits (LOG10, ATAN2, DAYS360) is followed by
                // `(`; a cell reference never is. Telling them apart by name wrongly
                // skipped real columns such as E (Euler's constant) in `E2`.
                if formula[captures.get_match().end()..].starts_with('(') {
                    captures[0].to_string()
                } else {
                    self.column_name(&captures[1]).to_string()
                }
            })
            .into_owned()
    }

    /// Sanitize table/sheet name to valid YAML key
    fn sanitize_name(name: &str) -> String {
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

    #[test]
    fn test_column_named_like_a_constant_is_translated() {
        // `E` is Euler's constant, but `E2` is still the cell in column E.
        let mut column_map = HashMap::new();
        column_map.insert("C".to_string(), "profit".to_string());
        column_map.insert("E".to_string(), "revenue".to_string());
        let translator = ReverseFormulaTranslator::new(column_map);
        assert_eq!(translator.translate("=C2/E2").unwrap(), "=profit/revenue");
    }

    #[test]
    fn test_function_with_digits_is_not_a_cell() {
        let mut column_map = HashMap::new();
        column_map.insert("A".to_string(), "revenue".to_string());
        let translator = ReverseFormulaTranslator::new(column_map);
        assert_eq!(
            translator.translate("=LOG10(A2)").unwrap(),
            "=LOG10(revenue)"
        );
    }

    /// A sheet name becomes a table name as the export spells it; a column
    /// name (not a letter) is kept; a letter the map lacks is kept too
    #[test]
    fn test_sheet_references() {
        let translator = create_test_translator();
        for (excel, yaml) in [
            ("=Sheet1!A12", "=sheet1.revenue"),
            ("=Sheet1!A123+B2", "=sheet1.revenue+cogs"),
            (
                "='Q1 Sales & Co-op (final)'!B7",
                "=q1_sales_and_co_op_final.cogs",
            ),
            ("=Sheet1!margin2", "=sheet1.margin"),
            ("=Sheet1!Z2", "=sheet1.Z"),
        ] {
            assert_eq!(translator.translate(excel).unwrap(), yaml, "{excel}");
        }
    }

    /// A range in one column is the column; a range from one column to
    /// another is left as written (but its cells are translated)
    #[test]
    fn test_ranges() {
        let translator = create_test_translator();
        for (excel, yaml) in [
            ("=SUM(A1:A10)", "=SUM(revenue)"),
            ("=SUM(A:A)+SUM(B:B)", "=SUM(revenue)+SUM(cogs)"),
            ("=SUM(A1:B10)", "=SUM(revenue:cogs)"),
            ("=SUM(A:B)", "=SUM(A:B)"),
            ("=SUM(Z:Z)+Z9", "=SUM(Z)+Z"),
            ("=A2:B2", "=revenue:cogs"),
        ] {
            assert_eq!(translator.translate(excel).unwrap(), yaml, "{excel}");
        }
    }
}
