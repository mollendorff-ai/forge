//! The text of a strategy or a player added to a game or deleted from it
//! (ADR-075, step 5).
//!
//! Each place that names it is changed in the style the file uses there: an
//! item of a list (`[hold, cut]` or a `- cut` line), a key of a mapping (in
//! `{...}` or on lines of its own), and at each level of `payoffs:` that is
//! the player's, a key whose cells are 0. A new player is a last level of
//! `payoffs:`: each cell `[1, 2]` becomes `{a: [1, 2, 0], b: [1, 2, 0]}`, the
//! game as it was whatever the new player does. A player deleted keeps the
//! payoffs of its first strategy. So a strategy or a player added and
//! deleted again gives the file back byte for byte.

use super::{Changed, Entrants, Plan};
use crate::edit::text::{parse, splice};
use crate::games::TableGame;
use std::ops::Range;
use yaml_edit::{Mapping, MappingEntry, Sequence, YamlNode};

type Edit = (Range<usize>, String);

/// The bytes of a node
fn span(node: &YamlNode) -> Range<usize> {
    let range = node.byte_range();
    range.start as usize..range.end as usize
}

/// Where the line holding `at` starts
fn line_start(text: &str, at: usize) -> usize {
    text[..at].rfind('\n').map_or(0, |i| i + 1)
}

/// Just after the line break that ends the line holding `at`, or the end
fn line_end(text: &str, at: usize) -> usize {
    text[at..].find('\n').map_or(text.len(), |i| at + i + 1)
}

/// `lines` inserted at `at`, the end of a line or of the file
fn insert_lines(text: &str, at: usize, lines: &str) -> Edit {
    if text[..at].ends_with('\n') {
        (at..at, format!("{lines}\n"))
    } else {
        (at..at, format!("\n{lines}"))
    }
}

/// The whole lines from `from`, a line's start, to `to`, a line's end; at
/// the end of a file without a last line break, the break before them
fn remove_lines(text: &str, from: usize, to: usize) -> Edit {
    let from = if text[..to].ends_with('\n') {
        from
    } else {
        from.saturating_sub(1)
    };
    (from..to, String::new())
}

/// The text before a key or an item on its line: its indentation, and a
/// block list's `- `
fn indent(text: &str, at: usize) -> &str {
    &text[line_start(text, at)..at]
}

/// What a key says, unquoted
fn key_of(entry: &MappingEntry) -> String {
    entry
        .key_node()
        .and_then(|key| key.as_scalar().map(yaml_edit::Scalar::as_string))
        .unwrap_or_default()
}

/// Where an entry's key ends
fn key_end(entry: &MappingEntry) -> usize {
    entry.key_node().map_or(0, |key| span(&key).end)
}

/// Where an entry of a flow mapping ends: before the `, ` after it
fn flow_end(text: &str, entry: &MappingEntry) -> usize {
    let range = entry.byte_range();
    let written = &text[range.start as usize..range.end as usize];
    range.start as usize
        + written
            .trim_end_matches(|c: char| c == ',' || c.is_whitespace())
            .len()
}

/// `name` added at the end of a list
fn push_item(text: &str, list: &Sequence, name: &str) -> Option<Edit> {
    let values: Vec<YamlNode> = list.values().collect();
    let end = span(values.last()?).end;
    Some(if list.is_flow_style() {
        (end..end, format!(", {name}"))
    } else {
        let prefix = indent(text, span(&values[0]).start);
        insert_lines(text, line_end(text, end), &format!("{prefix}{name}"))
    })
}

/// The item `index` of a list of two or more taken out
fn remove_item(text: &str, list: &Sequence, index: usize) -> Option<Edit> {
    let values: Vec<Range<usize>> = list.values().map(|v| span(&v)).collect();
    let item = values.get(index)?;
    Some(if !list.is_flow_style() {
        remove_lines(text, line_start(text, item.start), line_end(text, item.end))
    } else if index > 0 {
        (values[index - 1].end..item.end, String::new())
    } else {
        (item.start..values.get(1)?.start, String::new())
    })
}

