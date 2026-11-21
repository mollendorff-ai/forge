//! `forge game`: the dominated strategies and the equilibria of the games in
//! a model, and the standings of its tournaments (ADR-068).

use super::display_number;
use super::prediction::serialize_for_file;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::games::report::{Dominated, NamedEquilibrium};
use crate::games::{GameReport, TableReport, TournamentReport};
use crate::parser;
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

/// Work out every game of a model, or only `game`, and return what was found
/// (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, the model has no `games`
/// section, `game` or `scenario` is not in it, or a game cannot be
/// calculated (a payoff that fails, or that reads a failed cell).
pub fn game_core(
    file: &Path,
    game: Option<&str>,
    scenario: Option<&str>,
) -> ForgeResult<Vec<GameReport>> {
    let mut model = parser::parse_model(file)?;
    if model.games.is_empty() {
        return Err(ForgeError::Validation(
            "No 'games' section found in the model (model format 5.5.0, ADR-068)".to_string(),
        ));
    }
    if let Some(name) = game.filter(|name| !model.games.contains_key(*name)) {
        let names: Vec<&str> = model.games.keys().map(String::as_str).collect();
        return Err(ForgeError::Validation(format!(
            "Game '{name}' not found; the model has: {}",
            names.join(", ")
        )));
    }
    if let Some(name) = scenario {
        super::apply_scenario(&mut model, name)?;
    }
    crate::core::ArrayCalculator::new(model).calculate_games(game)
}

/// Execute the game command
///
/// # Errors
///
/// As [`game_core`], and when the output file cannot be written.
///
/// # Panics
///
/// Never in practice: a report is plain data and always serialises.
pub fn game(
    c: &mut Console<'_>,
    file: &Path,
    name: Option<&str>,
    scenario: Option<&str>,
    output: Option<PathBuf>,
) -> ForgeResult<()> {
    outln!(c, "{}", "🎲 Forge - Games".bold().green());
    outln!(c, "   File: {}", file.display());
    if let Some(s) = scenario {
        outln!(c, "   Scenario: {}", s.bright_yellow().bold());
    }
    outln!(c);
    let reports = game_core(file, name, scenario)?;
    for report in &reports {
        match report {
            GameReport::Table(r) => print_table(c, r),
            GameReport::Tournament(r) => print_tournament(c, r),
        }
    }
    if let Some(path) = output {
        let text = serialize_for_file(&reports, &path).expect("a report serialises");
        fs::write(&path, text).map_err(ForgeError::Io)?;
        outln!(
            c,
            "{}",
            format!("💾 Results written to {}", path.display())
                .bold()
                .green()
        );
    }
    Ok(())
}

fn print_table(c: &mut Console<'_>, r: &TableReport) {
    outln!(
        c,
        "{}",
        format!("{}: {}", r.game, r.players.join(" vs ")).bold()
    );
    for (player, strategies) in &r.strategies {
        outln!(c, "   {player} chooses from: {}", strategies.join(", "));
    }
    print_dominated(c, "Strictly dominated, removed in this order", &r.dominated);
    // A strictly dominated strategy is weakly dominated too: say it once
    let only_weakly: Vec<Dominated> = r
        .weakly_dominated
        .iter()
        .filter(|w| {
            !r.dominated
                .iter()
                .any(|d| d.player == w.player && d.strategy == w.strategy)
        })
        .cloned()
        .collect();
    print_dominated(c, "Weakly dominated (not removed)", &only_weakly);
    let count = r.equilibria.len();
    let kind = match (r.mixed_searched, count) {
        (true, 1) => "equilibrium",
        (true, _) => "equilibria",
        (false, 1) => "pure equilibrium",
        (false, _) => "pure equilibria",
    };
    outln!(c, "   {}", format!("{count} {kind}:").bold());
    for (k, e) in r.equilibria.iter().enumerate() {
        print_equilibrium(c, k + 1, e);
    }
    if !r.mixed_searched {
        outln!(
            c,
            "   {}",
            "Mixed equilibria were not looked for: they are found for two players \
             with at most ten strategies each."
                .yellow()
        );
    }
    for tie in &r.ties {
        outln!(c, "   {}", format!("Degenerate: {tie}").yellow());
    }
    if !r.ties.is_empty() {
        outln!(
            c,
            "   {}",
            "A tie can give a whole line of equilibria, so the mixed ones listed may not be all."
                .yellow()
        );
    }
    outln!(c, "   Payoff against an even mix:");
    for (player, pays) in &r.even_mix {
        let line: Vec<String> = pays
            .iter()
            .map(|(strategy, p)| format!("{strategy} {}", display_number(*p)))
            .collect();
        outln!(c, "      {player}: {}", line.join(", "));
    }
    outln!(c);
}

