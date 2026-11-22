//! A game as the model writes it, in `games:` (ADR-068, section 1).
//!
//! Players, named strategies and a payoff table whose cells are numbers or
//! formulas. Read from the YAML here; calculated into a [`Table`] by the
//! calculator.

use super::counts;
use super::dynamics::{self, Dynamics};
use super::table::Table;
use super::tournament::{self, Tournament};
use crate::error::{ForgeError, ForgeResult};
use yaml_serde::{Mapping, Value};

/// A table may have at most this many cells (profiles).
pub const MOST_CELLS: usize = 10_000;

/// Names a strategy may not have: `payoff` is the last part of a result
/// name (`<game>.equilibrium_1.<player>.payoff`), and `others` is the count
/// a payoff of the count form reads (`<game>.others.<strategy>`, ADR-073).
const RESERVED_STRATEGIES: [&str; 2] = ["payoff", "others"];

/// A payoff as written: a number, or a formula over the model.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Payoff {
    Number(f64),
    Formula(String),
}

/// One player's payoff in one cell, with where it is written, for messages.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cell {
    pub payoff: Payoff,
    /// `you's payoff in stag_hunt.payoffs.hare.stag`
    pub at: String,
    /// In the count form (ADR-073), the values its formula reads by name:
    /// how many of the other players play each strategy in this cell
    /// (`pg.others.contribute`). Empty in a table written out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub others: Vec<(String, f64)>,
}

/// A game in `games:`: played once, or a tournament of repeated play.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Game {
    Table(TableGame),
    Tournament(Tournament),
}

impl Game {
    /// The game's name in `games:`.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Table(game) => &game.name,
            Self::Tournament(tournament) => &tournament.name,
        }
    }

    /// Every formula the game reads, once each.
    #[must_use]
    pub fn formulas(&self) -> Vec<&str> {
        match self {
            Self::Table(game) => game.formulas(),
            Self::Tournament(tournament) => tournament.formulas(),
        }
    }

    /// The game played once, or `None` for a tournament.
    #[must_use]
    pub const fn played_once(&self) -> Option<&TableGame> {
        match self {
            Self::Table(game) => Some(game),
            Self::Tournament(_) => None,
        }
    }
}

/// A game played once.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TableGame {
    pub name: String,
    pub players: Vec<String>,
    /// Each player's strategies, in the order of `players`
    pub strategies: Vec<Vec<String>>,
    pub symmetric: bool,
    /// For each profile in table order, one cell per player
    pub cells: Vec<Vec<Cell>>,
    /// How the strategies' shares change from generation to generation, in
    /// a symmetric game (ADR-073)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamics: Option<Dynamics>,
}

impl TableGame {
    /// Every formula in the payoff table and the dynamics, once each.
    #[must_use]
    pub fn formulas(&self) -> Vec<&str> {
        let mut all: Vec<&str> = Vec::new();
        let dynamics = self.dynamics.iter().flat_map(Dynamics::formulas);
        for formula in self
            .cells
            .iter()
            .flatten()
            .filter_map(|cell| match &cell.payoff {
                Payoff::Formula(f) => Some(f.as_str()),
                Payoff::Number(_) => None,
            })
            .chain(dynamics)
        {
            if !all.contains(&formula) {
                all.push(formula);
            }
        }
        all
    }

    /// The table with every payoff worked out by `value`, which is given
    /// each formula, where it is written (`me's payoff in ...`) and the
    /// counts it reads in the count form; the first error stops it. A
    /// formula is worked out once for each set of counts it reads.
    ///
    /// # Errors
    ///
    /// Returns the error `value` gives for the first formula that fails.
    pub fn table<E>(
        &self,
        mut value: impl FnMut(&str, &str, &[(String, f64)]) -> Result<f64, E>,
    ) -> Result<Table, E> {
        let mut known: std::collections::HashMap<(&str, Vec<u64>), f64> =
            std::collections::HashMap::new();
        let mut payoffs = Vec::with_capacity(self.cells.len());
        for cells in &self.cells {
            let mut row = Vec::with_capacity(cells.len());
            for cell in cells {
                row.push(match &cell.payoff {
                    Payoff::Number(n) => *n,
                    Payoff::Formula(f) => {
                        let key = (
                            f.as_str(),
                            cell.others.iter().map(|o| o.1.to_bits()).collect(),
                        );
                        if let Some(n) = known.get(&key) {
                            *n
                        } else {
                            let n = value(f, &cell.at, &cell.others)?;
                            known.insert(key, n);
                            n
                        }
                    },
                });
            }
            payoffs.push(row);
        }
        Ok(Table {
            players: self.players.clone(),
            strategies: self.strategies.clone(),
            payoffs,
        })
    }
}

