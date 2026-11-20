//! What `forge game` reports for a game played once, and the same numbers
//! as scalars named under the game (ADR-068, sections 2 and 3).

use super::mixed::{equilibria, Equilibrium};
use super::pure::{even_mix, iterated_dominance, weakly_dominated};
use super::standings::TournamentReport;
use super::table::Table;
use indexmap::IndexMap;
use serde::Serialize;

/// A strategy and the one that dominates it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dominated {
    pub player: String,
    pub strategy: String,
    pub by: String,
}

/// One player's part of an equilibrium.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Play {
    /// The probability of each strategy
    pub strategies: IndexMap<String, f64>,
    /// The expected payoff
    pub payoff: f64,
}

/// One cell of the table: a strategy for each player, and what each gets.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Cell {
    /// One strategy per player, in the order of the players
    pub strategies: Vec<String>,
    /// One payoff per player, in the same order
    pub payoffs: Vec<f64>,
}

/// One equilibrium, player by player.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NamedEquilibrium {
    /// `pure` or `mixed`
    pub kind: &'static str,
    pub players: IndexMap<String, Play>,
}

/// Everything worked out for a game played once.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TableReport {
    pub game: String,
    pub players: Vec<String>,
    pub strategies: IndexMap<String, Vec<String>>,
    /// Removed in this order, each strictly dominated by a pure strategy
    pub dominated: Vec<Dominated>,
    /// Weakly dominated in the whole table; not removed
    pub weakly_dominated: Vec<Dominated>,
    pub equilibria: Vec<NamedEquilibrium>,
    /// Whether mixed equilibria were looked for (two players, at most ten
    /// strategies each); when not, the list has the pure ones only
    pub mixed_searched: bool,
    /// The ties that make the game degenerate: then mixed equilibria on a
    /// line between those listed may be missing
    pub ties: Vec<String>,
    /// Each strategy's payoff when the others mix evenly
    pub even_mix: IndexMap<String, IndexMap<String, f64>>,
    /// Every cell of the table, the first player's strategy outermost, as
    /// the model writes them: what Forge Studio draws
    pub cells: Vec<Cell>,
}

/// What was worked out for a game of either kind.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GameReport {
    Table(TableReport),
    Tournament(TournamentReport),
}

impl GameReport {
    /// Every result as a scalar named under the game.
    #[must_use]
    pub fn scalars(&self) -> Vec<(String, f64)> {
        match self {
            Self::Table(report) => report.scalars(),
            Self::Tournament(report) => report.scalars(),
        }
    }

    /// Why `name`, under this game, is not one of its results.
    #[must_use]
    pub fn why_not(&self, name: &str) -> String {
        match self {
            Self::Table(report) => report.why_not(name),
            Self::Tournament(report) => report.why_not(name),
        }
    }
}

/// Work out everything for a game played once.
#[must_use]
pub fn analyse(game: &str, table: &Table) -> TableReport {
    let name = |player: usize, strategy: usize| table.strategies[player][strategy].clone();
    let dominated = iterated_dominance(table)
        .into_iter()
        .map(|r| Dominated {
            player: table.players[r.player].clone(),
            strategy: name(r.player, r.strategy),
            by: name(r.player, r.by),
        })
        .collect();
    let weakly = weakly_dominated(table)
        .into_iter()
        .map(|w| Dominated {
            player: table.players[w.player].clone(),
            strategy: name(w.player, w.strategy),
            by: name(w.player, w.by),
        })
        .collect();
    let found = equilibria(table);
    let named = found
        .list
        .iter()
        .map(|e| named_equilibrium(table, e))
        .collect();
    let even = even_mix(table);
    TableReport {
        game: game.to_string(),
        players: table.players.clone(),
        strategies: table
            .players
            .iter()
            .cloned()
            .zip(table.strategies.iter().cloned())
            .collect(),
        dominated,
        weakly_dominated: weakly,
        equilibria: named,
        mixed_searched: found.mixed_searched,
        ties: found.ties,
        even_mix: table
            .players
            .iter()
            .zip(&even)
            .zip(&table.strategies)
            .map(|((player, pays), strategies)| {
                (
                    player.clone(),
                    strategies
                        .iter()
                        .cloned()
                        .zip(pays.iter().copied())
                        .collect(),
                )
            })
            .collect(),
        cells: table
            .profiles()
            .iter()
            .map(|profile| Cell {
                strategies: profile
                    .iter()
                    .zip(&table.strategies)
                    .map(|(&s, list)| list[s].clone())
                    .collect(),
                payoffs: table.payoffs[table.index(profile)].clone(),
            })
            .collect(),
    }
}

