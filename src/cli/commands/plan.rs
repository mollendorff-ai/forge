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

/// `model` made ready for a what-if `command` (scenarios, compare,
/// goal-seek, break-even, sensitivity, tornado, audit): with `plan`, each
/// uncertain input at its plan figure, as `forge calculate --plan` sets them
/// (ADR-076, addendum of 2026-11-26), and without it, refused if it has an
/// uncertain input, the refusal naming `--plan`.
///
/// A scenario applied after this sets an uncertain input to its own number,
/// as [`calculate_plan_core`] lets it, which applies the scenario first.
///
/// # Errors
///
/// Returns an error if a distribution does not parse, or, without `plan`,
/// if the model has an uncertain input.
pub(super) fn at_plan_or_refused(
    model: &mut ParsedModel,
    plan: bool,
    command: &str,
) -> ForgeResult<()> {
    if plan {
        set_at_plan(model).map(drop)
    } else {
        super::reject_distributions_in(model, command)
    }
}

/// What a what-if command says before it runs at plan, when it runs at plan.
pub(super) fn print_what_if_plan(c: &mut Console<'_>, plan: bool) {
    if plan {
        outln!(
            c,
            "{}",
            "📋 AT PLAN - each uncertain input at its plan figure\n".yellow()
        );
    }
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

/// The what-if commands at plan (ADR-076, addendum of 2026-11-26): a
/// launch whose price is uncertain, planned at 20 (its value:), so that
/// every figure is worked by hand: profit is price x units.
#[cfg(test)]
pub const LAUNCH: &str = r#"_forge_version: "5.1.0"
assumptions:
  price:
    value: 20
    formula: "=MC.Triangular(15, 20, 25)"
  units:
    value: 100
outputs:
  profit:
    value: 2000
    formula: "=assumptions.price * assumptions.units"
scenarios:
  low:
    probability: 0.5
    scalars:
      assumptions.units: 50
  high:
    probability: 0.5
    scalars:
      assumptions.units: 150
  dear:
    scalars:
      assumptions.price: 30
tornado:
  output: outputs.profit
  inputs:
    - name: assumptions.units
      low: 50
      high: 150
    - name: assumptions.price
      low: 15
      high: 25
"#;

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

    /// Scenarios: 20 x 50 and 20 x 150, half each, 2,000 on average; the
    /// uncertain price is an input at plan, not an output
    #[test]
    fn scenarios_at_plan_weigh_the_plan() {
        let (_dir, file) = written(LAUNCH);
        let results = super::super::scenarios_core(&file, None, true).unwrap();
        let profit = |name: &str| {
            results
                .scenarios
                .iter()
                .find(|s| s.name == name)
                .map(|s| s.outputs["outputs.profit"])
        };
        assert_eq!(profit("low"), Some(1000.0));
        assert_eq!(profit("high"), Some(3000.0));
        assert_eq!(
            results.expected_values["outputs.profit"].to_bits(),
            2000.0_f64.to_bits()
        );
        assert_eq!(results.unweighted, ["dear"]);
        assert!(
            results
                .scenarios
                .iter()
                .all(|s| !s.outputs.contains_key("assumptions.price")),
            "{results:?}"
        );
        // Without plan, refused with what to run instead
        let refused = super::super::scenarios_core(&file, None, false)
            .unwrap_err()
            .to_string();
        assert!(refused.contains("assumptions.price"), "{refused}");
        assert!(refused.contains("`--plan`"), "{refused}");
    }

    /// Compare: the price at plan, 20, beside a scenario that sets it to 30,
    /// whose own number wins (20 x 50 and 30 x 100)
    #[test]
    fn compare_at_plan_lets_a_scenario_set_an_uncertain_input() {
        let (_dir, file) = written(LAUNCH);
        let both = ["low".to_string(), "dear".to_string()];
        let result = super::super::compare_core(&file, &both, true).unwrap();
        let at = |var: &str, scenario: &str| result.values[var][scenario];
        assert_eq!(at("outputs.profit", "low"), Some(1000.0));
        assert_eq!(at("outputs.profit", "dear"), Some(3000.0));
        assert_eq!(at("assumptions.price", "low"), Some(20.0));
        assert_eq!(at("assumptions.price", "dear"), Some(30.0));
        // The same as calculate --plan in that scenario
        let dear = calculate_plan_core(&file, Some("dear")).unwrap();
        assert_eq!(dear.scalars["outputs.profit"], Some(3000.0));
        assert!(super::super::compare_core(&file, &both, false).is_err());
    }

    /// Tornado: units from 50 to 150 at the planned price, 1,000 to 3,000;
    /// the uncertain price from 15 to 25 at 100 units, 1,500 to 2,500
    #[test]
    fn a_tornado_at_plan_varies_an_uncertain_input_too() {
        let (_dir, file) = written(LAUNCH);
        let result = crate::cli::tornado_core(&file, None, true).unwrap();
        assert_eq!(result.base_value.to_bits(), 2000.0_f64.to_bits());
        let swing = |name: &str| {
            result
                .bars
                .iter()
                .find(|b| b.input_name == name)
                .map(|b| (b.output_at_low, b.output_at_high))
        };
        assert_eq!(swing("assumptions.units"), Some((1000.0, 3000.0)));
        assert_eq!(swing("assumptions.price"), Some((1500.0, 2500.0)));
        assert!(crate::cli::tornado_core(&file, None, false).is_err());
    }

    /// Goal seek: 2,500 at a price of 20 is 125 units; break-even between
    /// -10 and 10 units, 0; sensitivity of the price at 100 units, 1,500 to
    /// 2,500
    #[test]
    fn goal_seek_break_even_and_sensitivity_at_plan() {
        let (_dir, file) = written(LAUNCH);
        let seek = |value: f64, bounds: (Option<f64>, Option<f64>), plan: bool| {
            crate::cli::goal_seek_core(
                &file,
                "outputs.profit",
                value,
                "assumptions.units",
                bounds,
                1e-9,
                plan,
            )
        };
        let units = seek(2500.0, (None, None), true).unwrap().solution;
        assert!((units - 125.0).abs() < 1e-6, "{units}");
        let even = seek(0.0, (Some(-10.0), Some(10.0)), true).unwrap().solution;
        assert!(even.abs() < 1e-6, "{even}");
        assert!(seek(2500.0, (None, None), false).is_err());

        let sensitivity = |plan: bool| {
            crate::cli::sensitivity_core(
                &file,
                "assumptions.price",
                "15,25,5",
                None,
                None,
                "outputs.profit",
                plan,
            )
        };
        let result = serde_json::to_value(sensitivity(true).unwrap()).unwrap_or_default();
        let outputs: Vec<Option<f64>> = (0..3)
            .map(|i| result["data"]["OneVar"]["entries"][i]["output"].as_f64())
            .collect();
        assert_eq!(outputs, [Some(1500.0), Some(2000.0), Some(2500.0)]);
        assert!(sensitivity(false).is_err());
    }

    /// Audit: the profit calculated at plan, 2,000, and no error; without
    /// plan, the error says what to run
    #[test]
    fn an_audit_at_plan_calculates_the_plan() {
        let (_dir, file) = written(LAUNCH);
        let at_plan = super::super::audit::audit_core(&file, "outputs.profit", true).unwrap();
        assert_eq!(at_plan.calculated_value, Some(2000.0));
        assert_eq!(at_plan.calculation_error, None);
        let refused = super::super::audit::audit_core(&file, "outputs.profit", false).unwrap();
        let error = refused.calculation_error.unwrap_or_default();
        assert!(error.contains("`--plan`"), "{error}");
    }

    /// A distribution that does not parse is named at plan, by each command
    #[test]
    fn a_distribution_that_does_not_parse_is_named_by_the_what_if_commands() {
        let (_dir, file) = written(&LAUNCH.replace("MC.Triangular(15, 20, 25)", "MC.Normal(1)"));
        let errors = [
            super::super::scenarios_core(&file, None, true).map(drop),
            crate::cli::tornado_core(&file, None, true).map(drop),
            super::super::compare_core(&file, &["low".to_string()], true).map(drop),
        ];
        for error in errors {
            let error = error.unwrap_err().to_string();
            assert!(error.contains("assumptions.price"), "{error}");
        }
        let audit = super::super::audit::audit_core(&file, "outputs.profit", true).unwrap();
        let error = audit.calculation_error.unwrap_or_default();
        assert!(error.contains("assumptions.price"), "{error}");
    }

    /// Each what-if command takes --plan, says so, and prints the hand
    /// figures; without it, each is refused with what to run instead
    #[test]
    fn each_what_if_command_takes_plan() {
        let (_dir, file) = written(LAUNCH);
        let file = file.to_string_lossy().to_string();
        let runs: [(&[&str], &str); 7] = [
            (&["scenarios", &file], "2,000"),
            (&["compare", &file, "--scenarios", "low,dear"], "3,000"),
            (&["tornado", &file], "1,500"),
            (
                &[
                    "goal-seek",
                    &file,
                    "--target",
                    "outputs.profit",
                    "--value",
                    "2500",
                    "--vary",
                    "assumptions.units",
                ],
                "assumptions.units = 125",
            ),
            (
                &[
                    "break-even",
                    &file,
                    "--output",
                    "outputs.profit",
                    "--vary",
                    "assumptions.price",
                    "--min=-10",
                    "--max=10",
                ],
                "assumptions.price = 0",
            ),
            (
                &[
                    "sensitivity",
                    &file,
                    "--vary",
                    "assumptions.price",
                    "--range",
                    "15,25,5",
                    "--output",
                    "outputs.profit",
                ],
                "2,500",
            ),
            (&["audit", &file, "outputs.profit"], "Calculated: 2000"),
        ];
        for (args, figure) in runs {
            let at_plan: Vec<&str> = args.iter().copied().chain(["--plan"]).collect();
            let ran = crate::cli::forge(&at_plan);
            let said = ran.both();
            assert!(said.contains("AT PLAN"), "{args:?}: {said}");
            assert!(said.contains(figure), "{args:?}: {said}");
            assert!(!said.contains("Error"), "{args:?}: {said}");
            let refused = crate::cli::forge(args).both();
            assert!(refused.contains("`--plan`"), "{args:?}: {refused}");
        }
    }

    /// The acceptance of F-WHAT-IF-PLAN: the bagel's simulation file at plan
    /// is the plan of bagel.yaml, by break-even (15.97) and sensitivity (the
    /// year's 786.32 at the planned price of 16), and its one scenario, which
    /// has no probability, is compared as calculate --plan gives it
    #[test]
    fn the_bagels_simulation_at_plan_is_its_plan() {
        let uncertain = example("bagel/bagel_simulate.yaml");
        let plan = example("bagel/bagel.yaml");
        let even = |file: &Path, at_plan: bool| {
            crate::cli::goal_seek_core(
                file,
                "year.profit",
                0.0,
                "assumptions.price_per_dozen",
                (None, None),
                0.0001,
                at_plan,
            )
            .unwrap()
            .solution
        };
        let price = even(&uncertain, true);
        assert!((price - even(&plan, false)).abs() < 1e-9, "{price}");
        assert!((price - 15.97).abs() < 0.005, "{price}");

        let result = crate::cli::sensitivity_core(
            &uncertain,
            "assumptions.price_per_dozen",
            "16,16,1",
            None,
            None,
            "year.profit",
            true,
        )
        .unwrap();
        let result = serde_json::to_value(result).unwrap_or_default();
        let year = result["data"]["OneVar"]["entries"][0]["output"]
            .as_f64()
            .unwrap_or_default();
        assert!((year - 786.320_361_7).abs() < 1e-6, "{year}");

        let rent = super::super::compare_core(&uncertain, &["rent_hike".to_string()], true)
            .unwrap()
            .values["year.profit"]["rent_hike"];
        let calculated = calculate_plan_core(&uncertain, Some("rent_hike")).unwrap();
        let (rent, calculated) = (
            rent.unwrap_or_default(),
            calculated.scalars["year.profit"].unwrap_or_default(),
        );
        // calculate gives 15 significant digits
        assert!((rent - calculated).abs() < 1e-9, "{rent} {calculated}");
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
