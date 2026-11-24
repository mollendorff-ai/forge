//! `set_habit`: the habit a tournament's player plays (ADR-075, ADR-076
//! decision 5, step 5).
//!
//! A player is named as `rivalry.players.copycat` and `plays` is its new
//! habit (`tit_for_tat`, `random`, ...). The habit is written where the file
//! has it, `copycat: tit_for_tat` or the `plays:` of `copycat: {plays: random,
//! cooperate: 0.5}`, in the quotes the file uses, and the model is read back:
//! valid, with that player's habit as asked and nothing else changed. A
//! chance the player was given stays; Forge refuses the model when the new
//! habit does not take it, and says so.

use super::{confirm, habits};
use crate::edit::text::parse;
use crate::edit::{Change, Target};
use crate::games::strategy::Plays;
use crate::games::{Game, Tournament};
use crate::types::ParsedModel;
use std::collections::HashMap;
use yaml_edit::{Scalar, YamlFile};

/// The scalar that names the habit of `player` in `game`'s tournament
fn habit_of(file: &YamlFile, game: &str, player: &str) -> Result<Scalar, String> {
    file.document()
        .and_then(|d| d.as_mapping())
        .and_then(|m| m.get_mapping("games"))
        .and_then(|g| g.get_mapping(game))
        .and_then(|g| g.get_mapping("tournament"))
        .and_then(|t| t.get_mapping("players"))
        .and_then(|p| p.get(player))
        .and_then(|node| {
            node.as_scalar().cloned().or_else(|| {
                node.as_mapping()
                    .and_then(|map| map.get("plays"))
                    .and_then(|plays| plays.as_scalar().cloned())
            })
        })
        .ok_or_else(|| {
            format!(
                "games.{game}.tournament.players.{player} is not written where edit can change it \
                 (an alias, or another shape)"
            )
        })
}

/// `habit` in the quotes `old` was written in
fn quoted(old: &str, habit: &str) -> String {
    match old.chars().next() {
        Some(q @ ('"' | '\'')) => format!("{q}{habit}{q}"),
        _ => habit.to_string(),
    }
}

/// The tournament `game` and the player named by `names`, or why not
fn player_of<'m>(
    model: &'m ParsedModel,
    names: &[&str],
) -> Result<(&'m str, &'m Tournament), String> {
    let [game, "players", player] = names else {
        return Err(format!(
            "it is not named as a tournament's player: {}.players.<name>",
            names[0]
        ));
    };
    let found = model
        .games
        .get_key_value(*game)
        .map(|(n, g)| (n.as_str(), g));
    match found {
        Some((name, Game::Tournament(t))) if t.entrants.iter().any(|e| e.name == *player) => {
            Ok((name, t))
        },
        Some((_, Game::Tournament(_))) => Err(format!("{player} is not a player of {game}")),
        Some(_) => Err(format!(
            "{game} is not a tournament: its players have strategies, not a habit"
        )),
        None => Err(format!("{game} is not a game in the model")),
    }
}

/// The change, kept in `files` only when Forge reads the model back valid
/// with the player's habit as asked and nothing else changed
pub(in crate::edit) fn apply(
    files: &mut HashMap<String, String>,
    main: &str,
    before: &ParsedModel,
    target: &Target<'_>,
    change: &Change,
) -> Result<(), String> {
    let names: Vec<&str> = target.name.split('.').collect();
    let habit = change
        .plays
        .as_deref()
        .ok_or_else(|| format!("set_habit needs `plays`, one of {}", habits()))?;
    let plays = Plays::from_name(habit)
        .ok_or_else(|| format!("{habit} is not a habit a player plays: one of {}", habits()))?;
    let (name, tournament) = player_of(target.model, &names)?;
    let text = parse(&files[&target.file]).and_then(|file| {
        let scalar = habit_of(&file, name, names[2])?;
        scalar.set_value(&quoted(&scalar.value(), habit));
        Ok(file.to_string())
    })?;
    let mut written = tournament.clone();
    written
        .entrants
        .iter_mut()
        .filter(|e| e.name == names[2])
        .for_each(|e| e.plays = plays);
    confirm(
        files,
        main,
        before,
        target,
        name,
        Game::Tournament(written),
        text,
    )
}