/// A name a player, strategy or game can have: a letter or `_`, then
/// letters, digits and `_`. It becomes part of a scalar's name.
pub(super) fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub(super) const fn error(message: String) -> ForgeError {
    ForgeError::Parse(message)
}

/// A list of distinct names, at `at`.
fn names(value: &Value, at: &str, what: &str) -> ForgeResult<Vec<String>> {
    let Value::Sequence(items) = value else {
        return Err(error(format!("'{at}' must be a list of {what}")));
    };
    let mut list: Vec<String> = Vec::new();
    for item in items {
        let name = item.as_str().filter(|n| is_name(n)).ok_or_else(|| {
            error(format!(
                "'{at}' has {}, which is not a name: a {what} is named with letters, \
                     digits and _, starting with a letter",
                describe(item)
            ))
        })?;
        if list.iter().any(|n| n == name) {
            return Err(error(format!("'{at}' names {name} twice")));
        }
        list.push(name.to_string());
    }
    if list.is_empty() {
        return Err(error(format!("'{at}' is empty")));
    }
    Ok(list)
}

/// A YAML value in a few words, for a message.
pub(super) fn describe(value: &Value) -> String {
    match value {
        Value::String(s) => format!("\"{s}\""),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "nothing".to_string(),
        Value::Sequence(_) => "a list".to_string(),
        Value::Mapping(_) => "a mapping".to_string(),
        Value::Tagged(_) => "a tagged value".to_string(),
    }
}

/// Read the `games:` section.
///
/// # Errors
///
/// Returns an error naming the first game, key, strategy or cell that cannot
/// be used.
pub fn parse_games(section: &Value) -> ForgeResult<Vec<Game>> {
    let Value::Mapping(map) = section else {
        return Err(error(
            "'games' must be a mapping of game names to games".to_string(),
        ));
    };
    let mut games = Vec::new();
    for (key, value) in map {
        let name = key.as_str().filter(|n| is_name(n)).ok_or_else(|| {
            error(format!(
                "games: {} is not a name for a game: letters, digits and _, \
                     starting with a letter",
                describe(key)
            ))
        })?;
        games.push(parse_game(name, value)?);
    }
    for game in &games {
        if let Game::Tournament(t) = game {
            tournament::check_round(t, &games)?;
        }
    }
    Ok(games)
}

/// Whether a `games:` section, in a file of a format before 5.5.0, looks
/// like one: a mapping of mappings that each have `players` or `tournament`.
#[must_use]
pub fn looks_like_games(section: &Value) -> bool {
    let Value::Mapping(map) = section else {
        return false;
    };
    !map.is_empty()
        && map.values().all(|game| {
            matches!(game, Value::Mapping(m) if m.contains_key("players") || m.contains_key("tournament"))
        })
}

fn parse_game(name: &str, value: &Value) -> ForgeResult<Game> {
    let Value::Mapping(map) = value else {
        return Err(error(format!(
            "games.{name} must be a mapping with players, strategies and payoffs, \
             or with a tournament"
        )));
    };
    if map.contains_key("tournament") {
        return tournament::parse(name, map).map(Game::Tournament);
    }
    parse_table_game(name, map).map(Game::Table)
}

