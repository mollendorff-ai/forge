//! Every equilibrium of a two-player game, mixed ones included, by support
//! enumeration (ADR-068, section 2). For more players, the pure ones only.
//!
//! For each pair of supports of the same size, one strategy set for each
//! player, the mix of each player that leaves the other indifferent over
//! their support is the solution of a linear system. It is an equilibrium
//! when both mixes are probabilities and no strategy outside a support pays
//! more. In a nondegenerate game this finds every equilibrium, each once.
//! A game is degenerate when a mix of k strategies has more than k best
//! replies: then a whole line of equilibria can exist and some can be missed,
//! and the ties that make it so are named, so a list that may be incomplete
//! is never shown as complete.

use super::linear::solve;
use super::pure::pure_equilibria;
use super::table::Table;

/// Mixed equilibria are looked for when each player has at most this many
/// strategies: 184,756 pairs of supports for 10 and 10.
pub const MOST_STRATEGIES_FOR_MIXED: usize = 10;

/// A weight below this is a zero.
const ZERO: f64 = 1e-9;

/// One equilibrium: each player's probability of each strategy, and each
/// player's expected payoff.
#[derive(Debug, Clone, PartialEq)]
pub struct Equilibrium {
    pub probabilities: Vec<Vec<f64>>,
    pub payoffs: Vec<f64>,
}

impl Equilibrium {
    /// Whether every player plays one strategy for certain.
    #[must_use]
    pub fn is_pure(&self) -> bool {
        self.probabilities
            .iter()
            .all(|mix| mix.iter().filter(|&&p| p > ZERO).count() == 1)
    }
}

/// The equilibria of a game, and what may make the list incomplete.
#[derive(Debug, Clone, PartialEq)]
pub struct Equilibria {
    /// Pure ones first, in table order, then mixed ones by support size
    pub list: Vec<Equilibrium>,
    /// Whether mixed equilibria were looked for (two players, at most
    /// [`MOST_STRATEGIES_FOR_MIXED`] strategies each)
    pub mixed_searched: bool,
    /// Each tie that makes the game degenerate, in words; empty when it is not
    pub ties: Vec<String>,
}

/// Every equilibrium of `table` (see the module's description).
#[must_use]
pub fn equilibria(table: &Table) -> Equilibria {
    let two = table.players.len() == 2
        && table
            .strategies
            .iter()
            .all(|s| s.len() <= MOST_STRATEGIES_FOR_MIXED);
    if !two {
        let list = pure_equilibria(table)
            .iter()
            .map(|profile| pure(table, profile))
            .collect();
        return Equilibria {
            list,
            mixed_searched: false,
            ties: Vec::new(),
        };
    }
    let (row_count, column_count) = (table.strategies[0].len(), table.strategies[1].len());
    let first: Vec<Vec<f64>> = (0..row_count)
        .map(|i| {
            (0..column_count)
                .map(|j| table.payoff(&[i, j], 0))
                .collect()
        })
        .collect();
    let second: Vec<Vec<f64>> = (0..row_count)
        .map(|i| {
            (0..column_count)
                .map(|j| table.payoff(&[i, j], 1))
                .collect()
        })
        .collect();
    let tie = table.tie();
    let mut ties = pure_ties(table, &first, &second, tie);
    let mut list = Vec::new();
    for size in 1..=row_count.min(column_count) {
        for rows in subsets(row_count, size) {
            for columns in subsets(column_count, size) {
                let Some(column_mix) = indifferent(&rows, &columns, |i, j| first[i][j]) else {
                    continue;
                };
                let Some(row_mix) = indifferent(&columns, &rows, |j, i| second[i][j]) else {
                    continue;
                };
                let found = Candidate {
                    x: spread(row_count, &rows, &row_mix),
                    y: spread(column_count, &columns, &column_mix),
                };
                if let Some(equilibrium) =
                    found.check(table, (&first, &second), size, tie, &mut ties)
                {
                    list.push(equilibrium);
                }
            }
        }
    }
    ties.dedup();
    Equilibria {
        list,
        mixed_searched: true,
        ties,
    }
}

/// The equilibrium of a pure profile.
fn pure(table: &Table, profile: &[usize]) -> Equilibrium {
    Equilibrium {
        probabilities: profile
            .iter()
            .zip(&table.strategies)
            .map(|(&s, list)| {
                (0..list.len())
                    .map(|t| if t == s { 1.0 } else { 0.0 })
                    .collect()
            })
            .collect(),
        payoffs: (0..table.players.len())
            .map(|p| table.payoff(profile, p))
            .collect(),
    }
}

