//! A tournament as the model writes it (ADR-068, section 1): built-in
//! strategies playing a symmetric game of two strategies, round after
//! round, each against every other.
//!
//! ```yaml
//! axelrod:
//!   tournament:
//!     round: dilemma                # a symmetric game in the same section
//!     players:
//!       tft: tit_for_tat
//!       rand: {plays: random, cooperate: 0.5}
//!     rounds: 200                   # or continuation: 0.99
//!     noise: 0
//!     self_play: true
//! ```

use super::model::{describe, error, is_name, number_or_formula, Cell, Game, Payoff};
use super::repeated::Length;
use super::strategy::{Machine, Plays};
use crate::error::{ForgeError, ForgeResult};
use num_traits::ToPrimitive;
use yaml_serde::{Mapping, Value};

/// A match may have at most this many rounds.
pub const MOST_ROUNDS: u64 = 100_000;

/// The largest continuation: a match of 10,000 rounds on average.
pub const MOST_CONTINUATION: f64 = 0.9999;

/// A player of a tournament.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Entrant {
    pub name: String,
    pub plays: Plays,
    /// The strategy's parameter (`forgive`, `cooperate`), if it takes one
    /// and the model gives it
    pub chance: Option<Cell>,
}

/// How a match ends, as written.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Ends {
    Rounds(Cell),
    Continuation(Cell),
}

/// A tournament.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Tournament {
    pub name: String,
    /// The game played in each round
    pub round: String,
    pub entrants: Vec<Entrant>,
    pub ends: Ends,
    /// The chance that a move is flipped
    pub noise: Cell,
    /// Whether each player also plays a copy of itself
    pub self_play: bool,
}

/// A tournament's numbers, worked out.
#[derive(Debug, Clone)]
pub struct Worked {
    pub length: Length,
    pub noise: f64,
    /// Each entrant's machine, in the order of `entrants`
    pub machines: Vec<Machine>,
}

impl Tournament {
    /// Every number written as a formula: the length, the noise and the
    /// strategies' parameters, once each.
    #[must_use]
    pub fn formulas(&self) -> Vec<&str> {
        let (Ends::Rounds(length) | Ends::Continuation(length)) = &self.ends;
        let mut all: Vec<&str> = Vec::new();
        let cells = [length, &self.noise]
            .into_iter()
            .chain(self.entrants.iter().filter_map(|e| e.chance.as_ref()));
        for cell in cells {
            if let Payoff::Formula(f) = &cell.payoff {
                if !all.contains(&f.as_str()) {
                    all.push(f);
                }
            }
        }
        all
    }

    /// The length, noise and machines, with every formula worked out by
    /// `value`, which is given the formula and where it is written.
    ///
    /// # Errors
    ///
    /// Returns the error `value` gives for the first formula that fails, or
    /// one naming a number out of its range: rounds a whole number from 1 to
    /// [`MOST_ROUNDS`], continuation from 0 to [`MOST_CONTINUATION`], and
    /// noise and the parameters chances from 0 to 1.
    pub fn work_out(
        &self,
        mut value: impl FnMut(&str, &str) -> ForgeResult<f64>,
    ) -> ForgeResult<Worked> {
        let mut number = |cell: &Cell| match &cell.payoff {
            Payoff::Number(n) => Ok(*n),
            Payoff::Formula(f) => value(f, &cell.at),
        };
        let out_of_range = |cell: &Cell, got: f64, rule: &str| {
            ForgeError::Eval(format!(
                "game '{}': {} is {got}; {rule}",
                self.name, cell.at
            ))
        };
        let chance = |cell: &Cell, got: f64| {
            if (0.0..=1.0).contains(&got) {
                Ok(got)
            } else {
                Err(out_of_range(cell, got, "it is a chance, from 0 to 1"))
            }
        };
        let length = match &self.ends {
            Ends::Rounds(cell) => {
                let got = number(cell)?;
                let rounds = got
                    .to_u64()
                    .filter(|r| got.fract() == 0.0 && (1..=MOST_ROUNDS).contains(r));
                Length::Rounds(rounds.ok_or_else(|| {
                    out_of_range(
                        cell,
                        got,
                        &format!("it is a whole number of rounds, from 1 to {MOST_ROUNDS}"),
                    )
                })?)
            },
            Ends::Continuation(cell) => {
                let got = number(cell)?;
                if !(0.0..=MOST_CONTINUATION).contains(&got) {
                    return Err(out_of_range(
                        cell,
                        got,
                        &format!(
                            "it is the chance that a match goes on after a round, from 0 \
                             to {MOST_CONTINUATION}"
                        ),
                    ));
                }
                Length::Continuation(got)
            },
        };
        let noise = chance(&self.noise, number(&self.noise)?)?;
        let mut machines = Vec::with_capacity(self.entrants.len());
        for entrant in &self.entrants {
            let given = match &entrant.chance {
                Some(cell) => chance(cell, number(cell)?)?,
                None => entrant
                    .plays
                    .parameter()
                    .map_or(0.0, |(_, default)| default),
            };
            machines.push(entrant.plays.machine(given));
        }
        Ok(Worked {
            length,
            noise,
            machines,
        })
    }
}