fn parse_table_game(name: &str, map: &Mapping) -> ForgeResult<TableGame> {
    for key in map.keys() {
        let known = key.as_str().is_some_and(|k| {
            [
                "players",
                "strategies",
                "symmetric",
                "payoffs",
                "description",
                "dynamics",
            ]
            .contains(&k)
        });
        if !known {
            return Err(error(format!(
                "games.{name} has {}, which a game does not have: it has players, \
                 strategies, payoffs, and symmetric, dynamics or description if wanted \
                 (or a tournament)",
                describe(key)
            )));
        }
    }
    let field = |key: &str| {
        map.get(key)
            .ok_or_else(|| error(format!("games.{name} has no {key}")))
    };
    let players = names(
        field("players")?,
        &format!("games.{name}.players"),
        "player",
    )?;
    if players.len() < 2 {
        return Err(error(format!(
            "games.{name}.players has one player; a game has two or more"
        )));
    }
    let strategies = parse_strategies(name, field("strategies")?, &players)?;
    let symmetric = match map.get("symmetric") {
        None => false,
        Some(Value::Bool(b)) => *b,
        Some(other) => {
            return Err(error(format!(
                "games.{name}.symmetric must be true or false, not {}",
                describe(other)
            )))
        },
    };
    if symmetric && strategies.iter().any(|list| *list != strategies[0]) {
        return Err(error(format!(
            "games.{name} is symmetric, which needs every player to have the same strategies"
        )));
    }
    let count: usize = strategies.iter().map(Vec::len).product();
    if count > MOST_CELLS {
        return Err(error(format!(
            "games.{name} has {count} cells; a game may have at most {MOST_CELLS}"
        )));
    }
    let dynamics = map
        .get("dynamics")
        .map(|d| dynamics::parse(name, d, symmetric, &strategies[0]))
        .transpose()?;
    let mut game = TableGame {
        name: name.to_string(),
        players,
        strategies,
        symmetric,
        cells: Vec::with_capacity(count),
        dynamics,
    };
    let payoffs = field("payoffs")?;
    let counted = counts::count_form(payoffs).filter(|_| symmetric);
    game.cells = if let Some(map) = counted {
        counts::read(&game, map)?
    } else if symmetric && game.players.len() > 2 {
        return Err(counts::not_written_out(&game));
    } else if symmetric {
        mirror(&game, &read_cells(&game, payoffs)?)
    } else {
        read_cells(&game, payoffs)?
    };
    Ok(game)
}

/// One list for every player, or a list for each player by name.
fn parse_strategies(
    name: &str,
    value: &Value,
    players: &[String],
) -> ForgeResult<Vec<Vec<String>>> {
    let at = format!("games.{name}.strategies");
    let lists = match value {
        Value::Sequence(_) => vec![names(value, &at, "strategy")?; players.len()],
        Value::Mapping(map) => {
            for key in map.keys() {
                if !key.as_str().is_some_and(|k| players.iter().any(|p| p == k)) {
                    return Err(error(format!(
                        "'{at}' has {}, which is not one of the players ({})",
                        describe(key),
                        players.join(", ")
                    )));
                }
            }
            players
                .iter()
                .map(|player| {
                    let list = map
                        .get(player.as_str())
                        .ok_or_else(|| error(format!("'{at}' has no strategies for {player}")))?;
                    names(list, &format!("{at}.{player}"), "strategy")
                })
                .collect::<ForgeResult<Vec<_>>>()?
        },
        other => {
            return Err(error(format!(
                "'{at}' must be a list of strategies or a mapping of players to lists, not {}",
                describe(other)
            )))
        },
    };
    for (player, list) in players.iter().zip(&lists) {
        if let Some(reserved) = list
            .iter()
            .find(|s| RESERVED_STRATEGIES.contains(&s.as_str()))
        {
            let use_ = if reserved == "payoff" {
                format!("results use (<game>.equilibrium_1.{player}.payoff)")
            } else {
                "a payoff reads (<game>.others.<strategy>)".to_string()
            };
            return Err(error(format!(
                "'{at}': {player} has a strategy named {reserved}, a name {use_}; rename it"
            )));
        }
    }
    Ok(lists)
}

/// Every cell of `payoffs`, nested one level per player, in table order.
/// In a symmetric game a cell has the first player's payoff only.
fn read_cells(game: &TableGame, payoffs: &Value) -> ForgeResult<Vec<Vec<Cell>>> {
    let mut cells = Vec::new();
    let path = format!("games.{}.payoffs", game.name);
    read_level(game, payoffs, 0, &path, &mut cells)?;
    Ok(cells)
}

fn read_level(
    game: &TableGame,
    value: &Value,
    player: usize,
    path: &str,
    cells: &mut Vec<Vec<Cell>>,
) -> ForgeResult<()> {
    if player == game.players.len() {
        cells.push(read_leaf(game, value, path)?);
        return Ok(());
    }
    let Value::Mapping(map) = value else {
        return Err(error(format!(
            "'{path}' must be a mapping from {}'s strategies ({}), not {}",
            game.players[player],
            game.strategies[player].join(", "),
            describe(value)
        )));
    };
    check_keys(map, &game.strategies[player], &game.players[player], path)?;
    for strategy in &game.strategies[player] {
        let next = &map[strategy.as_str()];
        read_level(game, next, player + 1, &format!("{path}.{strategy}"), cells)?;
    }
    Ok(())
}

