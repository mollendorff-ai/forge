//! Renaming a game, a player, a tournament's player and a strategy (ADR-075,
//! steps 3 and 5), and every name that reads them.
//!
//! A game is named by its key in `games:`, by a tournament's `round:`, and
//! by its results (`price_war.equilibrium_1.lune.payoff`); a player by the
//! game's `players:` and `strategies:` and by the results under it; a
//! strategy by `strategies:`, each level of `payoffs:` that is its player's,
//! `dynamics.start` and the results that end with it (`others.cut`, a count a
//! payoff of the count form reads); a tournament's player by its key and
//! by the tournament's results (`rivalry.score.steady`).
//!
//! A change names them as `price_war`, `price_war.players.lune`,
//! `price_war.strategies.hold` (every player's) or
//! `price_war.strategies.lune.hold` (one player's, in a game that is not
//! symmetric), and `rivalry.players.steady`.

use super::{read::rekey, top_free, Entity, Rename};
use crate::core::array_calculator::references::Named;
use crate::edit::text::Site;
use crate::games::Game;
use crate::types::ParsedModel;

/// What `name` names when its first part is a game of `model`: the game, a
/// player or a strategy, with the name it has now and whether `new` is free
/// there; `None` when its first part is not a game
pub(super) fn entity(
    model: &ParsedModel,
    name: &str,
    new: &str,
) -> Result<Option<(Entity, String, bool)>, String> {
    let (g, rest) = name.split_once('.').unwrap_or((name, ""));
    let Some(game) = model.games.get(g) else {
        return Ok(None);
    };
    let parts: Vec<&str> = rest.split('.').filter(|p| !p.is_empty()).collect();
    let (game_name, word) = (g.to_string(), |s: &str| s.to_string());
    let found = match (parts.as_slice(), game) {
        ([], _) => (Entity::Game(game_name), word(g), top_free(model, new)),
        (["players", p], Game::Table(t)) if t.players.iter().any(|x| x == p) => (
            Entity::Player {
                game: game_name,
                player: word(p),
            },
            word(p),
            !t.players.iter().any(|x| x == new),
        ),
        (["players", p], Game::Tournament(t)) if t.entrants.iter().any(|e| e.name == *p) => (
            Entity::Entrant {
                game: game_name,
                entrant: word(p),
            },
            word(p),
            !t.entrants.iter().any(|e| e.name == new),
        ),
        (["strategies", s], Game::Table(t))
            if t.strategies.iter().any(|l| l.iter().any(|x| x == s)) =>
        {
            (
                Entity::Strategy {
                    game: game_name,
                    player: None,
                    strategy: word(s),
                },
                word(s),
                !t.strategies
                    .iter()
                    .any(|l| l.iter().any(|x| x == s) && l.iter().any(|x| x == new)),
            )
        },
        (["strategies", p, s], Game::Table(t)) => {
            let list = t
                .players
                .iter()
                .position(|x| x == p)
                .map(|i| &t.strategies[i])
                .filter(|l| l.iter().any(|x| x == s))
                .ok_or_else(|| format!("{p} is not a player of {g} with a strategy {s}"))?;
            if t.symmetric {
                return Err(format!(
                    "{g} is symmetric: its players have the same strategies, so rename {s} for all of them, as {g}.strategies.{s}"
                ));
            }
            (
                Entity::Strategy {
                    game: game_name,
                    player: Some(word(p)),
                    strategy: word(s),
                },
                word(s),
                !list.iter().any(|x| x == new),
            )
        },
        _ => {
            return Err(format!(
                "it is not in the game: a player is {g}.players.<name>, a strategy \
                 {g}.strategies.<name>, or {g}.strategies.<player>.<name> for one player's"
            ))
        },
    };
    Ok(Some(found))
}

/// Whether a result's first part says the next one is a player:
/// `equilibrium_1`, `dominated`, `even_mix`
fn by_player(first: &str) -> bool {
    first == "dominated"
        || first == "even_mix"
        || first
            .strip_prefix("equilibrium_")
            .is_some_and(|k| k.parse::<usize>().is_ok())
}

