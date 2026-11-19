//! A game as a cell of the calculation (ADR-068, section 3): it is
//! calculated after every cell its payoffs read, and its results become
//! scalars named under it, which later formulas can read.

use super::errors::locate_error;
use super::names::ScalarName;
use super::order::{formula_words, Cell};
use super::{evaluator, ArrayCalculator};
use crate::error::{ForgeError, ForgeResult};
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

    /// The cells a game's payoff formulas read.
    pub(super) fn game_dependencies(&self, name: &str) -> Vec<Cell> {
        let mut deps = Vec::new();
        for formula in self.model.games[name].formulas() {
            for word in formula_words(formula) {
                let dep = self
                    .qualified_column(&word)
                    .or_else(|| self.game_of(&word))
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

    /// Calculate game `name`: work out each payoff, find the equilibria, and
    /// store every result as a scalar.
    pub(super) fn calculate_game(&mut self, name: &str) -> ForgeResult<()> {
        let game = self.model.games[name].clone();
        let table = game.table(|formula, at| {
            self.payoff_value(formula)
                .map_err(|e| locate_error(e, &format!("game '{name}': {at}"), formula))
        })?;
        let report = crate::games::analyse(name, &table);
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

    /// The number a payoff formula gives, read as a top-level formula reads.
    fn payoff_value(&mut self, formula: &str) -> ForgeResult<f64> {
        let ast = self.parsed_by_text(formula)?;
        let empty = Table::new("_scalar_context".to_string());
        self.check_ambiguous_names_by_text(formula, None, &empty)?;
        let result = self
            .evaluate_in_context(None, None, |ctx| {
                evaluator::evaluate_cell(&ast, formula, ctx)
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
}
