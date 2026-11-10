//! `forge report`: the scalars of a calculated model side by side, one row per
//! group and one column per key.
//!
//! A report reads the file as `forge calculate` left it. It never calculates
//! and never writes the file: the YAML stays the source of truth. A formula
//! without a value is shown as `—` and named in a note.

use crate::core::array_calculator::evaluator::text::number_format::rounded_digits;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::types::ParsedModel;
use std::fmt::Write as _;
use std::path::Path;

/// How a report is printed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportFormat {
    /// Aligned columns for the terminal
    Table,
    /// Comma-separated, numbers at full precision
    Csv,
    /// One JSON document, numbers at full precision
    Json,
    /// Pipe tables, to paste into a document
    Markdown,
}

impl std::str::FromStr for ReportFormat {
    type Err = ForgeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "table" => Ok(Self::Table),
            "csv" => Ok(Self::Csv),
            "json" => Ok(Self::Json),
            "markdown" => Ok(Self::Markdown),
            other => Err(ForgeError::Validation(format!(
                "'{other}' is not a report format; use table, csv, json or markdown"
            ))),
        }
    }
}

/// What a report shows
#[derive(Debug, Clone, Default)]
pub struct ReportOptions {
    /// Only these groups (all when empty)
    pub groups: Vec<String>,
    /// Only these keys, in this order, as one table (all when empty)
    pub keys: Vec<String>,
    /// Leave out inputs, the values written with no formula
    pub no_inputs: bool,
}

/// One cell of a report
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cell {
    /// The value in the file
    Value(f64),
    /// A formula that has no value yet in the file
    Uncalculated,
    /// The group has no such key (only when keys are chosen)
    Absent,
}

/// One group of scalars, as a row
#[derive(Debug, Clone, PartialEq)]
pub struct ReportRow {
    /// The group's name; empty for scalars at the top level of the file
    pub group: String,
    /// One cell per key of the section
    pub cells: Vec<Cell>,
}

/// Groups that have the same keys, as one table
#[derive(Debug, Clone, PartialEq)]
pub struct ReportSection {
    /// The columns, inputs first and then formulas, as in the section's first group
    pub keys: Vec<String>,
    /// The groups, in file order
    pub rows: Vec<ReportRow>,
}

/// The scalars of a model, ready to print
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// One section per set of keys, in file order
    pub sections: Vec<ReportSection>,
    /// The formulas shown without a value, as `group.key`
    pub uncalculated: Vec<String>,
    /// The tables of the file, which a report does not show (listed only when
    /// neither groups nor keys are chosen)
    pub tables_not_shown: Vec<String>,
}

/// A scalar as the report reads it
struct Entry {
    key: String,
    input: bool,
    cell: Cell,
}

/// The scalars of one group, inputs first
struct Group {
    name: String,
    entries: Vec<Entry>,
}

/// The model's scalars grouped by the part of their name before the first dot,
/// in file order; within a group the inputs come first, each part in file order.
fn groups_of(model: &ParsedModel) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for (path, variable) in &model.scalars {
        let (group, key) = path.split_once('.').unwrap_or(("", path.as_str()));
        let entry = Entry {
            key: key.to_string(),
            input: variable.formula.is_none(),
            cell: variable.value.map_or(Cell::Uncalculated, Cell::Value),
        };
        if let Some(existing) = groups.iter_mut().find(|g| g.name == group) {
            existing.entries.push(entry);
        } else {
            groups.push(Group {
                name: group.to_string(),
                entries: vec![entry],
            });
        }
    }
    for group in &mut groups {
        group.entries.sort_by_key(|e| !e.input);
    }
    groups
}

const fn shown_name(group: &str) -> &str {
    if group.is_empty() {
        "(top level)"
    } else {
        group
    }
}

