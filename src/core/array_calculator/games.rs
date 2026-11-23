//! A game as a cell of the calculation (ADR-068, section 3): it is
//! calculated after every cell its payoffs read, and its results become
//! scalars named under it, which later formulas can read.

use super::errors::locate_error;
use super::names::ScalarName;
use super::order::{formula_words, Cell};
use super::{evaluator, ArrayCalculator};
use crate::error::{ForgeError, ForgeResult};
use crate::games::dynamics::run;
use crate::games::report::Written;
use crate::games::{analyse, standings, Game, GameReport};
use crate::types::{Table, Variable};

impl ArrayCalculator {
    /// The game `word` names a result of (`stag_hunt.equilibria` names
    /// `stag_hunt`), as the cell to calculate first.
    pub(super) fn game_of(&self, word: &str) -> Option<Cell> {
        let (game, _) = word.split_once('.')?;
        self.model
            .games
            .contains_key(game)
            .then(|| Cell::Game(game.to_string()))
    }

    /// The cells a game's formulas read, and a tournament's round.
    pub(super) fn game_dependencies(&self, name: &str) -> Vec<Cell> {
        let game = &self.model.games[name];
        let mut deps = Vec::new();
        if let Game::Tournament(t) = game {
            deps.push(Cell::Game(t.round.clone()));
        }
        // The counts a payoff of the count form reads are the game's own
        // (`pg.others.contribute`), not a result it waits for
        let counts = format!("{name}.others.");
        for formula in game.formulas() {
            for word in formula_words(formula)
                .into_iter()
                .filter(|w| !w.starts_with(&counts))
            {
                let dep = self
                    .qualified_column(&word)
                    .or_else(|| self.game_of(&word))
                    .or_else(|| self.network_of(&word))
                    .or_else(|| match self.resolve_scalar_name(&word, None) {
                        ScalarName::Found(full) => Some(Cell::Scalar(full)),
                        _ => None,
                    });
                if let Some(dep) = dep.filter(|d| !deps.contains(d)) {
                    deps.push(dep);
                }
            }
        }
        deps
    }

    /// Calculate the model and return what was worked out for each game
    /// (all of them, or only `only`), in the model's order.
    ///
    /// As [`Self::calculate_scalar`] does for one scalar, a failed formula
    /// that no game asked for uses does not fail the answer.
    ///
    /// # Errors
    ///
    /// Returns the calculation's report when a game asked for failed or uses
    /// a failed cell, or when the model cannot be calculated at all.
    pub fn calculate_games(mut self, only: Option<&str>) -> ForgeResult<Vec<GameReport>> {
        let outcome = self.calculate_cells()?;
        let asked = |game: &String| only.is_none_or(|name| name == game);
        if self
            .model
            .games
            .keys()
            .any(|game| asked(game) && outcome.failed.contains(&Cell::Game(game.clone())))
        {
            return Err(outcome.report());
        }
        Ok(self
            .model
            .games
            .keys()
            .filter(|game| asked(game))
            .map(|game| self.game_reports[game].clone())
            .collect())
    }

    /// Forget the results of an earlier calculation: a game's results are
    /// worked out again, like a formula's.
    pub(super) fn clear_game_results(&mut self) {
        if self.model.games.is_empty() {
            return;
        }
        let games: Vec<String> = self.model.games.keys().map(|g| format!("{g}.")).collect();
        self.model
            .scalars
            .retain(|name, _| !games.iter().any(|g| name.starts_with(g)));
    }