/// The ties among one player's best replies to a pure strategy of the
/// other: a strategy (a mix of one) with two best replies.
fn pure_ties(table: &Table, a: &[Vec<f64>], b: &[Vec<f64>], tie: f64) -> Vec<String> {
    let mut ties = Vec::new();
    let names = &table.strategies;
    for (j, column) in names[1].iter().enumerate() {
        let replies: Vec<f64> = a.iter().map(|row| row[j]).collect();
        if let Some(best) = tied_best(&replies, tie) {
            ties.push(format!(
                "against {}'s {column}, {}'s {} pay the same",
                table.players[1],
                table.players[0],
                and_list(
                    &best
                        .iter()
                        .map(|&i| names[0][i].as_str())
                        .collect::<Vec<_>>()
                )
            ));
        }
    }
    for (i, row) in names[0].iter().enumerate() {
        if let Some(best) = tied_best(&b[i], tie) {
            ties.push(format!(
                "against {}'s {row}, {}'s {} pay the same",
                table.players[0],
                table.players[1],
                and_list(
                    &best
                        .iter()
                        .map(|&j| names[1][j].as_str())
                        .collect::<Vec<_>>()
                )
            ));
        }
    }
    ties
}

/// The best replies among `payoffs`, when there are two or more.
fn tied_best(payoffs: &[f64], tie: f64) -> Option<Vec<usize>> {
    let most = payoffs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let best: Vec<usize> = (0..payoffs.len())
        .filter(|&s| payoffs[s] >= most - tie)
        .collect();
    (best.len() > 1).then_some(best)
}

/// `a`, `a and b`, `a, b and c`.
fn and_list(names: &[&str]) -> String {
    match names {
        [.., last] if names.len() > 1 => {
            format!("{} and {last}", names[..names.len() - 1].join(", "))
        },
        _ => names.join(""),
    }
}

/// The mix over `mixer` (k strategies) that makes every strategy of
/// `chooser` (k strategies) pay the same, where `pay(c, s)` is what the
/// chooser's strategy `c` gets against the mixer's `s`. `None` when the
/// equations have no single solution.
fn indifferent(
    chooser: &[usize],
    mixer: &[usize],
    pay: impl Fn(usize, usize) -> f64,
) -> Option<Vec<f64>> {
    let k = mixer.len();
    // Unknowns: the k weights, then the common payoff
    let mut rows: Vec<Vec<f64>> = chooser
        .iter()
        .map(|&c| {
            let mut row: Vec<f64> = mixer.iter().map(|&s| pay(c, s)).collect();
            row.push(-1.0);
            row
        })
        .collect();
    let mut sum = vec![1.0; k];
    sum.push(0.0);
    rows.push(sum);
    let mut right = vec![0.0; k];
    right.push(1.0);
    let mut solution = solve(rows, right)?;
    solution.truncate(k);
    Some(solution)
}

/// A mix over `support`, spread out over all `n` strategies.
fn spread(n: usize, support: &[usize], weights: &[f64]) -> Vec<f64> {
    let mut mix = vec![0.0; n];
    for (&s, &w) in support.iter().zip(weights) {
        mix[s] = w;
    }
    mix
}

/// A pair of mixes, one for each player, from the indifference equations.
struct Candidate {
    x: Vec<f64>,
    y: Vec<f64>,
}

