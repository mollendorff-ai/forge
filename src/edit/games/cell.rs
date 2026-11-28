//! `set_value` and `set_formula` on a number a game writes (ADR-075, ADR-076
//! decision 5): a payoff cell, a tournament's length, noise or player's
//! chance, or a game's dynamics.
//!
//! A cell is named by its game and the strategies that lead to it:
//! `stag_hunt.payoffs.hare.stag`. When the cell is a list, one payoff for
//! each player, the player ends the name: `inspection.payoffs.cheat.audit.human`.
//! A game of players alike (the count form) is named by its strategy:
//! `public_goods.payoffs.keep`. The payoff is written in the file's own
//! style, in place, and the model is read back: valid, with exactly the cells
//! that are written there changed (a symmetric game's second player reads
//! the first's). A tournament's numbers are `rivalry.tournament.noise`,
//! `rivalry.tournament.rounds` (or `continuation`) and
//! `axelrod.tournament.players.gen.forgive`; the dynamics', `price_war.dynamics.generations`,
//! `.baseline` and `.start.hold`.

use super::write::{key_indent, push_entry};
use super::{bare, included};
use crate::edit::text::{encode_formula, parse, splice, Style};
use crate::edit::{check, formula, value_text, Change, Op, Target};
use crate::games::model::Cell;
use crate::games::tournament::Ends;
use crate::games::{Game, Payoff};
use crate::types::ParsedModel;
use std::collections::HashMap;
use yaml_edit::{Mapping, Scalar, YamlFile, YamlNode};

/// The game that `name` points into by one of the places it writes a number:
/// `game.payoffs...`, `game.tournament...` or `game.dynamics...`
pub(in crate::edit) fn game_of<'m>(model: &'m ParsedModel, name: &str) -> Option<&'m Game> {
    let mut parts = name.split('.');
    let game = parts.next().and_then(|g| model.games.get(g));
    game.filter(|_| {
        parts
            .next()
            .is_some_and(|p| ["payoffs", "tournament", "dynamics"].contains(&p))
    })
}

/// The payoff a change gives, and its text before it takes the file's quotes
fn given(change: &Change) -> Result<(Payoff, String), String> {
    if change.op == Op::SetFormula {
        let formula = formula(change.formula.as_deref())?;
        return Ok((Payoff::Formula(formula.trim().to_string()), formula.into()));
    }
    let text = value_text(change.value.as_ref())?;
    let number = text
        .parse::<f64>()
        .map_err(|_| format!("{text} is not a number written in decimal"))?;
    Ok((Payoff::Number(number), text))
}

/// Where a number is: written as a scalar (whether it is in a flow
/// collection, and the player it is for in a list of payoffs), or not
/// written, in the mapping it would be added to
enum Place<'n> {
    Written(Scalar, bool, Option<&'n str>),
    Missing(Mapping),
}

/// The keys a tournament's noise and a game's dynamics have by default, 0
const DEFAULTS: [(&str, &str); 2] = [("tournament", "noise"), ("dynamics", "baseline")];

