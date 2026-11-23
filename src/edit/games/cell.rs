//! `set_value` and `set_formula` on a payoff cell (ADR-075, ADR-076
//! decision 5).
//!
//! A cell is named by its game and the strategies that lead to it:
//! `stag_hunt.payoffs.hare.stag`. When the cell is a list, one payoff for
//! each player, the player ends the name: `inspection.payoffs.cheat.audit.human`.
//! A game of players alike (the count form) is named by its strategy:
//! `public_goods.payoffs.keep`. The payoff is written in the file's own
//! style, in place, and the model is read back: valid, with exactly the cells
//! that are written there changed (a symmetric game's second player reads
//! the first's).

use super::{bare, included};
use crate::edit::text::{encode_formula, parse, Style};
use crate::edit::{check, formula, value_text, Change, Op, Target};
use crate::games::{Game, Payoff, TableGame};
use crate::types::ParsedModel;
use std::collections::HashMap;
use yaml_edit::{Scalar, YamlFile, YamlNode};

/// The game played once that `name` points into: `game.payoffs...`
pub(in crate::edit) fn table_of<'m>(model: &'m ParsedModel, name: &str) -> Option<&'m TableGame> {
    let mut parts = name.split('.');
    let game = parts
        .next()
        .and_then(|g| model.games.get(g))
        .and_then(Game::played_once);
    game.filter(|_| parts.next() == Some("payoffs"))
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

