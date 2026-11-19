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
            let name = sheet
                .values
                .get((row, 0))
                .map(ToString::to_string)
                .unwrap_or_default();
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
            let format = sheet.imported_format(&path, &[(row, 1)], shows_date, warnings);
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

#[cfg(test)]
mod tests {
    use super::*;
    use rust_xlsxwriter::{Format, Formula, Workbook, Worksheet};

    fn import(mut book: Workbook) -> (ParsedModel, Vec<String>) {
        let bytes = book.save_to_buffer().unwrap();
        ExcelImporter::from_bytes(bytes)
            .import_with_warnings()
            .unwrap()
    }

    /// A `Name` / `Value` sheet called `name`
    fn scalar_sheet<'a>(book: &'a mut Workbook, name: &str) -> &'a mut Worksheet {
        let sheet = book.add_worksheet().set_name(name).unwrap();
        sheet.write_string(0, 0, "Name").unwrap();
        sheet.write_string(0, 1, "Value").unwrap();
        sheet
    }

    #[test]
    fn a_boolean_is_one_or_zero_and_a_row_without_a_name_is_skipped() {
        let mut book = Workbook::new();
        let s = scalar_sheet(&mut book, "Scalars");
        s.write_string(1, 0, "flag").unwrap();
        s.write_boolean(1, 1, true).unwrap();
        s.write_number(2, 1, 7.0).unwrap();
        s.write_string(3, 0, "b").unwrap();
        s.write_number(3, 1, 2.0).unwrap();
        let (model, _) = import(book);
        let names: Vec<&String> = model.scalars.keys().collect();
        assert_eq!(names, ["flag", "b"]);
        assert_eq!(model.scalars["flag"].value, Some(1.0));
    }

    #[test]
    fn an_older_workbook_holds_the_formula_as_text_in_a_third_column() {
        let mut book = Workbook::new();
        let s = scalar_sheet(&mut book, "Scalars");
        s.write_string(0, 2, "Formula").unwrap();
        s.write_string(1, 0, "a").unwrap();
        s.write_number(1, 1, 3.0).unwrap();
        s.write_string(2, 0, "b").unwrap();
        s.write_number(2, 1, 6.0).unwrap();
        s.write_string(2, 2, "=a * 2").unwrap();
        let (model, _) = import(book);
        assert_eq!(model.scalars["a"].formula, None);
        assert_eq!(model.scalars["b"].formula.as_deref(), Some("=a * 2"));
    }

    #[test]
    fn a_formula_forge_cannot_read_is_imported_as_its_value() {
        let mut book = Workbook::new();
        let s = scalar_sheet(&mut book, "Scalars");
        s.write_string(1, 0, "x").unwrap();
        s.write_formula(1, 1, Formula::new("=Elsewhere!B2+1").set_result("5"))
            .unwrap();
        let (model, warnings) = import(book);
        assert_eq!(model.scalars["x"].formula, None);
        assert_eq!(model.scalars["x"].value, Some(5.0));
        assert!(
            warnings[0].starts_with("x: imported as a value ("),
            "{warnings:?}"
        );
        assert!(warnings[0].ends_with("=Elsewhere!B2+1"), "{warnings:?}");
    }

    #[test]
    fn a_date_cell_is_a_date_in_the_model_and_in_an_included_file() {
        let mut book = Workbook::new();
        let date = Format::new().set_num_format("d mmm yyyy");
        let s = scalar_sheet(&mut book, "Scalars");
        s.write_string(1, 0, "start").unwrap();
        s.write_number_with_format(1, 1, 46_023.0, &date).unwrap();
        // A formula reading the included file's date gives a date, so its
        // date format is kept
        s.write_string(2, 0, "next").unwrap();
        let next = Formula::new("=EDATE('costs.Scalars'!B2,1)").set_result("46054");
        s.write_formula_with_format(2, 1, next, &date).unwrap();
        let c = scalar_sheet(&mut book, "costs.Scalars");
        c.write_string(1, 0, "costs.opened").unwrap();
        c.write_number_with_format(1, 1, 46_023.0, &date).unwrap();
        let (model, warnings) = import(book);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(model.scalars["start"].is_date);
        let costs = &model.resolved_includes["costs"].model;
        assert!(costs.scalars["opened"].is_date);
        let next = &model.scalars["next"];
        assert_eq!(next.formula.as_deref(), Some("=EDATE(@costs.opened, 1)"));
        assert_eq!(next.metadata.format.as_deref(), Some("d mmm yyyy"));
    }
}
