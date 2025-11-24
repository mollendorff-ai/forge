//! Excel export module for v1.0.0 array models
//!
//! This module provides functionality to export v1.0.0 array models to Excel .xlsx files
//! with full formula support and cross-table references.

mod exporter;
mod formula_translator;

pub use exporter::ExcelExporter;
pub use formula_translator::FormulaTranslator;