/// Where the number at `names` is, in the game's mapping
fn find<'n>(
    file: &YamlFile,
    game: &str,
    players: &[String],
    names: &[&'n str],
) -> Result<Place<'n>, String> {
    let path = format!("games.{game}.{}", names[0]);
    let not_written =
        || format!("{path} is not written where edit can change it (an alias, or another shape)");
    let mut node = file
        .document()
        .and_then(|d| d.as_mapping())
        .and_then(|m| m.get_mapping("games"))
        .and_then(|g| g.get_mapping(game))
        .and_then(|g| g.get(names[0]))
        .ok_or_else(not_written)?;
    if let (YamlNode::Mapping(section), [section_name, key]) = (&node, names) {
        if DEFAULTS.contains(&(*section_name, *key)) && section.get(key).is_none() {
            return Ok(Place::Missing(section.clone()));
        }
    }
    let mut names = names[1..].iter();
    let mut flow = false;
    let mut listed = None;
    loop {
        node = match node {
            YamlNode::Scalar(scalar) => {
                return names
                    .next()
                    .map_or(Ok(Place::Written(scalar, flow, listed)), |extra| {
                        Err(format!(
                            "{extra} is beyond the number, which is one value: name the keys that \
                         lead to it"
                        ))
                    });
            },
            YamlNode::Mapping(map) => {
                let key = names
                    .next()
                    .ok_or("it is a group of values: name each key that leads to the number")?;
                flow = map.is_flow_style();
                map.get(key)
                    .ok_or_else(|| format!("{key} is not written under {path}"))?
            },
            YamlNode::Sequence(list) => {
                let player = names
                    .next()
                    .ok_or("it is a list of payoffs: end its name with the player")?;
                let index = players
                    .iter()
                    .position(|p| p == player)
                    .ok_or_else(|| format!("{player} is not a player of {game}"))?;
                flow = list.is_flow_style();
                listed = Some(*player);
                list.values()
                    .nth(index)
                    .expect("a list of payoffs has one for each player")
            },
            _ => return Err(not_written()),
        };
    }
}

/// Every number the game writes: its payoffs and dynamics, or a tournament's
/// length, noise and players' chances
fn cells_of(game: &mut Game) -> Vec<&mut Cell> {
    match game {
        Game::Table(table) => {
            let dynamics = table.dynamics.iter_mut().flat_map(|d| {
                d.start
                    .iter_mut()
                    .chain([&mut d.generations, &mut d.baseline])
            });
            table.cells.iter_mut().flatten().chain(dynamics).collect()
        },
        Game::Tournament(tournament) => {
            let (Ends::Rounds(length) | Ends::Continuation(length)) = &mut tournament.ends;
            let chances = tournament
                .entrants
                .iter_mut()
                .filter_map(|e| e.chance.as_mut());
            std::iter::once(length)
                .chain([&mut tournament.noise])
                .chain(chances)
                .collect()
        },
    }
}

/// Whether the number written at `at` is the one at `path`, for `player`
/// when it is given. A payoff says `me's payoff in games.g.payoffs.a.b` (and
/// in the count form `, with the others playing ...` after it); the others
/// say `games.g.dynamics.generations`.
fn is_at(at: &str, path: &str, player: Option<&str>) -> bool {
    at.strip_prefix("games.").map_or_else(
        || {
            at.split_once(" payoff in games.")
                .is_some_and(|(who, there)| {
                    (there == path
                        || there
                            .strip_prefix(path)
                            .is_some_and(|r| r.starts_with(", with")))
                        && player.is_none_or(|p| who == format!("{p}'s"))
                })
        },
        |there| there == path,
    )
}

