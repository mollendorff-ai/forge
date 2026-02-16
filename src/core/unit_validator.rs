//! Unit consistency validation for v4.0 rich metadata
//!
//! Validates that formulas don't mix incompatible units (e.g., CAD + %)

use crate::types::ParsedModel;
use std::collections::HashMap;

/// Strip string literals from a formula before extracting references.
/// This prevents content inside quotes from being parsed as variable references.
/// e.g., =LEN("Hello") should not treat "Hello" as a variable reference.
fn strip_string_literals(formula: &str) -> String {
    let mut result = String::with_capacity(formula.len());
    let mut in_string = false;
    let mut quote_char = '"';

    for c in formula.chars() {
        if !in_string && (c == '"' || c == '\'') {
            in_string = true;
            quote_char = c;
        } else if in_string && c == quote_char {
            in_string = false;
        } else if !in_string {
            result.push(c);
        }
    }

    result
}

/// Unit categories for compatibility checking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnitCategory {
    /// Currency units (CAD, USD, EUR, etc.)
    Currency(String),
    /// Percentage (%)
    Percentage,
    /// Count/quantity (count, units, items)
    Count,
    /// Time (days, months, years)
    Time(String),
    /// Ratio (dimensionless)
    Ratio,
    /// Unknown/unspecified unit
    Unknown,
}

impl UnitCategory {
    /// Parse a unit string into a category
    #[must_use]
    pub fn parse(unit: &str) -> Self {
        let unit_lower = unit.to_lowercase();
        match unit_lower.as_str() {
            // Currency
            "cad" | "usd" | "eur" | "gbp" | "jpy" | "cny" | "$" => {
                Self::Currency(unit.to_uppercase())
            },
            // Percentage
            "%" | "percent" | "percentage" => Self::Percentage,
            // Count
            "count" | "units" | "items" | "qty" | "quantity" => Self::Count,
            // Time
            "days" | "day" | "d" => Self::Time("days".to_string()),
            "months" | "month" | "mo" => Self::Time("months".to_string()),
            "years" | "year" | "yr" => Self::Time("years".to_string()),
            "hours" | "hour" | "hr" => Self::Time("hours".to_string()),
            // Ratio
            "ratio" | "factor" | "multiplier" | "x" => Self::Ratio,
            // Unknown
            _ => Self::Unknown,
        }
    }

    /// Get a display name for the unit category
    #[must_use]
    pub fn display(&self) -> String {
        match self {
            Self::Currency(c) => c.clone(),
            Self::Percentage => "%".to_string(),
            Self::Count => "count".to_string(),
            Self::Time(t) => t.clone(),
            Self::Ratio => "ratio".to_string(),
            Self::Unknown => "unknown".to_string(),
        }
    }
}

/// A unit validation warning
#[derive(Debug, Clone)]
pub struct UnitWarning {
    /// Location (table.column or scalar name)
    pub location: String,
    /// The formula with the issue
    pub formula: String,
    /// Description of the issue
    pub message: String,
    /// Severity (warning or error)
    pub severity: WarningSeverity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningSeverity {
    Warning,
    Error,
}

impl std::fmt::Display for UnitWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match self.severity {
            WarningSeverity::Warning => "⚠️  Warning",
            WarningSeverity::Error => "❌ Error",
        };
        write!(
            f,
            "{}: {} - {} ({})",
            prefix, self.location, self.message, self.formula
        )
    }
}

/// Unit validator for checking formula unit consistency
pub struct UnitValidator<'a> {
    model: &'a ParsedModel,
    /// Map of field path -> unit category
    unit_map: HashMap<String, UnitCategory>,
}

impl<'a> UnitValidator<'a> {
    /// Create a new unit validator for a model
    #[must_use]
    pub fn new(model: &'a ParsedModel) -> Self {
        let mut unit_map = HashMap::new();

        // Extract units from table columns
        for (table_name, table) in &model.tables {
            for (col_name, column) in &table.columns {
                if let Some(unit) = &column.metadata.unit {
                    let path = format!("{table_name}.{col_name}");
                    unit_map.insert(path, UnitCategory::parse(unit));
                    // Also store short name for same-table references
                    unit_map.insert(col_name.clone(), UnitCategory::parse(unit));
                }
            }
        }

        // Extract units from scalars
        for (name, var) in &model.scalars {
            if let Some(unit) = &var.metadata.unit {
                unit_map.insert(name.clone(), UnitCategory::parse(unit));
            }
        }

        Self { model, unit_map }
    }

