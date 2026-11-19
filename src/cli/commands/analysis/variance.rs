//! `forge variance`: budget against actual, line by line, with which way is
//! better and the lines past the alert threshold.

use crate::cli::console::outln;
use crate::cli::Console;
use crate::core::precision::same;
use crate::core::precision::to_15_digits;
use crate::core::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::cli::commands::display_number;

/// Variance result for a single variable
#[derive(Debug, Clone)]
pub struct VarianceResult {
    pub name: String,
    pub budget: f64,
    pub actual: f64,
    pub variance: f64,
    pub variance_pct: f64,
    /// `false` when the percentage means nothing (n/m): the budget is zero,
    /// or the budget and the actual have opposite signs
    pub pct_meaningful: bool,
    /// `None` when Forge cannot tell whether higher or lower is better
    pub is_favorable: Option<bool>,
    /// "higher", "lower" or "unknown": which way is good, and so how
    /// `is_favorable` was decided
    pub better: &'static str,
    pub exceeds_threshold: bool,
}

/// Compute variance analysis and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the budget or actual files cannot be parsed or calculation fails.
pub fn variance_core(
    budget_path: &Path,
    actual_path: &Path,
    threshold: f64,
) -> ForgeResult<crate::cli::commands::results::VarianceAnalysis> {
    let budget_model = parser::parse_model(budget_path)?;
    let actual_model = parser::parse_model(actual_path)?;

    let budget_calculator = ArrayCalculator::new(budget_model);
    let budget_result = budget_calculator.calculate_all()?;

    let actual_calculator = ArrayCalculator::new(actual_model);
    let actual_result = actual_calculator.calculate_all()?;

    let (variances, not_compared) = collect_variances(&budget_result, &actual_result, threshold);

    let changed = || variances.iter().filter(|v| !on_target(v));
    let favorable_count = changed().filter(|v| v.is_favorable == Some(true)).count();
    let unfavorable_count = changed().filter(|v| v.is_favorable == Some(false)).count();
    let no_direction_count = changed().filter(|v| v.is_favorable.is_none()).count();
    let on_target_count = variances.iter().filter(|v| on_target(v)).count();
    let alert_count = variances.iter().filter(|v| v.exceeds_threshold).count();

    let entries = variances
        .into_iter()
        .map(|v| crate::cli::commands::results::VarianceEntry {
            name: v.name,
            budget: v.budget,
            actual: v.actual,
            variance: v.variance,
            variance_pct: v.variance_pct,
            pct_meaningful: v.pct_meaningful,
            is_favorable: v.is_favorable,
            better: v.better.to_string(),
            exceeds_threshold: v.exceeds_threshold,
        })
        .collect();

    Ok(crate::cli::commands::results::VarianceAnalysis {
        results: entries,
        favorable_count,
        unfavorable_count,
        no_direction_count,
        on_target_count,
        alert_count,
        threshold,
        not_compared,
    })
}