impl Rename {
    /// The result `rest` of `game` once renamed; `None` when it is not
    pub(super) fn game_result(&self, game: &str, rest: &str) -> Option<Named> {
        let parts: Vec<&str> = rest.split('.').collect();
        // The places of the name, each with the name it holds now
        let places: Vec<(usize, &str)> = match &self.entity {
            Entity::Game(g) if g == game => {
                return Some(Named::Game {
                    game: self.new.clone(),
                    rest: rest.to_string(),
                })
            },
            // `equilibrium_1.lune.hold`, `dominated.lune.hold`
            Entity::Player { game: g, player } if g == game => match parts.as_slice() {
                [first, _, _] if by_player(first) => vec![(1, player.as_str())],
                _ => vec![],
            },
            Entity::Strategy {
                game: g,
                player: who,
                strategy,
            } if g == game => match parts.as_slice() {
                [first, player, _]
                    if by_player(first) && who.as_ref().is_none_or(|w| w == player) =>
                {
                    vec![(2, strategy.as_str())]
                },
                ["dynamics", "share", _] => vec![(2, strategy.as_str())],
                ["others", _] => vec![(1, strategy.as_str())],
                _ => vec![],
            },
            // `score.steady`, `match.steady.matcher`
            Entity::Entrant { game: g, entrant } if g == game => match parts.as_slice() {
                ["score" | "rank" | "cooperation" | "stable", _] => vec![(1, entrant.as_str())],
                ["match", _, _] => vec![(1, entrant.as_str()), (2, entrant.as_str())],
                _ => vec![],
            },
            _ => vec![],
        };
        let respelled: Vec<&str> = parts
            .iter()
            .enumerate()
            .map(|(i, part)| {
                if places.contains(&(i, part)) {
                    self.new.as_str()
                } else {
                    part
                }
            })
            .collect();
        let respelled = respelled.join(".");
        (respelled != rest).then(|| Named::Game {
            game: game.to_string(),
            rest: respelled,
        })
    }

    /// Whether `site` of `games:` names the renamed game, player or strategy
    /// itself.
    ///
    /// # Errors
    ///
    /// Returns why, when one player's strategy is renamed and the players
    /// share one list of them.
    pub(super) fn game_site(&self, site: &Site, model: &ParsedModel) -> Result<bool, String> {
        let path: Vec<&str> = site.path.iter().map(String::as_str).collect();
        let is = |name: &str| site.text == name;
        Ok(match (&self.entity, site.key, path.as_slice()) {
            (Entity::Game(g), true, ["games", k]) => k == g,
            (Entity::Game(g), false, ["games", _, "tournament", "round"]) => is(g),
            (Entity::Player { game, player }, false, ["games", k, "players", "#"]) => {
                k == game && is(player)
            },
            (Entity::Player { game, player }, true, ["games", k, "strategies", p]) => {
                k == game && p == player
            },
            (Entity::Entrant { game, entrant }, true, ["games", k, "tournament", "players", e]) => {
                k == game && e == entrant
            },
            (
                Entity::Strategy {
                    game,
                    player: who,
                    strategy,
                },
                key,
                ["games", k, part, rest @ ..],
            ) if k == game => match (key, *part, rest) {
                (false, "strategies", ["#"]) if is(strategy) => {
                    if who.is_some() {
                        return Err(format!(
                            "the players of {game} share one list of strategies: rename {strategy} \
                             for all of them, as {game}.strategies.{strategy}"
                        ));
                    }
                    true
                },
                (false, "strategies", [p, "#"]) => {
                    is(strategy) && who.as_ref().is_none_or(|w| w == p)
                },
                // A key at level i of the payoffs is a strategy of player i
                (true, "payoffs", levels) => {
                    let players = model
                        .games
                        .get(game.as_str())
                        .and_then(Game::played_once)
                        .map(|t| t.players.as_slice())
                        .unwrap_or_default();
                    levels.last() == Some(&strategy.as_str())
                        && who
                            .as_ref()
                            .is_none_or(|w| players.get(levels.len() - 1) == Some(w))
                },
                (true, "dynamics", ["start", s]) => s == strategy,
                _ => false,
            },
            _ => false,
        })
    }

