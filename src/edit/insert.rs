//! `insert`: a template's block added to the model's `games:` (ADR-075
//! step 6; ADR-076, decision 6).
//!
//! The block goes after the last game, indented as the file's games are, or
//! at the end of a file that has none, as a `games:` of its own. A model of
//! an earlier format than 5.5.0, which has no games, is raised to 5.5.0,
//! which adds to the earlier formats and changes none of their rules. The
//! change is kept only when Forge reads the model back valid with the
//! template's games added and nothing else changed.

use super::templates::{self, Template};
use super::text::parse;
use super::{check, games, is_name, Change};
use crate::parser::format_at_least;
use crate::types::ParsedModel;
use std::collections::HashMap;
use yaml_edit::YamlNode;

/// The model's file with the template's block added, and the name of its game
///
/// `known` says whether the model already uses a name for a game.
fn write(
    text: &str,
    template: &Template,
    new_name: Option<&str>,
    has_games: bool,
    known: &dyn Fn(&str) -> bool,
) -> Result<(String, String), String> {
    let file = parse(text)?;
    let top = file
        .document()
        .and_then(|d| d.as_mapping())
        .ok_or("it is not a mapping of names")?;
    let taken = |name: &str| top.contains_key(name) || known(name);
    let name = new_name.map_or_else(|| free(template, &taken), str::to_string);
    if !is_name(&name) {
        return Err(format!(
            "{name} is not a name: a letter or _, then letters, digits and _"
        ));
    }
    if let Some(used) = template.names(&name).into_iter().find(|n| taken(n)) {
        return Err(format!("{used} is already a name in the model"));
    }
    let mut text = text.to_string();
    let block = template.block(&name);
    match top.get("games") {
        None => {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text = format!("{text}\ngames:\n{block}");
        },
        Some(YamlNode::Mapping(games)) if has_games && !games.is_flow_style() => {
            let first = games
                .entries()
                .next()
                .map_or(0, |e| e.byte_range().start as usize);
            let unit = &text[text[..first].rfind('\n').map_or(0, |i| i + 1)..first];
            let at = games
                .entries()
                .last()
                .map_or(0, |e| e.byte_range().end as usize);
            let lines: Vec<String> = block
                .lines()
                .map(|line| {
                    let rest = line.trim_start_matches(' ');
                    let level = (line.len() - rest.len()) / 2;
                    if rest.is_empty() {
                        String::new()
                    } else {
                        format!("{}{rest}", unit.repeat(level))
                    }
                })
                .collect();
            let body = lines.join("\n");
            let added = if text[..at].ends_with('\n') {
                format!("\n{body}\n")
            } else {
                format!("\n\n{body}")
            };
            text.insert_str(at, &added);
        },
        Some(_) => {
            return Err(
                "its games: is written where edit cannot add to it (empty, on one line, or not a group of games)"
                    .into(),
            )
        },
    }
    Ok((raise(&text), name))
}

/// `text` in format 5.5.0 or later: an earlier `_forge_version` is set to 5.5.0
fn raise(text: &str) -> String {
    let file = parse(text).expect("the text with a block added is YAML");
    let top = file
        .document()
        .and_then(|d| d.as_mapping())
        .expect("a model is a mapping of names");
    if let Some(YamlNode::Scalar(version)) = top.get("_forge_version") {
        if !format_at_least(version.value().trim_matches(['"', '\'']), "5.5.0") {
            version.set_value("\"5.5.0\"");
        }
    }
    file.to_string()
}

/// The first of `default`, `default_2`, ... that the file does not use as a
/// name (the round of a tournament too)
fn free(template: &Template, taken: &dyn Fn(&str) -> bool) -> String {
    (1..=u32::MAX)
        .map(|n| {
            if n == 1 {
                template.default.to_string()
            } else {
                format!("{}_{n}", template.default)
            }
        })
        .find(|name| !template.names(name).iter().any(|n| taken(n)))
        .expect("an unbounded count finds a free name")
}

/// The games of `template` called `name`, as Forge reads them alone
fn read_games(template: &Template, name: &str) -> ParsedModel {
    let text = format!(
        "_forge_version: \"5.5.0\"\ngames:\n{}",
        template.block(name)
    );
    let files = HashMap::from([("template.yaml".to_string(), text)]);
    check::read(&files, "template.yaml").expect("a template is a model of its own")
}