/// Execute the variance command - budget vs actual analysis
///
/// # Errors
///
/// Returns an error if the budget or actual files cannot be parsed, calculation fails,
/// or the output file cannot be written.
pub fn variance(
    c: &mut Console<'_>,
    budget_path: &Path,
    actual_path: &Path,
    threshold: f64,
    output: Option<&Path>,
    (fail_on_alert, verbose): (bool, bool),
) -> ForgeResult<()> {
    outln!(c, "{}", "🔥 Forge - Variance Analysis".bold().green());
    outln!(c, "   Budget: {}", budget_path.display());
    outln!(c, "   Actual: {}", actual_path.display());
    outln!(c, "   Threshold: {threshold}%\n");

    // Parse both files
    if verbose {
        outln!(c, "{}", "📖 Parsing YAML files...".cyan());
    }

    let budget_model = parser::parse_model(budget_path)?;
    let actual_model = parser::parse_model(actual_path)?;

    // Calculate both models
    if verbose {
        outln!(c, "{}", "🧮 Calculating formulas...".cyan());
    }

    let budget_calculator = ArrayCalculator::new(budget_model);
    let budget_result = budget_calculator.calculate_all()?;

    let actual_calculator = ArrayCalculator::new(actual_model);
    let actual_result = actual_calculator.calculate_all()?;

    let (variances, not_compared) = collect_variances(&budget_result, &actual_result, threshold);

    // Handle output
    if let Some(output_path) = output {
        let extension = output_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match extension {
            "xlsx" => {
                export_variance_to_excel(output_path, &variances, threshold)?;
                outln!(
                    c,
                    "{}",
                    format!("✅ Variance report exported to {}", output_path.display())
                        .bold()
                        .green()
                );
            },
            "yaml" | "yml" => {
                export_variance_to_yaml(output_path, &variances, threshold)?;
                outln!(
                    c,
                    "{}",
                    format!("✅ Variance report exported to {}", output_path.display())
                        .bold()
                        .green()
                );
            },
            _ => {
                return Err(ForgeError::Export(format!(
                    "Unsupported output format: {extension}. Use .xlsx or .yaml"
                )));
            },
        }
    } else {
        // Print to terminal
        print_variance_table(c, &variances, threshold);
    }

    // Summary
    let count = |f: Option<bool>| {
        variances
            .iter()
            .filter(|v| !on_target(v) && v.is_favorable == f)
            .count()
    };
    let alert_count = variances.iter().filter(|v| v.exceeds_threshold).count();

    outln!(c);
    outln!(c, "   {} Favorable: {}  {} Unfavorable: {}  · No direction: {}  = On target: {}  {} Alerts (>{:.0}%): {}",
        "✅".green(),
        count(Some(true)).to_string().green(),
        "❌".red(),
        count(Some(false)).to_string().red(),
        count(None),
        variances.iter().filter(|v| on_target(v)).count(),
        "⚠️".yellow(),
        threshold,
        alert_count.to_string().yellow()
    );
    if count(None) > 0 {
        outln!(c, "   For a scalar with no direction, add `better: lower` (costs) or `better: higher` to it."
        );
    }
    if !not_compared.is_empty() {
        outln!(
            c,
            "   Not compared (missing or empty in one model, or a date): {}",
            not_compared.join(", ")
        );
    }

    if fail_on_alert && alert_count > 0 {
        return Err(ForgeError::Validation(format!(
            "{alert_count} line(s) exceed the {threshold}% threshold (--fail-on-alert)"
        )));
    }
    Ok(())
}

/// Which way is good for a scalar: its `better:` field in either model, or
/// else a word in its name that clearly means a cost or a result, or else
/// its group (`income.*` higher, `expenses.*` lower). Anything in an expense
/// group is lower; a salary, wage or interest in an income group is higher.
fn better_direction(
    name: &str,
    budget: &crate::types::Variable,
    actual: &crate::types::Variable,
) -> &'static str {
    let declared = budget
        .metadata
        .better
        .as_deref()
        .or(actual.metadata.better.as_deref());
    match declared {
        Some("lower") => return "lower",
        Some("higher") => return "higher",
        _ => {},
    }
    let lower = name.to_lowercase();
    let (groups, last) = lower.rsplit_once('.').unwrap_or(("", &lower));
    let words: Vec<&str> = last.split('_').collect();
    let has = |list: &[&str]| words.iter().any(|w| list.contains(w));
    // The group says which way a name goes when the name alone cannot:
    // `income.salary` is earned, `expenses.salary` is paid
    let group_words: Vec<&str> = groups.split(['.', '_']).collect();
    let group_is = |list: &[&str]| group_words.iter().any(|w| list.contains(w));
    let group = if group_is(&[
        "income", "revenue", "revenues", "sales", "earnings", "inflows",
    ]) {
        Some("higher")
    } else if group_is(&[
        "expense", "expenses", "cost", "costs", "spending", "opex", "capex", "outflows",
    ]) {
        Some("lower")
    } else {
        None
    };
    // Everything in an expense group is paid, a sales commission included
    if group == Some("lower") {
        return "lower";
    }
    // Earned by a household, paid by a business
    let either_way = ["salary", "salaries", "wage", "wages", "interest"];
    if group == Some("higher") && words.iter().any(|w| either_way.contains(w)) {
        return "higher";
    }
    if has(&[
        "cost",
        "costs",
        "cogs",
        "expense",
        "expenses",
        "opex",
        "capex",
        "rent",
        "wage",
        "wages",
        "salary",
        "salaries",
        "payroll",
        "labour",
        "labor",
        "fee",
        "fees",
        "tax",
        "taxes",
        "interest",
        "depreciation",
        "spend",
        "debt",
    ]) {
        "lower"
    } else if has(&[
        "revenue", "revenues", "sales", "profit", "income", "margin", "ebit", "ebitda", "npv",
        "irr",
    ]) {
        "higher"
    } else {
        group.unwrap_or("unknown")
    }
}

