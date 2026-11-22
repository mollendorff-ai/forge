//! Monte Carlo Excel Export
//!
//! Exports simulation results to Excel format with:
//! - Summary sheet with statistics, thresholds and drivers for each output
//! - Histogram data sheet with bin counts
//! - Input samples sheet (optional)
//! - Output samples sheet (optional)

use super::engine::SimulationResult;
use crate::core::precision;
use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

/// Why naming a sheet cannot fail: the names are fixed, short and free of
/// the characters Excel refuses.
const VALID_SHEET_NAME: &str = "a fixed sheet name is valid";

/// Export Monte Carlo results to Excel
///
/// # Errors
///
/// Returns an error if the workbook cannot be created or saved to `output_path`.
pub fn export_results(result: &SimulationResult, output_path: &Path) -> Result<(), String> {
    let mut workbook = Workbook::new();

    // Create formats
    let header_format = Format::new().set_bold();
    let number_format = Format::new().set_num_format("#,##0.0000");
    let percent_format = Format::new().set_num_format("0.00%");
    let int_format = Format::new().set_num_format("#,##0");

    // Summary sheet
    export_summary(
        &mut workbook,
        result,
        &header_format,
        &number_format,
        &percent_format,
    );

    // Histogram sheet
    export_histograms(&mut workbook, result, &header_format, &int_format);

    // Input samples sheet (first 1000 rows to keep file size manageable)
    export_samples(
        &mut workbook,
        "Input_Samples",
        &result.input_samples,
        &header_format,
        &number_format,
    );

    // Output samples sheet (first 1000 rows)
    let output_samples: std::collections::HashMap<String, Vec<f64>> = result
        .outputs
        .iter()
        .map(|(k, v)| (k.clone(), v.samples.clone()))
        .collect();
    export_samples(
        &mut workbook,
        "Output_Samples",
        &output_samples,
        &header_format,
        &number_format,
    );

    // Save workbook
    workbook
        .save(output_path)
        .map_err(|e| format!("Failed to save Excel file: {e}"))?;

    Ok(())
}