fn named_equilibrium(table: &Table, e: &Equilibrium) -> NamedEquilibrium {
    NamedEquilibrium {
        kind: if e.is_pure() { "pure" } else { "mixed" },
        players: table
            .players
            .iter()
            .enumerate()
            .map(|(p, player)| {
                (
                    player.clone(),
                    Play {
                        strategies: table.strategies[p]
                            .iter()
                            .cloned()
                            .zip(e.probabilities[p].iter().copied())
                            .collect(),
                        payoff: e.payoffs[p],
                    },
                )
            })
            .collect(),
    }
}

const fn flag(on: bool) -> f64 {
    if on {
        1.0
    } else {
        0.0
    }
}

impl TableReport {
    /// Every result as a scalar named under the game, in a fixed order.
    #[must_use]
    pub fn scalars(&self) -> Vec<(String, f64)> {
        let g = &self.game;
        let mut out = Vec::new();
        let pure = self.equilibria.iter().filter(|e| e.kind == "pure").count();
        out.push((
            format!("{g}.pure_equilibria"),
            crate::core::precision::count(pure),
        ));
        if self.mixed_searched {
            out.push((
                format!("{g}.equilibria"),
                crate::core::precision::count(self.equilibria.len()),
            ));
            out.push((format!("{g}.degenerate"), flag(!self.ties.is_empty())));
        }
        for (k, equilibrium) in self.equilibria.iter().enumerate() {
            for (player, play) in &equilibrium.players {
                let at = format!("{g}.equilibrium_{}.{player}", k + 1);
                for (strategy, p) in &play.strategies {
                    out.push((format!("{at}.{strategy}"), *p));
                }
                out.push((format!("{at}.payoff"), play.payoff));
            }
        }
        for (player, strategies) in &self.strategies {
            for strategy in strategies {
                let gone = self
                    .dominated
                    .iter()
                    .any(|d| &d.player == player && &d.strategy == strategy);
                out.push((format!("{g}.dominated.{player}.{strategy}"), flag(gone)));
            }
        }
        for (player, pays) in &self.even_mix {
            for (strategy, payoff) in pays {
                out.push((format!("{g}.even_mix.{player}.{strategy}"), *payoff));
            }
        }
        out
    }