fn cells_for(keys: &[String], entries: &[&Entry]) -> Vec<Cell> {
    keys.iter()
        .map(|k| {
            entries
                .iter()
                .find(|e| &e.key == k)
                .map_or(Cell::Absent, |e| e.cell)
        })
        .collect()
}

/// Build the report of a parsed model.
///
/// # Errors
///
/// Returns an error if a chosen group or key is not in the model.
pub fn build_report(model: &ParsedModel, options: &ReportOptions) -> ForgeResult<Report> {
    let mut groups = groups_of(model);

    if !options.groups.is_empty() {
        for name in &options.groups {
            if !groups.iter().any(|g| &g.name == name) {
                let names: Vec<&str> = groups
                    .iter()
                    .filter(|g| !g.name.is_empty())
                    .map(|g| g.name.as_str())
                    .collect();
                return Err(ForgeError::Validation(format!(
                    "'{name}' is not a group of scalars in the file. Groups: {}",
                    if names.is_empty() {
                        "none".to_string()
                    } else {
                        names.join(", ")
                    }
                )));
            }
        }
        groups.retain(|g| options.groups.contains(&g.name));
    }

    let sections = if options.keys.is_empty() {
        sections_by_keys(&groups, options.no_inputs)
    } else {
        vec![chosen_keys(&groups, &options.keys)?]
    };

    let mut uncalculated = Vec::new();
    for section in &sections {
        for row in &section.rows {
            for (key, cell) in section.keys.iter().zip(&row.cells) {
                if *cell == Cell::Uncalculated {
                    uncalculated.push(if row.group.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{key}", row.group)
                    });
                }
            }
        }
    }

    let tables_not_shown = if options.groups.is_empty() && options.keys.is_empty() {
        model.tables.keys().cloned().collect()
    } else {
        Vec::new()
    };

    Ok(Report {
        sections,
        uncalculated,
        tables_not_shown,
    })
}

/// Groups with the same set of keys share a section; sections in file order.
fn sections_by_keys(groups: &[Group], no_inputs: bool) -> Vec<ReportSection> {
    let mut sections: Vec<(Vec<String>, ReportSection)> = Vec::new();
    for group in groups {
        let entries: Vec<&Entry> = group
            .entries
            .iter()
            .filter(|e| !(no_inputs && e.input))
            .collect();
        if entries.is_empty() {
            continue;
        }
        let mut key_set: Vec<String> = entries.iter().map(|e| e.key.clone()).collect();
        key_set.sort_unstable();
        if let Some((_, section)) = sections.iter_mut().find(|(set, _)| *set == key_set) {
            let cells = cells_for(&section.keys, &entries);
            section.rows.push(ReportRow {
                group: group.name.clone(),
                cells,
            });
        } else {
            let keys: Vec<String> = entries.iter().map(|e| e.key.clone()).collect();
            let cells = cells_for(&keys, &entries);
            sections.push((
                key_set,
                ReportSection {
                    keys,
                    rows: vec![ReportRow {
                        group: group.name.clone(),
                        cells,
                    }],
                },
            ));
        }
    }
    sections.into_iter().map(|(_, section)| section).collect()
}

/// The chosen keys, in the order given, for every group that has one of them.
fn chosen_keys(groups: &[Group], keys: &[String]) -> ForgeResult<ReportSection> {
    for key in keys {
        if !groups
            .iter()
            .any(|g| g.entries.iter().any(|e| &e.key == key))
        {
            let mut known: Vec<&str> = Vec::new();
            for entry in groups.iter().flat_map(|g| &g.entries) {
                if !known.contains(&entry.key.as_str()) {
                    known.push(&entry.key);
                }
            }
            return Err(ForgeError::Validation(format!(
                "'{key}' is not a key of any group shown. Keys: {}",
                if known.is_empty() {
                    "none".to_string()
                } else {
                    known.join(", ")
                }
            )));
        }
    }
    let rows = groups
        .iter()
        .filter(|g| g.entries.iter().any(|e| keys.contains(&e.key)))
        .map(|g| {
            let entries: Vec<&Entry> = g.entries.iter().collect();
            ReportRow {
                group: g.name.clone(),
                cells: cells_for(keys, &entries),
            }
        })
        .collect();
    Ok(ReportSection {
        keys: keys.to_vec(),
        rows,
    })
}

