//! `forge tornado`: how far each input, from low to high, moves the output.

use super::write_result;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::tornado::{TornadoConfig, TornadoEngine};
use crate::types::ParsedModel;
use colored::Colorize;
use num_traits::ToPrimitive;
use std::path::{Path, PathBuf};

/// The `tornado:` section of the model in `file`, with `output_var` as its
/// output if given, and the model.
fn load(file: &Path, output_var: Option<&str>) -> ForgeResult<(TornadoConfig, ParsedModel)> {
    let yaml_content = parser::read_model_text(file)?;
    let model = parser::parse_model(file)?;
    crate::cli::commands::reject_distributions_in(&model, "tornado")?;

    // The text is YAML: the model was just read from it
    let value: yaml_serde::Value = parser::read_yaml(&yaml_content).unwrap_or_default();

    let mut config: TornadoConfig = if let Some(tornado_value) = value.get("tornado") {
        yaml_serde::from_value(tornado_value.clone())
            .map_err(|e| ForgeError::Validation(format!("tornado config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'tornado' section found in YAML".to_string(),
        ));
    };

    if let Some(out_var) = output_var {
        config.output = out_var.to_string();
    }
    Ok((config, model))
}

/// Generate tornado sensitivity analysis and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no tornado section exists,
/// or the analysis fails.
pub fn tornado_core(
    file: &Path,
    output_var: Option<&str>,
) -> ForgeResult<crate::tornado::TornadoResult> {
    let (config, model) = load(file, output_var)?;
    let engine = TornadoEngine::new(config, model).map_err(ForgeError::Validation)?;
    engine.analyze().map_err(ForgeError::Eval)
}

/// Execute the tornado command - sensitivity tornado diagram
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no tornado section exists,
/// or the analysis fails.
pub fn tornado(
    c: &mut Console<'_>,
    file: &Path,
    output_var: Option<&str>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🌪️ Forge - Tornado Diagram".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c);

    let result = tornado_core(file, output_var)?;

    // Display config
    outln!(
        c,
        "   Output variable: {}",
        result.section.output.bright_blue()
    );
    outln!(c, "   Inputs to vary: {}", result.section.inputs.len());
    if verbose {
        for input in &result.section.inputs {
            outln!(
                c,
                "      {} [{:.2} - {:.2}]",
                input.name.cyan(),
                input.low,
                input.high
            );
        }
    }
    outln!(c);

    if verbose {
        outln!(c, "{}", "🔄 Calculating sensitivities...".cyan());
    }

    // Display results
    outln!(c, "{}", "📊 Tornado Diagram:".bold().green());
    outln!(
        c,
        "   Base value: {}",
        crate::cli::commands::display_number(result.base_value).bold()
    );
    outln!(c);

    // Tornado bars (already sorted by impact)
    let max_impact = result
        .bars
        .iter()
        .map(|b| b.swing.abs())
        .fold(0.0f64, f64::max);

    let name_width = result
        .bars
        .iter()
        .map(|b| b.input_name.chars().count())
        .max()
        .unwrap_or(0);

    for bar in &result.bars {
        // A swing is at most the largest, so 0 to 30 characters; never under 1
        let bar_width = if max_impact > 0.0 {
            ((bar.swing.abs() / max_impact) * 30.0)
                .to_usize()
                .unwrap_or(0)
        } else {
            0
        };
        let bar_str = "█".repeat(bar_width.max(1));

        // `swing` is the output at the high input minus the output at the low input.
        outln!(
            c,
            "   {:name_width$} |{:30}| swing {}  (output {} at low, {} at high)",
            bar.input_name.bright_blue(),
            bar_str.cyan(),
            crate::cli::commands::display_number(bar.swing.abs()),
            crate::cli::commands::display_number(bar.output_at_low),
            crate::cli::commands::display_number(bar.output_at_high)
        );
        if let Some(note) = &bar.note {
            outln!(c, "   {:name_width$}  {}", "", note.yellow());
        }
    }
    outln!(c);

    // Write output if specified
    if let Some(output_path) = output_file {
        write_result(c, &result, &output_path)?;
    }

    outln!(c, "{}", "✅ Tornado diagram complete".bold().green());
    Ok(())
}

#[cfg(test)]
mod tests {
    /// A model with an `MC.*` input is refused with what to run instead
    /// (roadmap E-MC-MESSAGE)
    mod distributions {
        #[test]
        fn tornado_names_the_distribution_and_what_to_run() {
            crate::cli::commands::assert_distributions_refused(&["tornado", "FILE"], "tornado");
        }
    }

    mod tornado_note {
        //! `forge tornado` says when an input cannot move the output, instead of
        //! listing it with a swing of 0 and no reason (roadmap T1-PIPELINE-FINDINGS).

        use crate::cli::forge;

        #[test]
        fn the_printed_tornado_names_the_input_that_cannot_move_the_output() {
            let dir = tempfile::tempdir().expect("temp dir");
            let file = dir.path().join("t.yaml");
            std::fs::write(
                &file,
                r#"_forge_version: "5.1.0"
price: 10
tax_rate: 0.2
revenue: {formula: "=price * 100", value: 1000}
tornado:
  output: revenue
  inputs:
    - {name: price, low: 5, high: 15}
    - {name: tax_rate, low: 0.1, high: 0.3}
"#,
            )
            .expect("write");
            let ran = forge(&["tornado", file.to_str().expect("path")]);
            assert!(ran.success());
            assert_eq!(ran.out.matches("does not use it").count(), 1);
        }
    }

    mod printed_numbers {
        //! The amounts `forge tornado` prints
        //! follow `forge report`'s rule (Session Prompt part 1, item 5): two decimals
        //! and thousands separators, four significant digits under 1, and money with
        //! the sign before the `$`. JSON and files keep the raw numbers.

        use crate::cli::forge;

        #[test]
        fn tornado_prints_amounts_with_separators() {
            let text = forge(&["tornado", "examples/tornado.yaml"]).printed();
            assert!(text.contains("Base value: -625,000.00"), "{text}");
            // initial_investment 700,000 x 1: -425,000 at low, -1,125,000 at high
            assert!(
                text.contains(
                    "swing 700,000.00  (output -425,000.00 at low, -1,125,000.00 at high)"
                ),
                "{text}"
            );
        }

        #[test]
        fn tornado_keeps_the_digits_of_a_rate() {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("rate.yaml");
            // margin = profit / revenue = (1000 - cost) / 1000: cost 840 gives 0.16,
            // cost 903 gives 0.097; the swing is 0.063 (0.06 at two decimals)
            std::fs::write(
                &file,
                r#"_forge_version: "5.1.0"
tornado:
  output: plan.margin
  inputs:
    - name: plan.cost
      low: 840
      high: 903
plan:
  revenue: 1000
  cost: 870
  margin: {formula: "=(plan.revenue - plan.cost) / plan.revenue"}
"#,
            )
            .unwrap();
            let text = forge(&["tornado", file.to_str().unwrap()]).printed();
            assert!(text.contains("swing 0.063"), "{text}");
            assert!(text.contains("0.16 at low"), "{text}");
        }
    }

    mod microgrid_example {
        //! The tornado of examples/microgrid: each input of `tornado:` moved alone
        //! to its low and high, as the `run` of the Python in
        //! prediction/scenarios.rs (`microgrid_example`) gives it:
        //! ```python
        //! run(price=1.10), run(price=1.95)          # 7841813.091454271 20760578.82332357
        //! run(rate=0.06), run(rate=0.10)            # 17355226.48723297 9916484.447146453
        //! run(esc=0.01), run(esc=0.045)             # 9946176.727970488 16072140.515814856
        //! run(growth=0.0), run(growth=0.05)         # 10131332.837667756 14240595.342762135
        //! run(batt_cost=280e3), run(batt_cost=480e3)  # 14397190.43603617 11925419.290764734
        //! run(batt=7), run(batt=13)                 # 10911856.19285597 12169816.338769991
        //! run(yld=1350), run(yld=1700)              # 12187822.241630089 13161304.863400452
        //! ```

        use crate::cli::tornado_core;
        use std::path::Path;

        #[test]
        fn the_diesel_price_moves_the_value_most() {
            let result =
                tornado_core(Path::new("examples/microgrid/microgrid.yaml"), None).unwrap();
            let expected = [
                (
                    "fuel.diesel_price",
                    7_841_813.091_454_271,
                    20_760_578.823_323_57,
                ),
                (
                    "finance.discount_rate",
                    17_355_226.487_232_97,
                    9_916_484.447_146_453,
                ),
                (
                    "fuel.price_escalation",
                    9_946_176.727_970_488,
                    16_072_140.515_814_856,
                ),
                (
                    "demand.growth",
                    10_131_332.837_667_756,
                    14_240_595.342_762_135,
                ),
                (
                    "costs.battery_per_mwh",
                    14_397_190.436_036_17,
                    11_925_419.290_764_734,
                ),
                (
                    "system.battery_mwh",
                    10_911_856.192_855_97,
                    12_169_816.338_769_991,
                ),
                (
                    "system.solar_yield_mwh_per_mwp",
                    12_187_822.241_630_089,
                    13_161_304.863_400_452,
                ),
            ];
            // In order of the swing, largest first
            let names: Vec<_> = result.bars.iter().map(|b| b.input_name.as_str()).collect();
            let want: Vec<_> = expected.iter().map(|(name, _, _)| *name).collect();
            assert_eq!(names, want);
            for (bar, (name, low, high)) in result.bars.iter().zip(expected) {
                assert!(
                    (bar.output_at_low - low).abs() <= 1e-6,
                    "{name} low {}",
                    bar.output_at_low
                );
                assert!(
                    (bar.output_at_high - high).abs() <= 1e-6,
                    "{name} high {}",
                    bar.output_at_high
                );
            }
            // The plan sits at the best battery size: both ends of its range are worse
            let battery = &result.bars[5];
            assert!(battery.output_at_low < result.base_value);
            assert!(battery.output_at_high < result.base_value);
        }
    }

    mod fund_example {
        //! The tornado of examples/fund: each input moved alone, as the `flows`
        //! of the Python in prediction/scenarios.rs (`fund_example`) gives it,
        //! then `xirr(n)`:
        //! ```python
        //! flows(factor=0.7), flows(factor=1.3)          # 0.04373687769500806 0.16112291601241527
        //! flows(delay=-12), flows(delay=12)             # 0.1355861504581191 0.0900660541848615
        //! flows(carry_rate=0.1), flows(carry_rate=0.3)  # 0.11649677373176905 0.09940917752524847
        //! ```

        use crate::cli::tornado_core;
        use std::path::Path;

        #[test]
        fn the_exit_market_moves_the_investors_return_most() {
            let result = tornado_core(Path::new("examples/fund/fund.yaml"), None).unwrap();
            let expected = [
                (
                    "market.multiple_factor",
                    0.043_736_877_695_008_06,
                    0.161_122_916_012_415_27,
                ),
                (
                    "market.delay_months",
                    0.135_586_150_458_119_1,
                    0.090_066_054_184_861_5,
                ),
                (
                    "terms.carry_rate",
                    0.116_496_773_731_769_05,
                    0.099_409_177_525_248_47,
                ),
            ];
            let names: Vec<_> = result.bars.iter().map(|b| b.input_name.as_str()).collect();
            let want: Vec<_> = expected.iter().map(|(name, _, _)| *name).collect();
            assert_eq!(names, want);
            for (bar, (name, low, high)) in result.bars.iter().zip(expected) {
                assert!((bar.output_at_low - low).abs() <= 1e-9, "{name} low");
                assert!((bar.output_at_high - high).abs() <= 1e-9, "{name} high");
            }
        }
    }

    mod vineyard_example {
        //! The tornado of examples/vineyard: each input moved alone, as the
        //! `season` of the Python in prediction/scenarios.rs (`vineyard_example`)
        //! gives it:
        //! ```python
        //! season(y=0.5), season(y=1.3)                # -90429.99999999997 169682.00000000006
        //! season(price=1500), season(price=2500)      # -16659.99999999994 160940.00000000006
        //! season(frost=0), season(frost=0.3)          # 72140.00000000006 -25401.99999999997
        //! ```
        //! Moved alone, the crop swings the season most. Under uncertainty it
        //! barely moves it (`monte_carlo/correlation.rs`, `vineyard_example`): the
        //! price moves against it, which a tornado cannot show.

        use crate::cli::tornado_core;
        use std::path::Path;

        #[test]
        fn the_crop_moves_the_season_most_when_moved_alone() {
            let result = tornado_core(Path::new("examples/vineyard/vineyard.yaml"), None).unwrap();
            let expected = [
                ("harvest.yield_factor", -90_430.0, 169_682.0),
                ("harvest.price_per_tonne", -16_660.0, 160_940.0),
                ("harvest.frost_loss", 72_140.0, -25_402.0),
            ];
            let names: Vec<_> = result.bars.iter().map(|b| b.input_name.as_str()).collect();
            let want: Vec<_> = expected.iter().map(|(name, _, _)| *name).collect();
            assert_eq!(names, want);
            for (bar, (name, low, high)) in result.bars.iter().zip(expected) {
                assert!((bar.output_at_low - low).abs() <= 1e-6, "{name} low");
                assert!((bar.output_at_high - high).abs() <= 1e-6, "{name} high");
            }
        }
    }

    mod realestate_example {
        //! The tornado of examples/realestate: each input of `tornado:` moved alone
        //! to its low and high, as the `run` of the Python in
        //! prediction/scenarios.rs (`realestate_example`) gives it:
        //! ```python
        //! run(growth=0.01), run(growth=0.05)    # -803372.9902072488 1278481.6636162195
        //! run(cap=0.05), run(cap=0.07)          # 586272.8786825631 -356853.2152528884
        //! run(rate=0.07), run(rate=0.09)        # 531800.6356661897 -186627.01265773457
        //! run(vac=0.03), run(vac=0.12)          # 374118.4634693004 -281090.98475902807
        //! run(opex=4600), run(opex=6000)        # 407814.1831154749 -180416.51134763286
        //! run(egrowth=0.015), run(egrowth=0.04) # 311693.32743657194 -102450.19643294066
        //! ```

        use crate::cli::tornado_core;
        use std::path::Path;

        #[test]
        fn rent_growth_moves_the_value_most() {
            let result =
                tornado_core(Path::new("examples/realestate/realestate.yaml"), None).unwrap();
            let expected = [
                (
                    "market.rent_growth",
                    -803_372.990_207_248_8,
                    1_278_481.663_616_219_5,
                ),
                (
                    "market.exit_cap_rate",
                    586_272.878_682_563_1,
                    -356_853.215_252_888_4,
                ),
                (
                    "deal.discount_rate",
                    531_800.635_666_189_7,
                    -186_627.012_657_734_57,
                ),
                (
                    "building.vacancy",
                    374_118.463_469_300_4,
                    -281_090.984_759_028_07,
                ),
                (
                    "building.opex_per_unit_year",
                    407_814.183_115_474_9,
                    -180_416.511_347_632_86,
                ),
                (
                    "market.expense_growth",
                    311_693.327_436_571_94,
                    -102_450.196_432_940_66,
                ),
            ];
            // In order of the swing, largest first
            let names: Vec<_> = result.bars.iter().map(|b| b.input_name.as_str()).collect();
            let want: Vec<_> = expected.iter().map(|(name, _, _)| *name).collect();
            assert_eq!(names, want);
            for (bar, (name, low, high)) in result.bars.iter().zip(expected) {
                assert!(
                    (bar.output_at_low - low).abs() <= 1e-6,
                    "{name} low {}",
                    bar.output_at_low
                );
                assert!(
                    (bar.output_at_high - high).abs() <= 1e-6,
                    "{name} high {}",
                    bar.output_at_high
                );
            }
        }
    }

    mod acquisition_example {
        //! The tornado of examples/acquisition: each input of `tornado:` moved alone
        //! to its low and high, as the `run` of the Python in
        //! prediction/scenarios.rs (`acquisition_example`) gives it:
        //! ```python
        //! run(r=0.075), run(r=0.09)            # 428.94317671721774 -89.5534775250062
        //! run(tg=0.015), run(tg=0.03)          # -32.02129424545046 391.2519313997218
        //! run(cost=20), run(cost=55)           # -12.090702947912177 401.9569106929416
        //! run(premium=0.20), run(premium=0.45) # 346.90793341829067 40.90793341829067
        //! run(rev=0), run(rev=90)              # 28.3974515996465 322.5631743276117
        //! run(growth=0.04), run(growth=0.10)   # 98.45464031669553 361.1597890195958
        //! run(integ=80), run(integ=170)        # 237.73952921161435 178.1973481416553
        //! ```

        use crate::cli::tornado_core;
        use std::path::Path;

        #[test]
        fn the_valuation_assumptions_move_the_value_more_than_any_synergy() {
            let result = tornado_core(Path::new("examples/acquisition/deal.yaml"), None).unwrap();
            let expected = [
                (
                    "case.discount_rate",
                    428.943_176_717_217_74,
                    -89.553_477_525_006_2,
                ),
                (
                    "case.terminal_growth",
                    -32.021_294_245_450_46,
                    391.251_931_399_721_8,
                ),
                (
                    "synergies.cost_run_rate",
                    -12.090_702_947_912_177,
                    401.956_910_692_941_6,
                ),
                (
                    "offer.premium",
                    346.907_933_418_290_67,
                    40.907_933_418_290_67,
                ),
                (
                    "synergies.revenue_run_rate",
                    28.397_451_599_646_5,
                    322.563_174_327_611_7,
                ),
                (
                    "case.quillon_growth",
                    98.454_640_316_695_53,
                    361.159_789_019_595_8,
                ),
                (
                    "synergies.integration_cost",
                    237.739_529_211_614_35,
                    178.197_348_141_655_3,
                ),
            ];
            // In order of the swing, largest first
            let names: Vec<_> = result.bars.iter().map(|b| b.input_name.as_str()).collect();
            let want: Vec<_> = expected.iter().map(|(name, _, _)| *name).collect();
            assert_eq!(names, want);
            for (bar, (name, low, high)) in result.bars.iter().zip(expected) {
                assert!(
                    (bar.output_at_low - low).abs() <= 1e-9,
                    "{name} low {}",
                    bar.output_at_low
                );
                assert!(
                    (bar.output_at_high - high).abs() <= 1e-9,
                    "{name} high {}",
                    bar.output_at_high
                );
            }
        }
    }

    mod command {
        //! `forge tornado` end to end: what it refuses, the lines of `--verbose`,
        //! a bar that does not move, and `-o`.

        use crate::cli::forge;
        use crate::cli::tornado_core;
        use std::path::Path;

        const HEAD: &str = "_forge_version: \"5.1.0\"\nprice: 10\nunit: 4\nrevenue: {formula: \"=price * 100\", value: 1000}\nflat: {formula: \"=unit * 0 + 7\", value: 7}\n";
        const OK: &str =
            "tornado:\n  output: revenue\n  inputs:\n    - {name: price, low: 5, high: 15}\n";

        /// A model with `tornado` appended, in a temporary folder
        fn model(tornado: &str) -> (tempfile::TempDir, String) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("m.yaml");
            std::fs::write(&path, format!("{HEAD}{tornado}")).unwrap();
            let path = path.to_str().unwrap().to_string();
            (dir, path)
        }

        #[test]
        fn a_file_a_section_or_a_config_that_does_not_fit_stops_the_command() {
            let dir = tempfile::tempdir().unwrap();
            let broken = dir.path().join("broken.yaml");
            std::fs::write(&broken, "_forge_version: \"5.1.0\"\nprice: [1\n").unwrap();
            let broken = broken.to_str().unwrap();
            let (_dir, none) = model("");
            let (_dir2, unknown_key) = model("tornado:\n  output: revenue\n  inputs:\n    - {name: price, low: 5, high: 15}\n  foo: 1\n");
            let (_dir3, no_inputs) = model("tornado: {output: revenue, inputs: []}\n");
            let (_dir4, reversed) =
                model("tornado: {output: revenue, inputs: [{name: price, low: 15, high: 5}]}\n");
            let (_dir5, no_output) = model(&OK.replace("output: revenue", "output: nope"));
            for (path, want) in [
                (
                    "/nonexistent/m.yaml",
                    "IO error: /nonexistent/m.yaml: No such file or directory (os error 2)",
                ),
                (
                    broken,
                    "YAML parsing error: did not find expected ',' or ']' at line 3 column 1",
                ),
                (&none, "Validation error: No 'tornado' section found in YAML"),
                (
                    &unknown_key,
                    "tornado config error: unknown field `foo`, expected one of `output`, `inputs`, `steps`",
                ),
                (&no_inputs, "At least one input variable must be specified"),
                (
                    &reversed,
                    "Input 'price': low (15) must be less than high (5)",
                ),
                (
                    &no_output,
                    "Tornado output 'nope': not found; use an existing scalar ID",
                ),
            ] {
                let ran = forge(&["tornado", path]);
                assert!(!ran.success(), "{path}");
                let text = ran.both();
                assert!(text.contains(want), "{want}\n{text}");
                assert!(!text.contains("Tornado diagram complete"), "{text}");
            }
            // The same refusals without printing
            let err = tornado_core(Path::new(&none), None)
                .unwrap_err()
                .to_string();
            assert_eq!(err, "Validation error: No 'tornado' section found in YAML");
            let err = tornado_core(Path::new(&no_inputs), None)
                .unwrap_err()
                .to_string();
            assert_eq!(
                err,
                "Validation error: At least one input variable must be specified"
            );
        }

        #[test]
        fn an_output_that_does_not_calculate_stops_the_analysis() {
            let (_dir, path) = model(
                "unit2: {formula: \"=1 / (price - 10)\", value: 0}\ntornado:\n  output: unit2\n  inputs:\n    - {name: price, low: 5, high: 15}\n",
            );
            let ran = forge(&["tornado", &path]);
            assert!(!ran.success());
            let text = ran.both();
            assert!(
                text.contains("unit2: Division by zero (formula: =1 / (price - 10))"),
                "{text}"
            );
            assert!(!text.contains("Tornado Diagram:"), "{text}");
        }

        #[test]
        fn the_output_can_be_named_on_the_command_line() {
            let (_dir, path) = model(OK);
            let text = forge(&["tornado", &path, "--output-var", "flat"]).printed();
            assert!(text.contains("Output variable: flat"), "{text}");
            assert!(text.contains("Base value: 7.00"), "{text}");
            let result = tornado_core(Path::new(&path), Some("flat")).unwrap();
            assert_eq!(result.output, "flat");
        }

        #[test]
        fn an_input_that_does_not_move_the_output_has_a_bar_of_one_block_and_a_note() {
            let (_dir, path) = model(&OK.replace("output: revenue", "output: flat"));
            let text = forge(&["tornado", &path]).printed();
            assert!(
                text.contains(
                    "   price |█                             | swing 0.00  (output 7.00 at low, 7.00 at high)\n"
                ),
                "{text}"
            );
        }

        #[test]
        fn verbose_lists_the_inputs_and_says_when_it_calculates() {
            let (_dir, path) = model(OK);
            let text = forge(&["tornado", &path, "--verbose"]).printed();
            assert_eq!(
                text,
                format!(
                    "🌪️ Forge - Tornado Diagram\n   File: {path}\n\n   Output variable: revenue\n   Inputs to vary: 1\n      price [5.00 - 15.00]\n\n🔄 Calculating sensitivities...\n📊 Tornado Diagram:\n   Base value: 1,000.00\n\n   price |██████████████████████████████| swing 1,000.00  (output 500.00 at low, 1,500.00 at high)\n\n✅ Tornado diagram complete\n"
                )
            );
        }

        #[test]
        fn the_result_goes_to_a_file_or_the_command_says_why_it_could_not() {
            let (dir, path) = model(OK);
            let out = dir.path().join("o.json");
            let text = forge(&["tornado", &path, "-o", out.to_str().unwrap()]).printed();
            assert!(
                text.contains(&format!("💾 Results written to {}\n", out.display())),
                "{text}"
            );
            let json: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
            assert_eq!(json["bars"][0]["swing"], 1000.0);
            let ran = forge(&["tornado", &path, "-o", "/nonexistent/o.json"]);
            assert!(!ran.success());
            let text = ran.both();
            assert!(
                text.contains("Error: IO error: No such file or directory (os error 2)"),
                "{text}"
            );
            assert!(!text.contains("Results written"), "{text}");
        }
    }

    /// The section as the file writes it is in the result: the names it
    /// uses, before Forge resolves them (T1-CLI-FROM-CORES).
    #[test]
    fn the_result_has_the_section_as_written() {
        let result =
            super::tornado_core(std::path::Path::new("examples/tornado.yaml"), None).unwrap();
        assert_eq!(result.section.output, "valuation.npv");
        let names: Vec<&str> = result
            .section
            .inputs
            .iter()
            .map(|i| i.name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "revenue_growth",
                "operating_margin",
                "discount_rate",
                "tax_rate",
                "initial_investment"
            ]
        );
        assert_eq!(
            (result.section.inputs[0].low, result.section.inputs[0].high),
            (0.05, 0.2)
        );
        // The bars name each input in full
        assert!(result
            .bars
            .iter()
            .all(|b| b.input_name.starts_with("assumptions.")));
        let json = serde_json::to_value(&result).unwrap();
        assert!(json["section"]["inputs"][0].get("base").is_none(), "{json}");
    }
}
