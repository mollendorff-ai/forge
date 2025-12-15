//! Tornado Diagrams Module (Enterprise Only)
//!
//! Sensitivity analysis showing which inputs drive output variance:
//! - One-at-a-time sensitivity analysis
//! - Tornado chart data generation
//! - R sensitivity package validated calculations
//!
//! # Example
//!
//! ```yaml
//! tornado:
//!   output: npv
//!   inputs:
//!     - name: revenue_growth
//!       low: 0.02
//!       high: 0.08
//!     - name: discount_rate
//!       low: 0.08
//!       high: 0.12
//!     - name: operating_margin
//!       low: 0.15
//!       high: 0.25
//! ```
//!
//! Output:
//! ```text
//! NPV Sensitivity (Base: $1.2M)
//!
//! Revenue Growth    |████████████████████| +/- $450K
//! Discount Rate     |██████████████      | +/- $320K
//! Operating Margin  |██████████          | +/- $180K
//! ```

pub mod config;
pub mod engine;

// Re-exports
pub use config::{InputRange, TornadoConfig};
pub use engine::{SensitivityBar, TornadoEngine, TornadoResult};

#[cfg(test)]
mod tests;
