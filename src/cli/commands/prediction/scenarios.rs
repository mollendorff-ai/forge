//! `forge scenarios`: the model under each scenario, weighted by its
//! probability.

use super::serialize_for_file;
use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::scenarios::{ScenarioConfig, ScenarioEngine};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

/// Run scenario analysis and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no scenarios section exists,
/// or the scenario engine fails.
pub fn scenarios_core(
    file: &Path,
    scenario_filter: Option<&str>,
) -> ForgeResult<crate::scenarios::ScenarioResults> {
    let yaml_content = parser::read_model_text(file)?;
    let model = parser::parse_model(file)?;

    let value: yaml_serde::Value = yaml_serde::from_str(&yaml_content)
        .map_err(|e| ForgeError::Validation(format!("YAML parse error: {e}")))?;

    let config: ScenarioConfig = if let Some(scenarios_value) = value.get("scenarios") {
        let scenarios_map: indexmap::IndexMap<String, yaml_serde::Value> =
            yaml_serde::from_value(scenarios_value.clone())
                .map_err(|e| ForgeError::Validation(format!("scenarios config error: {e}")))?;

        let mut config = ScenarioConfig::default();
        for (name, def) in scenarios_map {
            // `forge compare` needs no probability; this command weighs by it
            if def
                .as_mapping()
                .is_some_and(|m| !m.contains_key("probability"))
            {
                return Err(ForgeError::Validation(format!(
                    "scenario '{name}' has no probability. forge scenarios weighs the scenarios \
                     by it, and the probabilities of all scenarios must sum to 1 (forge compare \
                     does not need it)"
                )));
            }
            let scenario_def = yaml_serde::from_value(def)
                .map_err(|e| ForgeError::Validation(format!("scenario '{name}' error: {e}")))?;
            config.scenarios.insert(name, scenario_def);
        }
        config
    } else {
        return Err(ForgeError::Validation(
            "No 'scenarios' section found in YAML".to_string(),
        ));
    };

    if let Some(name) = scenario_filter {
        if !config.scenarios.contains_key(name) {
            let mut available: Vec<_> = config.scenarios.keys().cloned().collect();
            available.sort();
            return Err(ForgeError::Validation(format!(
                "Scenario '{name}' not found. Available scenarios: {available:?}"
            )));
        }
    }

    // Report every calculated scalar (the ones with a formula) as an output,
    // so the probability of a positive result and the range are filled in.
    let outputs: Vec<String> = model
        .scalars
        .iter()
        .filter(|(_, v)| v.formula.is_some())
        .map(|(name, _)| name.clone())
        .collect();
    let engine = ScenarioEngine::new(config, model)
        .map_err(ForgeError::Validation)?
        .with_outputs(outputs);
    let mut results = engine.run().map_err(ForgeError::Eval)?;
    // The expected values and ranges still cover every scenario.
    if let Some(name) = scenario_filter {
        results.scenarios.retain(|s| s.name == name);
    }
    Ok(results)
}