/// Collect scalar variances between budget and actual calculation results.
///
/// Returns the variances, and the names that are in only one of the models
/// (those are not compared: treating a missing value as 0 would invent one).
fn collect_variances(
    budget_result: &crate::types::ParsedModel,
    actual_result: &crate::types::ParsedModel,
    threshold: f64,
) -> (Vec<VarianceResult>, Vec<String>) {
    let mut variances = Vec::new();
    let mut not_compared = Vec::new();

    for (name, budget_scalar) in &budget_result.scalars {
        let Some(actual_scalar) = actual_result.scalars.get(name) else {
            not_compared.push(name.clone());
            continue;
        };
        let (Some(budget_val), Some(actual_val)) = (budget_scalar.value, actual_scalar.value)
        else {
            not_compared.push(name.clone());
            continue;
        };
        // A date has no variance as a share of the budget (ADR-060)
        if budget_scalar.is_date || actual_scalar.is_date {
            not_compared.push(name.clone());
            continue;
        }

        let variance_abs = actual_val - budget_val;
        // Divide by the size of the budget, so a worse result is negative
        // even when the budget itself is negative (a loss, a negative NPV).
        let variance_pct = if budget_val.abs() > 0.0001 {
            (variance_abs / budget_val.abs()) * 100.0
        } else {
            0.0
        };

        let pct_meaningful = pct_is_meaningful(budget_val, actual_val);

        let better = better_direction(name, budget_scalar, actual_scalar);
        // A line on target is neither favorable nor unfavorable, so it is
        // not counted as either and says so (`null`), exactly as `on_target`
        let is_favorable = match better {
            _ if same(actual_val, budget_val) => None,
            "lower" => Some(actual_val < budget_val),
            "higher" => Some(actual_val > budget_val),
            _ => None,
        };

        // A change from a zero budget, or across zero, is larger than any
        // share of the budget, so it is always an alert
        let exceeds_threshold = if pct_meaningful {
            variance_pct.abs() >= threshold
        } else {
            variance_abs.abs() > 0.0
        };

        variances.push(VarianceResult {
            name: name.clone(),
            budget: budget_val,
            actual: actual_val,
            variance: variance_abs,
            variance_pct,
            pct_meaningful,
            is_favorable,
            better,
            exceeds_threshold,
        });
    }
    for name in actual_result.scalars.keys() {
        if !budget_result.scalars.contains_key(name) {
            not_compared.push(name.clone());
        }
    }
    (variances, not_compared)
}

/// Whether a variance as a percentage of the budget means anything.
///
/// It does not (n/m, "not meaningful", as finance reports write it) when the budget
/// is zero, or when the budget and the actual are on opposite sides of zero:
/// a profit of 786 that became a loss of 28,314 is not "-3,700%" worse, it is
/// 29,100 worse. The variance itself is always shown.
#[must_use]
pub fn pct_is_meaningful(budget: f64, actual: f64) -> bool {
    if budget.abs() <= 0.0001 {
        return false;
    }
    actual == 0.0 || budget.is_sign_positive() == actual.is_sign_positive()
}

fn on_target(v: &VarianceResult) -> bool {
    same(v.actual, v.budget)
}

/// The status word for a variance, for the terminal, Excel and YAML reports.
fn variance_status(v: &VarianceResult) -> &'static str {
    if on_target(v) {
        return "On target";
    }
    match (v.exceeds_threshold, v.is_favorable) {
        (true, Some(false)) => "ALERT - Unfavorable",
        (true, Some(true)) => "ALERT - Favorable",
        (true, None) => "ALERT - Changed",
        (false, Some(true)) => "Favorable",
        (false, Some(false)) => "Unfavorable",
        (false, None) => "No direction",
    }
}

