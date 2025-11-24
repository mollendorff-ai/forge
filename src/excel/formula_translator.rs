//! Formula translation from YAML to Excel syntax

use crate::error::{ForgeError, ForgeResult};
use std::collections::HashMap;

/// Translates YAML row formulas to Excel cell formulas
pub struct FormulaTranslator {
    /// Maps column names to Excel column letters (revenue → A, cogs → B, etc.)
    #[allow(dead_code)]
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
    /// - Input: `=revenue - cogs`, row 2
    /// - Output: `=A2-B2`
    pub fn translate_row_formula(
        &self,
        _formula: &str,
        _row_num: usize,
    ) -> ForgeResult<String> {
        // TODO: Implement in Phase 3.2
        Err(ForgeError::Export(
            "Formula translation not yet implemented".to_string(),
        ))
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
}