const KEYS: [&str; 6] = [
    "round",
    "players",
    "rounds",
    "continuation",
    "noise",
    "self_play",
];

/// Read game `name`, a mapping with `tournament`.
pub(super) fn parse(name: &str, game: &Mapping) -> ForgeResult<Tournament> {
    if let Some(key) = game.keys().find(|k| {
        !k.as_str()
            .is_some_and(|k| k == "tournament" || k == "description")
    }) {
        return Err(error(format!(
            "games.{name} has {} beside its tournament: a tournament is a game of its own, \
             with only a description beside it",
            describe(key)
        )));
    }
    let at = format!("games.{name}.tournament");
    let Value::Mapping(map) = &game["tournament"] else {
        return Err(error(format!(
            "'{at}' must be a mapping with round, players, and rounds or continuation"
        )));
    };
    if let Some(key) = map
        .keys()
        .find(|k| !k.as_str().is_some_and(|k| KEYS.contains(&k)))
    {
        return Err(error(format!(
            "'{at}' has {}, which a tournament does not have: it has {}",
            describe(key),
            KEYS.join(", ")
        )));
    }
    let round = match map.get("round") {
        Some(Value::String(round)) => round.clone(),
        Some(other) => {
            return Err(error(format!(
                "'{at}.round' must name a game, not {}",
                describe(other)
            )))
        },
        None => {
            return Err(error(format!(
                "'{at}' has no round: the game played in each round"
            )))
        },
    };
    let setting = |key: &str, value: &Value| {
        let place = format!("{at}.{key}");
        number_or_formula(value)
            .map(|payoff| Cell {
                payoff,
                at: place.clone(),
            })
            .ok_or_else(|| {
                error(format!(
                    "'{place}' is {}: it is a number or a formula starting with =",
                    describe(value)
                ))
            })
    };
    let ends = match (map.get("rounds"), map.get("continuation")) {
        (Some(rounds), None) => Ends::Rounds(setting("rounds", rounds)?),
        (None, Some(w)) => Ends::Continuation(setting("continuation", w)?),
        (Some(_), Some(_)) => {
            return Err(error(format!(
                "'{at}' has rounds and continuation; a match has one or the other"
            )))
        },
        (None, None) => {
            return Err(error(format!(
                "'{at}' has neither rounds nor continuation: say how long a match is"
            )))
        },
    };
    let noise = map.get("noise").map_or_else(
        || {
            Ok(Cell {
                payoff: Payoff::Number(0.0),
                at: format!("{at}.noise"),
            })
        },
        |noise| setting("noise", noise),
    )?;
    let self_play = match map.get("self_play") {
        None => true,
        Some(Value::Bool(b)) => *b,
        Some(other) => {
            return Err(error(format!(
                "'{at}.self_play' must be true or false, not {}",
                describe(other)
            )))
        },
    };
    let entrants = parse_entrants(&at, map.get("players"))?;
    Ok(Tournament {
        name: name.to_string(),
        round,
        entrants,
        ends,
        noise,
        self_play,
    })
}