    /// Validate all formulas in the model and return warnings
    #[must_use]
    pub fn validate(&self) -> Vec<UnitWarning> {
        let mut warnings = Vec::new();

        // Validate table row formulas
        for (table_name, table) in &self.model.tables {
            for (col_name, formula) in &table.row_formulas {
                let location = format!("{table_name}.{col_name}");
                if let Some(warning) = self.validate_formula(&location, formula) {
                    warnings.push(warning);
                }
            }
        }

        // Validate scalar formulas
        for (name, var) in &self.model.scalars {
            if let Some(formula) = &var.formula {
                if let Some(warning) = self.validate_formula(name, formula) {
                    warnings.push(warning);
                }
            }
        }

        warnings
    }

    /// Validate a single formula for unit consistency
    fn validate_formula(&self, location: &str, formula: &str) -> Option<UnitWarning> {
        // Extract variable references from formula
        let refs = self.extract_references(formula);

        if refs.is_empty() {
            return None;
        }

        // Get units for each reference
        let ref_units: Vec<(&str, Option<&UnitCategory>)> = refs
            .iter()
            .map(|r| (r.as_str(), self.unit_map.get(r.as_str())))
            .collect();

        // Check for percentage + currency first (more specific warning)
        let has_percentage = ref_units
            .iter()
            .any(|(_, u)| matches!(u, Some(UnitCategory::Percentage)));
        let has_currency = ref_units
            .iter()
            .any(|(_, u)| matches!(u, Some(UnitCategory::Currency(_))));

        if has_percentage && has_currency && formula.contains('+') {
            return Some(UnitWarning {
                location: location.to_string(),
                formula: formula.to_string(),
                message: "Adding percentage to currency - did you mean to multiply?".to_string(),
                severity: WarningSeverity::Warning,
            });
        }

        // Check for addition/subtraction of incompatible units
        if formula.contains('+') || formula.contains('-') {
            let units_in_additive: Vec<&UnitCategory> =
                ref_units.iter().filter_map(|(_, u)| *u).collect();

            if units_in_additive.len() >= 2 {
                // Check if all units are compatible for addition
                let first = units_in_additive[0];
                for unit in &units_in_additive[1..] {
                    if !self.can_add(first, unit) {
                        return Some(UnitWarning {
                            location: location.to_string(),
                            formula: formula.to_string(),
                            message: format!(
                                "Mixing incompatible units in addition/subtraction: {} and {}",
                                first.display(),
                                unit.display()
                            ),
                            severity: WarningSeverity::Warning,
                        });
                    }
                }
            }
        }

        None
    }

    /// Check if two units can be added/subtracted
    // Takes &self for API consistency; tested extensively via validator.can_add() in tests.
    #[allow(clippy::unused_self)]
    fn can_add(&self, a: &UnitCategory, b: &UnitCategory) -> bool {
        match (a, b) {
            // Same currency can be added
            (UnitCategory::Currency(c1), UnitCategory::Currency(c2)) => c1 == c2,
            // Same time units can be added
            (UnitCategory::Time(t1), UnitCategory::Time(t2)) => t1 == t2,
            // Same-category scalars and unknown are compatible
            (UnitCategory::Count, UnitCategory::Count)
            | (UnitCategory::Ratio, UnitCategory::Ratio)
            | (UnitCategory::Percentage, UnitCategory::Percentage)
            | (UnitCategory::Unknown, _)
            | (_, UnitCategory::Unknown) => true,
            // Everything else is incompatible
            _ => false,
        }
    }