/// Execute the scenarios command - probability-weighted scenario analysis
///
/// # Errors
///
/// Returns an error if the file cannot be parsed, no scenarios section exists,
/// or the scenario engine fails.
pub fn scenarios(
    c: &mut Console<'_>,
    file: &Path,
    scenario_filter: Option<&str>,
    output_file: Option<PathBuf>,
    verbose: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "📊 Forge - Scenario Analysis".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c);

    if verbose {
        match scenario_filter {
            Some(filter) => outln!(c, "{}", format!("🎯 Running scenario: {filter}").cyan()),
            None => outln!(c, "{}", "🔄 Running all scenarios...".cyan()),
        }
    }
    let results = scenarios_core(file, scenario_filter)?;
    // A number with a `format:` is printed as the format shows it (ADR-061)
    let formats = crate::cli::commands::scalar_formats(&parser::parse_model(file)?);
    let shown = |var: &str, value: f64| {
        formats
            .get(var)
            .and_then(|f| crate::cli::commands::number_shown_by(value, f))
            .unwrap_or_else(|| crate::cli::commands::display_number(value))
    };

    // Display results: what each scenario produced (the calculated scalars)
    outln!(c, "{}", "📈 Scenario Results:".bold().green());
    outln!(c);
    for result in &results.scenarios {
        outln!(
            c,
            "   {} (p={}%): {}",
            result.name.bright_blue().bold(),
            crate::cli::commands::format_number(result.probability * 100.0),
            result.description
        );
        for (var, value) in &result.outputs {
            outln!(c, "      {}: {}", var.cyan(), shown(var, *value));
        }
        outln!(c);
    }

    outln!(c, "   {}", "Expected Values (probability-weighted):".bold());
    for (var, ev) in &results.expected_values {
        if !results.ranges.contains_key(var) {
            continue; // an input, not a result
        }
        let (low, high) = results.ranges[var];
        let positive = results
            .probability_positive
            .get(var)
            .copied()
            .unwrap_or(0.0);
        outln!(
            c,
            "      {}: {}   (range {} to {}, P(>0) = {}%)",
            var.bright_blue(),
            shown(var, *ev),
            shown(var, low),
            shown(var, high),
            crate::cli::commands::format_number(positive * 100.0)
        );
    }
    if scenario_filter.is_some() {
        outln!(c, "      (expected values cover every scenario)");
    }
    outln!(c);

    // Write output if specified
    if let Some(output_path) = output_file {
        let output_str = serialize_for_file(&results, &output_path)?;
        fs::write(&output_path, output_str).map_err(ForgeError::Io)?;
        outln!(
            c,
            "{}",
            format!("💾 Results written to {}", output_path.display())
                .bold()
                .green()
        );
    }

    outln!(c, "{}", "✅ Scenario analysis complete".bold().green());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_scenarios_config_parsing() {
        let yaml = r#"
_forge_version: "9.0.0"

scenarios:
  base_case:
    probability: 0.50
    description: "Base case"
    scalars:
      revenue_growth: 0.05
  bull_case:
    probability: 0.50
    scalars:
      revenue_growth: 0.15

scalars:
  revenue_growth:
    value: 0.05
"#;
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "{yaml}").unwrap();

        // Test config parsing
        let value: yaml_serde::Value = yaml_serde::from_str(yaml).unwrap();
        assert!(value.get("scenarios").is_some());
    }

    mod printed_numbers {
        //! The numbers `forge scenarios` prints
        //! follow `forge report`'s rule (Session Prompt part 1, item 5): two decimals
        //! and thousands separators, four significant digits under 1, and money with
        //! the sign before the `$`. JSON and files keep the raw numbers.

        use crate::cli::forge;

        #[test]
        fn scenarios_print_numbers_as_forge_report_does() {
            let text = forge(&["scenarios", "examples/bagel/bagel.yaml"]).printed();
            assert!(text.contains("year.profit: 786.32\n"), "{text}");
            assert!(text.contains("year.revenue: 386,827.45\n"), "{text}");
            assert!(text.contains("year.npv_first_year: -95,579.52\n"), "{text}");
            // year.margin has `format: "0.0%"` (ADR-061, T2-CLI-FORMAT)
            assert!(text.contains("year.margin: 0.2%\n"), "{text}");
            assert!(
                text.contains("(range -71,197.20 to 80,811.24, P(>0) = 70%)"),
                "{text}"
            );
            assert!(!text.contains("786.320362"), "{text}");
        }
    }

    mod microgrid_example {
        //! The numbers in examples/microgrid/README.md, as computed outside Forge.
        //!
        //! The island, year by year, written from the model's description and
        //! not from its formulas:
        //! ```python
        //! def run(mwp=3.4, batt=10.0, price=1.45, esc=0.03, yld=1550.0, growth=0.025,
        //!         demand0=4200.0, batt_cost=380e3, rate=0.08, tenor=10):
        //!     capex = mwp * 1.1e6 + batt * batt_cost + 600e3
        //!     debt = capex * 0.7; ds = debt * 0.065 / (1 - 1.065 ** -tenor)
        //!     for n in range(1, 21):
        //!         d = demand0 * (1 + growth) ** (n - 1)              # demand, MWh
        //!         s = mwp * yld * 0.995 ** (n - 1)                    # solar made
        //!         b = batt * 0.975 ** (n - 11 if n >= 11 else n - 1)  # new battery in year 11
        //!         used = min(s, d * min(0.9, 0.3 + 0.9 * b / (d / 365)))
        //!         dz = d - used; p = price * (1 + esc) ** (n - 1)
        //!         base = d * 1000 / 3.5 * p + d * 35 + 60000
        //!         opex = dz * 1000 / 3.5 * p + dz * 35 + 60000 + mwp * 18000 + b * 8000
        //!         cash = base - opex - (batt * batt_cost * 0.55 if n == 11 else 0)
        //!         # npv at rate less capex; irr by bisection; dscr = cash / ds while n <= tenor;
        //!         # co2 = used * 1000 / 3.5 * 2.68 / 1000; cost per MWh = discounted costs /
        //!         # discounted MWh (the microgrid's with its capex)
        //! run()  # npv 13161304.863400452, irr 0.2301599113695468, equity irr
        //!        # 0.4109006868386767, payback year 5, lowest dscr 1.9645011477468062,
        //!        # co2 67755.24589294654, per MWh 575.8562601026869 (diesel) and
        //!        # 310.0629548448688, curtailed 0.11989415956599792, solar share in year 20
        //!        # 0.6895598431868311
        //! run(price=1.10, esc=0.01)    # cheap diesel: npv 5402750.368024642, dscr 1.487600862698773
        //! run(price=1.95, esc=0.045)   # fuel shock: npv 24675150.907605007, dscr 2.6457872692439968
        //! run(yld=1350, growth=0.05)   # cloudy and hungry: npv 12797727.120385006
        //! # weighted 0.45 / 0.20 / 0.15 / 0.20: 13263955.322352886
        //! ```
        //! The tornado, break-even, sweeps and decision tree of the README are this
        //! function with other arguments; their tests are with those commands.

        use crate::cli::scenarios_core;
        use std::path::Path;

        const MODEL: &str = "examples/microgrid/microgrid.yaml";

        fn close(a: f64, b: f64, tol: f64) -> bool {
            (a - b).abs() <= tol
        }

        #[test]
        fn the_plan_matches_the_independent_calculation() {
            let results = scenarios_core(Path::new(MODEL), Some("base")).unwrap();
            let base = &results.scenarios[0].outputs;
            let expected = [
                ("verdict.project_npv", 13_161_304.863_400_452, 1e-6),
                ("verdict.project_irr", 0.230_159_911_369_546_8, 1e-9),
                ("verdict.equity_irr", 0.410_900_686_838_676_7, 1e-9),
                ("verdict.payback_year", 5.0, 0.0),
                ("verdict.min_dscr", 1.964_501_147_746_806_2, 1e-12),
                ("verdict.co2_avoided_t", 67_755.245_892_946_54, 1e-8),
                ("verdict.lcoe_diesel", 575.856_260_102_686_9, 1e-9),
                ("verdict.lcoe_microgrid", 310.062_954_844_868_8, 1e-9),
                (
                    "verdict.solar_curtailed_share",
                    0.119_894_159_565_997_92,
                    1e-12,
                ),
                ("verdict.first_year_renewable_share", 0.9, 1e-12),
                (
                    "verdict.last_year_renewable_share",
                    0.689_559_843_186_831_1,
                    1e-12,
                ),
            ];
            for (name, want, tol) in expected {
                let got = base[name];
                assert!(close(got, want, tol), "{name}: {got}, not {want}");
            }
            // The tightest year for the lender is the first: the loan is repaid
            // before the battery is replaced in year 11
            assert!(close(base["verdict.min_dscr_year"], 1.0, 0.0));
        }

        #[test]
        fn each_scenario_matches_the_independent_calculation() {
            let results = scenarios_core(Path::new(MODEL), None).unwrap();
            let expected = [
                ("base", 13_161_304.863_400_452, 1.964_501_147_746_806_2),
                ("cheap_diesel", 5_402_750.368_024_642, 1.487_600_862_698_773),
                (
                    "fuel_shock",
                    24_675_150.907_605_007,
                    2.645_787_269_243_996_8,
                ),
                (
                    "cloudy_and_hungry",
                    12_797_727.120_385_006,
                    1.964_501_147_746_806_2,
                ),
            ];
            assert_eq!(results.scenarios.len(), expected.len());
            for (name, npv, dscr) in expected {
                let scenario = results.scenarios.iter().find(|s| s.name == name).unwrap();
                let got = scenario.outputs["verdict.project_npv"];
                assert!(close(got, npv, 1e-6), "{name}: npv {got}, not {npv}");
                let got = scenario.outputs["verdict.min_dscr"];
                assert!(close(got, dscr, 1e-12), "{name}: dscr {got}, not {dscr}");
            }
            let weighted = results.expected_values["verdict.project_npv"];
            assert!(close(weighted, 13_263_955.322_352_886, 1e-6), "{weighted}");
            // Profitable whatever happens; cheap diesel is the case that hurts
            assert!(close(
                results.probability_positive["verdict.project_npv"],
                1.0,
                0.0
            ));
        }
    }

    mod fund_example {
        //! The returns of examples/fund, as computed outside Forge.
        //!
        //! Corvane Seed Fund I, an investor's cash flows on their dates, written
        //! from the README's terms and not from the model's formulas, with scipy
        //! 1.18:
        //! ```python
        //! from datetime import date
        //! import calendar
        //! from scipy.optimize import brentq
        //! def add_months(d, m):  # Excel's EDATE: the same day, or the month's last
        //!     y, mo = divmod(d.month - 1 + m, 12)
        //!     y += d.year; mo += 1
        //!     return date(y, mo, min(d.day, calendar.monthrange(y, mo)[1]))
        //! DEALS = {  # invested, cost, months held, multiple
        //!     "halden": (date(2020, 4, 15), 3.0e6, 60, 0.0), "mirefield": (date(2020, 9, 1), 4.0e6, 72, 1.2),
        //!     "quarto": (date(2021, 1, 20), 3.5e6, 48, 3.5), "ossory": (date(2021, 6, 10), 4.5e6, 84, 0.5),
        //!     "telmar": (date(2021, 11, 30), 3.0e6, 36, 0.0), "brisk": (date(2022, 5, 16), 4.0e6, 66, 8.0),
        //!     "vantor": (date(2022, 10, 3), 3.5e6, 54, 1.8), "lumen": (date(2023, 3, 8), 4.5e6, 78, 2.5)}
        //! FEES = [0.02] * 5 + [0.0175, 0.015, 0.0125, 0.01, 0.01]
        //! def flows(factor=1.0, delay=0, carry_rate=0.2):
        //!     gross = [(inv, -cost) for inv, cost, _, _ in DEALS.values()]
        //!     net = list(gross)
        //!     exits = [(add_months(inv, hold + delay), cost * mult * factor)
        //!              for inv, cost, hold, mult in DEALS.values()]
        //!     for n, rate in enumerate(FEES):  # each March 31 from 2020
        //!         net.append((date(2020 + n, 3, 31), -40e6 * rate))
        //!     paid = sum(c for _, c, _, _ in DEALS.values()) + 40e6 * sum(FEES)
        //!     carry = carry_rate * max(0.0, sum(p for _, p in exits) - paid)
        //!     last = max(d for d, _ in exits)
        //!     gross += exits
        //!     net += [(d, p - (carry if d == last else 0.0)) for d, p in exits]
        //!     return sorted(gross), sorted(net), carry
        //! def xnpv(rate, cf):  # actual/365 days from the first date
        //!     return sum(a / (1 + rate) ** ((d - cf[0][0]).days / 365) for d, a in cf)
        //! def xirr(cf):
        //!     return brentq(lambda r: xnpv(r, cf), -0.9, 10, xtol=1e-15, rtol=1e-15)
        //! ```
        //! For each scenario, `g, n, carry = flows(...)`, then `xirr(g)`,
        //! `xirr(n)` and `xnpv(0.12, n)`:
        //! - plan: 0.15675607686941964, 0.10826144576361177, -1599342.071630658
        //! - `delay=12`: 0.13160355764556161, 0.0900660541848615, -4633065.970461081
        //! - `factor=0.75`: 0.09932808465767194, 0.05553937152662101, -8040127.253786546
        //! - both: 0.08385358625992839, 0.0464737043725587, -10382636.160815407
        //!
        //! Weighted 0.40 / 0.25 / 0.20 / 0.15, the value is -4963424.196147154.
        //! The tornado and the goal seek are this function with other arguments;
        //! their tests are with those commands.

        use crate::cli::scenarios_core;
        use std::path::Path;

        const MODEL: &str = "examples/fund/fund.yaml";

        #[test]
        fn the_returns_of_each_scenario_match_the_independent_calculation() {
            let results = scenarios_core(Path::new(MODEL), None).unwrap();
            let expected = [
                (
                    "base",
                    0.156_756_076_869_419_64,
                    0.108_261_445_763_611_77,
                    -1_599_342.071_630_658,
                    0.0,
                ),
                (
                    "late",
                    0.131_603_557_645_561_61,
                    0.090_066_054_184_861_5,
                    -4_633_065.970_461_081,
                    1.0,
                ),
                (
                    "cold",
                    0.099_328_084_657_671_94,
                    0.055_539_371_526_621_01,
                    -8_040_127.253_786_546,
                    0.0,
                ),
                (
                    "cold_and_late",
                    0.083_853_586_259_928_39,
                    0.046_473_704_372_558_7,
                    -10_382_636.160_815_407,
                    1.0,
                ),
            ];
            assert_eq!(results.scenarios.len(), expected.len());
            for (name, gross, net, value, after_term) in expected {
                let scenario = results.scenarios.iter().find(|s| s.name == name).unwrap();
                let out = |key: &str| scenario.outputs[key];
                // IRRs to 1e-9 and the value to the cent
                assert!(
                    (out("fund.gross_irr") - gross).abs() <= 1e-9,
                    "{name} gross"
                );
                assert!((out("fund.net_irr") - net).abs() <= 1e-9, "{name} net");
                assert!(
                    (out("fund.net_value") - value).abs() <= 0.005,
                    "{name} value"
                );
                // A year late, the last exit (lumen, 8 Sep 2030) is after the
                // fund's ten years (2 Mar 2030)
                assert!(
                    (out("fund.exits_after_term") - after_term).abs() < 1e-12,
                    "{name}"
                );
            }
            let base = &results.scenarios[0].outputs;
            assert!((base["fund.paid_in"] - 36_600_000.0).abs() <= 1e-6);
            assert!((base["fund.carry"] - 6_450_000.0).abs() <= 1e-6);
            assert!((base["fund.tvpi"] - 62_400_000.0 / 36_600_000.0).abs() <= 1e-12);
            assert!((base["fund.gross_multiple"] - 2.295).abs() <= 1e-12);
            let weighted = results.expected_values["fund.net_value"];
            assert!(
                (weighted + 4_963_424.196_147_154).abs() <= 0.005,
                "{weighted}"
            );
        }
    }

    mod vineyard_example {
        //! The plan and scenarios of examples/vineyard, as computed outside Forge.
        //!
        //! Les Trois Pentes, written from the README's description and not from
        //! the model's formulas:
        //! ```python
        //! HA, TPH, PF = [10, 8, 6], [7.5, 5.5, 8.0], [1.00, 1.35, 0.90]
        //! def season(y=1.0, price=2000.0, frost=0.0):
        //!     tonnes = sum(h * t for h, t in zip(HA, TPH)) * y * (1 - frost)
        //!     revenue = sum(h * t * f for h, t, f in zip(HA, TPH, PF)) * y * (1 - frost) * price
        //!     return revenue - 24 * 7000 - 180 * tonnes - 45000 - 40000, revenue, tonnes
        //! season()                  # 72140.00000000006, 355200.00000000006, 167.0
        //! season(1.25, 1550)        # glut: 53525.00000000006
        //! season(0.7, 2500)         # short crop: 36758.0
        //! season(frost=0.3)         # frost: -25401.99999999997
        //! # weighted 0.6 / 0.15 / 0.15 / 0.1: 54286.250000000044
        //! ```
        //! The tornado and the break-even price are this function with other
        //! arguments; their tests are with those commands. The simulation's are
        //! in `monte_carlo/correlation.rs` (`vineyard_example`).

        use crate::cli::scenarios_core;
        use std::path::Path;

        const MODEL: &str = "examples/vineyard/vineyard.yaml";

        #[test]
        fn the_plan_and_each_scenario_match_the_independent_calculation() {
            let results = scenarios_core(Path::new(MODEL), None).unwrap();
            let expected = [
                ("base", 72_140.0, 355_200.0, 167.0),
                ("glut", 53_525.0, 344_100.0, 208.75),
                ("short_crop", 36_758.0, 310_800.0, 116.9),
                ("frost", -25_402.0, 248_640.0, 116.9),
            ];
            assert_eq!(results.scenarios.len(), expected.len());
            for (name, profit, revenue, tonnes) in expected {
                let scenario = results.scenarios.iter().find(|s| s.name == name).unwrap();
                for (output, want) in [
                    ("season.profit", profit),
                    ("season.revenue", revenue),
                    ("season.tonnes", tonnes),
                ] {
                    let got = scenario.outputs[output];
                    assert!((got - want).abs() <= 1e-6, "{name} {output}: {got}");
                }
            }
            let base = &results.scenarios[0].outputs;
            assert!((base["season.margin"] - 0.203_096_846_846_846_98).abs() <= 1e-12);
            // A big crop or a short one costs less than a frost, because the
            // price moves the other way; a frost is this vineyard's alone
            let weighted = results.expected_values["season.profit"];
            assert!((weighted - 54_286.25).abs() <= 1e-6, "{weighted}");
        }
    }

    mod realestate_example {
        //! The numbers in examples/realestate/README.md, as computed outside Forge.
        //!
        //! Linden Court, written from the model's description and not from its
        //! formulas:
        //! ```python
        //! def run(rent=1450.0, growth=0.03, vac=0.06, opex=5200.0, egrowth=0.025,
        //!         cap=0.0575, price=4.6e6, rate=0.08, loan_rate=0.06, units=24):
        //!     def noi(y):  # rent and other income less vacancy, 4% management, costs
        //!         g = (1 + growth) ** (y - 1)
        //!         gross = units * 12 * (rent + 60) * g - units * 12 * rent * g * (vac + 0.01)
        //!         return gross * 0.96 - units * opex * (1 + egrowth) ** (y - 1)
        //!     cash = [noi(y) - units * 350 * (1 + egrowth) ** (y - 1) for y in range(1, 11)]
        //!     sale = noi(11) / cap * 0.98; cost = price * 1.015
        //!     dcf = sum(c / (1 + rate) ** y for y, c in enumerate(cash, 1)) + sale / (1 + rate) ** 10
        //!     loan = price * 0.65; pay = loan * loan_rate / (1 - (1 + loan_rate) ** -30)
        //!     # balance after 10 payments; irr by bisection; dscr = noi / pay
        //! run()  # dcf 4824715.314059857, npv 155715.31405985728, unlevered irr
        //!        # 0.08443306153837382, equity irr 0.11365179107917645, multiple
        //!        # 2.683287063848492, lowest dscr 1.2182201505538217, going-in cap
        //!        # 0.057526539130434776, sale 6196943.26019236
        //! run(growth=0.01, vac=0.10, cap=0.0675)       # soft: -1350622.8124960382, dscr 1.1443971973858285
        //! run(growth=0.045, vac=0.04, cap=0.0525)      # strong: 1487629.9493778758, dscr 1.2551316271378186
        //! run(loan_rate=0.08, rate=0.10, cap=0.07)     # rates up: -925263.637700696, dscr 0.9963404830928924
        //! # weighted 0.50 / 0.25 / 0.15 / 0.10: -129179.91745746916
        //! ```
        //! The tornado and the break-even price of the README are this function with
        //! other arguments; their tests are with those commands.

        use crate::cli::scenarios_core;
        use std::path::Path;

        const MODEL: &str = "examples/realestate/realestate.yaml";

        fn close(a: f64, b: f64, tol: f64) -> bool {
            (a - b).abs() <= tol
        }

        #[test]
        fn the_plan_matches_the_independent_calculation() {
            let results = scenarios_core(Path::new(MODEL), Some("base")).unwrap();
            let base = &results.scenarios[0].outputs;
            let expected = [
                ("valuation.dcf", 4_824_715.314_059_857, 1e-6),
                ("valuation.npv", 155_715.314_059_857_28, 1e-6),
                ("returns.unlevered_irr", 0.084_433_061_538_373_82, 1e-9),
                ("returns.equity_irr", 0.113_651_791_079_176_45, 1e-9),
                ("returns.equity_multiple", 2.683_287_063_848_492, 1e-9),
                ("returns.min_dscr", 1.218_220_150_553_821_7, 1e-12),
                (
                    "valuation.going_in_cap_rate",
                    0.057_526_539_130_434_776,
                    1e-12,
                ),
                ("sale.net_proceeds", 6_196_943.260_192_36, 1e-6),
            ];
            for (name, want, tol) in expected {
                let got = base[name];
                assert!(close(got, want, tol), "{name}: {got}, not {want}");
            }
        }

        #[test]
        fn each_scenario_matches_the_independent_calculation() {
            let results = scenarios_core(Path::new(MODEL), None).unwrap();
            let expected = [
                ("base", 155_715.314_059_857_28, 1.218_220_150_553_821_7),
                (
                    "soft_market",
                    -1_350_622.812_496_038_2,
                    1.144_397_197_385_828_5,
                ),
                (
                    "strong_market",
                    1_487_629.949_377_875_8,
                    1.255_131_627_137_818_6,
                ),
                ("rates_up", -925_263.637_700_696, 0.996_340_483_092_892_4),
            ];
            assert_eq!(results.scenarios.len(), expected.len());
            for (name, npv, dscr) in expected {
                let scenario = results.scenarios.iter().find(|s| s.name == name).unwrap();
                let got = scenario.outputs["valuation.npv"];
                assert!(close(got, npv, 1e-6), "{name}: npv {got}, not {npv}");
                let got = scenario.outputs["returns.min_dscr"];
                assert!(close(got, dscr, 1e-12), "{name}: dscr {got}, not {dscr}");
            }
            // Worth buying in the plan, but not once the scenarios are weighted
            let weighted = results.expected_values["valuation.npv"];
            assert!(close(weighted, -129_179.917_457_469_16, 1e-6), "{weighted}");
            assert!(close(
                results.probability_positive["valuation.npv"],
                0.65,
                1e-12
            ));
        }
    }

    mod acquisition_example {
        //! The numbers in examples/acquisition/README.md, as computed outside Forge.
        //!
        //! Project Kittiwake, written from the model's description and not from
        //! its formulas:
        //! ```python
        //! def run(premium=0.30, cash=0.60, debt_rate=0.065, cost=40.0, cost_ramp=3, rev=60.0,
        //!         rev_ramp=4, integ=100.0, growth=0.07, tg=0.025, r=0.08):
        //!     equity = 24.0 * (1 + premium) * 50; fees = 0.02 * equity
        //!     debt = cash * equity + fees; new_shares = (1 - cash) * equity / 40.0
        //!     paid = equity + 100 + fees; tm = (1 + tg) / (r - tg)
        //!     for y in 1..5:
        //!         buyer = 520 * 1.05 ** (y - 1); target = 78 * (1 + growth) ** (y - 1)
        //!         syn = cost * min(1, y / cost_ramp) + rev * min(1, (y - 1) / rev_ramp) * 0.30
        //!         ig = integ * [0.5, 0.35, 0.15, 0, 0][y - 1]
        //!         ni = buyer + target + (syn - ig - debt * debt_rate - 40) * 0.75
        //!         accretion = (ni / (200 + new_shares)) / (buyer / 200) - 1
        //!         target_cash = 62 * (1 + growth) ** (y - 1); synergy_cash = (syn - ig) * 0.75
        //!     # each cash stream: discounted at r, year 5 * tm as a value at year 5
        //!     value = standalone + synergies - paid; irr by bisection
        //! run()  # value 224.50793341829012, standalone 1312.5581578338633, synergies
        //!        # 603.149775584427, irr 0.10893359643045575, accretion year 1
        //!        # -0.11989974311402884, year 3 -0.011433658663979496, year 5
        //!        # 0.02831107907802166, adjusted year 1 0.0005155558726985809, first
        //!        # accretive year 4, leverage 2.042522522522523, cover 1.6754160432900738,
        //!        # synergies after year 5 0.9147585656658958
        //! run(cost_ramp=5, integ=150)       # slow integration: 166.93036365984085
        //! run(rev=0)                        # no revenue synergies: 28.3974515996465
        //! run(debt_rate=0.08, r=0.09)       # rates up: -89.5534775250062
        //! # weighted 0.45 / 0.25 / 0.20 / 0.10: 139.48530352061945
        //! ```
        //! The tornado, the walk-away price, the goal seeks, the sensitivity table,
        //! the year-one variance and the bidding tree are this function with other
        //! arguments; their tests are with those commands (`acquisition_*`).

        use crate::cli::scenarios_core;
        use std::path::Path;

        const MODEL: &str = "examples/acquisition/deal.yaml";

        fn close(a: f64, b: f64, tol: f64) -> bool {
            (a - b).abs() <= tol
        }

        #[test]
        fn the_plan_matches_the_independent_calculation() {
            let results = scenarios_core(Path::new(MODEL), Some("base")).unwrap();
            let base = &results.scenarios[0].outputs;
            let expected = [
                ("valuation.value_created", 224.507_933_418_290_12, 1e-9),
                ("valuation.standalone", 1_312.558_157_833_863_3, 1e-9),
                ("valuation.synergies", 603.149_775_584_427, 1e-9),
                ("valuation.deal_irr", 0.108_933_596_430_455_75, 1e-9),
                ("valuation.synergy_cover", 1.675_416_043_290_073_8, 1e-12),
                (
                    "valuation.synergies_after_year_5",
                    0.914_758_565_665_895_8,
                    1e-12,
                ),
                ("eps.year_1_accretion", -0.119_899_743_114_028_84, 1e-12),
                ("eps.year_3_accretion", -0.011_433_658_663_979_496, 1e-12),
                ("eps.year_5_accretion", 0.028_311_079_078_021_66, 1e-12),
                (
                    "eps.year_1_adjusted_accretion",
                    0.000_515_555_872_698_580_9,
                    1e-12,
                ),
                ("eps.first_accretive_year", 4.0, 0.0),
                ("financing.leverage_at_close", 2.042_522_522_522_523, 1e-12),
            ];
            for (name, want, tol) in expected {
                let got = base[name];
                assert!(close(got, want, tol), "{name}: {got}, not {want}");
            }
        }

        #[test]
        fn each_scenario_matches_the_independent_calculation() {
            let results = scenarios_core(Path::new(MODEL), None).unwrap();
            let expected = [
                ("base", 224.507_933_418_290_12),
                ("slow_integration", 166.930_363_659_840_85),
                ("no_revenue_synergies", 28.397_451_599_646_5),
                ("rates_up", -89.553_477_525_006_2),
            ];
            assert_eq!(results.scenarios.len(), expected.len());
            for (name, value) in expected {
                let scenario = results.scenarios.iter().find(|s| s.name == name).unwrap();
                let got = scenario.outputs["valuation.value_created"];
                assert!(close(got, value, 1e-9), "{name}: {got}, not {value}");
            }
            // Without the revenue synergies the deal barely pays for itself, and
            // weighted, it creates little more than half the plan's value
            let weighted = results.expected_values["valuation.value_created"];
            assert!(close(weighted, 139.485_303_520_619_45, 1e-9), "{weighted}");
            assert!(close(
                results.probability_positive["valuation.value_created"],
                0.9,
                1e-12
            ));
        }
    }
}