/// Print variance results as a table
///
/// # Errors
///
/// Returns an error if the output cannot be written.
pub fn print_variance_table(c: &mut Console<'_>, variances: &[VarianceResult], threshold: f64) {
    // As wide as the longest name, so every column lines up
    let w = variances
        .iter()
        .map(|v| v.name.chars().count())
        .max()
        .unwrap_or(0)
        .max(20);
    let rule = "─".repeat(w + 65);
    outln!(c, "\n{}", "📊 Budget vs Actual Variance:".bold().cyan());
    outln!(c, "{rule}");

    // Header
    outln!(
        c,
        "{:<w$} {:>12} {:>12} {:>12} {:>10} {:>8}",
        "Variable".bold(),
        "Budget".bold(),
        "Actual".bold(),
        "Variance".bold(),
        "Var %".bold(),
        "Status".bold()
    );
    outln!(c, "{rule}");

    // Data rows
    for v in variances {
        let var_str = display_number(v.variance);
        let pct_str = if v.pct_meaningful {
            format!("{:.1}%", v.variance_pct)
        } else {
            "n/m".to_string()
        };

        let mark = match v.is_favorable {
            _ if on_target(v) => "=",
            Some(true) => "✅",
            Some(false) => "❌",
            None => "·",
        };
        let status = if v.exceeds_threshold {
            format!("⚠️ {mark}")
        } else {
            mark.to_string()
        };

        // Color the variance based on favorability
        let (var_colored, pct_colored) = match v.is_favorable {
            Some(true) => (var_str.green(), pct_str.green()),
            Some(false) => (var_str.red(), pct_str.red()),
            None => (var_str.normal(), pct_str.normal()),
        };

        outln!(
            c,
            "{:<w$} {:>12} {:>12} {:>12} {:>10} {:>8}",
            v.name.bright_blue(),
            display_number(v.budget),
            display_number(v.actual),
            var_colored,
            pct_colored,
            status
        );
    }

    outln!(c, "{rule}");
    outln!(
        c,
        "   {} = exceeds {:.0}% threshold",
        "⚠️".yellow(),
        threshold
    );
    if variances.iter().any(|v| !v.pct_meaningful) {
        outln!(c, "   n/m = not meaningful: the budget is zero or the result changed sign; read the variance"
        );
    }
}

/// Export variance report to Excel
///
/// # Errors
///
/// Returns an error if the Excel workbook cannot be created or saved.
pub fn export_variance_to_excel(
    output: &Path,
    variances: &[VarianceResult],
    threshold: f64,
) -> ForgeResult<()> {
    #[cfg(not(feature = "excel"))]
    {
        let _ = (output, variances, threshold);
        Err(ForgeError::Validation(
            "Writing Excel needs Forge's `excel` feature, which this build does not have"
                .to_string(),
        ))
    }
    #[cfg(feature = "excel")]
    export_variance_to_excel_with_xlsx(output, variances, threshold)
}

