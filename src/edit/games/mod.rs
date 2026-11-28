//! `add_strategy`, `delete_strategy`, `add_player` and `delete_player`
//! (and `set_habit`, in [`habit`])
//! (ADR-075, decision 2 and step 5; ADR-076, decision 5).
//!
//! A change names a game's strategy as `sealed_bids.strategies.bid_50` (every
//! player's, or the list the players share) or
//! `inspection.strategies.human.ignore` (one player's), and a player as
//! `price_war.players.shop` or a tournament's as `rivalry.players.copycat`,
//! which takes the habit it plays (`plays: tit_for_tat`). A new strategy's
//! cells, and a new player's payoffs, are 0. The text is written in the
//! file's style ([`write`]), and the model is read back: valid, with the
//! game as asked and nothing else changed.

mod cell;
mod habit;
mod write;

pub(super) use cell::{apply as set_cell, game_of};
pub(super) use habit::apply as set_habit;

use super::{check, is_name, Change, Op, Target};
use crate::games::model::Cell;
use crate::games::strategy::Plays;
use crate::games::tournament::Entrant;
use crate::games::{Game, Payoff, TableGame, Tournament};
use crate::types::ParsedModel;
use serde_json::Value as Json;
use std::collections::HashMap;

/// What a change does to a game played once, by index of player in the
/// game as it is
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Plan {
    /// A strategy for every player, or for one
    AddStrategy {
        strategy: String,
        player: Option<usize>,
    },
    /// A strategy of every player who has it, or of one
    DeleteStrategy {
        strategy: String,
        player: Option<usize>,
    },
    /// A last player, with the last player's strategies
    AddPlayer {
        player: String,
    },
    DeletePlayer {
        index: usize,
    },
}

/// What a change does to a tournament's players
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Entrants {
    Add { entrant: String, plays: Plays },
    Delete { entrant: String },
}

/// A game and what a change does to it
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Changed<'g> {
    Table(&'g TableGame, Plan),
    Tournament(&'g Tournament, Entrants),
}

impl Changed<'_> {
    /// The game as Forge should read it after the change
    fn after(&self) -> Game {
        match self {
            Self::Table(table, plan) => Game::Table(after(table, plan)),
            Self::Tournament(tournament, entrants) => {
                Game::Tournament(after_tournament(tournament, entrants))
            },
        }
    }
}

/// The game `path` names in `model` and what `change` does to it.
///
/// # Errors
///
/// Returns why, when the path does not name a game's strategy or player as
/// the change needs it, or the change would leave the game without one.
fn plan_of<'m>(
    model: &'m ParsedModel,
    path: &str,
    change: &Change,
) -> Result<(String, Changed<'m>), String> {
    let (g, rest) = path.split_once('.').unwrap_or((path, ""));
    let game = model
        .games
        .get(g)
        .ok_or_else(|| format!("{g} is not a game of the model"))?;
    let parts: Vec<&str> = rest.split('.').collect();
    let named = |name: &str| {
        if is_name(name) {
            Ok(name.to_string())
        } else {
            Err(format!(
                "{name} is not a name: a letter or _, then letters, digits and _"
            ))
        }
    };
    let plan = match (game, parts.as_slice()) {
        (Game::Table(t), ["strategies", s]) => {
            Changed::Table(t, strategy_plan(t, None, &named(s)?, change.op)?)
        },
        (Game::Table(t), ["strategies", p, s]) => {
            let index = t
                .players
                .iter()
                .position(|x| x == p)
                .ok_or_else(|| format!("{p} is not a player of {g}"))?;
            if t.symmetric {
                return Err(format!(
                    "{g} is symmetric: its players have the same strategies, so name {s} for all \
                     of them, as {g}.strategies.{s}"
                ));
            }
            Changed::Table(t, strategy_plan(t, Some(index), &named(s)?, change.op)?)
        },
        (Game::Table(t), ["players", p]) => Changed::Table(
            t,
            player_plan(t, &named(p)?, change.op, change.plays.as_deref())?,
        ),
        (Game::Tournament(t), ["players", p]) => {
            let p = named(p)?;
            let has = t.entrants.iter().any(|e| e.name == p);
            match (change.op, change.plays.as_deref()) {
                (Op::AddPlayer, _) if has => return Err(format!("{p} is already a player of {g}")),
                (Op::AddPlayer, None) => {
                    return Err(format!(
                        "a tournament's new player needs `plays`, one of {}",
                        habits()
                    ))
                },
                (Op::AddPlayer, Some(habit)) => Changed::Tournament(
                    t,
                    Entrants::Add {
                        entrant: p,
                        plays: Plays::from_name(habit).ok_or_else(|| {
                            format!("{habit} is not a habit a player plays: one of {}", habits())
                        })?,
                    },
                ),
                (_, _) if !has => return Err(format!("{p} is not a player of {g}")),
                (_, _) if t.entrants.len() < 3 => {
                    return Err(format!("{g} has two players; a tournament has two or more"))
                },
                _ => Changed::Tournament(t, Entrants::Delete { entrant: p }),
            }
        },
        (Game::Tournament(t), ["strategies", ..]) => {
            return Err(format!(
                "{g} is a tournament: its players play the strategies of its round, {}",
                t.round
            ))
        },
        _ => {
            return Err(format!(
                "it is not named as a game's strategy or player: {g}.strategies.<name>, \
                 {g}.strategies.<player>.<name> for one player's, or {g}.players.<name>"
            ))
        },
    };
    Ok((g.to_string(), plan))
}

