//! Games: a payoff table whose cells are formulas over the model, its
//! dominated strategies and equilibria, and tournaments of repeated play
//! (ADR-068).

pub mod counts;
pub mod linear;
pub mod mixed;
pub mod model;
pub mod pure;
pub mod repeated;
pub mod report;
pub mod standings;
pub mod strategy;
pub mod table;
pub mod tournament;

pub use model::{parse_games, Game, Payoff, TableGame};
pub use report::{analyse, GameReport, TableReport};
pub use standings::{standings, TournamentReport};
pub use tournament::Tournament;
