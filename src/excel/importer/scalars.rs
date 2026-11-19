//! The `Name` / `Value` sheets: the scalars of the model and of each included
//! file.

use super::sheet::{date_of, number_of, Sheet};
use super::{ExcelImporter, Names};
use crate::excel::workbook_refs::dates::Holds;
use crate::excel::workbook_refs::{Context, ReverseRefs};
use crate::types::{ParsedModel, Variable};
use calamine::Data;

impl ExcelImporter {
    /// An included file's sheet: its scalars, under their own paths, become
    /// that file's model, read by the main model as `@{namespace}.{path}`.
    pub(super) fn import_include(
        sheet: &Sheet,
        namespace: &str,
        read: (&ReverseRefs, &Names),
        model: &mut ParsedModel,
        warnings: &mut Vec<String>,
    ) {
        let mut sheet_model = ParsedModel::new();
        Self::import_scalars(sheet, None, read, &mut sheet_model, warnings);
        let prefix = format!("{namespace}.");
        let included = Self::included_model(model, namespace);
        for (name, mut var) in sheet_model.scalars {
            let path = name[prefix.len()..].to_string();
            var.path.clone_from(&path);
            included.add_scalar(path, var);
        }
    }

    /// A `Name` / `Value` sheet: one scalar per row, named `{group}.{name}`.
    pub(super) fn import_scalars(
        sheet: &Sheet,
        group: Option<&str>,
        read: (&ReverseRefs, &Names),
        model: &mut ParsedModel,
        warnings: &mut Vec<String>,
    ) {
        let (refs, names) = read;
        let (height, _) = sheet.values.get_size();
        for row in 1..height {
            let Some(name) = sheet.values.get((row, 0)).map(ToString::to_string) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let path = group
                .as_ref()
                .map_or_else(|| name.clone(), |g| format!("{g}.{name}"));

            // A scalar is a number, or a date cell with no formula: a date,
            // held as the day number its cell holds (ADR-060)
            let cell = sheet.values.get((row, 1));
            let value = cell.and_then(|cell| match cell {
                Data::Bool(b) => Some(f64::from(u8::from(*b))),
                other => number_of(other),
            });
            let is_date = sheet.formula(row, 1).is_none() && cell.and_then(date_of).is_some();

            let formula = sheet.formula(row, 1).map_or_else(
                // Older workbooks: Forge formula as text in a third column
                || match sheet.values.get((row, 2)) {
                    Some(Data::String(f)) if !f.is_empty() => Some(f.clone()),
                    _ => None,
                },
                |excel| match refs
                    .to_forge(&excel, &sheet.name, sheet.excel_row(row))
                    .and_then(|forge| {
                        names.of(refs, &sheet.name).read_dates(
                            &forge,
                            Context::Scalar { group },
                            Holds::Other,
                        )
                    }) {
                    Ok(forge) => Some(forge),
                    Err(reason) => {
                        warnings.push(format!("{path}: imported as a value ({reason}): {excel}"));
                        None
                    },
                },
            );

            // Its number format (ADR-061); a formula's is a date by Forge's rule
            let shows_date = is_date || names.of(refs, &sheet.name).date_scalars.contains(&path);
            let format = sheet.imported_format(&path, [(row, 1)], shows_date, warnings);
            let mut variable = Variable::new(path.clone(), value, formula);
            variable.is_date = is_date;
            variable.metadata.format = format;
            model.add_scalar(path, variable);
        }
    }

    /// Text, a boolean or an error among numbers is 0 in Forge: name the cells.
    pub(super) fn warn_not_numbers(
        sheet: &Sheet,
        col: usize,
        (table_name, col_name): (&str, &str),
        data: &[Data],
        warnings: &mut Vec<String>,
    ) {
        let not_numbers: Vec<String> = data
            .iter()
            .enumerate()
            .filter(|(_, cell)| !matches!(cell, Data::Empty) && number_of(cell).is_none())
            .map(|(i, cell)| format!("{}{} ({cell})", sheet.letter(col), sheet.excel_row(i + 1)))
            .collect();
        if !not_numbers.is_empty() {
            let shown = not_numbers.iter().take(5).cloned().collect::<Vec<_>>();
            let more = not_numbers.len() - shown.len();
            warnings.push(format!(
                "{table_name}.{col_name}: a column of numbers holds text or another \
                 non-number in {} imported as 0: {}{}",
                if not_numbers.len() == 1 {
                    "1 cell,"
                } else {
                    "several cells,"
                },
                shown.join(", "),
                if more > 0 {
                    format!(" and {more} more")
                } else {
                    String::new()
                }
            ));
        }
    }
}
