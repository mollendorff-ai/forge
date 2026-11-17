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
///
/// An uncertain input (`MC.*`) has no Excel equivalent and no value of its own:
/// it is calculated, and written, at its plan value, the one `forge simulate`
/// shows the outputs at, so the workbook is the plan.
fn load_for_export(input: &Path) -> ForgeResult<(crate::types::ParsedModel, Option<String>)> {
    let mut model = parser::parse_model(input)?;
    let ragged = model.check_table_lengths().err();
    let plan: Vec<(String, f64, Option<String>)> = plan_values(&model)
        .into_iter()
        .map(|(name, plan)| {
            let formula = formula_of(&model, &name);
            (name, plan.value, formula)
        })
        .collect();
    // Each input keeps its formula, for the note, and its plan value; the
    // calculator, which cannot draw it, sees the value alone
    let mut at_plan = model.clone();
    for (name, value, formula) in &plan {
        let _ = model.override_scalar(name, Some(*value), formula.clone());
        let _ = at_plan.override_scalar(name, Some(*value), None);
    }
    Ok(
        match crate::core::ArrayCalculator::new(at_plan).calculate_all() {
            Ok(mut calculated) => {
                for (name, value, formula) in &plan {
                    let _ = calculated.override_scalar(name, Some(*value), formula.clone());
                }
                (calculated, ragged)
            },
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

/// The model's uncertain inputs and those of its included files
/// (`@namespace.name`), with their plan values: none when a distribution does
/// not parse, and the model then does not calculate, which export says.
fn plan_values(
    model: &crate::types::ParsedModel,
) -> indexmap::IndexMap<String, crate::monte_carlo::engine::PlanInput> {
    crate::monte_carlo::engine::MonteCarloEngine::new(
        crate::monte_carlo::MonteCarloConfig::default(),
    )
    .and_then(|mut engine| {
        engine
            .parse_distributions_from_model(model)
            .map(|()| engine.plan_inputs())
    })
    .unwrap_or_default()
}

/// The formula of scalar `name`, of the model or of an included file
/// (`@namespace.name`)
fn formula_of(model: &crate::types::ParsedModel, name: &str) -> Option<String> {
    let (scalars, key) = name
        .strip_prefix('@')
        .and_then(|rest| rest.split_once('.'))
        .map_or((&model.scalars, name), |(namespace, key)| {
            (&model.resolved_includes[namespace].model.scalars, key)
        });
    scalars[key].formula.clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    mod uncalculated {
        //! `forge export` of a model that does not calculate says so (roadmap
        //! T1-EXPORT-UNCALCULATED). It said "Export Complete" and nothing else, though
        //! the formulas were then written without results, and what a formula column
        //! holds is known only from the results (ADR-054).

        use super::*;
        use crate::cli::forge;

        const FAILING: &str = r#"_forge_version: "5.2.0"
t:
  d: ["2026-01-31", "2026-02-28"]
  next: "=EDATE(d, 1)"
  y4: "=LEFT(next, 4)"
r:
  missing: {formula: "=MATCH(99, t.d, 0)"}
"#;

        const FINE: &str = r#"_forge_version: "5.2.0"
t:
  d: ["2026-01-31", "2026-02-28"]
  next: "=EDATE(d, 1)"
r:
  found: {formula: "=MATCH(\"2026-02-28\", t.d, 0)"}
"#;

        fn model(dir: &std::path::Path, text: &str) -> std::path::PathBuf {
            let path = dir.join("m.yaml");
            std::fs::write(&path, text).unwrap();
            path
        }

        #[test]
        fn a_model_that_does_not_calculate_is_exported_with_the_error_named() {
            let dir = tempfile::tempdir().unwrap();
            let input = model(dir.path(), FAILING);
            let result = export_core(&input, &dir.path().join("m.xlsx")).unwrap();
            assert!(dir.path().join("m.xlsx").exists());
            let first = &result.warnings[0];
            for expected in ["does not calculate", "MATCH: value not found", "r.missing"] {
                assert!(first.contains(expected), "{first}");
            }
        }

        #[test]
        fn the_buffer_export_says_it_too() {
            let dir = tempfile::tempdir().unwrap();
            let input = model(dir.path(), FAILING);
            let result = export_buffer_core(&input).unwrap();
            assert!(result.warnings[0].contains("does not calculate"));
        }

        #[test]
        fn a_model_that_calculates_has_no_such_warning() {
            let dir = tempfile::tempdir().unwrap();
            let input = model(dir.path(), FINE);
            let result = export_core(&input, &dir.path().join("m.xlsx")).unwrap();
            assert_eq!(result.warnings, Vec::<String>::new());
            assert_eq!(
                export_buffer_core(&input).unwrap().warnings,
                Vec::<String>::new()
            );
        }

        #[test]
        fn the_command_prints_it_before_export_complete() {
            let dir = tempfile::tempdir().unwrap();
            let input = model(dir.path(), FAILING);
            let output = dir.path().join("m.xlsx");
            let ran = forge(&["export", input.to_str().unwrap(), output.to_str().unwrap()]);
            assert!(ran.success(), "{}", ran.both());
            let warned = ran.out.find("does not calculate").expect(&ran.out);
            let done = ran.out.find("Export Complete").expect(&ran.out);
            assert!(warned < done, "{}", ran.out);
        }
    }

    mod uncertain_inputs {
        //! An `MC.*` input has no Excel equivalent and no calculated value, so
        //! `forge export` left its cell empty while saying it wrote it as a
        //! value, and every formula that read it ran on nothing in Excel; the
        //! model did not calculate, so nothing was checked. Each uncertain
        //! input is now written as its plan value, the one `forge simulate`
        //! shows its outputs at: the workbook is the plan.
        //!
        //! examples/monte-carlo.yaml at plan, in Python: growth 0.15 (the
        //! normal's mean), costs 500000 (the triangular's mode), rate 0.10
        //! (the uniform's midpoint):
        //! ```python
        //! cf = []
        //! for y in range(1, 6):
        //!     e = 1e6 * 1.15 ** y - 500000 * (1 + 0.03 * (y - 1)); cf.append(e - max(0, e * 0.25))
        //! sum(f / 1.1 ** (i + 1) for i, f in enumerate(cf))  # 2794690.957755616
        //! ```

        use super::*;
        use calamine::{open_workbook, Data, Reader, Xlsx};

        fn cell(book: &mut Xlsx<std::io::BufReader<std::fs::File>>, sheet: &str, row: u32) -> f64 {
            match book.worksheet_range(sheet).unwrap().get_value((row, 1)) {
                Some(Data::Float(f)) => *f,
                other => panic!("{sheet} row {row}: {other:?}"),
            }
        }

        #[test]
        fn each_uncertain_input_is_written_as_its_plan_value() {
            let dir = tempfile::tempdir().unwrap();
            let output = dir.path().join("mc.xlsx");
            let result = export_core(Path::new("examples/monte-carlo.yaml"), &output).unwrap();
            let warnings = result.warnings.join("\n");
            assert!(!warnings.contains("does not calculate"), "{warnings}");
            assert!(
                warnings.contains(
                    "assumptions.revenue_growth: written as its plan value 0.15 (function \
                     MC.Normal has no Excel equivalent; forge simulate draws it): \
                     =MC.Normal(0.15, 0.05)"
                ),
                "{warnings}"
            );

            let mut book: Xlsx<_> = open_workbook(&output).unwrap();
            let plan = [(1, 0.15), (2, 500_000.0), (3, 0.10)];
            for (row, want) in plan {
                let got = cell(&mut book, "assumptions", row);
                assert!((got - want).abs() <= 1e-12, "row {row}: {got}, not {want}");
            }
            // The formula reading them is the workbook's, its cached result the plan's
            let npv = cell(&mut book, "valuation", 1);
            assert!((npv - 2_794_690.957_755_616).abs() <= 1e-6, "{npv}");
        }

        /// An included file's uncertain inputs too: the bagel's flour and wage,
        /// whose plans are written beside them (1.15 and 18.50). At plan the
        /// year is the plan's, 786.3203617 (the Python of
        /// cli/commands/examples.rs, `bagel_example`).
        #[test]
        fn an_included_files_uncertain_inputs_are_its_plan_too() {
            let dir = tempfile::tempdir().unwrap();
            let output = dir.path().join("bagel.xlsx");
            let model = Path::new("examples/bagel/bagel_simulate.yaml");
            let warnings = export_core(model, &output).unwrap().warnings.join("\n");
            assert!(!warnings.contains("does not calculate"), "{warnings}");
            for expected in [
                "@costs.ingredients.flour_per_kg: written as its plan value 1.15 (",
                "@costs.labour.wage_per_hour: written as its plan value 18.5 (",
                "assumptions.opening_dozens_per_day: written as its plan value 55 (",
            ] {
                assert!(warnings.contains(expected), "{expected}\n{warnings}");
            }
            let mut book: Xlsx<_> = open_workbook(&output).unwrap();
            let profit = cell(&mut book, "year", 2);
            assert!((profit - 786.320_361_7).abs() <= 1e-6, "{profit}");
        }

        /// A distribution that does not parse has no plan: the model does not
        /// calculate, and its cell is left empty, which the warning now says
        /// where it said "written as a value".
        #[test]
        fn a_distribution_that_does_not_parse_is_left_empty() {
            let dir = tempfile::tempdir().unwrap();
            let input = dir.path().join("m.yaml");
            std::fs::write(
                &input,
                "_forge_version: \"5.1.0\"\na:\n  x: {formula: \"=MC.Normal(1)\"}\n",
            )
            .unwrap();
            let warnings = export_core(&input, &dir.path().join("m.xlsx"))
                .unwrap()
                .warnings
                .join("\n");
            assert!(warnings.contains("does not calculate"), "{warnings}");
            assert!(
                warnings.contains(
                    "a.x: left empty (function MC.Normal has no Excel equivalent): =MC.Normal(1)"
                ),
                "{warnings}"
            );
        }
    }
}
