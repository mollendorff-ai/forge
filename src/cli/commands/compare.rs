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
    width_of(
        std::env::var("COLUMNS").ok().as_deref(),
        std::io::stdout().is_terminal(),
        window_columns(),
    )
}

/// The width for the value of `COLUMNS` (if set), whether the output is a
/// terminal, and what asking the terminal gives.
fn width_of(columns: Option<&str>, is_terminal: bool, window: Option<usize>) -> Option<usize> {
    if let Some(columns) = columns
        .and_then(|c| c.trim().parse::<usize>().ok())
        .filter(|&c| c > 0)
    {
        return Some(columns);
    }
    if !is_terminal {
        return None;
    }
    Some(window.unwrap_or(80))
}

#[cfg(unix)]
fn window_columns() -> Option<usize> {
    window_columns_of(libc::STDOUT_FILENO)
}

/// The columns of the terminal that file descriptor `fd` is, `None` when it is
/// not one or reports no width.
#[cfg(unix)]
fn window_columns_of(fd: libc::c_int) -> Option<usize> {
    let mut size = libc::winsize {
        ws_row: 0,
        ws_col: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    // SAFETY: TIOCGWINSZ writes a `winsize` into the pointer it is given, which
    // points at a live, writable `winsize`; a descriptor that is not open or
    // not a terminal makes the call fail, which is read as no width.
    let ok = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &raw mut size) } == 0;
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
        if let Some(row) = json["values"]
            .get_mut(variable)
            .and_then(serde_json::Value::as_object_mut)
        {
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
            let mut available: Vec<_> = base_model.scenarios.keys().collect();
            available.sort_unstable();
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
            let scenario = base_model.scenarios.get(name);
            let text = scenario.and_then(|s| s.description.clone());
            text.map(|text| (name.clone(), text))
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

    mod command_line {
        //! `forge compare` on the command line: the formats and the width of the
        //! table (Session Prompt, compare column overflow).

        use crate::cli::forge_at_width;

        const MODEL: &str = r#"_forge_version: "5.1.0"
living:
  rent: 1200
  total: {formula: "=living.rent + 450.5"}
bankruptcy:
  income: 3100
  monthly_surplus_payment: {formula: "=MAX(0, (bankruptcy.income - living.total) * 0.5)"}
scenarios:
  rent_1100_prosupport_recognized:
    probability: 0.5
    scalars: {living.rent: 1100}
  rent_1500_prosupport_not_recognized:
    probability: 0.5
    scalars: {living.rent: 1500, bankruptcy.income: 2800}
"#;

        const BOTH: &str = "rent_1100_prosupport_recognized,rent_1500_prosupport_not_recognized";

        /// `forge compare` of both scenarios on a terminal `columns` wide
        fn run(dir: &std::path::Path, columns: usize, args: &[&str]) -> String {
            let file = dir.join("m.yaml");
            std::fs::write(&file, MODEL).unwrap();
            let mut all = vec!["compare", file.to_str().unwrap(), "--scenarios", BOTH];
            all.extend(args);
            let ran = forge_at_width(Some(columns), &all);
            assert!(ran.success(), "{}", ran.err);
            ran.out
        }

        #[test]
        fn csv_is_the_numbers_alone_at_full_precision() {
            let dir = tempfile::tempdir().unwrap();
            // (3100 - 1550.5) * 0.5 = 774.75 and (2800 - 1950.5) * 0.5 = 424.75
            assert_eq!(
                run(dir.path(), 40, &["--format", "csv"]),
                "variable,rent_1100_prosupport_recognized,rent_1500_prosupport_not_recognized\n\
         living.rent,1100,1500\n\
         living.total,1550.5,1950.5\n\
         bankruptcy.income,3100,2800\n\
         bankruptcy.monthly_surplus_payment,774.75,424.75\n"
            );
        }

        #[test]
        fn the_table_fits_the_terminal() {
            let dir = tempfile::tempdir().unwrap();
            let wide = run(dir.path(), 200, &[]);
            assert!(wide
                .contains("rent_1100_prosupport_recognized  rent_1500_prosupport_not_recognized"));
            // 34 + 2 + 31 + 2 + 35 = 104 characters: two blocks in 80
            let narrow = run(dir.path(), 80, &[]);
            assert_eq!(narrow.matches("\nvariable ").count(), 2, "{narrow}");
            let table: Vec<&str> = narrow
                .lines()
                .filter(|l| {
                    l.starts_with("variable") || l.starts_with("scenario") || l.contains('.')
                })
                .filter(|l| !l.contains("File:") && !l.contains("Scenarios:"))
                .collect();
            assert!(!table.is_empty(), "{narrow}");
            for line in table {
                assert!(line.chars().count() <= 80, "{line}\n{narrow}");
            }
        }
    }

    mod acquisition_example {
        //! How Valdane pays for Quillon in examples/acquisition (ADR-067): three
        //! structures side by side, as computed outside Forge. The Python is in
        //! `prediction::scenarios::tests::acquisition_example`; `run(cash=...)`.

        use super::*;

        const MODEL: &str = "examples/acquisition/deal.yaml";

        fn close(a: f64, b: f64, tol: f64) -> bool {
            (a - b).abs() <= tol
        }

        #[test]
        fn the_payment_structures_match_the_independent_calculation() {
            let names: Vec<String> = ["base", "all_cash", "all_shares"]
                .map(String::from)
                .to_vec();
            let result = compare_core(Path::new(MODEL), &names).unwrap();
            let at = |variable: &str, scenario: &str| result.values[variable][scenario].unwrap();
            let expected = [
                // (variable, 60/40, all cash, all shares, tolerance)
                (
                    "valuation.value_created",
                    224.507_933_418_290_12,
                    224.507_933_418_290_12,
                    224.507_933_418_290_12,
                    1e-9,
                ),
                (
                    "valuation.value_created_per_share",
                    1.041_316_945_353_850_2,
                    1.122_539_667_091_450_7,
                    0.939_363_738_151_841_5,
                    1e-12,
                ),
                (
                    "eps.year_3_accretion",
                    -0.011_433_658_663_979_496,
                    0.012_613_291_470_434_218,
                    -0.041_618_114_899_645_064,
                    1e-12,
                ),
                (
                    "financing.leverage_at_close",
                    2.042_522_522_522_523,
                    2.604_684_684_684_684_7,
                    1.199_279_279_279_279_2,
                    1e-12,
                ),
                ("financing.new_shares", 15.6, 0.0, 39.0, 1e-9),
                (
                    "financing.quillon_holders_own",
                    0.072_356_215_213_358_08,
                    0.0,
                    0.163_179_916_317_991_67,
                    1e-12,
                ),
            ];
            for (variable, base, cash, shares, tol) in expected {
                for (scenario, want) in [("base", base), ("all_cash", cash), ("all_shares", shares)]
                {
                    let got = at(variable, scenario);
                    assert!(
                        close(got, want, tol),
                        "{variable} {scenario}: {got}, not {want}"
                    );
                }
            }
            // All cash is the one accretive in year 3, and the one past the
            // agencies' 2.5
            assert!(at("eps.year_3_accretion", "all_cash") > 0.0);
            assert!(at("financing.leverage_at_close", "all_cash") > 2.5);
        }
    }

    mod width {
        //! The width the table wraps at: `COLUMNS`, else the terminal's, else no limit.

        use super::super::window_columns;
        use super::super::{terminal_width, width_of};

        #[test]
        fn columns_wins_then_the_terminal_then_no_limit() {
            // `COLUMNS` is the width, whatever the terminal says
            assert_eq!(width_of(Some("120"), false, Some(1)), Some(120));
            assert_eq!(width_of(Some(" 90 "), true, Some(1)), Some(90));
            // A value that is not a width is as if it were not set
            assert_eq!(width_of(Some("wide"), false, Some(1)), None);
            assert_eq!(width_of(Some("0"), false, Some(1)), None);
            assert_eq!(width_of(None, false, Some(1)), None);
            assert_eq!(width_of(Some("0"), true, Some(132)), Some(132));
            assert_eq!(width_of(None, true, Some(100)), Some(100));
            // A terminal that does not say is 80 wide
            assert_eq!(width_of(None, true, None), Some(80));
        }

        #[test]
        fn the_width_of_this_process_is_read_from_its_environment_and_its_output() {
            use std::io::IsTerminal as _;
            assert_eq!(
                terminal_width(),
                width_of(
                    std::env::var("COLUMNS").ok().as_deref(),
                    std::io::stdout().is_terminal(),
                    window_columns()
                )
            );
        }

        #[cfg(unix)]
        mod window {
            use super::super::super::{window_columns, window_columns_of};

            /// A pseudo-terminal of `columns` columns: its two ends
            fn terminal(columns: u16) -> (libc::c_int, libc::c_int) {
                let size = libc::winsize {
                    ws_row: 24,
                    ws_col: columns,
                    ws_xpixel: 0,
                    ws_ypixel: 0,
                };
                let (mut master, mut slave) = (0, 0);
                // SAFETY: `openpty` writes two descriptors into the integers it is
                // given and reads the window size it is given; the name and the
                // terminal settings are left out (null).
                let made = unsafe {
                    libc::openpty(
                        &raw mut master,
                        &raw mut slave,
                        std::ptr::null_mut(),
                        std::ptr::null(),
                        &raw const size,
                    )
                };
                assert_eq!(made, 0);
                (master, slave)
            }

            fn close(fds: &[libc::c_int]) {
                for fd in fds {
                    // SAFETY: each descriptor was opened by the test and is closed once.
                    unsafe { libc::close(*fd) };
                }
            }

            #[test]
            fn a_terminal_gives_its_columns_and_nothing_else_does() {
                let (master, slave) = terminal(97);
                assert_eq!(window_columns_of(slave), Some(97));
                close(&[master, slave]);
                // A terminal with no width, a pipe and a descriptor that is not open
                let (master, slave) = terminal(0);
                assert_eq!(window_columns_of(slave), None);
                close(&[master, slave]);
                let mut ends = [0; 2];
                // SAFETY: `pipe` writes two descriptors into the array it is given.
                assert_eq!(unsafe { libc::pipe(ends.as_mut_ptr()) }, 0);
                assert_eq!(window_columns_of(ends[0]), None);
                close(&ends);
                assert_eq!(window_columns_of(-1), None);
            }

            #[test]
            fn standard_output_is_asked_when_there_is_no_columns() {
                assert_eq!(window_columns(), window_columns_of(libc::STDOUT_FILENO));
            }
        }
    }

    mod cells {
        //! `—` where a scenario has no value, a date as its text, and a format
        //! that cannot show a number falls back to the number.

        use super::super::{comparison_json, render_comparison, shown_row, ReportFormat};
        use crate::cli::commands::results::ComparisonResult;

        #[test]
        fn a_missing_value_is_a_dash_in_every_kind_of_row() {
            let values = [Some(1.5), None];
            assert_eq!(
                shown_row(&values, false, false, Some("0.0%")),
                ["150.0%", "—"]
            );
            assert_eq!(
                shown_row(&[Some(46326.0), None], false, true, None),
                ["2026-10-31", "—"]
            );
            assert_eq!(shown_row(&values, false, false, None), ["1.50", "—"]);
            // A format that cannot show the number leaves it as it is
            assert_eq!(
                shown_row(&values, false, false, Some("not a format")),
                ["1.50", "—"]
            );
        }

        fn with_dates(date: &str, value: Option<f64>) -> ComparisonResult {
            let row = indexmap::IndexMap::from([("a".to_string(), value)]);
            ComparisonResult {
                scenarios: vec!["a".to_string()],
                variables: vec![date.to_string()],
                values: indexmap::IndexMap::from([(date.to_string(), row)]),
                descriptions: indexmap::IndexMap::new(),
                dates: std::collections::BTreeSet::from([date.to_string()]),
                formats: std::collections::BTreeMap::new(),
            }
        }

        #[test]
        fn a_date_is_its_text_in_json_and_csv_and_nothing_stays_null() {
            let result = with_dates("s.d", Some(46326.0));
            assert_eq!(comparison_json(&result)["values"]["s.d"]["a"], "2026-10-31");
            assert_eq!(
                render_comparison(&result, &[], ReportFormat::Csv, None),
                "variable,a\ns.d,2026-10-31\n"
            );
            let none = with_dates("s.d", None);
            assert_eq!(
                comparison_json(&none)["values"]["s.d"]["a"],
                serde_json::Value::Null
            );
            // A date named in `dates` and not in `values` has no row to change
            let mut orphan = with_dates("s.d", None);
            orphan.values.clear();
            assert_eq!(comparison_json(&orphan)["values"], serde_json::json!({}));
        }
    }

    mod command {
        //! `forge compare` end to end: the model's own scalars, a date, a
        //! description, the verbose line, and what it refuses.

        use crate::cli::forge;

        const MODEL: &str = "_forge_version: \"5.4.0\"\ns:\n  x: 2\n  d: 2026-10-31\n  r: {formula: \"=1 / x\"}\n  pct: {value: 0.5, format: \"0.0%\"}\n  q: {formula: \"=d + 1\"}\nscenarios:\n  base:\n    probability: 0.4\n    scalars: {x: 2}\n  zero:\n    probability: 0.1\n    scalars: {x: 0}\n  bad:\n    probability: 0.1\n    scalars: {nope: 1}\n  later:\n    probability: 0.2\n    scalars: {d: 2026-12-31}\n  big:\n    probability: 0.2\n    description: a big x\n    scalars: {x: 4}\n";

        fn file() -> (tempfile::TempDir, String) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, MODEL).unwrap();
            let path = path.to_str().unwrap().to_string();
            (dir, path)
        }

        #[test]
        fn the_table_has_dates_descriptions_and_the_verbose_line() {
            let (_dir, path) = file();
            let text = forge(&["compare", &path, "-s", "base,later,big", "--verbose"]).printed();
            assert_eq!(
                text,
                format!(
                    "🔥 Forge - Scenario Comparison\n   File: {path}\n   Scenarios: base, later, big\n\n   Found 0 tables, 5 scalars, 5 scenarios\n\nvariable        base       later         big\ns.x                2           2           4\ns.d       2026-10-31  2026-12-31  2026-10-31\ns.r             0.50        0.50        0.25\ns.pct          50.0%       50.0%       50.0%\ns.q           46,327      46,388      46,327\n\nbig: a big x\n\n✅ Comparison complete\n"
                )
            );
        }

        #[test]
        fn csv_gives_a_date_as_its_text() {
            let (_dir, path) = file();
            let text =
                forge(&["compare", &path, "-s", "base,later,big", "--format", "csv"]).printed();
            assert_eq!(
                text,
                "variable,base,later,big\ns.x,2,2,4\ns.d,2026-10-31,2026-12-31,2026-10-31\ns.r,0.5,0.5,0.25\ns.pct,0.5,0.5,0.5\ns.q,46327,46388,46327\n"
            );
        }

        #[test]
        fn a_file_a_scenario_or_a_calculation_that_fails_stops_the_command() {
            let (_dir, path) = file();
            for (args, want) in [
                (
                    vec!["compare", "/nonexistent/m.yaml", "-s", "base"],
                    "IO error: No such file or directory (os error 2)",
                ),
                (
                    vec!["compare", path.as_str(), "-s", "nope"],
                    // Listed in name order, the same on every run
                    "Scenario 'nope' not found. Available: [\"bad\", \"base\", \"big\", \"later\", \"zero\"]",
                ),
                (
                    vec!["compare", path.as_str(), "-s", "base,bad"],
                    "Scenario 'bad': Scenario override 'nope' does not name a scalar in the model.",
                ),
                (
                    vec!["compare", path.as_str(), "-s", "base,zero"],
                    "Formula evaluation error: s.r: Division by zero (formula: =1 / x)",
                ),
            ] {
                let ran = forge(&args);
                assert!(!ran.success(), "{args:?}");
                let text = ran.both();
                assert!(text.contains(want), "{args:?}: {text}");
                assert!(!text.contains("Comparison complete"), "{text}");
            }
        }

        #[test]
        fn a_core_comparison_names_the_scenarios_in_order_when_one_is_missing() {
            let (_dir, path) = file();
            let compare = |file: &str| {
                super::super::compare_core(std::path::Path::new(file), &["nope".to_string()])
                    .unwrap_err()
                    .to_string()
            };
            assert_eq!(
                compare("/nonexistent/m.yaml"),
                "IO error: No such file or directory (os error 2)"
            );
            let err = compare(&path);
            assert_eq!(
                err,
                "Validation error: Scenario 'nope' not found. Available: [\"bad\", \"base\", \"big\", \"later\", \"zero\"]"
            );
        }
    }
}
