//! `forge compare`: the scalars of a model under several scenarios, one row
//! per scalar and one column per scenario.
//!
//! The table sizes each column from what it holds and keeps two spaces between
//! columns. When it is wider than the terminal it is printed in blocks of
//! columns that fit, each block with the row names: one row per scalar, or one
//! row per scenario when that needs fewer blocks (long scenario names). The
//! numbers follow `forge report`'s rule, and so do the formats.

use super::apply_scenario;
use super::report::{
    csv_text, display_number, display_whole, markdown_text, shown_input, ReportFormat,
};
use super::results::ComparisonResult;
use crate::cli::console::{out, outln};
use crate::cli::Console;
use crate::core::precision::{to_15_digits, Rounded};
use crate::core::ArrayCalculator;
use crate::error::ForgeError;
use crate::error::ForgeResult;
use crate::parser;
use colored::Colorize;
use std::fmt::Write as _;
use std::path::Path;

/// The width of the terminal the output goes to: `COLUMNS` if it is set, the
/// terminal's own width otherwise, and `None` when the output is not a
/// terminal (a file or a pipe), which is never wrapped.
#[must_use]
pub fn terminal_width() -> Option<usize> {
    use std::io::IsTerminal as _;
    if let Some(columns) = std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.trim().parse::<usize>().ok())
        .filter(|&c| c > 0)
    {
        return Some(columns);
    }
    if !std::io::stdout().is_terminal() {
        return None;
    }
    Some(window_columns().unwrap_or(80))
}

#[cfg(unix)]
fn window_columns() -> Option<usize> {
    let mut size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: TIOCGWINSZ writes a `winsize` into the pointer it is given, which
    // points at a live, writable `winsize`; standard output is open.
    let ok = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &raw mut size) } == 0;
    (ok && size.ws_col > 0).then_some(usize::from(size.ws_col))
}

#[cfg(not(unix))]
const fn window_columns() -> Option<usize> {
    None
}

/// The cells of one row, as the table and Markdown show them: whole numbers
/// without decimals when every value of the row is whole, an input as written,
/// and `—` where a scenario has no value.
fn shown_row(values: &[Option<f64>], input: bool, date: bool, format: Option<&str>) -> Vec<String> {
    // A `format:` says how every value of the row is shown (ADR-061)
    if let Some(format) = format {
        let shown: Option<Vec<String>> = values
            .iter()
            .map(|v| {
                v.map_or_else(
                    || Some("—".to_string()),
                    |v| super::number_shown_by(v, format),
                )
            })
            .collect();
        if let Some(shown) = shown {
            return shown;
        }
    }
    if date {
        return values
            .iter()
            .map(|v| v.map_or_else(|| "—".to_string(), date_text))
            .collect();
    }
    let whole = values
        .iter()
        .flatten()
        .all(|v| !v.is_finite() || display_number(*v).ends_with(".00"));
    values
        .iter()
        .map(|value| {
            value.map_or_else(
                || "—".to_string(),
                |v| {
                    let shown = if whole {
                        display_whole(v)
                    } else {
                        display_number(v)
                    };
                    if input {
                        shown_input(v, shown)
                    } else {
                        shown
                    }
                },
            )
        })
        .collect()
}

/// The text of the date with day number `serial` (ADR-060).
fn date_text(serial: f64) -> String {
    crate::core::array_calculator::evaluator::serial_date_text(serial)
}

/// A comparison as JSON, a date's value as its text, as in `forge calculate`'s
/// JSON (ADR-060). `forge compare --format json` and the MCP tool share it.
#[must_use]
pub fn comparison_json(result: &ComparisonResult) -> serde_json::Value {
    let mut json = serde_json::to_value(Rounded(result)).unwrap_or_default();
    for variable in &result.dates {
        if let Some(row) = json["values"][variable].as_object_mut() {
            for value in row.values_mut() {
                if let Some(serial) = value.as_f64() {
                    *value = serde_json::Value::String(date_text(serial));
                }
            }
        }
    }
    json
}