    /// Why `name`, under this game, is not one of its results: how many
    /// equilibria it has, or that mixed ones were not looked for.
    #[must_use]
    pub fn why_not(&self, name: &str) -> String {
        let count = self.equilibria.len();
        let rest = name
            .strip_prefix(&format!("{}.", self.game))
            .unwrap_or(name);
        let mut reason = format!("'{name}' is not a result of game '{}'", self.game);
        if rest.starts_with("equilibrium_") {
            let word = if count == 1 {
                "equilibrium"
            } else {
                "equilibria"
            };
            reason =
                format!("{reason}: it has {count} {word} (equilibrium_1 to equilibrium_{count})");
        } else if (rest == "equilibria" || rest == "degenerate") && !self.mixed_searched {
            reason.push_str(
                ": mixed equilibria are looked for in games of two players with at most \
                 ten strategies each, and this one has only its pure equilibria",
            );
        } else {
            reason.push_str(
                ": a game's results are pure_equilibria, equilibria, degenerate, \
                 equilibrium_<k>.<player>.<strategy>, equilibrium_<k>.<player>.payoff, \
                 dominated.<player>.<strategy> and even_mix.<player>.<strategy>",
            );
        }
        reason
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::table::tests::two;

    fn value(scalars: &[(String, f64)], name: &str) -> f64 {
        scalars.iter().find(|(n, _)| n == name).unwrap().1
    }

    #[test]
    fn the_inspection_game_reports_its_mixed_equilibrium_by_name() {
        // nashpy 0.0.43 on [[0, 0], [-4, 2]] and [[-1, 0], [-1, -3]]: one
        // equilibrium, the AI complies 2/3 of the time and the human audits
        // 1/3; the AI expects 0 and the human -1. By hand, nothing is
        // dominated (cheat pays more against trust, comply against audit).
        let table = two(
            ["ai", "human"],
            &["comply", "cheat"],
            &["audit", "trust"],
            &[[0.0, -1.0], [0.0, 0.0], [-4.0, -1.0], [2.0, -3.0]],
        );
        let report = analyse("inspection", &table);
        let scalars = report.scalars();
        assert!((value(&scalars, "inspection.pure_equilibria")).abs() < 1e-12);
        assert!((value(&scalars, "inspection.equilibria") - 1.0).abs() < 1e-12);
        assert!((value(&scalars, "inspection.degenerate")).abs() < 1e-12);
        assert!(
            (value(&scalars, "inspection.equilibrium_1.human.audit") - 1.0 / 3.0).abs() < 1e-12
        );
        assert!((value(&scalars, "inspection.equilibrium_1.ai.cheat") - 1.0 / 3.0).abs() < 1e-12);
        assert!((value(&scalars, "inspection.equilibrium_1.human.payoff") + 1.0).abs() < 1e-12);
        assert!((value(&scalars, "inspection.dominated.ai.cheat")).abs() < 1e-12);
        // Against an even mix: comply 0, cheat (-4 + 2) / 2 = -1; audit -1,
        // trust (0 - 3) / 2 = -1.5
        assert!((value(&scalars, "inspection.even_mix.ai.cheat") + 1.0).abs() < 1e-12);
        assert!((value(&scalars, "inspection.even_mix.human.trust") + 1.5).abs() < 1e-12);
        assert_eq!(report.equilibria[0].kind, "mixed");
        assert_eq!(scalars.len(), 3 + 6 + 4 + 4);
        // The cells, as the table was written: comply's row, then cheat's
        let cells: Vec<(Vec<&str>, Vec<f64>)> = report
            .cells
            .iter()
            .map(|c| {
                (
                    c.strategies.iter().map(String::as_str).collect(),
                    c.payoffs.clone(),
                )
            })
            .collect();
        assert_eq!(
            cells,
            [
                (vec!["comply", "audit"], vec![0.0, -1.0]),
                (vec!["comply", "trust"], vec![0.0, 0.0]),
                (vec!["cheat", "audit"], vec![-4.0, -1.0]),
                (vec!["cheat", "trust"], vec![2.0, -3.0]),
            ]
        );
    }

    #[test]
    fn the_prisoners_dilemma_names_what_is_dominated() {
        // By hand (T 5, R 3, P 1, S 0): defect pays more against both, so
        // cooperate goes for each, and (defect, defect) is the one
        // equilibrium, as nashpy 0.0.43 finds.
        let table = two(
            ["a", "b"],
            &["cooperate", "defect"],
            &["cooperate", "defect"],
            &[[3.0, 3.0], [0.0, 5.0], [5.0, 0.0], [1.0, 1.0]],
        );
        let report = analyse("dilemma", &table);
        assert_eq!(
            report.dominated,
            vec![
                Dominated {
                    player: "a".into(),
                    strategy: "cooperate".into(),
                    by: "defect".into()
                },
                Dominated {
                    player: "b".into(),
                    strategy: "cooperate".into(),
                    by: "defect".into()
                },
            ]
        );
        let scalars = report.scalars();
        assert!((value(&scalars, "dilemma.dominated.a.cooperate") - 1.0).abs() < 1e-12);
        assert!((value(&scalars, "dilemma.dominated.b.defect")).abs() < 1e-12);
        assert!((value(&scalars, "dilemma.pure_equilibria") - 1.0).abs() < 1e-12);
        assert_eq!(report.equilibria[0].kind, "pure");
    }

    #[test]
    fn a_weak_dominance_and_a_tie_are_reported() {
        // By hand: b pays at least what a pays, and more against y; the
        // column player is indifferent everywhere, so the game is
        // degenerate.
        let table = two(
            ["row", "column"],
            &["a", "b"],
            &["x", "y"],
            &[[1.0, 0.0], [1.0, 0.0], [1.0, 0.0], [2.0, 0.0]],
        );
        let report = analyse("g", &table);
        assert_eq!(report.weakly_dominated[0].strategy, "a");
        assert!(
            report
                .ties
                .contains(&"against column's x, row's a and b pay the same".to_string()),
            "{:?}",
            report.ties
        );
        assert!((value(&report.scalars(), "g.degenerate") - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_name_that_is_not_a_result_says_why() {
        let table = two(
            ["a", "b"],
            &["x", "y"],
            &["x", "y"],
            &[[4.0, 4.0], [0.0, 3.0], [3.0, 0.0], [3.0, 3.0]],
        );
        let report = analyse("g", &table);
        assert_eq!(
            report.why_not("g.equilibrium_4.a.x"),
            "'g.equilibrium_4.a.x' is not a result of game 'g': it has 3 equilibria \
             (equilibrium_1 to equilibrium_3)"
        );
        assert!(report
            .why_not("g.wins")
            .contains("a game's results are pure_equilibria"));
        let one = analyse("h", &two(["a", "b"], &["x"], &["y"], &[[1.0, 1.0]]));
        assert!(one
            .why_not("h.equilibrium_2.a.x")
            .contains("it has 1 equilibrium (equilibrium_1"));

        let mut payoffs = Vec::new();
        for _ in 0..8 {
            payoffs.push(vec![0.0; 3]);
        }
        let three = analyse(
            "t",
            &Table {
                players: vec!["a".into(), "b".into(), "c".into()],
                strategies: vec![vec!["x".into(), "y".into()]; 3],
                payoffs,
            },
        );
        assert!(three
            .why_not("t.equilibria")
            .contains("only its pure equilibria"));
        assert!(three
            .why_not("t.degenerate")
            .contains("only its pure equilibria"));
        let names: Vec<String> = three.scalars().into_iter().map(|(n, _)| n).collect();
        assert!(!names.contains(&"t.equilibria".to_string()));
        assert!(names.contains(&"t.pure_equilibria".to_string()));
    }
}