/// The nine habits, as a model names them
pub(super) fn habits() -> String {
    Plays::ALL.map(Plays::name).join(", ")
}

/// A strategy of every player, or of `player`, added or deleted
fn strategy_plan(
    game: &TableGame,
    player: Option<usize>,
    strategy: &str,
    op: Op,
) -> Result<Plan, String> {
    let concerned = |i: usize| player.is_none_or(|p| p == i);
    let lists = || {
        game.strategies
            .iter()
            .enumerate()
            .filter(|(i, _)| concerned(*i))
    };
    let strategy = strategy.to_string();
    if op == Op::AddStrategy {
        if let Some((i, _)) = lists().find(|(_, l)| l.contains(&strategy)) {
            return Err(format!(
                "{strategy} is already a strategy of {}",
                game.players[i]
            ));
        }
        return Ok(Plan::AddStrategy { strategy, player });
    }
    if !lists().any(|(_, l)| l.contains(&strategy)) {
        return Err("it is not a strategy of the game".into());
    }
    if let Some((i, _)) = lists().find(|(_, l)| l.len() == 1 && l.contains(&strategy)) {
        return Err(format!(
            "{strategy} is {}'s only strategy; a player has one or more",
            game.players[i]
        ));
    }
    Ok(Plan::DeleteStrategy { strategy, player })
}

/// A player of a game played once added or deleted
fn player_plan(
    game: &TableGame,
    player: &str,
    op: Op,
    plays: Option<&str>,
) -> Result<Plan, String> {
    let index = game.players.iter().position(|p| p == player);
    match (op, index) {
        (_, _) if plays.is_some() => Err(
            "`plays` is a tournament player's habit; a player of a game played once has strategies"
                .into(),
        ),
        (Op::AddPlayer, Some(_)) => Err(format!("{player} is already a player of {}", game.name)),
        (Op::AddPlayer, None) => Ok(Plan::AddPlayer {
            player: player.to_string(),
        }),
        (_, None) => Err(format!("{player} is not a player of {}", game.name)),
        (_, Some(_)) if game.players.len() < 3 => Err(format!(
            "{} has two players; a game has two or more",
            game.name
        )),
        (_, Some(index)) => Ok(Plan::DeletePlayer { index }),
    }
}

/// A payoff of 0, where a change writes one
const fn zero() -> Cell {
    Cell {
        payoff: Payoff::Number(0.0),
        at: String::new(),
        others: Vec::new(),
    }
}

/// Each player's strategy in profile `index`, the last player's moving
/// fastest, as a table lists its cells
fn profile(mut index: usize, counts: &[usize]) -> Vec<usize> {
    let mut profile = vec![0; counts.len()];
    for (slot, count) in profile.iter_mut().zip(counts).rev() {
        *slot = index % count;
        index /= count;
    }
    profile
}

/// The index of a profile in the table
fn index_of(profile: &[usize], counts: &[usize]) -> usize {
    profile
        .iter()
        .zip(counts)
        .fold(0, |index, (s, count)| index * count + s)
}

/// The game played once as Forge should read it after `plan`: its lists,
/// its cells (a new one 0, the others as they were) and its dynamics
pub(super) fn after(old: &TableGame, plan: &Plan) -> TableGame {
    let mut new = old.clone();
    let n = old.players.len();
    match plan {
        Plan::AddStrategy { strategy, player } => {
            for (i, list) in new.strategies.iter_mut().enumerate() {
                if player.is_none_or(|p| p == i) {
                    list.push(strategy.clone());
                }
            }
            if let Some(dynamics) = &mut new.dynamics {
                dynamics.start.push(zero());
            }
        },
        Plan::DeleteStrategy { strategy, player } => {
            for (i, list) in new.strategies.iter_mut().enumerate() {
                if player.is_none_or(|p| p == i) {
                    list.retain(|s| s != strategy);
                }
            }
            if let Some(dynamics) = &mut new.dynamics {
                let index = old.strategies[0].iter().position(|s| s == strategy);
                dynamics.start.remove(index.unwrap_or_default());
            }
        },
        Plan::AddPlayer { player } => {
            new.players.push(player.clone());
            new.strategies.push(old.strategies[n - 1].clone());
        },
        Plan::DeletePlayer { index } => {
            new.players.remove(*index);
            new.strategies.remove(*index);
        },
    }
    let counts: Vec<usize> = new.strategies.iter().map(Vec::len).collect();
    let old_counts: Vec<usize> = old.strategies.iter().map(Vec::len).collect();
    let counted = old
        .cells
        .first()
        .and_then(|cells| cells.first())
        .is_some_and(|cell| !cell.others.is_empty());
    let total: usize = counts.iter().product();
    new.cells = (0..total)
        .map(|index| {
            let now = profile(index, &counts);
            if counted {
                // Each player's payoff is its own strategy's, whatever the counts
                return now
                    .iter()
                    .enumerate()
                    .map(|(i, &s)| {
                        let own = &new.strategies[i][s];
                        old.strategies[0]
                            .iter()
                            .position(|x| x == own)
                            .map_or_else(zero, |o| {
                                old.cells[index_of(&vec![o; n], &old_counts)][0].clone()
                            })
                    })
                    .collect();
            }
            let was = |profile: &[usize]| old.cells[index_of(profile, &old_counts)].clone();
            match plan {
                Plan::AddPlayer { .. } => {
                    let mut cells = was(&now[..n]);
                    cells.push(zero());
                    cells
                },
                Plan::DeletePlayer { index } => {
                    let mut profile = now;
                    profile.insert(*index, 0);
                    let mut cells = was(&profile);
                    cells.remove(*index);
                    cells
                },
                _ => {
                    // The strategies by name, in the old lists
                    let profile: Option<Vec<usize>> = now
                        .iter()
                        .enumerate()
                        .map(|(i, &s)| {
                            old.strategies[i]
                                .iter()
                                .position(|x| *x == new.strategies[i][s])
                        })
                        .collect();
                    profile.map_or_else(|| vec![zero(); n], |profile| was(&profile))
                },
            }
        })
        .collect();
    new
}

