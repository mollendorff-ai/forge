//! Audit command and dependency tracking

use crate::core::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::path::Path;

use super::format_number;

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

/// Represents a dependency in the audit tree
pub struct AuditDependency {
    pub name: String,
    pub dep_type: String,
    pub formula: Option<String>,
    pub value: Option<f64>,
    pub children: Vec<Self>,
}

/// Convert `AuditDependency` tree to serializable `AuditDep`
fn to_audit_dep(dep: &AuditDependency) -> super::results::AuditDep {
    super::results::AuditDep {
        name: dep.name.clone(),
        dep_type: dep.dep_type.clone(),
        formula: dep.formula.clone(),
        value: dep.value,
        children: dep.children.iter().map(to_audit_dep).collect(),
    }
}

/// Audit a variable and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, the variable is not found,
/// or formula calculation fails.
pub fn audit_core(file: &Path, variable: &str) -> ForgeResult<super::results::AuditResult> {
    let model = parser::parse_model(file)?;
    let (var_type, formula, current_value) = find_variable(&model, variable)?;

    let dependencies = if formula.is_some() {
        let deps = build_dependency_tree(&model, variable, formula.as_ref(), 0)?;
        deps.iter().map(to_audit_dep).collect()
    } else {
        vec![]
    };

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    let (calculated_value, is_valid) = result.as_ref().map_or((None, false), |r| {
        r.scalars.get(variable).map_or((None, true), |scalar| {
            let calc_val = scalar.value;
            let valid = match (current_value, calc_val) {
                (Some(curr), Some(calc)) => (curr - calc).abs() < 0.0001,
                _ => true,
            };
            (calc_val, valid)
        })
    });

    Ok(super::results::AuditResult {
        variable: variable.to_string(),
        var_type,
        current_value,
        calculated_value,
        formula,
        dependencies,
        is_valid,
    })
}

/// Execute the audit command - show calculation dependency chain.
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, the variable is not found,
/// or formula calculation fails.
pub fn audit(file: &Path, variable: &str) -> ForgeResult<()> {
    println!("{}", "🔍 Forge - Audit Trail".bold().green());
    println!("   File: {}", file.display());
    println!("   Variable: {}\n", variable.bright_blue().bold());

    // Parse the model
    let model = parser::parse_model(file)?;

    // Try to find the variable
    let (var_type, formula, current_value) = find_variable(&model, variable)?;

    println!("{}", "📋 Variable Information:".bold().cyan());
    println!("   Type: {}", var_type.cyan());
    if let Some(val) = current_value {
        println!("   Current Value: {}", format_number(val).bold().green());
    }
    if let Some(ref f) = formula {
        println!("   Formula: {}", f.bright_yellow());
    }
    println!();

    // Build and display dependency tree
    if formula.is_some() {
        println!("{}", "🌳 Dependency Tree:".bold().cyan());
        let deps = build_dependency_tree(&model, variable, formula.as_ref(), 0)?;

        if deps.is_empty() {
            println!("   No dependencies (literal value)");
        } else {
            for dep in &deps {
                print_dependency(dep, 1);
            }
        }
        println!();
    }

    // Calculate and verify
    println!("{}", "🧮 Calculation Chain:".bold().cyan());
    let calculator = ArrayCalculator::new(model);
    match calculator.calculate_all() {
        Ok(result) => {
            // Find the calculated value
            if let Some(scalar) = result.scalars.get(variable) {
                if let Some(calc_val) = scalar.value {
                    println!("   Calculated: {}", format_number(calc_val).bold().green());

                    // Check if it matches current value
                    if let Some(curr) = current_value {
                        let diff = (curr - calc_val).abs();
                        if diff < 0.0001 {
                            println!("   {} Values match!", "✅".green());
                        } else {
                            println!("   {} Value mismatch!", "⚠️".yellow());
                            println!("      Current:    {}", format_number(curr).red());
                            println!("      Calculated: {}", format_number(calc_val).green());
                        }
                    }
                }
            } else {
                // Check in tables
                for (table_name, table) in &result.tables {
                    if let Some(col) = table.columns.get(variable) {
                        println!("   Table: {}", table_name.bright_blue());
                        println!("   Column values: {:?}", col.values);
                        break;
                    }
                }
            }
        },
        Err(e) => {
            println!("   {} Calculation error: {}", "❌".red(), e);
        },
    }

    println!();
    println!("{}", "✅ Audit complete".bold().green());
    Ok(())
}

/// Find a variable in the model and return its type, formula, and current value.
///
/// # Errors
///
/// Returns an error if the variable is not found in scalars, aggregations, or table columns.
pub fn find_variable(
    model: &crate::types::ParsedModel,
    name: &str,
) -> ForgeResult<(String, Option<String>, Option<f64>)> {
    // Check scalars first
    if let Some(scalar) = model.scalars.get(name) {
        let formula = scalar.formula.clone();
        let value = scalar.value;
        return Ok(("Scalar".to_string(), formula, value));
    }

    // Check aggregations
    if let Some(agg_formula) = model.aggregations.get(name) {
        return Ok(("Aggregation".to_string(), Some(agg_formula.clone()), None));
    }

    // Check table columns
    for (table_name, table) in &model.tables {
        if table.columns.contains_key(name) {
            let formula = table.row_formulas.get(name).cloned();
            return Ok((format!("Column in table '{table_name}'"), formula, None));
        }
    }

    Err(ForgeError::Validation(format!(
        "Variable '{}' not found in model. Available:\n  Scalars: {:?}\n  Aggregations: {:?}\n  Tables: {:?}",
        name,
        model.scalars.keys().collect::<Vec<_>>(),
        model.aggregations.keys().collect::<Vec<_>>(),
        model.tables.keys().collect::<Vec<_>>()
    )))
}

