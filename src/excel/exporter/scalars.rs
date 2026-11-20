//! The scalar sheets: one for each group of the model's scalars, and a
//! `{namespace}.Scalars` sheet for each included file.

use super::limits::{check_size, MAX_ROWS};
use super::ExcelExporter;
use crate::error::{ForgeError, ForgeResult};
use crate::excel::workbook_refs::{
    include_sheet, split_scalar_path, Context, WorkbookRefs, UNGROUPED_SHEET,
};
use crate::types::{ParsedModel, Variable};
use rust_xlsxwriter::{Note, Workbook};

/// One scalar of a sheet: what its row shows and the names its formula reads.
struct ScalarRow<'a> {
    /// What column A shows
    name: String,
    /// What a warning calls it
    label: String,
    /// The worksheet row, from 0
    row: u32,
    /// The scalar's path, whose group its formula's names are read in
    path: &'a str,
    var: &'a Variable,
    /// The names the formula is read with: the model's, or the included file's own
    refs: &'a WorkbookRefs,
}

impl ExcelExporter {
    /// Export scalars to grouped worksheets (one per prefix group)
    /// e.g., "utilities.extinction", "utilities.flourishing" -> worksheet "utilities";
    /// scalars without a group go to the "Scalars" worksheet.
    pub(super) fn export_scalars(
        &self,
        workbook: &mut Workbook,
        warnings: &mut Vec<String>,
    ) -> ForgeResult<()> {
        // Groups in the order of their first scalar, scalars in model order
        // (the same rows as `self.refs`).
        let mut groups: indexmap::IndexMap<Option<&str>, Vec<&String>> = indexmap::IndexMap::new();
        for path in self.model.scalars.keys() {
            groups
                .entry(split_scalar_path(path).0)
                .or_default()
                .push(path);
        }
        for (group, paths) in &groups {
            self.export_scalar_group(workbook, *group, paths, (warnings, MAX_ROWS))?;
        }
        Ok(())
    }

    /// Export one group of scalars to a worksheet: `Name` in A, `Value` in B
    fn export_scalar_group(
        &self,
        workbook: &mut Workbook,
        group: Option<&str>,
        paths: &[&String],
        (warnings, max_rows): (&mut Vec<String>, usize),
    ) -> ForgeResult<()> {
        let rows = paths.iter().map(|path| ScalarRow {
            name: split_scalar_path(path).1.to_string(),
            label: (*path).clone(),
            row: self.refs.scalars[path.as_str()].1 - 1,
            path,
            var: &self.model.scalars[*path],
            refs: &self.refs,
        });
        Self::write_scalar_sheet(
            workbook,
            group.unwrap_or(UNGROUPED_SHEET),
            rows.collect(),
            (warnings, max_rows),
        )
    }

    /// Export an included file's scalars to its `{namespace}.Scalars` sheet:
    /// `{namespace}.{path}` in A, the value or formula in B, in the included
    /// model's order (the rows `WorkbookRefs::add_include` points at). An
    /// included file with no scalars and no tables has no sheet.
    pub(super) fn export_namespaced_scalars(
        workbook: &mut Workbook,
        (namespace, included, own): (&str, &ParsedModel, &WorkbookRefs),
        warnings: &mut Vec<String>,
        max_rows: usize,
    ) -> ForgeResult<()> {
        if included.scalars.is_empty() && included.tables.is_empty() {
            return Ok(());
        }
        let rows = included.scalars.iter().map(|(path, var)| ScalarRow {
            name: format!("{namespace}.{path}"),
            label: format!("@{namespace}.{path}"),
            row: own.scalars[path.as_str()].1 - 1,
            path,
            var,
            refs: own,
        });
        Self::write_scalar_sheet(
            workbook,
            &include_sheet(namespace),
            rows.collect(),
            (warnings, max_rows),
        )
    }