#[cfg(test)]
mod tests {
    use crate::edit::tests::example_sets;
    use crate::edit::{check, edit_files, Change, Op};
    use crate::games::strategy::Plays;
    use crate::games::Game;
    use std::collections::{BTreeMap, HashMap};

    fn set_habit(path: &str, plays: Option<&str>) -> Change {
        super::super::tests::game_change(Op::SetHabit, path, plays)
    }

    fn run(
        files: &HashMap<String, String>,
        change: &Change,
    ) -> Result<BTreeMap<String, String>, String> {
        edit_files(files, "model.yaml", std::slice::from_ref(change)).map_err(|e| e.to_string())
    }

    const PD: &str = "_forge_version: \"5.6.0\"
games:
  pd:
    players: [me, you]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs:
      cooperate: {cooperate: 3, defect: 0}
      defect: {cooperate: 5, defect: 1}
";

    fn model(tournament: &str) -> HashMap<String, String> {
        HashMap::from([(
            "model.yaml".to_string(),
            format!("{PD}  duel:\n    tournament:\n{tournament}"),
        )])
    }

    /// The text after `plays` for `path`, and the text after setting it back
    fn there_and_back(files: &HashMap<String, String>, path: &str, from: &str, to: &str) -> String {
        let there = run(files, &set_habit(path, Some(to))).unwrap()["model.yaml"].clone();
        let mut now = files.clone();
        now.insert("model.yaml".into(), there.clone());
        let back = run(&now, &set_habit(path, Some(from))).unwrap();
        assert_eq!(back["model.yaml"], files["model.yaml"], "{path} back");
        there
    }

    #[test]
    fn a_habit_is_written_where_the_file_has_it_and_back_byte_for_byte() {
        let files = model(
            "      round: pd
      players:
        a: tit_for_tat   # first
        b: \"always_defect\"
        c: 'grim_trigger'
        d: {plays: random, cooperate: 0.5}
        e:
          plays: always_cooperate
      rounds: 10
",
        );
        let text = &files["model.yaml"];
        let was = |from: &str, to: &str| text.replacen(from, to, 1);
        assert_eq!(
            there_and_back(&files, "duel.players.a", "tit_for_tat", "random"),
            was("a: tit_for_tat   #", "a: random   #")
        );
        assert_eq!(
            there_and_back(&files, "duel.players.b", "always_defect", "random"),
            was("\"always_defect\"", "\"random\"")
        );
        assert_eq!(
            there_and_back(&files, "duel.players.c", "grim_trigger", "random"),
            was("'grim_trigger'", "'random'")
        );
        assert!(run(&files, &set_habit("duel.players.d", Some("random")))
            .unwrap()
            .is_empty());
        assert_eq!(
            there_and_back(&files, "duel.players.e", "always_cooperate", "tit_for_tat"),
            was("plays: always_cooperate", "plays: tit_for_tat")
        );
    }

    #[test]
    fn a_player_with_a_chance_keeps_it_and_forge_refuses_a_habit_that_does_not_take_it() {
        let files = model(
            "      round: pd
      players:
        a: tit_for_tat
        g: {plays: generous_tit_for_tat, forgive: 0.25}
      rounds: 10
",
        );
        let message = run(&files, &set_habit("duel.players.g", Some("random"))).unwrap_err();
        assert!(
            message.contains("set_habit duel.players.g: Forge refuses the model with this change"),
            "{message}"
        );
        assert!(message.contains("takes forgive"), "{message}");
        assert!(message.ends_with("No file was changed"), "{message}");
    }