/// [`export_variance_to_excel`], with Excel available.
#[cfg(feature = "excel")]
fn export_variance_to_excel_with_xlsx(
    output: &Path,
    variances: &[VarianceResult],
    threshold: f64,
) -> ForgeResult<()> {
    use rust_xlsxwriter::{Format, Workbook};

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Set column widths
    worksheet.set_column_width(0, 20).ok();
    worksheet.set_column_width(1, 12).ok();
    worksheet.set_column_width(2, 12).ok();
    worksheet.set_column_width(3, 12).ok();
    worksheet.set_column_width(4, 10).ok();
    worksheet.set_column_width(5, 10).ok();

    // Header format
    let header_format = Format::new().set_bold();

    // Headers
    worksheet
        .write_string_with_format(0, 0, "Variable", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 1, "Budget", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 2, "Actual", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 3, "Variance", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 4, "Var %", &header_format)
        .ok();
    worksheet
        .write_string_with_format(0, 5, "Status", &header_format)
        .ok();

    // Data rows
    let mut last_row = 0;
    for (row, v) in (1u32..=u32::MAX).zip(variances) {
        last_row = row;
        worksheet.write_string(row, 0, &v.name).ok();
        worksheet.write_number(row, 1, v.budget).ok();
        worksheet.write_number(row, 2, v.actual).ok();
        worksheet.write_number(row, 3, v.variance).ok();
        if v.pct_meaningful {
            worksheet.write_number(row, 4, v.variance_pct / 100.0).ok(); // As decimal for %
        } else {
            worksheet.write_string(row, 4, "n/m").ok();
        }

        worksheet.write_string(row, 5, variance_status(v)).ok();
    }

    // Add metadata row
    // Two rows below the last line
    let meta_row = last_row.saturating_add(3);
    worksheet
        .write_string(meta_row, 0, format!("Threshold: {threshold}%"))
        .ok();
    worksheet
        .write_string(
            meta_row + 1,
            0,
            concat!("Generated by Forge v", env!("CARGO_PKG_VERSION")),
        )
        .ok();

    workbook
        .save(output)
        .map_err(|e| ForgeError::Export(e.to_string()))?;

    Ok(())
}

/// Export variance report to YAML
///
/// # Errors
///
/// Returns an error if the output file cannot be created or written.
pub fn export_variance_to_yaml(
    output: &Path,
    variances: &[VarianceResult],
    threshold: f64,
) -> ForgeResult<()> {
    use std::io::Write as IoWrite;

    let mut content = String::new();
    content.push_str("# Forge Variance Analysis Report\n");
    content.push_str(concat!(
        "# Generated by Forge v",
        env!("CARGO_PKG_VERSION"),
        "\n"
    ));
    let _ = writeln!(content, "# Threshold: {threshold}%\n");

    content.push_str("metadata:\n");
    let _ = writeln!(content, "  threshold_pct: {threshold}");
    let _ = writeln!(content, "  total_items: {}", variances.len());
    let _ = writeln!(
        content,
        "  favorable_count: {}",
        variances
            .iter()
            .filter(|v| !on_target(v) && v.is_favorable == Some(true))
            .count()
    );
    let _ = writeln!(
        content,
        "  alert_count: {}\n",
        variances.iter().filter(|v| v.exceeds_threshold).count()
    );

    content.push_str("variances:\n");
    for v in variances {
        let _ = writeln!(content, "  {}:", v.name);
        let _ = writeln!(content, "    budget: {}", to_15_digits(v.budget));
        let _ = writeln!(content, "    actual: {}", to_15_digits(v.actual));
        let _ = writeln!(content, "    variance: {}", to_15_digits(v.variance));
        let _ = writeln!(content, "    variance_pct: {:.2}", v.variance_pct);
        let _ = writeln!(content, "    pct_meaningful: {}", v.pct_meaningful);
        let favorable = v
            .is_favorable
            .map_or_else(|| "null".to_string(), |f| f.to_string());
        let _ = writeln!(content, "    is_favorable: {favorable}");
        let _ = writeln!(content, "    better: {}", v.better);
        let _ = writeln!(content, "    status: {}", variance_status(v));
        let _ = writeln!(content, "    exceeds_threshold: {}", v.exceeds_threshold);
    }

    let mut file = fs::File::create(output)
        .map_err(|e| ForgeError::Export(format!("Failed to create file: {e}")))?;
    file.write_all(content.as_bytes())
        .map_err(|e| ForgeError::Export(format!("Failed to write file: {e}")))?;

    Ok(())
}

#[cfg(test)]
mod direction_tests {
    use super::better_direction;
    use crate::types::Variable;

    fn plain() -> Variable {
        Variable::new("x".to_string(), Some(1.0), None)
    }

