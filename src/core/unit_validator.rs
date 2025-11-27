//! Unit consistency validation for v4.0 rich metadata
//!
//! Validates that formulas don't mix incompatible units (e.g., CAD + %)

use crate::types::ParsedModel;
use std::collections::HashMap;

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
    pub fn parse(unit: &str) -> Self {
        let unit_lower = unit.to_lowercase();
        match unit_lower.as_str() {
            // Currency
            "cad" | "usd" | "eur" | "gbp" | "jpy" | "cny" | "$" => {
                UnitCategory::Currency(unit.to_uppercase())
            }
            // Percentage
            "%" | "percent" | "percentage" => UnitCategory::Percentage,
            // Count
            "count" | "units" | "items" | "qty" | "quantity" => UnitCategory::Count,
            // Time
            "days" | "day" | "d" => UnitCategory::Time("days".to_string()),
            "months" | "month" | "mo" => UnitCategory::Time("months".to_string()),
            "years" | "year" | "yr" => UnitCategory::Time("years".to_string()),
            "hours" | "hour" | "hr" => UnitCategory::Time("hours".to_string()),
            // Ratio
            "ratio" | "factor" | "multiplier" | "x" => UnitCategory::Ratio,
            // Unknown
            _ => UnitCategory::Unknown,
        }
    }

    /// Get a display name for the unit category
    pub fn display(&self) -> String {
        match self {
            UnitCategory::Currency(c) => c.clone(),
            UnitCategory::Percentage => "%".to_string(),
            UnitCategory::Count => "count".to_string(),
            UnitCategory::Time(t) => t.clone(),
            UnitCategory::Ratio => "ratio".to_string(),
            UnitCategory::Unknown => "unknown".to_string(),
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

#[derive(Debug, Clone, PartialEq)]
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
    pub fn new(model: &'a ParsedModel) -> Self {
        let mut unit_map = HashMap::new();

        // Extract units from table columns
        for (table_name, table) in &model.tables {
            for (col_name, column) in &table.columns {
                if let Some(unit) = &column.metadata.unit {
                    let path = format!("{}.{}", table_name, col_name);
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
    pub fn validate(&self) -> Vec<UnitWarning> {
        let mut warnings = Vec::new();

        // Validate table row formulas
        for (table_name, table) in &self.model.tables {
            for (col_name, formula) in &table.row_formulas {
                let location = format!("{}.{}", table_name, col_name);
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
        let ref_units: Vec<(&str, Option<&UnitCategory>)> =
            refs.iter().map(|r| (*r, self.unit_map.get(*r))).collect();

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

        // Check for percentage applied correctly
        if ref_units
            .iter()
            .any(|(_, u)| matches!(u, Some(UnitCategory::Percentage)))
        {
            // Percentage should be multiplied, not added
            if formula.contains('+') {
                let has_currency = ref_units
                    .iter()
                    .any(|(_, u)| matches!(u, Some(UnitCategory::Currency(_))));
                if has_currency {
                    return Some(UnitWarning {
                        location: location.to_string(),
                        formula: formula.to_string(),
                        message: "Adding percentage to currency - did you mean to multiply?"
                            .to_string(),
                        severity: WarningSeverity::Warning,
                    });
                }
            }
        }

        None
    }

    /// Check if two units can be added/subtracted
    fn can_add(&self, a: &UnitCategory, b: &UnitCategory) -> bool {
        match (a, b) {
            // Same currency can be added
            (UnitCategory::Currency(c1), UnitCategory::Currency(c2)) => c1 == c2,
            // Same time units can be added
            (UnitCategory::Time(t1), UnitCategory::Time(t2)) => t1 == t2,
            // Counts can be added
            (UnitCategory::Count, UnitCategory::Count) => true,
            // Ratios can be added
            (UnitCategory::Ratio, UnitCategory::Ratio) => true,
            // Percentages can be added
            (UnitCategory::Percentage, UnitCategory::Percentage) => true,
            // Unknown is permissive
            (UnitCategory::Unknown, _) | (_, UnitCategory::Unknown) => true,
            // Everything else is incompatible
            _ => false,
        }
    }

    /// Extract variable references from a formula
    fn extract_references<'b>(&self, formula: &'b str) -> Vec<&'b str> {
        let formula = formula.trim_start_matches('=');
        let mut refs = Vec::new();

        // Simple tokenization - split by operators and extract identifiers
        for token in formula.split(|c: char| {
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
                refs.push(token);
            }
        }

        refs
    }

    /// Infer the resulting unit of a formula (for calculated columns)
    pub fn infer_unit(&self, formula: &str) -> Option<UnitCategory> {
        let refs = self.extract_references(formula);

        // Get the first reference with a known unit
        for r in refs {
            if let Some(unit) = self.unit_map.get(r) {
                // For multiplication with percentage, result is the other unit
                if formula.contains('*') && matches!(unit, UnitCategory::Percentage) {
                    // Find the non-percentage unit
                    for r2 in self.extract_references(formula) {
                        if let Some(u2) = self.unit_map.get(r2) {
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
        assert!(refs.contains(&"revenue"));
        assert!(refs.contains(&"total_costs"));
        assert!(!refs.contains(&"SUM"));
    }
}
