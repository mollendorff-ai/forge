//! An uncertain model calculated at plan (ADR-076): each `MC.*` input at its
//! plan figure, the rule `forge simulate` uses for each output's value at
//! plan. Forge Studio shows an uncertain model this way on its cards.

use super::{apply_scenario, calculated, model_read, results::CalculationResult};
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::types::ParsedModel;
use colored::Colorize;
use std::collections::BTreeMap;
use std::path::Path;

/// [`super::calculate_core`] for a model with uncertain inputs, at plan.
///
/// Each `MC.*` input is at its plan figure (`value:` beside the formula, or
/// else the distribution's likeliest value), set after the scenario, so that a
/// scenario that sets an uncertain input fixes it, as in a simulation. A
/// model without one calculates as with `calculate_core`. The file is never
/// written: the plan is one reading of an uncertain model, not its results.
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, the scenario does not
/// exist, a distribution does not parse, or calculation fails.
pub fn calculate_plan_core(file: &Path, scenario: Option<&str>) -> ForgeResult<CalculationResult> {
    let mut model = parser::parse_model(file)?;
    let read = model_read(&model);
    if let Some(scenario_name) = scenario {
        apply_scenario(&mut model, scenario_name)?;
    }
    let at_plan = set_at_plan(&mut model)?;
    calculated(file, model, true, scenario, read, at_plan)
}

/// Each uncertain input of `model` and of its included files
/// (`@namespace.name`), and each row of an uncertain column
/// (`table.column (row 1)`), set to its plan figure; the figures, by name.
///
/// # Errors
///
/// Returns an error if a distribution does not parse.
///
/// # Panics
///
/// Never: every name is one the engine read from this model.
pub(super) fn set_at_plan(model: &mut ParsedModel) -> ForgeResult<BTreeMap<String, f64>> {
    let plan = crate::monte_carlo::engine::plan_of(model).map_err(ForgeError::Validation)?;
    Ok(plan
        .into_iter()
        .map(|(name, plan)| {
            model
                .set_input(&name, plan.value)
                .expect("the engine read the input from this model");
            (name, plan.value)
        })
        .collect())
}

/// What `forge calculate --plan` says before it calculates.
pub(super) fn print_plan_banner(c: &mut Console<'_>) {
    outln!(
        c,
        "{}",
        "📋 AT PLAN - each uncertain input at its plan figure; no changes will be written\n"
            .yellow()
    );
}

