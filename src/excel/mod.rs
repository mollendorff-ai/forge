//! Excel import/export module for v1.0.0 array models
//!
//! This module provides bidirectional Excel ↔ YAML conversion:
//! - Export: YAML → Excel (.xlsx) with formulas
//! - Import: Excel (.xlsx) → YAML with formulas

mod excel_functions;
mod exporter;
mod importer;
mod previous_rows;
mod text_results;
pub mod workbook_refs;

pub use exporter::ExcelExporter;
pub use importer::ExcelImporter;