/// Each of `strategies` is a key of `map`, and nothing else is.
pub(super) fn check_keys(
    map: &Mapping,
    strategies: &[String],
    player: &str,
    path: &str,
) -> ForgeResult<()> {
    for key in map.keys() {
        if !key
            .as_str()
            .is_some_and(|k| strategies.iter().any(|s| s == k))
        {
            return Err(error(format!(
                "'{path}' has {}, which is not a strategy of {player} ({})",
                describe(key),
                strategies.join(", ")
            )));
        }
    }
    if let Some(missing) = strategies.iter().find(|s| !map.contains_key(s.as_str())) {
        return Err(error(format!(
            "'{path}' has no entry for {player}'s {missing}"
        )));
    }
    Ok(())
}

/// The payoffs of one cell: one value per player, or one value in a
/// symmetric game.
fn read_leaf(game: &TableGame, value: &Value, path: &str) -> ForgeResult<Vec<Cell>> {
    let wanted = if game.symmetric {
        1
    } else {
        game.players.len()
    };
    let values: Vec<&Value> = match value {
        Value::Sequence(items) if !game.symmetric => items.iter().collect(),
        Value::Sequence(_) => {
            return Err(error(format!(
                "'{path}' is a list; in a symmetric game a cell is one payoff, the first \
                 player's"
            )))
        },
        one if game.symmetric => vec![one],
        other => {
            return Err(error(format!(
                "'{path}' must be a list of {wanted} payoffs, one for each of {}, not {}",
                game.players.join(", "),
                describe(other)
            )))
        },
    };
    if values.len() != wanted {
        return Err(error(format!(
            "'{path}' has {} payoffs; it needs {wanted}, one for each of {}",
            values.len(),
            game.players.join(", ")
        )));
    }
    values
        .iter()
        .zip(&game.players)
        .map(|(value, player)| {
            let at = format!("{player}'s payoff in {path}");
            let payoff = number_or_formula(value).ok_or_else(|| {
                error(format!(
                    "{at} is {}: a payoff is a number or a formula starting with =",
                    describe(value)
                ))
            })?;
            Ok(Cell {
                payoff,
                at,
                others: Vec::new(),
            })
        })
        .collect()
}

/// A number, or a formula: a text starting with `=`.
pub(super) fn number_or_formula(value: &Value) -> Option<Payoff> {
    match value {
        Value::Number(n) => n.as_f64().map(Payoff::Number),
        Value::String(s) if s.trim_start().starts_with('=') => {
            Some(Payoff::Formula(s.trim().to_string()))
        },
        _ => None,
    }
}

/// A symmetric game's cells with the second player's payoff added: in cell
/// (i, j) it is the first player's payoff in cell (j, i), and a message
/// about it names that cell, where its formula is written.
fn mirror(game: &TableGame, written: &[Vec<Cell>]) -> Vec<Vec<Cell>> {
    let n = game.strategies[0].len();
    let mut cells = written.to_vec();
    for i in 0..n {
        for j in 0..n {
            let source = &written[j * n + i][0];
            cells[i * n + j].push(Cell {
                payoff: source.payoff.clone(),
                at: source.at.clone(),
                others: Vec::new(),
            });
        }
    }
    cells
}

#[cfg(test)]
mod tests {
    use super::*;

    fn games(yaml: &str) -> ForgeResult<Vec<Game>> {
        parse_games(&yaml_serde::from_str(yaml).unwrap())
    }

    fn one(yaml: &str) -> TableGame {
        games(yaml).unwrap()[0].played_once().unwrap().clone()
    }

    fn fails(yaml: &str) -> String {
        games(yaml).unwrap_err().to_string()
    }

    const STAG_HUNT: &str = "
stag_hunt:
  players: [me, you]
  strategies: [stag, hare]
  symmetric: true
  payoffs:
    stag: {stag: 4, hare: 0}
    hare: {stag: =3 - c, hare: =3 - c}
";

