//! `forge game`: the dominated strategies and the equilibria of the games in
//! a model, and the standings of its tournaments (ADR-068).

use super::display_number;
use super::prediction::serialize_for_file;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::games::dynamics::DynamicsReport;
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
    let kind = match (r.mixed_searched, r.symmetric_searched, count) {
        (true, _, 1) => "equilibrium",
        (true, _, _) => "equilibria",
        (false, true, 1) => "pure or symmetric equilibrium",
        (false, true, _) => "pure or symmetric equilibria",
        (false, false, 1) => "pure equilibrium",
        (false, false, _) => "pure equilibria",
    };
    outln!(c, "   {}", format!("{count} {kind}:").bold());
    for (k, e) in r.equilibria.iter().enumerate() {
        print_equilibrium(c, k + 1, e);
    }
    if r.symmetric_searched {
        outln!(
            c,
            "   {}",
            "Mixed equilibria in which every player plays alike are listed; those in which \
             players mix differently were not looked for."
                .yellow()
        );
    } else if !r.mixed_searched {
        outln!(
            c,
            "   {}",
            "Mixed equilibria were not looked for: they are found for two players \
             with at most ten strategies each, and in symmetric games of two strategies."
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
    if let Some(dynamics) = &r.dynamics {
        print_dynamics(c, dynamics);
    }
    outln!(c);
}

/// Every generation of the dynamics, then the shares at the end.
fn print_dynamics(c: &mut Console<'_>, d: &DynamicsReport) {
    let end = d.end();
    let word = if d.generations == 1 {
        "generation"
    } else {
        "generations"
    };
    outln!(
        c,
        "   {}",
        format!(
            "Dynamics, {} {word} (baseline {}):",
            d.generations,
            display_number(d.baseline)
        )
        .bold()
    );
    let row = |cells: Vec<String>| {
        let cells: Vec<String> = cells.iter().map(|cell| format!("{cell:>10}")).collect();
        format!("      {}", cells.join(" "))
    };
    let mut heading = vec!["generation".to_string()];
    heading.extend(end.shares.keys().cloned());
    heading.push("payoff".to_string());
    outln!(c, "{}", row(heading));
    for (k, generation) in d.path.iter().enumerate() {
        let mut cells = vec![k.to_string()];
        cells.extend(
            generation
                .shares
                .values()
                .map(|share| format!("{share:.6}")),
        );
        cells.push(display_number(generation.payoff));
        outln!(c, "{}", row(cells));
    }
    let shares: Vec<String> = end
        .shares
        .iter()
        .map(|(strategy, share)| format!("{strategy} {share:.6}"))
        .collect();
    outln!(
        c,
        "   After the last generation: {}, average payoff {}",
        shares.join(", "),
        display_number(end.payoff)
    );
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

/// How often a strategy is played in a mixed equilibrium: four decimals,
/// always, so the shares of one equilibrium read alike (`0.2000` beside
/// `0.6667`, where a report's rule gave `0.20`).
fn share(p: f64) -> String {
    let (whole, fraction) =
        crate::core::array_calculator::evaluator::text::number_format::rounded_digits(p, 4);
    format!("{whole}.{fraction}")
}

fn print_equilibrium(c: &mut Console<'_>, number: usize, e: &NamedEquilibrium) {
    let stable = match e.stable {
        Some(true) => ", stable: a population returns to it",
        Some(false) => ", unstable: a population moves away from it",
        None => "",
    };
    outln!(c, "      {number}. {}{stable}", e.kind);
    for (player, play) in &e.players {
        let plays: Vec<String> = play
            .strategies
            .iter()
            .filter(|(_, p)| **p > 0.0)
            .map(|(strategy, p)| {
                if 1.0 - *p < 1e-12 {
                    strategy.clone()
                } else {
                    format!("{strategy} {}", share(*p))
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
  - In a symmetric game of two strategies, the mixes in which all play
    alike, for any number of players, and whether a population returns
    to each equilibrium in which all play alike (stable)
  - With a dynamics: key, in a symmetric game, the share of each strategy
    generation by generation (the discrete replicator equation), from a
    start, with a baseline added to every payoff

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
stag_hunt.even_mix.me.hare, stag_hunt.equilibrium_3.stable,
hawk_dove.dynamics.share.hawk, hawk_dove.dynamics.payoff, axelrod.score.tft,
axelrod.match.tft.ad, axelrod.stable.grim, and so on.

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
        assert!(
            out.contains("stag 0.7500, hare 0.2500 (payoff 3.00)"),
            "{out}"
        );
        // Marked symmetric with two strategies: a population returns to
        // either pure one and leaves the mix (above 3/4 stag spreads)
        assert!(
            out.contains("1. pure, stable: a population returns to it"),
            "{out}"
        );
        assert!(
            out.contains("3. mixed, unstable: a population moves away from it"),
            "{out}"
        );
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
        assert!(
            out.contains("stag 0.5000, hare 0.5000 (payoff 2.00)"),
            "{out}"
        );
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

    /// A game with dynamics of one generation, from shares given by
    /// formulas.
    const SPREADING: &str = r#"_forge_version: "5.5.0"
x: 0.75
games:
  hunt:
    players: [me, you]
    strategies: [stag, hare]
    symmetric: true
    payoffs:
      stag: {stag: 4, hare: 0}
      hare: {stag: 3, hare: 3}
    dynamics:
      start: {stag: =x + 0.05, hare: =0.2}
      generations: =1
      baseline: =x - 0.75
"#;

    #[test]
    fn the_path_of_the_dynamics_is_printed_and_written() {
        // By hand from a stag share of 0.8: stag pays 3.2, hare 3, the
        // average is 3.16, so the share becomes 0.8 * 3.2 / 3.16 = 0.810127
        let dir = tempfile::tempdir().unwrap();
        let file = write(&dir, SPREADING);
        let json = dir.path().join("r.json");
        let out = forge(&["game", &file, "-o", json.to_str().unwrap()]).printed();
        assert!(
            out.contains("Dynamics, 1 generation (baseline 0.00):"),
            "{out}"
        );
        assert!(out.contains("1   0.810127   0.189873"), "{out}");
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        let dynamics = &written[0]["dynamics"];
        assert_eq!(dynamics["generations"], 1);
        assert_eq!(dynamics["path"].as_array().unwrap().len(), 2);
        assert_eq!(dynamics["path"][0]["shares"]["stag"], 0.8);
        let stag = dynamics["path"][1]["shares"]["stag"].as_f64().unwrap();
        assert!((stag - 0.8 * 3.2 / 3.16).abs() < 1e-15, "{stag}");
        assert!((dynamics["path"][0]["payoff"].as_f64().unwrap() - 3.16).abs() < 1e-15);
        // A game without dynamics writes none
        let plain = write(&dir, MODEL);
        let out = forge(&["game", &plain, "-o", json.to_str().unwrap()]).printed();
        assert!(!out.contains("Dynamics"), "{out}");
        let written: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(written[0]["dynamics"], serde_json::Value::Null);
        // A baseline too low for a payoff is named, with what would do
        let low = write(
            &dir,
            &SPREADING
                .replace("=x + 0.05, hare: =0.2", "0, hare: 1")
                .replace("=x - 0.75", "-3"),
        );
        let ran = forge(&["game", &low]);
        assert!(!ran.success());
        let err = ran.both();
        assert!(
            err.contains(
                "game 'hunt': in generation 0 (0 is the start), hare pays 3 and the baseline is \
                 -3, so hare's payoff plus the baseline is 0, not above 0"
            ) && err.contains("a baseline above 0 is enough in every generation"),
            "{err}"
        );
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
        assert_eq!(ran.code, 1, "{ran:?}");
        let ran = forge(&["game", "no/such/file.yaml"]);
        assert_eq!(ran.code, 1, "{ran:?}");
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
        assert_eq!(ran.code, 1, "{ran:?}");
        assert!(ran.err.contains("IO error"), "{ran:?}");
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
        fn hawks_spread_to_v_over_c_and_stag_hunters_to_one_side() {
            // By hand: with a share x of hawks, hawk pays 4 - 5x and dove
            // 2 (1 - x), equal at V / C = 2/3. The path is the equation
            // iterated in Python 3.12 (x' = x (f + 5) / (average + 5), 200
            // times from 0.1): 0.6666666666552279, average payoff
            // 0.6666666667124218; at C = 8 and 12, 1/2 and 1/3. egttools
            // 0.2.1 checks every generation (validation/games/dynamics.py).
            let file = "examples/games/hawk-dove.yaml";
            let model = calculated(file, None);
            assert!((value(&model, "population.hawks") - 0.666_666_666_655_228).abs() < 1e-12);
            assert!((value(&model, "population.payoff") - 0.666_666_666_712_422).abs() < 1e-12);
            assert!((value(&model, "hawk_dove.dynamics.share.dove") - 1.0 / 3.0).abs() < 1e-10);
            assert!(close(
                value(&model, "hawk_dove.equilibrium_3.me.hawk"),
                2.0 / 3.0
            ));
            assert!(close(value(&model, "hawk_dove.equilibrium_3.stable"), 1.0));
            for (scenario, share) in [("costly", 0.5), ("deadly", 1.0 / 3.0)] {
                let moved = calculated(file, Some(scenario));
                let hawks = value(&moved, "population.hawks");
                assert!((hawks - share).abs() < 1e-14, "{scenario}: {hawks}");
                assert!(close(value(&moved, "population.by_hand"), share));
            }
            let out = forge(&["game", file]).printed();
            assert!(
                out.contains("Dynamics, 200 generations (baseline 5.00):"),
                "{out}"
            );
            assert!(
                out.contains("generation       hawk       dove     payoff"),
                "{out}"
            );
            // The first step by hand: hawk pays 3.5, dove 1.8, the average
            // is 1.97, so 0.1 * 8.5 / 6.97 = 0.121951
            assert!(out.contains("0   0.100000   0.900000       1.97"), "{out}");
            assert!(out.contains("1   0.121951   0.878049       1.96"), "{out}");
            assert!(
                out.contains("200   0.666667   0.333333     0.6667"),
                "{out}"
            );
            assert!(
                out.contains(
                    "After the last generation: hawk 0.666667, dove 0.333333, average payoff 0.6667"
                ),
                "{out}"
            );

            // The stag hunt at c = 0: stag pays 4x and hare 3, so from 0.8
            // stag spreads and from 0.7 it dies out; with c = 1.5 hare pays
            // 1.5 and stag spreads from above 3/8 (Python 3.12, 100 times:
            // 0.9999999999995072, 0 and 1)
            let file = "examples/games/stag-hunt.yaml";
            let up = calculated(file, None);
            let stags = value(&up, "kinship.stags_at_the_end");
            assert!((stags - 0.999_999_999_999_507_2).abs() < 1e-14, "{stags}");
            assert!((value(&up, "stag_hunt.dynamics.payoff") - 4.0).abs() < 1e-11);
            let down = calculated(file, Some("few_stags"));
            assert_eq!(value(&down, "kinship.stags_at_the_end").to_bits(), 0);
            assert!(close(value(&down, "stag_hunt.dynamics.share.hare"), 1.0));
            assert!(close(value(&down, "stag_hunt.dynamics.payoff"), 3.0));
            let costly = calculated(file, Some("c3"));
            assert!(close(value(&costly, "kinship.stags_at_the_end"), 1.0));
        }

        #[test]
        fn the_volunteers_dilemma_mixes_as_the_closed_form_says() {
            // By hand (the file's comment): each volunteers with
            // 1 - (c / b)^(1 / 4), 1 - 0.4^(1/4) = 0.204729 at c = 4 and
            // 1 - 0.2^(1/4) = 0.331260 at c = 2; nobody does with
            // 0.4^(5/4) = 0.318108 and 0.2^(5/4) = 0.133748 (Python 3.12)
            let file = "examples/games/volunteers-dilemma.yaml";
            let model = calculated(file, None);
            assert!(close(value(&model, "vd.pure_equilibria"), 5.0));
            assert!((value(&model, "risk.volunteers") - 0.204_729_271_232_949).abs() < 1e-12);
            assert!((value(&model, "risk.nobody") - 0.318_108_291_506_820).abs() < 1e-12);
            assert!(close(value(&model, "risk.stable"), 1.0));
            assert!(close(value(&model, "vd.degenerate"), 0.0));
            assert!(close(value(&model, "vd.equilibrium_6.a4.payoff"), 6.0));
            let cheap = calculated(file, Some("cheap"));
            assert!((value(&cheap, "risk.volunteers") - 0.331_259_695_023_578).abs() < 1e-12);
            assert!((value(&cheap, "risk.nobody") - 0.133_748_060_995_284).abs() < 1e-12);
            // At r = 5 in the public goods game every mix is an equilibrium
            let even = calculated("examples/games/public-goods.yaml", Some("break_even"));
            assert!(close(value(&even, "pg.degenerate"), 1.0));

            let out = forge(&["game", file]).printed();
            assert!(out.contains("6 pure or symmetric equilibria:"), "{out}");
            assert!(
                out.contains("6. mixed, stable: a population returns to it"),
                "{out}"
            );
            assert!(
                out.contains("a3: volunteer 0.2047, stay_out 0.7953 (payoff 6.00)"),
                "{out}"
            );
            assert!(
                out.contains(
                    "Mixed equilibria in which every player plays alike are listed; those in \
                     which players mix differently were not looked for."
                ),
                "{out}"
            );
            let one = forge(&["game", "examples/games/public-goods.yaml"]).printed();
            assert!(one.contains("1 pure or symmetric equilibrium:"), "{one}");
            assert!(
                one.contains("1. pure, stable: a population returns to it"),
                "{one}"
            );
        }

        #[test]
        fn the_bagel_shops_cut_across_the_city_and_fund_no_festival() {
            // The price war across the city (examples/bagel/price_war.yaml),
            // iterated in Python 3.12 from the plan's R, S, T and P (the
            // price war's test in examples/mod.rs works them out):
            //   x' = x f_hold / (x f_hold + (1 - x) f_cut), 24 times from 0.9
            // gives 0.9913363782163622 cutting, 19189.979129372958 a month;
            // half cut after 11 months (0.5474685); with loyal regulars
            // (0.55) 0.023538895793148717 cut, 21406.839796760803 a month
            let file = "examples/bagel/price_war.yaml";
            let war = calculated(file, None);
            let cut = value(&war, "war.cutting_in_two_years");
            assert!((cut - 0.991_336_378_216_362_2).abs() < 1e-12, "{cut}");
            let margin = value(&war, "war.city_margin_then");
            assert!((margin - 19_189.979_129_372_958).abs() < 1e-8, "{margin}");
            let loyal = calculated(file, Some("loyal_regulars"));
            let cut = value(&loyal, "war.cutting_in_two_years");
            assert!((cut - 0.023_538_895_793_148_717).abs() < 1e-12, "{cut}");
            let out = forge(&["game", file, "--game", "price_war"]).printed();
            assert!(
                out.contains("Dynamics, 24 generations (baseline 0.00):"),
                "{out}"
            );
            assert!(out.contains("11   0.452531   0.547469  19,685.06"), "{out}");

            // The street's festival (examples/bagel/street_fund.yaml), by
            // hand: a fee brings 8 dozen a day for 30 days at the plan's
            // margin of 13.0238667, 3,125.728 to the street, 625.1456 to
            // each shop; keeping pays 1,000 - 625.1456 = 374.8544 more
            // whatever the others do, so all keep; all paying would give
            // 5 x 625.1456 - 1,000 = 2,125.728 each. Paying pays from a
            // lift of 1,000 x 5 / (30 x 13.0238667) = 12.797019 dozen
            let file = "examples/bagel/street_fund.yaml";
            // Sums of products of decimals: within 1e-9 of the hand figures
            let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
            let street = calculated(file, None);
            assert!(near(value(&street, "festival.share_of_a_fee"), 625.1456));
            assert!(close(value(&street, "street.pure_equilibria"), 1.0));
            assert!(close(
                value(&street, "street.equilibrium_1.fruiterer.keep"),
                1.0
            ));
            assert!(close(value(&street, "outcome.each_gets"), 0.0));
            assert!(near(value(&street, "outcome.if_all_paid"), 2_125.728));
            assert!(near(
                value(&street, "outcome.keeping_pays_more_by"),
                374.8544
            ));
            assert!(near(
                value(&street, "outcome.lift_that_pays"),
                12.797_018_806_498_8
            ));
            assert!(close(value(&street, "street.dominated.cafe.pay"), 1.0));
            // Matched by the borough, or with twice the lift, a shop's own
            // fee earns it 1,250.2912, and all five pay: 5,251.456 each
            for scenario in ["borough_matches", "bigger_lift"] {
                let paid = calculated(file, Some(scenario));
                assert!(
                    close(value(&paid, "street.equilibrium_1.lune.pay"), 1.0),
                    "{scenario}"
                );
                assert!(
                    near(value(&paid, "outcome.each_gets"), 5_251.456),
                    "{scenario}"
                );
            }
            let out = forge(&["game", file]).printed();
            assert!(out.contains("lune: keep (payoff 0.00)"), "{out}");
            assert!(out.contains("lune: pay 875.44, keep 1,250.29"), "{out}");
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

        /// With the penalty at 8 the human audits a fifth of the time, shown
        /// with the same four decimals as the AI's thirds (it was `0.20`
        /// beside `0.6667`, the agents of 2026-11-24)
        #[test]
        fn a_mix_shows_four_decimals_for_every_strategy() {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("inspection8.yaml");
            let text = std::fs::read_to_string("examples/games/inspection.yaml")
                .unwrap()
                .replace("penalty: 4 ", "penalty: 8 ");
            std::fs::write(&file, text).unwrap();
            let out = forge(&["game", file.to_str().unwrap()]).printed();
            assert!(out.contains("ai: comply 0.6667, cheat 0.3333"), "{out}");
            assert!(out.contains("human: audit 0.2000, trust 0.8000"), "{out}");
        }

        #[test]
        fn forge_examples_runs_a_game_with_forge_game() {
            let out = forge(&["examples", "inspection", "--run"]).printed();
            assert!(out.contains("🎲 Forge - Games"), "{out}");
            assert!(out.contains("human: audit 0.3333, trust 0.6667"), "{out}");
        }
    }
}
