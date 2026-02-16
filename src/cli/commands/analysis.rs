//! Analysis commands: variance, sensitivity, `goal_seek`, `break_even`, compare

use crate::core::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use super::{apply_scenario, format_number};

/// Variance result for a single variable
#[derive(Debug, Clone)]
pub struct VarianceResult {
    pub name: String,
    pub budget: f64,
    pub actual: f64,
    pub variance: f64,
    pub variance_pct: f64,
    pub is_favorable: bool,
    pub exceeds_threshold: bool,
}

/// Execute the compare command - compare results across scenarios
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, a scenario does not exist,
/// or calculation fails.
pub fn compare(file: &Path, scenarios: &[String], verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Scenario Comparison".bold().green());
    println!("   File: {}", file.display());
    println!(
        "   Scenarios: {}\n",
        scenarios.join(", ").bright_yellow().bold()
    );

    // Parse model
    let base_model = parser::parse_model(file)?;

    // Validate scenarios exist
    for scenario_name in scenarios {
        if !base_model.scenarios.contains_key(scenario_name) {
            let available: Vec<_> = base_model.scenarios.keys().collect();
            return Err(ForgeError::Validation(format!(
                "Scenario '{scenario_name}' not found. Available: {available:?}"
            )));
        }
    }

    if verbose {
        println!(
            "   Found {} tables, {} scalars, {} scenarios",
            base_model.tables.len(),
            base_model.scalars.len(),
            base_model.scenarios.len()
        );
    }

    // Calculate results for each scenario
    let mut results: Vec<(String, crate::types::ParsedModel)> = Vec::new();

    for scenario_name in scenarios {
        let mut model = base_model.clone();
        apply_scenario(&mut model, scenario_name)?;

        let calculator = ArrayCalculator::new(model);
        let calculated = calculator.calculate_all()?;
        results.push((scenario_name.clone(), calculated));
    }

    // Collect all scalar names
    let mut all_scalars: Vec<String> = results
        .iter()
        .flat_map(|(_, m)| m.scalars.keys().cloned())
        .collect();
    all_scalars.sort();
    all_scalars.dedup();

    // Print comparison table
    println!("\n{}", "📊 Scenario Comparison:".bold().cyan());
    println!("{}", "─".repeat(20 + scenarios.len() * 15));

    // Header row
    print!("{:<20}", "Variable".bold());
    for scenario_name in scenarios {
        print!("{:>15}", scenario_name.bright_yellow().bold());
    }
    println!();
    println!("{}", "─".repeat(20 + scenarios.len() * 15));

    // Data rows
    for scalar_name in &all_scalars {
        print!("{:<20}", scalar_name.bright_blue());

        for (_, result_model) in &results {
            if let Some(var) = result_model.scalars.get(scalar_name) {
                if let Some(value) = var.value {
                    print!("{:>15}", format_number(value).green());
                } else {
                    print!("{:>15}", "-".dimmed());
                }
            } else {
                print!("{:>15}", "-".dimmed());
            }
        }
        println!();
    }

    println!("{}", "─".repeat(20 + scenarios.len() * 15));
    println!("\n{}", "✅ Comparison complete".bold().green());

    Ok(())
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

    let variances = collect_variances(&budget_result, &actual_result, threshold);

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
    let favorable_count = variances.iter().filter(|v| v.is_favorable).count();
    let unfavorable_count = variances.len() - favorable_count;
    let alert_count = variances.iter().filter(|v| v.exceeds_threshold).count();

    println!();
    println!(
        "   {} Favorable: {}  {} Unfavorable: {}  {} Alerts (>{:.0}%): {}",
        "✅".green(),
        favorable_count.to_string().green(),
        "❌".red(),
        unfavorable_count.to_string().red(),
        "⚠️".yellow(),
        threshold,
        alert_count.to_string().yellow()
    );

    Ok(())
}

/// Collect scalar variances between budget and actual calculation results
fn collect_variances(
    budget_result: &crate::types::ParsedModel,
    actual_result: &crate::types::ParsedModel,
    threshold: f64,
) -> Vec<VarianceResult> {
    let mut all_scalars: Vec<String> = budget_result
        .scalars
        .keys()
        .chain(actual_result.scalars.keys())
        .cloned()
        .collect();
    all_scalars.sort();
    all_scalars.dedup();

    let mut variances = Vec::new();
    for name in &all_scalars {
        let budget_val = budget_result
            .scalars
            .get(name)
            .and_then(|v| v.value)
            .unwrap_or(0.0);
        let actual_val = actual_result
            .scalars
            .get(name)
            .and_then(|v| v.value)
            .unwrap_or(0.0);

        let variance_abs = actual_val - budget_val;
        let variance_pct = if budget_val.abs() > 0.0001 {
            (variance_abs / budget_val) * 100.0
        } else {
            0.0
        };

        // Determine favorability (heuristic based on name)
        let is_expense = name.to_lowercase().contains("expense")
            || name.to_lowercase().contains("cost")
            || name.to_lowercase().contains("cogs");
        let is_favorable = if is_expense {
            actual_val <= budget_val // Lower expenses = favorable
        } else {
            actual_val >= budget_val // Higher revenue/profit = favorable
        };

        let exceeds_threshold = variance_pct.abs() >= threshold;

        variances.push(VarianceResult {
            name: name.clone(),
            budget: budget_val,
            actual: actual_val,
            variance: variance_abs,
            variance_pct,
            is_favorable,
            exceeds_threshold,
        });
    }
    variances
}

