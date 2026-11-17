//! Analysis commands: variance, sensitivity, goal-seek and break-even.

mod goal_seek;
mod sensitivity;
mod variance;

pub use goal_seek::{break_even, goal_seek, goal_seek_core};
pub use sensitivity::{sensitivity, sensitivity_core, sensitivity_in_format};
pub use variance::{pct_is_meaningful, variance, variance_core};
// What the command module's tests use
#[cfg(test)]
pub use sensitivity::{calculate_with_override, parse_range};
#[cfg(test)]
pub use variance::{
    export_variance_to_excel, export_variance_to_yaml, print_variance_table, VarianceResult,
};