/// An entry added at the end of a mapping: `key: value` in a flow mapping,
/// or `line`, the entry's lines in a block, without the last line break
fn push_entry(text: &str, map: &Mapping, flow: &str, block: &str) -> Option<Edit> {
    let last = map.entries().last()?;
    Some(if map.is_flow_style() {
        let end = flow_end(text, &last);
        (end..end, format!(", {flow}"))
    } else {
        insert_lines(text, last.byte_range().end as usize, block)
    })
}

/// The entry `key` of a mapping taken out, with its lines in a block
fn remove_entry(text: &str, map: &Mapping, key: &str) -> Option<Edit> {
    let entries: Vec<MappingEntry> = map.entries().collect();
    let index = entries.iter().position(|e| key_of(e) == key)?;
    let range = |e: &MappingEntry| e.byte_range().start as usize..e.byte_range().end as usize;
    let entry = range(&entries[index]);
    Some(if !map.is_flow_style() {
        remove_lines(text, line_start(text, entry.start), entry.end)
    } else if index > 0 {
        let before = flow_end(text, &entries[index - 1]);
        (before..flow_end(text, &entries[index]), String::new())
    } else {
        (entry.start..range(entries.get(1)?).start, String::new())
    })
}

/// The indentation of a block mapping's keys
fn key_indent<'t>(text: &'t str, map: &Mapping) -> &'t str {
    let first = map
        .entries()
        .next()
        .and_then(|e| e.key_node())
        .map_or(0, |key| span(&key).start);
    indent(text, first)
}

/// What edit cannot change, said
fn not_written(at: &str) -> String {
    format!("{at} is not written where edit can change it (an alias, or another shape)")
}

/// The mapping `key` of `map`, which is at `path`
fn mapping(map: &Mapping, key: &str, path: &str) -> Result<Mapping, String> {
    match map.get(key) {
        Some(YamlNode::Mapping(found)) => Ok(found),
        _ => Err(not_written(path)),
    }
}

/// The list `key` of `map`, which is at `path`
fn list(map: &Mapping, key: &str, path: &str) -> Result<Sequence, String> {
    match map.get(key) {
        Some(YamlNode::Sequence(found)) => Ok(found),
        _ => Err(not_written(path)),
    }
}

/// The payoffs of a game written out, as a change walks them
struct Payoffs<'a> {
    text: &'a str,
    plan: &'a Plan,
    /// The game's players before the change
    players: usize,
    /// Each player's strategies after it
    strategies: &'a [Vec<String>],
    symmetric: bool,
    /// Where the game's payoffs are, for messages
    at: String,
}