/// The plan figures the model is calculated with.
pub(super) fn print_at_plan(c: &mut Console<'_>, at_plan: &BTreeMap<String, f64>) {
    if at_plan.is_empty() {
        outln!(
            c,
            "   No input is uncertain: the model is calculated as it is.\n"
        );
        return;
    }
    outln!(c, "   {}", "At plan:".bold());
    for (name, value) in at_plan {
        outln!(
            c,
            "      {} = {}",
            name.bright_blue(),
            super::display_number(*value)
        );
    }
    outln!(c);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;
    use crate::cli::{simulate_core, SimulateOptions};
    use std::path::PathBuf;

    fn example(path: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .join(path)
    }

    /// Every example with a `monte_carlo:` section.
    const UNCERTAIN: [&str; 8] = [
        "bagel/bagel_simulate.yaml",
        "microgrid/microgrid_risk.yaml",
        "realestate/realestate_risk.yaml",
        "realestate/realestate_flex.yaml",
        "acquisition/deal_risk.yaml",
        "vineyard/vineyard_risk.yaml",
        "vineyard/vineyard_independent.yaml",
        "monte-carlo.yaml",
    ];

    /// The value `result` gives `output` as simulate names it: a scalar, or
    /// a row of a column (`months.profit (row 3)`).
    fn value_of(result: &CalculationResult, output: &str) -> f64 {
        if let Some((column, row)) = output
            .strip_suffix(')')
            .and_then(|o| o.split_once(" (row "))
        {
            let (table, column) = column.split_once('.').unwrap();
            let row: usize = row.parse().unwrap();
            return result.tables[table].values[column][row - 1]
                .as_f64()
                .unwrap();
        }
        result.scalars[output].unwrap()
    }

    #[test]
    fn every_output_at_plan_is_the_one_simulate_gives() {
        for path in UNCERTAIN {
            let file = example(path);
            let plan = calculate_plan_core(&file, None).unwrap();
            let options = SimulateOptions {
                iterations: Some(10),
                ..SimulateOptions::default()
            };
            let simulated = simulate_core(&file, options).unwrap();
            assert!(!simulated.outputs.is_empty(), "{path}");
            for (name, output) in &simulated.outputs {
                let want = output.at_plan.unwrap();
                let got = value_of(&plan, name);
                assert!(
                    (got - want).abs() <= 1e-9 * want.abs().max(1.0),
                    "{path}: {name} is {got} at plan, simulate says {want}"
                );
            }
            // The inputs, at the figures simulate plans them at
            let inputs: BTreeMap<String, f64> = simulated
                .plan_inputs
                .iter()
                .map(|(name, p)| (name.clone(), p.value))
                .collect();
            assert_eq!(plan.at_plan, inputs, "{path}");
            assert!(plan.dry_run && !plan.file_updated, "{path}");
        }
    }

    #[test]
    fn the_bagels_year_at_plan_is_its_plan() {
        let plan = calculate_plan_core(&example("bagel/bagel_simulate.yaml"), None).unwrap();
        let profit = plan.scalars["year.profit"].unwrap();
        assert!((profit - 786.320_361_7).abs() < 1e-6, "{profit}");
        // An input of the included file is planned by its namespace
        assert!(plan.at_plan.keys().any(|k| k.starts_with('@')), "{plan:?}");
    }

    const UNCERTAIN_MODEL: &str = r#"_forge_version: "5.1.0"
inputs:
  price:
    value: 10
    formula: "=MC.Triangular(8, 9, 12)"
  volume:
    formula: "=MC.Normal(100, 5)"
years:
  n: [1, 2]
  shock: "=MC.Uniform(0, 2)"
  sales: "=inputs.volume * shock"
result:
  revenue:
    formula: "=inputs.price * inputs.volume"
scenarios:
  dear:
    probability: 1
    scalars:
      inputs.price: 20
"#;

    fn written(text: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("m.yaml");
        std::fs::write(&file, text).unwrap();
        (dir, file)
    }

    #[test]
    fn each_uncertain_input_is_at_its_value_or_its_likeliest_figure() {
        let (_dir, file) = written(UNCERTAIN_MODEL);
        let plan = calculate_plan_core(&file, None).unwrap();
        // price by its value:, volume by its mean, each row of shock by its midpoint
        let want: BTreeMap<String, f64> = [
            ("inputs.price", 10.0),
            ("inputs.volume", 100.0),
            ("years.shock (row 1)", 1.0),
            ("years.shock (row 2)", 1.0),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        assert_eq!(plan.at_plan, want);
        assert_eq!(plan.scalars["result.revenue"], Some(1000.0));
        assert_eq!(
            plan.tables["years"].values["sales"],
            [serde_json::json!(100.0), serde_json::json!(100.0)]
        );
        // calculate refuses it, as before
        let refused = super::super::calculate_core(&file, true, None).unwrap_err();
        assert!(
            refused.to_string().contains("forge calculate --plan"),
            "{refused}"
        );
    }

    #[test]
    fn a_scenario_that_sets_an_uncertain_input_fixes_it() {
        let (_dir, file) = written(UNCERTAIN_MODEL);
        let plan = calculate_plan_core(&file, Some("dear")).unwrap();
        assert_eq!(plan.scalars["result.revenue"], Some(2000.0));
        assert!(
            !plan.at_plan.contains_key("inputs.price"),
            "{:?}",
            plan.at_plan
        );
        let absent = calculate_plan_core(&file, Some("absent")).unwrap_err();
        assert!(absent.to_string().contains("absent"), "{absent}");
    }

    #[test]
    fn a_distribution_that_does_not_parse_is_named() {
        let (_dir, file) = written(
            "_forge_version: \"5.1.0\"\ninputs:\n  price:\n    formula: \"=MC.Normal(1)\"\n",
        );
        let error = calculate_plan_core(&file, None).unwrap_err().to_string();
        assert!(error.contains("inputs.price"), "{error}");
    }

    #[test]
    fn a_certain_model_calculates_as_it_is() {
        let file = example("bagel/bagel.yaml");
        let plan = calculate_plan_core(&file, None).unwrap();
        let calculated = super::super::calculate_core(&file, true, None).unwrap();
        assert!(plan.at_plan.is_empty());
        assert_eq!(plan.scalars, calculated.scalars);
        let json = serde_json::to_value(&plan).unwrap();
        assert!(json.get("at_plan").is_none(), "{json}");
    }

    #[test]
    fn the_command_prints_the_plan_and_writes_nothing() {
        let (_dir, file) = written(UNCERTAIN_MODEL);
        let (result, captured) =
            capture(|c| super::super::calculate(c, &file, false, false, None, true));
        result.unwrap();
        let out = &captured.out;
        assert!(out.contains("AT PLAN"), "{out}");
        assert!(out.contains("inputs.volume = 100"), "{out}");
        assert!(out.contains("revenue"), "{out}");
        assert_eq!(std::fs::read_to_string(&file).unwrap(), UNCERTAIN_MODEL);

        let (certain_dir, certain) = written(
            "_forge_version: \"5.1.0\"\ninputs:\n  a: 2\nout:\n  b:\n    formula: \"=inputs.a * 3\"\n",
        );
        let (result, captured) =
            capture(|c| super::super::calculate(c, &certain, true, false, None, true));
        result.unwrap();
        let out = &captured.out;
        assert!(out.contains("No input is uncertain"), "{out}");
        drop(certain_dir);

        let (_bad_dir, bad) = written(
            "_forge_version: \"5.1.0\"\ninputs:\n  price:\n    formula: \"=MC.Normal(1)\"\n",
        );
        let (result, _) = capture(|c| super::super::calculate(c, &bad, false, false, None, true));
        assert!(result.unwrap_err().to_string().contains("inputs.price"));
    }

    /// A file that cannot be read is an error at plan too, from the core and
    /// from the command.
    #[test]
    fn a_file_that_cannot_be_read_is_an_error_at_plan() {
        let missing = Path::new("no-such-folder/m.yaml");
        let core = calculate_plan_core(missing, None).unwrap_err().to_string();
        let (result, _) =
            capture(|c| super::super::calculate(c, missing, false, false, None, true));
        assert_eq!(result.unwrap_err().to_string(), core);
    }
}