    #[test]
    fn a_change_that_cannot_be_made_is_refused_with_the_reason() {
        let files = model(
            "      round: pd
      players: {a: tit_for_tat, b: always_defect}
      rounds: 10
",
        );
        let aliased = HashMap::from([(
            "model.yaml".to_string(),
            files["model.yaml"].replace(
                "players: {a: tit_for_tat, b: always_defect}",
                "players: &roster {a: tit_for_tat, b: always_defect}",
            ) + "  again:\n    tournament: {round: pd, players: *roster, rounds: 10}\n",
        )]);
        let mut cases: Vec<(&HashMap<String, String>, Change, &str)> = vec![
            (
                &files,
                set_habit("duel.players.a", None),
                "set_habit needs `plays`, one of always_cooperate",
            ),
            (
                &files,
                set_habit("duel.players.a", Some("sulk")),
                "sulk is not a habit a player plays: one of always_cooperate",
            ),
            (
                &files,
                set_habit("duel.players.z", Some("random")),
                "z is not a player of duel",
            ),
            (
                &files,
                set_habit("pd.players.me", Some("random")),
                "pd is not a tournament: its players have strategies, not a habit",
            ),
            (
                &files,
                set_habit("nowhere.players.a", Some("random")),
                "nowhere is not a game in the model",
            ),
            (
                &files,
                set_habit("duel.strategies.a", Some("random")),
                "it is not named as a tournament's player: duel.players.<name>",
            ),
            (
                &aliased,
                set_habit("duel.players.a", Some("random")),
                "Forge reads the new text as more than this change",
            ),
            (
                &aliased,
                set_habit("again.players.a", Some("random")),
                "games.again.tournament.players.a is not written where edit can change it",
            ),
        ];
        cases.push((
            &files,
            Change {
                value: Some(crate::edit::Value::Number(1.0)),
                ..set_habit("duel.players.a", Some("random"))
            },
            "set_habit does not take `value`",
        ));
        for (files, change, why) in cases {
            let message = run(files, &change).unwrap_err();
            assert!(message.contains(why), "{why}: {message}");
        }
    }

    /// Each habit of each player of each example's tournaments, then the
    /// habit it had: the text back byte for byte where Forge allows the habit
    #[test]
    fn every_habit_of_every_example_is_changed_and_back() {
        let (mut kept, mut refused) = (0, Vec::new());
        for (dir, files) in example_sets() {
            let mut mains: Vec<&String> = files.keys().filter(|k| !k.contains('/')).collect();
            mains.sort();
            for main in mains {
                let model = check::read(&files, main).unwrap();
                let entrants = model.games.iter().filter_map(|(g, game)| match game {
                    Game::Tournament(t) => Some((g, t)),
                    Game::Table(_) => None,
                });
                for (g, t) in entrants {
                    for e in &t.entrants {
                        for habit in Plays::ALL {
                            let path = format!("{g}.players.{}", e.name);
                            let set = |name: &str, files: &HashMap<String, String>| {
                                let change = set_habit(&path, Some(name));
                                edit_files(files, main, &[change]).map_err(|e| e.to_string())
                            };
                            let there = set(habit.name(), &files);
                            let back = there.and_then(|there| {
                                let mut now = files.clone();
                                now.extend(there);
                                set(e.plays.name(), &now)
                            });
                            match back {
                                Ok(back) if back.iter().all(|(n, t)| files[n] == *t) => kept += 1,
                                other => refused.push(format!(
                                    "{}/{main} {path} {}: {other:?}",
                                    dir.display(),
                                    habit.name()
                                )),
                            }
                        }
                    }
                }
            }
        }
        assert_eq!((kept, refused.len()), (112, 95), "{refused:#?}");
        // What Forge refuses, and why: the roster two tournaments share; a
        // player's chance a new habit does not take; formulas that divide by
        // what a new habit makes equal
        let why = |text: &str| refused.iter().filter(|r| r.contains(text)).count();
        assert_eq!(why("noisy-tournament.yaml clean."), 40, "{refused:#?}");
        assert_eq!(why("Forge reads the new text as more than this change"), 40);
        assert_eq!(why("noisy-tournament.yaml noisy."), 45, "{refused:#?}");
        assert_eq!(why("is not written where edit can change it"), 45);
        assert_eq!(why("axelrod.yaml axelrod.players.rand"), 8, "{refused:#?}");
        assert_eq!(
            why("Parse error: 'games.axelrod.tournament.players.rand'"),
            8
        );
        assert_eq!(
            why("kinship.crossover_c: Division by zero"),
            2,
            "{refused:#?}"
        );
    }
}