    #[test]
    fn the_group_decides_what_the_name_cannot() {
        let v = plain();
        for (name, expected) in [
            ("income.salary", "higher"),
            ("income.side_work", "higher"),
            ("income.total", "higher"),
            ("expenses.salary", "lower"),
            ("expenses.groceries", "lower"),
            ("costs.staff.wages", "lower"),
            ("revenue.interest", "higher"),
            ("payroll.salary", "lower"),
            ("assumptions.rent", "lower"),
            ("income.tax", "lower"),
            ("expenses.sales_commission", "lower"),
            ("assumptions.growth", "unknown"),
        ] {
            assert_eq!(better_direction(name, &v, &v), expected, "{name}");
        }
        let mut declared = plain();
        declared.metadata.better = Some("lower".to_string());
        assert_eq!(better_direction("income.salary", &declared, &v), "lower");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod not_meaningful {
        //! A variance percentage that means nothing is n/m (T2-VARIANCE-NM): on a
        //! zero budget, or when the budget and the actual have opposite signs. Only
        //! the text changes: the values are the same.

        use super::*;
        use crate::cli::forge_at_width;
        use std::path::Path;

        const PLAN: &str = "examples/bagel/bagel.yaml";
        const ACTUAL: &str = "examples/bagel/actual.yaml";

        /// The lines `forge` prints on a wide terminal, words separated by single
        /// spaces, so column widths do not matter
        fn lines(args: &[&str]) -> Vec<String> {
            let ran = forge_at_width(Some(200), args);
            assert!(ran.success(), "{}", ran.err);
            ran.out
                .lines()
                .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
                .collect()
        }

        fn has(lines: &[String], wanted: &str) -> bool {
            lines.iter().any(|l| l == wanted)
        }

        #[test]
        fn a_percentage_is_not_meaningful_on_a_zero_budget_or_across_zero() {
            // Zero (or as good as zero) budget: any change is an infinite share of it
            assert!(!pct_is_meaningful(0.0, 5.0));
            assert!(!pct_is_meaningful(0.000_01, 1.0));
            // The sign flips: a profit that became a loss, and back
            assert!(!pct_is_meaningful(786.32, -28_313.87));
            assert!(!pct_is_meaningful(-100.0, 50.0));
            // Same side of zero, or down to exactly zero (-100%): meaningful
            assert!(pct_is_meaningful(-95_579.52, -123_258.79));
            assert!(pct_is_meaningful(100.0, 0.0));
            assert!(pct_is_meaningful(4_800.0, 4_950.0));
        }

        /// The plan's profit is 786.32 and the actual -28,313.87: -3,700.8% says
        /// nothing, 29,100 worse says it all. The rows whose sign flipped are n/m and
        /// stay alerts; every other row keeps its percentage.
        #[test]
        fn bagel_variance_marks_the_rows_that_changed_sign() {
            let v = variance_core(Path::new(PLAN), Path::new(ACTUAL), 10.0).unwrap();
            let not_meaningful: Vec<&str> = v
                .results
                .iter()
                .filter(|r| !r.pct_meaningful)
                .map(|r| r.name.as_str())
                .collect();
            assert_eq!(
                not_meaningful,
                ["year.profit", "year.margin", "year.first_year_return"]
            );
            for name in &not_meaningful {
                let row = v.results.iter().find(|r| r.name == *name).unwrap();
                assert!(row.exceeds_threshold, "{name} is still an alert");
            }
            // Same five alerts as before the rule
            assert_eq!(v.alert_count, 5);
        }

        #[test]
        fn variance_prints_n_m() {
            let out = lines(&["variance", PLAN, ACTUAL]);
            assert!(
                has(&out, "year.profit 786.32 -28,313.87 -29,100.19 n/m ⚠️ ❌"),
                "{out:#?}"
            );
            assert!(
                has(
                    &out,
                    "year.npv_first_year -95,579.52 -123,258.79 -27,679.27 -29.0% ⚠️ ❌"
                ),
                "{out:#?}"
            );
        }
    }

    mod printed_numbers {
        //! `forge variance` prints the budget, the actual and the variance as `forge
        //! report` prints numbers (the end-to-end pass of 2026-11-12).

        use crate::cli::forge;

        #[test]
        fn variance_prints_budget_actual_and_variance_as_report_does() {
            let text = forge(&[
                "variance",
                "examples/bagel/bagel.yaml",
                "examples/bagel/actual.yaml",
            ])
            .printed();
            assert!(text.contains("assumptions.rent_per_month"), "{text}");
            for cell in ["4,800.00", "4,950.00", "150.00", "386,827.45", "-16,923.70"] {
                assert!(text.contains(cell), "{cell}\n{text}");
            }
            assert!(!text.contains("386827.449538"), "{text}");
        }
    }

    mod acquisition_example {
        //! examples/acquisition/actual.yaml: the deal underwritten again a year
        //! after closing. The `run` of the Python in prediction/scenarios.rs
        //! (`acquisition_example`) with the year's news,
        //! `run(growth=0.05, debt_rate=0.0685, cost=46, rev=35, integ=125)`: value
        //! created 112.05793522726526, standalone 1227.3807642324014, synergies
        //! 575.8771709948642. Half the value is gone, and most of it with
        //! Quillon's own growth, not with the synergies.

        use crate::cli::variance_core;
        use std::path::Path;

        #[test]
        fn half_the_value_is_gone_and_most_of_it_is_not_the_synergies() {
            let v = variance_core(
                Path::new("examples/acquisition/deal.yaml"),
                Path::new("examples/acquisition/actual.yaml"),
                10.0,
            )
            .unwrap();
            let get = |n: &str| v.results.iter().find(|r| r.name == n).unwrap();
            let close = |a: f64, b: f64| (a - b).abs() <= 1e-9;
            let value = get("valuation.value_created");
            assert!(
                close(value.actual, 112.057_935_227_265_26),
                "{}",
                value.actual
            );
            assert_eq!(value.is_favorable, Some(false));
            let standalone = get("valuation.standalone");
            assert!(close(standalone.actual, 1_227.380_764_232_401_4));
            let synergies = get("valuation.synergies");
            assert!(close(synergies.actual, 575.877_170_994_864_2));
            assert!(standalone.variance < 3.0 * synergies.variance);
            // More cost out than planned; the rest of the news is bad
            assert_eq!(get("synergies.cost_run_rate").is_favorable, Some(true));
            for bad in [
                "synergies.revenue_run_rate",
                "synergies.integration_cost",
                "case.quillon_growth",
                "financing.debt_rate",
            ] {
                assert_eq!(get(bad).is_favorable, Some(false), "{bad}");
            }
            assert!(v.not_compared.is_empty(), "{:?}", v.not_compared);
        }
    }

    mod reports {
        //! The terminal summary, the Excel and YAML reports, and the lines that
        //! are not compared, on two small models.

        use super::*;
        use crate::cli::console::capture;

        const BUDGET: &str = "_forge_version: \"5.4.0\"\nsales: 100\ncosts: 50\nmystery: 10\nzero: 0\nonly_budget: 1\nwhen: {formula: \"=DATE(2026, 1, 1)\"}\n";
        const ACTUAL: &str = "_forge_version: \"5.4.0\"\nsales: 150\ncosts: 70\nmystery: 12\nzero: 5\nonly_actual: 2\nwhen: {formula: \"=DATE(2026, 2, 1)\"}\n";

        fn files(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
            let (budget, actual) = (dir.join("budget.yaml"), dir.join("actual.yaml"));
            fs::write(&budget, BUDGET).unwrap();
            fs::write(&actual, ACTUAL).unwrap();
            (budget, actual)
        }

        fn run(
            dir: &Path,
            output: Option<&Path>,
            flags: (bool, bool),
        ) -> (ForgeResult<()>, String) {
            let (budget, actual) = files(dir);
            let (result, captured) =
                capture(|c| variance(c, &budget, &actual, 10.0, output, flags));
            (result, captured.out)
        }

        #[test]
        fn the_terminal_report_says_what_is_not_compared_and_has_no_direction() {
            let dir = tempfile::tempdir().unwrap();
            let (result, out) = run(dir.path(), None, (false, true));
            result.unwrap();
            for line in [
                "Parsing YAML files",
                "Calculating formulas",
                "Not compared (missing or empty in one model, or a date):",
                "add `better: lower`",
                "only_budget",
                "only_actual",
                "when",
            ] {
                assert!(out.contains(line), "{line} in {out}");
            }
        }

        #[test]
        fn an_alert_fails_the_command_when_asked_to() {
            let dir = tempfile::tempdir().unwrap();
            let (result, _) = run(dir.path(), None, (true, false));
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("exceed the 10% threshold"));
        }

        #[test]
        fn the_numbers_of_a_line_come_out_as_the_budget_and_the_actual_say() {
            let dir = tempfile::tempdir().unwrap();
            let (budget, actual) = files(dir.path());
            let found = variance_core(&budget, &actual, 10.0).unwrap();
            let line = |name: &str| found.results.iter().find(|r| r.name == name).unwrap();
            assert_exact!(line("sales").variance, 50.0);
            assert_eq!(line("sales").is_favorable, Some(true));
            assert_eq!(line("costs").is_favorable, Some(false));
            // A zero budget has no percentage, and a change from it is an alert
            assert_exact!(line("zero").variance_pct, 0.0);
            assert!(!line("zero").pct_meaningful && line("zero").exceeds_threshold);
            assert_eq!(line("mystery").is_favorable, None);
            assert_eq!(found.not_compared, ["only_budget", "when", "only_actual"]);
        }

        #[test]
        fn a_report_is_written_as_excel_or_yaml_and_other_files_are_refused() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = dir.path().join("report.xlsx");
            let (result, out) = run(dir.path(), Some(&xlsx), (false, false));
            result.unwrap();
            assert!(out.contains("Variance report exported to"), "{out}");
            let mut book: calamine::Xlsx<_> = calamine::open_workbook(&xlsx).unwrap();
            let sheet = calamine::Reader::worksheet_range(&mut book, "Sheet1").unwrap();
            let text = |row, col| sheet.get((row, col)).map(ToString::to_string);
            assert_eq!(text(0, 0).as_deref(), Some("Variable"));
            assert_eq!(text(0, 5).as_deref(), Some("Status"));
            // Rows follow the order of the file: sales first
            assert_eq!(text(1, 0).as_deref(), Some("sales"));
            assert!((0..8).any(|row| text(row, 4).as_deref() == Some("n/m")));
            let last = sheet.height() - 1;
            assert!(text(last, 0).unwrap().starts_with("Generated by Forge v"));
            assert_eq!(text(last - 1, 0).as_deref(), Some("Threshold: 10%"));

            let yaml = dir.path().join("report.yml");
            let (result, _) = run(dir.path(), Some(&yaml), (false, false));
            result.unwrap();
            let report = fs::read_to_string(&yaml).unwrap();
            assert!(report.contains("threshold_pct: 10"), "{report}");

            let (result, _) = run(
                dir.path(),
                Some(&dir.path().join("report.csv")),
                (false, false),
            );
            let message = result.unwrap_err().to_string();
            assert!(
                message.contains("Unsupported output format: csv"),
                "{message}"
            );
            let (result, _) = run(dir.path(), Some(&dir.path().join("report")), (false, false));
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Unsupported output format: ."));
        }

