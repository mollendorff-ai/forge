//! `forge game`: the dominated strategies and the equilibria of the games in
//! a model (ADR-068).

use super::display_number;
use super::prediction::serialize_for_file;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::games::report::{Dominated, NamedEquilibrium};
use crate::games::TableReport;
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
) -> ForgeResult<Vec<TableReport>> {
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
        print_table(c, report);
    }
    if let Some(path) = output {
        let text = serialize_for_file(&reports, &path)?;
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
    about = "Find the equilibria of the games in a model",
    long_about = "Find the dominated strategies and the equilibria of the games in a model.

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

The same numbers are scalars named under the game, which formulas and the
other commands can read: stag_hunt.equilibrium_1.me.stag, stag_hunt.equilibria,
stag_hunt.even_mix.me.hare, and so on.

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
        assert!(out.contains("pure equilibria:"), "{out}");
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
}
