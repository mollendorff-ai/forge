//! The count form of a symmetric game (ADR-073, question 1).
//!
//! A symmetric game of many players is written as one payoff for each of a
//! player's own strategies, a number or a formula over how many of the other
//! players play each strategy (`pg.others.contribute`). Forge expands it into
//! the whole table here, so every part that reads a game reads a table, as
//! for one written out.

use super::model::{check_keys, describe, error, number_or_formula, Cell, TableGame};
use crate::error::{ForgeError, ForgeResult};
use yaml_serde::{Mapping, Value};

/// `payoffs`, if it is written in the count form: a mapping in which a
/// strategy has a payoff, not a mapping of the next player's strategies.
pub(super) fn count_form(payoffs: &Value) -> Option<&Mapping> {
    match payoffs {
        Value::Mapping(map) if map.values().any(|v| !matches!(v, Value::Mapping(_))) => Some(map),
        _ => None,
    }
}

/// The refusal of a symmetric game of three or more players written out as
/// a table: it shows the count form.
pub(super) fn not_written_out(game: &TableGame) -> ForgeError {
    let strategies = &game.strategies[0];
    let example = strategies
        .iter()
        .map(|s| format!("{s}: =..."))
        .collect::<Vec<_>>()
        .join(", ");
    error(format!(
        "games.{name}.payoffs: a symmetric game of {n} players is written with one payoff for \
         each strategy, a number or a formula over how many of the other players play each \
         ({name}.others.{first}): payoffs: {{{example}}}",
        name = game.name,
        n = game.players.len(),
        first = strategies[0],
    ))
}

/// Every cell of a game in the count form, in table order, one payoff per
/// player: the payoff of the player's own strategy, with the counts of the
/// other players' strategies in that cell.
pub(super) fn read(game: &TableGame, map: &Mapping) -> ForgeResult<Vec<Vec<Cell>>> {
    let path = format!("games.{}.payoffs", game.name);
    let strategies = &game.strategies[0];
    check_keys(map, strategies, &game.players[0], &path)?;
    let written = strategies
        .iter()
        .map(|strategy| {
            let value = &map[strategy.as_str()];
            number_or_formula(value).ok_or_else(|| {
                error(format!(
                    "'{path}.{strategy}' is {}: in the count form a strategy has one payoff, \
                     a number or a formula starting with =",
                    describe(value)
                ))
            })
        })
        .collect::<ForgeResult<Vec<_>>>()?;

    let k = strategies.len();
    let n = game.players.len();
    let cells = (0..n).fold(1, |cells, _| cells * k);
    let mut table = Vec::with_capacity(cells);
    for index in 0..cells {
        // The strategy of each player in this cell, the last moving fastest
        let mut profile = vec![0; n];
        let mut rest = index;
        for slot in profile.iter_mut().rev() {
            *slot = rest % k;
            rest /= k;
        }
        let mut row = Vec::with_capacity(n);
        for (player, &own) in profile.iter().enumerate() {
            let mut count = vec![0_u32; k];
            for (other, &s) in profile.iter().enumerate() {
                if other != player {
                    count[s] += 1;
                }
            }
            let said = strategies
                .iter()
                .zip(&count)
                .map(|(s, c)| format!("{c} {s}"))
                .collect::<Vec<_>>()
                .join(", ");
            row.push(Cell {
                payoff: written[own].clone(),
                at: format!(
                    "{}'s payoff in {path}.{}, with the others playing {said}",
                    game.players[player], strategies[own]
                ),
                others: strategies
                    .iter()
                    .zip(&count)
                    .map(|(s, &c)| (format!("{}.others.{s}", game.name), f64::from(c)))
                    .collect(),
            });
        }
        table.push(row);
    }
    Ok(table)
}

#[cfg(test)]
mod tests {
    use crate::games::model::{parse_games, Payoff};
    use crate::games::TableGame;

    fn game(yaml: &str) -> Result<TableGame, String> {
        parse_games(&yaml_serde::from_str(yaml).unwrap())
            .map(|games| games[0].played_once().unwrap().clone())
            .map_err(|e| e.to_string())
    }

    const PUBLIC_GOODS: &str = "
pg:
  players: [a, b, c, d, e]
  strategies: [contribute, keep]
  symmetric: true
  payoffs:
    contribute: =r * 10 * (pg.others.contribute + 1) / 5
    keep: =10 + r * 10 * pg.others.contribute / 5
";

    /// The public goods game's payoffs, by hand: contributing pays
    /// r e (k + 1) / n and keeping e + r e k / n, with k of the others
    /// contributing (e = 10, n = 5)
    fn by_hand(r: f64, contributes: bool, k: u32) -> f64 {
        let k = f64::from(k);
        if contributes {
            r * 10.0 * (k + 1.0) / 5.0
        } else {
            10.0 + r * 10.0 * k / 5.0
        }
    }