impl Payoffs<'_> {
    /// A cell of 0s: one payoff, or one per player
    fn zero_cell(&self) -> String {
        if self.symmetric {
            "0".into()
        } else {
            format!("[{}]", vec!["0"; self.players].join(", "))
        }
    }

    /// The payoffs of 0 from `level` down, on one line
    fn flow_zero(&self, level: usize) -> String {
        if level == self.players {
            return self.zero_cell();
        }
        let entries: Vec<String> = self.strategies[level]
            .iter()
            .map(|s| format!("{s}: {}", self.flow_zero(level + 1)))
            .collect();
        format!("{{{}}}", entries.join(", "))
    }

    /// `name: ...`, the payoffs of 0 from `level` down, written as
    /// `sibling` writes the same level: on lines of their own when it does,
    /// else on one line
    fn zero_entry(
        &self,
        name: &str,
        pad: &str,
        sibling: Option<&YamlNode>,
        level: usize,
    ) -> String {
        match sibling {
            Some(YamlNode::Mapping(next)) if !next.is_flow_style() && level < self.players => {
                let inner = key_indent(self.text, next);
                let first = next.entries().next().and_then(|e| e.value_node());
                let lines: Vec<String> = self.strategies[level]
                    .iter()
                    .map(|s| self.zero_entry(s, inner, first.as_ref(), level + 1))
                    .collect();
                format!("{pad}{name}:\n{}", lines.join("\n"))
            },
            _ => format!("{pad}{name}: {}", self.flow_zero(level)),
        }
    }

    /// The changes to `node`, the payoffs at `level` (a mapping of that
    /// player's strategies, or a cell after the last player); `holder` is
    /// where the key holding it ends
    fn walk(
        &self,
        node: Option<&YamlNode>,
        level: usize,
        holder: usize,
        edits: &mut Vec<Edit>,
    ) -> Result<(), String> {
        if level == self.players {
            return self.cell(node, edits);
        }
        let Some(YamlNode::Mapping(map)) = node else {
            return Err(not_written(&self.at));
        };
        if *self.plan == (Plan::DeletePlayer { index: level }) {
            return self.keep_first(map, level, holder, edits);
        }
        let entries: Vec<MappingEntry> = map.entries().collect();
        for entry in &entries {
            let key = key_of(entry);
            if let Plan::DeleteStrategy { strategy, player } = self.plan {
                if *strategy == key && player.is_none_or(|p| p == level) {
                    edits.extend(remove_entry(self.text, map, &key));
                    continue;
                }
            }
            let value = entry.value_node();
            self.walk(value.as_ref(), level + 1, key_end(entry), edits)?;
        }
        if let Plan::AddStrategy { strategy, player } = self.plan {
            if player.is_none_or(|p| p == level) {
                let first = entries.first().and_then(MappingEntry::value_node);
                let pad = key_indent(self.text, map);
                let block = self.zero_entry(strategy, pad, first.as_ref(), level + 1);
                let flow = format!("{strategy}: {}", self.flow_zero(level + 1));
                edits.extend(push_entry(self.text, map, &flow, &block));
            }
        }
        Ok(())
    }

    /// Why a cell is not changed: it is not written on one line
    fn one_line(&self) -> String {
        format!(
            "{}: edit changes a cell written [1, 2] on one line",
            self.at
        )
    }

    /// A cell: a new player's level under it, or a deleted player's payoff
    /// out of it
    fn cell(&self, node: Option<&YamlNode>, edits: &mut Vec<Edit>) -> Result<(), String> {
        let cell = || self.one_line();
        match (self.plan, node) {
            (Plan::AddPlayer { .. }, Some(node @ YamlNode::Sequence(items)))
                if items.is_flow_style() =>
            {
                let range = span(node);
                let last = items.values().last().ok_or_else(cell)?;
                let end = span(&last).end - range.start;
                let text = &self.text[range.clone()];
                let grown = format!("{}, 0{}", &text[..end], &text[end..]);
                let entries: Vec<String> = self.strategies[self.players]
                    .iter()
                    .map(|s| format!("{s}: {grown}"))
                    .collect();
                edits.push((range, format!("{{{}}}", entries.join(", "))));
            },
            (Plan::DeletePlayer { index }, Some(YamlNode::Sequence(items))) => {
                edits.push(remove_item(self.text, items, *index).ok_or_else(cell)?);
            },
            (Plan::AddPlayer { .. } | Plan::DeletePlayer { .. }, _) => return Err(cell()),
            _ => {},
        }
        Ok(())
    }

    /// The deleted player's level `map` written as its first strategy's
    /// payoffs, a level higher: in place of a flow mapping, after the key
    /// that holds a block one (its comment kept), or its lines moved left
    fn keep_first(
        &self,
        map: &Mapping,
        level: usize,
        holder: usize,
        edits: &mut Vec<Edit>,
    ) -> Result<(), String> {
        let first = map.entries().next();
        let Some(kept) = first.as_ref().and_then(MappingEntry::value_node) else {
            return Err(not_written(&self.at));
        };
        let mut inner = Vec::new();
        let under = first.as_ref().map_or(0, key_end);
        self.walk(Some(&kept), level + 1, under, &mut inner)?;
        let range = span(&YamlNode::Mapping(map.clone()));
        let text = self.text;
        let from = |at: usize| {
            inner
                .iter()
                .map(|(r, new)| (r.start - at..r.end - at, new.clone()))
                .collect()
        };
        let written = |r: Range<usize>| splice(&text[r.clone()], from(r.start));
        match &kept {
            _ if map.is_flow_style() => edits.push((range, written(span(&kept)))),
            YamlNode::Mapping(lower) if !lower.is_flow_style() => {
                let lines = line_start(text, span(&kept).start)..span(&kept).end;
                let cut = key_indent(text, lower)
                    .len()
                    .saturating_sub(key_indent(text, map).len());
                let moved: String = written(lines)
                    .split_inclusive('\n')
                    .map(|line| {
                        let spaces = line.len() - line.trim_start_matches(' ').len();
                        &line[spaces.min(cut)..]
                    })
                    .collect();
                edits.push((line_start(text, range.start)..range.end, moved));
            },
            _ if written(span(&kept)).contains('\n') => return Err(self.one_line()),
            _ => {
                let colon = holder + text[holder..].find(':').unwrap_or_default() + 1;
                let comment = &text[colon..line_end(text, colon)];
                edits.push((colon..range.end, {
                    let comment = comment.trim_end_matches('\n');
                    let tail = if text[..range.end].ends_with('\n') {
                        "\n"
                    } else {
                        ""
                    };
                    format!(" {}{comment}{tail}", written(span(&kept)))
                }));
            },
        }
        Ok(())
    }
}

