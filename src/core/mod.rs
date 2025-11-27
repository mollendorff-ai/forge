//! Core calculation engine for v1.0.0 array models

pub mod array_calculator;
pub mod unit_validator;

pub use array_calculator::ArrayCalculator;
pub use unit_validator::{UnitValidator, UnitWarning};
