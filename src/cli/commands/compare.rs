//! `forge compare`: the scalars of a model under several scenarios, one row
//! per scalar and one column per scenario.
//!
//! The table sizes each column from what it holds and keeps two spaces between
//! columns. When it is wider than the terminal it is printed in blocks of
//! columns that fit, each block with the row names: one row per scalar, or one
//! row per scenario when that needs fewer blocks (long scenario names). The
//! numbers follow `forge report`'s rule, and so do the formats.

use super::report::{
    csv_text, display_number, display_whole, markdown_text, shown_input, ReportFormat,
};
use super::results::ComparisonResult;
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
fn shown_row(values: &[Option<f64>], input: bool) -> Vec<String> {
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
            let mut out = serde_json::to_string_pretty(result).unwrap_or_default();
            out.push('\n');
            out
        },
    }
}

fn render_table(result: &ComparisonResult, inputs: &[String], width: Option<usize>) -> String {
    let cells: Vec<Vec<String>> = result
        .variables
        .iter()
        .map(|v| shown_row(&row_values(result, v), inputs.contains(v)))
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
        let cells = shown_row(&row_values(result, variable), inputs.contains(variable));
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
            .chain(
                row_values(result, variable)
                    .iter()
                    .map(|v| v.map(|v| v.to_string()).unwrap_or_default()),
            )
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
    file: &Path,
    scenarios: &[String],
    verbose: bool,
    format: ReportFormat,
) -> ForgeResult<()> {
    let table = format == ReportFormat::Table;
    if table {
        println!("{}", "🔥 Forge - Scenario Comparison".bold().green());
        println!("   File: {}", file.display());
        println!("   Scenarios: {}\n", scenarios.join(", "));
    }

    let model = parser::parse_model(file)?;
    if verbose && table {
        println!(
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
    print!(
        "{}",
        render_comparison(&result, &inputs, format, terminal_width())
    );
    if table {
        println!("\n{}", "✅ Comparison complete".bold().green());
    }
    Ok(())
}

#[cfg(test)]
#[path = "compare_tests.rs"]
mod tests;