/// Print variance results as a table
pub fn print_variance_table(variances: &[VarianceResult], threshold: f64) {
    println!("\n{}", "📊 Budget vs Actual Variance:".bold().cyan());
    println!("{}", "─".repeat(85));

    // Header
    println!(
        "{:<20} {:>12} {:>12} {:>12} {:>10} {:>8}",
        "Variable".bold(),
        "Budget".bold(),
        "Actual".bold(),
        "Variance".bold(),
        "Var %".bold(),
        "Status".bold()
    );
    println!("{}", "─".repeat(85));

    // Data rows
    for v in variances {
        let var_str = format_number(v.variance);
        let pct_str = format!("{:.1}%", v.variance_pct);

        let status = if v.exceeds_threshold && !v.is_favorable {
            "⚠️ ❌".to_string()
        } else if v.exceeds_threshold {
            "⚠️ ✅".to_string()
        } else if v.is_favorable {
            "✅".to_string()
        } else {
            "❌".to_string()
        };

        // Color the variance based on favorability
        let var_colored = if v.is_favorable {
            var_str.green()
        } else {
            var_str.red()
        };
        let pct_colored = if v.is_favorable {
            pct_str.green()
        } else {
            pct_str.red()
        };

        println!(
            "{:<20} {:>12} {:>12} {:>12} {:>10} {:>8}",
            v.name.bright_blue(),
            format_number(v.budget),
            format_number(v.actual),
            var_colored,
            pct_colored,
            status
        );
    }

    println!("{}", "─".repeat(85));
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

        let status = if v.exceeds_threshold && !v.is_favorable {
            "ALERT - Unfavorable"
        } else if v.exceeds_threshold {
            "ALERT - Favorable"
        } else if v.is_favorable {
            "Favorable"
        } else {
            "Unfavorable"
        };
        worksheet.write_string(row, 5, status).ok();
    }

    // Add metadata row
    // Truncation impossible: variance reports have far fewer than u32::MAX rows
    #[allow(clippy::cast_possible_truncation)]
    let meta_row = (variances.len() + 3) as u32;
    worksheet
        .write_string(meta_row, 0, format!("Threshold: {threshold}%"))
        .ok();
    worksheet
        .write_string(meta_row + 1, 0, "Generated by Forge v2.3.0")
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
    content.push_str("# Generated by Forge v2.3.0\n");
    let _ = writeln!(content, "# Threshold: {threshold}%\n");

    content.push_str("metadata:\n");
    let _ = writeln!(content, "  threshold_pct: {threshold}");
    let _ = writeln!(content, "  total_items: {}", variances.len());
    let _ = writeln!(
        content,
        "  favorable_count: {}",
        variances.iter().filter(|v| v.is_favorable).count()
    );
    let _ = writeln!(
        content,
        "  alert_count: {}\n",
        variances.iter().filter(|v| v.exceeds_threshold).count()
    );

    content.push_str("variances:\n");
    for v in variances {
        let _ = writeln!(content, "  {}:", v.name);
        let _ = writeln!(content, "    budget: {}", v.budget);
        let _ = writeln!(content, "    actual: {}", v.actual);
        let _ = writeln!(content, "    variance: {}", v.variance);
        let _ = writeln!(content, "    variance_pct: {:.2}", v.variance_pct);
        let _ = writeln!(content, "    is_favorable: {}", v.is_favorable);
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
        println!("{}", "─".repeat(30));
        println!("{:>12} {:>15}", vary.bold(), output.bold());
        println!("{}", "─".repeat(30));

        for val in &values1 {
            match calculate_with_override(&base_model, vary, *val, output) {
                Ok(result) => {
                    println!(
                        "{:>12} {:>15}",
                        format!("{val:.4}").bright_yellow(),
                        format_number(result).green()
                    );
                },
                Err(e) => {
                    println!(
                        "{:>12} {:>15}",
                        format!("{val:.4}").bright_yellow(),
                        format!("ERR: {e}").red()
                    );
                },
            }
        }
        println!("{}", "─".repeat(30));
    }

    println!("\n{}", "✅ Sensitivity analysis complete".bold().green());
    Ok(())
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
    let (min, max) = bounds;
    println!("{}", "🔥 Forge - Goal Seek".bold().green());
    println!("   File: {}", file.display());
    println!("   Target: {} = {}", target.bright_blue(), value);
    println!("   Vary: {}", vary.bright_yellow());
    println!("   Tolerance: {tolerance}\n");

    // Parse model
    let base_model = parser::parse_model(file)?;

    // Validate variables
    if !base_model.scalars.contains_key(vary) {
        return Err(ForgeError::Validation(format!(
            "Variable '{}' not found. Available scalars: {:?}",
            vary,
            base_model.scalars.keys().collect::<Vec<_>>()
        )));
    }

    // Get current value of vary to set default bounds
    let current_value = base_model
        .scalars
        .get(vary)
        .and_then(|s| s.value)
        .unwrap_or(1.0);

    // Set bounds (default: 0.01x to 100x current value)
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

    if verbose {
        println!("   Current value of {vary}: {current_value}");
        println!("   Search bounds: [{lower}, {upper}]");
    }

    // Bisection method
    let max_iterations = 100;
    let mut low = lower;
    let mut high = upper;

    // Check bounds first
    let f_low = calculate_with_override(&base_model, vary, low, target)? - value;
    let f_high = calculate_with_override(&base_model, vary, high, target)? - value;

    if verbose {
        println!("   f({}) = {} (target diff: {})", low, f_low + value, f_low);
        println!(
            "   f({}) = {} (target diff: {})",
            high,
            f_high + value,
            f_high
        );
    }

    // Check if solution exists in range (signs should differ)
    if f_low * f_high > 0.0 {
        let expanded =
            expand_search_range(&base_model, vary, target, value, lower, upper, verbose)?;
        low = expanded.0;
        high = expanded.1;
    }

    // Bisection iteration
    let mut mid = f64::midpoint(low, high);
    let mut iteration = 0;

    while (high - low) > tolerance && iteration < max_iterations {
        mid = f64::midpoint(low, high);
        let f_mid = calculate_with_override(&base_model, vary, mid, target)? - value;

        if verbose && iteration % 10 == 0 {
            println!(
                "   Iteration {}: {} = {} (diff: {:.6})",
                iteration,
                vary,
                mid,
                f_mid.abs()
            );
        }

        let f_low_check = calculate_with_override(&base_model, vary, low, target)? - value;

        if f_mid.abs() < tolerance {
            break;
        }

        if f_low_check * f_mid < 0.0 {
            high = mid;
        } else {
            low = mid;
        }

        iteration += 1;
    }

    // Final result
    let final_value = calculate_with_override(&base_model, vary, mid, target)?;
    print_goal_seek_result(vary, target, mid, final_value, value, tolerance, iteration);
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

    // Header row
    print!("{:>12}", vary.bright_yellow());
    for val2 in &values2 {
        print!("{:>12}", format!("{val2:.4}").dimmed());
    }
    println!();
    println!("{}", "─".repeat(12 + values2.len() * 12));

    // Data rows
    for val1 in values1 {
        print!("{:>12}", format!("{val1:.4}").bright_yellow());

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
            match calculator.calculate_all() {
                Ok(result) => {
                    if let Some(scalar) = result.scalars.get(output) {
                        if let Some(v) = scalar.value {
                            print!("{:>12}", format_number(v).green());
                        } else {
                            print!("{:>12}", "-".dimmed());
                        }
                    } else {
                        print!("{:>12}", "?".red());
                    }
                },
                Err(_) => {
                    print!("{:>12}", "ERR".red());
                },
            }
        }
        println!();
    }
    Ok(())
}

/// Expand the search range when initial bounds have the same sign
fn expand_search_range(
    base_model: &crate::types::ParsedModel,
    vary: &str,
    target: &str,
    value: f64,
    lower: f64,
    upper: f64,
    verbose: bool,
) -> ForgeResult<(f64, f64)> {
    println!(
        "{}",
        "⚠️  No sign change in initial range - expanding search...".yellow()
    );

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
            if verbose {
                println!("   Found valid range: [{exp_low}, {exp_high}]");
            }
            return Ok((exp_low, exp_high));
        }
    }

    Err(ForgeError::Validation(format!(
        "No solution found in search range. The target value {value} may not be achievable by varying '{vary}'."
    )))
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
        format_number(mid).bold().green(),
        target.bright_blue(),
        format_number(final_value).green()
    );

    let error = (final_value - value).abs();
    if error < tolerance {
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
    println!("   Finding where {} = 0\n", output.bright_blue());

    // Break-even is just goal-seek with value = 0
    goal_seek(file, output, 0.0, vary, (min, max), 0.0001, verbose)
}