//==============================================================================
// Numbers
//==============================================================================

/// A number as a report shows it in a table or in Markdown.
///
/// Two decimals and thousands separators (`1,450.00`). A number under 1 keeps
/// four significant digits, at most six decimals, so that a rate of 0.0479 is
/// not shown as 0.05.
///
/// The digits are those a spreadsheet shows: the number to 15 significant
/// digits, then rounded half away from zero, so 2.675 is `2.68` although the
/// nearest double is a little under it.
#[must_use]
pub fn display_number(x: f64) -> String {
    if !x.is_finite() {
        return x.to_string();
    }
    let magnitude = x.abs();
    let decimals = if magnitude == 0.0 || magnitude >= 1.0 {
        2
    } else if magnitude >= 0.1 {
        4
    } else if magnitude >= 0.01 {
        5
    } else {
        6
    };
    let (whole, mut fraction) = rounded_digits(magnitude, decimals);
    while fraction.len() > 2 && fraction.ends_with('0') {
        fraction.pop();
    }
    let is_zero = whole == "0" && fraction.bytes().all(|b| b == b'0');
    let sign = if x < 0.0 && !is_zero { "-" } else { "" };
    format!("{sign}{}.{fraction}", with_thousands(&whole))
}

fn with_thousands(whole: &str) -> String {
    let mut out = String::with_capacity(whole.len() + whole.len() / 3);
    for (i, c) in whole.chars().enumerate() {
        if i > 0 && (whole.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

//==============================================================================
// Rendering
//==============================================================================

/// The report in a format, as printed on standard output.
#[must_use]
pub fn render(report: &Report, format: ReportFormat) -> String {
    match format {
        ReportFormat::Table => render_table(report),
        ReportFormat::Csv => render_csv(report),
        ReportFormat::Json => render_json(report),
        ReportFormat::Markdown => render_markdown(report),
    }
}

fn shown_cell(cell: Cell) -> String {
    match cell {
        Cell::Value(v) => display_number(v),
        Cell::Uncalculated => "—".to_string(),
        Cell::Absent => String::new(),
    }
}

fn render_table(report: &Report) -> String {
    let mut out = String::new();
    for (i, section) in report.sections.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let mut lines: Vec<Vec<String>> = vec![std::iter::once("group".to_string())
            .chain(section.keys.iter().cloned())
            .collect()];
        for row in &section.rows {
            lines.push(
                std::iter::once(shown_name(&row.group).to_string())
                    .chain(row.cells.iter().map(|c| shown_cell(*c)))
                    .collect(),
            );
        }
        let columns = section.keys.len() + 1;
        let widths: Vec<usize> = (0..columns)
            .map(|c| {
                lines
                    .iter()
                    .map(|l| l[c].chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        for line in &lines {
            let mut text = String::new();
            for (c, item) in line.iter().enumerate() {
                let width = widths[c];
                if c == 0 {
                    let _ = write!(text, "{item:<width$}");
                } else {
                    let _ = write!(text, "  {item:>width$}");
                }
            }
            out.push_str(text.trim_end());
            out.push('\n');
        }
    }
    out
}

fn markdown_text(text: &str) -> String {
    text.replace('|', "\\|")
}

fn render_markdown(report: &Report) -> String {
    let mut out = String::new();
    for (i, section) in report.sections.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let header: Vec<String> = std::iter::once("group".to_string())
            .chain(section.keys.iter().map(|k| markdown_text(k)))
            .collect();
        let _ = writeln!(out, "| {} |", header.join(" | "));
        let rule: Vec<&str> = std::iter::once("---")
            .chain(section.keys.iter().map(|_| "---:"))
            .collect();
        let _ = writeln!(out, "| {} |", rule.join(" | "));
        for row in &section.rows {
            let items: Vec<String> = std::iter::once(markdown_text(shown_name(&row.group)))
                .chain(row.cells.iter().map(|c| shown_cell(*c)))
                .collect();
            let _ = writeln!(out, "| {} |", items.join(" | "));
        }
    }
    out
}

fn csv_text(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

fn render_csv(report: &Report) -> String {
    let mut out = String::new();
    for (i, section) in report.sections.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let header: Vec<String> = std::iter::once("group".to_string())
            .chain(section.keys.iter().map(|k| csv_text(k)))
            .collect();
        out.push_str(&header.join(","));
        out.push('\n');
        for row in &section.rows {
            let items: Vec<String> = std::iter::once(csv_text(&row.group))
                .chain(row.cells.iter().map(|c| match c {
                    Cell::Value(v) => v.to_string(),
                    Cell::Uncalculated | Cell::Absent => String::new(),
                }))
                .collect();
            out.push_str(&items.join(","));
            out.push('\n');
        }
    }
    out
}

fn render_json(report: &Report) -> String {
    let sections: Vec<serde_json::Value> = report
        .sections
        .iter()
        .map(|section| {
            let rows: Vec<serde_json::Value> = section
                .rows
                .iter()
                .map(|row| {
                    let mut values = serde_json::Map::new();
                    for (key, cell) in section.keys.iter().zip(&row.cells) {
                        match cell {
                            Cell::Value(v) => {
                                values.insert(key.clone(), serde_json::json!(v));
                            },
                            Cell::Uncalculated => {
                                values.insert(key.clone(), serde_json::Value::Null);
                            },
                            Cell::Absent => {},
                        }
                    }
                    serde_json::json!({ "group": row.group, "values": values })
                })
                .collect();
            serde_json::json!({ "keys": section.keys, "rows": rows })
        })
        .collect();
    let document = serde_json::json!({
        "sections": sections,
        "uncalculated": report.uncalculated,
        "tables_not_shown": report.tables_not_shown,
    });
    let mut out = serde_json::to_string_pretty(&document).unwrap_or_default();
    out.push('\n');
    out
}

/// One-line notes for standard error: formulas without a value, tables not
/// shown, or nothing to report.
#[must_use]
pub fn notes(report: &Report, file: &Path) -> Vec<String> {
    let mut notes = Vec::new();
    if report.sections.is_empty() {
        notes.push(format!(
            "No groups of scalars to report in {}",
            file.display()
        ));
    }
    if !report.uncalculated.is_empty() {
        let count = report.uncalculated.len();
        notes.push(format!(
            "Note: {count} formula{} no value yet and {} shown as —; run `forge calculate {}` first ({})",
            if count == 1 { " has" } else { "s have" },
            if count == 1 { "is" } else { "are" },
            file.display(),
            report.uncalculated.join(", ")
        ));
    }
    if !report.tables_not_shown.is_empty() {
        notes.push(format!(
            "Note: tables are not shown ({}); `forge export` writes them to Excel",
            report.tables_not_shown.join(", ")
        ));
    }
    notes
}

/// Read a file and build its report, without calculating or writing it.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed, or a chosen group or
/// key is not in it.
pub fn report_core(file: &Path, options: &ReportOptions) -> ForgeResult<Report> {
    let model = parser::parse_model(file)?;
    build_report(&model, options)
}

/// `forge report`: print a file's scalars side by side, and the notes on
/// standard error.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed, or a chosen group or
/// key is not in it.
pub fn report(file: &Path, options: &ReportOptions, format: ReportFormat) -> ForgeResult<()> {
    let report = report_core(file, options)?;
    print!("{}", render(&report, format));
    for note in notes(&report, file) {
        eprintln!("{note}");
    }
    Ok(())
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
