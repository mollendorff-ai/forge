//! What `forge game` reports for a game played once, and the same numbers
//! as scalars named under the game (ADR-068, sections 2 and 3).

use super::dynamics::DynamicsReport;
use super::mixed::{equilibria, Equilibrium};
use super::population::{population, Population};
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
    /// Where each payoff is written, in the same order, as `edit` names it
    /// (`set_formula` takes the name); empty where the report is not worked
    /// out from a model
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub written: Vec<Written>,
}

/// One payoff as the model writes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Written {
    /// `stag_hunt.payoffs.hare.stag`, and a list's player last
    /// (`inspection.payoffs.cheat.audit.human`)
    pub path: String,
    /// The formula, when the payoff is one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
}

/// One equilibrium, player by player.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NamedEquilibrium {
    /// `pure` or `mixed`
    pub kind: &'static str,
    pub players: IndexMap<String, Play>,
    /// For a symmetric equilibrium of a symmetric game of two strategies:
    /// whether the replicator dynamics return to it after a small change in
    /// the shares (ADR-073)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stable: Option<bool>,
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
    /// Whether the symmetric mixed equilibria were looked for, in a
    /// symmetric game of two strategies and more than two players: the
    /// list then has the pure ones and those in which every player mixes
    /// the same way, not those in which players mix differently (ADR-073)
    pub symmetric_searched: bool,
    /// The ties that make the game degenerate: then mixed equilibria on a
    /// line between those listed may be missing
    pub ties: Vec<String>,
    /// Each strategy's payoff when the others mix evenly
    pub even_mix: IndexMap<String, IndexMap<String, f64>>,
    /// Every cell of the table, the first player's strategy outermost, as
    /// the model writes them: what Forge Studio draws
    pub cells: Vec<Cell>,
    /// The shares of the strategies, generation by generation, when the
    /// game has a `dynamics:` key (ADR-073)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dynamics: Option<DynamicsReport>,
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

/// Work out everything for a game played once; `symmetric` when the model
/// says it is.
#[must_use]
pub fn analyse(game: &str, table: &Table, symmetric: bool) -> TableReport {
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
    let mut found = equilibria(table);
    // A population reads the first player's payoffs for all: a symmetric game only
    let people = symmetric.then(|| population(table)).flatten();
    let symmetric_searched = people.is_some() && !found.mixed_searched;
    if let Some(people) = people.as_ref().filter(|_| symmetric_searched) {
        add_symmetric_mixes(table, people, &mut found.list, &mut found.ties);
    }
    let named = found
        .list
        .iter()
        .map(|e| named_equilibrium(table, e, people.as_ref()))
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
        symmetric_searched,
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
                written: Vec::new(),
            })
            .collect(),
        dynamics: None,
    }
}

/// The symmetric mixed equilibria of a game of more than two players, after
/// the pure ones, by increasing share of the first strategy; or the tie that
/// makes every mix one.
fn add_symmetric_mixes(
    table: &Table,
    people: &Population,
    list: &mut Vec<Equilibrium>,
    ties: &mut Vec<String>,
) {
    let n = table.players.len();
    let (first, second) = (&table.strategies[0][0], &table.strategies[0][1]);
    if people.degenerate {
        ties.push(format!(
            "{first} and {second} pay the same however many of the others play each, so \
             every mix in which all play alike is an equilibrium, and none of them is listed"
        ));
    }
    for rest in people
        .rests
        .iter()
        .filter(|r| r.share > 0.0 && r.share < 1.0)
    {
        list.push(Equilibrium {
            probabilities: vec![vec![rest.share, 1.0 - rest.share]; n],
            payoffs: vec![people.payoff(rest.share); n],
        });
    }
}

/// Whether the dynamics return to `e`, if it is a symmetric equilibrium of a
/// symmetric game of two strategies.
fn stable(e: &Equilibrium, people: Option<&Population>) -> Option<bool> {
    let share = e.probabilities[0][0];
    let alike = e
        .probabilities
        .iter()
        .all(|mix| (mix[0] - share).abs() < 1e-9);
    people
        .filter(|_| alike)
        .and_then(|p| p.rest_at(share))
        .map(|r| r.stable)
}

