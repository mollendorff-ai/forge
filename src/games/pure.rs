//! What a game's pure strategies alone tell, for any number of players.
//!
//! Which strategies are dominated, which cells are equilibria, and what each
//! strategy pays against an even mix (ADR-068, section 2).

use super::table::Table;

/// A strategy removed because another of its player's strategies pays more
/// against everything the others can still play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Removed {
    pub player: usize,
    pub strategy: usize,
    /// The strategy that pays more
    pub by: usize,
}

/// A strategy that another pays at least as much as everywhere, and more
/// somewhere, in the whole table. Reported, not removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeaklyDominated {
    pub player: usize,
    pub strategy: usize,
    pub by: usize,
}

/// The profiles of the others, with `player`'s strategy left at 0, over the
/// strategies still in play (`alive`).
fn others(alive: &[Vec<bool>], player: usize) -> Vec<Vec<usize>> {
    let mut all: Vec<Vec<usize>> = vec![Vec::new()];
    for (p, list) in alive.iter().enumerate() {
        let choices: Vec<usize> = if p == player {
            vec![0]
        } else {
            (0..list.len()).filter(|&s| list[s]).collect()
        };
        all = all
            .into_iter()
            .flat_map(|prefix| {
                choices.iter().map(move |&s| {
                    let mut next = prefix.clone();
                    next.push(s);
                    next
                })
            })
            .collect();
    }
    all
}

/// `player`'s payoff with `strategy` against `rest` (whose own entry is ignored).
fn pays(table: &Table, rest: &[usize], player: usize, strategy: usize) -> f64 {
    let mut profile = rest.to_vec();
    profile[player] = strategy;
    table.payoff(&profile, player)
}

/// Remove strictly dominated strategies one at a time, until none is left.
///
/// Only a pure strategy dominates here. Each step takes the first player,
/// then the first strategy, that can go, so the order is the same on every
/// run.
#[must_use]
pub fn iterated_dominance(table: &Table) -> Vec<Removed> {
    let tie = table.tie();
    let mut alive: Vec<Vec<bool>> = table
        .strategies
        .iter()
        .map(|s| vec![true; s.len()])
        .collect();
    let mut removed = Vec::new();
    'search: loop {
        for player in 0..table.players.len() {
            let rest = others(&alive, player);
            let live: Vec<usize> = (0..alive[player].len())
                .filter(|&s| alive[player][s])
                .collect();
            for &strategy in &live {
                for &by in &live {
                    if by != strategy
                        && rest.iter().all(|o| {
                            pays(table, o, player, by) > pays(table, o, player, strategy) + tie
                        })
                    {
                        alive[player][strategy] = false;
                        removed.push(Removed {
                            player,
                            strategy,
                            by,
                        });
                        continue 'search;
                    }
                }
            }
        }
        return removed;
    }
}

/// Strategies weakly dominated in the whole table, by one of the player's
/// own: never paying more, and paying less against something.
#[must_use]
pub fn weakly_dominated(table: &Table) -> Vec<WeaklyDominated> {
    let tie = table.tie();
    let alive: Vec<Vec<bool>> = table
        .strategies
        .iter()
        .map(|s| vec![true; s.len()])
        .collect();
    let mut found = Vec::new();
    for player in 0..table.players.len() {
        let rest = others(&alive, player);
        let count = table.strategies[player].len();
        for strategy in 0..count {
            let by = (0..count).find(|&by| {
                by != strategy
                    && rest.iter().all(|o| {
                        pays(table, o, player, by) >= pays(table, o, player, strategy) - tie
                    })
                    && rest.iter().any(|o| {
                        pays(table, o, player, by) > pays(table, o, player, strategy) + tie
                    })
            });
            if let Some(by) = by {
                found.push(WeaklyDominated {
                    player,
                    strategy,
                    by,
                });
            }
        }
    }
    found
}

/// Every cell where no player gains by changing strategy alone, in table
/// order. Complete for any number of players.
#[must_use]
pub fn pure_equilibria(table: &Table) -> Vec<Vec<usize>> {
    let tie = table.tie();
    table
        .profiles()
        .into_iter()
        .filter(|profile| {
            (0..table.players.len()).all(|player| {
                let here = table.payoff(profile, player);
                (0..table.strategies[player].len())
                    .all(|other| pays(table, profile, player, other) <= here + tie)
            })
        })
        .collect()
}