fn row_values(result: &ComparisonResult, variable: &str) -> Vec<Option<f64>> {
    result
        .scenarios
        .iter()
        .map(|s| {
            result
                .values
                .get(variable)
                .and_then(|v| v.get(s))
                .copied()
                .flatten()
        })
        .collect()
}

/// The comparison in a format. `inputs` names the scalars written with no
/// formula, shown as written; `width` is the width to fit the table in
/// (`None`: no limit).
#[must_use]
pub fn render_comparison(
    result: &ComparisonResult,
    inputs: &[String],
    format: ReportFormat,
    width: Option<usize>,
) -> String {
    match format {
        ReportFormat::Table => format!(
            "{}{}",
            render_table(result, inputs, width),
            legend(result, false)
        ),
        ReportFormat::Markdown => {
            format!(
                "{}{}",
                render_markdown(result, inputs),
                legend(result, true)
            )
        },
        ReportFormat::Csv => render_csv(result),
        ReportFormat::Json => {
            let json = comparison_json(result);
            let mut out = serde_json::to_string_pretty(&json).unwrap_or_default();
            out.push('\n');
            out
        },
    }
}

fn render_table(result: &ComparisonResult, inputs: &[String], width: Option<usize>) -> String {
    let cells: Vec<Vec<String>> = result
        .variables
        .iter()
        .map(|v| {
            shown_row(
                &row_values(result, v),
                inputs.contains(v),
                result.dates.contains(v),
                result.formats.get(v).map(String::as_str),
            )
        })
        .collect();
    let by_variable = grid(
        "variable",
        &result.variables,
        &result.scenarios,
        &cells,
        width,
    );
    if width.is_none() || by_variable.len() == 1 {
        return by_variable.join("\n");
    }
    // One row per scenario, when that needs fewer blocks
    let transposed: Vec<Vec<String>> = (0..result.scenarios.len())
        .map(|s| cells.iter().map(|row| row[s].clone()).collect())
        .collect();
    let by_scenario = grid(
        "scenario",
        &result.scenarios,
        &result.variables,
        &transposed,
        width,
    );
    if by_scenario.len() < by_variable.len() {
        by_scenario.join("\n")
    } else {
        by_variable.join("\n")
    }
}

/// A table with row names on the left and `cells[row][column]` right-aligned,
/// as blocks of columns that fit in `width` (at least one column a block), each
/// block with the row names.
fn grid(
    corner: &str,
    rows: &[String],
    columns: &[String],
    cells: &[Vec<String>],
    width: Option<usize>,
) -> Vec<String> {
    let first = rows
        .iter()
        .map(|name| name.chars().count())
        .chain(std::iter::once(corner.chars().count()))
        .max()
        .unwrap_or(0);
    let widths: Vec<usize> = columns
        .iter()
        .enumerate()
        .map(|(c, name)| {
            cells
                .iter()
                .map(|row| row[c].chars().count())
                .chain(std::iter::once(name.chars().count()))
                .max()
                .unwrap_or(0)
        })
        .collect();

    let mut blocks: Vec<Vec<usize>> = Vec::new();
    let mut used = first;
    for (c, w) in widths.iter().enumerate() {
        let fits = width.is_none_or(|limit| used + 2 + w <= limit);
        match blocks.last_mut() {
            Some(block) if fits => block.push(c),
            _ => {
                blocks.push(vec![c]);
                used = first;
            },
        }
        used += 2 + w;
    }

    blocks
        .iter()
        .map(|block| {
            let mut out = String::new();
            let mut line = format!("{corner:<first$}");
            for &c in block {
                let w = widths[c];
                let _ = write!(line, "  {:>w$}", columns[c]);
            }
            out.push_str(line.trim_end());
            out.push('\n');
            for (name, row) in rows.iter().zip(cells) {
                let mut line = format!("{name:<first$}");
                for &c in block {
                    let w = widths[c];
                    let _ = write!(line, "  {:>w$}", row[c]);
                }
                out.push_str(line.trim_end());
                out.push('\n');
            }
            out
        })
        .collect()
}