        #[test]
        fn a_report_that_cannot_be_written_is_an_error() {
            let dir = tempfile::tempdir().unwrap();
            let elsewhere = dir.path().join("absent").join("report.xlsx");
            let (result, _) = run(dir.path(), Some(&elsewhere), (false, false));
            assert!(result.is_err());
        }

        #[test]
        fn each_status_word_names_the_direction_and_the_alert() {
            let line = |actual, exceeds, favorable| VarianceResult {
                name: "x".to_string(),
                budget: 1.0,
                actual,
                variance: actual - 1.0,
                variance_pct: 0.0,
                pct_meaningful: true,
                is_favorable: favorable,
                better: "unknown",
                exceeds_threshold: exceeds,
            };
            assert_eq!(variance_status(&line(1.0, false, None)), "On target");
            assert_eq!(
                variance_status(&line(2.0, true, Some(false))),
                "ALERT - Unfavorable"
            );
            assert_eq!(
                variance_status(&line(2.0, true, Some(true))),
                "ALERT - Favorable"
            );
            assert_eq!(variance_status(&line(2.0, true, None)), "ALERT - Changed");
            assert_eq!(variance_status(&line(2.0, false, Some(true))), "Favorable");
            assert_eq!(
                variance_status(&line(2.0, false, Some(false))),
                "Unfavorable"
            );
            assert_eq!(variance_status(&line(2.0, false, None)), "No direction");
        }
    }
}
