//! Excel exporter implementation

mod cells;
mod layout;
mod limits;
mod scalars;

use super::text_results::{self, TextCell, TextResult};
use super::workbook_refs::{Context, WorkbookRefs};
use crate::error::{ForgeError, ForgeResult};
use crate::types::{ParsedModel, Table};
use limits::{check_size, MAX_ROWS};
use rust_xlsxwriter::{Formula, Note, Workbook};
use std::collections::HashMap;
use std::path::Path;

/// Excel exporter for v1.0.0 array models and v5.0.0 scalar models
pub struct ExcelExporter {
    model: ParsedModel,
    /// Sheet layout shared by every formula: where each column and scalar lives
    refs: WorkbookRefs,
    /// Included files by namespace, in name order: the calculated model and
    /// the layout its own formulas are written with
    includes: Vec<(String, ParsedModel, WorkbookRefs)>,
}

/// How a scalar formula that Excel cannot express is written, for its warning
/// and its note: an uncertain input (`MC.*`) as its plan value, the value
/// `forge simulate` shows the outputs at; any other formula as its calculated
/// value; and a formula with no value, in a model that does not calculate,
/// left empty.
fn written_as(formula: &str, value: Option<f64>, reason: &str) -> String {
    let uncertain = formula
        .trim_start_matches('=')
        .trim_start()
        .starts_with("MC.");
    match value {
        Some(plan) if uncertain => {
            format!("written as its plan value {plan} ({reason}; forge simulate draws it)")
        },
        Some(_) => format!("written as a value ({reason})"),
        None => format!("left empty ({reason})"),
    }
}

impl ExcelExporter {
    /// Create a new Excel exporter.
    ///
    /// Pass a calculated model if you have one: a formula that cannot be written
    /// as an Excel formula is exported as its calculated value.
    #[must_use]
    pub fn new(model: ParsedModel) -> Self {
        // Columns in model order: the letters must match what export_table writes.
        let tables = model.tables.iter().map(|(name, table)| {
            let columns: Vec<String> = table.column_names().into_iter().cloned().collect();
            (name.as_str(), columns, Self::row_count(table))
        });
        let mut refs = WorkbookRefs::new(tables, model.scalars.keys().map(String::as_str));
        refs.add_date_scalars(Self::date_scalars(&model));
        for (name, table) in &model.tables {
            refs.add_column_types(name, Self::column_types(table));
            refs.add_date_columns(name, Self::date_columns(table));
            refs.add_mixed_columns(name, Self::mixed_columns(table));
            refs.add_column_lengths(name, Self::column_lengths(table));
        }

        // `resolved_includes` is a HashMap, so sort the namespaces: with two or
        // more includes the sheet order would otherwise change between runs.
        let mut namespaces: Vec<&String> = model.resolved_includes.keys().collect();
        namespaces.sort();
        let includes = namespaces
            .into_iter()
            .map(|namespace| {
                let resolved = &model.resolved_includes[namespace];
                // Calculate the included model, so a value it defines by formula
                // is cached. If it does not calculate, write what the file
                // holds; the main model's calculation reports that error.
                let included = crate::core::ArrayCalculator::new(resolved.model.clone())
                    .calculate_all()
                    .unwrap_or_else(|_| resolved.model.clone());
                let paths = || included.scalars.keys().map(String::as_str);
                let tables = || {
                    included.tables.iter().map(|(name, table)| {
                        let columns: Vec<String> =
                            table.column_names().into_iter().cloned().collect();
                        (name.clone(), columns, Self::row_count(table))
                    })
                };
                // `@costs.labour.wage` in the main model is a cell on the
                // include's sheet, and `@costs.prices.flour` a column of the
                // include's table, so a value changed there flows through.
                refs.add_include(namespace, paths());
                refs.add_include_tables(namespace, tables());
                let dated: Vec<String> = Self::date_scalars(&included)
                    .map(|path| format!("@{namespace}.{path}"))
                    .collect();
                refs.add_date_scalars(dated.iter().map(String::as_str));
                let mut own = WorkbookRefs::for_include(namespace, paths(), tables());
                own.add_date_scalars(Self::date_scalars(&included));
                for (name, table) in &included.tables {
                    let key = format!("@{namespace}.{name}");
                    refs.add_column_types(&key, Self::column_types(table));
                    refs.add_date_columns(&key, Self::date_columns(table));
                    refs.add_mixed_columns(&key, Self::mixed_columns(table));
                    refs.add_column_lengths(&key, Self::column_lengths(table));
                    own.add_column_types(name, Self::column_types(table));
                    own.add_date_columns(name, Self::date_columns(table));
                    own.add_mixed_columns(name, Self::mixed_columns(table));
                    own.add_column_lengths(name, Self::column_lengths(table));
                }
                (namespace.clone(), included, own)
            })
            .collect();
        Self {
            model,
            refs,
            includes,
        }
    }

