//! `forge report`: the scalars of a calculated model side by side, one row per
//! group and one column per key.
//!
//! A report reads the file as `forge calculate` left it. It never calculates
//! and never writes the file: the YAML stays the source of truth. A formula
//! without a value is shown as `—` and named in a note.

use crate::cli::console::{errln, out};
use crate::cli::Console;
use crate::core::array_calculator::evaluator::text::number_format::rounded_digits;
use crate::core::precision::to_15_digits;
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
    /// Comma-separated, numbers to 15 significant digits
    Csv,
    /// One JSON document, numbers to 15 significant digits
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
                "'{other}' is not a format; use table, csv, json or markdown"
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
    /// A date in the file, as its day number (ADR-060)
    Date(f64),
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
    /// Per cell, whether it is an input (written in the file, not calculated)
    pub inputs: Vec<bool>,
    /// Per cell, its value as its `format:` shows it (ADR-061), if it has one:
    /// the table and Markdown show this, CSV and JSON the value
    pub formatted: Vec<Option<String>>,
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
    formatted: Option<String>,
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
            formatted: variable.formatted(),
            cell: match variable.value {
                Some(v) if variable.is_date => Cell::Date(v),
                Some(v) => Cell::Value(v),
                None => Cell::Uncalculated,
            },
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