    #[test]
    fn a_symmetric_game_mirrors_the_first_players_payoffs() {
        let game = one(STAG_HUNT);
        assert_eq!(game.players, vec!["me", "you"]);
        assert_eq!(game.strategies, vec![vec!["stag", "hare"]; 2]);
        assert!(game.symmetric);
        // Cell (stag, hare): me 0; you get what me gets in (hare, stag)
        assert_eq!(game.cells[1][0].payoff, Payoff::Number(0.0));
        assert_eq!(game.cells[1][1].payoff, Payoff::Formula("=3 - c".into()));
        assert_eq!(
            game.cells[1][1].at,
            "me's payoff in games.stag_hunt.payoffs.hare.stag"
        );
        assert_eq!(game.formulas(), vec!["=3 - c"]);
        let table = game
            .table(|f, _, _| {
                assert_eq!(f, "=3 - c");
                Ok::<f64, String>(2.5)
            })
            .unwrap();
        assert_eq!(
            table.payoffs,
            vec![
                vec![4.0, 4.0],
                vec![0.0, 2.5],
                vec![2.5, 0.0],
                vec![2.5, 2.5]
            ]
        );
    }

    #[test]
    fn a_formula_that_fails_names_its_cell() {
        let err = one(STAG_HUNT)
            .table(|_, at, _| Err::<f64, String>(format!("{at}: Unknown variable: c")))
            .unwrap_err();
        assert_eq!(
            err,
            "me's payoff in games.stag_hunt.payoffs.hare.stag: Unknown variable: c"
        );
    }