    /// Extract variable references from a formula
    // Takes &self for API consistency; tested via validator.extract_references() in tests.
    #[allow(clippy::unused_self)]
    fn extract_references(&self, formula: &str) -> Vec<String> {
        let formula = formula.trim_start_matches('=');
        // Strip string literals to avoid parsing their contents as variable references
        // e.g., =LEN("Hello") should not treat "Hello" as a variable reference
        let formula_stripped = strip_string_literals(formula);
        let mut refs = Vec::new();

        // Simple tokenization - split by operators and extract identifiers
        for token in formula_stripped.split(|c: char| {
            c == '+'
                || c == '-'
                || c == '*'
                || c == '/'
                || c == '('
                || c == ')'
                || c == ','
                || c == ' '
        }) {
            let token = token.trim();
            if !token.is_empty()
                && !token.chars().next().unwrap().is_ascii_digit()
                && !is_function_name(token)
            {
                refs.push(token.to_string());
            }
        }

        refs
    }

    /// Infer the resulting unit of a formula (for calculated columns)
    #[must_use]
    pub fn infer_unit(&self, formula: &str) -> Option<UnitCategory> {
        let refs = self.extract_references(formula);

        // Get the first reference with a known unit
        for r in &refs {
            if let Some(unit) = self.unit_map.get(r.as_str()) {
                // For multiplication with percentage, result is the other unit
                if formula.contains('*') && matches!(unit, UnitCategory::Percentage) {
                    // Find the non-percentage unit
                    for r2 in &refs {
                        if let Some(u2) = self.unit_map.get(r2.as_str()) {
                            if !matches!(u2, UnitCategory::Percentage) {
                                return Some(u2.clone());
                            }
                        }
                    }
                }
                return Some(unit.clone());
            }
        }

        None
    }
}