    /// Calculate game `name`: work out each payoff, find the equilibria (or
    /// play the tournament), and store every result as a scalar.
    pub(super) fn calculate_game(&mut self, name: &str) -> ForgeResult<()> {
        let value = |calc: &mut Self, formula: &str, at: &str, others: &[(String, f64)]| {
            calc.formula_number(formula, others)
                .map_err(|e| locate_error(e, &format!("game '{name}': {at}"), formula))
        };
        let settings = self.model.games[name].settings();
        let mut report = match self.model.games[name].clone() {
            Game::Table(game) => {
                let table = game.table(|formula, at, others| value(self, formula, at, others))?;
                let mut report = analyse(name, &table, game.symmetric);
                for (cell, profile) in report.cells.iter_mut().zip(table.profiles()) {
                    cell.written = game
                        .written(table.index(&profile))
                        .into_iter()
                        .map(|(path, formula)| Written {
                            path,
                            formula: formula.map(str::to_string),
                            number: None,
                        })
                        .collect();
                }
                if let Some(dynamics) = &game.dynamics {
                    let settings =
                        dynamics.work_out(name, |formula, at| value(self, formula, at, &[]))?;
                    report.dynamics = Some(run(name, &table, &settings)?);
                }
                self.game_tables.insert(name.to_string(), table);
                GameReport::Table(report)
            },
            Game::Tournament(tournament) => {
                let worked =
                    tournament.work_out(&mut |formula, at| value(self, formula, at, &[]))?;
                // The round is a dependency, so it was calculated first
                let round = &self.game_tables[&tournament.round];
                GameReport::Tournament(standings(&tournament, round, &worked))
            },
        };
        match &mut report {
            GameReport::Table(r) => r.settings = settings,
            GameReport::Tournament(r) => r.settings = settings,
        }
        for (result, value) in report.scalars() {
            if let Some(ctx) = self.ctx.as_mut() {
                ctx.scalars
                    .insert(result.clone(), evaluator::Value::Number(value));
            }
            self.model
                .scalars
                .insert(result.clone(), Variable::new(result, Some(value), None));
        }
        self.game_reports.insert(name.to_string(), report);
        Ok(())
    }

    /// The number a formula of a section (a game's payoff, a network's
    /// probability) gives, read as a top-level formula reads, with `others`
    /// (the counts of the count form, ADR-073) readable by name for this
    /// formula only.
    pub(super) fn formula_number(
        &mut self,
        formula: &str,
        others: &[(String, f64)],
    ) -> ForgeResult<f64> {
        let ast = self.parsed_by_text(formula)?;
        let empty = Table::new("_scalar_context".to_string());
        self.check_ambiguous_names_by_text(formula, None, &empty)?;
        let result = self
            .evaluate_in_context(None, None, |ctx| {
                for (name, count) in others {
                    ctx.scalars
                        .insert(name.clone(), evaluator::Value::Number(*count));
                }
                let result = evaluator::evaluate_cell(&ast, formula, ctx);
                for (name, _) in others {
                    ctx.scalars.remove(name);
                }
                result
            })
            .map_err(|e| ForgeError::Eval(e.message))?;
        let single = match &result {
            evaluator::Value::Array(items) if items.len() == 1 => &items[0],
            other => other,
        };
        single.as_cell_number().ok_or_else(|| {
            ForgeError::Eval(format!("the payoff is {}, not a number", describe(single)))
        })
    }

    /// A formula that names a result its game does not have (an
    /// `equilibrium_3` of a game with two) says so, before it is evaluated.
    pub(super) fn check_game_results(&self, formula: &str) -> ForgeResult<()> {
        for word in formula_words(formula) {
            let Some(Cell::Game(game)) = self.game_of(&word) else {
                continue;
            };
            let known = self.model.scalars.contains_key(&word);
            if let (false, Some(report)) = (known, self.game_reports.get(&game)) {
                return Err(ForgeError::Eval(report.why_not(&word)));
            }
        }
        Ok(())
    }

    /// Why `name` is not a scalar of the calculated model, when it names a
    /// game's result.
    pub(super) fn missing_game_result(&self, name: &str) -> Option<String> {
        let Some(Cell::Game(game)) = self.game_of(name) else {
            return None;
        };
        self.game_reports.get(&game).map(|r| r.why_not(name))
    }
}