/// Build the dependency tree for a variable.
///
/// # Errors
///
/// Returns an error if recursive dependency resolution fails.
pub fn build_dependency_tree(
    model: &crate::types::ParsedModel,
    _name: &str,
    formula: Option<&String>,
    depth: usize,
) -> ForgeResult<Vec<AuditDependency>> {
    // Prevent infinite recursion
    if depth > 20 {
        return Ok(vec![]);
    }

    let mut deps = Vec::new();

    if let Some(f) = formula {
        // Extract references from formula
        let refs = extract_references_from_formula(f);

        for ref_name in refs {
            let mut dep = AuditDependency {
                name: ref_name.clone(),
                dep_type: "Unknown".to_string(),
                formula: None,
                value: None,
                children: vec![],
            };

            // Try to find this reference in the model
            if let Some(scalar) = model.scalars.get(&ref_name) {
                dep.dep_type = "Scalar".to_string();
                dep.formula.clone_from(&scalar.formula);
                dep.value = scalar.value;

                // Recursively get children
                if scalar.formula.is_some() {
                    dep.children = build_dependency_tree(
                        model,
                        &ref_name,
                        scalar.formula.as_ref(),
                        depth + 1,
                    )?;
                }
            } else if let Some(agg) = model.aggregations.get(&ref_name) {
                dep.dep_type = "Aggregation".to_string();
                dep.formula = Some(agg.clone());
                dep.children = build_dependency_tree(model, &ref_name, Some(agg), depth + 1)?;
            } else {
                // Check if it's a table column
                for (table_name, table) in &model.tables {
                    if table.columns.contains_key(&ref_name) {
                        dep.dep_type = format!("Column[{table_name}]");
                        dep.formula = table.row_formulas.get(&ref_name).cloned();
                        break;
                    }
                }
            }

            deps.push(dep);
        }
    }

    Ok(deps)
}

/// Extract variable references from a formula.
///
/// # Panics
///
/// Panics if a non-empty word has no first character, which cannot happen since
/// empty words are skipped.
#[must_use]
pub fn extract_references_from_formula(formula: &str) -> Vec<String> {
    let formula = formula.trim_start_matches('=');
    // Strip string literals to avoid parsing their contents as variable references
    // e.g., =LEN("Hello") should not treat "Hello" as a variable reference
    let formula_stripped = strip_string_literals(formula);
    let mut refs = Vec::new();

    // Known function names to exclude
    let functions = [
        "SUM",
        "AVERAGE",
        "AVG",
        "MAX",
        "MIN",
        "COUNT",
        "PRODUCT",
        "SUMIF",
        "COUNTIF",
        "AVERAGEIF",
        "SUMIFS",
        "COUNTIFS",
        "AVERAGEIFS",
        "MAXIFS",
        "MINIFS",
        "ROUND",
        "ROUNDUP",
        "ROUNDDOWN",
        "CEILING",
        "FLOOR",
        "SQRT",
        "POWER",
        "MOD",
        "ABS",
        "IF",
        "AND",
        "OR",
        "NOT",
        "CONCAT",
        "UPPER",
        "LOWER",
        "TRIM",
        "LEN",
        "MID",
        "TODAY",
        "DATE",
        "YEAR",
        "MONTH",
        "DAY",
        "MATCH",
        "INDEX",
        "XLOOKUP",
        "VLOOKUP",
        "IFERROR",
        "TRUE",
        "FALSE",
        "UNIQUE",
        "COUNTUNIQUE",
    ];

    for word in formula_stripped.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if word.is_empty() {
            continue;
        }
        // Skip if starts with number
        if word.chars().next().unwrap().is_numeric() {
            continue;
        }
        // Skip function names
        if functions.contains(&word.to_uppercase().as_str()) {
            continue;
        }
        // Skip if already added
        if !refs.contains(&word.to_string()) {
            refs.push(word.to_string());
        }
    }

    refs
}

/// Print a dependency with indentation
pub fn print_dependency(dep: &AuditDependency, indent: usize) {
    let prefix = "   ".repeat(indent);
    let arrow = if indent > 0 { "└─ " } else { "" };

    print!("{}{}{} ", prefix, arrow, dep.name.bright_blue());
    print!("({})", dep.dep_type.cyan());

    if let Some(val) = dep.value {
        print!(" = {}", format_number(val).green());
    }

    if let Some(ref f) = dep.formula {
        print!(" {}", f.yellow());
    }

    println!();

    for child in &dep.children {
        print_dependency(child, indent + 1);
    }
}
