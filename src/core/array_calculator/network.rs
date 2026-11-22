//! A Bayesian network as a cell of the calculation (ADR-074): its formulas
//! are worked out after every cell they read, and the posteriors of its
//! named queries become scalars, `bayesian_network.<query>.<state>`, which
//! later formulas can read. It is ordered as a game is (`games.rs`).

use super::errors::locate_error;
use super::names::ScalarName;
use super::order::{formula_words, Cell};
use super::{evaluator, ArrayCalculator};
use crate::bayesian::written::SECTION;
use crate::bayesian::BayesianConfig;
use crate::error::{ForgeError, ForgeResult};
use crate::types::Variable;

/// Why the network's step can read the network: only a model that has one
/// has the step.
const ONLY_WITH_A_NETWORK: &str = "only a model with a network has this step";

impl ArrayCalculator {
    /// The network, when `word` names one of its results
    /// (`bayesian_network.one.defector`), as the cell to calculate first.
    pub(super) fn network_of(&self, word: &str) -> Option<Cell> {
        let rest = word.strip_prefix(SECTION)?;
        (self.model.network.is_some() && rest.starts_with('.')).then_some(Cell::Network)
    }

    /// The cells the network's formulas read.
    pub(super) fn network_dependencies(&self) -> Vec<Cell> {
        let mut deps = Vec::new();
        let network = self.model.network.as_ref().expect(ONLY_WITH_A_NETWORK);
        for formula in network.formulas() {
            for word in formula_words(formula) {
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

    /// Calculate the model and return its network, each probability worked
    /// out, for `forge bayesian` to ask it what the command line asks.
    ///
    /// As [`Self::calculate_scalar`] does for one scalar, a failed formula
    /// that the network does not read does not fail the answer.
    ///
    /// # Errors
    ///
    /// Returns the calculation's report when the network failed or reads a
    /// failed cell, or when the model cannot be calculated at all; and an
    /// error when the model has no network.
    pub fn calculate_network(mut self) -> ForgeResult<BayesianConfig> {
        let outcome = self.calculate_cells()?;
        if outcome.failed.contains(&Cell::Network) {
            return Err(outcome.report());
        }
        self.worked_network.ok_or_else(|| {
            ForgeError::Validation(format!("No '{SECTION}' section found in the model"))
        })
    }

    /// Forget the results of an earlier calculation: the network's are
    /// worked out again, like a formula's.
    pub(super) fn clear_network_results(&mut self) {
        self.worked_network = None;
        if self.model.network.is_some() {
            let prefix = format!("{SECTION}.");
            self.model
                .scalars
                .retain(|name, _| !name.starts_with(&prefix));
        }
    }

    /// Calculate the network: work out each formula, check that each row is
    /// probabilities, answer each query, and store every posterior as a
    /// scalar.
    pub(super) fn calculate_network_cell(&mut self) -> ForgeResult<()> {
        let written = self.model.network.clone().expect(ONLY_WITH_A_NETWORK);
        let worked = written.work_out(|formula, at| {
            self.formula_number(formula, &[])
                .map_err(|e| locate_error(e, at, formula))
        })?;
        let results = worked
            .posteriors()
            .map_err(|e| ForgeError::Eval(format!("{SECTION}: {e}")))?;
        for (result, value) in results {
            if let Some(ctx) = self.ctx.as_mut() {
                ctx.scalars
                    .insert(result.clone(), evaluator::Value::Number(value));
            }
            self.model
                .scalars
                .insert(result.clone(), Variable::new(result, Some(value), None));
        }
        self.worked_network = Some(worked);
        Ok(())
    }

    /// A formula that names a result the network does not give (a query or
    /// a state it does not have) says so, before it is evaluated.
    pub(super) fn check_network_results(&self, formula: &str) -> ForgeResult<()> {
        for word in formula_words(formula) {
            if let Some(why) = self.missing_network_result(&word) {
                return Err(ForgeError::Eval(why));
            }
        }
        Ok(())
    }

    /// Why `name` is not a scalar of the calculated model, when it names a
    /// result of the network.
    pub(super) fn missing_network_result(&self, name: &str) -> Option<String> {
        let network = self
            .model
            .network
            .as_ref()
            .filter(|_| self.network_of(name).is_some())?;
        (!self.model.scalars.contains_key(name)).then(|| network.why_not(name))
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

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    /// ADR-074's example: kinship-protocol's mistake or defection, its prior
    /// `p` and noise `e` scalars, and Bayes' rule by hand beside it.
    fn mistake(p: f64, e: f64, extra: &str) -> String {
        format!(
            "_forge_version: \"5.6.0\"
p: {p}
e: {e}
bayesian_network:
  name: Mistake or defection?
  nodes:
    partner: {{type: discrete, states: [cooperator, defector], prior: [=1 - p, =p]}}
    move_1:
      type: discrete
      states: [cooperate, defect]
      parents: [partner]
      cpt: {{cooperator: [=1 - e, =e], defector: [=e, =1 - e]}}
    move_2:
      type: discrete
      states: [cooperate, defect]
      parents: [partner]
      cpt: {{cooperator: [=1 - e, =e], defector: [=e, =1 - e]}}
  queries:
    one: {{node: partner, evidence: {{move_1: defect}}}}
    two: {{node: partner, evidence: {{move_1: defect, move_2: defect}}}}
by_hand:
  one: {{formula: \"=p * (1 - e) / (p * (1 - e) + (1 - p) * e)\"}}
  two: {{formula: \"=p * (1 - e) ^ 2 / (p * (1 - e) ^ 2 + (1 - p) * e ^ 2)\"}}
  takes: {{formula: \"=IF(bayesian_network.one.defector > 0.5 + 1e-9, 1, IF(bayesian_network.two.defector > 0.5 + 1e-9, 2, 3))\"}}
{extra}"
        )
    }

    #[test]
    fn the_posteriors_are_scalars_that_formulas_read_and_agree_with_bayes_rule() {
        // By hand (kinship-protocol's figures): at p = 0.1 and e = 0.05 the
        // partner is a defector with 0.095 / 0.14 after one defection and
        // 0.09025 / 0.0925 after two; it takes one. At p = e = 0.01 the first
        // ties at exactly 1/2, so it takes two
        for (p, e, one, two, takes) in [
            (0.1, 0.05, 0.095 / 0.14, 0.09025 / 0.0925, 1.0),
            (0.01, 0.01, 0.5, 0.99, 2.0),
            (0.01, 0.2, 0.008 / 0.206, 0.0064 / 0.046, 3.0),
        ] {
            let model = calculate(&mistake(p, e, "")).unwrap();
            let net_one = value(&model, "bayesian_network.one.defector");
            let net_two = value(&model, "bayesian_network.two.defector");
            assert!(close(net_one, one), "p {p} e {e}: {net_one}");
            assert!(close(net_two, two), "p {p} e {e}: {net_two}");
            assert!(close(value(&model, "by_hand.one"), net_one), "p {p} e {e}");
            assert!(close(value(&model, "by_hand.two"), net_two), "p {p} e {e}");
            assert!(close(value(&model, "by_hand.takes"), takes), "p {p} e {e}");
            assert!(close(
                value(&model, "bayesian_network.one.cooperator"),
                1.0 - one
            ));
            // A result has no formula, so it is never written into the file
            assert!(model.scalars["bayesian_network.one.defector"]
                .formula
                .is_none());
        }
    }

    #[test]
    fn a_scenario_moves_the_network_and_expect_pins_it() {
        let text = mistake(
            0.1,
            0.05,
            "scenarios:\n  rare_clean:\n    scalars: {p: 0.01, e: 0.01}\n    expect: \
             {bayesian_network.one.defector: =1/2, by_hand.one: =1/2, by_hand.takes: 2}\n",
        );
        let mut under = parse(&text).unwrap();
        crate::cli::commands::apply_scenario(&mut under, "rare_clean").unwrap();
        let under = ArrayCalculator::new(under).calculate_all().unwrap();
        assert!(close(value(&under, "bayesian_network.one.defector"), 0.5));
        // Calculated again, the results are the scenario's, not the stored ones
        let base = ArrayCalculator::new(under).calculate_all().unwrap();
        assert!(close(value(&base, "bayesian_network.one.defector"), 0.5));
        let mut again = base;
        again.scalars.get_mut("p").unwrap().value = Some(0.1);
        again.scalars.get_mut("e").unwrap().value = Some(0.05);
        let again = ArrayCalculator::new(again).calculate_all().unwrap();
        assert!(close(
            value(&again, "bayesian_network.one.defector"),
            0.095 / 0.14
        ));
    }

    #[test]
    fn calculate_network_gives_the_network_worked_out() {
        let config = ArrayCalculator::new(parse(&mistake(0.25, 0.5, "")).unwrap())
            .calculate_network()
            .unwrap();
        assert_eq!(config.nodes["partner"].prior, [0.75, 0.25]);
        assert_eq!(config.nodes["move_1"].cpt["defector"], [0.5, 0.5]);
        // A failed formula the network does not read does not fail it
        let broken = mistake(0.25, 0.5, "other:\n  bad: {formula: \"=1 / 0\"}\n");
        let config = ArrayCalculator::new(parse(&broken).unwrap())
            .calculate_network()
            .unwrap();
        assert_eq!(config.nodes["partner"].prior, [0.75, 0.25]);
        // One it reads does, with where
        let mut failing = parse(&mistake(0.25, 0.5, "")).unwrap();
        failing.scalars.get_mut("p").unwrap().formula = Some("=1 / 0".to_string());
        let err = ArrayCalculator::new(failing)
            .calculate_network()
            .unwrap_err()
            .to_string();
        assert!(err.contains("p: Division by zero"), "{err}");
        assert!(err.contains("bayesian_network"), "{err}");
        // A model that cannot be calculated at all says why
        let cycle = mistake(
            0.25,
            0.5,
            "a:\n  x: {formula: \"=a.y\"}\n  y: {formula: \"=a.x\"}\n",
        );
        let err = ArrayCalculator::new(parse(&cycle).unwrap())
            .calculate_network()
            .unwrap_err()
            .to_string();
        assert!(err.contains("depend on each other"), "{err}");
        let none = ArrayCalculator::new(parse("_forge_version: \"5.6.0\"\nx: 1\n").unwrap())
            .calculate_network()
            .unwrap_err()
            .to_string();
        assert!(
            none.contains("No 'bayesian_network' section found in the model"),
            "{none}"
        );
    }

    #[test]
    fn a_probability_that_fails_or_is_not_a_probability_says_where() {
        let err =
            calculate(&mistake(0.1, 0.05, "").replace("=1 - p, =p", "=1 - p, =p / 0")).unwrap_err();
        assert!(
            err.contains(
                "bayesian_network.nodes.partner.prior.defector: Division by zero (formula: =p / 0)"
            ),
            "{err}"
        );
        // What reads the network is not calculated
        assert!(err.contains("by_hand.takes"), "{err}");
        let err =
            calculate(&mistake(0.1, 0.05, "").replacen("[=1 - e, =e]", "[=1 - e, =e / 0]", 1))
                .unwrap_err();
        assert!(
            err.contains(
                "bayesian_network.nodes.move_1.cpt.cooperator.defect: Division by zero (formula: \
                 =e / 0)"
            ),
            "{err}"
        );
        let err = calculate(&mistake(0.25, 0.05, "").replace("=1 - p, =p", "=1 - p, =p * 2"))
            .unwrap_err();
        assert!(
            err.contains(
                "bayesian_network: Node 'partner': prior probabilities must sum to 1.0, got 1.25"
            ),
            "{err}"
        );
        let err = calculate(&mistake(1.5, 0.05, "")).unwrap_err();
        assert!(
            err.contains(
                "bayesian_network: Node 'partner': in its prior probabilities, state \
                 'cooperator' has -0.5; a probability is from 0 to 1"
            ),
            "{err}"
        );
        // A text probability is refused when the file is read
        let err = parse(&mistake(0.1, 0.05, "").replace("=1 - p, =p", "'0.9', =p")).unwrap_err();
        assert!(
            err.contains("bayesian_network.nodes.partner.prior.cooperator is the text \"0.9\""),
            "{err}"
        );
    }

    #[test]
    fn a_result_the_network_does_not_give_is_named() {
        let text = mistake(0.1, 0.05, "").replace(
            "bayesian_network.one.defector > 0.5",
            "bayesian_network.one.defectr > 0.5",
        );
        let err = calculate(&text).unwrap_err();
        assert!(
            err.contains("by_hand.takes")
                && err.contains(
                    "'bayesian_network.one.defectr' is not a result of the network: query 'one' \
                     gives bayesian_network.one.cooperator, bayesian_network.one.defector"
                ),
            "{err}"
        );
        // A table column says the same
        let column = format!(
            "{}t:\n  x: [1, 2]\n  y: \"=x * bayesian_network.three.defector\"\n",
            mistake(0.1, 0.05, "")
        );
        let err = calculate(&column).unwrap_err();
        assert!(
            err.contains("t.y") && err.contains("its queries are one, two"),
            "{err}"
        );
        // So does one output asked for
        let err = ArrayCalculator::new(parse(&mistake(0.1, 0.05, "")).unwrap())
            .calculate_scalar("bayesian_network.one.nobody")
            .unwrap_err()
            .to_string();
        assert!(err.contains("query 'one' gives"), "{err}");
        let got = ArrayCalculator::new(parse(&mistake(0.1, 0.05, "")).unwrap())
            .calculate_scalar("bayesian_network.two.defector")
            .unwrap();
        assert!(close(got, 0.09025 / 0.0925));
        // Without a network, the name is a name like any other
        let err =
            calculate("_forge_version: \"5.6.0\"\nx:\n  y: {formula: \"=bayesian_network.a.b\"}\n")
                .unwrap_err();
        assert!(!err.contains("not a result of the network"), "{err}");
    }

    #[test]
    fn a_network_that_reads_its_own_results_is_a_cycle() {
        let text = mistake(0.1, 0.05, "").replace(
            "[=1 - p, =p]",
            "[=1 - by_hand.takes / 3, =by_hand.takes / 3]",
        );
        let err = calculate(&text).unwrap_err();
        assert!(
            err.contains("depend on each other") && err.contains("bayesian_network"),
            "{err}"
        );
        let text = mistake(0.1, 0.05, "").replace(
            "[=1 - p, =p]",
            "[=bayesian_network.one.cooperator, =bayesian_network.one.defector]",
        );
        let err = calculate(&text).unwrap_err();
        assert!(
            err.contains("Circular dependency detected: bayesian_network refers to itself"),
            "{err}"
        );
    }

    #[test]
    fn the_network_reads_columns_and_games_and_a_game_reads_the_network() {
        // The prior reads a column's sum (0.1 + 0.2) and the stag hunt's
        // mixed stag share (3/4); the hunt's hare payoff reads a posterior
        let text = "_forge_version: \"5.6.0\"
t:
  x: [1, 2]
  share: \"=x / 10\"
games:
  hunt:
    players: [me, you]
    strategies: [stag, hare]
    symmetric: true
    payoffs:
      stag: {stag: 4, hare: 0}
      hare: {stag: 3, hare: 3}
  later:
    players: [me, you]
    strategies: [go, stay]
    symmetric: true
    payoffs:
      go: {go: =4 * bayesian_network.q.yes, stay: 0}
      stay: {go: 1, stay: 1}
bayesian_network:
  nodes:
    a: {states: [yes, no], prior: [=SUM(t.share), =1 - SUM(t.share)]}
    b: {states: [yes, no], prior: [=hunt.equilibrium_3.me.stag, =1 - hunt.equilibrium_3.me.stag]}
  queries:
    q: {node: a}
    r: {node: b}
";
        let model = calculate(text).unwrap();
        assert!(close(value(&model, "bayesian_network.q.yes"), 0.3));
        assert!(close(value(&model, "bayesian_network.r.yes"), 0.75));
        // go pays 4 * 0.3 = 1.2 against go, stay 1: the mix is 1 / 1.2 go
        let go = value(&model, "later.equilibrium_3.me.go");
        assert!((go - 1.0 / 1.2).abs() < 1e-9, "{go}");
    }

    #[test]
    fn an_included_network_is_read_with_its_alias() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("net.yaml"), mistake(0.1, 0.05, "")).unwrap();
        let main = dir.path().join("main.yaml");
        std::fs::write(
            &main,
            "_forge_version: \"5.6.0\"
_includes:
  - file: net.yaml
    as: kin
out:
  one: {formula: \"=@kin.bayesian_network.one.defector\"}
",
        )
        .unwrap();
        let model = crate::parser::parse_model(&main).unwrap();
        let model = ArrayCalculator::new(model).calculate_all().unwrap();
        assert!(close(value(&model, "out.one"), 0.095 / 0.14));
    }

    /// A network of numbers only is calculated before any formula, and
    /// what reads it still sees its results
    #[test]
    fn a_network_of_numbers_is_read_by_formulas() {
        let text = "_forge_version: \"5.6.0\"
bayesian_network:
  nodes:
    a: {states: [x, y], prior: [0.25, 0.75]}
  queries:
    q: {node: a}
out:
  twice: {formula: \"=bayesian_network.q.x * 2\"}
";
        let model = calculate(text).unwrap();
        assert!(close(value(&model, "out.twice"), 0.5));
    }

    #[test]
    fn a_network_without_queries_is_still_checked() {
        let text = "_forge_version: \"5.6.0\"\nk: 2\nbayesian_network:\n  nodes:\n    a: {states: [x, y], prior: [=k / 4, 0.75]}\n";
        let err = calculate(text).unwrap_err();
        assert!(err.contains("must sum to 1.0, got 1.25"), "{err}");
        let fine = calculate(&text.replace("0.75", "0.5")).unwrap();
        assert!(!fine
            .scalars
            .keys()
            .any(|k| k.starts_with("bayesian_network")));
    }
}