fn row_for(group: &str, keys: &[String], entries: &[&Entry]) -> ReportRow {
    let found: Vec<Option<&&Entry>> = keys
        .iter()
        .map(|k| entries.iter().find(|e| &e.key == k))
        .collect();
    ReportRow {
        group: group.to_string(),
        cells: found
            .iter()
            .map(|e| e.map_or(Cell::Absent, |e| e.cell))
            .collect(),
        inputs: found.iter().map(|e| e.is_some_and(|e| e.input)).collect(),
        formatted: found
            .iter()
            .map(|e| e.and_then(|e| e.formatted.clone()))
            .collect(),
    }
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
            let row = row_for(&group.name, &section.keys, &entries);
            section.rows.push(row);
        } else {
            let keys: Vec<String> = entries.iter().map(|e| e.key.clone()).collect();
            let row = row_for(&group.name, &keys, &entries);
            sections.push((
                key_set,
                ReportSection {
                    keys,
                    rows: vec![row],
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
            row_for(&g.name, keys, &entries)
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

/// The same number with no decimals, for a column of whole numbers
/// (`1,700`, `3`, `0`).
pub fn display_whole(x: f64) -> String {
    display_number(x).trim_end_matches(".00").to_string()
}

/// Whether a column shows whole numbers only: every value it shows ends in
/// `.00`, so `3.50` or `0.0479` keeps the column's decimals for every cell.
fn column_is_whole(section: &ReportSection, column: usize) -> bool {
    section.rows.iter().all(|row| match row.cells[column] {
        Cell::Value(v) => !v.is_finite() || display_number(v).ends_with(".00"),
        Cell::Date(_) | Cell::Uncalculated | Cell::Absent => true,
    })
}

/// An input is shown as written when the column's rule would round it:
/// `0.14975` is not `0.1498`.
pub fn shown_input(x: f64, shown: String) -> String {
    let same = shown
        .replace(',', "")
        .parse::<f64>()
        .is_ok_and(|p| p.to_bits() == x.to_bits());
    if same || !x.is_finite() {
        return shown;
    }
    let text = x.abs().to_string();
    let (whole, fraction) = text.split_once('.').unwrap_or((text.as_str(), ""));
    let sign = if x < 0.0 { "-" } else { "" };
    if fraction.is_empty() {
        format!("{sign}{}", with_thousands(whole))
    } else {
        format!("{sign}{}.{fraction}", with_thousands(whole))
    }
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

/// A cell as table and Markdown show it; `whole` says its column has whole
/// numbers only.
fn shown_cell(cell: Cell, input: bool, whole: bool) -> String {
    match cell {
        Cell::Value(v) => {
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
        Cell::Date(v) => date_text(v),
        Cell::Uncalculated => "—".to_string(),
        Cell::Absent => String::new(),
    }
}

/// The text of the date with day number `serial` (ADR-060).
fn date_text(serial: f64) -> String {
    crate::core::array_calculator::evaluator::serial_date_text(serial)
}

/// The cells of a row, as table and Markdown show them.
fn shown_row(section: &ReportSection, wholes: &[bool], row: &ReportRow) -> Vec<String> {
    (0..section.keys.len())
        .map(|c| {
            row.formatted
                .get(c)
                .cloned()
                .flatten()
                .unwrap_or_else(|| shown_cell(row.cells[c], row.inputs[c], wholes[c]))
        })
        .collect()
}

fn whole_columns(section: &ReportSection) -> Vec<bool> {
    (0..section.keys.len())
        .map(|c| column_is_whole(section, c))
        .collect()
}

fn render_table(report: &Report) -> String {
    let mut out = String::new();
    for (i, section) in report.sections.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        let wholes = whole_columns(section);
        let mut lines: Vec<Vec<String>> = vec![std::iter::once("group".to_string())
            .chain(section.keys.iter().cloned())
            .collect()];
        for row in &section.rows {
            lines.push(
                std::iter::once(shown_name(&row.group).to_string())
                    .chain(shown_row(section, &wholes, row))
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

pub fn markdown_text(text: &str) -> String {
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
        let wholes = whole_columns(section);
        for row in &section.rows {
            let items: Vec<String> = std::iter::once(markdown_text(shown_name(&row.group)))
                .chain(shown_row(section, &wholes, row))
                .collect();
            let _ = writeln!(out, "| {} |", items.join(" | "));
        }
    }
    out
}

pub fn csv_text(text: &str) -> String {
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
                    Cell::Value(v) => to_15_digits(*v).to_string(),
                    Cell::Date(v) => date_text(*v),
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
                            Cell::Date(v) => {
                                values.insert(key.clone(), serde_json::json!(date_text(*v)));
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
    let mut out = serde_json::to_string_pretty(&crate::core::precision::Rounded(&document))
        .unwrap_or_default();
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
pub fn report(
    c: &mut Console<'_>,
    file: &Path,
    options: &ReportOptions,
    format: ReportFormat,
) -> ForgeResult<()> {
    let report = report_core(file, options)?;
    out!(c, "{}", render(&report, format));
    for note in notes(&report, file) {
        errln!(c, "{note}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    mod report {
        use super::*;
        use std::io::Write as _;

        fn model_of(yaml: &str) -> ParsedModel {
            let mut file = tempfile::NamedTempFile::new().expect("temp file");
            file.write_all(yaml.as_bytes()).expect("write");
            parser::parse_model(file.path()).expect("parse")
        }

        fn report_of(yaml: &str, options: &ReportOptions) -> Report {
            build_report(&model_of(yaml), options).expect("report")
        }

        const OPTIONS: &str = r#"_forge_version: "5.1.0"
oak_suite:
  rent: 1450
  utilities: 60
  year1_monthly: {formula: "=oak_suite.rent + oak_suite.utilities", value: 1510}
elm_loft:
  utilities: 90
  rent: 1390
  year1_monthly: {formula: "=elm_loft.rent + elm_loft.utilities", value: 1480}
assumptions:
  rate: 0.0479
"#;

        #[test]
        fn numbers_have_two_decimals_and_thousands_separators() {
            assert_eq!(display_number(1450.0), "1,450.00");
            assert_eq!(display_number(1_234_567.891), "1,234,567.89");
            assert_eq!(display_number(-1234.5), "-1,234.50");
            assert_eq!(display_number(999.995), "1,000.00");
            assert_eq!(display_number(100.0), "100.00");
            assert_eq!(display_number(0.0), "0.00");
        }

        #[test]
        fn numbers_under_one_keep_four_significant_digits() {
            assert_eq!(display_number(0.0479), "0.0479");
            assert_eq!(display_number(0.5), "0.50");
            assert_eq!(display_number(0.25), "0.25");
            assert_eq!(display_number(0.123_456), "0.1235");
            assert_eq!(display_number(-0.0479), "-0.0479");
            assert_eq!(display_number(0.99996), "1.00");
            assert_eq!(display_number(0.000_123_4), "0.000123");
        }

        #[test]
        fn rounding_is_half_away_from_zero_on_the_shown_digits() {
            // The nearest doubles to these are a little under the tie.
            assert_eq!(display_number(2.675), "2.68");
            assert_eq!(display_number(1.005), "1.01");
            assert_eq!(display_number(-2.675), "-2.68");
            assert_eq!(display_number(1_234.565), "1,234.57");
        }

        #[test]
        fn a_residue_near_zero_is_zero_without_a_sign() {
            assert_eq!(display_number(-1.2e-10), "0.00");
            assert_eq!(display_number(4e-9), "0.00");
            assert_eq!(display_number(-0.0), "0.00");
        }

        #[test]
        fn numbers_that_are_not_finite_say_so() {
            assert_eq!(display_number(f64::NAN), "NaN");
            assert_eq!(display_number(f64::INFINITY), "inf");
        }

        #[test]
        fn groups_with_the_same_keys_share_a_table_aligned_by_key() {
            let report = report_of(OPTIONS, &ReportOptions::default());
            assert_eq!(report.sections.len(), 2);
            let first = &report.sections[0];
            assert_eq!(first.keys, ["rent", "utilities", "year1_monthly"]);
            assert_eq!(first.rows[0].group, "oak_suite");
            // elm_loft lists utilities before rent; its cells follow the section's keys.
            assert_eq!(
                first.rows[1].cells,
                [Cell::Value(1390.0), Cell::Value(90.0), Cell::Value(1480.0)]
            );
            assert_eq!(report.sections[1].keys, ["rate"]);
            assert_eq!(report.sections[1].rows[0].group, "assumptions");
        }

        #[test]
        fn groups_with_other_keys_get_their_own_table_in_file_order() {
            let yaml = r#"_forge_version: "5.1.0"
a:
  x: 1
  y: 2
b:
  z: 3
c:
  y: 4
  x: 5
d:
  x: 6
"#;
            let report = report_of(yaml, &ReportOptions::default());
            let shape: Vec<(Vec<String>, Vec<String>)> = report
                .sections
                .iter()
                .map(|s| {
                    (
                        s.keys.clone(),
                        s.rows.iter().map(|r| r.group.clone()).collect(),
                    )
                })
                .collect();
            assert_eq!(
                shape,
                [
                    (vec!["x".into(), "y".into()], vec!["a".into(), "c".into()]),
                    (vec!["z".into()], vec!["b".into()]),
                    (vec!["x".into()], vec!["d".into()]),
                ]
            );
        }

        #[test]
        fn inputs_come_before_formulas() {
            let yaml = r#"_forge_version: "5.1.0"
a:
  total: {formula: "=a.x + a.y", value: 3}
  x: 1
  y: 2
"#;
            let report = report_of(yaml, &ReportOptions::default());
            assert_eq!(report.sections[0].keys, ["x", "y", "total"]);
        }

        #[test]
        fn no_inputs_leaves_only_formulas() {
            let report = report_of(
                OPTIONS,
                &ReportOptions {
                    no_inputs: true,
                    ..ReportOptions::default()
                },
            );
            // assumptions has only an input, so it has no row left.
            assert_eq!(report.sections.len(), 1);
            assert_eq!(report.sections[0].keys, ["year1_monthly"]);
            assert_eq!(report.sections[0].rows.len(), 2);
        }

        #[test]
        fn chosen_groups_are_shown_in_file_order() {
            let report = report_of(
                OPTIONS,
                &ReportOptions {
                    groups: vec!["elm_loft".into(), "oak_suite".into()],
                    ..ReportOptions::default()
                },
            );
            let groups: Vec<&str> = report.sections[0]
                .rows
                .iter()
                .map(|r| r.group.as_str())
                .collect();
            assert_eq!(groups, ["oak_suite", "elm_loft"]);
            assert_eq!(report.tables_not_shown, Vec::<String>::new());
        }

        #[test]
        fn an_unknown_group_is_an_error_that_lists_the_groups() {
            let err = build_report(
                &model_of(OPTIONS),
                &ReportOptions {
                    groups: vec!["unit_807".into()],
                    ..ReportOptions::default()
                },
            )
            .unwrap_err()
            .to_string();
            assert!(err.contains("'unit_807' is not a group"), "{err}");
            assert!(err.contains("oak_suite, elm_loft, assumptions"), "{err}");
        }

        #[test]
        fn chosen_keys_make_one_table_in_their_order() {
            let report = report_of(
                OPTIONS,
                &ReportOptions {
                    keys: vec!["year1_monthly".into(), "rent".into(), "rate".into()],
                    ..ReportOptions::default()
                },
            );
            assert_eq!(report.sections.len(), 1);
            let section = &report.sections[0];
            assert_eq!(section.keys, ["year1_monthly", "rent", "rate"]);
            assert_eq!(
                section.rows[0].cells,
                [Cell::Value(1510.0), Cell::Value(1450.0), Cell::Absent]
            );
            assert_eq!(
                section.rows[2].cells,
                [Cell::Absent, Cell::Absent, Cell::Value(0.0479)]
            );
        }

        #[test]
        fn an_unknown_key_is_an_error_that_lists_the_keys() {
            let err = build_report(
                &model_of(OPTIONS),
                &ReportOptions {
                    keys: vec!["deposit".into()],
                    ..ReportOptions::default()
                },
            )
            .unwrap_err()
            .to_string();
            assert!(err.contains("'deposit' is not a key"), "{err}");
            assert!(
                err.contains("rent, utilities, year1_monthly, rate"),
                "{err}"
            );
        }

        const UNCALCULATED: &str = r#"_forge_version: "5.1.0"
a:
  x: 1
  total: {formula: "=a.x * 2"}
summary:
  rate: 0.05
  t: {formula: "=summary.rate * 2"}
items:
  amount: [1, 2]
"#;

        #[test]
        fn formulas_without_a_value_are_named() {
            let report = report_of(UNCALCULATED, &ReportOptions::default());
            assert_eq!(report.sections[0].rows[0].cells[1], Cell::Uncalculated);
            assert_eq!(report.uncalculated, ["a.total", "summary.t"]);
            assert_eq!(report.tables_not_shown, ["items"]);
            let notes = notes(&report, Path::new("m.yaml"));
            assert_eq!(
                notes,
                [
                    "Note: 2 formulas have no value yet and are shown as —; run `forge calculate m.yaml` first (a.total, summary.t)",
                    "Note: tables are not shown (items); `forge export` writes them to Excel",
                ]
            );
        }

        #[test]
        fn top_level_scalars_are_one_row() {
            let yaml = r#"_forge_version: "5.1.0"
price: 16
quantity: 3
"#;
            let report = report_of(yaml, &ReportOptions::default());
            assert_eq!(report.sections[0].rows[0].group, "");
            assert!(render(&report, ReportFormat::Table)
                .starts_with("group        price  quantity\n(top level)"));
        }

        #[test]
        fn a_file_with_no_scalars_says_so() {
            let yaml = r#"_forge_version: "5.1.0"
items:
  amount: [1, 2]
"#;
            let report = report_of(yaml, &ReportOptions::default());
            assert_eq!(report.sections, Vec::<ReportSection>::new());
            assert_eq!(render(&report, ReportFormat::Table), "");
            assert_eq!(
                notes(&report, Path::new("m.yaml"))[0],
                "No groups of scalars to report in m.yaml"
            );
        }

        #[test]
        fn table_format() {
            let report = report_of(OPTIONS, &ReportOptions::default());
            assert_eq!(
                render(&report, ReportFormat::Table),
                "\
group       rent  utilities  year1_monthly
oak_suite  1,450         60          1,510
elm_loft   1,390         90          1,480

group          rate
assumptions  0.0479
"
            );
        }

        #[test]
        fn markdown_format() {
            let report = report_of(OPTIONS, &ReportOptions::default());
            assert_eq!(
                render(&report, ReportFormat::Markdown),
                "\
| group | rent | utilities | year1_monthly |
| --- | ---: | ---: | ---: |
| oak_suite | 1,450 | 60 | 1,510 |
| elm_loft | 1,390 | 90 | 1,480 |

| group | rate |
| --- | ---: |
| assumptions | 0.0479 |
"
            );
        }

        #[test]
        fn markdown_is_one_pipe_table_per_key_set() {
            let text = render(
                &report_of(UNCALCULATED, &ReportOptions::default()),
                ReportFormat::Markdown,
            );
            let tables: Vec<&str> = text.trim_end().split("\n\n").collect();
            assert_eq!(tables.len(), 2);
            for table in tables {
                let lines: Vec<&str> = table.lines().collect();
                let cells = |line: &str| line.matches(" | ").count();
                assert!(lines.len() >= 3, "{table}");
                assert!(lines[1].starts_with("| --- |"), "{table}");
                for line in &lines {
                    assert!(line.starts_with("| ") && line.ends_with(" |"), "{line}");
                    assert_eq!(cells(line), cells(lines[0]), "{table}");
                }
            }
            assert!(text.contains("| a | 1 | — |"), "{text}");
        }

        #[test]
        fn csv_format_has_every_digit_and_no_separators() {
            let yaml = r#"_forge_version: "5.1.0"
"a,b":
  big: 1234567.891
  rate: 0.0479
  total: {formula: "=1"}
"#;
            let report = report_of(yaml, &ReportOptions::default());
            assert_eq!(
                render(&report, ReportFormat::Csv),
                "group,big,rate,total\n\"a,b\",1234567.891,0.0479,\n"
            );
        }

        #[test]
        fn csv_sections_are_separated_by_a_blank_line() {
            let report = report_of(OPTIONS, &ReportOptions::default());
            assert_eq!(
                render(&report, ReportFormat::Csv),
                "\
group,rent,utilities,year1_monthly
oak_suite,1450,60,1510
elm_loft,1390,90,1480

group,rate
assumptions,0.0479
"
            );
        }

        #[test]
        fn json_format() {
            let report = report_of(UNCALCULATED, &ReportOptions::default());
            let value: serde_json::Value =
                serde_json::from_str(&render(&report, ReportFormat::Json)).expect("json");
            assert_eq!(
                value,
                serde_json::json!({
                    "sections": [
                        {"keys": ["x", "total"], "rows": [{"group": "a", "values": {"x": 1.0, "total": null}}]},
                        {"keys": ["rate", "t"], "rows": [{"group": "summary", "values": {"rate": 0.05, "t": null}}]},
                    ],
                    "uncalculated": ["a.total", "summary.t"],
                    "tables_not_shown": ["items"],
                })
            );
        }

        #[test]
        fn json_leaves_out_keys_a_group_does_not_have() {
            let report = report_of(
                OPTIONS,
                &ReportOptions {
                    keys: vec!["rent".into(), "rate".into()],
                    ..ReportOptions::default()
                },
            );
            let value: serde_json::Value =
                serde_json::from_str(&render(&report, ReportFormat::Json)).expect("json");
            assert_eq!(
                value["sections"][0]["rows"][2],
                serde_json::json!({"group": "assumptions", "values": {"rate": 0.0479}})
            );
        }

        #[test]
        fn formats_are_read_by_name() {
            assert_eq!(
                "markdown".parse::<ReportFormat>().unwrap(),
                ReportFormat::Markdown
            );
            assert!("html".parse::<ReportFormat>().is_err());
        }

        const MIXED: &str = r#"_forge_version: "5.1.0"
unit_a:
  list_rent: 1700
  free_months: 3
  spread: 0
  fee: 12.5
  rate: 0.14975
unit_b:
  list_rent: 1250
  free_months: 0
  spread: 0
  fee: 8
  rate: 0.2
"#;

        #[test]
        fn a_column_of_whole_numbers_has_no_decimals() {
            let report = report_of(MIXED, &ReportOptions::default());
            assert_eq!(
                render(&report, ReportFormat::Table),
                "\
group   list_rent  free_months  spread    fee     rate
unit_a      1,700            3       0  12.50  0.14975
unit_b      1,250            0       0   8.00     0.20
"
            );
        }

        #[test]
        fn the_rule_is_per_column_so_a_column_never_mixes_3_and_3_50() {
            let report = report_of(MIXED, &ReportOptions::default());
            let markdown = render(&report, ReportFormat::Markdown);
            // fee: 12.5 and 8 keep two decimals in both rows
            assert!(markdown.contains("| 12.50 |") && markdown.contains("| 8.00 |"));
            // free_months: 3 and 0 are whole
            assert!(markdown.contains("| 3 |") && markdown.contains("| 0 |"));
        }

        #[test]
        fn an_input_is_shown_as_written_where_rounding_would_change_it() {
            let report = report_of(MIXED, &ReportOptions::default());
            let table = render(&report, ReportFormat::Table);
            // 0.14975 would be 0.1498 under the four-digit rule
            assert!(table.contains("0.14975"), "{table}");
            assert!(!table.contains("0.1498"), "{table}");
        }

        #[test]
        fn a_calculated_value_is_never_shown_as_an_input_is() {
            let yaml = r#"_forge_version: "5.1.0"
a:
  x: {formula: "=1 / 3", value: 0.333333333333}
"#;
            let report = report_of(yaml, &ReportOptions::default());
            assert!(render(&report, ReportFormat::Table).contains("0.3333"));
        }

        #[test]
        fn whole_columns_leave_csv_and_json_alone() {
            let report = report_of(MIXED, &ReportOptions::default());
            assert!(render(&report, ReportFormat::Csv).contains("unit_a,1700,3,0,12.5,0.14975"));
            assert!(render(&report, ReportFormat::Json).contains("\"free_months\": 3.0"));
        }

        #[test]
        fn an_uncalculated_cell_does_not_stop_a_column_being_whole() {
            let yaml = r#"_forge_version: "5.1.0"
a:
  n: 3
  total: {formula: "=a.n * 2"}
b:
  n: 4
  total: {formula: "=b.n * 2", value: 8}
"#;
            let report = report_of(yaml, &ReportOptions::default());
            assert_eq!(
                render(&report, ReportFormat::Table),
                "group  n  total\na      3      —\nb      4      8\n"
            );
        }
    }
}