/// The list of built-in strategies, for a message.
fn known() -> String {
    Plays::ALL
        .iter()
        .map(|p| p.name())
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_entrants(at: &str, players: Option<&Value>) -> ForgeResult<Vec<Entrant>> {
    let Some(Value::Mapping(players)) = players else {
        return Err(error(format!(
            "'{at}.players' must be a mapping of each player's name to the strategy it \
             plays ({})",
            known()
        )));
    };
    let mut entrants = Vec::with_capacity(players.len());
    for (key, value) in players {
        let name = key.as_str().filter(|n| is_name(n)).ok_or_else(|| {
            error(format!(
                "'{at}.players' has {}, which is not a name: letters, digits and _, \
                 starting with a letter",
                describe(key)
            ))
        })?;
        entrants.push(parse_entrant(&format!("{at}.players.{name}"), name, value)?);
    }
    if entrants.len() < 2 {
        return Err(error(format!(
            "'{at}.players' has {} player; a tournament has two or more",
            entrants.len()
        )));
    }
    Ok(entrants)
}

fn parse_entrant(at: &str, name: &str, value: &Value) -> ForgeResult<Entrant> {
    let strategy = |value: &Value| {
        value.as_str().and_then(Plays::from_name).ok_or_else(|| {
            error(format!(
                "'{at}' plays {}, which is not a built-in strategy: {}",
                describe(value),
                known()
            ))
        })
    };
    let Value::Mapping(map) = value else {
        return Ok(Entrant {
            name: name.to_string(),
            plays: strategy(value)?,
            chance: None,
        });
    };
    let plays = strategy(
        map.get("plays")
            .ok_or_else(|| error(format!("'{at}' has no plays: the strategy it plays")))?,
    )?;
    let parameter = plays.parameter().map(|(key, _)| key);
    if let Some(key) = map
        .keys()
        .find(|k| k.as_str() != Some("plays") && k.as_str() != parameter)
    {
        let takes = parameter.map_or_else(
            || format!("{} takes no parameter", plays.name()),
            |p| format!("{} takes {p}", plays.name()),
        );
        return Err(error(format!(
            "'{at}' has {}: {takes} (generous_tit_for_tat takes forgive, random takes \
             cooperate)",
            describe(key)
        )));
    }
    let chance = match parameter.and_then(|key| map.get(key).map(|v| (key, v))) {
        Some((key, value)) => {
            let place = format!("{at}.{key}");
            Some(Cell {
                payoff: number_or_formula(value).ok_or_else(|| {
                    error(format!(
                        "'{place}' is {}: it is a chance, a number or a formula starting \
                         with =",
                        describe(value)
                    ))
                })?,
                at: place,
            })
        },
        None => None,
    };
    Ok(Entrant {
        name: name.to_string(),
        plays,
        chance,
    })
}

/// The round of `tournament` is a game played once in `games`, symmetric,
/// with two strategies.
pub(super) fn check_round(tournament: &Tournament, games: &[Game]) -> ForgeResult<()> {
    let at = format!("games.{}.tournament.round", tournament.name);
    let round = &tournament.round;
    match games.iter().find(|g| g.name() == round) {
        None => Err(error(format!(
            "'{at}' is {round}, which is not a game in this games section"
        ))),
        Some(Game::Tournament(_)) => Err(error(format!(
            "'{at}' is {round}, a tournament; a round is a game played once"
        ))),
        Some(Game::Table(table)) if !table.symmetric || table.strategies[0].len() != 2 => {
            Err(error(format!(
                "'{at}' is {round}, which a tournament cannot play: a round is a symmetric \
                 game of two strategies, cooperate first and defect second"
            )))
        },
        Some(Game::Table(_)) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::model::parse_games;

    const DILEMMA: &str = "
dilemma:
  players: [me, you]
  strategies: [cooperate, defect]
  symmetric: true
  payoffs:
    cooperate: {cooperate: 3, defect: 0}
    defect: {cooperate: =5 - c, defect: =1 - c}
";

    fn games(yaml: &str) -> ForgeResult<Vec<Game>> {
        parse_games(&yaml_serde::from_str(&format!("{DILEMMA}{yaml}")).unwrap())
    }

    fn tournament(yaml: &str) -> Tournament {
        let all = games(yaml).unwrap();
        assert!(all[0].played_once().is_some());
        assert_eq!(all[1].played_once(), None);
        // The dilemma comes first, so both arms run
        all.into_iter()
            .find_map(|game| match game {
                Game::Tournament(t) => Some(t),
                Game::Table(_) => None,
            })
            .unwrap()
    }

    fn fails(yaml: &str) -> String {
        games(yaml).unwrap_err().to_string()
    }

    const AXELROD: &str = "
axelrod:
  description: kinship-protocol's five strategies
  tournament:
    round: dilemma
    players:
      grim: grim_trigger
      gen: {plays: generous_tit_for_tat, forgive: =forgive}
      rand: {plays: random}
      half: {plays: random, cooperate: 0.25}
    continuation: =w
    noise: =w / 10
";

    #[test]
    fn a_tournament_names_its_round_and_its_players() {
        let t = tournament(AXELROD);
        assert_eq!(t.name, "axelrod");
        assert_eq!(t.round, "dilemma");
        assert!(t.self_play);
        let names: Vec<&str> = t.entrants.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["grim", "gen", "rand", "half"]);
        assert_eq!(t.entrants[0].plays, Plays::GrimTrigger);
        assert_eq!(t.entrants[2].chance, None);
        assert_eq!(
            t.entrants[1].chance,
            Some(Cell {
                payoff: Payoff::Formula("=forgive".into()),
                at: "games.axelrod.tournament.players.gen.forgive".into()
            })
        );
        assert_eq!(t.formulas(), ["=w", "=w / 10", "=forgive"]);
        assert_eq!(
            Game::Tournament(t.clone()).formulas(),
            ["=w", "=w / 10", "=forgive"]
        );
        assert_eq!(Game::Tournament(t.clone()).name(), "axelrod");

        let worked = t
            .work_out(|formula, at| {
                Ok(match formula {
                    "=w" => 0.9,
                    "=forgive" => {
                        assert_eq!(at, "games.axelrod.tournament.players.gen.forgive");
                        0.1
                    },
                    _ => 0.09,
                })
            })
            .unwrap();
        assert_eq!(worked.length, Length::Continuation(0.9));
        assert!((worked.noise - 0.09).abs() < f64::EPSILON);
        assert_eq!(worked.machines[1].cooperate, [1.0, 0.1]);
        // A parameter not given takes its default
        assert_eq!(worked.machines[2].cooperate, [0.5]);
        assert_eq!(worked.machines[3].cooperate, [0.25]);
    }

    #[test]
    fn rounds_noise_and_self_play_can_be_written() {
        let t = tournament(
            "
t:
  tournament: {round: dilemma, players: {a: tit_for_tat, b: always_defect}, rounds: =n, self_play: false}
",
        );
        assert!(!t.self_play);
        assert_eq!(t.formulas(), ["=n"]);
        let worked = t.work_out(|_, _| Ok(200.0)).unwrap();
        assert_eq!(worked.length, Length::Rounds(200));
        assert!(worked.noise.abs() < f64::EPSILON);
        assert_eq!(worked.machines[1].cooperate, [0.0]);
    }

    #[test]
    fn numbers_out_of_their_range_are_named() {
        let with = |ends: &str, extra: &str| {
            tournament(&format!(
                "
t:
  tournament:
    round: dilemma
    players: {{a: tit_for_tat, b: {{plays: random, cooperate: =p}}}}
    {ends}
    {extra}
"
            ))
        };
        let check = |t: &Tournament, p: f64, expected: &str| {
            let err = t.work_out(|_, _| Ok(p)).unwrap_err().to_string();
            assert!(err.contains(expected), "{err}");
        };
        let rounds = "games.t.tournament.rounds is";
        check(
            &with("rounds: =p", ""),
            2.5,
            &format!("{rounds} 2.5; it is a whole number"),
        );
        check(&with("rounds: =p", ""), 0.0, &format!("{rounds} 0;"));
        check(&with("rounds: =p", ""), 100_001.0, "from 1 to 100000");
        check(&with("rounds: =p", ""), f64::NAN, &format!("{rounds} NaN"));
        check(
            &with("continuation: =p", ""),
            1.0,
            "games.t.tournament.continuation is 1; it is the chance that a match goes on \
             after a round, from 0 to 0.9999",
        );
        check(&with("continuation: =p", ""), -0.1, "continuation is -0.1");
        check(
            &with("rounds: 10", "noise: =p"),
            1.5,
            "game 't': games.t.tournament.noise is 1.5; it is a chance, from 0 to 1",
        );
        check(
            &with("rounds: 10", ""),
            -1.0,
            "games.t.tournament.players.b.cooperate is -1",
        );
        // The error of a formula is the one `value` gives
        let err = with("rounds: 10", "")
            .work_out(|_, at| Err(ForgeError::Eval(format!("{at}: Unknown variable: p"))))
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "Formula evaluation error: games.t.tournament.players.b.cooperate: Unknown variable: p"
        );
        let err = with("rounds: =p", "")
            .work_out(|_, _| Err(ForgeError::Eval("bad rounds".into())))
            .unwrap_err();
        assert!(err.to_string().contains("bad rounds"));
        let err = with("continuation: =p", "")
            .work_out(|_, _| Err(ForgeError::Eval("bad continuation".into())))
            .unwrap_err();
        assert!(err.to_string().contains("bad continuation"));
        let err = with("rounds: 10", "noise: =p")
            .work_out(|_, _| Err(ForgeError::Eval("bad noise".into())))
            .unwrap_err();
        assert!(err.to_string().contains("bad noise"));
    }

    #[test]
    fn tournaments_that_cannot_be_used_are_named() {
        let players = "players: {a: tit_for_tat, b: always_defect}";
        let cases = [
            (
                format!("t: {{tournament: {{round: dilemma, {players}, rounds: 1}}, players: [x]}}"),
                "games.t has \"players\" beside its tournament",
            ),
            ("t: {tournament: 3}".to_string(), "'games.t.tournament' must be a mapping"),
            (
                format!("t: {{tournament: {{round: dilemma, {players}, rounds: 1, turns: 3}}}}"),
                "'games.t.tournament' has \"turns\", which a tournament does not have: it has \
                 round, players, rounds, continuation, noise, self_play",
            ),
            (
                format!("t: {{tournament: {{{players}, rounds: 1}}}}"),
                "'games.t.tournament' has no round",
            ),
            (
                format!("t: {{tournament: {{round: [dilemma], {players}, rounds: 1}}}}"),
                "'games.t.tournament.round' must name a game, not a list",
            ),
            (
                format!("t: {{tournament: {{round: nope, {players}, rounds: 1}}}}"),
                "'games.t.tournament.round' is nope, which is not a game in this games section",
            ),
            (
                format!("t: {{tournament: {{round: dilemma, {players}, rounds: 1}}}}\nu: {{tournament: {{round: t, {players}, rounds: 1}}}}"),
                "'games.u.tournament.round' is t, a tournament; a round is a game played once",
            ),
            (
                format!("t: {{tournament: {{round: dilemma, {players}}}}}"),
                "has neither rounds nor continuation",
            ),
            (
                format!("t: {{tournament: {{round: dilemma, {players}, rounds: 1, continuation: 0.5}}}}"),
                "has rounds and continuation; a match has one or the other",
            ),
            (
                format!("t: {{tournament: {{round: dilemma, {players}, rounds: many}}}}"),
                "'games.t.tournament.rounds' is \"many\": it is a number or a formula",
            ),
            (
                format!("t: {{tournament: {{round: dilemma, {players}, continuation: true}}}}"),
                "'games.t.tournament.continuation' is true",
            ),
            (
                format!("t: {{tournament: {{round: dilemma, {players}, rounds: 1, noise: []}}}}"),
                "'games.t.tournament.noise' is a list",
            ),
            (
                format!("t: {{tournament: {{round: dilemma, {players}, rounds: 1, self_play: 1}}}}"),
                "'games.t.tournament.self_play' must be true or false, not 1",
            ),
            (
                "t: {tournament: {round: dilemma, rounds: 1}}".to_string(),
                "'games.t.tournament.players' must be a mapping of each player's name to the \
                 strategy it plays (always_cooperate, always_defect, tit_for_tat,",
            ),
            (
                "t: {tournament: {round: dilemma, players: {a: tit_for_tat}, rounds: 1}}".to_string(),
                "'games.t.tournament.players' has 1 player; a tournament has two or more",
            ),
            (
                "t: {tournament: {round: dilemma, players: {a.b: tit_for_tat, c: random}, rounds: 1}}".to_string(),
                "'games.t.tournament.players' has \"a.b\", which is not a name",
            ),
            (
                "t: {tournament: {round: dilemma, players: {a: pavlov, b: random}, rounds: 1}}".to_string(),
                "'games.t.tournament.players.a' plays \"pavlov\", which is not a built-in \
                 strategy: always_cooperate",
            ),
            (
                "t: {tournament: {round: dilemma, players: {a: {forgive: 0.1}, b: random}, rounds: 1}}".to_string(),
                "'games.t.tournament.players.a' has no plays",
            ),
            (
                "t: {tournament: {round: dilemma, players: {a: {plays: 3}, b: random}, rounds: 1}}".to_string(),
                "'games.t.tournament.players.a' plays 3",
            ),
            (
                "t: {tournament: {round: dilemma, players: {a: {plays: tit_for_tat, forgive: 0.1}, b: random}, rounds: 1}}".to_string(),
                "'games.t.tournament.players.a' has \"forgive\": tit_for_tat takes no parameter \
                 (generous_tit_for_tat takes forgive, random takes cooperate)",
            ),
            (
                "t: {tournament: {round: dilemma, players: {a: {plays: random, forgive: 0.1}, b: random}, rounds: 1}}".to_string(),
                "'games.t.tournament.players.a' has \"forgive\": random takes cooperate",
            ),
            (
                "t: {tournament: {round: dilemma, players: {a: {plays: random, cooperate: often}, b: random}, rounds: 1}}".to_string(),
                "'games.t.tournament.players.a.cooperate' is \"often\": it is a chance",
            ),
        ];
        for (yaml, expected) in cases {
            let err = fails(&yaml);
            assert!(err.contains(expected), "{yaml}\n{err}");
        }
    }

    #[test]
    fn a_round_is_a_symmetric_game_of_two_strategies() {
        let tournament =
            "t: {tournament: {round: g, players: {a: tit_for_tat, b: random}, rounds: 1}}";
        for game in [
            "g: {players: [a, b], strategies: [x, y], payoffs: {x: {x: [1, 1], y: [0, 0]}, y: {x: [0, 0], y: [1, 1]}}}",
            "g: {players: [a, b], strategies: [x, y, z], symmetric: true, payoffs: {x: {x: 1, y: 0, z: 0}, y: {x: 0, y: 1, z: 0}, z: {x: 0, y: 0, z: 1}}}",
        ] {
            let err = fails(&format!("{game}\n{tournament}"));
            assert!(
                err.contains("'games.t.tournament.round' is g, which a tournament cannot play: a \
                 round is a symmetric game of two strategies"),
                "{err}"
            );
        }
    }
}