fn print_tournament(c: &mut Console<'_>, r: &TournamentReport) {
    let length = r.rounds.map_or_else(
        || {
            format!(
                "goes on after each round with chance {}",
                display_number(r.continuation.unwrap_or_default())
            )
        },
        |n| format!("{n} rounds a match"),
    );
    outln!(
        c,
        "{}",
        format!("{}: a tournament of {}", r.game, r.round).bold()
    );
    let noise = if r.noise > 0.0 {
        format!(
            ", each move flipped with chance {}",
            display_number(r.noise)
        )
    } else {
        String::new()
    };
    let self_play = if r.self_play { ", self-play" } else { "" };
    outln!(c, "   {length}{noise}{self_play}");
    outln!(
        c,
        "   Ranking (payoff per round; share of moves that cooperate):"
    );
    let width = r.matches.keys().map(String::len).max().unwrap_or(0);
    for s in &r.ranking {
        outln!(
            c,
            "      {:>2}. {:<width$}  {:.3}  cooperates {:.3}  ({})",
            s.rank,
            s.player,
            s.score,
            s.cooperation,
            s.plays.name()
        );
    }
    outln!(c, "   Each row's payoff per round against each column:");
    let cell = width.max(6);
    let header: Vec<String> = r.matches.keys().map(|b| format!("{b:>cell$}")).collect();
    outln!(c, "      {:<width$} {}", "", header.join(" "));
    for (a, row) in &r.matches {
        let line: Vec<String> = row.values().map(|p| format!("{p:>cell$.3}")).collect();
        outln!(c, "      {a:<width$} {}", line.join(" "));
    }
    if r.stable.is_empty() {
        outln!(c, "   No player is collectively stable.");
    } else {
        outln!(
            c,
            "   Collectively stable (no player does better against it than it does against \
             itself): {}",
            r.stable.join(", ")
        );
    }
    outln!(c);
}

fn print_dominated(c: &mut Console<'_>, title: &str, list: &[Dominated]) {
    if list.is_empty() {
        return;
    }
    outln!(c, "   {title}:");
    for d in list {
        outln!(
            c,
            "      {}: {} is dominated by {}",
            d.player,
            d.strategy,
            d.by
        );
    }
}

fn print_equilibrium(c: &mut Console<'_>, number: usize, e: &NamedEquilibrium) {
    outln!(c, "      {number}. {}", e.kind);
    for (player, play) in &e.players {
        let plays: Vec<String> = play
            .strategies
            .iter()
            .filter(|(_, p)| **p > 0.0)
            .map(|(strategy, p)| {
                if 1.0 - *p < 1e-12 {
                    strategy.clone()
                } else {
                    format!("{strategy} {}", display_number(*p))
                }
            })
            .collect();
        outln!(
            c,
            "         {player}: {} (payoff {})",
            plays.join(", "),
            display_number(play.payoff)
        );
    }
}

/// The arguments of `forge game`.
#[cfg(feature = "cli")]
#[derive(clap::Args)]
#[command(
    about = "Find the equilibria of the games in a model, and play its tournaments",
    long_about = "Find the dominated strategies and the equilibria of the games in a model,
and play its tournaments.

A game is a payoff table of numbers or formulas in the 'games' section of a
model (format 5.5.0). Every payoff can read the rest of the model, so a
scenario or an analysis command moves the game with it.

REPORTS, PER GAME:
  - Strategies strictly dominated by another of the player's own, in the
    order they are removed; weakly dominated ones are named, not removed
  - Pure-strategy Nash equilibria, for any number of players
  - Mixed equilibria, for two players with at most ten strategies each
    (support enumeration); a tie that makes the list possibly incomplete
    is named as degenerate
  - Each strategy's payoff against an even mix of the others' strategies

A tournament plays a symmetric game of two strategies (cooperate, defect)
round after round, between built-in strategies: always_cooperate,
always_defect, tit_for_tat, suspicious_tit_for_tat, tit_for_two_tats,
grim_trigger, win_stay_lose_shift, generous_tit_for_tat (forgive) and
random (cooperate). A match lasts a number of rounds, or goes on after each
round with a chance (continuation), and noise flips each move with a chance.
The values are exact expected values, not samples.

REPORTS, PER TOURNAMENT:
  - Each player's score (payoff per round over its matches), rank and share
    of moves that cooperate
  - The match table: each player's payoff per round against each other
  - The collectively stable players: none does better against one than it
    does against itself

The same numbers are scalars named under the game, which formulas and the
other commands can read: stag_hunt.equilibrium_1.me.stag, stag_hunt.equilibria,
stag_hunt.even_mix.me.hare, axelrod.score.tft, axelrod.match.tft.ad,
axelrod.stable.grim, and so on.