/// The change, kept in `files` only when Forge reads the model back valid
/// with the number as asked and nothing else changed
pub(in crate::edit) fn apply(
    files: &mut HashMap<String, String>,
    main: &str,
    before: &ParsedModel,
    target: &Target<'_>,
    change: &Change,
) -> Result<(), String> {
    let names: Vec<&str> = target.name.split('.').collect();
    let game = game_of(target.model, &target.name).expect("the caller found a game's numbers");
    let players = match game {
        Game::Table(table) => table.players.as_slice(),
        Game::Tournament(_) => &[],
    };
    let (payoff, raw) = given(change)?;
    let (text, player) = parse(&files[&target.file]).and_then(|file| {
        let place = find(&file, names[0], players, &names[1..])?;
        let (style, flow) = match &place {
            Place::Written(scalar, flow, _) => (
                match scalar.value().chars().next() {
                    Some('"') => Style::Double,
                    Some('\'') => Style::Single,
                    _ => Style::Plain,
                },
                *flow,
            ),
            Place::Missing(map) => (Style::Plain, map.is_flow_style()),
        };
        let new = match payoff {
            Payoff::Formula(_) => encode_formula(&raw, style, flow),
            Payoff::Number(_) => raw,
        };
        match place {
            Place::Written(scalar, _, player) => {
                scalar.set_value(&new);
                Ok((file.to_string(), player))
            },
            Place::Missing(map) => {
                let key = names[names.len() - 1];
                let text = file.to_string();
                let block = format!("{}{key}: {new}", key_indent(&text, &map));
                let edit = push_entry(&text, &map, &format!("{key}: {new}"), &block)
                    .expect("a game's section has a key");
                Ok((splice(&text, vec![edit]), None))
            },
        }
    })?;
    let path = names[..names.len() - usize::from(player.is_some())].join(".");
    let mut written = game.clone();
    for cell in cells_of(&mut written) {
        if is_at(&cell.at, &path, player) {
            cell.payoff = payoff.clone();
        }
    }
    let mut want = before.clone();
    included(&mut want, &target.aliases)
        .games
        .insert(names[0].to_string(), written);
    let mut was = check::snapshot(&want);
    bare(&mut was);
    let mut changed = files.clone();
    changed.insert(target.file.clone(), text);
    let after = check::read(&changed, main)
        .and_then(|after| check::valid(&after).map(|()| after))
        .map_err(|e| format!("Forge refuses the model with this change: {e}"))?;
    let mut now = check::snapshot(&after);
    bare(&mut now);
    (was == now)
        .then(|| *files = changed)
        .ok_or("Forge reads the new text as more than this change, so it was not made")
        .map_err(String::from)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{game_change, replaced, TRIO};
    use crate::edit::{edit_files, Change, Op, Value};
    use std::collections::HashMap;

    const HUNT: &str = "_forge_version: \"5.6.0\"
c: 0
games:
  stag_hunt:
    players: [me, you]
    strategies: [stag, hare]
    symmetric: true
    payoffs:
      stag: {stag: 4, hare: 0}
      hare: {stag: =3 - c, hare: \"=3 - c\"}
results:
  top:
    formula: \"=stag_hunt.pure_equilibria\"
";

    fn files(text: &str) -> HashMap<String, String> {
        HashMap::from([("model.yaml".to_string(), text.to_string())])
    }

    fn cell_change(op: Op, path: &str, value: Option<Value>, formula: Option<String>) -> Change {
        Change {
            value,
            formula,
            ..game_change(op, path, None)
        }
    }

    fn cell(op: Op, path: &str, value: Option<Value>, formula: Option<&str>) -> Change {
        Change {
            value,
            formula: formula.map(str::to_string),
            ..game_change(op, path, None)
        }
    }

    fn set(text: &str, change: &Change) -> Result<String, String> {
        edit_files(&files(text), "model.yaml", std::slice::from_ref(change))
            .map(|done| done["model.yaml"].clone())
            .map_err(|e| e.to_string())
    }

    fn formula(text: &str, path: &str, formula: &str) -> Result<String, String> {
        set(text, &cell(Op::SetFormula, path, None, Some(formula)))
    }

    fn number(text: &str, path: &str, value: Value) -> Result<String, String> {
        set(text, &cell(Op::SetValue, path, Some(value), None))
    }

    #[test]
    fn a_cell_is_a_number_or_a_formula_in_the_style_of_its_place() {
        let hunt = |pairs: &[(&str, &str)]| replaced(HUNT, pairs);
        assert_eq!(
            number(HUNT, "stag_hunt.payoffs.stag.stag", Value::Number(5.0)).unwrap(),
            hunt(&[("stag: 4,", "stag: 5,")])
        );
        assert_eq!(
            number(
                HUNT,
                "stag_hunt.payoffs.stag.hare",
                Value::Text("1e-5".into())
            )
            .unwrap(),
            hunt(&[("hare: 0}", "hare: 1e-5}")])
        );
        assert_eq!(
            formula(HUNT, "stag_hunt.payoffs.hare.stag", "=2 - c").unwrap(),
            hunt(&[("stag: =3 - c", "stag: =2 - c")])
        );
        // The quotes the file uses, and quotes where a flow mapping needs them
        assert_eq!(
            formula(HUNT, "stag_hunt.payoffs.hare.hare", "=c + 1").unwrap(),
            hunt(&[("\"=3 - c\"", "\"=c + 1\"")])
        );
        assert_eq!(
            formula(HUNT, "stag_hunt.payoffs.stag.hare", "=MAX(c, 1)").unwrap(),
            hunt(&[("hare: 0}", "hare: \"=MAX(c, 1)\"}")])
        );
        assert_eq!(
            formula(HUNT, "stag_hunt.payoffs.stag.stag", "=c + 4").unwrap(),
            hunt(&[("stag: 4,", "stag: =c + 4,")])
        );
        // A formula becomes a number
        assert_eq!(
            number(HUNT, "stag_hunt.payoffs.hare.stag", Value::Number(2.0)).unwrap(),
            hunt(&[("stag: =3 - c", "stag: 2")])
        );
    }

    #[test]
    fn a_list_of_payoffs_is_changed_for_the_player_that_ends_the_name() {
        assert_eq!(
            number(TRIO, "trio.payoffs.up.left.near.bob", Value::Number(9.0)).unwrap(),
            replaced(TRIO, &[("near: [1, 2, 3]", "near: [1, 9, 3]")])
        );
        assert_eq!(
            formula(TRIO, "trio.payoffs.up.right.far.ann", "=inputs.x * 2").unwrap(),
            replaced(TRIO, &[("far: [=inputs.x,", "far: [=inputs.x * 2,")])
        );
        assert_eq!(
            formula(TRIO, "trio.payoffs.down.left.far.cy", "=inputs.x").unwrap(),
            replaced(
                TRIO,
                &[(
                    "far: [1, 1, 1]\n        right",
                    "far: [1, 1, =inputs.x]\n        right"
                )]
            )
        );
    }

    #[test]
    fn a_change_that_cannot_be_made_is_refused_with_the_reason() {
        let refused = |result: Result<String, String>, why: &str| {
            let message = result.unwrap_err();
            assert!(message.contains(why), "{message}");
            assert!(message.ends_with(". No file was changed"), "{message}");
        };
        refused(
            formula(HUNT, "stag_hunt.payoffs.stag", "=1"),
            "it is a group of values: name each key that leads to the number",
        );
        refused(
            formula(HUNT, "stag_hunt.payoffs.stag.hare.you", "=1"),
            "you is beyond the number, which is one value",
        );
        refused(
            formula(HUNT, "stag_hunt.payoffs.stag.nothing", "=1"),
            "nothing is not written under games.stag_hunt.payoffs",
        );
        refused(
            formula(TRIO, "trio.payoffs.up.left.near", "=1"),
            "it is a list of payoffs: end its name with the player",
        );
        refused(
            formula(TRIO, "trio.payoffs.up.left.near.zed", "=1"),
            "zed is not a player of trio",
        );
        refused(
            formula(HUNT, "stag_hunt.payoffs.stag.hare", "1"),
            "a formula starts with =",
        );
        refused(
            number(HUNT, "stag_hunt.payoffs.stag.hare", Value::Number(f64::NAN)),
            "is not a value: a value is a finite number",
        );
        refused(
            number(
                HUNT,
                "stag_hunt.payoffs.stag.hare",
                Value::Text("0x10".into()),
            ),
            "0x10 is not a number written in decimal",
        );
        refused(
            formula(HUNT, "stag_hunt.payoffs.stag.hare", "=nothing + 1"),
            "Forge refuses the model with this change",
        );
        let alias = replaced(
            HUNT,
            &[
                ("stag: {stag: 4, hare: 0}", "stag: &s {stag: 4, hare: 0}"),
                ("hare: {stag: =3 - c, hare: \"=3 - c\"}", "hare: *s"),
            ],
        );
        refused(
            formula(&alias, "stag_hunt.payoffs.hare.stag", "=1"),
            "is not written where edit can change it",
        );
        // A game that is another's alias
        let games = replaced(
            HUNT,
            &[
                ("  stag_hunt:\n", "  stag_hunt: &hunt\n"),
                ("results:", "  copy: *hunt\nresults:"),
            ],
        );
        refused(
            formula(&games, "copy.payoffs.stag.hare", "=1"),
            "is not written where edit can change it",
        );
        let single = replaced(HUNT, &[("hare: \"=3 - c\"", "hare: '=3 - c'")]);
        assert_eq!(
            formula(&single, "stag_hunt.payoffs.hare.hare", "=c + 1").unwrap(),
            replaced(&single, &[("'=3 - c'", "'=c + 1'")])
        );
        let named = replaced(HUNT, &[("payoffs:\n      stag: {stag: 4, hare: 0}\n      hare: {stag: =3 - c, hare: \"=3 - c\"}", "payoffs: &p\n      stag: {stag: 4, hare: 0}\n      hare: {stag: =3 - c, hare: \"=3 - c\"}")]);
        assert!(formula(&named, "stag_hunt.payoffs.hare.stag", "=1").is_ok());
    }

    const DILEMMA: &str = "_forge_version: \"5.6.0\"
games:
  d: {players: [me, you], strategies: [cooperate, defect], symmetric: true, payoffs: {cooperate: {cooperate: 3, defect: 0}, defect: {cooperate: 5, defect: 1}}, dynamics: {start: {cooperate: 0.5, defect: 0.5}, generations: 10}}
  t:
    tournament:
      round: d
      players:
        a: tit_for_tat
        r: {plays: random, cooperate: 0.5}
      continuation: 0.9
";

    #[test]
    fn a_tournaments_numbers_and_a_games_dynamics_are_changed_and_added() {
        let dilemma = |pairs: &[(&str, &str)]| replaced(DILEMMA, pairs);
        assert_eq!(
            number(DILEMMA, "t.tournament.continuation", Value::Number(0.5)).unwrap(),
            dilemma(&[("continuation: 0.9", "continuation: 0.5")])
        );
        assert_eq!(
            number(
                DILEMMA,
                "t.tournament.players.r.cooperate",
                Value::Number(0.25)
            )
            .unwrap(),
            dilemma(&[("cooperate: 0.5}", "cooperate: 0.25}")])
        );
        assert_eq!(
            formula(DILEMMA, "d.dynamics.generations", "=2 * 5").unwrap(),
            dilemma(&[("generations: 10}", "generations: =2 * 5}")])
        );
        // A key the file leaves at its default is added: after the last key,
        // in the indentation of the block, or in the braces
        assert_eq!(
            number(DILEMMA, "t.tournament.noise", Value::Number(0.05)).unwrap(),
            dilemma(&[(
                "continuation: 0.9\n",
                "continuation: 0.9\n      noise: 0.05\n"
            )])
        );
        assert!(formula(DILEMMA, "d.dynamics.baseline", "=1, 2")
            .unwrap_err()
            .contains("refuses"));
        assert_eq!(
            number(DILEMMA, "d.dynamics.baseline", Value::Number(1.0)).unwrap(),
            dilemma(&[("generations: 10}", "generations: 10, baseline: 1}")])
        );
    }

    #[test]
    fn a_number_a_game_does_not_write_is_refused_with_the_reason() {
        let refused = |result: Result<String, String>, why: &str| {
            let message = result.unwrap_err();
            assert!(message.contains(why), "{message}");
        };
        refused(
            number(DILEMMA, "t.tournament.rounds", Value::Number(5.0)),
            "rounds is not written under games.t.tournament",
        );
        refused(
            number(DILEMMA, "t.tournament.baseline", Value::Number(1.0)),
            "baseline is not written under games.t.tournament",
        );
        refused(
            number(DILEMMA, "t.tournament", Value::Number(1.0)),
            "it is a group of values: name each key that leads to the number",
        );
        refused(
            formula(DILEMMA, "t.tournament.players.a", "=1"),
            "Forge refuses the model with this change",
        );
        refused(
            number(DILEMMA, "d.dynamics.start.cooperate", Value::Number(0.7)),
            "add up to",
        );
    }

    /// Every number every game of every example writes, its payoffs, a
    /// tournament's length, noise and chances and the dynamics, is changed
    /// and changed back: a formula byte for byte; a number to a formula and
    /// back to the number, on the one line it is on (or one line added, for
    /// a noise or a baseline the file leaves at its default)
    #[test]
    fn every_number_of_every_example_is_changed_and_back() {
        use crate::edit::check;
        use crate::games::Payoff;
        let (mut kept, mut formulas, mut added) = (0, 0, 0);
        for (dir, files) in crate::edit::tests::example_sets() {
            let mut mains: Vec<&String> = files
                .keys()
                .filter(|k| !k.contains('/') && check::read(&files, k).is_ok())
                .collect();
            mains.sort();
            for main in mains {
                let model = check::read(&files, main).unwrap();
                for game in model.games.values() {
                    let mut named: Vec<(String, Payoff)> = Vec::new();
                    for cell in game
                        .played_once()
                        .iter()
                        .flat_map(|t| t.cells.iter().flatten())
                    {
                        let (who, there) = cell.at.split_once(" payoff in games.").unwrap();
                        let path = there.split(", with").next().unwrap();
                        let who = who.strip_suffix("'s").unwrap();
                        let symmetric = game.played_once().unwrap().symmetric;
                        let name = if symmetric {
                            path.to_string()
                        } else {
                            format!("{path}.{who}")
                        };
                        if !named.iter().any(|(n, _)| *n == name) {
                            named.push((name, cell.payoff.clone()));
                        }
                    }
                    for w in game.settings() {
                        let payoff = w
                            .formula
                            .map_or_else(|| Payoff::Number(w.number.unwrap()), Payoff::Formula);
                        named.push((w.path, payoff));
                    }
                    for (name, payoff) in named {
                        let (now, back) = match &payoff {
                            Payoff::Formula(f) => (
                                cell_change(
                                    Op::SetFormula,
                                    &name,
                                    None,
                                    Some(format!("=({})", &f[1..])),
                                ),
                                cell_change(Op::SetFormula, &name, None, Some(f.clone())),
                            ),
                            Payoff::Number(n) => (
                                cell_change(Op::SetFormula, &name, None, Some(format!("={n}"))),
                                cell_change(Op::SetValue, &name, Some(Value::Number(*n)), None),
                            ),
                        };
                        let place = format!("{}/{main} {name}", dir.display());
                        let there = edit_files(&files, main, &[now]);
                        let again = there.and_then(|there| {
                            let mut moved = files.clone();
                            moved.extend(there);
                            edit_files(&moved, main, &[back]).map(|back| (moved, back))
                        });
                        assert!(again.is_ok(), "{place}: {again:?}");
                        let (moved, back) = again.unwrap();
                        let mut text = moved;
                        text.extend(back);
                        let lines =
                            |t: &String| t.split('\n').map(str::to_string).collect::<Vec<_>>();
                        let (a, b) = (lines(&files[main]), lines(&text[main]));
                        let differing = a.iter().zip(&b).filter(|(x, y)| x != y).count();
                        if matches!(payoff, Payoff::Formula(_)) {
                            assert_eq!(text[main], files[main], "{place}");
                            formulas += 1;
                        } else if b.len() == a.len() + 1 {
                            added += 1;
                        } else {
                            assert!(a.len() == b.len() && differing <= 1, "{place}");
                        }
                        kept += 1;
                    }
                }
            }
        }
        assert_eq!((kept, formulas, added), (85, 48, 6));
    }
}