    /// Export the model to an Excel .xlsx file
    ///
    /// # Errors
    ///
    /// Returns an error if the workbook cannot be saved to the specified path.
    pub fn export(&self, output_path: &Path) -> ForgeResult<()> {
        self.export_with_warnings(output_path).map(|_| ())
    }

    /// Export to a file and return one warning per formula that was written as
    /// its value because Excel cannot express it.
    ///
    /// # Errors
    ///
    /// Returns an error if the workbook cannot be saved to the specified path.
    pub fn export_with_warnings(&self, output_path: &Path) -> ForgeResult<Vec<String>> {
        let (bytes, warnings) = self.export_to_buffer_with_warnings()?;
        std::fs::write(output_path, bytes)
            .map_err(|e| ForgeError::IO(format!("Failed to save Excel file: {e}")))?;
        Ok(warnings)
    }

    /// Export the model to an in-memory Excel buffer.
    ///
    /// # Errors
    ///
    /// Returns an error if the workbook cannot be serialized.
    pub fn export_to_buffer(&self) -> ForgeResult<Vec<u8>> {
        self.export_to_buffer_with_warnings()
            .map(|(bytes, _)| bytes)
    }

    /// Export to memory and return the warnings [`Self::export_with_warnings`] gives.
    ///
    /// # Errors
    ///
    /// Returns an error if the workbook cannot be serialized.
    pub fn export_to_buffer_with_warnings(&self) -> ForgeResult<(Vec<u8>, Vec<String>)> {
        let (mut workbook, warnings, text_results) = self.build_workbook()?;
        let bytes = workbook
            .save_to_buffer()
            .map_err(|e| ForgeError::IO(format!("Failed to create Excel buffer: {e}")))?;
        Ok((text_results::mark_as_text(bytes, &text_results), warnings))
    }

    fn build_workbook(&self) -> ForgeResult<(Workbook, Vec<String>, Vec<TextResult>)> {
        let mut workbook = Workbook::new();
        let mut warnings = Vec::new();
        let mut text_results = Vec::new();

        // Tables in model order, which is the same on every run
        for (table_name, table) in &self.model.tables {
            Self::export_table(
                &mut workbook,
                &self.refs,
                (table_name, table_name),
                table,
                (&mut warnings, &mut text_results),
            )?;
        }

        self.export_scalars(&mut workbook, &mut warnings)?;

        // Included files: tables on `{namespace}.{table}` sheets and scalars
        // on a `{namespace}.Scalars` sheet, each with its formulas in the
        // included file's own names.
        for (namespace, included, own) in &self.includes {
            for (table_name, table) in &included.tables {
                let sheet = format!("{namespace}.{table_name}");
                Self::export_table(
                    &mut workbook,
                    own,
                    (&sheet, table_name),
                    table,
                    (&mut warnings, &mut text_results),
                )?;
            }
            // Written even without scalars: it is how an import knows the
            // `{namespace}.{table}` sheets belong to an included file
            Self::export_namespaced_scalars(
                &mut workbook,
                (namespace, included, own),
                &mut warnings,
                MAX_ROWS,
            )?;
        }

        // Excel has no counterpart for scenarios (named sets of input overrides),
        // so they are not written. Say so, instead of dropping them silently.
        if !self.model.scenarios.is_empty() {
            let mut names: Vec<&str> = self.model.scenarios.keys().map(String::as_str).collect();
            names.sort_unstable();
            warnings.push(format!(
                "scenarios: not exported ({}); Excel has no equivalent, so keep the YAML file as the source of truth",
                names.join(", ")
            ));
        }

        // Excel cannot find an equilibrium. The payoff formulas are not
        // written, and the results are plain numbers as of the export (the
        // scalars of the calculated model), which do not follow the inputs.
        if !self.model.games.is_empty() {
            let mut names: Vec<&str> = self.model.games.keys().map(String::as_str).collect();
            names.sort_unstable();
            warnings.push(format!(
                "games: not exported ({}); Excel cannot find an equilibrium, so the results are written as numbers as of now and do not follow the inputs, and the payoff formulas are not written. Keep the YAML file as the source of truth",
                names.join(", ")
            ));
        }

        Ok((workbook, warnings, text_results))
    }