/// `change`: the template `change.path` inserted into `main`, kept in `files`
/// only when Forge agrees
pub(super) fn apply(
    files: &mut HashMap<String, String>,
    main: &str,
    before: &ParsedModel,
    change: &Change,
) -> Result<(), String> {
    let template = templates::find(&change.path).ok_or_else(|| {
        let names: Vec<&str> = templates::TEMPLATES.iter().map(|t| t.name).collect();
        format!(
            "there is no template {}: they are {}",
            change.path,
            names.join(", ")
        )
    })?;
    let (text, name) = write(
        &files[main],
        template,
        change.new_name.as_deref(),
        !before.games.is_empty(),
        &|name| before.games.contains_key(name),
    )
    .map_err(|why| format!("{why} ({main})"))?;
    let mut changed = files.clone();
    changed.insert(main.to_string(), text);
    let mut want = before.clone();
    want.games.extend(read_games(template, &name).games);
    let after = check::read(&changed, main)
        .and_then(|after| check::valid(&after).map(|()| after))
        .map_err(|e| format!("Forge refuses the model with this change: {e}"))?;
    let mut was = check::snapshot(&want);
    let mut now = check::snapshot(&after);
    games::bare(&mut was);
    games::bare(&mut now);
    // Forge reads the template's games and nothing else, as the block is
    // text added after the last game and the model's names are checked
    let more = "Forge reads the new text as more than this change, so it was not made".to_string();
    (was == now)
        .then_some(())
        .ok_or(more)
        .map(|()| *files = changed)
}

#[cfg(test)]
mod tests {
    use super::super::{edit_files, templates, Change, Op};
    use super::write;
    use std::collections::{BTreeMap, HashMap};

    fn insert(template: &str, new_name: Option<&str>) -> Change {
        Change {
            op: Op::Insert,
            path: template.into(),
            value: None,
            formula: None,
            unit: None,
            after: None,
            new_name: new_name.map(str::to_string),
            plays: None,
        }
    }

    fn run(text: &str, change: &Change) -> Result<String, String> {
        let files = HashMap::from([("m.yaml".to_string(), text.to_string())]);
        edit_files(&files, "m.yaml", std::slice::from_ref(change))
            .map(|done: BTreeMap<String, String>| done.get("m.yaml").cloned().unwrap_or_default())
            .map_err(|e| e.to_string())
    }

    const PLAIN: &str = "_forge_version: \"5.1.0\"\nprice: 16\n";