/// The scenarios' descriptions, below the table: a blank line and one line per
/// scenario that has one.
fn legend(result: &ComparisonResult, markdown: bool) -> String {
    if result.descriptions.is_empty() {
        return String::new();
    }
    let mut out = String::from("\n");
    for scenario in &result.scenarios {
        if let Some(text) = result.descriptions.get(scenario) {
            if markdown {
                let _ = writeln!(out, "- **{}**: {text}", markdown_text(scenario));
            } else {
                let _ = writeln!(out, "{scenario}: {text}");
            }
        }
    }
    out
}

fn render_markdown(result: &ComparisonResult, inputs: &[String]) -> String {
    let mut out = String::new();
    let header: Vec<String> = std::iter::once("variable".to_string())
        .chain(result.scenarios.iter().map(|s| markdown_text(s)))
        .collect();
    let _ = writeln!(out, "| {} |", header.join(" | "));
    let rule: Vec<&str> = std::iter::once("---")
        .chain(result.scenarios.iter().map(|_| "---:"))
        .collect();
    let _ = writeln!(out, "| {} |", rule.join(" | "));
    for variable in &result.variables {
        let cells = shown_row(
            &row_values(result, variable),
            inputs.contains(variable),
            result.dates.contains(variable),
            result.formats.get(variable).map(String::as_str),
        );
        let items: Vec<String> = std::iter::once(markdown_text(variable))
            .chain(cells)
            .collect();
        let _ = writeln!(out, "| {} |", items.join(" | "));
    }
    out
}

fn render_csv(result: &ComparisonResult) -> String {
    let mut out = String::new();
    let header: Vec<String> = std::iter::once("variable".to_string())
        .chain(result.scenarios.iter().map(|s| csv_text(s)))
        .collect();
    out.push_str(&header.join(","));
    out.push('\n');
    for variable in &result.variables {
        let items: Vec<String> = std::iter::once(csv_text(variable))
            .chain(row_values(result, variable).iter().map(|v| {
                v.map(|v| {
                    if result.dates.contains(variable) {
                        date_text(v)
                    } else {
                        to_15_digits(v).to_string()
                    }
                })
                .unwrap_or_default()
            }))
            .collect();
        out.push_str(&items.join(","));
        out.push('\n');
    }
    out
}

