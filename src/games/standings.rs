//! What a tournament works out (ADR-068, sections 2 and 3).
//!
//! Each player's score, rank and cooperation, the match table, and which strategies are
//! collectively stable, and the same numbers as scalars named under it.

use super::repeated::{play, Length};
use super::strategy::Plays;
use super::table::Table;
use super::tournament::{Tournament, Worked};
use indexmap::IndexMap;
use serde::Serialize;

/// One player's place in a tournament.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Standing {
    pub player: String,
    pub plays: Plays,
    /// Its expected payoff per round over all its matches
    pub score: f64,
    /// 1 for the best score; players whose scores tie share a rank
    pub rank: usize,
    /// The expected share of its moves that were cooperation
    pub cooperation: f64,
}

/// Everything worked out for a tournament.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TournamentReport {
    pub game: String,
    /// The game played in each round
    pub round: String,
    /// The number of rounds of a match, when it is fixed
    pub rounds: Option<u64>,
    /// The chance that a match goes on after a round, when it ends at random
    pub continuation: Option<f64>,
    pub noise: f64,
    pub self_play: bool,
    /// Best first; players that tie are in the order of the model
    pub ranking: Vec<Standing>,
    /// `matches[a][b]`: a's expected payoff per round against b
    pub matches: IndexMap<String, IndexMap<String, f64>>,
    /// The collectively stable players, in the order of the model: no
    /// player does better against one than it does against itself
    pub stable: Vec<String>,
}

/// Play every match of `tournament`, with its numbers worked out and its
/// round's payoffs in `round` (a symmetric table of two strategies,
/// cooperate first).
#[must_use]
pub fn standings(tournament: &Tournament, round: &Table, worked: &Worked) -> TournamentReport {
    let pays = [
        [round.payoff(&[0, 0], 0), round.payoff(&[0, 1], 0)],
        [round.payoff(&[1, 0], 0), round.payoff(&[1, 1], 0)],
    ];
    let tie = round.tie();
    let n = worked.machines.len();
    let mut payoff = vec![vec![0.0; n]; n];
    let mut cooperation = vec![vec![0.0; n]; n];
    for a in 0..n {
        for b in a..n {
            let [first, second] = play(
                &worked.machines[a],
                &worked.machines[b],
                &pays,
                worked.length,
                worked.noise,
            );
            (payoff[a][b], cooperation[a][b]) = (first.payoff, first.cooperation);
            (payoff[b][a], cooperation[b][a]) = (second.payoff, second.cooperation);
        }
    }
    // Every match has the same expected length, so a score, the expected
    // total over the expected rounds, is the mean over the opponents
    let opponents = |a: usize| (0..n).filter(move |&b| tournament.self_play || b != a);
    let mean = |table: &[Vec<f64>], a: usize| {
        let values: Vec<f64> = opponents(a).map(|b| table[a][b]).collect();
        values.iter().sum::<f64>() / crate::core::precision::count(values.len())
    };
    let scores: Vec<f64> = (0..n).map(|a| mean(&payoff, a)).collect();
    let names: Vec<&String> = tournament.entrants.iter().map(|e| &e.name).collect();
    let mut ranking: Vec<Standing> = (0..n)
        .map(|a| Standing {
            player: names[a].clone(),
            plays: tournament.entrants[a].plays,
            score: scores[a],
            rank: 1 + scores.iter().filter(|&&s| s > scores[a] + tie).count(),
            cooperation: mean(&cooperation, a),
        })
        .collect();
    ranking.sort_by_key(|s| s.rank);
    let stable = (0..n)
        .filter(|&a| (0..n).all(|b| payoff[b][a] <= payoff[a][a] + tie))
        .map(|a| names[a].clone())
        .collect();
    let (rounds, continuation) = match worked.length {
        Length::Rounds(r) => (Some(r), None),
        Length::Continuation(w) => (None, Some(w)),
    };
    TournamentReport {
        game: tournament.name.clone(),
        round: tournament.round.clone(),
        rounds,
        continuation,
        noise: worked.noise,
        self_play: tournament.self_play,
        ranking,
        matches: names
            .iter()
            .zip(&payoff)
            .map(|(a, row)| {
                (
                    (*a).clone(),
                    names
                        .iter()
                        .map(|b| (*b).clone())
                        .zip(row.iter().copied())
                        .collect(),
                )
            })
            .collect(),
        stable,
    }
}

impl TournamentReport {
    /// Every result as a scalar named under the tournament, player by
    /// player in the order of the model.
    ///
    /// # Panics
    ///
    /// Never: [`standings`] gives every player of the match table a place in
    /// the ranking.
    #[must_use]
    pub fn scalars(&self) -> Vec<(String, f64)> {
        let g = &self.game;
        let mut out = Vec::new();
        for player in self.matches.keys() {
            let standing = self
                .ranking
                .iter()
                .find(|s| &s.player == player)
                .expect("every player has a standing");
            out.push((format!("{g}.score.{player}"), standing.score));
            out.push((
                format!("{g}.rank.{player}"),
                crate::core::precision::count(standing.rank),
            ));
            out.push((format!("{g}.cooperation.{player}"), standing.cooperation));
        }
        for (a, row) in &self.matches {
            for (b, payoff) in row {
                out.push((format!("{g}.match.{a}.{b}"), *payoff));
            }
        }
        for player in self.matches.keys() {
            let stable = self.stable.contains(player);
            out.push((format!("{g}.stable.{player}"), f64::from(u8::from(stable))));
        }
        out
    }