/// The new text of the file holding game `name`, with the change made in
/// it.
///
/// # Errors
///
/// Returns why, when a place the change makes is not written where edit can
/// change it, or one player's strategy is named in a list the players share.
pub(super) fn write(text: &str, name: &str, changed: &Changed<'_>) -> Result<String, String> {
    let file = parse(text)?;
    let top = file
        .document()
        .and_then(|d| d.as_mapping())
        .ok_or("it is not a mapping of names")?;
    let at = format!("games.{name}");
    let map = mapping(&mapping(&top, "games", "games")?, name, &at)?;
    let edits = match changed {
        Changed::Tournament(_, entrants) => {
            let players = mapping(
                &mapping(&map, "tournament", &format!("{at}.tournament"))?,
                "players",
                &format!("{at}.tournament.players"),
            )?;
            match entrants {
                Entrants::Add { entrant, plays } => {
                    let pad = key_indent(text, &players);
                    let entry = format!("{entrant}: {}", plays.name());
                    push_entry(text, &players, &entry, &format!("{pad}{entry}"))
                },
                Entrants::Delete { entrant } => remove_entry(text, &players, entrant),
            }
            .into_iter()
            .collect()
        },
        Changed::Table(table, plan) => table_edits(text, &map, &at, table, plan)?,
    };
    Ok(splice(text, edits))
}

/// The changes to a game played once: its lists of players and strategies,
/// its payoffs, and the start of its dynamics
fn table_edits(
    text: &str,
    map: &Mapping,
    at: &str,
    game: &TableGame,
    plan: &Plan,
) -> Result<Vec<Edit>, String> {
    let mut edits = Vec::new();
    let players = list(map, "players", &format!("{at}.players"))?;
    match plan {
        Plan::AddPlayer { player } => edits.extend(push_item(text, &players, player)),
        Plan::DeletePlayer { index } => edits.extend(remove_item(text, &players, *index)),
        _ => {},
    }
    let strategies = super::after(game, plan).strategies;
    edits.extend(strategy_edits(text, map, at, game, plan)?);
    let counted = game
        .cells
        .first()
        .and_then(|cells| cells.first())
        .is_some_and(|cell| !cell.others.is_empty());
    let payoffs = map
        .find_entry_by_key("payoffs")
        .and_then(|entry| match entry.value_node() {
            Some(YamlNode::Mapping(root)) => Some((key_end(&entry), root)),
            _ => None,
        });
    let Some((holder, root)) = payoffs else {
        return Err(not_written(&format!("{at}.payoffs")));
    };
    match plan {
        Plan::AddStrategy { strategy, .. } | Plan::DeleteStrategy { strategy, .. } if counted => {
            edits.extend(start_edit(text, &root, strategy, plan));
        },
        Plan::AddPlayer { .. } | Plan::DeletePlayer { .. } if game.symmetric => {},
        _ => Payoffs {
            text,
            plan,
            players: game.players.len(),
            strategies: &strategies,
            symmetric: game.symmetric,
            at: format!("{at}.payoffs"),
        }
        .walk(Some(&YamlNode::Mapping(root)), 0, holder, &mut edits)?,
    }
    if let (Some(_), Plan::AddStrategy { strategy, .. } | Plan::DeleteStrategy { strategy, .. }) =
        (&game.dynamics, plan)
    {
        let start = mapping(
            &mapping(map, "dynamics", &format!("{at}.dynamics"))?,
            "start",
            &format!("{at}.dynamics.start"),
        )?;
        edits.extend(start_edit(text, &start, strategy, plan));
    }
    Ok(edits)
}

