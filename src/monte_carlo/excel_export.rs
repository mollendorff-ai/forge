//! Monte Carlo Excel Export
//!
//! Exports simulation results to Excel format with:
//! - Summary sheet with statistics for each output
//! - Histogram data sheet with bin counts
//! - Input samples sheet (optional)
//! - Output samples sheet (optional)

use super::engine::SimulationResult;
use rust_xlsxwriter::{Format, Workbook};
use std::path::Path;

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
    )?;

    // Histogram sheet
    export_histograms(&mut workbook, result, &header_format, &int_format)?;

    // Input samples sheet (first 1000 rows to keep file size manageable)
    export_samples(
        &mut workbook,
        "Input_Samples",
        &result.input_samples,
        &header_format,
        &number_format,
    )?;

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
    )?;

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
) -> Result<(), String> {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name("Summary").map_err(|e| e.to_string())?;

    // Metadata row
    let _ = worksheet.write_string(0, 0, "Monte Carlo Simulation Results");
    let _ = worksheet.write_string(1, 0, "Iterations:");
    let _ = worksheet.write_number(1, 1, result.iterations_completed as f64);
    let _ = worksheet.write_string(2, 0, "Sampling:");
    let _ = worksheet.write_string(2, 1, &result.config.sampling);
    let _ = worksheet.write_string(3, 0, "Execution Time (ms):");
    let _ = worksheet.write_number(3, 1, result.execution_time_ms as f64);

    if let Some(seed) = result.config.seed {
        let _ = worksheet.write_string(4, 0, "Seed:");
        let _ = worksheet.write_number(4, 1, seed as f64);
    }

    // Output statistics table - starting at row 7
    let start_row = 7;

    // Headers
    let headers = [
        "Variable", "Mean", "Median", "Std Dev", "Min", "Max", "P10", "P25", "P50", "P75", "P90",
        "P95", "P99",
    ];

    for (col, header) in headers.iter().enumerate() {
        // cast_possible_truncation: headers array has 13 elements, well within u16 range
        #[allow(clippy::cast_possible_truncation)]
        let col_u16 = col as u16;
        let _ = worksheet.write_string_with_format(start_row, col_u16, *header, header_format);
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
        for (col_offset, &p) in percentile_keys.iter().enumerate() {
            if let Some(&val) = stats.percentiles.get(&p) {
                // cast_possible_truncation: 7 percentile columns starting at 6, well within u16 range
                #[allow(clippy::cast_possible_truncation)]
                let col_u16 = (6 + col_offset) as u16;
                let _ = worksheet.write_number_with_format(row, col_u16, val, number_format);
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

    // Auto-fit columns
    let _ = worksheet.set_column_width(0, 20);
    for col in 1..13 {
        let _ = worksheet.set_column_width(col, 12);
    }

    Ok(())
}

/// Export histogram data sheet
fn export_histograms(
    workbook: &mut Workbook,
    result: &SimulationResult,
    header_format: &Format,
    int_format: &Format,
) -> Result<(), String> {
    let worksheet = workbook.add_worksheet();
    worksheet
        .set_name("Histograms")
        .map_err(|e| e.to_string())?;

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
        for i in 0..hist.counts.len() {
            // cast_possible_truncation: histogram bins capped at 50, well within u32 range
            #[allow(clippy::cast_possible_truncation)]
            let row = (i + 2) as u32;
            let bin_start = hist.bin_edges.get(i).copied().unwrap_or(0.0);
            let bin_end = hist.bin_edges.get(i + 1).copied().unwrap_or(0.0);
            let count = hist.counts.get(i).copied().unwrap_or(0);
            let freq = hist.frequencies.get(i).copied().unwrap_or(0.0);

            let _ = worksheet.write_number(row, col, bin_start);
            let _ = worksheet.write_number(row, col + 1, bin_end);
            let _ = worksheet.write_number_with_format(row, col + 2, count as f64, int_format);
            let _ = worksheet.write_number(row, col + 3, freq);
        }

        // Set column widths
        let _ = worksheet.set_column_width(col, 12);
        let _ = worksheet.set_column_width(col + 1, 12);
        let _ = worksheet.set_column_width(col + 2, 10);
        let _ = worksheet.set_column_width(col + 3, 12);

        col += 5; // Leave gap between variables
    }

    Ok(())
}

/// Export samples sheet (limited to first 1000 rows)
fn export_samples(
    workbook: &mut Workbook,
    sheet_name: &str,
    samples: &std::collections::HashMap<String, Vec<f64>>,
    header_format: &Format,
    number_format: &Format,
) -> Result<(), String> {
    if samples.is_empty() {
        return Ok(());
    }

    let worksheet = workbook.add_worksheet();
    worksheet.set_name(sheet_name).map_err(|e| e.to_string())?;

    // Sort variable names for consistent output
    let mut var_names: Vec<&String> = samples.keys().collect();
    var_names.sort();

    // Write headers
    for (col, var_name) in var_names.iter().enumerate() {
        // cast_possible_truncation: number of variables is small, well within u16 range
        #[allow(clippy::cast_possible_truncation)]
        let col_u16 = col as u16;
        let _ = worksheet.write_string_with_format(0, col_u16, *var_name, header_format);
        let _ = worksheet.set_column_width(col_u16, 15);
    }

    // Write sample data (limit to 1000 rows to keep file size manageable)
    let max_rows = 1000;
    let num_samples = samples.values().next().map_or(0, std::vec::Vec::len);
    let rows_to_write = num_samples.min(max_rows);

    for row_idx in 0..rows_to_write {
        for (col, var_name) in var_names.iter().enumerate() {
            if let Some(sample_vec) = samples.get(*var_name) {
                if let Some(&val) = sample_vec.get(row_idx) {
                    // cast_possible_truncation: rows capped at 1000, cols bounded by variable count
                    #[allow(clippy::cast_possible_truncation)]
                    let row_u32 = (row_idx + 1) as u32;
                    #[allow(clippy::cast_possible_truncation)]
                    let col_u16 = col as u16;
                    let _ =
                        worksheet.write_number_with_format(row_u32, col_u16, val, number_format);
                }
            }
        }
    }

    // Add note if truncated
    if num_samples > max_rows {
        // cast_possible_truncation: rows_to_write capped at 1000, well within u32 range
        #[allow(clippy::cast_possible_truncation)]
        let note_row = (rows_to_write + 2) as u32;
        let _ = worksheet.write_string(
            note_row,
            0,
            format!("Note: Showing first {max_rows} of {num_samples} samples"),
        );
    }

    Ok(())
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
}