/// A value in a few words, for a payoff that is not a number.
fn describe(value: &evaluator::Value) -> String {
    match value {
        evaluator::Value::Text(t) => format!("the text \"{t}\""),
        other => other.as_text(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::ArrayCalculator;
    use crate::types::ParsedModel;

    fn parse(text: &str) -> Result<ParsedModel, String> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        std::fs::write(&path, text).unwrap();
        crate::parser::parse_model(&path).map_err(|e| e.to_string())
    }

    fn calculate(text: &str) -> Result<ParsedModel, String> {
        parse(text).and_then(|m| {
            ArrayCalculator::new(m)
                .calculate_all()
                .map_err(|e| e.to_string())
        })
    }

    fn value(model: &ParsedModel, name: &str) -> f64 {
        model.scalars[name].value.unwrap()
    }

    /// kinship-protocol's stag hunt with its cost `c` on each hare payoff,
    /// and its flag written as a formula over the game's results.
    fn stag_hunt(c: f64) -> String {
        format!(
            "_forge_version: \"5.5.0\"
c: {c}
games:
  stag_hunt:
    players: [me, you]
    strategies: [stag, hare]
    symmetric: true
    payoffs:
      stag: {{stag: 4, hare: 0}}
      hare: {{stag: =3 - c, hare: =3 - c}}
flags:
  coop_wins:
    formula: \"=IF(stag_hunt.even_mix.me.stag > stag_hunt.even_mix.me.hare, 1, 0)\"
  crossover_c:
    formula: \"=stag_hunt.even_mix.me.hare + c - stag_hunt.even_mix.me.stag\"
"
        )
    }

    #[test]
    fn kinship_protocols_stag_hunt_flags_come_from_the_game() {
        // stag_hunt.yaml stores c0_coop_wins to c3_coop_wins as 0, 0, 0, 1
        // for c = 0, 0.5, 1, 1.5, and crossover_c = 1. By hand: against an
        // even mix stag pays 2 and hare 3 - c, so stag wins only when c > 1.
        for (c, wins) in [(0.0, 0.0), (0.5, 0.0), (1.0, 0.0), (1.5, 1.0)] {
            let model = calculate(&stag_hunt(c)).unwrap();
            assert!(
                (value(&model, "flags.coop_wins") - wins).abs() < 1e-12,
                "c = {c}"
            );
            assert!(
                (value(&model, "flags.crossover_c") - 1.0).abs() < 1e-12,
                "c = {c}"
            );
            // nashpy 0.0.43 on [[4, 0], [3 - c, 3 - c]]: the mixed
            // equilibrium plays stag with (3 - c) / 4 (3/4 at c = 0)
            let stag = value(&model, "stag_hunt.equilibrium_3.me.stag");
            assert!((stag - (3.0 - c) / 4.0).abs() < 1e-12, "c = {c}: {stag}");
            assert!((value(&model, "stag_hunt.pure_equilibria") - 2.0).abs() < 1e-12);
            assert!((value(&model, "stag_hunt.equilibria") - 3.0).abs() < 1e-12);
        }
    }

    /// The public goods game of five players in the count form (ADR-073).
    fn public_goods(r: f64, extra: &str) -> String {
        format!(
            "_forge_version: \"5.5.0\"
endowment: 10
r: {r}
games:
  pg:
    players: [a, b, c, d, e]
    strategies: [contribute, keep]
    symmetric: true
    payoffs:
      contribute: =r * endowment * (pg.others.contribute + 1) / 5
      keep: =endowment + r * endowment * pg.others.contribute / 5
{extra}"
        )
    }

    #[test]
    fn the_public_goods_game_of_five_players() {
        // By hand: keeping minus contributing is e (1 - r / n) whatever the
        // others do, so below r = n = 5 everyone keeps, above it everyone
        // contributes, and at r = 5 all 2^5 = 32 profiles are equilibria
        // (kinship-protocol's 1, 32 and 1, which Gambit's enumpure_solve
        // gave on rc.31 with the 32 cells written out)
        let below = calculate(&public_goods(1.6, "")).unwrap();
        assert!((value(&below, "pg.pure_equilibria") - 1.0).abs() < 1e-12);
        assert!((value(&below, "pg.equilibrium_1.a.keep") - 1.0).abs() < 1e-12);
        assert!((value(&below, "pg.equilibrium_1.e.keep") - 1.0).abs() < 1e-12);
        assert!((value(&below, "pg.equilibrium_1.c.payoff") - 10.0).abs() < 1e-12);
        assert!((value(&below, "pg.dominated.b.contribute") - 1.0).abs() < 1e-12);
        // Against an even mix two of the four others contribute on average,
        // and both payoffs are linear in that count: contributing gives
        // 1.6 * 10 * 3 / 5 = 9.6, keeping 10 + 1.6 * 10 * 2 / 5 = 16.4
        assert!((value(&below, "pg.even_mix.a.contribute") - 9.6).abs() < 1e-12);
        assert!((value(&below, "pg.even_mix.a.keep") - 16.4).abs() < 1e-12);

        let at = calculate(&public_goods(5.0, "")).unwrap();
        assert!((value(&at, "pg.pure_equilibria") - 32.0).abs() < 1e-12);

        let above = calculate(&public_goods(6.0, "")).unwrap();
        assert!((value(&above, "pg.pure_equilibria") - 1.0).abs() < 1e-12);
        assert!((value(&above, "pg.equilibrium_1.d.contribute") - 1.0).abs() < 1e-12);
        // Everyone contributes: 6 * 10 * 5 / 5 = 60 each
        assert!((value(&above, "pg.equilibrium_1.d.payoff") - 60.0).abs() < 1e-12);
        assert!((value(&above, "pg.dominated.e.keep") - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_formula_reads_the_games_results_but_not_its_counts() {
        let flag = "kept:\n  formula: \"=pg.equilibrium_1.a.keep * 2\"\n";
        let model = calculate(&public_goods(1.6, flag)).unwrap();
        assert!((value(&model, "kept") - 2.0).abs() < 1e-12);
        let counts = "seen:\n  formula: \"=pg.others.keep\"\n";
        let err = calculate(&public_goods(1.6, counts)).unwrap_err();
        assert!(
            err.contains(
                "'pg.others.keep' is not a result of game 'pg': it is how many of the other \
                 players play a strategy, which only the payoffs of game 'pg' can read"
            ),
            "{err}"
        );
    }

    #[test]
    fn results_are_worked_out_again_and_never_written_back() {
        // A calculated model, calculated again with another cost, gives the
        // new results, not the stored ones; results have no formula, so
        // `forge calculate` does not write them into the file.
        let first = calculate(&stag_hunt(0.0)).unwrap();
        let mut again = first.clone();
        again.scalars.get_mut("c").unwrap().value = Some(1.5);
        let again = ArrayCalculator::new(again).calculate_all().unwrap();
        assert!((value(&again, "stag_hunt.even_mix.me.hare") - 1.5).abs() < 1e-12);
        assert!(first.scalars["stag_hunt.equilibria"].formula.is_none());
    }

    #[test]
    fn one_output_can_be_a_games_result() {
        // As tornado and goal seek read an output. The inspection game:
        // nashpy 0.0.43 gives an audit rate of 1/3 with the penalty at 4,
        // 1/5 at 8; by hand gain / (gain + penalty).
        let text = |penalty: f64| {
            format!(
                "_forge_version: \"5.5.0\"
inputs: {{gain: 2, penalty: {penalty}, audit_cost: 1, harm: 3}}
games:
  inspection:
    players: [ai, human]
    strategies: {{ai: [comply, cheat], human: [audit, trust]}}
    payoffs:
      comply: {{audit: [0, =-inputs.audit_cost], trust: [0, 0]}}
      cheat: {{audit: [=-inputs.penalty, =-inputs.audit_cost], trust: [=inputs.gain, =-inputs.harm]}}
"
            )
        };
        for (penalty, audit) in [(4.0, 1.0 / 3.0), (8.0, 0.2)] {
            let got = ArrayCalculator::new(parse(&text(penalty)).unwrap())
                .calculate_scalar("inspection.equilibrium_1.human.audit")
                .unwrap();
            assert!((got - audit).abs() < 1e-12, "penalty {penalty}: {got}");
            let cheat = ArrayCalculator::new(parse(&text(penalty)).unwrap())
                .calculate_scalar("inspection.equilibrium_1.ai.cheat")
                .unwrap();
            assert!((cheat - 1.0 / 3.0).abs() < 1e-12);
        }
        let err = ArrayCalculator::new(parse(&text(4.0)).unwrap())
            .calculate_scalar("inspection.equilibrium_2.human.audit")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("it has 1 equilibrium (equilibrium_1 to equilibrium_1)"),
            "{err}"
        );
        let err = ArrayCalculator::new(parse(&text(4.0)).unwrap())
            .calculate_scalar("nothing.here")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("Output variable 'nothing.here' not found"),
            "{err}"
        );
    }

    #[test]
    fn a_result_the_game_does_not_have_is_named() {
        let text = stag_hunt(0.0).replace(
            "stag_hunt.even_mix.me.stag > stag_hunt.even_mix.me.hare",
            "stag_hunt.equilibrium_4.me.stag > 0",
        );
        let err = calculate(&text).unwrap_err();
        assert!(
            err.contains("flags.coop_wins")
                && err.contains("'stag_hunt.equilibrium_4.me.stag' is not a result of game 'stag_hunt': it has 3 equilibria"),
            "{err}"
        );
    }

    #[test]
    fn a_table_reads_a_games_result_and_a_game_reads_a_table() {
        // The payoff of mutual hare is the sum of a column (1 + 2 = 3),
        // and a column reads the mixed equilibrium's stag share, 3/4.
        let text = "_forge_version: \"5.5.0\"
t:
  x: [1, 2]
  share: \"=x * stag_hunt.equilibrium_3.me.stag\"
games:
  stag_hunt:
    players: [me, you]
    strategies: [stag, hare]
    symmetric: true
    payoffs:
      stag: {stag: 4, hare: 0}
      hare: {stag: =SUM(t.x), hare: =SUM(t.x)}
";
        let model = calculate(text).unwrap();
        let share = value(&model, "stag_hunt.equilibrium_3.me.stag");
        assert!((share - 0.75).abs() < 1e-12, "{share}");
        assert_eq!(
            model.tables["t"].columns["share"].values,
            crate::types::ColumnValue::Number(vec![share, 2.0 * share])
        );
        let missing = text.replace("equilibrium_3", "equilibrium_9");
        assert!(calculate(&missing).unwrap_err().contains("t.share"));
    }

    #[test]
    fn a_game_that_reads_its_own_results_is_a_cycle() {
        let text = "_forge_version: \"5.5.0\"
loop:
  back:
    formula: \"=g.even_mix.a.x\"
games:
  g:
    players: [a, b]
    strategies: [x, y]
    symmetric: true
    payoffs:
      x: {x: =loop.back, y: 0}
      y: {x: 0, y: 1}
";
        let err = calculate(text).unwrap_err();
        assert!(
            err.contains("formulas loop.back, games.g depend on each other"),
            "{err}"
        );
    }

    #[test]
    fn a_payoff_that_fails_or_is_not_a_number_names_its_cell() {
        let base = "_forge_version: \"5.5.0\"
games:
  g:
    players: [a, b]
    strategies: [x]
    payoffs:
      x: {x: [PAYOFF, 0]}
reads:
  it:
    formula: \"=g.pure_equilibria\"
";
        let err = calculate(&base.replace("PAYOFF", "=1 / 0")).unwrap_err();
        assert!(
            err.contains("game 'g': a's payoff in games.g.payoffs.x.x: Division by zero"),
            "{err}"
        );
        // What uses the failed game is not calculated, and says why
        assert!(err.contains("reads.it"), "{err}");
        let err = calculate(&base.replace("PAYOFF", r#"'="high"'"#)).unwrap_err();
        assert!(
            err.contains("the payoff is the text \"high\", not a number"),
            "{err}"
        );
        let err = calculate(&base.replace("PAYOFF", "=nope + 1")).unwrap_err();
        assert!(err.contains("a's payoff in games.g.payoffs.x.x"), "{err}");
        let err = calculate(&base.replace("PAYOFF", "=(1 +")).unwrap_err();
        assert!(err.contains("syntax error"), "{err}");
        let err = calculate(&base.replace("PAYOFF", "'={1, 2}'")).unwrap_err();
        assert!(err.contains("the payoff is [1, 2], not a number"), "{err}");
        // An array of one value is that value
        let one = calculate(&base.replace("PAYOFF", "'={5}'")).unwrap();
        assert!((value(&one, "g.equilibrium_1.a.payoff") - 5.0).abs() < 1e-12);
        // A short name two groups have is ambiguous, as in any formula
        let two = format!("{base}p:\n  rate: 1\nq:\n  rate: 2\n").replace("PAYOFF", "=rate");
        let err = calculate(&two).unwrap_err();
        assert!(
            err.contains("a's payoff in games.g.payoffs.x.x") && err.contains("ambiguous"),
            "{err}"
        );
    }

    #[test]
    fn a_game_in_an_included_file_is_read_with_its_alias() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("game.yaml"),
            "_forge_version: \"5.5.0\"
games:
  g:
    players: [a, b]
    strategies: [x, y]
    symmetric: true
    payoffs:
      x: {x: 4, y: 0}
      y: {x: 3, y: 3}
",
        )
        .unwrap();
        let main = dir.path().join("main.yaml");
        std::fs::write(
            &main,
            "_forge_version: \"5.5.0\"
_includes:
  - file: game.yaml
    as: hunt
out:
  n:
    formula: \"=@hunt.g.equilibria\"
",
        )
        .unwrap();
        let model = crate::parser::parse_model(&main).unwrap();
        let model = ArrayCalculator::new(model).calculate_all().unwrap();
        assert!((value(&model, "out.n") - 3.0).abs() < 1e-12);
    }

    #[test]
    fn games_need_format_5_5_and_a_name_of_their_own() {
        let game = "games:\n  g:\n    players: [a, b]\n    strategies: [x]\n    payoffs:\n      x: {x: [1, 1]}\n";
        let err = parse(&format!("_forge_version: \"5.4.0\"\n{game}")).unwrap_err();
        assert!(
            err.contains("'games' is a section of games, which model format 5.5.0 adds (ADR-068)")
                && err.contains("this file says 5.4.0"),
            "{err}"
        );
        // Before 5.5.0, a group named games is a group, as it was
        let broken = parse(&format!("_forge_version: \"5.5.0\"\n{game}").replace("[1, 1]", "[1]"))
            .unwrap_err();
        assert!(
            broken.contains("'games.g.payoffs.x.x' has 1 payoffs"),
            "{broken}"
        );
        let old = parse("_forge_version: \"5.1.0\"\ngames:\n  played: 3\n").unwrap();
        assert_eq!(old.games.len(), 0);
        assert!((old.scalars["games.played"].value.unwrap() - 3.0).abs() < f64::EPSILON);

        for (other, what) in [
            ("g:\n  x: [1, 2]\n", "a table"),
            ("g: 3\n", "a scalar"),
            ("g:\n  k: 2\n", "the group of scalar 'g.k'"),
        ] {
            let err = parse(&format!("_forge_version: \"5.5.0\"\n{other}{game}")).unwrap_err();
            assert!(
                err.contains(&format!("game 'g' has the name of {what}")),
                "{err}"
            );
        }
    }

    #[test]
    fn the_documents_of_a_file_share_their_games() {
        let doc = |name: &str, game: &str| {
            format!(
                "---\n_forge_version: \"5.5.0\"\n_name: {name}\ngames:\n  {game}:\n    players: [a, b]\n    strategies: [x]\n    payoffs:\n      x: {{x: [1, 2]}}\n"
            )
        };
        let both = parse(&format!("{}{}", doc("one", "g"), doc("two", "h"))).unwrap();
        assert_eq!(both.games.keys().collect::<Vec<_>>(), ["g", "h"]);
        let twice = parse(&format!("{}{}", doc("one", "g"), doc("two", "g"))).unwrap_err();
        assert!(
            twice.contains("Game 'g' is defined in document 'one' and again in 'two'"),
            "{twice}"
        );
        let clash = parse(&format!(
            "{}---\n_forge_version: \"5.5.0\"\n_name: two\ng:\n  k: 1\n",
            doc("one", "g")
        ))
        .unwrap_err();
        assert!(
            clash.contains("game 'g' has the name of the group of scalar 'g.k'"),
            "{clash}"
        );
    }

    /// Hawk and dove with value 4 and cost `C`, and dynamics that read the
    /// model.
    fn hawk_dove(dynamics: &str, extra: &str) -> String {
        format!(
            "_forge_version: \"5.5.0\"
C: 6
first: 0.1
games:
  hd:
    players: [me, you]
    strategies: [hawk, dove]
    symmetric: true
    payoffs:
      hawk: {{hawk: =(4 - C) / 2, dove: 4}}
      dove: {{hawk: 0, dove: 2}}
    dynamics:
{dynamics}
{extra}"
        )
    }

    const SPREAD: &str = "      start: {hawk: =first, dove: =1 - first}
      generations: 300
      baseline: =C";

    #[test]
    fn the_dynamics_read_the_model_and_formulas_read_their_end() {
        // By hand: hawks settle at V / C, 2/3 at C = 6 and 1/2 at C = 8,
        // where each expects 2 (1 - V / C)
        let reads = "out:\n  hawks:\n    formula: \"=hd.dynamics.share.hawk * 3\"\n";
        let model = calculate(&hawk_dove(SPREAD, reads)).unwrap();
        assert!((value(&model, "out.hawks") - 2.0).abs() < 1e-10);
        assert!((value(&model, "hd.dynamics.payoff") - 2.0 / 3.0).abs() < 1e-10);
        let mut again = model;
        again.scalars.get_mut("C").unwrap().value = Some(8.0);
        let again = ArrayCalculator::new(again).calculate_all().unwrap();
        assert!((value(&again, "hd.dynamics.share.hawk") - 0.5).abs() < 1e-10);
        assert!((value(&again, "hd.dynamics.payoff") - 1.0).abs() < 1e-10);
        // A one-output command reads the end too
        let one = ArrayCalculator::new(parse(&hawk_dove(SPREAD, "")).unwrap())
            .calculate_scalar("hd.dynamics.share.dove")
            .unwrap();
        assert!((one - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn dynamics_that_cannot_be_worked_out_say_where() {
        let err = calculate(&hawk_dove(&SPREAD.replace("=first,", "=first / 0,"), "")).unwrap_err();
        assert!(
            err.contains("game 'hd': games.hd.dynamics.start.hawk: Division by zero"),
            "{err}"
        );
        let err = calculate(&hawk_dove(&SPREAD.replace("=1 - first", "=0.5"), "")).unwrap_err();
        assert!(
            err.contains(
                "game 'hd': the shares of games.hd.dynamics.start add up to 0.6; the shares of \
                 every strategy add up to 1"
            ),
            "{err}"
        );
        // Hawk against hawk pays -1, so with baseline 0 a start of hawks
        // only is refused, with the baseline that is enough
        let err = calculate(&hawk_dove(
            &SPREAD
                .replace("=C", "0")
                .replace("=first,", "1,")
                .replace("=1 - first", "0"),
            "",
        ))
        .unwrap_err();
        assert!(
            err.contains("game 'hd': in generation 0 (0 is the start), hawk pays -1")
                && err.contains("a baseline above 1 is enough"),
            "{err}"
        );
        // What reads the dynamics' results names what there is
        let reads = "out:\n  owls:\n    formula: \"=hd.dynamics.share.owl\"\n";
        let err = calculate(&hawk_dove(SPREAD, reads)).unwrap_err();
        assert!(
            err.contains(
                "'hd.dynamics.share.owl' is not a result of game 'hd': its dynamics give \
                 dynamics.share.hawk, dynamics.share.dove and dynamics.payoff, after the last \
                 generation"
            ),
            "{err}"
        );
        let none = stag_hunt(0.0).replace(
            "stag_hunt.even_mix.me.stag > stag_hunt.even_mix.me.hare",
            "stag_hunt.dynamics.share.stag > 0.5",
        );
        let err = calculate(&none).unwrap_err();
        assert!(
            err.contains(
                "'stag_hunt.dynamics.share.stag' is not a result of game 'stag_hunt': the \
                 dynamics' results come from a dynamics: key, with start and generations, and \
                 games.stag_hunt has none"
            ),
            "{err}"
        );
        // A start that reads the game's own results would wait for itself
        let err = calculate(&hawk_dove(
            &SPREAD.replace("=first,", "=hd.equilibrium_3.me.hawk,"),
            "",
        ))
        .unwrap_err();
        assert!(
            err.contains("Circular dependency detected: games.hd refers to itself"),
            "{err}"
        );
    }

    #[test]
    fn dynamics_are_for_a_symmetric_game() {
        let text = hawk_dove(SPREAD, "")
            .replace("    symmetric: true\n", "")
            .replace(
                "hawk: {hawk: =(4 - C) / 2, dove: 4}\n      dove: {hawk: 0, dove: 2}",
                "hawk: {hawk: [-1, -1], dove: [4, 0]}\n      dove: {hawk: [0, 4], dove: [2, 2]}",
            );
        let err = parse(&text).unwrap_err();
        assert!(
            err.contains(
                "'games.hd.dynamics': the dynamics follow one population whose members all play \
                 the same game, so the game must be marked symmetric: true, and games.hd is not"
            ),
            "{err}"
        );
    }

    /// kinship-protocol's tournament: its prisoner's dilemma with a cost
    /// `c` on each defection, and five strategies over 200 rounds.
    fn axelrod(c: f64, extra: &str) -> String {
        format!(
            "_forge_version: \"5.5.0\"
c: {c}
w: 0.5
games:
  axelrod:
    tournament:
      round: dilemma
      players:
        grim: grim_trigger
        tft: tit_for_tat
        ad: always_defect
        ac: always_cooperate
        rand: {{plays: random, cooperate: =1 - w}}
      rounds: 200
{extra}
  dilemma:
    players: [me, you]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs:
      cooperate: {{cooperate: 3, defect: 0}}
      defect: {{cooperate: =5 - c, defect: =1 - c}}
flags:
  coop_wins:
    formula: \"=IF(axelrod.score.ac > axelrod.score.ad, 1, 0)\"
"
        )
    }

    #[test]
    fn a_tournaments_results_are_scalars_formulas_read() {
        // By hand and against axelrod (src/games/standings.rs): AD scores
        // 2.208 - c and AC 2.1, so AC wins once c passes 0.108; the round
        // is calculated before the tournament though written after it
        for (c, wins) in [(0.1, 0.0), (0.12, 1.0)] {
            let model = calculate(&axelrod(c, "")).unwrap();
            assert!(
                (value(&model, "flags.coop_wins") - wins).abs() < 1e-12,
                "c {c}"
            );
            assert!((value(&model, "axelrod.score.ad") - (2.208 - c)).abs() < 1e-12);
            assert!((value(&model, "axelrod.cooperation.rand") - 0.5).abs() < 1e-12);
            // TFT gets S = 0 once, then P - c in 199 rounds
            let tft = value(&model, "axelrod.match.tft.ad");
            assert!((tft - 199.0 * (1.0 - c) / 200.0).abs() < 1e-12, "{tft}");
        }
        let reports = ArrayCalculator::new(parse(&axelrod(0.0, "")).unwrap())
            .calculate_games(None)
            .unwrap();
        let kinds: Vec<&str> = reports
            .iter()
            .map(|r| match r {
                crate::games::GameReport::Table(t) => t.game.as_str(),
                crate::games::GameReport::Tournament(t) => t.game.as_str(),
            })
            .collect();
        assert_eq!(kinds, ["axelrod", "dilemma"]);
        let json = serde_json::to_value(&reports).unwrap();
        assert_eq!(json[0]["kind"], "tournament");
        assert_eq!(json[0]["ranking"][0]["plays"], "grim_trigger");
        assert_eq!(json[1]["kind"], "table");
        // The tournament says which numbers it writes, and where `edit` finds them
        let settings = json[0]["settings"].as_array().unwrap();
        let paths: Vec<&str> = settings
            .iter()
            .map(|w| w["path"].as_str().unwrap())
            .collect();
        assert_eq!(
            paths,
            [
                "axelrod.tournament.rounds",
                "axelrod.tournament.noise",
                "axelrod.tournament.players.rand.cooperate"
            ]
        );
        assert_eq!(settings[0]["number"], 200.0);
        // Each payoff of a table says where `edit` finds it, and its formula
        // when it is one (the dilemma is symmetric: a cell is the row's)
        let cells = json[1]["cells"].as_array().unwrap();
        for cell in cells {
            let written = cell["written"].as_array().unwrap();
            assert_eq!(written.len(), 2);
            for payoff in written {
                assert!(payoff["path"]
                    .as_str()
                    .unwrap()
                    .starts_with("dilemma.payoffs."));
            }
        }
        assert!(cells.iter().any(|c| c["written"][0]["formula"].is_string()));
    }

    #[test]
    fn a_continuation_from_a_scalar_moves_the_stable_strategies() {
        // ADR-068's sweep: Grim is stable against AD once w >= (T - c - R)
        // / (T - P), 0.5 at c = 0; by hand AD gets (1 - w) 5 + w against it
        for (w, stable) in [(0.45, 0.0), (0.55, 1.0)] {
            let text = axelrod(0.0, "")
                .replace("rounds: 200", "continuation: =w")
                .replace("w: 0.5", &format!("w: {w}"));
            let got = ArrayCalculator::new(parse(&text).unwrap())
                .calculate_scalar("axelrod.stable.grim")
                .unwrap();
            assert!((got - stable).abs() < 1e-12, "w {w}");
        }
    }

    #[test]
    fn a_tournament_that_fails_names_where() {
        let err = calculate(&axelrod(0.0, "      noise: =1 / 0")).unwrap_err();
        assert!(
            err.contains("game 'axelrod': games.axelrod.tournament.noise: Division by zero"),
            "{err}"
        );
        // What reads a failed tournament is not calculated
        assert!(err.contains("flags.coop_wins"), "{err}");
        let err = calculate(&axelrod(0.0, "      noise: =c + 2")).unwrap_err();
        assert!(
            err.contains("game 'axelrod': games.axelrod.tournament.noise is 2; it is a chance"),
            "{err}"
        );
        let err = calculate(&axelrod(0.0, "").replace("axelrod.score.ac", "axelrod.score.pavlov"))
            .unwrap_err();
        assert!(
            err.contains("'axelrod.score.pavlov' is not a result of tournament 'axelrod'")
                && err.contains("its players are grim, tft, ad, ac, rand"),
            "{err}"
        );
        // A round that fails stops the tournament that plays it
        let err = calculate(&axelrod(0.0, "").replace("=5 - c", "=5 / 0")).unwrap_err();
        assert!(
            err.contains("Division by zero") && err.contains("games.axelrod"),
            "{err}"
        );
        // A round that reads the tournament's results is a cycle
        let err =
            calculate(&axelrod(0.0, "").replace("=5 - c", "=5 - axelrod.score.ad")).unwrap_err();
        assert!(err.contains("depend on each other"), "{err}");
    }
}