/// For each player and strategy, its payoff when every other player plays
/// each of their strategies equally often.
#[must_use]
pub fn even_mix(table: &Table) -> Vec<Vec<f64>> {
    let alive: Vec<Vec<bool>> = table
        .strategies
        .iter()
        .map(|s| vec![true; s.len()])
        .collect();
    (0..table.players.len())
        .map(|player| {
            let rest = others(&alive, player);
            // The number of the others' profiles; there is at least one
            let count = crate::core::precision::count(rest.len());
            (0..table.strategies[player].len())
                .map(|strategy| {
                    rest.iter()
                        .map(|o| pays(table, o, player, strategy))
                        .sum::<f64>()
                        / count
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::table::tests::two;

    // The stag hunt of kinship-protocol's stag_hunt.yaml: stag/stag 4,
    // stag/hare 0, hare/stag 3, hare/hare 3.
    fn stag_hunt() -> Table {
        two(
            ["me", "you"],
            &["stag", "hare"],
            &["stag", "hare"],
            &[[4.0, 4.0], [0.0, 3.0], [3.0, 0.0], [3.0, 3.0]],
        )
    }

    #[test]
    fn the_stag_hunt_has_two_pure_equilibria_and_nothing_dominated() {
        // nashpy 0.0.43, support_enumeration: (stag, stag) and (hare, hare)
        // are its two pure equilibria; by hand, neither strategy pays more
        // against both of the other's.
        let table = stag_hunt();
        assert_eq!(pure_equilibria(&table), vec![vec![0, 0], vec![1, 1]]);
        assert_eq!(iterated_dominance(&table), Vec::new());
        assert_eq!(weakly_dominated(&table), Vec::new());
    }

    #[test]
    fn the_even_mix_is_kinship_protocols_coop_base_and_defect_base() {
        // stag_hunt.yaml: coop_base = 0.5 * 4 + 0.5 * 0 = 2,
        // defect_base = 0.5 * 3 + 0.5 * 3 = 3
        let even = even_mix(&stag_hunt());
        assert_eq!(even, vec![vec![2.0, 3.0], vec![2.0, 3.0]]);
    }

    #[test]
    fn the_prisoners_dilemma_removes_cooperate_for_both() {
        // By hand: with T 5, R 3, P 1, S 0, defect pays 5 > 3 against
        // cooperate and 1 > 0 against defect, so cooperate goes for each
        // player in turn; the one pure equilibrium is (defect, defect), as
        // nashpy 0.0.43 finds.
        let table = two(
            ["a", "b"],
            &["cooperate", "defect"],
            &["cooperate", "defect"],
            &[[3.0, 3.0], [0.0, 5.0], [5.0, 0.0], [1.0, 1.0]],
        );
        assert_eq!(
            iterated_dominance(&table),
            vec![
                Removed {
                    player: 0,
                    strategy: 0,
                    by: 1
                },
                Removed {
                    player: 1,
                    strategy: 0,
                    by: 1
                },
            ]
        );
        assert_eq!(pure_equilibria(&table), vec![vec![1, 1]]);
    }

    #[test]
    fn dominance_is_iterated_through_what_the_other_can_still_play() {
        // By hand: for the column player, right pays less than middle in
        // every row (0 < 1, 1 < 2), so right goes. Then, against left and
        // middle only, down pays less than up (0 < 1, 0 < 2) and goes. Then
        // against up, left pays less than middle (0 < 2) and goes. What is
        // left, (up, middle), is the one pure equilibrium (nashpy 0.0.43).
        let table = two(
            ["row", "column"],
            &["up", "down"],
            &["left", "middle", "right"],
            &[
                [1.0, 0.0],
                [1.0, 2.0],
                [0.0, 1.0],
                [0.0, 3.0],
                [0.0, 1.0],
                [2.0, 0.0],
            ],
        );
        assert_eq!(
            iterated_dominance(&table),
            vec![
                Removed {
                    player: 1,
                    strategy: 2,
                    by: 1
                },
                Removed {
                    player: 0,
                    strategy: 1,
                    by: 0
                },
                Removed {
                    player: 1,
                    strategy: 0,
                    by: 1
                },
            ]
        );
        assert_eq!(pure_equilibria(&table), vec![vec![0, 1]]);
    }

    #[test]
    fn a_tie_is_weak_dominance_and_not_removed() {
        // By hand: b pays 1 against x and 2 against y, a pays 1 and 1, so b
        // weakly dominates a; nothing strictly. Both (a, x) and (b, x) are
        // equilibria, since the column player is indifferent everywhere.
        let table = two(
            ["row", "column"],
            &["a", "b"],
            &["x", "y"],
            &[[1.0, 0.0], [1.0, 0.0], [1.0, 0.0], [2.0, 0.0]],
        );
        assert_eq!(iterated_dominance(&table), Vec::new());
        assert_eq!(
            weakly_dominated(&table),
            vec![WeaklyDominated {
                player: 0,
                strategy: 0,
                by: 1
            }]
        );
        assert_eq!(
            pure_equilibria(&table),
            vec![vec![0, 0], vec![1, 0], vec![1, 1]]
        );
    }

    #[test]
    fn three_players_have_pure_equilibria_and_an_even_mix() {
        // A coordination game of three: each gets 1 when all three choose
        // the same, else 0. By hand: (x, x, x) and (y, y, y) are the pure
        // equilibria; against two others who mix evenly, each choice meets
        // agreement in 1 of 4 profiles, so pays 1/4.
        let mut payoffs = Vec::new();
        for a in 0..2 {
            for b in 0..2 {
                for c in 0..2 {
                    let same = if a == b && b == c { 1.0 } else { 0.0 };
                    payoffs.push(vec![same; 3]);
                }
            }
        }
        let table = Table {
            players: vec!["a".into(), "b".into(), "c".into()],
            strategies: vec![vec!["x".into(), "y".into()]; 3],
            payoffs,
        };
        assert_eq!(pure_equilibria(&table), vec![vec![0, 0, 0], vec![1, 1, 1]]);
        assert_eq!(even_mix(&table), vec![vec![0.25, 0.25]; 3]);
        assert_eq!(iterated_dominance(&table), Vec::new());
    }
}