    /// The count form gives the 32-cell table written out, cell by cell:
    /// each player's payoff is its own strategy's, with the counts of the
    /// other four in that cell
    #[test]
    fn the_count_form_is_the_table_written_out() {
        let pg = game(PUBLIC_GOODS).unwrap();
        assert_eq!(pg.cells.len(), 32);
        let r = 1.6;
        let table = pg
            .table(|formula, _, others| {
                let k = others[0].1;
                assert_eq!(others[0].0, "pg.others.contribute");
                assert_eq!(others[1].0, "pg.others.keep");
                assert!((others[0].1 + others[1].1 - 4.0).abs() < f64::EPSILON);
                Ok::<f64, String>(if formula.starts_with("=r * 10 * (") {
                    r * 10.0 * (k + 1.0) / 5.0
                } else {
                    10.0 + r * 10.0 * k / 5.0
                })
            })
            .unwrap();
        for (index, row) in table.payoffs.iter().enumerate() {
            // Cell index in binary, a first: 0 is contribute, 1 is keep
            let plays: Vec<bool> = (0..5).map(|p| (index >> (4 - p)) & 1 == 0).collect();
            let contributing = u32::try_from(plays.iter().filter(|&&c| c).count()).unwrap();
            for (player, &contributes) in plays.iter().enumerate() {
                let k = contributing - u32::from(contributes);
                assert!(
                    (row[player] - by_hand(r, contributes, k)).abs() < 1e-12,
                    "cell {index}, player {player}: {} against {}",
                    row[player],
                    by_hand(r, contributes, k)
                );
            }
        }
        assert_eq!(
            pg.cells[1][4].at,
            "e's payoff in games.pg.payoffs.keep, with the others playing 4 contribute, 0 keep"
        );
    }

    /// A formula is worked out once for each own strategy and each split of
    /// the others: 2 x 5 = 10 times for five players, not 160
    #[test]
    fn each_formula_is_worked_out_once_for_each_count() {
        let pg = game(PUBLIC_GOODS).unwrap();
        let mut calls = 0;
        pg.table(|_, _, _| {
            calls += 1;
            Ok::<f64, String>(1.0)
        })
        .unwrap();
        assert_eq!(calls, 10);
    }

    /// Two players can use the count form too, and a number is a payoff
    #[test]
    fn two_players_and_a_number() {
        let pg = game(
            "d:\n  players: [me, you]\n  strategies: [c, d]\n  symmetric: true\n  payoffs:\n    \
             c: 3\n    d: =1 + 4 * d.others.c\n",
        )
        .unwrap();
        assert_eq!(pg.cells.len(), 4);
        assert_eq!(pg.cells[0][0].payoff, Payoff::Number(3.0));
        assert_eq!(pg.cells[2][0].others[0], ("d.others.c".into(), 1.0));
        assert_eq!(pg.cells[2][1].others[0], ("d.others.c".into(), 0.0));
    }

    /// What cannot be read is named: a symmetric game of three players
    /// written out (the message shows the count form), a payoff that is a
    /// list, a strategy with no payoff or one that is not a strategy, and a
    /// strategy named others
    #[test]
    fn what_cannot_be_read_is_named() {
        let three =
            "g:\n  players: [a, b, c]\n  strategies: [x, y]\n  symmetric: true\n  payoffs:\n";
        for (payoffs, message) in [
            (
                "    x: {x: {x: 1, y: 1}, y: {x: 1, y: 1}}\n    y: {x: {x: 1, y: 1}, y: {x: 1, y: 1}}\n",
                "games.g.payoffs: a symmetric game of 3 players is written with one payoff for \
                 each strategy, a number or a formula over how many of the other players play \
                 each (g.others.x): payoffs: {x: =..., y: =...}",
            ),
            (
                "    x: [1, 2]\n    y: 1\n",
                "'games.g.payoffs.x' is a list: in the count form a strategy has one payoff",
            ),
            (
                "    x: 1\n    y: {x: 1}\n",
                "'games.g.payoffs.y' is a mapping: in the count form a strategy has one payoff",
            ),
            ("    x: 1\n", "'games.g.payoffs' has no entry for a's y"),
            (
                "    x: 1\n    y: 2\n    z: 3\n",
                "'games.g.payoffs' has \"z\", which is not a strategy of a (x, y)",
            ),
            ("    x: done\n    y: 1\n", "'games.g.payoffs.x' is \"done\""),
        ] {
            let err = game(&format!("{three}{payoffs}")).unwrap_err();
            assert!(err.contains(message), "{payoffs}: {err}");
        }
        let err = game(
            "g:\n  players: [a, b]\n  strategies: [others, y]\n  symmetric: true\n  payoffs:\n    \
             others: 1\n    y: 1\n",
        )
        .unwrap_err();
        assert!(
            err.contains(
                "a has a strategy named others, a name a payoff reads (<game>.others.<strategy>)"
            ),
            "{err}"
        );
        let err = game(
            "g:\n  players: [a, b, c]\n  strategies: {a: [x, y], b: [x, y], c: [x]}\n  \
             symmetric: true\n  payoffs: {x: 1, y: 1}\n",
        )
        .unwrap_err();
        assert!(
            err.contains("is symmetric, which needs every player to have the same strategies"),
            "{err}"
        );
    }
}