EXAMPLES:
  forge game model.yaml                       # Every game in the model
  forge game model.yaml --game stag_hunt      # One game
  forge game model.yaml --scenario costly     # With a scenario's overrides
  forge game model.yaml -o equilibria.json    # Write the results (JSON or YAML)"
)]
pub struct GameArgs {
    /// Path to YAML file with a games section
    pub file: PathBuf,

    /// Only this game
    #[arg(short, long)]
    pub game: Option<String>,

    /// Scenario name to apply (uses variable overrides from 'scenarios' section)
    #[arg(short, long)]
    pub scenario: Option<String>,

    /// Output file (.yaml or .json)
    #[arg(short, long)]
    pub output: Option<PathBuf>,
}

#[cfg(feature = "cli")]
impl GameArgs {
    /// Run the command on `c`.
    ///
    /// # Errors
    ///
    /// As [`game`].
    pub fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        game(
            c,
            &self.file,
            self.game.as_deref(),
            self.scenario.as_deref(),
            self.output,
        )
    }
}

#[cfg(all(test, feature = "cli"))]
mod tests {
    use crate::cli::forge;

    const MODEL: &str = r#"_forge_version: "5.5.0"
c: 0
scenarios:
  costly:
    c: 1
games:
  stag_hunt:
    players: [me, you]
    strategies: [stag, hare]
    symmetric: true
    payoffs:
      stag: {stag: 4, hare: 0}
      hare: {stag: =3 - c, hare: =3 - c}
  prisoners:
    players: [a, b]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs:
      cooperate: {cooperate: 3, defect: 0}
      defect: {cooperate: 5, defect: 1}
"#;

    fn write(dir: &tempfile::TempDir, text: &str) -> String {
        let path = dir.path().join("m.yaml");
        std::fs::write(&path, text).unwrap();
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn it_prints_the_equilibria_of_every_game() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(&dir, MODEL);
        let out = forge(&["game", &file]).printed();
        // Stag hunt (nashpy 0.0.43, and by hand): (stag, stag), (hare, hare)
        // and the mix with stag at 3/4
        assert!(out.contains("stag_hunt: me vs you"), "{out}");
        assert!(out.contains("3 equilibria:"), "{out}");
        assert!(out.contains("stag 0.75, hare 0.25 (payoff 3.00)"), "{out}");
        // Prisoner's dilemma: cooperate is strictly dominated, (defect,
        // defect) is the only equilibrium
        assert!(out.contains("a: cooperate is dominated by defect"), "{out}");
        assert!(out.contains("1 equilibrium:"), "{out}");
        assert!(
            out.contains("a: cooperate 1.50, defect 3.00"),
            "an even mix: {out}"
        );
        assert!(!out.contains("Scenario"), "{out}");
    }

