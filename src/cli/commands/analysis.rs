//! Analysis commands: variance, sensitivity, `goal_seek`, `break_even`, compare

use crate::core::precision::to_15_digits;
use crate::core::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use super::{apply_scenario, display_number};

/// Variance result for a single variable
#[derive(Debug, Clone)]
pub struct VarianceResult {
    pub name: String,
    pub budget: f64,
    pub actual: f64,
    pub variance: f64,
    pub variance_pct: f64,
    /// `None` when Forge cannot tell whether higher or lower is better
    pub is_favorable: Option<bool>,
    /// "higher", "lower" or "unknown": which way is good, and so how
    /// `is_favorable` was decided
    pub better: &'static str,
    pub exceeds_threshold: bool,
}

/// Compare scenarios and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, a scenario does not exist,
/// or calculation fails.
pub fn compare_core(
    file: &Path,
    scenarios: &[String],
) -> ForgeResult<super::results::ComparisonResult> {
    let base_model = parser::parse_model(file)?;

    for scenario_name in scenarios {
        if !base_model.scenarios.contains_key(scenario_name) {
            let available: Vec<_> = base_model.scenarios.keys().collect();
            return Err(ForgeError::Validation(format!(
                "Scenario '{scenario_name}' not found. Available: {available:?}"
            )));
        }
    }

    let mut results: Vec<(String, crate::types::ParsedModel)> = Vec::new();
    for scenario_name in scenarios {
        let mut model = base_model.clone();
        apply_scenario(&mut model, scenario_name)?;
        let calculator = ArrayCalculator::new(model);
        let calculated = calculator.calculate_all()?;
        results.push((scenario_name.clone(), calculated));
    }

    // Model order, the same on every run
    let mut all_scalars: Vec<String> = Vec::new();
    for name in results.iter().flat_map(|(_, m)| m.scalars.keys()) {
        if !all_scalars.contains(name) {
            all_scalars.push(name.clone());
        }
    }

    let mut values = indexmap::IndexMap::new();
    let mut dates = std::collections::BTreeSet::new();
    for scalar_name in &all_scalars {
        let mut scenario_values = indexmap::IndexMap::new();
        for (scenario_name, result_model) in &results {
            let scalar = result_model.scalars.get(scalar_name);
            if scalar.is_some_and(|v| v.is_date) {
                dates.insert(scalar_name.clone());
            }
            scenario_values.insert(scenario_name.clone(), scalar.and_then(|v| v.value));
        }
        values.insert(scalar_name.clone(), scenario_values);
    }

    let descriptions = scenarios
        .iter()
        .filter_map(|name| {
            let text = base_model.scenarios.get(name)?.description.clone()?;
            Some((name.clone(), text))
        })
        .collect();

    Ok(super::results::ComparisonResult {
        scenarios: scenarios.to_vec(),
        variables: all_scalars,
        values,
        descriptions,
        dates,
    })
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
) -> ForgeResult<super::results::VarianceAnalysis> {
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
        .map(|v| super::results::VarianceEntry {
            name: v.name,
            budget: v.budget,
            actual: v.actual,
            variance: v.variance,
            variance_pct: v.variance_pct,
            is_favorable: v.is_favorable,
            better: v.better.to_string(),
            exceeds_threshold: v.exceeds_threshold,
        })
        .collect();

    Ok(super::results::VarianceAnalysis {
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
    budget_path: &Path,
    actual_path: &Path,
    threshold: f64,
    output: Option<&Path>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Variance Analysis".bold().green());
    println!("   Budget: {}", budget_path.display());
    println!("   Actual: {}", actual_path.display());
    println!("   Threshold: {threshold}%\n");

    // Parse both files
    if verbose {
        println!("{}", "📖 Parsing YAML files...".cyan());
    }

    let budget_model = parser::parse_model(budget_path)?;
    let actual_model = parser::parse_model(actual_path)?;

    // Calculate both models
    if verbose {
        println!("{}", "🧮 Calculating formulas...".cyan());
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
                println!(
                    "{}",
                    format!("✅ Variance report exported to {}", output_path.display())
                        .bold()
                        .green()
                );
            },
            "yaml" | "yml" => {
                export_variance_to_yaml(output_path, &variances, threshold)?;
                println!(
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
        print_variance_table(&variances, threshold);
    }

    // Summary
    let count = |f: Option<bool>| {
        variances
            .iter()
            .filter(|v| !on_target(v) && v.is_favorable == f)
            .count()
    };
    let alert_count = variances.iter().filter(|v| v.exceeds_threshold).count();

    println!();
    println!(
        "   {} Favorable: {}  {} Unfavorable: {}  · No direction: {}  = On target: {}  {} Alerts (>{:.0}%): {}",
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
        println!(
            "   For a scalar with no direction, add `better: lower` (costs) or `better: higher` to it."
        );
    }
    if !not_compared.is_empty() {
        println!(
            "   Not compared (missing or empty in one model, or a date): {}",
            not_compared.join(", ")
        );
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

        let better = better_direction(name, budget_scalar, actual_scalar);
        let is_favorable = match better {
            "lower" => Some(actual_val <= budget_val),
            "higher" => Some(actual_val >= budget_val),
            _ => None,
        };

        let exceeds_threshold = variance_pct.abs() >= threshold;

        variances.push(VarianceResult {
            name: name.clone(),
            budget: budget_val,
            actual: actual_val,
            variance: variance_abs,
            variance_pct,
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

#[allow(clippy::float_cmp)] // exactly on budget, not "close"
fn on_target(v: &VarianceResult) -> bool {
    v.actual == v.budget
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
pub fn print_variance_table(variances: &[VarianceResult], threshold: f64) {
    // As wide as the longest name, so every column lines up
    let w = variances
        .iter()
        .map(|v| v.name.chars().count())
        .max()
        .unwrap_or(0)
        .max(20);
    let rule = "─".repeat(w + 65);
    println!("\n{}", "📊 Budget vs Actual Variance:".bold().cyan());
    println!("{rule}");

    // Header
    println!(
        "{:<w$} {:>12} {:>12} {:>12} {:>10} {:>8}",
        "Variable".bold(),
        "Budget".bold(),
        "Actual".bold(),
        "Variance".bold(),
        "Var %".bold(),
        "Status".bold()
    );
    println!("{rule}");

    // Data rows
    for v in variances {
        let var_str = display_number(v.variance);
        let pct_str = format!("{:.1}%", v.variance_pct);

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

        println!(
            "{:<w$} {:>12} {:>12} {:>12} {:>10} {:>8}",
            v.name.bright_blue(),
            display_number(v.budget),
            display_number(v.actual),
            var_colored,
            pct_colored,
            status
        );
    }

    println!("{rule}");
    println!("   {} = exceeds {:.0}% threshold", "⚠️".yellow(), threshold);
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
    for (i, v) in variances.iter().enumerate() {
        // Truncation impossible: variance reports have far fewer than u32::MAX rows
        #[allow(clippy::cast_possible_truncation)]
        let row = (i + 1) as u32;

        worksheet.write_string(row, 0, &v.name).ok();
        worksheet.write_number(row, 1, v.budget).ok();
        worksheet.write_number(row, 2, v.actual).ok();
        worksheet.write_number(row, 3, v.variance).ok();
        worksheet.write_number(row, 4, v.variance_pct / 100.0).ok(); // As decimal for %

        worksheet.write_string(row, 5, variance_status(v)).ok();
    }

    // Add metadata row
    // Truncation impossible: variance reports have far fewer than u32::MAX rows
    #[allow(clippy::cast_possible_truncation)]
    let meta_row = (variances.len() + 3) as u32;
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

/// Parse a range string "start,end,step" into a vector of values
///
/// # Errors
///
/// Returns an error if the range format is invalid, values cannot be parsed,
/// step is non-positive, or start exceeds end.
pub fn parse_range(range: &str) -> ForgeResult<Vec<f64>> {
    let parts: Vec<&str> = range.split(',').collect();
    if parts.len() != 3 {
        return Err(ForgeError::Validation(format!(
            "Invalid range format '{range}'. Expected: start,end,step (e.g., 0.01,0.15,0.02)"
        )));
    }

    let start: f64 = parts[0]
        .trim()
        .parse()
        .map_err(|_| ForgeError::Validation(format!("Invalid start value: '{}'", parts[0])))?;
    let end: f64 = parts[1]
        .trim()
        .parse()
        .map_err(|_| ForgeError::Validation(format!("Invalid end value: '{}'", parts[1])))?;
    let step: f64 = parts[2]
        .trim()
        .parse()
        .map_err(|_| ForgeError::Validation(format!("Invalid step value: '{}'", parts[2])))?;

    if step <= 0.0 {
        return Err(ForgeError::Validation("Step must be positive".to_string()));
    }
    if start > end {
        return Err(ForgeError::Validation(
            "Start must be less than or equal to end".to_string(),
        ));
    }

    // Use integer step count to avoid while-float comparison
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // step count is always a small positive number
    let num_steps = ((end - start) / step + 1.001).floor() as usize;
    #[allow(clippy::cast_precision_loss)] // step index is always small enough for f64
    let values: Vec<f64> = (0..num_steps)
        .map(|i| step.mul_add(i as f64, start))
        .collect();

    Ok(values)
}

/// Calculate model with a specific variable override and return the output value
///
/// # Errors
///
/// Returns an error if calculation fails or the output variable is not found.
pub fn calculate_with_override(
    base_model: &crate::types::ParsedModel,
    var_name: &str,
    var_value: f64,
    output_name: &str,
) -> ForgeResult<f64> {
    base_model
        .refuse_date_input(var_name, "this command")
        .map_err(ForgeError::Validation)?;
    let mut model = base_model.clone();

    // Override the variable
    if let Some(scalar) = model.scalars.get_mut(var_name) {
        scalar.value = Some(var_value);
        scalar.formula = None; // Clear formula since we're using override
    } else {
        // Create new scalar
        model.scalars.insert(
            var_name.to_string(),
            crate::types::Variable::new(var_name.to_string(), Some(var_value), None),
        );
    }

    // Calculate
    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all()?;

    // Get output value
    result.scalars.get(output_name).map_or_else(
        || {
            Err(ForgeError::Validation(format!(
                "Output variable '{output_name}' not found in model"
            )))
        },
        |scalar| {
            scalar.value.ok_or_else(|| {
                ForgeError::Validation(format!("Output variable '{output_name}' has no value"))
            })
        },
    )
}

/// Run sensitivity analysis and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// ranges are invalid, or calculation fails.
pub fn sensitivity_core(
    file: &Path,
    vary: &str,
    range: &str,
    vary2: Option<&str>,
    range2: Option<&str>,
    output: &str,
) -> ForgeResult<super::results::SensitivityResult> {
    let base_model = parser::parse_model(file)?;

    if !base_model.scalars.contains_key(vary) {
        return Err(ForgeError::Validation(format!(
            "Variable '{}' not found. Available scalars: {:?}",
            vary,
            base_model.scalars.keys().collect::<Vec<_>>()
        )));
    }
    for name in std::iter::once(vary).chain(vary2) {
        base_model
            .refuse_date_input(name, "sensitivity")
            .map_err(ForgeError::Validation)?;
    }

    let values1 = parse_range(range)?;

    let data = if let (Some(v2), Some(r2)) = (vary2, range2) {
        if !base_model.scalars.contains_key(v2) {
            return Err(ForgeError::Validation(format!(
                "Variable '{}' not found. Available scalars: {:?}",
                v2,
                base_model.scalars.keys().collect::<Vec<_>>()
            )));
        }
        let values2 = parse_range(r2)?;
        let mut matrix = Vec::new();
        for val1 in &values1 {
            let mut row = Vec::new();
            for val2 in &values2 {
                let mut model = base_model.clone();
                if let Some(s) = model.scalars.get_mut(vary) {
                    s.value = Some(*val1);
                    s.formula = None;
                }
                if let Some(s) = model.scalars.get_mut(v2) {
                    s.value = Some(*val2);
                    s.formula = None;
                }
                let calculator = ArrayCalculator::new(model);
                let cell = match calculator.calculate_all() {
                    Ok(result) => result.scalars.get(output).and_then(|s| s.value),
                    Err(_) => None,
                };
                row.push(cell);
            }
            matrix.push(row);
        }
        super::results::SensitivityData::TwoVar {
            vary2: v2.to_string(),
            row_values: values1,
            col_values: values2,
            matrix,
        }
    } else {
        let mut entries = Vec::new();
        for val in &values1 {
            match calculate_with_override(&base_model, vary, *val, output) {
                Ok(result) => entries.push(super::results::SensitivityEntry {
                    input: *val,
                    output: Some(result),
                    error: None,
                }),
                Err(e) => entries.push(super::results::SensitivityEntry {
                    input: *val,
                    output: None,
                    error: Some(e.to_string()),
                }),
            }
        }
        super::results::SensitivityData::OneVar { entries }
    };

    Ok(super::results::SensitivityResult {
        vary: vary.to_string(),
        output: output.to_string(),
        data,
    })
}

/// Execute the sensitivity command
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// ranges are invalid, or calculation fails.
pub fn sensitivity(
    file: &Path,
    vary: &str,
    range: &str,
    vary2: Option<&str>,
    range2: Option<&str>,
    output: &str,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Sensitivity Analysis".bold().green());
    println!("   File: {}", file.display());
    println!("   Vary: {} ({})", vary.bright_yellow(), range);
    if let Some(v2) = vary2 {
        println!(
            "   Vary2: {} ({})",
            v2.bright_yellow(),
            range2.unwrap_or("?")
        );
    }
    println!("   Output: {}\n", output.bright_blue());

    // Parse model
    let base_model = parser::parse_model(file)?;

    // Validate that vary variable exists
    if !base_model.scalars.contains_key(vary) {
        return Err(ForgeError::Validation(format!(
            "Variable '{}' not found. Available scalars: {:?}",
            vary,
            base_model.scalars.keys().collect::<Vec<_>>()
        )));
    }

    for name in std::iter::once(vary).chain(vary2) {
        base_model
            .refuse_date_input(name, "sensitivity")
            .map_err(ForgeError::Validation)?;
    }

    // Parse range
    let values1 = parse_range(range)?;

    if verbose {
        println!(
            "   Range 1: {} values from {} to {}",
            values1.len(),
            values1.first().unwrap_or(&0.0),
            values1.last().unwrap_or(&0.0)
        );
    }

    // Two-variable analysis
    if let (Some(v2), Some(r2)) = (vary2, range2) {
        run_two_var_sensitivity(&base_model, vary, v2, r2, output, &values1, verbose)?;
    } else {
        // One-variable analysis
        println!(
            "\n{} {} → {}",
            "📊 Sensitivity Table:".bold().cyan(),
            vary.yellow(),
            output.bright_blue()
        );
        // Wide enough for the names and the numbers, so the header lines up
        let rows: Vec<(String, Result<f64, String>)> = values1
            .iter()
            .map(|val| {
                (
                    format!("{val:.4}"),
                    calculate_with_override(&base_model, vary, *val, output)
                        .map_err(|e| e.to_string()),
                )
            })
            .collect();
        let shown = |result: &Result<f64, String>| match result {
            Ok(v) => display_number(*v),
            Err(e) => format!("ERR: {e}"),
        };
        let input_width = rows
            .iter()
            .map(|(input, _)| input.len())
            .chain([vary.chars().count()])
            .max()
            .unwrap_or(12);
        let output_width = rows
            .iter()
            .map(|(_, result)| shown(result).chars().count())
            .chain([output.chars().count()])
            .max()
            .unwrap_or(15);
        let rule = "─".repeat(input_width + 1 + output_width);
        println!("{rule}");
        println!(
            "{:>input_width$} {:>output_width$}",
            vary.bold(),
            output.bold()
        );
        println!("{rule}");
        for (input, result) in &rows {
            let text = shown(result);
            println!(
                "{:>input_width$} {:>output_width$}",
                input.bright_yellow(),
                if result.is_ok() {
                    text.green()
                } else {
                    text.red()
                }
            );
        }
        println!("{rule}");
    }

    println!("\n{}", "✅ Sensitivity analysis complete".bold().green());
    Ok(())
}

/// `forge sensitivity` in the format asked for: the table, or JSON for
/// programs that read it, with the numbers at their 15 significant digits
/// and not as the table shows them.
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// ranges are invalid, or calculation fails.
pub fn sensitivity_in_format(
    file: &Path,
    (vary, range): (&str, &str),
    (vary2, range2): (Option<&str>, Option<&str>),
    output: &str,
    (format, verbose): (&str, bool),
) -> ForgeResult<()> {
    if format != "json" {
        return sensitivity(file, vary, range, vary2, range2, output, verbose);
    }
    let result = sensitivity_core(file, vary, range, vary2, range2, output)?;
    let json = serde_json::to_string_pretty(&crate::core::precision::Rounded(&result))
        .map_err(|e| ForgeError::Validation(format!("Failed to write JSON: {e}")))?;
    println!("{json}");
    Ok(())
}

/// Run goal-seek and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// or no solution exists in the search range.
pub fn goal_seek_core(
    file: &Path,
    target: &str,
    value: f64,
    vary: &str,
    bounds: (Option<f64>, Option<f64>),
    tolerance: f64,
) -> ForgeResult<super::results::GoalSeekResult> {
    let (min, max) = bounds;
    let base_model = parser::parse_model(file)?;

    if !base_model.scalars.contains_key(vary) {
        return Err(ForgeError::Validation(format!(
            "Variable '{}' not found. Available scalars: {:?}",
            vary,
            base_model.scalars.keys().collect::<Vec<_>>()
        )));
    }

    let current_value = base_model
        .scalars
        .get(vary)
        .and_then(|s| s.value)
        .unwrap_or(1.0);

    let lower = min.unwrap_or_else(|| {
        if current_value > 0.0 {
            current_value * 0.01
        } else if current_value < 0.0 {
            current_value * 100.0
        } else {
            -1000.0
        }
    });
    let upper = max.unwrap_or(if current_value > 0.0 {
        current_value * 100.0
    } else if current_value < 0.0 {
        current_value * 0.01
    } else {
        1000.0
    });

    let mut low = lower;
    let mut high = upper;

    let f_low = calculate_with_override(&base_model, vary, low, target)? - value;
    let f_high = calculate_with_override(&base_model, vary, high, target)? - value;

    if f_low * f_high > 0.0 {
        let expanded = expand_search_range_quiet(&base_model, vary, target, value, lower, upper)?;
        low = expanded.0;
        high = expanded.1;
    }

    // Bisect until the output is within `tolerance` of the target, or the
    // interval cannot be split any further. `tolerance` is about the output,
    // so it is also what decides `converged`.
    let max_iterations = 200;
    let mut f_low = calculate_with_override(&base_model, vary, low, target)? - value;
    let mut mid = f64::midpoint(low, high);
    let mut iteration = 0;

    while iteration < max_iterations {
        mid = f64::midpoint(low, high);
        if mid <= low || mid >= high {
            break; // no float left between low and high
        }
        let f_mid = calculate_with_override(&base_model, vary, mid, target)? - value;
        iteration += 1;

        if f_mid.abs() <= tolerance {
            break;
        }

        if f_low * f_mid < 0.0 {
            high = mid;
        } else {
            low = mid;
            f_low = f_mid;
        }
    }

    let final_value = calculate_with_override(&base_model, vary, mid, target)?;
    let error = (final_value - value).abs();

    Ok(super::results::GoalSeekResult {
        vary: vary.to_string(),
        target: target.to_string(),
        target_value: value,
        solution: mid,
        achieved: final_value,
        error,
        iterations: iteration,
        converged: error <= tolerance,
    })
}

/// Expand search range without printing (for core function)
fn expand_search_range_quiet(
    base_model: &crate::types::ParsedModel,
    vary: &str,
    target: &str,
    value: f64,
    lower: f64,
    upper: f64,
) -> ForgeResult<(f64, f64)> {
    for factor in [10.0, 100.0, 1000.0] {
        let exp_low = if lower > 0.0 {
            lower / factor
        } else {
            lower * factor
        };
        let exp_high = if upper > 0.0 {
            upper * factor
        } else {
            upper / factor
        };

        let f_exp_low = calculate_with_override(base_model, vary, exp_low, target)? - value;
        let f_exp_high = calculate_with_override(base_model, vary, exp_high, target)? - value;

        if f_exp_low * f_exp_high <= 0.0 {
            return Ok((exp_low, exp_high));
        }
    }

    Err(ForgeError::Validation(format!(
        "No solution found in search range. The target value {value} may not be achievable by varying '{vary}'."
    )))
}

/// Execute the goal-seek command
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, variables are not found,
/// or no solution exists in the search range.
pub fn goal_seek(
    file: &Path,
    target: &str,
    value: f64,
    vary: &str,
    bounds: (Option<f64>, Option<f64>),
    tolerance: f64,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Goal Seek".bold().green());
    println!("   File: {}", file.display());
    println!("   Target: {} = {}", target.bright_blue(), value);
    println!("   Vary: {}", vary.bright_yellow());
    println!("   Tolerance: {tolerance}\n");
    solve(file, target, value, vary, bounds, tolerance, verbose)
}

/// Find the input, then print the solution (the part goal-seek and
/// break-even share, after their own headers).
fn solve(
    file: &Path,
    target: &str,
    value: f64,
    vary: &str,
    bounds: (Option<f64>, Option<f64>),
    tolerance: f64,
    verbose: bool,
) -> ForgeResult<()> {
    let result = goal_seek_core(file, target, value, vary, bounds, tolerance)?;
    if verbose {
        println!(
            "   Bisection stopped after {} evaluations, error {}",
            result.iterations, result.error
        );
    }
    print_goal_seek_result(
        vary,
        target,
        result.solution,
        result.achieved,
        value,
        tolerance,
        result.iterations,
    );
    Ok(())
}

/// Run two-variable sensitivity analysis matrix
fn run_two_var_sensitivity(
    base_model: &crate::types::ParsedModel,
    vary: &str,
    v2: &str,
    r2: &str,
    output: &str,
    values1: &[f64],
    verbose: bool,
) -> ForgeResult<()> {
    for name in [vary, v2] {
        base_model
            .refuse_date_input(name, "sensitivity")
            .map_err(ForgeError::Validation)?;
    }
    if !base_model.scalars.contains_key(v2) {
        return Err(ForgeError::Validation(format!(
            "Variable '{}' not found. Available scalars: {:?}",
            v2,
            base_model.scalars.keys().collect::<Vec<_>>()
        )));
    }

    let values2 = parse_range(r2)?;

    if verbose {
        println!(
            "   Range 2: {} values from {} to {}",
            values2.len(),
            values2.first().unwrap_or(&0.0),
            values2.last().unwrap_or(&0.0)
        );
    }

    // Calculate matrix
    println!(
        "\n{} {} → {}",
        "📊 Sensitivity Matrix:".bold().cyan(),
        format!("({vary}, {v2})").yellow(),
        output.bright_blue()
    );

    // Calculate every cell first, so the columns can be as wide as their numbers
    let cell = |val1: f64, val2: f64| {
        let mut model = base_model.clone();
        if let Some(s) = model.scalars.get_mut(vary) {
            s.value = Some(val1);
            s.formula = None;
        }
        if let Some(s) = model.scalars.get_mut(v2) {
            s.value = Some(val2);
            s.formula = None;
        }
        match ArrayCalculator::new(model).calculate_all() {
            Ok(result) => result.scalars.get(output).map_or_else(
                || "?".to_string(),
                |scalar| scalar.value.map_or_else(|| "-".to_string(), display_number),
            ),
            Err(_) => "ERR".to_string(),
        }
    };
    let rows: Vec<(String, Vec<String>)> = values1
        .iter()
        .map(|val1| {
            (
                format!("{val1:.4}"),
                values2.iter().map(|val2| cell(*val1, *val2)).collect(),
            )
        })
        .collect();
    let heads: Vec<String> = values2.iter().map(|val2| format!("{val2:.4}")).collect();
    let first_width = rows
        .iter()
        .map(|(input, _)| input.len())
        .chain([vary.chars().count()])
        .max()
        .unwrap_or(12);
    let width = rows
        .iter()
        .flat_map(|(_, cells)| cells.iter())
        .chain(heads.iter())
        .map(|text| text.chars().count())
        .max()
        .unwrap_or(0)
        .max(12)
        + 1;

    // Header row
    print!("{:>first_width$}", vary.bright_yellow());
    for head in &heads {
        print!("{:>width$}", head.dimmed());
    }
    println!();
    println!("{}", "─".repeat(first_width + heads.len() * width));

    // Data rows
    for (input, cells) in &rows {
        print!("{:>first_width$}", input.bright_yellow());
        for text in cells {
            match text.as_str() {
                "ERR" | "?" => print!("{:>width$}", text.red()),
                "-" => print!("{:>width$}", text.dimmed()),
                _ => print!("{:>width$}", text.green()),
            }
        }
        println!();
    }
    Ok(())
}

/// Print goal-seek result summary
fn print_goal_seek_result(
    vary: &str,
    target: &str,
    mid: f64,
    final_value: f64,
    value: f64,
    tolerance: f64,
    iteration: i32,
) {
    println!("{}", "─".repeat(50));
    println!(
        "{}",
        format!("🎯 Solution found in {iteration} iterations:")
            .bold()
            .green()
    );
    println!(
        "   {} = {} → {} = {}",
        vary.bright_yellow().bold(),
        display_number(mid).bold().green(),
        target.bright_blue(),
        display_number(final_value).green()
    );

    let error = (final_value - value).abs();
    if error <= tolerance {
        println!("   {} Within tolerance", "✅".green());
    } else {
        println!(
            "   {} Error: {} (tolerance: {})",
            "⚠️".yellow(),
            error,
            tolerance
        );
    }
    println!("{}", "─".repeat(50));
}

/// Execute the break-even command
///
/// # Errors
///
/// Returns an error if goal-seek fails to find a zero-crossing point.
pub fn break_even(
    file: &Path,
    output: &str,
    vary: &str,
    min: Option<f64>,
    max: Option<f64>,
    verbose: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Break-Even Analysis".bold().green());
    println!("   File: {}", file.display());
    println!("   Finding where {} = 0", output.bright_blue());
    println!("   Vary: {}", vary.bright_yellow());
    println!("   Tolerance: {BREAK_EVEN_TOLERANCE}\n");

    // Break-even is goal-seek with value = 0
    solve(
        file,
        output,
        0.0,
        vary,
        (min, max),
        BREAK_EVEN_TOLERANCE,
        verbose,
    )
}

/// How close to zero the output must be for `forge break-even`.
const BREAK_EVEN_TOLERANCE: f64 = 0.0001;

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