    #[test]
    fn each_player_can_have_their_own_strategies() {
        let game = one("
inspection:
  description: An AI complies or cheats; a human audits or trusts
  players: [ai, human]
  strategies:
    ai: [comply, cheat]
    human: [audit, trust]
  payoffs:
    comply: {audit: [0, =-audit_cost], trust: [0, 0]}
    cheat: {audit: [=-penalty, =-audit_cost], trust: [=gain, =-harm]}
");
        assert_eq!(game.strategies[1], vec!["audit", "trust"]);
        assert!(!game.symmetric);
        assert_eq!(game.cells[2][0].payoff, Payoff::Formula("=-penalty".into()));
        assert_eq!(
            game.cells[2][0].at,
            "ai's payoff in games.inspection.payoffs.cheat.audit"
        );
        assert_eq!(
            game.formulas(),
            vec!["=-audit_cost", "=-penalty", "=gain", "=-harm"]
        );
    }

    #[test]
    fn three_players_nest_three_levels() {
        let game = one("
g:
  players: [a, b, c]
  strategies: [x, y]
  payoffs:
    x: {x: {x: [1, 1, 1], y: [0, 0, 0]}, y: {x: [0, 0, 0], y: [0, 0, 0]}}
    y: {x: {x: [0, 0, 0], y: [0, 0, 0]}, y: {x: [0, 0, 0], y: [2, 2, 2]}}
");
        assert_eq!(game.cells.len(), 8);
        assert_eq!(game.cells[7][2].payoff, Payoff::Number(2.0));
        assert_eq!(game.cells[7][2].at, "c's payoff in games.g.payoffs.y.y.y");
    }

    #[test]
    fn games_that_cannot_be_used_are_named() {
        let cases = [
            ("[1]", "'games' must be a mapping of game names to games"),
            ("1x: {}", "games: \"1x\" is not a name for a game"),
            (
                "g: 3",
                "games.g must be a mapping with players, strategies and payoffs, or with a \
                 tournament",
            ),
            (
                "g: {players: [a, b], rules: 1}",
                "games.g has \"rules\", which a game does not have: it has players, \
                 strategies, payoffs, and symmetric, dynamics or description if wanted (or a \
                 tournament)",
            ),
            ("g: {strategies: [x]}", "games.g has no players"),
            (
                "g: {players: a}",
                "'games.g.players' must be a list of player",
            ),
            ("g: {players: [a, a]}", "'games.g.players' names a twice"),
            (
                "g: {players: [a, b.c]}",
                "'games.g.players' has \"b.c\", which is not a name",
            ),
            ("g: {players: []}", "'games.g.players' is empty"),
            (
                "g: {players: [a]}",
                "games.g.players has one player; a game has two or more",
            ),
        ];
        for (yaml, expected) in cases {
            let err = fails(yaml);
            assert!(err.contains(expected), "{yaml}\n{err}");
        }
    }

    #[test]
    fn strategies_and_cells_that_cannot_be_used_are_named() {
        let cases = [
            ("g: {players: [a, b]}", "games.g has no strategies"),
            (
                "g: {players: [a, b], strategies: [x, x]}",
                "'games.g.strategies' names x twice",
            ),
            (
                "g: {players: [a, b], strategies: 1}",
                "'games.g.strategies' must be a list of strategies or a mapping",
            ),
            (
                "g: {players: [a, b], strategies: {a: [x], c: [y]}}",
                "'games.g.strategies' has \"c\", which is not one of the players (a, b)",
            ),
            (
                "g: {players: [a, b], strategies: {a: [x]}}",
                "'games.g.strategies' has no strategies for b",
            ),
            (
                "g: {players: [a, b], strategies: [x, payoff]}",
                "a has a strategy named payoff",
            ),
            (
                "g: {players: [a, b], strategies: [x], symmetric: yes please}",
                "games.g.symmetric must be true or false, not \"yes please\"",
            ),
            (
                "g: {players: [a, b], strategies: {a: [x], b: [y]}, symmetric: true}",
                "games.g is symmetric, which needs every player to have the same strategies",
            ),
            (
                "g: {players: [a, b], strategies: [x]}",
                "games.g has no payoffs",
            ),
            (
                "g: {players: [a, b], strategies: [x], payoffs: [1]}",
                "'games.g.payoffs' must be a mapping from a's strategies (x), not a list",
            ),
            (
                "g: {players: [a, b], strategies: [x, y], payoffs: {x: {x: [1, 1], y: [1, 1]}}}",
                "'games.g.payoffs' has no entry for a's y",
            ),
            (
                "g: {players: [a, b], strategies: [x], payoffs: {x: {z: [1, 1]}}}",
                "'games.g.payoffs.x' has \"z\", which is not a strategy of b (x)",
            ),
            (
                "g: {players: [a, b], strategies: [x], payoffs: {x: {x: 1}}}",
                "'games.g.payoffs.x.x' must be a list of 2 payoffs, one for each of a, b, not 1",
            ),
            (
                "g: {players: [a, b], strategies: [x], payoffs: {x: {x: [1]}}}",
                "'games.g.payoffs.x.x' has 1 payoffs; it needs 2",
            ),
            (
                "g: {players: [a, b], strategies: [x], symmetric: true, payoffs: {x: {x: [1, 1]}}}",
                "'games.g.payoffs.x.x' is a list; in a symmetric game a cell is one payoff",
            ),
            (
                "g: {players: [a, b], strategies: [x], payoffs: {x: {x: [1, three]}}}",
                "b's payoff in games.g.payoffs.x.x is \"three\": a payoff is a number or a formula",
            ),
            (
                "g: {players: [a, b], strategies: [x], payoffs: {x: {x: [1, true]}}}",
                "b's payoff in games.g.payoffs.x.x is true",
            ),
            (
                "g: {players: [a, b], strategies: [x], payoffs: {x: {x: [1, null]}}}",
                "is nothing",
            ),
            (
                "g: {players: [a, b], strategies: [x], payoffs: {x: {x: [1, {k: 1}]}}}",
                "is a mapping",
            ),
            (
                "g: {players: [a, b], strategies: [x], payoffs: {x: {x: [1, !t 1]}}}",
                "is a tagged value",
            ),
        ];
        for (yaml, expected) in cases {
            let err = fails(yaml);
            assert!(err.contains(expected), "{yaml}\n{err}");
        }
    }

    #[test]
    fn a_game_may_have_at_most_ten_thousand_cells() {
        let strategies: Vec<String> = (0..101).map(|i| format!("s{i}")).collect();
        let yaml = format!(
            "g: {{players: [a, b], strategies: [{}], payoffs: {{}}}}",
            strategies.join(", ")
        );
        assert!(fails(&yaml).contains("games.g has 10201 cells; a game may have at most 10000"));
    }

    #[test]
    fn a_section_before_5_5_is_recognised_as_games() {
        let yaml = |text: &str| -> Value { yaml_serde::from_str(text).unwrap() };
        assert!(looks_like_games(&yaml(STAG_HUNT)));
        assert!(looks_like_games(&yaml("t: {tournament: {}}")));
        assert!(!looks_like_games(&yaml("price: 3")));
        assert!(!looks_like_games(&yaml("{}")));
        assert!(!looks_like_games(&yaml("[1]")));
    }
}