    /// Export a single table to a worksheet
    /// Write a table to its sheet. `names` is (the sheet, the table's name in
    /// `refs`); they differ for a table of an included file.
    fn export_table(
        workbook: &mut Workbook,
        refs: &WorkbookRefs,
        names: (&str, &str),
        table: &Table,
        out: (&mut Vec<String>, &mut Vec<TextResult>),
    ) -> ForgeResult<()> {
        let (sheet_name, table_name) = names;
        let (warnings, text_results) = out;
        let sheet = workbook.worksheets().len();
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(sheet_name)
            .map_err(|e| ForgeError::Export(format!("Failed to set worksheet name: {e}")))?;

        // Data and formula columns in model order (the same order as `self.refs`)
        let column_names: Vec<String> = table.column_names().into_iter().cloned().collect();

        let row_count = Self::row_count(table);
        // Past Excel's limits no cell can be written: say so once, up front
        check_size(sheet_name, (row_count + 1, column_names.len()), MAX_ROWS)?;
        Self::write_header(worksheet, table, &column_names)?;

        // The columns written as dates, found once (not once per cell)
        let date_columns: std::collections::HashSet<&str> = Self::date_columns(table).collect();
        let formats = Self::column_formats(table, &column_names);

        // Formula columns that Excel cannot express are written as values.
        let mut untranslatable: HashMap<&str, String> = HashMap::new();
        for (name, formula) in &table.row_formulas {
            let ctx = Context::Row {
                table: table_name,
                row: 2,
            };
            // A column of dates must hold dates in the workbook too
            let holds = Self::holds(table, name, &date_columns);
            let translated = refs.to_excel_as(formula, ctx, holds).map(|_| ());
            if let Err(reason) = translated {
                warnings.push(format!(
                    "{sheet_name}.{name}: written as values ({reason}): {formula}"
                ));
                untranslatable.insert(name.as_str(), reason);
            }
        }

        // Write data rows (starting at row 1). `excel_row` is the row as Excel
        // numbers it (the header is row 1); past Excel's limits the writer
        // refuses the cell, long before the counters end.
        for (row_idx, excel_row) in (0..row_count).zip(2u32..=u32::MAX) {
            for (col, col_name) in (0u16..=u16::MAX).zip(&column_names) {
                let formula = table.row_formulas.get(col_name);
                // The formula was translated for row 2 above, and what it
                // translates to does not depend on the row
                let translated = formula
                    .filter(|_| !untranslatable.contains_key(col_name.as_str()))
                    .and_then(|formula| {
                        let ctx = Context::Row {
                            table: table_name,
                            row: excel_row,
                        };
                        let holds = Self::holds(table, col_name, &date_columns);
                        refs.to_excel_as(formula, ctx, holds).ok()
                    });
                if let Some(translated) = translated {
                    // Cache Forge's result, so viewers that do not recalculate show it
                    let excel_formula = Self::with_cached_result(
                        Formula::new(&translated.formula),
                        table.columns.get(col_name),
                        (row_idx, translated.is_date),
                        (
                            TextCell {
                                sheet,
                                position: (excel_row - 1, col),
                            },
                            text_results,
                        ),
                    );
                    let format = formats.get(col_name.as_str());
                    match (format, translated.is_date) {
                        (Some(format), _) => worksheet.write_formula_with_format(
                            excel_row - 1,
                            col,
                            excel_formula,
                            format,
                        ),
                        (None, true) => worksheet.write_formula_with_format(
                            excel_row - 1,
                            col,
                            excel_formula,
                            &Self::date_format(),
                        ),
                        (None, false) => worksheet.write_formula(excel_row - 1, col, excel_formula),
                    }
                    .expect(limits::CHECKED);
                } else if let Some(column) = table.columns.get(col_name) {
                    // Data value, or the calculated value of an untranslatable formula
                    Self::write_cell_value(
                        worksheet,
                        (excel_row - 1, col),
                        &column.values,
                        row_idx,
                        (
                            date_columns.contains(col_name.as_str()),
                            formats.get(col_name.as_str()),
                        ),
                    )?;
                }
                if row_idx == 0 {
                    if let (Some(formula), Some(reason)) =
                        (formula, untranslatable.get(col_name.as_str()))
                    {
                        let note = Note::new(format!(
                            "Forge formula: {formula}\nWritten as values: {reason}"
                        ))
                        .set_author("Forge");
                        worksheet
                            .insert_note(excel_row - 1, col, &note)
                            .map_err(|e| ForgeError::Export(format!("Failed to add note: {e}")))?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Column, Variable};
    use crate::types::{ColumnValue, Metadata};

    // =========================================================================
    // Metadata Note Formatting Tests
    // =========================================================================

    // =========================================================================
    // ExcelExporter Construction Tests
    // =========================================================================

    #[test]
    fn test_exporter_new_empty_model() {
        let model = ParsedModel::new();
        let exporter = ExcelExporter::new(model);
        assert!(exporter.refs.columns.is_empty());
        assert!(exporter.refs.rows.is_empty());
    }

    #[test]
    fn test_exporter_new_with_table() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        assert!(exporter.refs.columns.contains_key("sales"));
        assert_eq!(exporter.refs.rows.get("sales"), Some(&3));
    }

    #[test]
    fn test_exporter_new_with_row_formula() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("calc".to_string());
        table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0]),
        ));
        table.add_row_formula("total".to_string(), "=SUM(amount)".to_string());
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        // Column map should include both data column and formula column
        let col_map = exporter.refs.columns.get("calc").unwrap();
        assert!(col_map.contains_key("amount"));
        assert!(col_map.contains_key("total"));
    }

    #[test]
    fn test_exporter_new_multiple_tables() {
        let mut model = ParsedModel::new();

        let mut table1 = Table::new("sales".to_string());
        table1.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        ));
        model.add_table(table1);

        let mut table2 = Table::new("costs".to_string());
        table2.add_column(Column::new(
            "expense".to_string(),
            ColumnValue::Number(vec![50.0, 75.0, 100.0]),
        ));
        model.add_table(table2);

        let exporter = ExcelExporter::new(model);

        assert!(exporter.refs.columns.contains_key("sales"));
        assert!(exporter.refs.columns.contains_key("costs"));
        assert_eq!(exporter.refs.rows.get("sales"), Some(&2));
        assert_eq!(exporter.refs.rows.get("costs"), Some(&3));
    }

    #[test]
    fn test_exporter_column_maps_keep_model_order() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "zebra".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        table.add_column(Column::new(
            "alpha".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        table.add_column(Column::new(
            "beta".to_string(),
            ColumnValue::Number(vec![3.0]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);
        let col_map = exporter.refs.columns.get("data").unwrap();

        // zebra -> A, alpha -> B, beta -> C (the order the columns were added)
        assert_eq!(col_map.get("zebra"), Some(&"A".to_string()));
        assert_eq!(col_map.get("alpha"), Some(&"B".to_string()));
        assert_eq!(col_map.get("beta"), Some(&"C".to_string()));
    }

    #[test]
    fn test_exporter_empty_table() {
        let mut model = ParsedModel::new();
        let table = Table::new("empty".to_string());
        model.add_table(table);

        let exporter = ExcelExporter::new(model);
        assert_eq!(exporter.refs.rows.get("empty"), Some(&0));
    }

    // =========================================================================
    // Excel Export Tests (File I/O)
    // =========================================================================

    #[test]
    fn test_export_empty_model() {
        use tempfile::TempDir;

        let model = ParsedModel::new();
        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("empty.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_single_table() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("sales.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());

        // File should have non-zero size
        let metadata = std::fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_export_with_text_column() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "names".to_string(),
            ColumnValue::Text(vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("names.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_date_column() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("timeline".to_string());
        table.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec![
                "2024-01-01".to_string(),
                "2024-02-01".to_string(),
                "2024-03-01".to_string(),
            ]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("dates.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_boolean_column() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("flags".to_string());
        table.add_column(Column::new(
            "active".to_string(),
            ColumnValue::Boolean(vec![true, false, true]),
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("flags.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_row_formula() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("calc".to_string());
        table.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        table.add_column(Column::new(
            "quantity".to_string(),
            ColumnValue::Number(vec![2.0, 3.0, 4.0]),
        ));
        table.add_row_formula("total".to_string(), "=price * quantity".to_string());
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("calculated.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_scalars() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        model.add_scalar(
            "tax_rate".to_string(),
            Variable::new("tax_rate".to_string(), Some(0.15), None),
        );
        model.add_scalar(
            "profit".to_string(),
            Variable::new(
                "profit".to_string(),
                Some(50000.0),
                Some("=revenue - costs".to_string()),
            ),
        );

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("scalars.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_multiple_tables() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        let mut revenue_table = Table::new("revenue".to_string());
        revenue_table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![1000.0, 2000.0, 3000.0]),
        ));
        model.add_table(revenue_table);

        let mut costs_table = Table::new("costs".to_string());
        costs_table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![500.0, 750.0, 1000.0]),
        ));
        model.add_table(costs_table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("multi.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_with_metadata() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        let metadata = Metadata {
            unit: Some("CAD".to_string()),
            notes: Some("Revenue data".to_string()),
            source: Some("finance.yaml".to_string()),
            validation_status: Some("VALIDATED".to_string()),
            last_updated: Some("2024-01-01".to_string()),
            better: None,
            format: None,
        };

        table.add_column(Column::with_metadata(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
            metadata,
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("metadata.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_mixed_column_types() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("mixed".to_string());

        table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        ));
        table.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec!["A".to_string(), "B".to_string()]),
        ));
        table.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec!["2024-01-01".to_string(), "2024-02-01".to_string()]),
        ));
        table.add_column(Column::new(
            "active".to_string(),
            ColumnValue::Boolean(vec![true, false]),
        ));

        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("mixed.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_to_nonexistent_directory_fails() {
        let model = ParsedModel::new();
        let exporter = ExcelExporter::new(model);

        let output_path = std::path::Path::new("/nonexistent/dir/output.xlsx");

        let result = exporter.export(output_path);
        assert!(result.is_err());
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // Additional coverage tests
    // ═══════════════════════════════════════════════════════════════════════════

    #[test]
    fn test_export_with_row_formulas() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        ));
        table.add_column(Column::new(
            "quantity".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));

        // Add a formula column
        table
            .row_formulas
            .insert("total".to_string(), "=price * quantity".to_string());

        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("formulas.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_scalars_with_formulas() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Add input scalar (no formula)
        model.scalars.insert(
            "inputs.rate".to_string(),
            crate::types::Variable::new("inputs.rate".to_string(), Some(0.05), None),
        );

        // Add output scalar (with formula)
        model.scalars.insert(
            "outputs.result".to_string(),
            crate::types::Variable::new(
                "outputs.result".to_string(),
                Some(500.0),
                Some("=inputs.rate * 10000".to_string()),
            ),
        );

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("scalars.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_aggregations() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Add a table
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);

        // Add aggregation
        model
            .aggregations
            .insert("total_sales".to_string(), "=SUM(sales.amount)".to_string());

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("aggregations.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_empty_table() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let table = Table::new("empty".to_string());
        // Don't add any columns
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("empty.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_large_table() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("large".to_string());

        // Create 1000 row table
        let values: Vec<f64> = (0..1000).map(f64::from).collect();
        table.add_column(Column::new("id".to_string(), ColumnValue::Number(values)));

        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("large.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_cross_table_formula() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // First table
        let mut revenue = Table::new("revenue".to_string());
        revenue.add_column(Column::new(
            "amount".to_string(),
            ColumnValue::Number(vec![1000.0, 2000.0]),
        ));
        model.add_table(revenue);

        // Second table referencing first
        let mut profit = Table::new("profit".to_string());
        profit.add_column(Column::new(
            "margin".to_string(),
            ColumnValue::Number(vec![0.2, 0.3]),
        ));
        profit
            .row_formulas
            .insert("amount".to_string(), "=revenue.amount * margin".to_string());
        model.add_table(profit);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("cross_table.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_all_metadata_fields() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        let mut table = Table::new("complete".to_string());

        // Full metadata
        let metadata = Metadata {
            unit: Some("USD".to_string()),
            notes: Some("Complete metadata test".to_string()),
            source: Some("test.yaml".to_string()),
            validation_status: Some("PENDING".to_string()),
            last_updated: Some("2025-12-04".to_string()),
            better: None,
            format: None,
        };

        table.add_column(Column::with_metadata(
            "value".to_string(),
            ColumnValue::Number(vec![42.0]),
            metadata,
        ));
        model.add_table(table);

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("full_metadata.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    // =========================================================================
    // Grouped Scalar Export Tests (v5.0.0 scalar models)
    // =========================================================================

    #[test]
    fn test_export_grouped_scalars() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Add scalars in different groups (simulating v5.0.0 scalar model)
        // Group: utilities
        model.scalars.insert(
            "utilities.extinction".to_string(),
            Variable::new("utilities.extinction".to_string(), Some(0.0), None),
        );
        model.scalars.insert(
            "utilities.flourishing".to_string(),
            Variable::new("utilities.flourishing".to_string(), Some(100.0), None),
        );

        // Group: scenario_probs
        model.scalars.insert(
            "scenario_probs.p_unaligned".to_string(),
            Variable::new("scenario_probs.p_unaligned".to_string(), Some(0.35), None),
        );
        model.scalars.insert(
            "scenario_probs.p_aligned".to_string(),
            Variable::new("scenario_probs.p_aligned".to_string(), Some(0.25), None),
        );

        // Group: analysis (with formula referencing other groups)
        model.scalars.insert(
            "analysis.expected_value".to_string(),
            Variable::new(
                "analysis.expected_value".to_string(),
                Some(25.0),
                Some("=scenario_probs.p_aligned * utilities.flourishing".to_string()),
            ),
        );

        let exporter = ExcelExporter::new(model);

        // Verify scalar locations were built correctly
        assert!(exporter.refs.scalars.contains_key("utilities.extinction"));
        assert!(exporter
            .refs
            .scalars
            .contains_key("scenario_probs.p_unaligned"));
        assert!(exporter
            .refs
            .scalars
            .contains_key("analysis.expected_value"));

        // Verify worksheet assignments
        let util_loc = exporter.refs.scalars.get("utilities.extinction").unwrap();
        assert_eq!(util_loc.0, "utilities");

        let prob_loc = exporter
            .refs
            .scalars
            .get("scenario_probs.p_unaligned")
            .unwrap();
        assert_eq!(prob_loc.0, "scenario_probs");

        let analysis_loc = exporter
            .refs
            .scalars
            .get("analysis.expected_value")
            .unwrap();
        assert_eq!(analysis_loc.0, "analysis");

        // Export and verify file creation
        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("grouped_scalars.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_export_scalars_without_dots() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Scalars without dots should go to "Scalars" worksheet
        model.scalars.insert(
            "tax_rate".to_string(),
            Variable::new("tax_rate".to_string(), Some(0.15), None),
        );
        model.scalars.insert(
            "discount_rate".to_string(),
            Variable::new("discount_rate".to_string(), Some(0.10), None),
        );

        let exporter = ExcelExporter::new(model);

        // Verify they go to "Scalars" worksheet
        let tax_loc = exporter.refs.scalars.get("tax_rate").unwrap();
        assert_eq!(tax_loc.0, "Scalars");

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("ungrouped_scalars.xlsx");

        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_warns_when_scenarios_are_not_exported() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        model.scalars.insert(
            "rate".to_string(),
            Variable::new("rate".to_string(), Some(0.1), None),
        );
        for name in ["bull", "base"] {
            model.scenarios.insert(
                name.to_string(),
                crate::types::Scenario {
                    overrides: std::collections::HashMap::from([("rate".to_string(), 0.2)]),
                    description: None,
                    dates: std::collections::BTreeSet::new(),
                    expect: Vec::new(),
                },
            );
        }

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("with_scenarios.xlsx");
        let warnings = ExcelExporter::new(model)
            .export_with_warnings(&path)
            .unwrap();

        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(
            warnings,
            ["scenarios: not exported (base, bull); Excel has no equivalent, so keep the YAML file as the source of truth"]
        );
    }

    /// Excel cannot find an equilibrium, so a game's payoff formulas are not
    /// written and its results are numbers as of the export: changing an
    /// input in Excel would not move them. The export says so (ADR-068).
    #[test]
    fn a_game_is_not_exported_and_the_export_says_so() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("game.yaml");
        std::fs::write(
            &path,
            "_forge_version: \"5.5.0\"
c: 0.5
games:
  stag_hunt:
    players: [me, you]
    strategies: [stag, hare]
    symmetric: true
    payoffs:
      stag: {stag: 4, hare: 0}
      hare: {stag: =3 - c, hare: =3 - c}
  chicken:
    players: [me, you]
    strategies: [swerve, straight]
    symmetric: true
    payoffs:
      swerve: {swerve: 3, straight: 1}
      straight: {swerve: 5, straight: 0}
",
        )
        .unwrap();
        let model = crate::parser::parse_model(&path).unwrap();
        let model = crate::core::ArrayCalculator::new(model)
            .calculate_all()
            .unwrap();
        let out = dir.path().join("game.xlsx");
        let warnings = ExcelExporter::new(model)
            .export_with_warnings(&out)
            .unwrap();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(
            warnings[0],
            "games: not exported (chicken, stag_hunt); Excel cannot find an equilibrium, so the \
             results are written as numbers as of now and do not follow the inputs, and the \
             payoff formulas are not written. Keep the YAML file as the source of truth"
        );
    }

    #[test]
    fn test_export_without_scenarios_has_no_scenario_warning() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();
        model.scalars.insert(
            "rate".to_string(),
            Variable::new("rate".to_string(), Some(0.1), None),
        );

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("no_scenarios.xlsx");
        let warnings = ExcelExporter::new(model)
            .export_with_warnings(&path)
            .unwrap();

        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn test_translate_grouped_scalar_formula() {
        use super::super::workbook_refs::Context;
        let mut model = ParsedModel::new();
        model.scalars.insert(
            "utilities.flourishing".to_string(),
            Variable::new("utilities.flourishing".to_string(), Some(100.0), None),
        );
        model.scalars.insert(
            "scenario_probs.p_aligned".to_string(),
            Variable::new("scenario_probs.p_aligned".to_string(), Some(0.25), None),
        );
        let exporter = ExcelExporter::new(model);
        let t = exporter
            .refs
            .to_excel(
                "=scenario_probs.p_aligned * utilities.flourishing",
                Context::Scalar {
                    group: Some("analysis"),
                },
            )
            .unwrap();
        assert_eq!(t.formula, "='scenario_probs'!$B$2 * 'utilities'!$B$2");
    }

    #[test]
    fn test_export_scalar_with_mc_formula() {
        use tempfile::TempDir;

        let mut model = ParsedModel::new();

        // Scalar with Monte Carlo formula (not supported in Excel)
        model.scalars.insert(
            "inputs.probability".to_string(),
            Variable::new(
                "inputs.probability".to_string(),
                Some(0.35), // Calculated value
                Some("=MC.Triangular(0.25, 0.35, 0.45)".to_string()),
            ),
        );

        let exporter = ExcelExporter::new(model);

        let dir = TempDir::new().unwrap();
        let output_path = dir.path().join("mc_scalar.xlsx");

        // Should succeed - MC formulas are written as values with formula in comment
        let result = exporter.export(&output_path);
        assert!(result.is_ok());
    }

    use calamine::{Data, Reader, Xlsx};

    const VERSION: &str = "_forge_version: \"5.4.0\"\n";

    /// A model with a file it includes, held in memory
    fn included(main: &str, inc: &str) -> ParsedModel {
        let files = HashMap::from([("inc.yaml".to_string(), format!("{VERSION}{inc}"))]);
        let main = format!("{VERSION}_includes:\n  - file: inc.yaml\n    as: {main}");
        crate::parser::parse_model_text("m.yaml", &main, &files).unwrap()
    }

    fn workbook(model: ParsedModel) -> (Xlsx<std::io::Cursor<Vec<u8>>>, Vec<String>) {
        let (bytes, warnings) = ExcelExporter::new(model)
            .export_to_buffer_with_warnings()
            .unwrap();
        (Xlsx::new(std::io::Cursor::new(bytes)).unwrap(), warnings)
    }

    fn refused(model: ParsedModel) -> String {
        ExcelExporter::new(model)
            .export_to_buffer()
            .unwrap_err()
            .to_string()
    }

    fn one_column(table_name: &str, column: &str) -> Table {
        let mut table = Table::new(table_name.to_string());
        table.add_column(Column::new(
            column.to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        table
    }

    fn scalar_model(path: &str, variable: Variable) -> ParsedModel {
        let mut model = ParsedModel::new();
        model.scalars.insert(path.to_string(), variable);
        model
    }

    const TOO_LONG: &str = "String exceeds Excel's limit of 32,767 characters.";

    #[test]
    fn a_buffer_is_a_workbook() {
        let mut model = ParsedModel::new();
        model.add_table(one_column("t", "a"));
        let bytes = ExcelExporter::new(model).export_to_buffer().unwrap();
        assert_eq!(&bytes[..2], b"PK");
    }

    #[test]
    fn a_table_that_does_not_fit_a_worksheet_is_refused_before_it_is_written() {
        let mut wide = Table::new("t".to_string());
        for i in 0..=16_384 {
            wide.add_column(Column::new(format!("c{i}"), ColumnValue::Number(vec![1.0])));
        }
        let mut model = ParsedModel::new();
        model.add_table(wide);
        assert_eq!(
            refused(model),
            "Excel export error: t: 16385 columns do not fit in an Excel worksheet, which holds 16384"
        );
        let mut tall = Table::new("t".to_string());
        tall.add_column(Column::new(
            "a".to_string(),
            ColumnValue::Number(vec![1.0; 1_048_576]),
        ));
        let mut model = ParsedModel::new();
        model.add_table(tall);
        assert_eq!(
            refused(model),
            "Excel export error: t: 1048577 rows, the header's included, do not fit in an Excel worksheet, which holds 1048576"
        );
    }

    #[test]
    fn a_table_with_a_name_or_text_excel_cannot_hold_is_refused_with_the_reason() {
        let long = "x".repeat(33_000);
        let name = "t".repeat(40);
        let mut model = ParsedModel::new();
        model.add_table(one_column(&name, "a"));
        assert_eq!(
            refused(model),
            format!(
                "Excel export error: Failed to set worksheet name: Worksheet name 'worksheet.set_name(\"{name}\")' exceeds Excel's limit of 31 characters."
            )
        );
        let mut model = ParsedModel::new();
        model.add_table(one_column("t", &long));
        assert_eq!(
            refused(model),
            format!("Excel export error: Failed to write header: {TOO_LONG}")
        );
        let mut text = Table::new("t".to_string());
        text.add_column(Column::new("a".to_string(), ColumnValue::Text(vec![long])));
        let mut model = ParsedModel::new();
        model.add_table(text);
        assert_eq!(
            refused(model),
            format!("Excel export error: Failed to write text: {TOO_LONG}")
        );
        // A formula Excel cannot express is a value with a note of the formula
        let mut formula = one_column("t", "a");
        formula.add_row_formula(
            "b".to_string(),
            format!("=MC.Normal(a, 1){}", "+1".repeat(17_000)),
        );
        let mut model = ParsedModel::new();
        model.add_table(formula);
        assert_eq!(
            refused(model),
            format!("Excel export error: Failed to add note: {TOO_LONG}")
        );
    }

    #[test]
    fn a_table_and_a_scalar_group_with_one_name_are_refused_when_the_workbook_is_saved() {
        let mut model = scalar_model("T.n", Variable::new("n".to_string(), Some(1.0), None));
        model.add_table(one_column("t", "a"));
        assert_eq!(
            refused(model),
            "IO error: Failed to create Excel buffer: Worksheet name 't' has already been used in this workbook."
        );
    }

    #[test]
    fn a_model_that_cannot_be_written_leaves_no_file() {
        let mut model = ParsedModel::new();
        model.add_table(one_column(&"t".repeat(40), "a"));
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("out.xlsx");
        let err = ExcelExporter::new(model).export(&path).unwrap_err();
        assert!(
            err.to_string()
                .starts_with("Excel export error: Failed to set worksheet name"),
            "{err}"
        );
        assert!(!path.exists());
    }

    #[test]
    fn a_formula_column_without_data_and_an_empty_column_hold_nothing_known() {
        let mut table = one_column("t", "a");
        table.add_column(Column::new(
            "empty".to_string(),
            ColumnValue::Number(vec![]),
        ));
        let dates = std::collections::HashSet::new();
        assert_eq!(
            ExcelExporter::holds(&table, "empty", &dates),
            crate::excel::workbook_refs::dates::Holds::Unknown
        );
        assert_eq!(
            ExcelExporter::holds(&table, "none", &dates),
            crate::excel::workbook_refs::dates::Holds::Unknown
        );
        assert_eq!(
            ExcelExporter::holds(&table, "a", &dates),
            crate::excel::workbook_refs::dates::Holds::Numbers
        );
    }

    #[test]
    fn an_included_file_has_a_sheet_for_its_scalars_and_one_for_each_table() {
        let (mut book, warnings) = workbook(included(
            "inc\ns:\n  x: 1\n",
            "v:\n  d: 2026-10-31\n  n: {value: 2, notes: \"two\"}\n  f: {formula: \"=MC.Normal(1, 1)\"}\n  g: {formula: \"=n * 2\"}\nt:\n  a: [1, 2]\n",
        ));
        assert_eq!(
            warnings,
            ["@inc.v.f: left empty (function MC.Normal has no Excel equivalent): =MC.Normal(1, 1)"]
        );
        assert_eq!(book.sheet_names(), ["s", "inc.t", "inc.Scalars"]);
        let scalars = book.worksheet_range("inc.Scalars").unwrap();
        let names: Vec<String> = scalars.rows().map(|r| r[0].to_string()).collect();
        assert_eq!(names, ["Name", "inc.v.d", "inc.v.n", "inc.v.f", "inc.v.g"]);
        assert_eq!(scalars.get_value((2, 1)), Some(&Data::Float(2.0)));
    }

    #[test]
    fn an_included_file_with_nothing_in_it_has_no_sheet_and_one_with_only_a_table_has_two() {
        let (book, warnings) = workbook(included("inc\ns:\n  x: 1\n", ""));
        assert_eq!(book.sheet_names(), ["s"]);
        assert_eq!(warnings, Vec::<String>::new());
        let (book, _) = workbook(included("inc\ns:\n  x: 1\n", "t:\n  a: [1, 2]\n"));
        assert_eq!(book.sheet_names(), ["s", "inc.t", "inc.Scalars"]);
    }

    #[test]
    fn an_included_file_whose_sheets_cannot_be_named_is_refused() {
        let long = "i".repeat(30);
        let reason = |sheet: &str| {
            format!(
                "Worksheet name 'worksheet.set_name(\"{sheet}\")' exceeds Excel's limit of 31 characters."
            )
        };
        assert_eq!(
            refused(included(&format!("{long}\ns:\n  x: 1\n"), "v:\n  n: 2\n")),
            format!(
                "Excel export error: Failed to set worksheet name '{long}.Scalars': {}",
                reason(&format!("{long}.Scalars"))
            )
        );
        assert_eq!(
            refused(included(&format!("{long}\ns:\n  x: 1\n"), "t:\n  a: [1]\n")),
            format!(
                "Excel export error: Failed to set worksheet name: {}",
                reason(&format!("{long}.t"))
            )
        );
    }
}