/// `strategy: 0` added to a mapping of strategies, or its entry taken out
fn start_edit(text: &str, map: &Mapping, strategy: &str, plan: &Plan) -> Option<Edit> {
    if matches!(plan, Plan::AddStrategy { .. }) {
        let entry = format!("{strategy}: 0");
        push_entry(
            text,
            map,
            &entry,
            &format!("{}{entry}", key_indent(text, map)),
        )
    } else {
        remove_entry(text, map, strategy)
    }
}

/// The changes to `strategies:`: one list the players share, or a list for
/// each player by name
fn strategy_edits(
    text: &str,
    map: &Mapping,
    at: &str,
    game: &TableGame,
    plan: &Plan,
) -> Result<Vec<Edit>, String> {
    let mut edits = Vec::new();
    match map.get("strategies") {
        Some(YamlNode::Sequence(shared)) => match plan {
            Plan::AddStrategy {
                player: Some(p),
                strategy,
            }
            | Plan::DeleteStrategy {
                player: Some(p),
                strategy,
            } => {
                return Err(format!(
                    "the players of {name} share one list of strategies: name {strategy} for \
                     all of them, as {name}.strategies.{strategy}, not {player}'s alone",
                    name = game.name,
                    player = game.players[*p],
                ))
            },
            Plan::AddStrategy { strategy, .. } => edits.extend(push_item(text, &shared, strategy)),
            Plan::DeleteStrategy { strategy, .. } => {
                let index = game.strategies[0].iter().position(|s| s == strategy);
                edits.extend(index.and_then(|i| remove_item(text, &shared, i)));
            },
            _ => {},
        },
        Some(YamlNode::Mapping(lists)) => {
            for (i, player) in game.players.iter().enumerate() {
                let own = || list(&lists, player, &format!("{at}.strategies.{player}"));
                match plan {
                    Plan::AddStrategy { strategy, player } if player.is_none_or(|p| p == i) => {
                        edits.extend(push_item(text, &own()?, strategy));
                    },
                    Plan::DeleteStrategy { strategy, player } if player.is_none_or(|p| p == i) => {
                        let index = game.strategies[i].iter().position(|s| s == strategy);
                        edits.extend(index.and_then(|s| remove_item(text, &own().ok()?, s)));
                    },
                    Plan::DeletePlayer { index } if *index == i => {
                        edits.extend(remove_entry(text, &lists, player));
                    },
                    _ => {},
                }
            }
            if let Plan::AddPlayer { player } = plan {
                edits.extend(copy_last_list(text, &lists, player));
            }
        },
        _ => return Err(not_written(&format!("{at}.strategies"))),
    }
    Ok(edits)
}