/// Export summary statistics sheet
fn export_summary(
    workbook: &mut Workbook,
    result: &SimulationResult,
    header_format: &Format,
    number_format: &Format,
    percent_format: &Format,
) {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Summary").expect(VALID_SHEET_NAME);

    // Metadata row
    let _ = worksheet.write_string(0, 0, "Monte Carlo Simulation Results");
    let _ = worksheet.write_string(1, 0, "Iterations:");
    let _ = worksheet.write_number(1, 1, precision::count(result.iterations_completed));
    let _ = worksheet.write_string(2, 0, "Sampling:");
    let _ = worksheet.write_string(2, 1, &result.config.sampling);
    let _ = worksheet.write_string(3, 0, "Execution Time (ms):");
    let _ = worksheet.write_number(3, 1, precision::unsigned(result.execution_time_ms));

    if let Some(seed) = result.config.seed {
        let _ = worksheet.write_string(4, 0, "Seed:");
        let _ = worksheet.write_number(4, 1, precision::unsigned(seed));
    }

    // Output statistics table - starting at row 7
    let start_row = 7;

    // Headers
    let headers = [
        "Variable", "Mean", "Median", "Std Dev", "Min", "Max", "P10", "P25", "P50", "P75", "P90",
        "P95", "P99",
    ];

    for (col, header) in (0u16..).zip(headers) {
        let _ = worksheet.write_string_with_format(start_row, col, header, header_format);
    }

    // Data rows
    let mut row = start_row + 1;
    for (var_name, output) in &result.outputs {
        let stats = &output.statistics;

        let _ = worksheet.write_string(row, 0, var_name);
        let _ = worksheet.write_number_with_format(row, 1, stats.mean, number_format);
        let _ = worksheet.write_number_with_format(row, 2, stats.median, number_format);
        let _ = worksheet.write_number_with_format(row, 3, stats.std_dev, number_format);
        let _ = worksheet.write_number_with_format(row, 4, stats.min, number_format);
        let _ = worksheet.write_number_with_format(row, 5, stats.max, number_format);

        // Percentiles
        let percentile_keys = [10, 25, 50, 75, 90, 95, 99];
        for (col, p) in (6u16..).zip(percentile_keys) {
            if let Some(&val) = stats.percentiles.get(&p) {
                let _ = worksheet.write_number_with_format(row, col, val, number_format);
            }
        }

        row += 1;
    }

    // Threshold probabilities section
    row += 2;
    let _ = worksheet.write_string_with_format(row, 0, "Threshold Probabilities", header_format);
    row += 1;

    let _ = worksheet.write_string_with_format(row, 0, "Variable", header_format);
    let _ = worksheet.write_string_with_format(row, 1, "Threshold", header_format);
    let _ = worksheet.write_string_with_format(row, 2, "Probability", header_format);
    row += 1;

    for (var_name, output) in &result.outputs {
        for (threshold, prob) in &output.threshold_probabilities {
            let _ = worksheet.write_string(row, 0, var_name);
            let _ = worksheet.write_string(row, 1, threshold);
            let _ = worksheet.write_number_with_format(row, 2, *prob, percent_format);
            row += 1;
        }
    }

    // Drivers: each sampled input's rank correlation with each output
    row += 2;
    let _ = worksheet.write_string_with_format(row, 0, "Drivers", header_format);
    row += 1;
    let _ = worksheet.write_string_with_format(row, 0, "Variable", header_format);
    let _ = worksheet.write_string_with_format(row, 1, "Input", header_format);
    let _ = worksheet.write_string_with_format(row, 2, "Rank correlation", header_format);
    row += 1;
    for (var_name, output) in &result.outputs {
        for driver in &output.drivers {
            let _ = worksheet.write_string(row, 0, var_name);
            let _ = worksheet.write_string(row, 1, &driver.input);
            let _ =
                worksheet.write_number_with_format(row, 2, driver.rank_correlation, number_format);
            row += 1;
        }
    }

    // Auto-fit columns
    let _ = worksheet.set_column_width(0, 20);
    for col in 1..13 {
        let _ = worksheet.set_column_width(col, 12);
    }
}

/// Export histogram data sheet
fn export_histograms(
    workbook: &mut Workbook,
    result: &SimulationResult,
    header_format: &Format,
    int_format: &Format,
) {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Histograms").expect(VALID_SHEET_NAME);

    let mut col: u16 = 0;

    for (var_name, output) in &result.outputs {
        // Variable name header
        let _ = worksheet.write_string_with_format(0, col, var_name, header_format);

        // Column headers
        let _ = worksheet.write_string_with_format(1, col, "Bin Start", header_format);
        let _ = worksheet.write_string_with_format(1, col + 1, "Bin End", header_format);
        let _ = worksheet.write_string_with_format(1, col + 2, "Count", header_format);
        let _ = worksheet.write_string_with_format(1, col + 3, "Frequency", header_format);

        // Histogram data
        let hist = &output.histogram;

        // Iterate over bin edges (pairs of consecutive edges form bins)
        for (i, row) in (0..hist.counts.len()).zip(2u32..) {
            let bin_start = hist.bin_edges.get(i).copied().unwrap_or(0.0);
            let bin_end = hist.bin_edges.get(i + 1).copied().unwrap_or(0.0);
            let count = hist.counts.get(i).copied().unwrap_or(0);
            let freq = hist.frequencies.get(i).copied().unwrap_or(0.0);

            let _ = worksheet.write_number(row, col, bin_start);
            let _ = worksheet.write_number(row, col + 1, bin_end);
            let _ = worksheet.write_number_with_format(
                row,
                col + 2,
                precision::count(count),
                int_format,
            );
            let _ = worksheet.write_number(row, col + 3, freq);
        }

        // Set column widths
        let _ = worksheet.set_column_width(col, 12);
        let _ = worksheet.set_column_width(col + 1, 12);
        let _ = worksheet.set_column_width(col + 2, 10);
        let _ = worksheet.set_column_width(col + 3, 12);

        col += 5; // Leave gap between variables
    }
}