    /// Write the rows of a sheet of scalars: a name, and a value or a
    /// formula. A formula Excel cannot express is written as its value, with
    /// a warning and a note.
    fn write_scalar_sheet(
        workbook: &mut Workbook,
        sheet: &str,
        rows: Vec<ScalarRow<'_>>,
        (warnings, max_rows): (&mut Vec<String>, usize),
    ) -> ForgeResult<()> {
        check_size(sheet, (rows.len() + 1, 2), max_rows)?;
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(sheet).map_err(|e| {
            ForgeError::Export(format!("Failed to set worksheet name '{sheet}': {e}"))
        })?;
        Self::write_name_value_header(worksheet);

        for scalar in rows {
            let ScalarRow {
                name,
                label,
                row,
                path,
                var,
                refs,
            } = scalar;
            worksheet
                .write_string(row, 0, name)
                .map_err(|e| ForgeError::Export(format!("Failed to write scalar name: {e}")))?;

            let mut notes: Vec<String> = Vec::new();
            let group = split_scalar_path(path).0;
            let translated = var
                .formula
                .as_ref()
                .map(|f| (f, refs.to_excel(f, Context::Scalar { group })));
            match translated {
                Some((_, Ok(t))) => Self::write_scalar_formula(worksheet, row, var, &t),
                Some((formula, Err(reason))) => {
                    let written = super::written_as(formula, var.value, &reason);
                    warnings.push(format!("{label}: {written}: {formula}"));
                    notes.push(format!("Forge formula: {formula}\n{written}"));
                    Self::write_scalar_value(worksheet, row, var);
                },
                None => Self::write_scalar_value(worksheet, row, var),
            }

            if let Some(note_text) = Self::format_metadata_note(&var.metadata) {
                notes.push(note_text);
            }
            if !notes.is_empty() {
                let note = Note::new(notes.join("\n\n")).set_author("Forge");
                worksheet
                    .insert_note(row, 1, &note)
                    .map_err(|e| ForgeError::Export(format!("Failed to add scalar note: {e}")))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Metadata;

    const TOO_LONG: &str = "String exceeds Excel's limit of 32,767 characters.";

    fn model(path: &str, variable: Variable) -> ParsedModel {
        let mut model = ParsedModel::new();
        model.scalars.insert(path.to_string(), variable);
        model
    }

    fn refused(model: ParsedModel) -> String {
        ExcelExporter::new(model)
            .export_to_buffer()
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_scalar_with_a_name_or_note_excel_cannot_hold_is_refused_with_the_reason() {
        let long = "x".repeat(33_000);
        let group = "g".repeat(40);
        assert_eq!(
            refused(model(
                &format!("{group}.n"),
                Variable::new("n".to_string(), Some(1.0), None)
            )),
            format!(
                "Excel export error: Failed to set worksheet name '{group}': Worksheet name 'worksheet.set_name(\"{group}\")' exceeds Excel's limit of 31 characters."
            )
        );
        assert_eq!(
            refused(model(
                &format!("g.{long}"),
                Variable::new("n".to_string(), Some(1.0), None)
            )),
            format!("Excel export error: Failed to write scalar name: {TOO_LONG}")
        );
        let mut noted = Variable::new("n".to_string(), Some(1.0), None);
        noted.metadata = Metadata {
            notes: Some(long),
            ..Metadata::default()
        };
        assert_eq!(
            refused(model("g.n", noted)),
            format!("Excel export error: Failed to add scalar note: {TOO_LONG}")
        );
        // A formula Excel cannot express is its value, with a note of the formula
        let draw = Variable::new(
            "n".to_string(),
            Some(1.0),
            Some(format!("=MC.Normal(1, 1){}", "+1".repeat(17_000))),
        );
        assert_eq!(
            refused(model("g.n", draw)),
            format!("Excel export error: Failed to add scalar note: {TOO_LONG}")
        );
    }

    #[test]
    fn a_group_with_more_scalars_than_a_sheet_has_rows_is_refused() {
        let mut model = ParsedModel::new();
        for name in ["g.a", "g.b", "g.c"] {
            model.scalars.insert(
                name.to_string(),
                Variable::new(name.to_string(), Some(1.0), None),
            );
        }
        let exporter = ExcelExporter::new(model);
        let paths: Vec<&String> = exporter.model.scalars.keys().collect();
        let mut workbook = Workbook::new();
        let mut warnings = Vec::new();
        // Three scalars and the header fit in four rows, and not in three
        exporter
            .export_scalar_group(&mut workbook, Some("g"), &paths, (&mut warnings, 4))
            .unwrap();
        let err = exporter
            .export_scalar_group(&mut workbook, Some("h"), &paths, (&mut warnings, 3))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Excel export error: h: 4 rows, the header's included, do not fit in an Excel worksheet, which holds 3"
        );
        assert_eq!(workbook.worksheets().len(), 1);
    }

    #[test]
    fn an_included_file_with_more_scalars_than_a_sheet_has_rows_is_refused() {
        let files = std::collections::HashMap::from([(
            "inc.yaml".to_string(),
            "_forge_version: \"5.4.0\"\nv:\n  a: 1\n  b: 2\n".to_string(),
        )]);
        let main = "_forge_version: \"5.4.0\"\n_includes:\n  - file: inc.yaml\n    as: inc\n";
        let exporter =
            ExcelExporter::new(crate::parser::parse_model_text("m.yaml", main, &files).unwrap());
        let (namespace, included, own) = &exporter.includes[0];
        let mut workbook = Workbook::new();
        let mut warnings = Vec::new();
        let err = ExcelExporter::export_namespaced_scalars(
            &mut workbook,
            (namespace, included, own),
            &mut warnings,
            2,
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Excel export error: inc.Scalars: 3 rows, the header's included, do not fit in an Excel worksheet, which holds 2"
        );
    }

    #[test]
    fn an_included_scalar_keeps_its_metadata_as_a_note() {
        let files = std::collections::HashMap::from([(
            "inc.yaml".to_string(),
            "_forge_version: \"5.4.0\"\nv:\n  n: {value: 2, unit: CAD}\n".to_string(),
        )]);
        let main = "_forge_version: \"5.4.0\"\n_includes:\n  - file: inc.yaml\n    as: inc\n";
        let bytes =
            ExcelExporter::new(crate::parser::parse_model_text("m.yaml", main, &files).unwrap())
                .export_to_buffer()
                .unwrap();
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut notes = String::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).unwrap();
            if file.name().unwrap().starts_with("xl/comments") {
                std::io::Read::read_to_string(&mut file, &mut notes).unwrap();
            }
        }
        assert!(notes.contains("Unit: CAD"), "{notes}");
    }
}