fn named_equilibrium(
    table: &Table,
    e: &Equilibrium,
    people: Option<&Population>,
) -> NamedEquilibrium {
    NamedEquilibrium {
        stable: stable(e, people),
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
        }
        if self.mixed_searched || self.symmetric_searched {
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
            if let Some(stable) = equilibrium.stable {
                out.push((format!("{g}.equilibrium_{}.stable", k + 1), flag(stable)));
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
        if let Some(dynamics) = &self.dynamics {
            let end = dynamics.end();
            for (strategy, share) in &end.shares {
                out.push((format!("{g}.dynamics.share.{strategy}"), *share));
            }
            out.push((format!("{g}.dynamics.payoff"), end.payoff));
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
        if rest.starts_with("others.") {
            reason = format!(
                "{reason}: it is how many of the other players play a strategy, which only \
                 the payoffs of game '{}' can read",
                self.game
            );
        } else if let Some(k) = rest
            .strip_prefix("equilibrium_")
            .and_then(|r| r.strip_suffix(".stable"))
            .and_then(|k| k.parse::<usize>().ok())
            .filter(|k| (1..=count).contains(k))
        {
            reason = format!(
                "{reason}: equilibrium_{k}.stable is worked out for an equilibrium in which \
                 every player plays alike, in a game marked symmetric with two strategies, and \
                 equilibrium_{k} is not one"
            );
        } else if rest.starts_with("equilibrium_") {
            let word = if count == 1 {
                "equilibrium"
            } else {
                "equilibria"
            };
            reason =
                format!("{reason}: it has {count} {word} (equilibrium_1 to equilibrium_{count})");
        } else if rest == "equilibria" && self.symmetric_searched {
            reason = format!(
                "{reason}: in a game of {} players Forge looks for the mixed equilibria in which \
                 every player plays alike, not those in which players mix differently, so it \
                 has no count of them all; pure_equilibria counts the pure ones",
                self.players.len()
            );
        } else if rest == "dynamics" || rest.starts_with("dynamics.") {
            reason = self.dynamics.as_ref().map_or_else(
                || {
                    format!(
                        "{reason}: the dynamics' results come from a dynamics: key, with start \
                         and generations, and games.{} has none",
                        self.game
                    )
                },
                |dynamics| {
                    let shares: Vec<String> = dynamics
                        .end()
                        .shares
                        .keys()
                        .map(|s| format!("dynamics.share.{s}"))
                        .collect();
                    format!(
                        "{reason}: its dynamics give {} and dynamics.payoff, after the last \
                         generation",
                        shares.join(", ")
                    )
                },
            );
        } else if (rest == "equilibria" || rest == "degenerate")
            && !self.mixed_searched
            && !self.symmetric_searched
        {
            reason.push_str(
                ": mixed equilibria are looked for in games of two players with at most \
                 ten strategies each, and in symmetric games of two strategies, and this one \
                 has only its pure equilibria",
            );
        } else {
            reason.push_str(
                ": a game's results are pure_equilibria, equilibria, degenerate, \
                 equilibrium_<k>.<player>.<strategy>, equilibrium_<k>.<player>.payoff, \
                 equilibrium_<k>.stable, dominated.<player>.<strategy>, \
                 even_mix.<player>.<strategy>, and with a dynamics: key \
                 dynamics.share.<strategy> and dynamics.payoff",
            );
        }
        reason
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::population::tests::symmetric;
    use crate::games::table::tests::two;

    fn value(scalars: &[(String, f64)], name: &str) -> f64 {
        scalars.iter().find(|(n, _)| n == name).unwrap().1
    }

    /// A game that is not symmetric, its first player with two strategies
    /// and the other with one, has no population to read (it panicked once,
    /// reading the other's second strategy)
    #[test]
    fn a_game_whose_other_player_has_one_strategy_is_analysed() {
        let table = two(
            ["row", "column"],
            &["up", "down"],
            &["only"],
            &[[1.0, 1.0], [0.0, 0.0]],
        );
        let report = analyse("g", &table, false);
        let scalars = report.scalars();
        assert!((value(&scalars, "g.pure_equilibria") - 1.0).abs() < 1e-12);
        assert!((value(&scalars, "g.equilibrium_1.row.up") - 1.0).abs() < 1e-12);
        assert!((value(&scalars, "g.dominated.row.down") - 1.0).abs() < 1e-12);
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
        let report = analyse("inspection", &table, false);
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
        let report = analyse("dilemma", &table, false);
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
        let report = analyse("g", &table, false);
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
    fn the_volunteers_dilemma_of_five_lists_its_symmetric_mix_after_the_pure_ones() {
        // Volunteering pays 10 - 4 = 6, staying out 10 if anyone else
        // volunteers and 0 if not. By hand: five pure equilibria, each with
        // one volunteer, then the mix in which each volunteers with
        // 1 - 0.4^(1/4) = 0.204729..., and a population returns to it.
        let table = symmetric(5, |own, k| match (own, k) {
            (0, _) => 6.0,
            (_, 0) => 0.0,
            _ => 10.0,
        });
        let report = analyse("vd", &table, true);
        let scalars = report.scalars();
        let share = 1.0 - 0.4_f64.powf(0.25);
        assert!((value(&scalars, "vd.pure_equilibria") - 5.0).abs() < 1e-12);
        assert!((value(&scalars, "vd.degenerate")).abs() < 1e-12);
        assert!((value(&scalars, "vd.equilibrium_6.p3.a") - share).abs() < 1e-12);
        assert!((value(&scalars, "vd.equilibrium_6.p5.b") - (1.0 - share)).abs() < 1e-12);
        assert!((value(&scalars, "vd.equilibrium_6.p1.payoff") - 6.0).abs() < 1e-12);
        assert!((value(&scalars, "vd.equilibrium_6.stable") - 1.0).abs() < 1e-12);
        assert_eq!(report.equilibria[5].kind, "mixed");
        assert_eq!(report.equilibria[5].stable, Some(true));
        // One volunteer and four who stay out do not play alike
        assert_eq!(report.equilibria[0].stable, None);
        let names: Vec<String> = scalars.into_iter().map(|(n, _)| n).collect();
        assert!(!names.contains(&"vd.equilibria".to_string()));
        assert!(!names.contains(&"vd.equilibrium_1.stable".to_string()));
        assert!(report.symmetric_searched && !report.mixed_searched);
        assert_eq!(
            report.why_not("vd.equilibrium_1.stable"),
            "'vd.equilibrium_1.stable' is not a result of game 'vd': equilibrium_1.stable is \
             worked out for an equilibrium in which every player plays alike, in a game marked \
             symmetric with two strategies, and equilibrium_1 is not one"
        );
        assert!(report
            .why_not("vd.equilibrium_7.stable")
            .contains("it has 6 equilibria"));
        assert!(report
            .why_not("vd.equilibria")
            .contains("in a game of 5 players Forge looks for the mixed equilibria in which"));
        // Not marked symmetric: the pure ones only
        let unmarked = analyse("vd", &table, false);
        assert_eq!(unmarked.equilibria.len(), 5);
        assert!(!unmarked.symmetric_searched);
        assert!(unmarked
            .why_not("vd.degenerate")
            .contains("only its pure equilibria"));
    }

    #[test]
    fn the_public_goods_game_at_r_equal_to_n_names_the_tie_and_lists_no_mix() {
        // Contributing pays 10 (r / 5 - 1) more than keeping: nothing at
        // r = 5, so all 32 profiles are equilibria, and so is every mix
        let table = symmetric(5, |own, k| {
            let k = crate::core::precision::count(k);
            if own == 0 {
                10.0 * (k + 1.0)
            } else {
                10.0_f64.mul_add(k, 10.0)
            }
        });
        let report = analyse("pg", &table, true);
        assert_eq!(report.equilibria.len(), 32);
        assert!(report.equilibria.iter().all(|e| e.kind == "pure"));
        assert_eq!(
            report.ties,
            [
                "a and b pay the same however many of the others play each, so every mix in \
              which all play alike is an equilibrium, and none of them is listed"
            ]
        );
        let scalars = report.scalars();
        assert!((value(&scalars, "pg.degenerate") - 1.0).abs() < 1e-12);
        // Nothing returns: the dynamics stand still everywhere
        assert!(report.equilibria.iter().all(|e| e.stable.is_none()));
    }

    #[test]
    fn the_stag_hunt_of_two_says_which_equilibria_a_population_returns_to() {
        // Stag 4 against stag, 0 against hare; hare 3: the support
        // enumeration finds all-stag, all-hare and the mix at 3/4, and by
        // hand a population returns to both pure ones and leaves the mix
        let table = two(
            ["a", "b"],
            &["stag", "hare"],
            &["stag", "hare"],
            &[[4.0, 4.0], [0.0, 3.0], [3.0, 0.0], [3.0, 3.0]],
        );
        let report = analyse("sh", &table, true);
        let stable: Vec<Option<bool>> = report.equilibria.iter().map(|e| e.stable).collect();
        assert_eq!(stable, [Some(true), Some(true), Some(false)]);
        let scalars = report.scalars();
        assert!((value(&scalars, "sh.equilibrium_3.stable")).abs() < 1e-12);
        assert!((value(&scalars, "sh.equilibrium_3.a.stag") - 0.75).abs() < 1e-12);
        assert!(!report.symmetric_searched && report.mixed_searched);
        // A game not marked symmetric has no stability
        assert!(analyse("sh", &table, false)
            .equilibria
            .iter()
            .all(|e| e.stable.is_none()));
    }

    #[test]
    fn for_two_players_the_symmetric_equilibria_are_the_support_enumerations() {
        // 2,000 seeded symmetric games of two strategies: the equilibria in
        // which both play alike, found by support enumeration (ADR-068),
        // are the population's, share for share, and each has a stability
        let mut seed: u64 = 202_611;
        let mut draw = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            crate::core::precision::unsigned(seed % 2001) / 100.0 - 10.0
        };
        let mut mixes = 0;
        for _ in 0..2000 {
            let cells = [draw(), draw(), draw(), draw()];
            let table = symmetric(2, |own, k| cells[own * 2 + (1 - k)]);
            let people = population(&table).unwrap();
            let report = analyse("g", &table, true);
            let share = |e: &NamedEquilibrium, p: &str| e.players[p].strategies["a"];
            let mut alike: Vec<f64> = report
                .equilibria
                .iter()
                .filter(|e| (share(e, "p1") - share(e, "p2")).abs() < 1e-9)
                .map(|e| share(e, "p1"))
                .collect();
            alike.sort_by(f64::total_cmp);
            let rests: Vec<f64> = people.rests.iter().map(|r| r.share).collect();
            if report.ties.is_empty() {
                assert_eq!(alike.len(), rests.len(), "{cells:?} {report:?} {people:?}");
                for (a, r) in alike.iter().zip(&rests) {
                    assert!((a - r).abs() < 1e-9, "{cells:?}");
                }
                mixes += rests.iter().filter(|&&r| r > 0.0 && r < 1.0).count();
            }
        }
        // Mixed ones among them, not only pure
        assert!(mixes > 200, "{mixes}");
    }

    #[test]
    fn a_name_that_is_not_a_result_says_why() {
        let table = two(
            ["a", "b"],
            &["x", "y"],
            &["x", "y"],
            &[[4.0, 4.0], [0.0, 3.0], [3.0, 0.0], [3.0, 3.0]],
        );
        let report = analyse("g", &table, false);
        assert_eq!(
            report.why_not("g.equilibrium_4.a.x"),
            "'g.equilibrium_4.a.x' is not a result of game 'g': it has 3 equilibria \
             (equilibrium_1 to equilibrium_3)"
        );
        assert!(report
            .why_not("g.wins")
            .contains("a game's results are pure_equilibria"));
        let one = analyse("h", &two(["a", "b"], &["x"], &["y"], &[[1.0, 1.0]]), false);
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
            false,
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