    #[test]
    fn a_scenario_and_one_game_can_be_asked_for() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(&dir, MODEL);
        let out = forge(&["game", &file, "--game", "stag_hunt", "-s", "costly"]).printed();
        // c = 1: hare pays 2 against both, the mix is stag at (3 - 1) / 4
        assert!(out.contains("Scenario: costly"), "{out}");
        assert!(out.contains("stag 0.50, hare 0.50 (payoff 2.00)"), "{out}");
        assert!(!out.contains("prisoners"), "{out}");
    }

    #[test]
    fn weak_dominance_ties_and_games_without_mixed_search_are_said() {
        let dir = tempfile::tempdir().unwrap();
        // b's r is weakly dominated by l (equal against x), and b is
        // indifferent between them when a plays x
        let file = write(
            &dir,
            r#"_forge_version: "5.5.0"
games:
  weak:
    players: [a, b]
    strategies:
      a: [x, y]
      b: [l, r]
    payoffs:
      x: {l: [2, 1], r: [0, 1]}
      y: {l: [0, 1], r: [2, 0]}
  solo:
    players: [p, q, r]
    strategies: [u, d]
    payoffs:
      u:
        u: {u: [1, 1, 1], d: [1, 1, 0]}
        d: {u: [1, 0, 1], d: [1, 0, 0]}
      d:
        u: {u: [0, 1, 1], d: [0, 1, 0]}
        d: {u: [0, 0, 1], d: [0, 0, 0]}
  three:
    players: [p, q, r]
    strategies: [u, d]
    payoffs:
      u:
        u: {u: [1, 1, 1], d: [0, 0, 0]}
        d: {u: [0, 0, 0], d: [0, 0, 0]}
      d:
        u: {u: [0, 0, 0], d: [0, 0, 0]}
        d: {u: [0, 0, 0], d: [0, 0, 0]}
"#,
        );
        let out = forge(&["game", &file]).printed();
        assert!(out.contains("Weakly dominated (not removed):"), "{out}");
        assert!(
            out.contains("Degenerate: against a's x, b's l and r pay the same"),
            "{out}"
        );
        assert!(out.contains("whole line of equilibria"), "{out}");
        assert!(
            out.contains("Mixed equilibria were not looked for"),
            "{out}"
        );
        assert!(out.contains("5 pure equilibria:"), "{out}");
        // Each of the three gains 1 by choosing u, whatever the others do:
        // u is dominant and (u, u, u) is the only equilibrium
        assert!(out.contains("1 pure equilibrium:"), "{out}");
    }

    #[test]
    fn results_are_written_as_json_or_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(&dir, MODEL);
        let json = dir.path().join("r.json");
        let out = forge(&[
            "game",
            &file,
            "-g",
            "prisoners",
            "-o",
            json.to_str().unwrap(),
        ])
        .printed();
        assert!(out.contains("Results written to"), "{out}");
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(written[0]["game"], "prisoners");
        assert_eq!(written[0]["equilibria"][0]["kind"], "pure");
        assert_eq!(written[0]["dominated"][0]["strategy"], "cooperate");
        let yaml = dir.path().join("r.yaml");
        let out = forge(&["game", &file, "-o", yaml.to_str().unwrap()]).printed();
        assert!(out.contains("Results written to"), "{out}");
        let text = std::fs::read_to_string(&yaml).unwrap();
        assert!(text.contains("game: stag_hunt") && text.contains("game: prisoners"));
    }

    #[test]
    fn it_names_what_is_wrong() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(&dir, MODEL);
        let ran = forge(&["game", &file, "--game", "nope"]);
        assert_eq!(ran.code, 1);
        assert!(
            ran.err
                .contains("Game 'nope' not found; the model has: stag_hunt, prisoners"),
            "{}",
            ran.err
        );
        let ran = forge(&["game", &file, "--scenario", "nope"]);
        assert_eq!(ran.code, 1, "{}", ran.both());
        let ran = forge(&["game", "no/such/file.yaml"]);
        assert_eq!(ran.code, 1, "{}", ran.both());
        // A model that cannot be calculated at all (a cycle) stops the command
        let cycle = write(
            &dir,
            &MODEL.replace("c: 0\n", "c: {formula: \"=d\"}\nd: {formula: \"=c\"}\n"),
        );
        let ran = forge(&["game", &cycle]);
        assert!(ran.err.contains("depend on each other"), "{}", ran.err);
        // An output that cannot be written is an error too (the cycle
        // above was written over the model, so it is written again)
        let file = write(&dir, MODEL);
        let ran = forge(&["game", &file, "-o", dir.path().to_str().unwrap()]);
        assert_eq!(ran.code, 1, "{}", ran.both());
        assert!(ran.err.contains("IO error"), "{}", ran.both());
        let plain = write(&dir, "_forge_version: \"5.5.0\"\nx: 1\n");
        let ran = forge(&["game", &plain]);
        assert!(ran.err.contains("No 'games' section"), "{}", ran.err);
        let broken = write(
            &dir,
            &MODEL.replace(
                "hare: {stag: =3 - c, hare: =3 - c}",
                "hare: {stag: =1 / 0, hare: 1}",
            ),
        );
        let ran = forge(&["game", &broken, "--game", "stag_hunt"]);
        assert!(ran.err.contains("Division by zero"), "{}", ran.err);
        // A game nobody asked for does not fail the answer
        let out = forge(&["game", &broken, "--game", "prisoners"]).printed();
        assert!(out.contains("prisoners: a vs b"), "{out}");
    }

    const TOURNAMENT: &str = r#"_forge_version: "5.5.0"
games:
  dilemma:
    players: [me, you]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs:
      cooperate: {cooperate: 3, defect: 0}
      defect: {cooperate: 5, defect: 1}
  axelrod:
    tournament:
      round: dilemma
      players:
        grim: grim_trigger
        tft: tit_for_tat
        ad: always_defect
        ac: always_cooperate
        rand: random
      rounds: 200