/// The scalar the cell is written as, whether it is in a flow collection,
/// and the player it is for when the cell is a list
fn find<'n>(
    file: &YamlFile,
    game: &TableGame,
    names: &[&'n str],
) -> Result<(Scalar, bool, Option<&'n str>), String> {
    let path = format!("games.{}.payoffs", game.name);
    let not_written =
        || format!("{path} is not written where edit can change it (an alias, or another shape)");
    let mut node = file
        .document()
        .and_then(|d| d.as_mapping())
        .and_then(|m| m.get_mapping("games"))
        .and_then(|g| g.get_mapping(&game.name))
        .and_then(|g| g.get("payoffs"))
        .ok_or_else(not_written)?;
    let mut names = names.iter();
    let mut flow = false;
    let mut listed = None;
    loop {
        node = match node {
            YamlNode::Scalar(scalar) => {
                return names.next().map_or(Ok((scalar, flow, listed)), |extra| {
                    Err(format!(
                        "{extra} is beyond the cell, which is one payoff: name the strategies \
                         that lead to it"
                    ))
                });
            },
            YamlNode::Mapping(map) => {
                let key = names
                    .next()
                    .ok_or("name the strategy of each player that leads to the cell")?;
                flow = map.is_flow_style();
                map.get(key)
                    .ok_or_else(|| format!("{key} is not a strategy written under {path}"))?
            },
            YamlNode::Sequence(list) => {
                let player = names
                    .next()
                    .ok_or("the cell is a list of payoffs: end its name with the player")?;
                let index = game
                    .players
                    .iter()
                    .position(|p| p == player)
                    .ok_or_else(|| format!("{player} is not a player of {}", game.name))?;
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

/// Whether the cell written at `at` (`me's payoff in games.g.payoffs.a.b`,
/// and in the count form `, with the others playing ...` after it) is the
/// one at `path`, for `player` when it is given
fn is_at(at: &str, path: &str, player: Option<&str>) -> bool {
    at.split_once(" payoff in games.")
        .is_some_and(|(who, there)| {
            (there == path
                || there
                    .strip_prefix(path)
                    .is_some_and(|r| r.starts_with(", with")))
                && player.is_none_or(|p| who == format!("{p}'s"))
        })
}

/// The change, kept in `files` only when Forge reads the model back valid
/// with the cell as asked and nothing else changed
pub(in crate::edit) fn apply(
    files: &mut HashMap<String, String>,
    main: &str,
    before: &ParsedModel,
    target: &Target<'_>,
    change: &Change,
) -> Result<(), String> {
    let names: Vec<&str> = target.name.split('.').collect();
    let table = table_of(target.model, &target.name).expect("the caller found a game's payoffs");
    let (payoff, raw) = given(change)?;
    let (text, player) = parse(&files[&target.file]).and_then(|file| {
        let (scalar, flow, player) = find(&file, table, &names[2..])?;
        let style = match scalar.value().chars().next() {
            Some('"') => Style::Double,
            Some('\'') => Style::Single,
            _ => Style::Plain,
        };
        scalar.set_value(&match payoff {
            Payoff::Formula(_) => encode_formula(&raw, style, flow),
            Payoff::Number(_) => raw,
        });
        Ok((file.to_string(), player))
    })?;
    let path = names[..names.len() - usize::from(player.is_some())].join(".");
    let mut game = table.clone();
    for cell in game.cells.iter_mut().flatten() {
        if is_at(&cell.at, &path, player) {
            cell.payoff = payoff.clone();
        }
    }
    let mut want = before.clone();
    included(&mut want, &target.aliases)
        .games
        .insert(game.name.clone(), Game::Table(game));
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
            "name the strategy of each player that leads to the cell",
        );
        refused(
            formula(HUNT, "stag_hunt.payoffs.stag.hare.you", "=1"),
            "you is beyond the cell, which is one payoff",
        );
        refused(
            formula(HUNT, "stag_hunt.payoffs.stag.nothing", "=1"),
            "nothing is not a strategy written under games.stag_hunt.payoffs",
        );
        refused(
            formula(TRIO, "trio.payoffs.up.left.near", "=1"),
            "the cell is a list of payoffs: end its name with the player",
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

    /// Every written cell of every game of every example is changed and
    /// changed back: a formula byte for byte, a number to the one line it is
    /// on; Forge keeps what the model allows and says why for the rest
    #[test]
    fn every_cell_of_every_example_is_changed_and_back() {
        use crate::edit::check;
        use crate::games::Payoff;
        let (mut kept, mut formulas) = (0, 0);
        for (dir, files) in crate::edit::tests::example_sets() {
            let mut mains: Vec<&String> = files
                .keys()
                .filter(|k| !k.contains('/') && check::read(&files, k).is_ok())
                .collect();
            mains.sort();
            for main in mains {
                let model = check::read(&files, main).unwrap();
                for table in model.games.values().filter_map(|g| g.played_once()) {
                    let mut seen: Vec<String> = Vec::new();
                    for cell in table.cells.iter().flatten() {
                        let (who, there) = cell.at.split_once(" payoff in games.").unwrap();
                        let path = there.split(", with").next().unwrap();
                        let who = who.strip_suffix("'s").unwrap();
                        let name = if table.symmetric {
                            path.to_string()
                        } else {
                            format!("{path}.{who}")
                        };
                        if seen.contains(&name) {
                            continue;
                        }
                        seen.push(name.clone());
                        let (op, now, back) = match &cell.payoff {
                            Payoff::Formula(f) => (
                                Op::SetFormula,
                                cell_change(
                                    Op::SetFormula,
                                    &name,
                                    None,
                                    Some(format!("=({})", &f[1..])),
                                ),
                                cell_change(Op::SetFormula, &name, None, Some(f.clone())),
                            ),
                            Payoff::Number(n) => (
                                Op::SetValue,
                                cell_change(
                                    Op::SetValue,
                                    &name,
                                    Some(Value::Number(n + 1.0)),
                                    None,
                                ),
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
                        if op == Op::SetFormula {
                            assert_eq!(text[main], files[main], "{place}");
                            formulas += 1;
                        } else {
                            assert!(a.len() == b.len() && differing <= 1, "{place}");
                        }
                        kept += 1;
                    }
                }
            }
        }
        assert_eq!((kept, formulas), (58, 37));
    }
}