/// The tournament as Forge should read it after the change
fn after_tournament(old: &Tournament, entrants: &Entrants) -> Tournament {
    let mut new = old.clone();
    match entrants {
        Entrants::Add { entrant, plays } => new.entrants.push(Entrant {
            name: entrant.clone(),
            plays: *plays,
            chance: None,
        }),
        Entrants::Delete { entrant } => new.entrants.retain(|e| e.name != *entrant),
    }
    new
}

/// `json` without where each payoff is written and the counts a payoff of
/// the count form reads: they are said for messages, from the names
/// compared
pub(super) fn bare(json: &mut Json) {
    fn strip(json: &mut Json) {
        match json {
            Json::Array(items) => items.iter_mut().for_each(strip),
            Json::Object(map) => {
                map.retain(|key, _| key != "at" && key != "others");
                map.values_mut().for_each(strip);
            },
            _ => {},
        }
    }
    if let Json::Object(map) = json {
        for (key, value) in map.iter_mut() {
            if key == "games" {
                strip(value);
            } else {
                bare(value);
            }
        }
    }
}

/// `model`, or the model of the file included along `aliases`
pub(super) fn included<'m>(
    mut model: &'m mut ParsedModel,
    aliases: &[String],
) -> &'m mut ParsedModel {
    for alias in aliases {
        model = &mut model
            .resolved_includes
            .get_mut(alias)
            .expect("the model read back has the includes it had")
            .model;
    }
    model
}

/// The change, kept in `files` only when Forge reads the model back valid
/// with the game as asked and nothing else changed
pub(super) fn apply(
    files: &mut HashMap<String, String>,
    main: &str,
    before: &ParsedModel,
    target: &Target<'_>,
    change: &Change,
) -> Result<(), String> {
    let (name, game) = plan_of(target.model, &target.name, change)?;
    let text = write::write(&files[&target.file], &name, &game)
        .map_err(|why| format!("{why} ({})", target.file))?;
    confirm(files, main, before, target, &name, game.after(), text)
}