    /// The games of `model` with the renamed game, player or strategy
    /// renamed in them (and not what reads them)
    pub(super) fn rename_games(&self, model: &mut ParsedModel) {
        let new = &self.new;
        match &self.entity {
            Entity::Game(g) => {
                model.games = rekey(std::mem::take(&mut model.games), g, new);
                for game in model.games.values_mut() {
                    match game {
                        Game::Table(t) if t.name == *g => t.name.clone_from(new),
                        Game::Tournament(t) => {
                            if t.name == *g {
                                t.name.clone_from(new);
                            }
                            if t.round == *g {
                                t.round.clone_from(new);
                            }
                        },
                        Game::Table(_) => {},
                    }
                }
            },
            Entity::Player { game, player } => {
                if let Some(Game::Table(t)) = model.games.get_mut(game) {
                    for p in t.players.iter_mut().filter(|p| *p == player) {
                        p.clone_from(new);
                    }
                }
            },
            Entity::Strategy {
                game,
                player: who,
                strategy,
            } => {
                if let Some(Game::Table(t)) = model.games.get_mut(game) {
                    for (p, list) in t.players.iter().zip(t.strategies.iter_mut()) {
                        if who.as_ref().is_none_or(|w| w == p) {
                            for s in list.iter_mut().filter(|s| *s == strategy) {
                                s.clone_from(new);
                            }
                        }
                    }
                }
            },
            Entity::Entrant { game, entrant } => {
                if let Some(Game::Tournament(t)) = model.games.get_mut(game) {
                    for e in t.entrants.iter_mut().filter(|e| e.name == *entrant) {
                        e.name.clone_from(new);
                    }
                }
            },
            _ => {},
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::tests::rename;
    use crate::edit::{edit_files, Change};
    use crate::games::Game;
    use crate::types::ParsedModel;
    use std::collections::{BTreeMap, HashMap};

    /// Each game of `model`, its players and strategies, and a tournament's
    /// players, as a rename names them
    pub(in super::super) fn names_of_games(model: &ParsedModel) -> Vec<String> {
        let mut names = Vec::new();
        for (g, game) in &model.games {
            names.push(g.clone());
            match game {
                Game::Table(t) => {
                    names.extend(t.players.iter().map(|p| format!("{g}.players.{p}")));
                    let mut strategies: Vec<&String> = t.strategies.iter().flatten().collect();
                    strategies.sort();
                    strategies.dedup();
                    names.extend(strategies.iter().map(|s| format!("{g}.strategies.{s}")));
                },
                Game::Tournament(t) => {
                    names.extend(t.entrants.iter().map(|e| format!("{g}.players.{}", e.name)));
                },
            }
        }
        names
    }

    /// The YAML files of the example folder `dir`, by their path in it
    fn folder(dir: &str) -> HashMap<String, String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join(dir);
        std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|x| x == "yaml"))
            .map(|path| {
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                (name, std::fs::read_to_string(&path).unwrap())
            })
            .collect()
    }

    /// `change` on `main` of `files`, the files it changes
    fn run(
        files: &HashMap<String, String>,
        main: &str,
        change: Change,
    ) -> Result<BTreeMap<String, String>, String> {
        edit_files(files, main, &[change]).map_err(|e| e.to_string())
    }

    /// `text` with each of `pairs` replaced, every time it is written
    fn replaced(text: &str, pairs: &[(&str, &str)]) -> String {
        pairs.iter().fold(text.to_string(), |text, (from, to)| {
            assert!(text.contains(from), "{from}");
            text.replace(from, to)
        })
    }

    /// `change` gives `main` as `want`, and `back` gives it back as it was
    fn renamed_and_back(dir: &str, main: &str, change: Change, back: Change, want: &str) {
        let files = folder(dir);
        let changed = run(&files, main, change).unwrap();
        assert_eq!(changed.keys().collect::<Vec<_>>(), [main]);
        assert_eq!(changed[main], want);
        let mut now = files.clone();
        now.extend(changed);
        assert_eq!(run(&now, main, back).unwrap()[main], files[main]);
    }

    #[test]
    fn a_game_is_renamed_where_a_tournament_and_its_results_name_it() {
        let text = &folder("bagel")["price_war.yaml"];
        let want = replaced(
            text,
            &[
                ("\n  price_war:\n", "\n  pricing:\n"),
                ("round: price_war", "round: pricing"),
                ("\"=price_war.", "\"=pricing."),
            ],
        );
        renamed_and_back(
            "bagel",
            "price_war.yaml",
            rename("price_war", "pricing"),
            rename("pricing", "price_war"),
            &want,
        );
        // A tournament, and the results of the count form
        let text = &folder("games")["public-goods.yaml"];
        let want = replaced(
            text,
            &[
                ("\n  pg:\n", "\n  public:\n"),
                (
                    "(pg.others.contribute + 1)",
                    "(public.others.contribute + 1)",
                ),
                ("* pg.others", "* public.others"),
                ("\"=pg.", "\"=public."),
                ("- pg.even_mix", "- public.even_mix"),
                // and under expect:
                ("pg.pure_equilibria", "public.pure_equilibria"),
                ("pg.equilibrium_1", "public.equilibrium_1"),
            ],
        );
        renamed_and_back(
            "games",
            "public-goods.yaml",
            rename("pg", "public"),
            rename("public", "pg"),
            &want,
        );
        let text = &folder("games")["axelrod.yaml"];
        let want = replaced(
            text,
            &[
                ("\n  axelrod:\n", "\n  tour:\n"),
                ("axelrod.score.", "tour.score."),
                ("axelrod.cooperation.", "tour.cooperation."),
                ("axelrod.rank.", "tour.rank."),
            ],
        );
        renamed_and_back(
            "games",
            "axelrod.yaml",
            rename("axelrod", "tour"),
            rename("tour", "axelrod"),
            &want,
        );
    }

    #[test]
    fn a_player_and_a_strategy_are_renamed_in_the_game_and_its_results() {
        let text = &folder("bagel")["price_war.yaml"];
        let want = replaced(
            text,
            &[
                ("players: [lune, rival]", "players: [shop, rival]"),
                ("dominated.lune.hold", "dominated.shop.hold"),
            ],
        );
        renamed_and_back(
            "bagel",
            "price_war.yaml",
            rename("price_war.players.lune", "shop"),
            rename("price_war.players.shop", "lune"),
            &want,
        );
        // A symmetric game's strategy: its list, both levels of the payoffs,
        // the start of the dynamics and the results
        let want = replaced(
            text,
            &[
                ("strategies: [hold, cut]", "strategies: [keep, cut]"),
                ("hold: {hold: =market.R", "keep: {keep: =market.R"),
                ("cut: {hold: =market.T", "cut: {keep: =market.T"),
                ("start: {hold: =1 -", "start: {keep: =1 -"),
                ("dominated.lune.hold", "dominated.lune.keep"),
            ],
        );
        renamed_and_back(
            "bagel",
            "price_war.yaml",
            rename("price_war.strategies.hold", "keep"),
            rename("price_war.strategies.keep", "hold"),
            &want,
        );
        // The share of a strategy at the end of the dynamics
        let want = replaced(
            text,
            &[
                ("strategies: [hold, cut]", "strategies: [hold, lower]"),
                ("cut: =market.S", "lower: =market.S"),
                (
                    "cut: {hold: =market.T, cut: =market.P}",
                    "lower: {hold: =market.T, lower: =market.P}",
                ),
                ("cut: =city.cutting_now}", "lower: =city.cutting_now}"),
                ("dynamics.share.cut", "dynamics.share.lower"),
            ],
        );
        renamed_and_back(
            "bagel",
            "price_war.yaml",
            rename("price_war.strategies.cut", "lower"),
            rename("price_war.strategies.lower", "cut"),
            &want,
        );
        // A tournament's player and its results
        let text = &folder("games")["axelrod.yaml"];
        let want = replaced(
            text,
            &[
                (
                    "        ac: always_cooperate",
                    "        coop: always_cooperate",
                ),
                ("axelrod.score.ac", "axelrod.score.coop"),
                ("axelrod.cooperation.ac", "axelrod.cooperation.coop"),
            ],
        );
        renamed_and_back(
            "games",
            "axelrod.yaml",
            rename("axelrod.players.ac", "coop"),
            rename("axelrod.players.coop", "ac"),
            &want,
        );
    }

    #[test]
    fn one_players_strategy_is_renamed_at_that_players_level_only() {
        let text = &folder("games")["inspection.yaml"];
        let want = replaced(
            text,
            &[
                ("human: [audit, trust]", "human: [check, trust]"),
                ("{audit: [0,", "{check: [0,"),
                ("{audit: [=-inputs.penalty", "{check: [=-inputs.penalty"),
                (
                    "output: inspection.equilibrium_1.human.audit",
                    "output: inspection.equilibrium_1.human.check",
                ),
                ("human.audit: =", "human.check: ="),
            ],
        );
        renamed_and_back(
            "games",
            "inspection.yaml",
            rename("inspection.strategies.human.audit", "check"),
            rename("inspection.strategies.human.check", "audit"),
            &want,
        );
        // A player with strategies of its own
        let want = replaced(
            text,
            &[
                ("players: [ai, human]", "players: [ai, person]"),
                ("      human: [audit", "      person: [audit"),
                ("equilibrium_1.human.audit", "equilibrium_1.person.audit"),
                ("equilibrium_1.human.payoff", "equilibrium_1.person.payoff"),
            ],
        );
        renamed_and_back(
            "games",
            "inspection.yaml",
            rename("inspection.players.human", "person"),
            rename("inspection.players.person", "human"),
            &want,
        );
        // A strategy the players share, written once, renamed for all of them
        let text = &folder("games")["battle-of-the-sexes.yaml"];
        let want = replaced(
            text,
            &[
                (
                    "strategies: [opera, football]",
                    "strategies: [ballet, football]",
                ),
                ("opera: {opera:", "ballet: {ballet:"),
                ("football: {opera:", "football: {ballet:"),
                (".opera: =", ".ballet: ="),
            ],
        );
        renamed_and_back(
            "games",
            "battle-of-the-sexes.yaml",
            rename("evening.strategies.opera", "ballet"),
            rename("evening.strategies.ballet", "opera"),
            &want,
        );
    }

    #[test]
    fn a_rename_in_a_game_that_cannot_be_made_is_refused_with_the_reason() {
        let refused = |dir: &str, main: &str, change: Change, why: &str| {
            let message = run(&folder(dir), main, change).unwrap_err();
            assert!(message.contains(why), "{message}");
            assert!(message.ends_with(". No file was changed"), "{message}");
        };
        let war = |change: Change, why: &str| refused("bagel", "price_war.yaml", change, why);
        war(
            rename("price_war", "rivalry"),
            "rivalry is already a name there",
        );
        // A group of the model has the name
        war(rename("price_war", "war"), "war is already a name there");
        war(
            rename("price_war", "market"),
            "market is already a name there",
        );
        war(
            rename("price_war.players.lune", "rival"),
            "rival is already a name there",
        );
        war(
            rename("price_war.strategies.hold", "cut"),
            "cut is already a name there",
        );
        war(
            rename("rivalry.players.steady", "matcher"),
            "matcher is already a name there",
        );
        war(
            rename("price_war.nothing", "x"),
            "it is not in the game: a player is price_war.players.<name>, a strategy \
             price_war.strategies.<name>, or price_war.strategies.<player>.<name> for one player's",
        );
        war(
            rename("price_war.players.nobody", "x"),
            "it is not in the game",
        );
        war(
            rename("price_war.strategies.nothing", "x"),
            "it is not in the game",
        );
        war(
            rename("rivalry.strategies.hold", "x"),
            "it is not in the game",
        );
        war(
            rename("price_war.strategies.lune.hold", "keep"),
            "price_war is symmetric: its players have the same strategies, so rename hold for all \
             of them, as price_war.strategies.hold",
        );
        // A name results use, refused by Forge
        war(
            rename("price_war.strategies.hold", "payoff"),
            "Forge refuses the model with this change",
        );
        refused(
            "games",
            "inspection.yaml",
            rename("inspection.strategies.nobody.audit", "x"),
            "nobody is not a player of inspection with a strategy audit",
        );
        refused(
            "games",
            "battle-of-the-sexes.yaml",
            rename("evening.strategies.alex.opera", "ballet"),
            "the players of evening share one list of strategies: rename opera for all of them, \
             as evening.strategies.opera",
        );
        // Players written once and read by two tournaments (`players: *roster`)
        refused(
            "games",
            "noisy-tournament.yaml",
            rename("noisy.players.ac", "x"),
            "it is not written where edit can rename it",
        );
        refused(
            "games",
            "noisy-tournament.yaml",
            rename("clean.players.ac", "x"),
            "Forge reads the new text as more than this rename",
        );
    }

    /// A rename, a game, a result of it, and the result once renamed (`z`)
    const RESPELLED: [(&str, &str, &str, Option<&str>); 17] = [
        (
            "price_war.players.lune",
            "price_war",
            "equilibrium_1.lune.hold",
            Some("price_war.equilibrium_1.z.hold"),
        ),
        (
            "price_war.players.lune",
            "price_war",
            "even_mix.lune.cut",
            Some("price_war.even_mix.z.cut"),
        ),
        ("price_war.players.lune", "price_war", "equilibria", None),
        (
            "price_war.players.lune",
            "price_war",
            "equilibrium_x.lune.hold",
            None,
        ),
        (
            "price_war.players.lune",
            "rivalry",
            "dominated.lune.hold",
            None,
        ),
        (
            "price_war.strategies.hold",
            "price_war",
            "equilibrium_2.rival.hold",
            Some("price_war.equilibrium_2.rival.z"),
        ),
        (
            "price_war.strategies.hold",
            "price_war",
            "equilibrium_1.lune.payoff",
            None,
        ),
        (
            "price_war.strategies.hold",
            "price_war",
            "dynamics.share.hold",
            Some("price_war.dynamics.share.z"),
        ),
        (
            "price_war.strategies.hold",
            "price_war",
            "others.hold",
            Some("price_war.others.z"),
        ),
        (
            "price_war.strategies.hold",
            "price_war",
            "dynamics.payoff",
            None,
        ),
        (
            "rivalry.players.steady",
            "rivalry",
            "rank.steady",
            Some("rivalry.rank.z"),
        ),
        (
            "rivalry.players.steady",
            "rivalry",
            "match.steady.steady",
            Some("rivalry.match.z.z"),
        ),
        (
            "rivalry.players.steady",
            "rivalry",
            "match.matcher.steady",
            Some("rivalry.match.matcher.z"),
        ),
        ("rivalry.players.steady", "rivalry", "stable.matcher", None),
        ("rivalry.players.steady", "rivalry", "best", None),
        (
            "price_war",
            "price_war",
            "pure_equilibria",
            Some("z.pure_equilibria"),
        ),
        ("price_war", "rivalry", "score.steady", None),
    ];

    #[test]
    fn a_result_name_is_respelled_only_where_it_names_what_is_renamed() {
        let files = folder("bagel");
        let model = crate::edit::check::read(&files, "price_war.yaml").unwrap();
        let target = |path: &str| crate::edit::locate(&model, "price_war.yaml", path).unwrap();
        let respell = |path: &str, game: &str, rest: &str| {
            let rename = super::super::Rename::of(&target(path), "z").unwrap();
            rename.game_result(game, rest)
        };
        let game_result = |text: &str| {
            let (game, rest) = text.split_once('.').unwrap();
            super::Named::Game {
                game: game.to_string(),
                rest: rest.to_string(),
            }
        };
        for (path, game, rest, want) in RESPELLED {
            assert_eq!(
                respell(path, game, rest),
                want.map(game_result),
                "{path}: {game}.{rest}"
            );
        }
    }

    /// A player, strategy or entrant that is not of the kind of game it is
    /// said to be in, or of no game at all, renames nothing
    #[test]
    fn a_name_of_another_kind_of_game_or_of_no_game_renames_nothing() {
        use super::super::{Entity, Rename};
        let named = |entity: Entity| Rename {
            file: "m.yaml".to_string(),
            entity,
            new: "z".to_string(),
        };
        let unchanged = |file: &str, entity: Entity| {
            let mut model = crate::edit::check::read(&folder("games"), file).unwrap();
            let before = serde_json::to_value(&model.games).unwrap();
            named(entity).rename_games(&mut model);
            (before, serde_json::to_value(&model.games).unwrap())
        };
        let player = |game: &str| Entity::Player {
            game: game.to_string(),
            player: "ai".to_string(),
        };
        let strategy = |game: &str| Entity::Strategy {
            game: game.to_string(),
            player: None,
            strategy: "cheat".to_string(),
        };
        let entrant = |game: &str| Entity::Entrant {
            game: game.to_string(),
            entrant: "ai".to_string(),
        };
        for entity in [
            player("nope"),
            player("clean"),
            strategy("nope"),
            strategy("clean"),
            entrant("nope"),
        ] {
            let (before, after) = unchanged("noisy-tournament.yaml", entity);
            assert_eq!(before, after);
        }
        let (before, after) = unchanged("inspection.yaml", entrant("inspection"));
        assert_eq!(before, after);
    }

    #[test]
    fn one_players_strategy_is_respelled_in_that_players_results_only() {
        let files = folder("games");
        let model = crate::edit::check::read(&files, "inspection.yaml").unwrap();
        let target =
            crate::edit::locate(&model, "inspection.yaml", "inspection.strategies.ai.cheat")
                .unwrap();
        let rename = super::super::Rename::of(&target, "lie").unwrap();
        let spelled = |rest: &str| rename.game_result("inspection", rest);
        assert!(spelled("equilibrium_1.ai.cheat").is_some());
        assert_eq!(spelled("equilibrium_1.human.cheat"), None);
    }
}
