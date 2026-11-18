//! `forge tornado`: how far each input, from low to high, moves the output.

use super::serialize_for_file;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::tornado::{TornadoConfig, TornadoEngine};
use colored::Colorize;
use num_traits::ToPrimitive;
use std::fs;
use std::path::{Path, PathBuf};

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
    let yaml_content = parser::read_model_text(file)?;
    let model = parser::parse_model(file)?;

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

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

    // Parse YAML and model
    let yaml_content = parser::read_model_text(file)?;
    let model = parser::parse_model(file)?;

    // Parse tornado config
    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let mut config: TornadoConfig = if let Some(tornado_value) = value.get("tornado") {
        yaml_serde::from_value(tornado_value.clone())
            .map_err(|e| ForgeError::Validation(format!("tornado config error: {e}")))?
    } else {
        return Err(ForgeError::Validation(
            "No 'tornado' section found in YAML".to_string(),
        ));
    };

    // Override output variable if specified
    if let Some(out_var) = output_var {
        config.output = out_var.to_string();
    }

    // Display config
    outln!(c, "   Output variable: {}", config.output.bright_blue());
    outln!(c, "   Inputs to vary: {}", config.inputs.len());
    if verbose {
        for input in &config.inputs {
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

    // Create engine and analyze
    let engine = TornadoEngine::new(config, model).map_err(ForgeError::Validation)?;

    if verbose {
        outln!(c, "{}", "🔄 Calculating sensitivities...".cyan());
    }

    let result = engine.analyze().map_err(ForgeError::Eval)?;

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
        let output_str = serialize_for_file(&result, &output_path)?;
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        outln!(
            c,
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    outln!(c, "{}", "✅ Tornado diagram complete".bold().green());
    Ok(())
}

#[cfg(test)]
mod tests {

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
            assert!(ran.success(), "{}", ran.both());
            assert_eq!(ran.out.matches("does not use it").count(), 1, "{}", ran.out);
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
            assert!(
                text.contains("0.16 at low") || text.contains("0.160 at low"),
                "{text}"
            );
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
}