/// `text` as the target's file, kept in `files` only when Forge reads the
/// model back valid with `game` as the game `name` and nothing else changed
pub(super) fn confirm(
    files: &mut HashMap<String, String>,
    main: &str,
    before: &ParsedModel,
    target: &Target<'_>,
    name: &str,
    game: Game,
    text: String,
) -> Result<(), String> {
    let mut changed = files.clone();
    changed.insert(target.file.clone(), text);
    let after_model = check::read(&changed, main)
        .and_then(|after| check::valid(&after).map(|()| after))
        .map_err(|e| format!("Forge refuses the model with this change: {e}"))?;
    let mut want = before.clone();
    let model = included(&mut want, &target.aliases);
    model.games.insert(name.to_string(), game);
    let mut was = check::snapshot(&want);
    let mut now = check::snapshot(&after_model);
    bare(&mut was);
    bare(&mut now);
    if was != now {
        return Err("Forge reads the new text as more than this change, so it was not made".into());
    }
    *files = changed;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{check, edit_files, Change, Op};
    use super::{plan_of, write};
    use crate::games::Game;
    use std::collections::{BTreeMap, HashMap};

    /// A change to a game: `path`, with `plays` for a tournament's player
    pub(super) fn game_change(op: Op, path: &str, plays: Option<&str>) -> Change {
        Change {
            op,
            path: path.into(),
            value: None,
            formula: None,
            unit: None,
            after: None,
            new_name: None,
            plays: plays.map(str::to_string),
        }
    }

    /// Three players, each level of payoffs written as a block or in braces,
    /// lists as blocks and in brackets
    pub(super) const TRIO: &str = "_forge_version: \"5.6.0\"
inputs:
  x: 1
games:
  trio:
    description: Three players
    players:
      - ann
      - bob
      - cy
    strategies:
      ann: [up, down]
      bob: [left, right]
      cy:
        - near
        - far
    payoffs:                  # [ann, bob, cy]
      up:
        left:   # ann up, bob left
          near: [1, 2, 3]
          far: [4, 5, 6]
        right: {near: [7, 8, 9], far: [=inputs.x, 0, 0]}
      down:
        left:
          near: [0, 0, 0]
          far: [1, 1, 1]
        right: {near: [2, 2, 2], far: [3, 3, 3]}
results:
  top:
    formula: \"=trio.pure_equilibria\"
";

    /// `change` on `main` of `files`: the files it changes, or why not
    fn run(
        files: &HashMap<String, String>,
        main: &str,
        change: &Change,
    ) -> Result<BTreeMap<String, String>, String> {
        edit_files(files, main, std::slice::from_ref(change)).map_err(|e| e.to_string())
    }

    fn trio() -> HashMap<String, String> {
        HashMap::from([("model.yaml".to_string(), TRIO.to_string())])
    }

    /// `text` with each of `pairs` replaced, each once and in turn
    pub(super) fn replaced(text: &str, pairs: &[(&str, &str)]) -> String {
        pairs.iter().fold(text.to_string(), |text, (from, to)| {
            assert!(text.contains(from), "{from}");
            text.replacen(from, to, 1)
        })
    }

    /// `add` gives `want` (when given), and `back` gives the file back as it
    /// was, byte for byte
    fn there_and_back(
        files: &HashMap<String, String>,
        add: &Change,
        back: &Change,
        want: Option<&str>,
    ) {
        let main = "model.yaml";
        let there = run(files, main, add).unwrap();
        if let Some(want) = want {
            assert_eq!(there[main], want);
        }
        let mut now = files.clone();
        now.extend(there);
        assert_eq!(run(&now, main, back).unwrap()[main], files[main]);
    }

    fn add_strategy(path: &str) -> Change {
        game_change(Op::AddStrategy, path, None)
    }

    fn delete_strategy(path: &str) -> Change {
        game_change(Op::DeleteStrategy, path, None)
    }

    fn add_player(path: &str, plays: Option<&str>) -> Change {
        game_change(Op::AddPlayer, path, plays)
    }

    fn delete_player(path: &str) -> Change {
        game_change(Op::DeletePlayer, path, None)
    }

    #[test]
    fn a_strategy_is_added_at_each_of_its_players_levels_in_the_files_style() {
        // One player's: its list, and its level of the payoffs, a block
        // written as the strategy beside it is
        let want = replaced(
            TRIO,
            &[
                ("bob: [left, right]", "bob: [left, right, middle]"),
                (
                    "        right: {near: [7, 8, 9], far: [=inputs.x, 0, 0]}\n",
                    "        right: {near: [7, 8, 9], far: [=inputs.x, 0, 0]}\n        middle:\n          near: [0, 0, 0]\n          far: [0, 0, 0]\n",
                ),
                (
                    "        right: {near: [2, 2, 2], far: [3, 3, 3]}\n",
                    "        right: {near: [2, 2, 2], far: [3, 3, 3]}\n        middle:\n          near: [0, 0, 0]\n          far: [0, 0, 0]\n",
                ),
            ],
        );
        there_and_back(
            &trio(),
            &add_strategy("trio.strategies.bob.middle"),
            &delete_strategy("trio.strategies.bob.middle"),
            Some(&want),
        );
        // The last player's: a line of its block list, and a key in each cell's
        // mapping, in braces or on a line
        let want = replaced(
            TRIO,
            &[
                ("        - far\n", "        - far\n        - away\n"),
                (
                    "          far: [4, 5, 6]\n",
                    "          far: [4, 5, 6]\n          away: [0, 0, 0]\n",
                ),
                (
                    "far: [=inputs.x, 0, 0]}",
                    "far: [=inputs.x, 0, 0], away: [0, 0, 0]}",
                ),
                (
                    "          far: [1, 1, 1]\n",
                    "          far: [1, 1, 1]\n          away: [0, 0, 0]\n",
                ),
                ("far: [3, 3, 3]}", "far: [3, 3, 3], away: [0, 0, 0]}"),
            ],
        );
        there_and_back(
            &trio(),
            &add_strategy("trio.strategies.cy.away"),
            &delete_strategy("trio.strategies.cy.away"),
            Some(&want),
        );
        // Every player's, at every level
        there_and_back(
            &trio(),
            &add_strategy("trio.strategies.wait"),
            &delete_strategy("trio.strategies.wait"),
            None,
        );
    }

    #[test]
    fn a_strategy_is_deleted_with_its_cells_first_last_or_between() {
        let want = replaced(
            TRIO,
            &[
                ("        - near\n        - far\n", "        - far\n"),
                ("          near: [1, 2, 3]\n", ""),
                ("{near: [7, 8, 9], far:", "{far:"),
                ("          near: [0, 0, 0]\n", ""),
                ("{near: [2, 2, 2], far:", "{far:"),
            ],
        );
        assert_eq!(
            run(
                &trio(),
                "model.yaml",
                &delete_strategy("trio.strategies.near")
            )
            .unwrap()["model.yaml"],
            want
        );
        let want = replaced(
            TRIO,
            &[
                ("ann: [up, down]", "ann: [down]"),
                ("      up:\n        left:   # ann up, bob left\n          near: [1, 2, 3]\n          far: [4, 5, 6]\n        right: {near: [7, 8, 9], far: [=inputs.x, 0, 0]}\n", ""),
            ],
        );
        assert_eq!(
            run(
                &trio(),
                "model.yaml",
                &delete_strategy("trio.strategies.ann.up")
            )
            .unwrap()["model.yaml"],
            want
        );
    }

    #[test]
    fn a_player_is_deleted_its_first_strategys_payoffs_kept() {
        let at = |path: &str| {
            run(&trio(), "model.yaml", &delete_player(path)).unwrap()["model.yaml"].clone()
        };
        let payoffs = |text: &str| {
            text[text.find("    payoffs:").unwrap()..text.find("results:").unwrap()].to_string()
        };
        // The first: the payoffs of its first strategy, moved left
        let ann = at("trio.players.ann");
        assert!(
            ann.contains("    players:\n      - bob\n      - cy\n"),
            "{ann}"
        );
        assert!(
            ann.contains("    strategies:\n      bob: [left, right]\n"),
            "{ann}"
        );
        assert_eq!(
            payoffs(&ann),
            "    payoffs:                  # [ann, bob, cy]
      left:   # ann up, bob left
        near: [2, 3]
        far: [5, 6]
      right: {near: [8, 9], far: [0, 0]}
"
        );
        // One between: each of its levels becomes its first strategy's
        assert_eq!(
            payoffs(&at("trio.players.bob")),
            "    payoffs:                  # [ann, bob, cy]
      up:
        near: [1, 3]
        far: [4, 6]
      down:
        near: [0, 0]
        far: [1, 1]
"
        );
        // The last: each cell, after its key, the key's comment kept
        let cy = at("trio.players.cy");
        assert!(
            cy.contains("      bob: [left, right]\n    payoffs:"),
            "{cy}"
        );
        assert_eq!(
            payoffs(&cy),
            "    payoffs:                  # [ann, bob, cy]
      up:
        left: [1, 2]   # ann up, bob left
        right: [7, 8]
      down:
        left: [0, 0]
        right: [2, 2]
"
        );
    }

    #[test]
    fn a_player_is_added_last_with_the_last_players_strategies_and_back() {
        let want = replaced(
            TRIO,
            &[
                ("      - cy\n", "      - cy\n      - dee\n"),
                (
                    "        - far\n",
                    "        - far\n      dee:\n        - near\n        - far\n",
                ),
                (
                    "near: [1, 2, 3]",
                    "near: {near: [1, 2, 3, 0], far: [1, 2, 3, 0]}",
                ),
            ],
        );
        let there = run(&trio(), "model.yaml", &add_player("trio.players.dee", None)).unwrap();
        assert!(there["model.yaml"].starts_with(&want[..want.find("far: [4, 5, 6]").unwrap()]));
        there_and_back(
            &trio(),
            &add_player("trio.players.dee", None),
            &delete_player("trio.players.dee"),
            None,
        );
        // A game of two in braces, its players' lists in braces
        let files = HashMap::from([(
            "model.yaml".to_string(),
            "_forge_version: \"5.6.0\"
games:
  pair:
    players: [a, b]
    strategies: {a: [x, y], b: [x, y]}
    payoffs: {x: {x: [1, 1], y: [0, 0]}, y: {x: [0, 0], y: [2, 2]}}
"
            .to_string(),
        )]);
        let want = "_forge_version: \"5.6.0\"
games:
  pair:
    players: [a, b, c]
    strategies: {a: [x, y], b: [x, y], c: [x, y]}
    payoffs: {x: {x: {x: [1, 1, 0], y: [1, 1, 0]}, y: {x: [0, 0, 0], y: [0, 0, 0]}}, y: {x: {x: [0, 0, 0], y: [0, 0, 0]}, y: {x: [2, 2, 0], y: [2, 2, 0]}}}
";
        there_and_back(
            &files,
            &add_player("pair.players.c", None),
            &delete_player("pair.players.c"),
            Some(want),
        );
        // The first player of three in braces
        let three = HashMap::from([("model.yaml".to_string(), want.to_string())]);
        let first = run(&three, "model.yaml", &delete_player("pair.players.a")).unwrap();
        assert_eq!(
            first["model.yaml"],
            "_forge_version: \"5.6.0\"
games:
  pair:
    players: [b, c]
    strategies: {b: [x, y], c: [x, y]}
    payoffs: {x: {x: [1, 0], y: [1, 0]}, y: {x: [0, 0], y: [0, 0]}}
"
        );
    }

    /// A symmetric game of many in the count form, its start on lines
    const CROWD: &str = "_forge_version: \"5.6.0\"
games:
  crowd:
    players: [p1, p2, p3]
    strategies: [go, stay, wait]
    symmetric: true
    payoffs: {go: =10 - 3 * crowd.others.go, stay: 5, wait: 4}
    dynamics:
      start:
        go: 0
        stay: 0.5
        wait: 0.5
      generations: 10
";

    #[test]
    fn the_count_form_gets_a_payoff_and_its_players_alone_change() {
        let files = HashMap::from([("model.yaml".to_string(), CROWD.to_string())]);
        let want = replaced(
            CROWD,
            &[
                ("[go, stay, wait]", "[go, stay, wait, run]"),
                ("wait: 4}", "wait: 4, run: 0}"),
                ("        wait: 0.5\n", "        wait: 0.5\n        run: 0\n"),
            ],
        );
        there_and_back(
            &files,
            &add_strategy("crowd.strategies.run"),
            &delete_strategy("crowd.strategies.run"),
            Some(&want),
        );
        // The first in braces, and its share on its line
        let want = replaced(
            CROWD,
            &[
                ("[go, stay, wait]", "[stay, wait]"),
                ("{go: =10 - 3 * crowd.others.go, stay: 5", "{stay: 5"),
                ("        go: 0\n", ""),
            ],
        );
        assert_eq!(
            run(
                &files,
                "model.yaml",
                &delete_strategy("crowd.strategies.go")
            )
            .unwrap()["model.yaml"],
            want
        );
        let want = CROWD.replace("[p1, p2, p3]", "[p1, p2, p3, p4]");
        there_and_back(
            &files,
            &add_player("crowd.players.p4", None),
            &delete_player("crowd.players.p4"),
            Some(&want),
        );
        assert_eq!(
            run(&files, "model.yaml", &delete_player("crowd.players.p1")).unwrap()["model.yaml"],
            CROWD.replace("[p1, p2, p3]", "[p2, p3]")
        );
    }

    #[test]
    fn a_tournaments_player_is_added_with_its_habit_and_deleted() {
        let text = "_forge_version: \"5.6.0\"
games:
  pd:
    players: [me, you]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs:
      cooperate: {cooperate: 3, defect: 0}
      defect: {cooperate: 5, defect: 1}
  duel:
    tournament: {round: pd, players: {a: tit_for_tat, b: always_defect}, rounds: 10}
";
        let files = HashMap::from([("model.yaml".to_string(), text.to_string())]);
        let want = text.replace("b: always_defect}", "b: always_defect, c: random}");
        there_and_back(
            &files,
            &add_player("duel.players.c", Some("random")),
            &delete_player("duel.players.c"),
            Some(&want),
        );
        let mut three = files.clone();
        three.insert("model.yaml".into(), want);
        assert_eq!(
            run(&three, "model.yaml", &delete_player("duel.players.a")).unwrap()["model.yaml"],
            text.replace(
                "{a: tit_for_tat, b: always_defect}",
                "{b: always_defect, c: random}"
            )
        );
        let refused = |change: Change, why: &str| {
            let message = run(&files, "model.yaml", &change).unwrap_err();
            assert!(message.contains(why), "{message}");
        };
        refused(
            add_player("duel.players.a", Some("random")),
            "a is already a player of duel",
        );
        refused(
            add_player("duel.players.c", None),
            "a tournament's new player needs `plays`, one of always_cooperate, always_defect, \
             tit_for_tat, suspicious_tit_for_tat, tit_for_two_tats, grim_trigger, \
             win_stay_lose_shift, generous_tit_for_tat, random",
        );
        refused(
            add_player("duel.players.c", Some("sulk")),
            "sulk is not a habit a player plays: one of always_cooperate",
        );
        refused(delete_player("duel.players.z"), "z is not a player of duel");
        refused(
            delete_player("duel.players.a"),
            "duel has two players; a tournament has two or more",
        );
        refused(
            add_strategy("duel.strategies.x"),
            "duel is a tournament: its players play the strategies of its round, pd",
        );
    }

    #[test]
    fn a_file_without_a_last_line_break_gets_one_before_a_new_line_and_back() {
        let text = "_forge_version: \"5.6.0\"
games:
  g:
    strategies: [x, y]
    payoffs:
      x: {x: [1, 1], y: [0, 0]}
      y: {x: [0, 0], y: [1, 1]}
    players:
      - a
      - b";
        let files = HashMap::from([("model.yaml".to_string(), text.to_string())]);
        there_and_back(
            &files,
            &add_player("g.players.c", None),
            &delete_player("g.players.c"),
            None,
        );
        let text = "_forge_version: \"5.6.0\"
games:
  g:
    players: [a, b]
    strategies: [x, y]
    symmetric: true
    payoffs:
      x: {x: 1, y: 0}
      y: {x: 0, y: 1}";
        let files = HashMap::from([("model.yaml".to_string(), text.to_string())]);
        let want = format!(
            "{}\n      z: {{x: 0, y: 0, z: 0}}",
            text.replace("[x, y]", "[x, y, z]")
                .replace("y: 0}", "y: 0, z: 0}")
                .replace("y: 1}", "y: 1, z: 0}")
        );
        there_and_back(
            &files,
            &add_strategy("g.strategies.z"),
            &delete_strategy("g.strategies.z"),
            Some(&want),
        );
    }

    #[test]
    fn a_game_in_an_included_file_is_changed_there() {
        let lib = "_forge_version: \"5.6.0\"
games:
  pd:
    players: [me, you]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs:
      cooperate: {cooperate: 3, defect: 0}
      defect: {cooperate: 5, defect: 1}
";
        let main = "_forge_version: \"5.6.0\"
_includes:
  - file: lib.yaml
    as: lib
x: 1
";
        let files = HashMap::from([
            ("model.yaml".to_string(), main.to_string()),
            ("lib.yaml".to_string(), lib.to_string()),
        ]);
        let there = run(
            &files,
            "model.yaml",
            &add_strategy("@lib.pd.strategies.wait"),
        )
        .unwrap();
        assert_eq!(there.keys().collect::<Vec<_>>(), ["lib.yaml"]);
        assert_eq!(
            there["lib.yaml"],
            lib.replace("[cooperate, defect]", "[cooperate, defect, wait]")
                .replace("defect: 0}", "defect: 0, wait: 0}")
                .replace("defect: 1}", "defect: 1, wait: 0}")
                + "      wait: {cooperate: 0, defect: 0, wait: 0}\n"
        );
    }

    /// A change, the model it is made on, and the reason it is refused:
    /// `trio`, `one` (trio with ann's one strategy), or an example of
    /// examples/games
    const REFUSED: [(&str, Op, &str, Option<&str>, &str); 21] = [
        (
            "trio",
            Op::AddStrategy,
            "nothing.strategies.x",
            None,
            "nothing is not a game of the model",
        ),
        (
            "trio",
            Op::AddStrategy,
            "trio.strategies.1x",
            None,
            "1x is not a name: a letter or _",
        ),
        (
            "trio",
            Op::AddStrategy,
            "trio.strategies.bob.1x",
            None,
            "1x is not a name",
        ),
        (
            "trio",
            Op::AddPlayer,
            "trio.players.1x",
            None,
            "1x is not a name",
        ),
        (
            "trio",
            Op::AddStrategy,
            "trio.strategies.nobody.x",
            None,
            "nobody is not a player of trio",
        ),
        (
            "trio",
            Op::AddStrategy,
            "trio.strategies.up",
            None,
            "up is already a strategy of ann",
        ),
        (
            "trio",
            Op::AddStrategy,
            "trio.strategies.bob.left",
            None,
            "left is already a strategy of bob",
        ),
        (
            "trio",
            Op::DeleteStrategy,
            "trio.strategies.nothing",
            None,
            "it is not a strategy of the game",
        ),
        (
            "trio",
            Op::DeleteStrategy,
            "trio.strategies.ann.near",
            None,
            "it is not a strategy of the game",
        ),
        (
            "trio",
            Op::AddStrategy,
            "trio.strategies.x",
            Some("random"),
            "add_strategy does not take `plays`",
        ),
        (
            "trio",
            Op::AddPlayer,
            "trio.players.dee",
            Some("random"),
            "`plays` is a tournament player's habit",
        ),
        (
            "trio",
            Op::AddPlayer,
            "trio.players.ann",
            None,
            "ann is already a player of trio",
        ),
        (
            "trio",
            Op::DeletePlayer,
            "trio.players.nobody",
            None,
            "nobody is not a player of trio",
        ),
        (
            "trio",
            Op::AddStrategy,
            "trio.nothing",
            None,
            "it is not named as a game's strategy or player: trio.strategies.<name>, \
             trio.strategies.<player>.<name> for one player's, or trio.players.<name>",
        ),
        (
            "one",
            Op::DeleteStrategy,
            "trio.strategies.up",
            None,
            "up is ann's only strategy; a player has one or more",
        ),
        (
            "battle-of-the-sexes.yaml",
            Op::DeletePlayer,
            "evening.players.alex",
            None,
            "evening has two players; a game has two or more",
        ),
        (
            "battle-of-the-sexes.yaml",
            Op::AddStrategy,
            "evening.strategies.alex.ballet",
            None,
            "the players of evening share one list of strategies: name ballet for all of them, \
             as evening.strategies.ballet, not alex's alone",
        ),
        (
            "battle-of-the-sexes.yaml",
            Op::DeleteStrategy,
            "evening.strategies.sam.opera",
            None,
            "not sam's alone",
        ),
        (
            "chicken.yaml",
            Op::AddStrategy,
            "chicken.strategies.me.wait",
            None,
            "chicken is symmetric: its players have the same strategies, so name wait for all of \
             them, as chicken.strategies.wait",
        ),
        (
            "axelrod.yaml",
            Op::AddPlayer,
            "axelrod.players.1x",
            Some("random"),
            "1x is not a name",
        ),
        (
            "noisy-tournament.yaml",
            Op::DeletePlayer,
            "noisy.players.ac",
            None,
            "games.noisy.tournament.players is not written",
        ),
    ];

    #[test]
    fn a_change_that_cannot_be_made_is_refused_with_the_reason() {
        let one = TRIO.replace("ann: [up, down]", "ann: [up]").replace(
            "      down:\n        left:\n          near: [0, 0, 0]\n          far: [1, 1, 1]\n        right: {near: [2, 2, 2], far: [3, 3, 3]}\n",
            "",
        );
        for (on, op, path, plays, why) in REFUSED {
            let text = match on {
                "trio" => TRIO.to_string(),
                "one" => one.clone(),
                example => std::fs::read_to_string(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("examples/games")
                        .join(example),
                )
                .unwrap(),
            };
            let files = HashMap::from([("model.yaml".to_string(), text)]);
            let message = run(&files, "model.yaml", &game_change(op, path, plays)).unwrap_err();
            assert!(message.contains(why), "{path}: {message}");
            assert!(message.ends_with(". No file was changed"), "{message}");
        }
    }

    /// The text of every game of every example, with a strategy and a
    /// player added and deleted again: the file as it was, byte for byte,
    /// whatever Forge then makes of the game
    fn text_there_and_back(
        files: &HashMap<String, String>,
        main: &str,
        add: &Change,
        back: &Change,
    ) -> Result<String, String> {
        let model = check::read(files, main).unwrap();
        let (name, changed) = plan_of(&model, &add.path, add).unwrap();
        let text = write::write(&files[main], &name, &changed)?;
        let mut after = model.clone();
        after.games.insert(name.clone(), changed.after());
        let (_, back) = plan_of(&after, &back.path, back).unwrap();
        Ok(write::write(&text, &name, &back).unwrap())
    }

    /// Every game of every example gets a strategy and a player added and
    /// deleted again: the text back byte for byte; Forge keeps those the
    /// model allows, and says why for the others
    #[test]
    fn every_game_of_every_example_gets_a_strategy_and_a_player_and_back() {
        let (mut kept, mut text_back) = (0, 0);
        let mut refused: Vec<String> = Vec::new();
        let mut text_refused: Vec<String> = Vec::new();
        for (dir, files) in super::super::tests::example_sets() {
            let mut mains: Vec<&String> = files.keys().filter(|k| !k.contains('/')).collect();
            mains.sort();
            for main in mains {
                let model = check::read(&files, main).unwrap();
                for (g, game) in &model.games {
                    let player = format!("{g}.players.edit_added");
                    let strategy = format!("{g}.strategies.edit_added");
                    let pairs = match game {
                        Game::Table(_) => vec![
                            (add_strategy(&strategy), delete_strategy(&strategy)),
                            (add_player(&player, None), delete_player(&player)),
                        ],
                        Game::Tournament(_) => vec![(
                            add_player(&player, Some("tit_for_tat")),
                            delete_player(&player),
                        )],
                    };
                    for (add, back) in &pairs {
                        let place = format!("{}/{main} {}", dir.display(), add.path);
                        match text_there_and_back(&files, main, add, back) {
                            Ok(again) if again == files[main] => text_back += 1,
                            other => text_refused.push(format!("{place}: {other:?}")),
                        }
                        let there = run(&files, main, add);
                        let back = there.and_then(|there| {
                            let mut now = files.clone();
                            now.extend(there);
                            run(&now, main, back)
                        });
                        match back {
                            Ok(back) if back.iter().all(|(name, text)| files[name] == *text) => {
                                kept += 1;
                            },
                            other => refused.push(format!("{place}: {other:?}")),
                        }
                    }
                }
            }
        }
        // Only the tournament whose players are another's (`players: *roster`)
        // is not written where edit can change it
        assert_eq!(text_refused.len(), 1, "{text_refused:#?}");
        assert!(
            text_refused[0].contains(
                "noisy.players.edit_added: Err(\"games.noisy.tournament.players is not written"
            ),
            "{text_refused:#?}"
        );
        assert_eq!((text_back, kept), (34, 18), "{refused:#?}");
        // What Forge refuses, and why: a tournament's round has two
        // strategies; a symmetric game of three is written in the count form;
        // the roster two tournaments share; formulas that read an equilibrium
        // a new strategy or player takes away
        let why = |text: &str| refused.iter().filter(|r| r.contains(text)).count();
        assert_eq!(
            why("which a tournament cannot play: a round is a symmetric game of two strategies"),
            4,
            "{refused:#?}"
        );
        assert_eq!(
            why("a symmetric game of 3 players is written with one payoff for each strategy"),
            9,
            "{refused:#?}"
        );
        assert_eq!(why("noisy-tournament.yaml clean.players.edit_added: Err(\"Validation error: add_player clean.players.edit_added: Forge reads the new text as more than this change"), 1);
        assert_eq!(why("noisy-tournament.yaml noisy.players.edit_added: Err(\"Validation error: add_player noisy.players.edit_added: games.noisy.tournament.players is not written"), 1);
        assert_eq!(why("'vd.equilibrium_6"), 2, "{refused:#?}");
        assert_eq!(refused.len(), 17, "{refused:#?}");
    }
}
