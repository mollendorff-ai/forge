//! Games: a payoff table whose cells are formulas over the model, its
//! dominated strategies and equilibria, and tournaments of repeated play
//! (ADR-068).

pub mod linear;
pub mod mixed;
pub mod model;
pub mod pure;
pub mod report;
pub mod table;

pub use model::{parse_games, Game, Payoff};
pub use report::{analyse, TableReport};