    /// Why `name`, under this tournament, is not one of its results.
    #[must_use]
    pub fn why_not(&self, name: &str) -> String {
        let players: Vec<&str> = self.matches.keys().map(String::as_str).collect();
        format!(
            "'{name}' is not a result of tournament '{}': a tournament's results are \
             score.<player>, rank.<player>, cooperation.<player>, match.<player>.<player> \
             and stable.<player>, and its players are {}",
            self.game,
            players.join(", ")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::model::{parse_games, Game};
    use crate::games::table::tests::two;

    /// The prisoner's dilemma of Axelrod's tournaments, R 3, S 0, T 5, P 1,
    /// with a cost `c` taken off each defection, as kinship-protocol's.
    fn dilemma(c: f64) -> Table {
        two(
            ["me", "you"],
            &["cooperate", "defect"],
            &["cooperate", "defect"],
            &[
                [3.0, 3.0],
                [0.0, 5.0 - c],
                [5.0 - c, 0.0],
                [1.0 - c, 1.0 - c],
            ],
        )
    }

    /// The tournament `players`, its numbers worked out with `value` for
    /// any formula.
    fn run(players: &str, ends: &str, round: &Table, value: f64) -> TournamentReport {
        let yaml = format!(
            "d: {{players: [a, b], strategies: [c, d], symmetric: true, payoffs: {{c: {{c: 3, d: 0}}, d: {{c: 5, d: 1}}}}}}
t:
  tournament:
    round: d
    players: {{{players}}}
    {ends}
"
        );
        let games = parse_games(&yaml_serde::from_str(&yaml).unwrap()).unwrap();
        let t = games
            .into_iter()
            .find_map(|g| match g {
                Game::Tournament(t) => Some(t),
                Game::Table(_) => None,
            })
            .unwrap();
        let worked = t.work_out(&mut |_, _| Ok(value)).unwrap();
        standings(&t, round, &worked)
    }

    fn value(scalars: &[(String, f64)], name: &str) -> f64 {
        scalars.iter().find(|(n, _)| n == name).unwrap().1
    }

    const KINSHIP: &str =
        "grim: grim_trigger, tft: tit_for_tat, ad: always_defect, ac: always_cooperate, rand: random";

    #[test]
    fn kinship_protocols_tournament_has_exact_scores() {
        // kinship-protocol's axelrod_tournament.yaml: five strategies, 200
        // rounds, self-play. By hand (ADR-068): Grim's matches pay 3, 3,
        // 199/200, 3 and 2.985, a mean of 2.596. axelrod 4.14.0 with 300
        // seeded runs of each match gives 2.599, 2.447, 2.208, 2.102 and
        // 1.909; kinship stores 2.622, 2.445, 2.212, 2.112 and 1.9, half of
        // a table that cannot be right (5.244 a round, more than T = 5).
        let report = run(KINSHIP, "rounds: 200", &dilemma(0.0), 0.0);
        let expected = [
            ("grim", 2.596),
            ("tft", 2.448_25),
            ("ad", 2.208),
            ("ac", 2.1),
            ("rand", 1.908_75),
        ];
        for (standing, (player, score)) in report.ranking.iter().zip(expected) {
            assert_eq!(standing.player, player);
            assert!(
                (standing.score - score).abs() < 1e-12,
                "{player}: {}",
                standing.score
            );
        }
        assert_eq!(
            report.ranking.iter().map(|s| s.rank).collect::<Vec<_>>(),
            [1, 2, 3, 4, 5]
        );
        // TFT against AD over 200 rounds: 199/200, by hand
        assert!((report.matches["tft"]["ad"] - 0.995).abs() < 1e-12);
        let scalars = report.scalars();
        assert!((value(&scalars, "t.score.tft") - 2.448_25).abs() < 1e-12);
        assert!((value(&scalars, "t.rank.rand") - 5.0).abs() < 1e-12);
        assert!((value(&scalars, "t.cooperation.ad")).abs() < 1e-12);
        assert!((value(&scalars, "t.cooperation.ac") - 1.0).abs() < 1e-12);
        assert!((value(&scalars, "t.cooperation.rand") - 0.5).abs() < 1e-12);
        assert!((value(&scalars, "t.match.ad.ac") - 5.0).abs() < 1e-12);
        // 3 per player, 25 matches, 5 stable flags
        assert_eq!(scalars.len(), 15 + 25 + 5);
        // Over a fixed 200 rounds, nothing in the roster beats Grim or TFT
        // against themselves (3); AD beats AC (5 > 3) and Random
        // (3 > 2.25), and AD is stable, as nothing beats 1 against it
        assert_eq!(report.stable, ["grim", "tft", "ad"]);
        assert!((value(&scalars, "t.stable.ac")).abs() < 1e-12);
        assert!((value(&scalars, "t.stable.grim") - 1.0).abs() < 1e-12);
        assert_eq!(report.rounds, Some(200));
        assert_eq!(report.continuation, None);
    }

    #[test]
    fn the_crossover_of_always_cooperate_and_always_defect_is_0_108() {
        // kinship-protocol stores crossover_c 0.1 from Grok's numbers. With
        // the exact scores, AD defects every time and AC never, so the cost
        // c of a defection closes the gap of 0.108 at c = 0.108, by hand:
        // AD's score loses exactly c, AC's loses only what its opponents'
        // defections cost them, which is nothing to AC itself
        for (c, ac_first) in [(0.107, false), (0.109, true)] {
            let report = run(KINSHIP, "rounds: 200", &dilemma(c), 0.0);
            let score = |p: &str| report.ranking.iter().find(|s| s.player == p).unwrap().score;
            assert!((score("ad") - (2.208 - c)).abs() < 1e-12, "c {c}");
            assert!((score("ac") - 2.1).abs() < 1e-12, "c {c}");
            assert_eq!(score("ac") > score("ad"), ac_first, "c {c}");
        }
    }

    #[test]
    fn noise_moves_a_forgiving_strategy_ahead_of_tit_for_tat() {
        // The noisy tournament of ADR-068, its roster fixed on the exact
        // numbers: without noise TFT is first; with each move flipped with
        // 0.05, TitForTwoTats is first, generous TFT second and TFT third.
        // axelrod 4.14.0, 2,000 runs of each match with seeds 0 to 1999:
        // tf2t 2.51741, gtft 2.47058, tft 2.45689, ac 2.35040, ad 2.23713,
        // the same order; every match within 2.2 standard errors.
        let roster = "ac: always_cooperate, ad: always_defect, tft: tit_for_tat, \
                      gtft: generous_tit_for_tat, tf2t: tit_for_two_tats";
        let clean = run(roster, "rounds: 200", &dilemma(0.0), 0.0);
        assert_eq!(clean.ranking[0].player, "tft");
        let noisy = run(
            roster,
            "rounds: 200\n    noise: =noise",
            &dilemma(0.0),
            0.05,
        );
        let order: Vec<&str> = noisy.ranking.iter().map(|s| s.player.as_str()).collect();
        assert_eq!(order, ["tf2t", "gtft", "tft", "ac", "ad"]);
        let axelrod = [2.51741, 2.47058, 2.45689, 2.35040, 2.23713];
        for (standing, mean) in noisy.ranking.iter().zip(axelrod) {
            assert!(
                (standing.score - mean).abs() < 0.005,
                "{}: {}",
                standing.player,
                standing.score
            );
        }
        assert!((noisy.noise - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn grim_trigger_becomes_stable_at_the_threshold_continuation() {
        // ADR-068: against AD, Grim is stable once w >= (T - c - R) / (T - P),
        // 0.5 at c = 0. By hand, AD gets (1 - w) 5 + w 1 against Grim, and
        // Grim gets 3 against itself.
        let roster = "grim: grim_trigger, ad: always_defect";
        for (w, stable) in [(0.45, false), (0.5, true), (0.55, true)] {
            let report = run(roster, "continuation: =w", &dilemma(0.0), w);
            assert_eq!(report.stable.contains(&"grim".to_string()), stable, "w {w}");
            assert_eq!(report.continuation, Some(w));
        }
        // With c = 0.5 the threshold is (5 - 0.5 - 3) / 4 = 0.375
        let report = run(roster, "continuation: =w", &dilemma(0.5), 0.37);
        assert!(!report.stable.contains(&"grim".to_string()));
        let report = run(roster, "continuation: =w", &dilemma(0.5), 0.38);
        assert!(report.stable.contains(&"grim".to_string()));
    }

    #[test]
    fn without_self_play_a_score_is_over_the_others() {
        // By hand: AC gets 0 against AD, AD 5 against AC; their own matches
        // (3 and 1) are left out of the score but kept in the match table
        let report = run(
            "ac: always_cooperate, ad: always_defect",
            "rounds: 10\n    self_play: false",
            &dilemma(0.0),
            0.0,
        );
        assert!((report.ranking[0].score - 5.0).abs() < 1e-12);
        assert!((report.ranking[1].score).abs() < 1e-12);
        assert!((report.matches["ac"]["ac"] - 3.0).abs() < 1e-12);
        assert!(!report.self_play);
        // Ties share a rank: two AC players score the same
        let tied = run(
            "a: always_cooperate, b: always_cooperate",
            "rounds: 3",
            &dilemma(0.0),
            0.0,
        );
        assert_eq!(
            tied.ranking.iter().map(|s| s.rank).collect::<Vec<_>>(),
            [1, 1]
        );
        assert_eq!(tied.round, "d");
        assert!(tied
            .why_not("t.score.c")
            .contains("a tournament's results are score.<player>"));
        assert!(tied.why_not("t.score.c").ends_with("its players are a, b"));
    }
}