"#;

    #[test]
    fn it_prints_a_tournaments_ranking_match_table_and_stable_players() {
        let dir = tempfile::tempdir().unwrap();
        let file = write(&dir, TOURNAMENT);
        let out = forge(&["game", &file, "--game", "axelrod"]).printed();
        // The exact scores of kinship-protocol's tournament (by hand and
        // against axelrod 4.14.0, src/games/standings.rs). Grim cooperates
        // in all of three matches, once in 200 against AD, and (2 - 2^-199)
        // times in 200 against Random: (3 + 0.005 + 0.01) / 5 = 0.603
        assert!(out.contains("axelrod: a tournament of dilemma"), "{out}");
        assert!(out.contains("200 rounds a match, self-play"), "{out}");
        assert!(
            out.contains(" 1. grim  2.596  cooperates 0.603  (grim_trigger)"),
            "{out}"
        );
        assert!(
            out.contains(" 5. rand  1.909  cooperates 0.500  (random)"),
            "{out}"
        );
        assert!(
            out.contains("Each row's payoff per round against each column"),
            "{out}"
        );
        // TFT's row: 3 against Grim and itself, 199/200 against AD
        assert!(
            out.contains("tft   3.000  3.000  0.995  3.000  2.246"),
            "{out}"
        );
        assert!(
            out.contains("than it does against itself): grim, tft, ad"),
            "{out}"
        );
        assert!(!out.contains("dilemma: me vs you"), "{out}");
    }

    #[test]
    fn a_tournament_with_noise_and_continuation_says_so() {
        let dir = tempfile::tempdir().unwrap();
        // By hand over 10 rounds: STFT gets (5 + 9 * 3) / 10 = 3.2 against
        // AC, more than AC's 3 against itself, and AC gets 2.7 against STFT,
        // more than STFT's 1 against itself: neither is stable
        let file = write(
            &dir,
            &TOURNAMENT.replace(
                "        grim: grim_trigger\n        tft: tit_for_tat\n        ad: always_defect\n        ac: always_cooperate\n        rand: random\n      rounds: 200",
                "        ac: always_cooperate\n        stft: suspicious_tit_for_tat\n      rounds: 10",
            ),
        );
        let out = forge(&["game", &file, "-g", "axelrod"]).printed();
        assert!(out.contains("No player is collectively stable."), "{out}");
        assert!(out.contains("stft  3.200  1.000"), "{out}");
        let noisy = write(
            &dir,
            &TOURNAMENT.replace(
                "      rounds: 200",
                "      continuation: 0.9\n      noise: 0.05\n      self_play: false",
            ),
        );
        let out = forge(&["game", &noisy, "-g", "axelrod"]).printed();
        assert!(
            out.contains(
                "goes on after each round with chance 0.90, each move flipped with chance 0.05"
            ),
            "{out}"
        );
        assert!(!out.contains("self-play"), "{out}");
        let json = dir.path().join("t.json");
        let out = forge(&["game", &noisy, "-o", json.to_str().unwrap()]).printed();
        assert!(out.contains("Results written to"), "{out}");
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(written[0]["kind"], "table");
        assert_eq!(written[1]["kind"], "tournament");
        assert_eq!(written[1]["continuation"], 0.9);
        assert_eq!(written[1]["rounds"], serde_json::Value::Null);
    }

    /// The examples in examples/games, each number worked out outside
    /// Forge: nashpy 0.0.43 for the equilibria, the Python axelrod library
    /// 4.14.0 for the tournaments (src/games/standings.rs), or by hand.
    mod examples {
        use crate::cli::forge;
        use crate::types::ParsedModel;
        use std::path::Path;

        fn calculated(file: &str, scenario: Option<&str>) -> ParsedModel {
            let mut model = crate::parser::parse_model(Path::new(file)).unwrap();
            if let Some(name) = scenario {
                crate::cli::commands::apply_scenario(&mut model, name).unwrap();
            }
            crate::core::ArrayCalculator::new(model)
                .calculate_all()
                .unwrap()
        }

        fn value(model: &ParsedModel, name: &str) -> f64 {
            model.scalars[name].value.unwrap()
        }

        fn close(a: f64, b: f64) -> bool {
            (a - b).abs() < 1e-12
        }

        #[test]
        fn kinship_protocols_stag_hunt_and_chicken_give_their_stored_flags() {
            // kinship-protocol stores coop_wins 0, 0, 0, 1 for each, and
            // crossover_c 1 and 0.5. nashpy: stag at (3 - c) / 4 (0.75,
            // 0.625, 0.5, 0.375), swerve at (1 + c) / 3 (1/3, 0.416667,
            // 0.5, 2/3)
            let costs = [
                (None, 0.0, 0.0),
                (Some("c1"), 0.5, 0.25),
                (Some("c2"), 1.0, 0.5),
                (Some("c3"), 1.5, 1.0),
            ];
            for (k, (scenario, stag_c, chicken_c)) in costs.into_iter().enumerate() {
                let wins = if k == 3 { 1.0 } else { 0.0 };
                let stag = calculated("examples/games/stag-hunt.yaml", scenario);
                assert!(
                    close(value(&stag, "kinship.coop_wins"), wins),
                    "{scenario:?}"
                );
                assert!(close(value(&stag, "kinship.crossover_c"), 1.0));
                assert!(close(
                    value(&stag, "kinship.stag_share"),
                    (3.0 - stag_c) / 4.0
                ));
                assert!(close(value(&stag, "stag_hunt.equilibria"), 3.0));
                let chicken = calculated("examples/games/chicken.yaml", scenario);
                assert!(
                    close(value(&chicken, "kinship.coop_wins"), wins),
                    "{scenario:?}"
                );
                assert!(close(value(&chicken, "kinship.crossover_c"), 0.5));
                assert!(close(
                    value(&chicken, "kinship.swerve_share"),
                    (1.0 + chicken_c) / 3.0
                ));
            }
        }

        #[test]
        fn kinship_protocols_tournament_is_played_not_typed_in() {
            // Exact (by hand for Grim, and within axelrod's sampling
            // error): 2.596, 2.44825, 2.208, 2.1, 1.90875. kinship stores
            // coop_wins 0, 0, 1, 1 at c = 0, 0.1, 0.2, 0.3, which this
            // keeps; its crossover_c 0.1 is 0.108: AD's score falls by c,
            // AC's not at all
            let base = calculated("examples/games/axelrod.yaml", None);
            for (player, score) in [
                ("grim", 2.596),
                ("tft", 2.448_25),
                ("ad", 2.208),
                ("ac", 2.1),
                ("rand", 1.908_75),
            ] {
                assert!(
                    close(value(&base, &format!("axelrod.score.{player}")), score),
                    "{player}"
                );
            }
            // Grim defects against AD in 199 rounds of 200 and against
            // Random once it has defected: 1 - (3 + 0.005 + 0.01) / 5
            assert!(close(value(&base, "kinship.grim_defect_rate"), 0.397));
            for (scenario, c, wins) in [
                (None, 0.0, 0.0),
                (Some("c1"), 0.1, 0.0),
                (Some("c2"), 0.2, 1.0),
                (Some("c3"), 0.3, 1.0),
            ] {
                let model = calculated("examples/games/axelrod.yaml", scenario);
                assert!(close(value(&model, "kinship.coop_wins"), wins), "c {c}");
                assert!(close(value(&model, "kinship.crossover_c"), 0.108), "c {c}");
                assert!(close(value(&model, "axelrod.score.ad"), 2.208 - c), "c {c}");
            }
        }

        #[test]
        fn a_harsher_penalty_lowers_the_audit_rate_and_not_the_cheating() {
            // nashpy: the human audits 1/2, 1/3 and 1/5 at a penalty of 2,
            // 4 and 8, gain / (gain + penalty); the AI cheats 1/3 at each,
            // audit_cost / harm
            let file = Path::new("examples/games/inspection.yaml");
            let audit = crate::cli::tornado_core(file, None).unwrap();
            let penalty = audit
                .bars
                .iter()
                .find(|b| b.input_name == "inputs.penalty")
                .unwrap();
            assert!(close(audit.base_value, 1.0 / 3.0));
            assert!(close(penalty.output_at_low, 0.5) && close(penalty.output_at_high, 0.2));
            assert_eq!(audit.bars[0].input_name, "inputs.penalty");
            let cheat =
                crate::cli::tornado_core(file, Some("inspection.equilibrium_1.ai.cheat")).unwrap();
            let penalty = cheat
                .bars
                .iter()
                .find(|b| b.input_name == "inputs.penalty")
                .unwrap();
            assert!(
                close(penalty.output_at_low, 1.0 / 3.0) && close(penalty.output_at_high, 1.0 / 3.0)
            );
            // An audit that costs 1.5 makes the AI cheat 1/2
            let cost = cheat
                .bars
                .iter()
                .find(|b| b.input_name == "inputs.audit_cost")
                .unwrap();
            assert!(close(cost.output_at_high, 0.5));
            let model = calculated("examples/games/inspection.yaml", None);
            assert!(close(value(&model, "inspection.pure_equilibria"), 0.0));
        }

        #[test]
        fn the_public_goods_game_keeps_below_five_and_contributes_above() {
            // By hand (the file's comment): keeping pays 10 (1 - r / 5) more
            // whatever the others do, so one equilibrium below r = 5 (all
            // keep, 10 each) and above it (all contribute, 6 x 10 = 60
            // each), and all 2^5 = 32 profiles at r = 5; Gambit's
            // enumpure_solve agrees in validation/games. Against an even mix
            // two of the others contribute: 1.6 x 10 x 3 / 5 = 9.6 against
            // 10 + 1.6 x 10 x 2 / 5 = 16.4
            let file = "examples/games/public-goods.yaml";
            let model = calculated(file, None);
            assert!(close(value(&model, "pg.pure_equilibria"), 1.0));
            assert!(close(value(&model, "pg.equilibrium_1.a3.keep"), 1.0));
            assert!(close(value(&model, "dilemma.all_keep"), 10.0));
            assert!(close(value(&model, "dilemma.all_contribute"), 16.0));
            assert!(close(value(&model, "dilemma.cost_of_keeping"), -6.8));
            assert!(close(value(&model, "pg.even_mix.a5.contribute"), 9.6));
            let even = calculated(file, Some("break_even"));
            assert!(close(value(&even, "pg.pure_equilibria"), 32.0));
            assert!(close(value(&even, "dilemma.cost_of_keeping"), 0.0));
            let generous = calculated(file, Some("generous"));
            assert!(close(value(&generous, "pg.pure_equilibria"), 1.0));
            assert!(close(
                value(&generous, "pg.equilibrium_1.a2.contribute"),
                1.0
            ));
            assert!(close(value(&generous, "dilemma.all_keep"), 60.0));
        }

        #[test]
        fn noise_drops_tit_for_tat_from_first_to_third() {
            // Exact, and the same order as axelrod's 2,000 seeded runs
            let model = calculated("examples/games/noisy-tournament.yaml", None);
            assert!(close(value(&model, "results.tft_place_without_noise"), 1.0));
            assert!(close(value(&model, "results.tft_place_with_noise"), 3.0));
            assert!(close(value(&model, "noisy.rank.tf2t"), 1.0));
            assert!(close(value(&model, "noisy.rank.gtft"), 2.0));
            assert!((value(&model, "noisy.score.tf2t") - 2.518_395).abs() < 1e-6);
            assert!((value(&model, "clean.score.tft") - 2.599).abs() < 1e-12);
            // Without noise TFT, GTFT, TF2T and AD are stable; with it only
            // AD and GTFT
            assert!(close(value(&model, "clean.stable.tf2t"), 1.0));
            assert!(close(value(&model, "noisy.stable.tf2t"), 0.0));
            assert!(close(value(&model, "noisy.stable.gtft"), 1.0));
        }

        #[test]
        fn the_battle_of_the_sexes_has_two_pure_equilibria_and_a_mix() {
            // nashpy and by hand: Alex at the opera with 3/5, Sam with 2/5,
            // each expecting 6/5
            let model = calculated("examples/games/battle-of-the-sexes.yaml", None);
            assert!(close(value(&model, "evening.pure_equilibria"), 2.0));
            assert!(close(
                value(&model, "evening.equilibrium_3.alex.opera"),
                0.6
            ));
            assert!(close(value(&model, "evening.equilibrium_3.sam.opera"), 0.4));
            assert!(close(
                value(&model, "evening.equilibrium_3.alex.payoff"),
                1.2
            ));
            assert!(close(
                value(&model, "evening.equilibrium_3.sam.payoff"),
                1.2
            ));
        }

        #[test]
        fn cooperation_holds_from_the_threshold_continuation() {
            // By hand: AD gets 5 - 4w a round against Grim, which gets 3
            // against itself, so Grim is stable from w = (5 - 3) / (5 - 1)
            let file = Path::new("examples/games/continuation.yaml");
            let model = calculated("examples/games/continuation.yaml", None);
            assert!(close(value(&model, "kinship.threshold"), 0.5));
            // 5 - 4 x 0.6 at w = 0.6
            assert!(close(value(&model, "long_run.match.ad.grim"), 2.6));
            let sweep = crate::cli::sensitivity_core(
                file,
                "w",
                "0.3,0.7,0.05",
                None,
                None,
                "long_run.stable.grim",
            )
            .unwrap();
            let sweep = serde_json::to_value(&sweep).unwrap();
            let entries = sweep["data"]["OneVar"]["entries"].as_array().unwrap();
            assert_eq!(entries.len(), 9);
            for entry in entries {
                let w = entry["input"].as_f64().unwrap();
                let stable = if w >= 0.5 - 1e-9 { 1.0 } else { 0.0 };
                assert_eq!(entry["output"].as_f64(), Some(stable), "w {w}");
            }
        }

        /// `examples/acquisition/deal_game.yaml`, by hand and with nashpy
        /// 0.0.43, from deal.yaml's value created at 30% (224.50793341829)
        /// and its synergies (603.149775584427), Python checked in
        /// `prediction/scenarios.rs` (`acquisition_example`):
        /// ```python
        /// point = 24 * 50 * 1.02 / 100                     # 12.24 a point of premium
        /// v30, v40 = 224.50793341829, 224.50793341829 - 10 * point   # 102.10793341829
        /// t30, t40 = v30 - 603.149775584427 + 530, v40 - 603.149775584427 + 530
        /// a, b, c = (v30 - 12) / 2, v40, (v40 - 12) / 2   # Valdane: tie at 30, wins at 40, tie at 40
        /// q = (a - b) / (a - b + c + 12)                   # Thornvale bids 40%: 0.0677456420
        /// r = ((t30 - 12) / 2 - t40) / ((t30 - 12) / 2 - t40 + (t40 - 12) / 2 + 12)  # 0.6653745275
        /// # nashpy: (30, 30) 106.2539667 / 69.6790789, (40, 40) 45.0539667 / 8.4790789,
        /// # the mixed one 98.2427758 / 15.3319004; with 400, (30, 30) alone
        /// ```
        #[test]
        fn the_sealed_bids_are_a_trap_both_can_fall_into() {
            let file = "examples/acquisition/deal_game.yaml";
            let plan = calculated(file, None);
            let near = |model: &ParsedModel, name: &str, expected: f64| {
                let got = value(model, name);
                assert!((got - expected).abs() < 1e-9, "{name}: {got}");
            };
            for (name, expected) in [
                ("bids.cost_per_point", 12.24),
                ("worth.valdane_at_30", 224.507_933_418_29),
                ("worth.valdane_at_40", 102.107_933_418_29),
                ("worth.thornvale_at_30", 151.358_157_833_863),
                ("worth.thornvale_at_40", 28.958_157_833_863),
                ("contest.escalation_costs_each", 61.2),
                ("contest.outcomes", 2.0),
                ("contest.thornvale_never_high", 0.0),
                ("sealed_bids.equilibria", 3.0),
                ("sealed_bids.equilibrium_1.valdane.bid_30", 1.0),
                ("sealed_bids.equilibrium_1.thornvale.bid_30", 1.0),
                (
                    "sealed_bids.equilibrium_1.valdane.payoff",
                    106.253_966_709_145,
                ),
                (
                    "sealed_bids.equilibrium_1.thornvale.payoff",
                    69.679_078_916_932,
                ),
                ("sealed_bids.equilibrium_2.valdane.bid_40", 1.0),
                ("sealed_bids.equilibrium_2.thornvale.bid_40", 1.0),
                (
                    "sealed_bids.equilibrium_2.valdane.payoff",
                    45.053_966_709_145,
                ),
                (
                    "sealed_bids.equilibrium_2.thornvale.payoff",
                    8.479_078_916_932,
                ),
                (
                    "sealed_bids.equilibrium_3.valdane.bid_40",
                    0.665_374_527_501,
                ),
                (
                    "sealed_bids.equilibrium_3.thornvale.bid_40",
                    0.067_745_642_007,
                ),
                (
                    "sealed_bids.equilibrium_3.valdane.payoff",
                    98.242_775_814_508,
                ),
                (
                    "sealed_bids.equilibrium_3.thornvale.payoff",
                    15.331_900_375_852,
                ),
            ] {
                near(&plan, name, expected);
            }
            // The deal's own value at 40%, as deal_bid.yaml has it (102.11)
            let mut deal =
                crate::parser::parse_model(Path::new("examples/acquisition/deal.yaml")).unwrap();
            deal.scalars.get_mut("offer.premium").unwrap().value = Some(0.40);
            let deal = crate::core::ArrayCalculator::new(deal)
                .calculate_all()
                .unwrap();
            near(&deal, "valuation.value_created", 102.107_933_418_29);
            // Cost cuts worth 400: 40% loses Thornvale money, and both bid 30%
            let cost_only = calculated(file, Some("thornvale_cost_only"));
            near(&cost_only, "worth.thornvale_at_30", 21.358_157_833_863);
            near(&cost_only, "worth.thornvale_at_40", -101.041_842_166_137);
            near(&cost_only, "contest.outcomes", 1.0);
            near(&cost_only, "contest.thornvale_never_high", 1.0);
            near(&cost_only, "sealed_bids.equilibria", 1.0);
            near(&cost_only, "sealed_bids.equilibrium_1.valdane.bid_30", 1.0);
            near(
                &cost_only,
                "sealed_bids.equilibrium_1.thornvale.payoff",
                4.679_078_916_932,
            );
        }

        #[test]
        fn forge_examples_runs_a_game_with_forge_game() {
            let out = forge(&["examples", "inspection", "--run"]).printed();
            assert!(out.contains("🎲 Forge - Games"), "{out}");
            assert!(out.contains("human: audit 0.3333, trust 0.6667"), "{out}");
        }
    }
}