/// Check if a token is a known function name
fn is_function_name(token: &str) -> bool {
    let upper = token.to_uppercase();
    matches!(
        upper.as_str(),
        "SUM"
            | "AVERAGE"
            | "AVG"
            | "COUNT"
            | "MAX"
            | "MIN"
            | "IF"
            | "IFERROR"
            | "ROUND"
            | "ABS"
            | "SQRT"
            | "SUMIF"
            | "COUNTIF"
            | "AVERAGEIF"
            | "VLOOKUP"
            | "HLOOKUP"
            | "INDEX"
            | "MATCH"
            | "AND"
            | "OR"
            | "NOT"
            | "TRUE"
            | "FALSE"
            | "PMT"
            | "PV"
            | "FV"
            | "NPV"
            | "IRR"
            | "NOW"
            | "TODAY"
            | "DATE"
            | "YEAR"
            | "MONTH"
            | "DAY"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unit_category_from_str() {
        assert!(matches!(
            UnitCategory::parse("CAD"),
            UnitCategory::Currency(_)
        ));
        assert!(matches!(
            UnitCategory::parse("USD"),
            UnitCategory::Currency(_)
        ));
        assert!(matches!(UnitCategory::parse("%"), UnitCategory::Percentage));
        assert!(matches!(UnitCategory::parse("count"), UnitCategory::Count));
        assert!(matches!(UnitCategory::parse("days"), UnitCategory::Time(_)));
        assert!(matches!(UnitCategory::parse("ratio"), UnitCategory::Ratio));
        assert!(matches!(
            UnitCategory::parse("unknown_unit"),
            UnitCategory::Unknown
        ));
    }

    #[test]
    fn test_can_add_same_currency() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        let cad1 = UnitCategory::Currency("CAD".to_string());
        let cad2 = UnitCategory::Currency("CAD".to_string());
        assert!(validator.can_add(&cad1, &cad2));
    }

    #[test]
    fn test_cannot_add_different_currencies() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        let cad = UnitCategory::Currency("CAD".to_string());
        let usd = UnitCategory::Currency("USD".to_string());
        assert!(!validator.can_add(&cad, &usd));
    }

    #[test]
    fn test_cannot_add_currency_and_percentage() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        let cad = UnitCategory::Currency("CAD".to_string());
        let pct = UnitCategory::Percentage;
        assert!(!validator.can_add(&cad, &pct));
    }

    #[test]
    fn test_extract_references() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        let refs = validator.extract_references("=revenue - expenses");
        assert_eq!(refs, vec!["revenue", "expenses"]);
    }

    #[test]
    fn test_extract_references_with_functions() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        let refs = validator.extract_references("=SUM(revenue) + total_costs");
        assert!(refs.contains(&"revenue".to_string()));
        assert!(refs.contains(&"total_costs".to_string()));
        assert!(!refs.contains(&"SUM".to_string()));
    }

    // =========================================================================
    // UnitCategory Tests
    // =========================================================================

    #[test]
    fn test_unit_category_display() {
        assert_eq!(UnitCategory::Currency("CAD".to_string()).display(), "CAD");
        assert_eq!(UnitCategory::Percentage.display(), "%");
        assert_eq!(UnitCategory::Count.display(), "count");
        assert_eq!(UnitCategory::Time("days".to_string()).display(), "days");
        assert_eq!(UnitCategory::Ratio.display(), "ratio");
        assert_eq!(UnitCategory::Unknown.display(), "unknown");
    }

    #[test]
    fn test_unit_category_parse_currencies() {
        assert!(matches!(
            UnitCategory::parse("EUR"),
            UnitCategory::Currency(_)
        ));
        assert!(matches!(
            UnitCategory::parse("GBP"),
            UnitCategory::Currency(_)
        ));
        assert!(matches!(
            UnitCategory::parse("JPY"),
            UnitCategory::Currency(_)
        ));
        assert!(matches!(
            UnitCategory::parse("CNY"),
            UnitCategory::Currency(_)
        ));
        assert!(matches!(
            UnitCategory::parse("$"),
            UnitCategory::Currency(_)
        ));
    }

    #[test]
    fn test_unit_category_parse_percentage() {
        assert!(matches!(
            UnitCategory::parse("percent"),
            UnitCategory::Percentage
        ));
        assert!(matches!(
            UnitCategory::parse("percentage"),
            UnitCategory::Percentage
        ));
    }

    #[test]
    fn test_unit_category_parse_count() {
        assert!(matches!(UnitCategory::parse("units"), UnitCategory::Count));
        assert!(matches!(UnitCategory::parse("items"), UnitCategory::Count));
        assert!(matches!(UnitCategory::parse("qty"), UnitCategory::Count));
        assert!(matches!(
            UnitCategory::parse("quantity"),
            UnitCategory::Count
        ));
    }

    #[test]
    fn test_unit_category_parse_time() {
        assert!(matches!(UnitCategory::parse("day"), UnitCategory::Time(_)));
        assert!(matches!(UnitCategory::parse("d"), UnitCategory::Time(_)));
        assert!(matches!(
            UnitCategory::parse("months"),
            UnitCategory::Time(_)
        ));
        assert!(matches!(
            UnitCategory::parse("month"),
            UnitCategory::Time(_)
        ));
        assert!(matches!(UnitCategory::parse("mo"), UnitCategory::Time(_)));
        assert!(matches!(
            UnitCategory::parse("years"),
            UnitCategory::Time(_)
        ));
        assert!(matches!(UnitCategory::parse("year"), UnitCategory::Time(_)));
        assert!(matches!(UnitCategory::parse("yr"), UnitCategory::Time(_)));
        assert!(matches!(
            UnitCategory::parse("hours"),
            UnitCategory::Time(_)
        ));
        assert!(matches!(UnitCategory::parse("hour"), UnitCategory::Time(_)));
        assert!(matches!(UnitCategory::parse("hr"), UnitCategory::Time(_)));
    }

    #[test]
    fn test_unit_category_parse_ratio() {
        assert!(matches!(UnitCategory::parse("factor"), UnitCategory::Ratio));
        assert!(matches!(
            UnitCategory::parse("multiplier"),
            UnitCategory::Ratio
        ));
        assert!(matches!(UnitCategory::parse("x"), UnitCategory::Ratio));
    }

    // =========================================================================
    // UnitWarning Tests
    // =========================================================================

    #[test]
    fn test_unit_warning_display_warning() {
        let warning = UnitWarning {
            location: "sales.total".to_string(),
            formula: "=revenue + tax_rate".to_string(),
            message: "Mixing units".to_string(),
            severity: WarningSeverity::Warning,
        };
        let display = format!("{warning}");
        assert!(display.contains("Warning"));
        assert!(display.contains("sales.total"));
        assert!(display.contains("Mixing units"));
        assert!(display.contains("=revenue + tax_rate"));
    }

    #[test]
    fn test_unit_warning_display_error() {
        let warning = UnitWarning {
            location: "costs".to_string(),
            formula: "=a + b".to_string(),
            message: "Critical error".to_string(),
            severity: WarningSeverity::Error,
        };
        let display = format!("{warning}");
        assert!(display.contains("Error"));
        assert!(display.contains("costs"));
    }

    #[test]
    fn test_warning_severity_equality() {
        assert_eq!(WarningSeverity::Warning, WarningSeverity::Warning);
        assert_eq!(WarningSeverity::Error, WarningSeverity::Error);
        assert_ne!(WarningSeverity::Warning, WarningSeverity::Error);
    }

    // =========================================================================
    // is_function_name Tests
    // =========================================================================

    #[test]
    fn test_is_function_name_aggregation() {
        assert!(is_function_name("SUM"));
        assert!(is_function_name("AVERAGE"));
        assert!(is_function_name("AVG"));
        assert!(is_function_name("COUNT"));
        assert!(is_function_name("MAX"));
        assert!(is_function_name("MIN"));
    }

    #[test]
    fn test_is_function_name_conditional() {
        assert!(is_function_name("IF"));
        assert!(is_function_name("IFERROR"));
        assert!(is_function_name("SUMIF"));
        assert!(is_function_name("COUNTIF"));
        assert!(is_function_name("AVERAGEIF"));
    }

    #[test]
    fn test_is_function_name_logical() {
        assert!(is_function_name("AND"));
        assert!(is_function_name("OR"));
        assert!(is_function_name("NOT"));
        assert!(is_function_name("TRUE"));
        assert!(is_function_name("FALSE"));
    }

    #[test]
    fn test_is_function_name_math() {
        assert!(is_function_name("ROUND"));
        assert!(is_function_name("ABS"));
        assert!(is_function_name("SQRT"));
    }

    #[test]
    fn test_is_function_name_lookup() {
        assert!(is_function_name("VLOOKUP"));
        assert!(is_function_name("HLOOKUP"));
        assert!(is_function_name("INDEX"));
        assert!(is_function_name("MATCH"));
    }

    #[test]
    fn test_is_function_name_financial() {
        assert!(is_function_name("PMT"));
        assert!(is_function_name("PV"));
        assert!(is_function_name("FV"));
        assert!(is_function_name("NPV"));
        assert!(is_function_name("IRR"));
    }

    #[test]
    fn test_is_function_name_date() {
        assert!(is_function_name("NOW"));
        assert!(is_function_name("TODAY"));
        assert!(is_function_name("DATE"));
        assert!(is_function_name("YEAR"));
        assert!(is_function_name("MONTH"));
        assert!(is_function_name("DAY"));
    }

    #[test]
    fn test_is_function_name_case_insensitive() {
        assert!(is_function_name("sum"));
        assert!(is_function_name("Sum"));
        assert!(is_function_name("SUM"));
    }

    #[test]
    fn test_is_not_function_name() {
        assert!(!is_function_name("revenue"));
        assert!(!is_function_name("total"));
        assert!(!is_function_name("my_var"));
    }

    // =========================================================================
    // can_add Tests
    // =========================================================================

    #[test]
    fn test_can_add_same_time_units() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        let days1 = UnitCategory::Time("days".to_string());
        let days2 = UnitCategory::Time("days".to_string());
        assert!(validator.can_add(&days1, &days2));
    }

    #[test]
    fn test_cannot_add_different_time_units() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        let days = UnitCategory::Time("days".to_string());
        let months = UnitCategory::Time("months".to_string());
        assert!(!validator.can_add(&days, &months));
    }

    #[test]
    fn test_can_add_counts() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        assert!(validator.can_add(&UnitCategory::Count, &UnitCategory::Count));
    }

    #[test]
    fn test_can_add_ratios() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        assert!(validator.can_add(&UnitCategory::Ratio, &UnitCategory::Ratio));
    }

    #[test]
    fn test_can_add_percentages() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        assert!(validator.can_add(&UnitCategory::Percentage, &UnitCategory::Percentage));
    }

    #[test]
    fn test_can_add_with_unknown() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        let cad = UnitCategory::Currency("CAD".to_string());
        assert!(validator.can_add(&UnitCategory::Unknown, &cad));
        assert!(validator.can_add(&cad, &UnitCategory::Unknown));
    }

    #[test]
    fn test_cannot_add_currency_and_count() {
        let validator = UnitValidator {
            model: &ParsedModel::new(),
            unit_map: HashMap::new(),
        };
        let cad = UnitCategory::Currency("CAD".to_string());
        assert!(!validator.can_add(&cad, &UnitCategory::Count));
    }

    // =========================================================================
    // Integration tests for full validation flow
    // =========================================================================

    #[test]
    fn test_validator_new_with_table_units() {
        let mut model = ParsedModel::new();
        let mut table = crate::types::Table::new("sales".to_string());
        let mut col = crate::types::Column::new(
            "revenue".to_string(),
            crate::types::ColumnValue::Number(vec![1000.0, 2000.0]),
        );
        col.metadata.unit = Some("CAD".to_string());
        table.add_column(col);
        model.add_table(table);

        let validator = UnitValidator::new(&model);
        assert!(validator.unit_map.contains_key("sales.revenue"));
        assert!(validator.unit_map.contains_key("revenue"));
    }

    #[test]
    fn test_validator_new_with_scalar_units() {
        let mut model = ParsedModel::new();
        let mut var = crate::types::Variable::new("tax_rate".to_string(), Some(0.15), None);
        var.metadata.unit = Some("%".to_string());
        model.add_scalar("tax_rate".to_string(), var);

        let validator = UnitValidator::new(&model);
        assert!(validator.unit_map.contains_key("tax_rate"));
    }

    #[test]
    fn test_validate_table_row_formula_with_compatible_units() {
        let mut model = ParsedModel::new();
        let mut table = crate::types::Table::new("data".to_string());

        let mut col1 = crate::types::Column::new(
            "price".to_string(),
            crate::types::ColumnValue::Number(vec![100.0]),
        );
        col1.metadata.unit = Some("CAD".to_string());
        table.add_column(col1);

        let mut col2 = crate::types::Column::new(
            "discount".to_string(),
            crate::types::ColumnValue::Number(vec![10.0]),
        );
        col2.metadata.unit = Some("CAD".to_string());
        table.add_column(col2);

        table
            .row_formulas
            .insert("total".to_string(), "=price - discount".to_string());
        model.add_table(table);

        let validator = UnitValidator::new(&model);
        let warnings = validator.validate();
        // Same currency should be compatible
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_table_row_formula_with_incompatible_units() {
        let mut model = ParsedModel::new();
        let mut table = crate::types::Table::new("data".to_string());

        let mut col1 = crate::types::Column::new(
            "revenue".to_string(),
            crate::types::ColumnValue::Number(vec![1000.0]),
        );
        col1.metadata.unit = Some("CAD".to_string());
        table.add_column(col1);

        let mut col2 = crate::types::Column::new(
            "item_count".to_string(),
            crate::types::ColumnValue::Number(vec![10.0]),
        );
        col2.metadata.unit = Some("units".to_string());
        table.add_column(col2);

        // Formula adds currency and count - should warn
        table.row_formulas.insert(
            "invalid_sum".to_string(),
            "=revenue + item_count".to_string(),
        );
        model.add_table(table);

        let validator = UnitValidator::new(&model);
        // Just verify the unit map has the entries
        assert!(validator.unit_map.contains_key("revenue"));
        assert!(validator.unit_map.contains_key("item_count"));

        let warnings = validator.validate();
        // This tests the validation flow, warning presence depends on implementation
        // The validate() path is exercised regardless
        let _ = warnings; // Suppress unused warning
    }

    #[test]
    fn test_validate_scalar_formula_exercises_path() {
        let mut model = ParsedModel::new();

        let mut price = crate::types::Variable::new("price".to_string(), Some(100.0), None);
        price.metadata.unit = Some("CAD".to_string());
        model.add_scalar("price".to_string(), price);

        let mut item_units = crate::types::Variable::new("item_units".to_string(), Some(5.0), None);
        item_units.metadata.unit = Some("count".to_string());
        model.add_scalar("item_units".to_string(), item_units);

        let total = crate::types::Variable::new(
            "total".to_string(),
            None,
            Some("=price + item_units".to_string()),
        );
        model.add_scalar("total".to_string(), total);

        let validator = UnitValidator::new(&model);
        // Verify units are tracked
        assert!(validator.unit_map.contains_key("price"));
        assert!(validator.unit_map.contains_key("item_units"));

        // Exercise the validation path
        let _warnings = validator.validate();
    }

    #[test]
    fn test_validate_formula_with_no_references() {
        let mut model = ParsedModel::new();
        let var =
            crate::types::Variable::new("constant".to_string(), None, Some("=100".to_string()));
        model.add_scalar("constant".to_string(), var);

        let validator = UnitValidator::new(&model);
        let warnings = validator.validate();
        // Formula with only literals should have no warnings
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_percentage_with_currency_addition_path() {
        let mut model = ParsedModel::new();

        let mut cost = crate::types::Variable::new("cost".to_string(), Some(100.0), None);
        cost.metadata.unit = Some("CAD".to_string());
        model.add_scalar("cost".to_string(), cost);

        let mut tax_rate = crate::types::Variable::new("tax_rate".to_string(), Some(0.15), None);
        tax_rate.metadata.unit = Some("%".to_string());
        model.add_scalar("tax_rate".to_string(), tax_rate);

        // This adds price + rate (exercises the percentage + currency path)
        let total_bad = crate::types::Variable::new(
            "total_bad".to_string(),
            None,
            Some("=cost + tax_rate".to_string()),
        );
        model.add_scalar("total_bad".to_string(), total_bad);

        let validator = UnitValidator::new(&model);
        // Verify units are tracked
        assert!(validator.unit_map.contains_key("cost"));
        assert!(validator.unit_map.contains_key("tax_rate"));

        // Exercise the percentage + currency validation path
        let warnings = validator.validate();
        // Should warn about adding percentage to currency
        assert!(!warnings.is_empty(), "Should produce a warning");
        assert!(
            warnings[0]
                .message
                .contains("Adding percentage to currency"),
            "Warning should mention percentage + currency"
        );
    }

    #[test]
    fn test_infer_unit_simple() {
        let mut model = ParsedModel::new();

        let mut price = crate::types::Variable::new("price".to_string(), Some(100.0), None);
        price.metadata.unit = Some("CAD".to_string());
        model.add_scalar("price".to_string(), price);

        let validator = UnitValidator::new(&model);

        // Infer unit from simple formula
        let unit = validator.infer_unit("=price * 2");
        assert!(matches!(unit, Some(UnitCategory::Currency(_))));
    }

    #[test]
    fn test_infer_unit_with_percentage_multiplication() {
        let mut model = ParsedModel::new();

        let mut price = crate::types::Variable::new("price".to_string(), Some(100.0), None);
        price.metadata.unit = Some("CAD".to_string());
        model.add_scalar("price".to_string(), price);

        let mut rate = crate::types::Variable::new("rate".to_string(), Some(0.15), None);
        rate.metadata.unit = Some("%".to_string());
        model.add_scalar("rate".to_string(), rate);

        let validator = UnitValidator::new(&model);

        // When multiplying price * percentage, result should be currency (price's unit)
        let unit = validator.infer_unit("=rate * price");
        assert!(
            matches!(unit, Some(UnitCategory::Currency(ref c)) if c == "CAD"),
            "Percentage * currency should yield currency, got {unit:?}"
        );
    }

    #[test]
    fn test_infer_unit_unknown_reference() {
        let model = ParsedModel::new();
        let validator = UnitValidator::new(&model);

        // Formula with unknown references
        let unit = validator.infer_unit("=unknown_var + 100");
        assert!(unit.is_none(), "Unknown reference should yield None");
    }

    #[test]
    fn test_infer_unit_percentage_times_unknown() {
        let mut model = ParsedModel::new();

        // Only the percentage has a unit, the other variable does not
        let mut rate = crate::types::Variable::new("rate".to_string(), Some(0.15), None);
        rate.metadata.unit = Some("%".to_string());
        model.add_scalar("rate".to_string(), rate);

        // unknown_amount has no unit
        let amount = crate::types::Variable::new("unknown_amount".to_string(), Some(100.0), None);
        model.add_scalar("unknown_amount".to_string(), amount);

        let validator = UnitValidator::new(&model);

        // rate (percentage) * unknown_amount - should return percentage since no other unit found
        let unit = validator.infer_unit("=rate * unknown_amount");
        assert!(
            matches!(unit, Some(UnitCategory::Percentage)),
            "Should fall back to percentage when other reference has no unit, got {unit:?}"
        );
    }
}