    #[test]
    fn every_template_is_a_valid_block_with_its_own_names() {
        let all = templates::templates();
        let names: Vec<&str> = all.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            ["game_of_two", "game_of_many", "tournament", "dynamics"]
        );
        for t in &all {
            assert!(t.yaml.contains(&format!("  {}:", t.default)), "{}", t.name);
            assert!(!t.yaml.contains("{name}"), "{}", t.name);
            let done = run(PLAIN, &insert(&t.name, None)).unwrap();
            assert!(done.contains(&t.yaml), "{}: {done}", t.name);
        }
        let tournament = templates::find("tournament").unwrap();
        assert_eq!(tournament.names("war"), ["war_round", "war"]);
        assert!(templates::find("nothing").is_none());
    }

    #[test]
    fn a_model_without_games_gets_a_games_section_and_the_format_that_has_games() {
        let done = run(PLAIN, &insert("game_of_two", None)).unwrap();
        let block = templates::find("game_of_two").unwrap().block("new_game");
        assert_eq!(
            done,
            format!("_forge_version: \"5.5.0\"\nprice: 16\n\ngames:\n{block}")
        );
        // No last line break; a later format stays
        let done = run(
            "_forge_version: \"5.6.0\"\nprice: 16",
            &insert("dynamics", Some("hunt")),
        )
        .unwrap();
        assert!(
            done.starts_with("_forge_version: \"5.6.0\"\nprice: 16\n\ngames:\n  hunt:\n"),
            "{done}"
        );
    }

    #[test]
    fn a_game_is_added_after_the_last_in_the_indentation_the_games_use() {
        let two = "_forge_version: \"5.5.0\"\ngames:\n  old:\n    players: [a, b]\n    strategies: [x, y]\n    payoffs:\n      x: {x: [1, 1], y: [0, 0]}\n      y: {x: [0, 0], y: [1, 1]}\n";
        let done = run(two, &insert("game_of_two", None)).unwrap();
        assert!(done.starts_with(two), "{done}");
        assert!(done.ends_with("\n\n  new_game:\n    description: Two players choose at once; each cell is [row's payoff, column's payoff]\n    players: [row, column]\n    strategies: [cooperate, defect]\n    payoffs:\n      cooperate: {cooperate: [3, 3], defect: [0, 5]}\n      defect: {cooperate: [5, 0], defect: [1, 1]}\n"), "{done}");
        // Four spaces a level, and a file with no last line break
        let four = two.replace("  ", "    ");
        let four = four.trim_end();
        let done = run(four, &insert("game_of_two", None)).unwrap();
        assert!(
            done.contains("\n\n    new_game:\n        description: Two"),
            "{done}"
        );
        assert!(
            done.contains("\n        payoffs:\n            cooperate: {cooperate"),
            "{done}"
        );
    }

    #[test]
    fn a_name_is_free_given_or_refused() {
        let first = run(PLAIN, &insert("game_of_two", None)).unwrap();
        let second = run(&first, &insert("game_of_two", None)).unwrap();
        assert!(second.contains("\n  new_game_2:\n"), "{second}");
        let named = run(&first, &insert("game_of_two", Some("duel"))).unwrap();
        assert!(named.contains("\n  duel:\n"), "{named}");
        // A tournament's round is taken too
        let round =
            format!("{PLAIN}new_war_round: 1\n").replace("new_war_round", "new_tournament_round");
        let done = run(&round, &insert("tournament", None)).unwrap();
        assert!(done.contains("\n  new_tournament_2_round:\n"), "{done}");
        let message = |change: &Change, text: &str| run(text, change).unwrap_err();
        assert_eq!(
            message(&insert("game_of_two", Some("new_game")), &first),
            "Validation error: insert game_of_two: new_game is already a name in the model (m.yaml). No file was changed"
        );
        assert_eq!(
            message(&insert("game_of_two", Some("price")), PLAIN),
            "Validation error: insert game_of_two: price is already a name in the model (m.yaml). No file was changed"
        );
        assert_eq!(
            message(&insert("game_of_two", Some("two words")), PLAIN),
            "Validation error: insert game_of_two: two words is not a name: a letter or _, then letters, digits and _ (m.yaml). No file was changed"
        );
        assert_eq!(
            message(&insert("sealed", None), PLAIN),
            "Validation error: insert sealed: there is no template sealed: they are game_of_two, game_of_many, tournament, dynamics. No file was changed"
        );
    }

    #[test]
    fn a_games_section_edit_cannot_add_to_is_refused_with_the_file() {
        let flow = "_forge_version: \"5.5.0\"\ngames: {}\n";
        assert_eq!(
            run(flow, &insert("game_of_two", None)).unwrap_err(),
            "Validation error: insert game_of_two: its games: is written where edit cannot add to it (empty, on one line, or not a group of games) (m.yaml). No file was changed"
        );
        let template = templates::find("game_of_two").unwrap();
        let bad = |text: &str| write(text, template, None, false, &|_| false).unwrap_err();
        assert_eq!(
            bad("a: 'x\n"),
            "its YAML cannot be read (Unterminated quoted string)"
        );
        assert_eq!(bad("- a\n"), "it is not a mapping of names");
        // A model Forge cannot calculate stays as it is
        let broken = "_forge_version: \"5.5.0\"\nprice: 1\ncost:\n  formula: \"=missing + 1\"\n";
        assert!(run(broken, &insert("game_of_two", None))
            .unwrap_err()
            .contains("Forge refuses the model with this change"));
    }

    /// Each template into each model of the examples: Forge keeps it and
    /// reads its games, or says why not
    #[test]
    fn every_template_is_valid_in_every_example() {
        let mut kept = 0;
        let mut refused: Vec<String> = Vec::new();
        for (dir, files) in super::super::tests::example_sets() {
            let mut mains: Vec<&String> = files.keys().filter(|k| !k.contains('/')).collect();
            mains.sort();
            for main in mains {
                if super::check::read(&files, main).is_err() {
                    continue;
                }
                for template in templates::TEMPLATES {
                    match edit_files(&files, main, &[insert(template.name, None)]) {
                        Ok(done) => {
                            assert!(done[main].contains(&template.block(template.default)));
                            kept += 1;
                        },
                        Err(e) => {
                            refused.push(format!(
                                "{}/{main} {}: {e}",
                                dir.display(),
                                template.name
                            ));
                        },
                    }
                }
            }
        }
        assert!(refused.is_empty(), "{kept} kept: {refused:#?}");
    }
}
