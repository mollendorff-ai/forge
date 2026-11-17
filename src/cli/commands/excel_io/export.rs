//! `forge export`: a model as an Excel workbook, its formulas as formulas.

use crate::error::ForgeResult;
use crate::excel::ExcelExporter;
use crate::parser;
// Printing the commands belongs to the command line (feature `cli`)
use std::path::Path;
#[cfg(feature = "cli")]
use {crate::cli::console::outln, crate::cli::Console, colored::Colorize};

/// Parse a model for export and calculate it, so a formula that Excel cannot
/// express can be written as its value. If calculation fails, the formulas are
/// still exported, without results, and the second part says why: what each
/// formula column holds (ADR-054) is known only from the results.
fn load_for_export(input: &Path) -> ForgeResult<(crate::types::ParsedModel, Option<String>)> {
    let model = parser::parse_model(input)?;
    let ragged = model.check_table_lengths().err();
    Ok(
        match crate::core::ArrayCalculator::new(model.clone()).calculate_all() {
            Ok(calculated) => (calculated, ragged),
            Err(error) => (
                model,
                Some(format!(
                    "the model does not calculate ({error}): the formulas are written without \
                     results, and a formula over dates or text may not give in Excel what it \
                     gives in Forge. Fix the error and export again"
                )),
            ),
        },
    )
}

/// Export YAML to Excel and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed or the Excel export fails.
pub fn export_core(
    input: &Path,
    output: &Path,
) -> ForgeResult<crate::cli::commands::results::ExportResult> {
    let (model, uncalculated) = load_for_export(input)?;
    let table_count = model.tables.len();
    let scalar_count = model.scalars.len();

    let exporter = ExcelExporter::new(model);
    let mut warnings: Vec<String> = uncalculated.into_iter().collect();
    warnings.extend(exporter.export_with_warnings(output)?);
    warnings.extend(crate::cli::commands::carried::yaml_only(input));

    Ok(crate::cli::commands::results::ExportResult {
        input_path: input.display().to_string(),
        output_path: output.display().to_string(),
        table_count,
        scalar_count,
        warnings,
    })
}

/// Export YAML to an in-memory Excel buffer and return base64-encoded content.
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed or the Excel buffer export fails.
#[cfg(feature = "cli")]
pub fn export_buffer_core(
    input: &Path,
) -> ForgeResult<crate::cli::commands::results::ExportBufferResult> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let (model, uncalculated) = load_for_export(input)?;
    let table_count = model.tables.len();
    let scalar_count = model.scalars.len();

    let exporter = ExcelExporter::new(model);
    let (buffer, written_as_values) = exporter.export_to_buffer_with_warnings()?;
    let base64_content = STANDARD.encode(&buffer);
    let mut warnings: Vec<String> = uncalculated.into_iter().collect();
    warnings.extend(written_as_values);
    warnings.extend(crate::cli::commands::carried::yaml_only(input));

    Ok(crate::cli::commands::results::ExportBufferResult {
        table_count,
        scalar_count,
        excel_base64: base64_content,
        byte_count: buffer.len(),
        warnings,
    })
}

/// A workbook made in memory, for a caller with no files (Forge Studio).
pub struct ExportBytes {
    /// The .xlsx file's bytes, as `forge export` would write them
    pub bytes: Vec<u8>,
    pub table_count: usize,
    pub scalar_count: usize,
    /// As `forge export`: the model does not calculate, a formula written as its value
    pub warnings: Vec<String>,
}

/// [`export_core`] into memory, with the same workbook and the same warnings.
///
/// # Errors
///
/// Returns an error if the model cannot be parsed or the workbook cannot be made.
pub fn export_bytes_core(input: &Path) -> ForgeResult<ExportBytes> {
    let (model, uncalculated) = load_for_export(input)?;
    let table_count = model.tables.len();
    let scalar_count = model.scalars.len();
    let (bytes, written_as_values) = ExcelExporter::new(model).export_to_buffer_with_warnings()?;
    let mut warnings: Vec<String> = uncalculated.into_iter().collect();
    warnings.extend(written_as_values);
    Ok(ExportBytes {
        bytes,
        table_count,
        scalar_count,
        warnings,
    })
}

/// Execute the export command.
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed or the Excel export fails.
#[cfg(feature = "cli")]
pub fn export(c: &mut Console<'_>, input: &Path, output: &Path, verbose: bool) -> ForgeResult<()> {
    outln!(c, "{}", "🔥 Forge - Excel Export".bold().green());
    outln!(c, "   Input:  {}", input.display());
    outln!(c, "   Output: {}\n", output.display());

    if verbose {
        outln!(c, "{}", "📖 Parsing YAML file...".cyan());
    }
    let (model, uncalculated) = load_for_export(input)?;
    if verbose {
        let (t, s) = (model.tables.len(), model.scalars.len());
        outln!(c, "   Found {t} tables, {s} scalars\n");
        outln!(c, "{}", "📊 Exporting to Excel...".cyan());
    }

    let exporter = ExcelExporter::new(model);
    let mut warnings = exporter.export_with_warnings(output)?;
    warnings.extend(crate::cli::commands::carried::yaml_only(input));

    if let Some(reason) = &uncalculated {
        outln!(c, "{}", "⚠️  Exported, but not checked:".yellow());
        outln!(c, "   - {reason}\n");
    }
    outln!(c, "{}", "✅ Export Complete!".bold().green());
    outln!(c, "   Excel file: {}\n", output.display());
    if !warnings.is_empty() {
        outln!(
            c,
            "{}",
            "⚠️  Not exported, or not as Excel formulas:".yellow()
        );
        for warning in &warnings {
            outln!(c, "   - {warning}");
        }
        outln!(c);
    }

    Ok(())
}