/// Execute the compare command: calculate the model under each scenario and
/// print the scalars side by side.
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, a scenario does not exist,
/// or calculation fails.
pub fn compare(
    c: &mut Console<'_>,
    file: &Path,
    scenarios: &[String],
    verbose: bool,
    format: ReportFormat,
) -> ForgeResult<()> {
    let table = format == ReportFormat::Table;
    if table {
        outln!(c, "{}", "🔥 Forge - Scenario Comparison".bold().green());
        outln!(c, "   File: {}", file.display());
        outln!(c, "   Scenarios: {}\n", scenarios.join(", "));
    }

    let model = parser::parse_model(file)?;
    if verbose && table {
        outln!(
            c,
            "   Found {} tables, {} scalars, {} scenarios\n",
            model.tables.len(),
            model.scalars.len(),
            model.scenarios.len()
        );
    }
    let inputs: Vec<String> = model
        .scalars
        .iter()
        .filter(|(_, v)| v.formula.is_none())
        .map(|(name, _)| name.clone())
        .collect();

    let result = super::compare_core(file, scenarios)?;
    out!(
        c,
        "{}",
        render_comparison(&result, &inputs, format, c.width)
    );
    if table {
        outln!(c, "\n{}", "✅ Comparison complete".bold().green());
    }
    Ok(())
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
) -> ForgeResult<crate::cli::commands::results::ComparisonResult> {
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

    Ok(crate::cli::commands::results::ComparisonResult {
        scenarios: scenarios.to_vec(),
        variables: all_scalars,
        values,
        descriptions,
        dates,
        formats: crate::cli::commands::scalar_formats(&base_model),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    mod compare {
        //! Tests for `forge compare`'s table and formats.

        use super::*;

        /// Eight scenarios with long names, as in a consumer-proposal model
        fn eight() -> ComparisonResult {
            let scenarios: Vec<String> = [1100, 1300, 1500, 1700]
                .iter()
                .flat_map(|rent| {
                    [
                        format!("rent_{rent}_prosupport_recognized"),
                        format!("rent_{rent}_prosupport_not_recognized"),
                    ]
                })
                .collect();
            let variables = vec![
                "bankruptcy.monthly_surplus_payment".to_string(),
                "living.months".to_string(),
                "living.rate".to_string(),
            ];
            let mut values = indexmap::IndexMap::new();
            for (i, variable) in variables.iter().enumerate() {
                let mut row = indexmap::IndexMap::new();
                let surplus = [
                    774.75, 724.75, 674.75, 624.75, 574.75, 524.75, 474.75, 424.75,
                ];
                for (s, scenario) in scenarios.iter().enumerate() {
                    let value = match i {
                        0 => surplus[s],
                        1 => 60.0,
                        _ => 0.149_75,
                    };
                    row.insert(scenario.clone(), Some(value));
                }
                values.insert(variable.clone(), row);
            }
            ComparisonResult {
                scenarios,
                variables,
                values,
                descriptions: indexmap::IndexMap::new(),
                dates: std::collections::BTreeSet::new(),
                formats: std::collections::BTreeMap::new(),
            }
        }

        fn inputs() -> Vec<String> {
            vec!["living.months".to_string(), "living.rate".to_string()]
        }

        #[test]
        fn every_column_has_room_and_two_spaces_between() {
            let text = render_comparison(&eight(), &inputs(), ReportFormat::Table, None);
            let lines: Vec<&str> = text.lines().collect();
            assert_eq!(lines.len(), 4, "{text}");
            // Names never run together
            assert!(lines[0]
                .contains("rent_1100_prosupport_recognized  rent_1100_prosupport_not_recognized"));
            // Every row has the width of the header: values stay in their columns
            let width = lines[0].chars().count();
            for line in &lines {
                assert_eq!(line.chars().count(), width, "{text}");
            }
            // Numbers: report's rule, whole numbers whole, an input as written
            assert!(
                lines[1].starts_with("bankruptcy.monthly_surplus_payment  "),
                "{text}"
            );
            assert!(lines[1].contains("774.75") && lines[1].contains("424.75"));
            assert!(lines[2].ends_with(" 60"), "{text}");
            assert!(lines[3].ends_with(" 0.14975"), "{text}");
        }

        #[test]
        fn a_table_wider_than_the_terminal_is_printed_in_blocks() {
            let text = render_comparison(&eight(), &inputs(), ReportFormat::Table, Some(80));
            let blocks: Vec<&str> = text.split("\n\n").collect();
            assert!(blocks.len() > 1, "{text}");
            for line in text.lines() {
                assert!(line.chars().count() <= 80, "{line}");
            }
            // Long scenario names and few scalars: one row per scenario, so every
            // block lists the eight scenarios and each scalar is in one block
            for block in &blocks {
                assert!(block.starts_with("scenario "), "{text}");
                for scenario in &eight().scenarios {
                    assert!(
                        block.lines().any(|l| l.starts_with(scenario.as_str())),
                        "{text}"
                    );
                }
            }
            for variable in &eight().variables {
                let hits = blocks
                    .iter()
                    .filter(|b| {
                        b.lines()
                            .next()
                            .unwrap()
                            .split_whitespace()
                            .any(|w| w == variable)
                    })
                    .count();
                assert_eq!(hits, 1, "{variable}");
            }
        }

        #[test]
        fn many_scalars_and_short_scenario_names_keep_one_row_per_scalar() {
            let scenarios: Vec<String> = (1..=8).map(|i| format!("s{i}")).collect();
            let variables: Vec<String> = (1..=30).map(|i| format!("group.value_{i}")).collect();
            let mut values = indexmap::IndexMap::new();
            for v in &variables {
                values.insert(
                    v.clone(),
                    scenarios
                        .iter()
                        .map(|s| (s.clone(), Some(1234.5)))
                        .collect(),
                );
            }
            let result = ComparisonResult {
                scenarios,
                variables,
                values,
                descriptions: indexmap::IndexMap::new(),
                dates: std::collections::BTreeSet::new(),
                formats: std::collections::BTreeMap::new(),
            };
            let text = render_comparison(&result, &[], ReportFormat::Table, Some(60));
            assert!(text.starts_with("variable "), "{text}");
            assert!(text.split("\n\n").count() > 1, "{text}");
            for line in text.lines() {
                assert!(line.chars().count() <= 60, "{line}");
            }
        }

        #[test]
        fn one_column_wider_than_the_terminal_still_gets_a_block() {
            let text = render_comparison(&eight(), &inputs(), ReportFormat::Table, Some(10));
            // One row per scalar: eight blocks; one row per scenario: three
            assert_eq!(text.split("\n\n").count(), 3, "{text}");
        }

        #[test]
        fn csv_and_json_keep_full_precision() {
            let mut result = eight();
            result.values["living.rate"][0] = Some(0.149_753_000_1);
            result.values["living.rate"][1] = None;
            let csv = render_comparison(&result, &inputs(), ReportFormat::Csv, None);
            let lines: Vec<&str> = csv.lines().collect();
            assert!(lines[0].starts_with("variable,rent_1100_prosupport_recognized,"));
            assert!(
                lines[3].starts_with("living.rate,0.1497530001,,0.14975"),
                "{csv}"
            );

            let json = render_comparison(&result, &inputs(), ReportFormat::Json, Some(10));
            let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed["scenarios"].as_array().unwrap().len(), 8);
            assert_eq!(
                parsed["values"]["living.rate"]["rent_1100_prosupport_recognized"],
                0.149_753_000_1
            );
            assert!(
                parsed["values"]["living.rate"]["rent_1100_prosupport_not_recognized"].is_null()
            );
        }

        #[test]
        fn markdown_is_one_pipe_table_whatever_the_width() {
            let text = render_comparison(&eight(), &inputs(), ReportFormat::Markdown, Some(20));
            let lines: Vec<&str> = text.lines().collect();
            assert_eq!(lines.len(), 5);
            assert_eq!(lines[1], format!("| --- |{}", " ---: |".repeat(8)));
            assert!(lines[2].starts_with("| bankruptcy.monthly_surplus_payment | 774.75 |"));
            assert!(lines[4].contains("| 0.14975 |"));
        }

        #[test]
        fn descriptions_are_a_legend_below_the_table() {
            let mut result = eight();
            result.descriptions.insert(
                "rent_1500_prosupport_recognized".to_string(),
                "Studio at $1,500, support recognized".to_string(),
            );
            let table = render_comparison(&result, &inputs(), ReportFormat::Table, None);
            assert!(
                table.ends_with(
                    "\nrent_1500_prosupport_recognized: Studio at $1,500, support recognized\n"
                ),
                "{table}"
            );
            let markdown = render_comparison(&result, &inputs(), ReportFormat::Markdown, None);
            assert!(
                markdown.ends_with(
                    "\n- **rent_1500_prosupport_recognized**: Studio at $1,500, support recognized\n"
                ),
                "{markdown}"
            );
            // Data formats carry the descriptions only in JSON, and the CSV stays a table
            let csv = render_comparison(&result, &inputs(), ReportFormat::Csv, None);
            assert!(!csv.contains("Studio"));
            let json = render_comparison(&result, &inputs(), ReportFormat::Json, None);
            assert!(json.contains("\"descriptions\""));
            // No description, no legend and no field
            let plain = render_comparison(&eight(), &inputs(), ReportFormat::Json, None);
            assert!(!plain.contains("descriptions"));
        }
    }
}
