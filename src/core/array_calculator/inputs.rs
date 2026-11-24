//! What moves a game (ADR-076 decision 5): the inputs its numbers read.
//!
//! The formulas a game writes are followed as the calculation orders them
//! ([`ArrayCalculator::cell_dependencies`]), through scalars, columns,
//! another game (a tournament's round) and the Bayesian network, and into an
//! included file for `@alias.name`, whose own formulas are followed in that
//! file. What is reached and has no formula, or a distribution (an uncertain
//! input), is an input. Columns of values are data, not inputs, and are left
//! out.

use super::names::ScalarName;
use super::order::{formula_words, Cell};
use super::ArrayCalculator;
use crate::monte_carlo::is_distribution;
use std::collections::BTreeSet;

impl ArrayCalculator {
    /// The inputs game `name` reads, in the order they are reached, as the
    /// model names them (`market.cut_price`, `@plan.assumptions.price`)
    pub(super) fn inputs_of(&self, name: &str) -> Vec<String> {
        let mut inputs = Vec::new();
        self.inputs_from(Cell::Game(name.to_string()), "", &mut inputs);
        inputs
    }

    /// The inputs `start` reads, each named with `prefix` (`@plan.` in a file
    /// included as `plan`), added to `inputs` once each
    fn inputs_from(&self, start: Cell, prefix: &str, inputs: &mut Vec<String>) {
        let mut seen = BTreeSet::new();
        let mut stack = vec![start];
        while let Some(cell) = stack.pop() {
            if !seen.insert(cell.clone()) {
                continue;
            }
            if let Cell::Scalar(name) = &cell {
                let input = format!("{prefix}{name}");
                let formula = self.model.scalars[name].formula.as_deref();
                if formula.is_none_or(is_distribution) && !inputs.contains(&input) {
                    inputs.push(input);
                }
            }
            for word in self.formulas_of(&cell).into_iter().flat_map(formula_words) {
                self.included_inputs(&word, prefix, inputs);
            }
            stack.extend(self.cell_dependencies(&cell).into_iter().rev());
        }
    }

    /// The inputs that `word`, `@alias.name`, reads in the file included as
    /// `alias`; nothing for any other word
    fn included_inputs(&self, word: &str, prefix: &str, inputs: &mut Vec<String>) {
        let found = word
            .strip_prefix('@')
            .and_then(|rest| rest.split_once('.'))
            .and_then(|(alias, name)| {
                let included = self.model.resolved_includes.get(alias)?;
                let calc = Self::new(included.model.clone());
                match calc.resolve_scalar_name(name, None) {
                    ScalarName::Found(full) => Some((calc, alias, full)),
                    _ => None,
                }
            });
        if let Some((calc, alias, full)) = found {
            calc.inputs_from(Cell::Scalar(full), &format!("{prefix}@{alias}."), inputs);
        }
    }

    /// The formulas written in `cell`
    fn formulas_of(&self, cell: &Cell) -> Vec<&str> {
        match cell {
            Cell::Scalar(name) => self.model.scalars[name]
                .formula
                .as_deref()
                .into_iter()
                .collect(),
            Cell::Column { table, column } => {
                vec![self.model.tables[table].row_formulas[column].as_str()]
            },
            Cell::Columns { table, columns } => columns
                .iter()
                .map(|column| self.model.tables[table].row_formulas[column].as_str())
                .collect(),
            Cell::Game(name) => self.model.games[name].formulas(),
            Cell::Network => self
                .model
                .network
                .as_ref()
                .map(crate::bayesian::WrittenNetwork::formulas)
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::order::Cell;
    use super::super::ArrayCalculator;
    use std::path::Path;

    /// The calculator of the model at `main`, with `files` beside it
    fn calculator(files: &[(&str, &str)]) -> ArrayCalculator {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in files {
            std::fs::write(dir.path().join(name), text).unwrap();
        }
        ArrayCalculator::new(crate::parser::parse_model(&dir.path().join(files[0].0)).unwrap())
    }

    const PD: &str = "  pd:
    players: [me, you]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs:
";

    #[test]
    fn the_price_wars_inputs_are_traced_into_the_files_it_includes() {
        let model = crate::parser::parse_model(Path::new("examples/bagel/price_war.yaml")).unwrap();
        let calc = ArrayCalculator::new(model);
        let once = [
            "market.dozens_per_day",
            "@plan.assumptions.price_per_dozen",
            "@plan.assumptions.card_fee_rate",
            "@plan.assumptions.bagels_per_kg_flour",
            "@costs.ingredients.flour_per_kg",
            "@plan.assumptions.sesame_grams_per_bagel",
            "@costs.ingredients.sesame_per_kg",
            "@costs.ingredients.honey_water_per_dozen",
            "@plan.assumptions.open_days_per_month",
            "market.undercut_share",
            "market.cut_price",
            "market.cut_growth",
            "city.cutting_now",
        ];
        assert_eq!(calc.inputs_of("price_war"), once);
        // The tournament reads its round's, and its own continuation
        assert_eq!(
            calc.inputs_of("rivalry"),
            [&once[..], &["continuation"]].concat()
        );
    }

    #[test]
    fn a_column_and_the_network_are_followed_and_data_is_left_out() {
        let calc = calculator(&[(
            "m.yaml",
            &format!(
                "_forge_version: \"5.6.0\"
k: 2
unused: 7
spread:
  value: 1
  formula: =MC.Triangular(0.5, 1, 1.5)
p: 0.2
e: 0.1
t:
  a: [1, 2, 3]
  b: =a * k
bayesian_network:
  nodes:
    partner: {{type: discrete, states: [good, bad], prior: [=1 - p, =p]}}
    move:
      type: discrete
      states: [ok, slip]
      parents: [partner]
      cpt: {{good: [=1 - e, =e], bad: [=e, =1 - e]}}
  queries:
    seen: {{node: partner, evidence: {{move: slip}}}}
games:
{PD}      cooperate: {{cooperate: =SUM(t.b), defect: =spread}}
      defect: {{cooperate: =bayesian_network.seen.bad, defect: 1}}
"
            ),
        )]);
        assert_eq!(calc.inputs_of("pd"), ["k", "spread", "e", "p"]);
    }

    #[test]
    fn a_name_an_included_file_has_no_scalar_for_moves_nothing() {
        let calc = calculator(&[
            (
                "m.yaml",
                &format!(
                    "_forge_version: \"5.6.0\"
_includes:
  - file: data.yaml
    as: data
games:
{PD}      cooperate: {{cooperate: =SUM(@data.t.a), defect: =@nowhere.x}}
      defect: {{cooperate: =@data.rate, defect: =@data.twice}}
"
                ),
            ),
            (
                "data.yaml",
                "_forge_version: \"5.6.0\"
rate: 3
twice:
  formula: =rate * 2
t:
  a: [1, 2]
",
            ),
        ]);
        assert_eq!(calc.inputs_of("pd"), ["@data.rate"]);
    }

    #[test]
    fn the_formulas_of_columns_calculated_together_are_each_columns() {
        let calc = calculator(&[(
            "m.yaml",
            "_forge_version: \"5.6.0\"
t:
  a: [1, 2]
  b: =PREV(c, 0) + a
  c: =b * 2
",
        )]);
        let both = Cell::Columns {
            table: "t".into(),
            columns: vec!["b".into(), "c".into()],
        };
        assert_eq!(calc.formulas_of(&both), ["=PREV(c, 0) + a", "=b * 2"]);
    }
}