/// A new player's list of strategies: the last player's, copied
fn copy_last_list(text: &str, lists: &Mapping, player: &str) -> Option<Edit> {
    let last = lists.entries().last().and_then(|e| e.value_node())?;
    let pad = key_indent(text, lists);
    let flow = format!("{player}: {}", &text[span(&last)]);
    let block = match &last {
        YamlNode::Sequence(items) if !items.is_flow_style() => {
            let lines: Vec<String> = items
                .values()
                .map(|item| {
                    let range = span(&item);
                    format!("{}{}", indent(text, range.start), &text[range])
                })
                .collect();
            format!("{pad}{player}:\n{}", lines.join("\n"))
        },
        _ => format!("{pad}{flow}"),
    };
    push_entry(text, lists, &flow, &block)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{game_change, TRIO};
    use super::super::{plan_of, Changed, Plan};
    use super::write;
    use crate::edit::{check, Op};
    use crate::types::ParsedModel;
    use std::collections::HashMap;

    fn model(text: &str) -> ParsedModel {
        check::read(
            &HashMap::from([("m.yaml".to_string(), text.to_string())]),
            "m.yaml",
        )
        .unwrap()
    }

    /// `games:` with `game` under it, as the text of a file
    fn games(game: &str) -> String {
        format!("games:\n  {game}")
    }

    /// Text that is not what Forge read is refused at the first place edit
    /// cannot change, or left as it is where there is nothing to change
    #[test]
    fn a_text_unlike_the_game_is_refused_where_edit_cannot_change_it() {
        let trio = model(TRIO);
        let t = trio.games["trio"]
            .played_once()
            .expect("trio is a game played once");
        let at = |plan: Plan, text: &str| write(text, "trio", &Changed::Table(t, plan));
        let dee = || Plan::AddPlayer {
            player: "dee".into(),
        };
        let refused = |text: &str, why: &str| Err(why.to_string()) == at(dee(), text);
        let not_written = |place: &str| {
            format!("{place} is not written where edit can change it (an alias, or another shape)")
        };
        assert!(refused(
            "a: \"1\n",
            "its YAML cannot be read (Unterminated quoted string)"
        ));
        assert!(refused("- 1\n", "it is not a mapping of names"));
        assert!(refused("x: 1\n", &not_written("games")));
        assert!(refused("games:\n  other: 1\n", &not_written("games.trio")));
        assert!(refused(
            &games("trio: {strategies: [x]}\n"),
            &not_written("games.trio.players")
        ));
        assert!(refused(
            &games("trio: {players: [a], strategies: 1}\n"),
            &not_written("games.trio.strategies")
        ));
        let no_payoffs = not_written("games.trio.payoffs");
        assert!(refused(
            &games("trio: {players: [a], strategies: [x]}\n"),
            &no_payoffs
        ));
        assert!(refused(
            &games("trio: {players: [a], strategies: [x], payoffs: 1}\n"),
            &no_payoffs
        ));
        // An empty list and an empty mapping of lists are left; a level
        // that is not a mapping, and an empty cell, are refused
        let cell = "games.trio.payoffs: edit changes a cell written [1, 2] on one line";
        assert!(refused(
            &games("trio: {players: [], strategies: {}, payoffs: {up: {left: {near: []}}}}\n"),
            cell
        ));
        assert!(refused(
            &games("trio: {players: [a], strategies: [x], payoffs: {up: {left: 1}}}\n"),
            &no_payoffs
        ));
        assert!(refused(
            &games("trio:\n    players: [a]\n    strategies: [x]\n    payoffs:\n      up:\n        left:\n          near:\n            - 1\n"),
            cell,
        ));
        let gone = |index: usize, text: &str| at(Plan::DeletePlayer { index }, &games(text));
        assert_eq!(
            gone(
                2,
                "trio: {players: [a], strategies: {}, payoffs: {up: {left: {}}}}\n"
            ),
            Err(no_payoffs.clone())
        );
        assert_eq!(
            gone(
                0,
                "trio: {players: [a], strategies: [x], payoffs: {up: [1]}}\n"
            ),
            Err(no_payoffs)
        );
        assert_eq!(
            gone(
                2,
                "trio: {players: [a, b, c], strategies: [x], payoffs: {up: {left: {near: [1]}}}}\n"
            ),
            Err(cell.to_string())
        );
        // A cell written over lines, kept for the deleted player
        assert_eq!(
            gone(
                2,
                "trio:\n    players: [ann, bob, cy]\n    strategies: [x]\n    payoffs:\n      up:\n        left:\n          near:\n            - 1\n            - 2\n            - 3\n"
            ),
            Err(cell.to_string())
        );
        // A list of strategies that is not a list
        let lists = "trio: {players: [a, b, c], strategies: {ann: 1, bob: [x], cy: [x]}, payoffs: {up: 1}}\n";
        let wait = |player: Option<usize>| Plan::AddStrategy {
            strategy: "wait".into(),
            player,
        };
        assert_eq!(
            at(wait(None), &games(lists)),
            Err(not_written("games.trio.strategies.ann"))
        );
        let up = Plan::DeleteStrategy {
            strategy: "up".into(),
            player: None,
        };
        // ann's list is not one and up is the only key: nothing to take out
        assert_eq!(at(up, &games(lists)), Ok(games(lists)));
        // The last player's level at the end of a file without a last line break
        assert_eq!(
            gone(2, "trio:\n    players: [ann, bob, cy]\n    strategies: [x]\n    payoffs:\n      up:\n        left:\n          near: [1, 2, 3]\n          far: [4, 5, 6]"),
            Ok(games("trio:\n    players: [ann, bob]\n    strategies: [x]\n    payoffs:\n      up:\n        left: [1, 2]"))
        );
    }

    #[test]
    fn a_dynamics_or_a_tournament_unlike_the_text_is_refused_or_left() {
        let hawk = model(
            &std::fs::read_to_string(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("examples/games/hawk-dove.yaml"),
            )
            .unwrap(),
        );
        let t = hawk.games["hawk_dove"]
            .played_once()
            .expect("hawk_dove is a game played once");
        let plan = || {
            Changed::Table(
                t,
                Plan::AddStrategy {
                    strategy: "wait".into(),
                    player: None,
                },
            )
        };
        let game = |dynamics: &str| {
            games(&format!("hawk_dove:\n    players: [a, b]\n    strategies: [hawk, dove]\n    payoffs: {{hawk: {{hawk: 1, dove: 2}}, dove: {{hawk: 0, dove: 1}}}}\n    dynamics: {dynamics}\n"))
        };
        let not_written = |place: &str| {
            Err(format!(
                "{place} is not written where edit can change it (an alias, or another shape)"
            ))
        };
        assert_eq!(
            write(&game("1"), "hawk_dove", &plan()),
            not_written("games.hawk_dove.dynamics")
        );
        assert_eq!(
            write(&game("{start: 1}"), "hawk_dove", &plan()),
            not_written("games.hawk_dove.dynamics.start")
        );
        let duel = model(
            "_forge_version: \"5.6.0\"
games:
  pd:
    players: [me, you]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs: {cooperate: {cooperate: 3, defect: 0}, defect: {cooperate: 5, defect: 1}}
  duel:
    tournament: {round: pd, players: {a: tit_for_tat, b: always_defect, c: random}, rounds: 10}
",
        );
        let plan = |op: Op, path: &str, plays: Option<&str>| {
            plan_of(&duel, path, &game_change(op, path, plays))
                .unwrap()
                .1
        };
        let add = || plan(Op::AddPlayer, "duel.players.d", Some("random"));
        let delete = || plan(Op::DeletePlayer, "duel.players.a", None);
        let at = |changed: Changed<'_>, text: &str| write(text, "duel", &changed);
        assert_eq!(
            at(add(), &games("duel: {x: 1}\n")),
            not_written("games.duel.tournament")
        );
        assert_eq!(
            at(add(), &games("duel: {tournament: {round: pd}}\n")),
            not_written("games.duel.tournament.players")
        );
        // Nothing to add after, nothing to take out, or nothing after the first
        for (entrants, text) in [
            (add(), "duel: {tournament: {players: {}}}\n"),
            (delete(), "duel: {tournament: {players: {b: x}}}\n"),
            (delete(), "duel: {tournament: {players: {a: x}}}\n"),
        ] {
            assert_eq!(at(entrants, &games(text)), Ok(games(text)));
        }
    }
}