/// Export samples sheet (limited to first 1000 rows)
fn export_samples(
    workbook: &mut Workbook,
    sheet_name: &str,
    samples: &std::collections::HashMap<String, Vec<f64>>,
    header_format: &Format,
    number_format: &Format,
) {
    if samples.is_empty() {
        return;
    }

    let worksheet = workbook.add_worksheet();
    worksheet.set_name(sheet_name).expect(VALID_SHEET_NAME);

    // Sort variable names for consistent output
    let mut var_names: Vec<&String> = samples.keys().collect();
    var_names.sort();

    // Write headers
    // Past Excel's last column the writer refuses the cell, long before the counter ends
    for (col, var_name) in (0u16..=u16::MAX).zip(&var_names) {
        let _ = worksheet.write_string_with_format(0, col, *var_name, header_format);
        let _ = worksheet.set_column_width(col, 15);
    }

    // Write sample data (limit to 1000 rows to keep file size manageable)
    let max_rows: u16 = 1000;
    // The longest column, not whichever the map lists first
    let num_samples = samples.values().map(Vec::len).max().unwrap_or(0);
    let rows_to_write = num_samples.min(usize::from(max_rows));

    for (row_idx, row) in (0..rows_to_write).zip(1u32..) {
        for (col, var_name) in (0u16..=u16::MAX).zip(&var_names) {
            // A column shorter than the first has no cell in this row
            if let Some(&val) = samples[*var_name].get(row_idx) {
                let _ = worksheet.write_number_with_format(row, col, val, number_format);
            }
        }
    }

    // Add note if truncated: the note goes below the last row written
    if num_samples > usize::from(max_rows) {
        let note_row = u32::from(max_rows) + 2;
        let _ = worksheet.write_string(
            note_row,
            0,
            format!("Note: Showing first {max_rows} of {num_samples} samples"),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monte_carlo::config::{MonteCarloConfig, OutputConfig};
    use crate::monte_carlo::distributions::Distribution;
    use crate::monte_carlo::engine::MonteCarloEngine;

    #[test]
    fn test_excel_export() {
        // Create a simple simulation
        let config = MonteCarloConfig {
            enabled: true,
            iterations: 1000,
            sampling: "latin_hypercube".to_string(),
            seed: Some(42),
            outputs: vec![OutputConfig {
                variable: "revenue".to_string(),
                percentiles: vec![10, 50, 90],
                threshold: Some("> 50".to_string()),
                thresholds: vec![],
                expected_shortfall: vec![],
                higher_is_worse: false,
                row: None,
                label: None,
            }],
            correlations: vec![],
        };

        let mut engine = MonteCarloEngine::new(config).unwrap();
        engine.add_distribution("revenue", Distribution::normal(100.0, 20.0).unwrap());

        let result = engine.run().unwrap();

        // Create temp file for Excel output
        let temp_dir = std::env::temp_dir();
        let output_path = temp_dir.join("mc_test_export.xlsx");

        // Export to Excel
        let export_result = export_results(&result, &output_path);
        assert!(
            export_result.is_ok(),
            "Excel export failed: {export_result:?}"
        );

        // Verify file exists and has reasonable size
        let metadata = std::fs::metadata(&output_path).unwrap();
        assert!(metadata.len() > 1000, "Excel file too small");

        // Cleanup
        let _ = std::fs::remove_file(&output_path);
    }

    fn simulated(iterations: usize, seed: Option<u64>) -> SimulationResult {
        let config = MonteCarloConfig {
            enabled: true,
            iterations,
            sampling: "monte_carlo".to_string(),
            seed,
            outputs: vec![OutputConfig {
                variable: "revenue".to_string(),
                percentiles: vec![50],
                threshold: None,
                thresholds: vec![],
                expected_shortfall: vec![],
                higher_is_worse: false,
                row: None,
                label: None,
            }],
            correlations: vec![],
        };
        let mut engine = MonteCarloEngine::new(config).unwrap();
        engine.add_distribution("revenue", Distribution::normal(100.0, 20.0).unwrap());
        engine.run().unwrap()
    }

    fn book(result: &SimulationResult) -> calamine::Xlsx<std::io::BufReader<std::fs::File>> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mc.xlsx");
        export_results(result, &path).unwrap();
        calamine::open_workbook(&path).unwrap()
    }

    fn text(range: &calamine::Range<calamine::Data>, row: u32, col: u32) -> String {
        range.get_value((row, col)).unwrap().to_string()
    }

    /// The workbook of a seeded run of more than 1,000 draws: its summary, and
    /// the samples cut at 1,000 rows with a note saying so.
    #[test]
    fn a_long_seeded_run_is_summarised_and_its_samples_cut_with_a_note() {
        use calamine::Reader;
        let mut workbook = book(&simulated(1100, Some(42)));
        assert_eq!(
            workbook.sheet_names(),
            ["Summary", "Histograms", "Input_Samples", "Output_Samples"]
        );
        let summary = workbook.worksheet_range("Summary").unwrap();
        assert_eq!(text(&summary, 0, 0), "Monte Carlo Simulation Results");
        assert_eq!(text(&summary, 1, 1), "1100");
        assert_eq!(text(&summary, 2, 1), "monte_carlo");
        assert_eq!(text(&summary, 4, 0), "Seed:");
        assert_eq!(text(&summary, 4, 1), "42");
        let inputs = workbook.worksheet_range("Input_Samples").unwrap();
        assert_eq!(text(&inputs, 0, 0), "revenue");
        assert_eq!(inputs.get_size(), (1003, 1));
        assert_eq!(
            workbook
                .worksheet_range("Output_Samples")
                .unwrap()
                .get_size(),
            (1003, 1)
        );
        // The note sits below the header, the 1,000 rows and a blank row
        assert_eq!(
            text(&inputs, 1002, 0),
            "Note: Showing first 1000 of 1100 samples"
        );
    }

    /// With no seed there is no seed row; with no input draws, no sheet of them.
    #[test]
    fn a_run_with_no_seed_and_no_inputs_has_neither_row_nor_sheet() {
        use calamine::Reader;
        let mut result = simulated(100, None);
        result.input_samples.clear();
        let mut workbook = book(&result);
        assert_eq!(
            workbook.sheet_names(),
            ["Summary", "Histograms", "Output_Samples"]
        );
        let summary = workbook.worksheet_range("Summary").unwrap();
        assert_eq!(text(&summary, 3, 0), "Execution Time (ms):");
        assert_eq!(text(&summary, 4, 0), "");
    }

    /// A column shorter than the first leaves its cells empty in the rows it
    /// lacks.
    #[test]
    fn a_shorter_column_has_empty_cells() {
        use calamine::Reader;
        let samples = std::collections::HashMap::from([
            ("a".to_string(), vec![1.0, 2.0, 3.0]),
            ("b".to_string(), vec![9.0]),
        ]);
        let mut workbook = Workbook::new();
        let (header, number) = (Format::new().set_bold(), Format::new());
        export_samples(&mut workbook, "S", &samples, &header, &number);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("s.xlsx");
        workbook.save(&path).unwrap();
        let mut read: calamine::Xlsx<_> = calamine::open_workbook(&path).unwrap();
        let range = read.worksheet_range("S").unwrap();
        // The range ends at its last cell: a missing cell is an empty one
        let cell = |r: u32, c: u32| {
            range
                .get_value((r, c))
                .map_or(String::new(), ToString::to_string)
        };
        let column = |c: u32| -> Vec<String> { (1..4).map(|r| cell(r, c)).collect() };
        assert_eq!(column(0), ["1", "2", "3"]);
        assert_eq!(column(1), ["9", "", ""]);
    }

    /// A workbook that cannot be saved says so.
    #[test]
    fn a_workbook_that_cannot_be_saved_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no-such-folder").join("mc.xlsx");
        assert_eq!(
            export_results(&simulated(100, Some(1)), &path).unwrap_err(),
            "Failed to save Excel file: No such file or directory (os error 2)"
        );
    }
}