impl Candidate {
    /// The equilibrium, if the mixes are probabilities and nothing outside
    /// the supports (of `size` strategies each) pays more. A zero weight inside
    /// a support, or a best reply outside one, is a tie, added to `ties`; an
    /// equilibrium with a zero weight is left out, since its smaller supports
    /// give it.
    fn check(
        &self,
        table: &Table,
        (first, second): (&[Vec<f64>], &[Vec<f64>]),
        size: usize,
        tie: f64,
        ties: &mut Vec<String>,
    ) -> Option<Equilibrium> {
        let support = |mix: &[f64]| -> Vec<usize> {
            (0..mix.len()).filter(|&s| mix[s].abs() > ZERO).collect()
        };
        let (rows, columns) = (support(&self.x), support(&self.y));
        if self.x.iter().chain(&self.y).any(|&w| w < -ZERO) {
            return None;
        }
        // Each row's payoff against y, each column's against x
        let row_pays: Vec<f64> = first
            .iter()
            .map(|row| row.iter().zip(&self.y).map(|(p, w)| p * w).sum())
            .collect();
        let column_pays: Vec<f64> = (0..self.y.len())
            .map(|j| second.iter().zip(&self.x).map(|(row, w)| row[j] * w).sum())
            .collect();
        let row_value: f64 = rows.iter().map(|&i| row_pays[i] * self.x[i]).sum();
        let column_value: f64 = columns.iter().map(|&j| column_pays[j] * self.y[j]).sum();
        if row_pays.iter().any(|&p| p > row_value + tie)
            || column_pays.iter().any(|&p| p > column_value + tie)
        {
            return None;
        }
        let best_rows = row_pays.iter().filter(|&&p| p >= row_value - tie).count();
        let best_columns = column_pays
            .iter()
            .filter(|&&p| p >= column_value - tie)
            .count();
        let shrunk = rows.len() != size || columns.len() != size;
        if shrunk || best_rows > size || best_columns > size {
            let names = |player: usize, mix: &[f64]| -> String {
                and_list(
                    &support(mix)
                        .iter()
                        .map(|&s| table.strategies[player][s].as_str())
                        .collect::<Vec<_>>(),
                )
            };
            ties.push(format!(
                "with {} mixing {} and {} mixing {}, more strategies than these pay the most",
                table.players[0],
                names(0, &self.x),
                table.players[1],
                names(1, &self.y)
            ));
            if shrunk {
                return None;
            }
        }
        Some(Equilibrium {
            probabilities: vec![self.x.clone(), self.y.clone()],
            payoffs: vec![row_value, column_value],
        })
    }
}

/// Every set of `k` of `n` strategies, in order (`[0, 1]`, `[0, 2]`, ...).
fn subsets(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut all = Vec::new();
    walk(n, k, 0, &mut Vec::with_capacity(k), &mut all);
    all
}

/// Add to `all` every way to finish `current` with strategies from `start` on.
fn walk(n: usize, k: usize, start: usize, current: &mut Vec<usize>, all: &mut Vec<Vec<usize>>) {
    if current.len() == k {
        all.push(current.clone());
        return;
    }
    for s in start..n {
        current.push(s);
        walk(n, k, s + 1, current, all);
        current.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::table::tests::two;

    fn close(got: &[f64], want: &[f64]) -> bool {
        got.len() == want.len() && got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-9)
    }

    fn assert_mix(e: &Equilibrium, x: &[f64], y: &[f64], payoffs: &[f64]) {
        assert!(
            close(&e.probabilities[0], x) && close(&e.probabilities[1], y),
            "{e:?}"
        );
        assert!(close(&e.payoffs, payoffs), "{e:?}");
    }

    #[test]
    fn the_stag_hunt_mixes_stag_three_times_in_four() {
        // nashpy 0.0.43, support_enumeration on [[4, 0], [3, 3]] and its
        // transpose: ([1, 0], [1, 0]), ([0, 1], [0, 1]), ([0.75, 0.25],
        // [0.75, 0.25]). By hand: stag pays 4q against stag played with q,
        // hare pays 3, so q = 3/4, and each gets 3.
        let table = two(
            ["me", "you"],
            &["stag", "hare"],
            &["stag", "hare"],
            &[[4.0, 4.0], [0.0, 3.0], [3.0, 0.0], [3.0, 3.0]],
        );
        let found = equilibria(&table);
        assert!(found.mixed_searched && found.ties.is_empty(), "{found:?}");
        assert_eq!(found.list.len(), 3);
        assert_mix(&found.list[0], &[1.0, 0.0], &[1.0, 0.0], &[4.0, 4.0]);
        assert_mix(&found.list[1], &[0.0, 1.0], &[0.0, 1.0], &[3.0, 3.0]);
        assert_mix(&found.list[2], &[0.75, 0.25], &[0.75, 0.25], &[3.0, 3.0]);
        assert!(found.list[0].is_pure() && !found.list[2].is_pure());
    }

    #[test]
    fn chicken_has_two_pure_equilibria_and_swerves_a_third_of_the_time() {
        // nashpy 0.0.43: ([1, 0], [0, 1]), ([0, 1], [1, 0]) and
        // ([1/3, 2/3], [1/3, 2/3]). By hand: swerve pays 3q + (1 - q),
        // straight pays 5q, equal at q = 1/3; each gets 5/3.
        let table = two(
            ["me", "you"],
            &["swerve", "straight"],
            &["swerve", "straight"],
            &[[3.0, 3.0], [1.0, 5.0], [5.0, 1.0], [0.0, 0.0]],
        );
        let found = equilibria(&table);
        assert_eq!(found.list.len(), 3);
        assert_mix(&found.list[0], &[1.0, 0.0], &[0.0, 1.0], &[1.0, 5.0]);
        assert_mix(&found.list[1], &[0.0, 1.0], &[1.0, 0.0], &[5.0, 1.0]);
        let third = 1.0 / 3.0;
        let each = 5.0 / 3.0;
        assert_mix(
            &found.list[2],
            &[third, 1.0 - third],
            &[third, 1.0 - third],
            &[each, each],
        );
    }

    #[test]
    fn the_inspection_game_has_only_a_mixed_equilibrium() {
        // The AI complies or cheats, the human audits or trusts: cheating
        // gains 2 uncaught and costs the penalty 4 if caught; an audit costs
        // the human 1, undetected cheating costs 3. nashpy 0.0.43 on
        // [[0, 0], [-4, 2]] and [[-1, 0], [-1, -3]]: one equilibrium,
        // ([2/3, 1/3], [1/3, 2/3]). By hand: audit = gain / (gain + penalty)
        // = 1/3, cheat = audit cost / harm = 1/3; the AI gets 0, the human
        // -1.
        let table = two(
            ["ai", "human"],
            &["comply", "cheat"],
            &["audit", "trust"],
            &[[0.0, -1.0], [0.0, 0.0], [-4.0, -1.0], [2.0, -3.0]],
        );
        let found = equilibria(&table);
        assert_eq!(found.list.len(), 1, "{found:?}");
        assert_mix(
            &found.list[0],
            &[2.0 / 3.0, 1.0 / 3.0],
            &[1.0 / 3.0, 2.0 / 3.0],
            &[0.0, -1.0],
        );
    }

    #[test]
    fn a_harsher_penalty_lowers_the_audit_rate_and_not_the_cheating() {
        // nashpy 0.0.43 with the penalty at 8: ([2/3, 1/3], [0.2, 0.8]).
        // By hand: audit = 2 / (2 + 8) = 1/5; cheating stays 1/3.
        let table = two(
            ["ai", "human"],
            &["comply", "cheat"],
            &["audit", "trust"],
            &[[0.0, -1.0], [0.0, 0.0], [-8.0, -1.0], [2.0, -3.0]],
        );
        let found = equilibria(&table);
        assert_eq!(found.list.len(), 1);
        assert_mix(
            &found.list[0],
            &[2.0 / 3.0, 1.0 / 3.0],
            &[0.2, 0.8],
            &[0.0, -1.0],
        );
    }

    #[test]
    fn the_battle_of_the_sexes_has_two_pure_and_one_mixed() {
        // nashpy 0.0.43 on [[2, 0], [0, 1]] and [[1, 0], [0, 2]]:
        // ([1, 0], [1, 0]), ([0, 1], [0, 1]), ([2/3, 1/3], [1/3, 2/3]).
        // By hand: the second goes to the opera with q where 2q = 1 - q,
        // q = 1/3; each expects 2/3.
        let table = two(
            ["first", "second"],
            &["opera", "football"],
            &["opera", "football"],
            &[[2.0, 1.0], [0.0, 0.0], [0.0, 0.0], [1.0, 2.0]],
        );
        let found = equilibria(&table);
        assert_eq!(found.list.len(), 3);
        assert_mix(
            &found.list[2],
            &[2.0 / 3.0, 1.0 / 3.0],
            &[1.0 / 3.0, 2.0 / 3.0],
            &[2.0 / 3.0, 2.0 / 3.0],
        );
    }

    #[test]
    fn rock_paper_scissors_mixes_all_three_evenly() {
        // nashpy 0.0.43: one equilibrium, each strategy 1/3, value 0; the
        // search goes through supports of 3, and no pure cell is one.
        let win = [[0.0, -1.0, 1.0], [1.0, 0.0, -1.0], [-1.0, 1.0, 0.0]];
        let cells: Vec<[f64; 2]> = win.iter().flatten().map(|&p| [p, -p]).collect();
        let table = two(
            ["a", "b"],
            &["rock", "paper", "scissors"],
            &["rock", "paper", "scissors"],
            &cells,
        );
        let found = equilibria(&table);
        assert_eq!(found.list.len(), 1, "{found:?}");
        let third = [1.0 / 3.0; 3];
        assert_mix(&found.list[0], &third, &third, &[0.0, 0.0]);
        assert_eq!(found.ties, Vec::<String>::new());
    }

    #[test]
    fn a_degenerate_game_names_its_ties() {
        // By hand: against x, the row player's a and b both pay 1, so a mix
        // of one (x) has two best replies: degenerate. Every mix of a and b
        // against x is an equilibrium (the column player gets 1 from x and
        // 0 from y); the pure ones are listed, the line between them cannot
        // be, and the tie is named.
        let table = two(
            ["row", "column"],
            &["a", "b"],
            &["x", "y"],
            &[[1.0, 1.0], [0.0, 0.0], [1.0, 1.0], [0.0, 0.0]],
        );
        let found = equilibria(&table);
        assert_eq!(found.list.len(), 2, "{found:?}");
        assert!(found.list.iter().all(Equilibrium::is_pure));
        assert!(
            found
                .ties
                .contains(&"against column's x, row's a and b pay the same".to_string()),
            "{:?}",
            found.ties
        );
        assert!(
            found
                .ties
                .contains(&"against column's y, row's a and b pay the same".to_string()),
            "{:?}",
            found.ties
        );
    }

    #[test]
    fn a_mix_with_an_extra_best_reply_is_named() {
        // By hand: row mixing a and b evenly against column mixing x and y
        // evenly is an equilibrium (a and b pay 1/2 each, x and y pay 1/2
        // each). c pays 1/2 against anything, so it is a third best reply
        // for a support of two: degenerate at that mix. The equilibrium is
        // still listed, and the tie named. nashpy 0.0.43 finds the same one
        // equilibrium, ([1/2, 1/2, 0], [1/2, 1/2]).
        let table = two(
            ["row", "column"],
            &["a", "b", "c"],
            &["x", "y"],
            &[
                [1.0, 0.0],
                [0.0, 1.0],
                [0.0, 1.0],
                [1.0, 0.0],
                [0.5, 0.0],
                [0.5, 0.0],
            ],
        );
        let found = equilibria(&table);
        assert_eq!(found.list.len(), 1, "{found:?}");
        assert_mix(&found.list[0], &[0.5, 0.5, 0.0], &[0.5, 0.5], &[0.5, 0.5]);
        assert!(
            found
                .ties
                .iter()
                .any(|t| t == "with row mixing a and b and column mixing x and y, more strategies than these pay the most"),
            "{:?}",
            found.ties
        );
    }

    #[test]
    fn three_players_get_their_pure_equilibria_only() {
        // Each of three gets 1 when all choose the same: by hand, (x, x, x)
        // and (y, y, y); mixed ones are not looked for.
        let mut payoffs = Vec::new();
        for a in 0..2 {
            for b in 0..2 {
                for c in 0..2 {
                    payoffs.push(vec![if a == b && b == c { 1.0 } else { 0.0 }; 3]);
                }
            }
        }
        let table = Table {
            players: vec!["a".into(), "b".into(), "c".into()],
            strategies: vec![vec!["x".into(), "y".into()]; 3],
            payoffs,
        };
        let found = equilibria(&table);
        assert!(!found.mixed_searched);
        assert_eq!(found.list.len(), 2);
        assert_eq!(found.list[1].probabilities, vec![vec![0.0, 1.0]; 3]);
        assert_eq!(found.list[1].payoffs, vec![1.0; 3]);
    }

    #[test]
    fn more_than_ten_strategies_get_pure_equilibria_only() {
        // Eleven strategies against one: by hand, the row player's best,
        // the last (payoff 10), is the one equilibrium.
        let rows: Vec<String> = (0..11).map(|i| format!("s{i}")).collect();
        let names: Vec<&str> = rows.iter().map(String::as_str).collect();
        let cells: Vec<[f64; 2]> = (0..11).map(|i| [f64::from(i), 0.0]).collect();
        let table = two(["a", "b"], &names, &["only"], &cells);
        let found = equilibria(&table);
        assert!(!found.mixed_searched);
        assert_eq!(found.list.len(), 1);
        assert!((found.list[0].probabilities[0][10] - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn subsets_and_lists_read_in_order() {
        assert_eq!(subsets(3, 2), vec![vec![0, 1], vec![0, 2], vec![1, 2]]);
        assert_eq!(and_list(&["a"]), "a");
        assert_eq!(and_list(&["a", "b", "c"]), "a, b and c");
    }
}
