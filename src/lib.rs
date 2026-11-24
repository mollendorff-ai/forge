//! Forge: financial models as YAML, calculated with Excel's formulas.
//!
//! A model is a text file of inputs, tables and formulas. Forge reads it,
//! works out every formula in dependency order and gives the numbers back,
//! and the same engine answers a command line, an MCP server for AI agents,
//! a REST API and a browser (`forge-wasm`). This crate is that engine as a
//! library, with the 203 functions Forge's own formulas can use, the analyses
//! (Monte Carlo, scenarios, break-even and goal seek, decision trees, real
//! options, tornado, bootstrap, Bayesian networks, games) and Excel export
//! and import.
//!
//! A reader who wants to use Forge rather than build on it starts at the
//! [README](https://github.com/mollendorff-ai/forge#readme); one who embeds
//! it, at [docs/EMBEDDING.md](https://github.com/mollendorff-ai/forge/blob/main/docs/EMBEDDING.md).
//!
//! # A model in memory
//!
//! [`parser::parse_model_text`] reads a model from a string, reading nothing
//! from the file system, and [`core::ArrayCalculator`] calculates it:
//!
//! ```
//! use std::collections::HashMap;
//!
//! use mollendorff_forge::core::ArrayCalculator;
//! use mollendorff_forge::parser::parse_model_text;
//!
//! let text = r#"
//! _forge_version: "5.1.0"
//! plan:
//!   price: 40
//!   units: 120
//!   revenue: {formula: "=plan.price * plan.units"}
//! sales:
//!   units: [10, 20, 30]
//!   revenue: "=units * plan.price"
//! "#;
//!
//! let model = parse_model_text("plan.yaml", text, &HashMap::new())?;
//! let calculated = ArrayCalculator::new(model).calculate_all()?;
//!
//! assert_eq!(calculated.scalars["plan.revenue"].value, Some(4800.0));
//! # Ok::<(), mollendorff_forge::ForgeError>(())
//! ```
//!
//! A file on disk is read with [`parser::parse_model`]. A model with
//! `_includes` needs the text of each included file in the map, by the path
//! the model writes.
//!
//! # Features
//!
//! - `cli` (default): the command line, the MCP server and the REST API, and
//!   `excel`.
//! - `excel`: [`excel`], the workbook export and import.
//! - With `default-features = false` the crate is the parser, the
//!   calculator, the analysis engines and the commands' `*_core` functions,
//!   which build for `wasm32-unknown-unknown`.
//!
//! # Where to look
//!
//! | Module | What it holds |
//! | --- | --- |
//! | [`parser`] | Reading a model, from a file or from memory |
//! | [`core`] | The calculator: formulas, dependency order, the functions' evaluation |
//! | [`types`] | `ParsedModel`, its tables and scalars |
//! | [`functions`] | The registry of the functions, with their syntax and descriptions |
//! | [`edit`] | Changing a model's text and keeping its comments |
//! | [`writer`] | Writing results back into a model's file |
//! | [`monte_carlo`], [`scenarios`], [`tornado`], [`decision_trees`], [`real_options`], [`bootstrap`], [`bayesian`], [`games`] | The analyses |
//! | [`cli`] | The commands: each has a `*_core` function that returns a result and prints nothing |

/// `assert_eq!` for two numbers that must be exactly equal: the same bits,
/// so a result off in its last digit, or `-0.0` for `0.0`, fails. A test
/// states the exact value the engine gives (checked against Excel, Gnumeric
/// or R), and an approximate one compares with a tolerance instead.
#[cfg(test)]
macro_rules! assert_exact {
    ($got:expr, $want:expr $(,)?) => {{
        let (got, want): (f64, f64) = ($got, $want);
        assert!(
            got.to_bits() == want.to_bits(),
            "{} is {got:?}, not exactly {want:?}",
            stringify!($got)
        );
    }};
    ($got:expr, $want:expr, $($message:tt)+) => {{
        let (got, want): (f64, f64) = ($got, $want);
        assert!(
            got.to_bits() == want.to_bits(),
            "{} is {got:?}, not exactly {want:?}: {}",
            stringify!($got),
            format_args!($($message)+)
        );
    }};
}

pub mod core;
pub mod edit;
pub mod error;
pub mod functions;
pub mod parser;
pub mod types;
pub mod writer;

// Analysis modules
pub mod bayesian;
pub mod bootstrap;
pub mod decision_trees;
pub mod games;
pub mod monte_carlo;
pub mod real_options;
pub mod scenarios;
pub mod tornado;

// Excel export and import (feature `excel`, part of the default `cli`)
#[cfg(feature = "excel")]
pub mod excel;

// The commands. Their core functions (`calculate_core`, `simulate_core`, ...)
// build without the `cli` feature, for a browser (ADR-055); watch, update,
// export and import need it.
pub mod cli;

// The MCP server and the REST API (feature `cli`, the default)
#[cfg(feature = "cli")]
pub mod api;
#[cfg(feature = "cli")]
pub mod inline_model;
#[cfg(feature = "cli")]
pub mod mcp;
#[cfg(feature = "cli")]
pub mod path_root;

/// The version of Forge, as in Cargo.toml (`10.0.0-rc.13`)
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Re-export commonly used types
pub use error::{ForgeError, ForgeResult};
pub use types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[cfg(test)]
mod tests {

    mod config_strictness {
        //! Unknown keys are errors, not silently ignored
        //!
        //! A misspelt or unsupported key used to be dropped, so the model ran with a
        //! default instead (for example `expansion_cost` gave an Expand option a cost
        //! of 0). Each case below names a key that some file actually used.

        use crate::bayesian::BayesianConfig;
        use crate::bootstrap::BootstrapConfig;
        use crate::decision_trees::DecisionTreeConfig;
        use crate::monte_carlo::MonteCarloConfig;
        use crate::parser::validate_against_schema;
        use crate::real_options::RealOptionsConfig;
        use crate::scenarios::ScenarioDefinition;
        use crate::tornado::TornadoConfig;
        use serde::de::DeserializeOwned;

        fn rejects<T: DeserializeOwned>(yaml: &str, key: &str) {
            let err = yaml_serde::from_str::<T>(yaml)
                .err()
                .map(|e| e.to_string())
                .unwrap_or_default();
            assert!(
                err.contains(&format!("unknown field `{key}`")),
                "`{key}` gave `{err}` in:\n{yaml}"
            );
        }

        fn accepts<T: DeserializeOwned>(yaml: &str) {
            assert!(
                yaml_serde::from_str::<T>(yaml).is_ok(),
                "rejected valid config:\n{yaml}"
            );
        }

        const UNDERLYING: &str =
            "underlying: {current_value: 100, volatility: 0.3, risk_free_rate: 0.05, time_horizon: 1}\n";

        #[test]
        fn test_real_options_unknown_keys() {
            accepts::<RealOptionsConfig>(&format!(
                "{UNDERLYING}binomial_steps: 50\noptions:\n  - {{type: expand, name: e, exercise_cost: 5, expansion_factor: 2}}\n"
            ));
            rejects::<RealOptionsConfig>(
                &format!(
                    "{UNDERLYING}options:\n  - {{type: expand, name: e, expansion_cost: 5}}\n"
                ),
                "expansion_cost",
            );
            rejects::<RealOptionsConfig>(
                &format!(
                    "{UNDERLYING}options:\n  - {{type: abandon, name: a, available_after: 1}}\n"
                ),
                "available_after",
            );
            rejects::<RealOptionsConfig>(
                "underlying: {current_value: 100, volatility: 0.3, risk_free_rate: 0.05, time_horizon: 1, steps: 50}\n",
                "steps",
            );
            rejects::<RealOptionsConfig>(
                &format!("{UNDERLYING}investment: {{cost: 10}}\n"),
                "investment",
            );
        }

        #[test]
        fn test_tornado_unknown_keys() {
            accepts::<TornadoConfig>(
                "output: npv\ninputs:\n  - {name: g, low: 0.1, high: 0.2, base: 0.15}\n",
            );
            rejects::<TornadoConfig>("output: npv\nbase_values: {g: 0.1}\n", "base_values");
            rejects::<TornadoConfig>(
                "output: npv\ninputs:\n  - {name: g, low: 0.1, high: 0.2, label: Growth}\n",
                "label",
            );
        }

        #[test]
        fn test_bootstrap_unknown_keys() {
            accepts::<BootstrapConfig>(
                "data: [1, 2, 3]\nstatistic: percentile\npercentile_value: 5\n",
            );
            rejects::<BootstrapConfig>(
                "data: [1, 2, 3]\nstatistics:\n  - {type: mean}\n",
                "statistics",
            );
        }

        #[test]
        fn test_monte_carlo_unknown_keys() {
            accepts::<MonteCarloConfig>(
                "iterations: 100\nseed: 1\noutputs:\n  - {variable: npv, percentiles: [10, 90], threshold: \"> 0\"}\n",
            );
            rejects::<MonteCarloConfig>("iteration: 100\n", "iteration");
            rejects::<MonteCarloConfig>(
                "outputs:\n  - {variable: npv, percentile: [10]}\n",
                "percentile",
            );
            rejects::<MonteCarloConfig>(
                "correlations:\n  - {variables: [a, b], correlation: 0.5}\n",
                "correlation",
            );
        }

        #[test]
        fn test_decision_tree_unknown_keys() {
            accepts::<DecisionTreeConfig>(
                "root:\n  type: decision\n  branches:\n    go: {cost: 10, value: 100}\n",
            );
            rejects::<DecisionTreeConfig>(
                "root:\n  type: chance\n  branches:\n    up: {probabilty: 0.6, value: 100}\n",
                "probabilty",
            );
            rejects::<DecisionTreeConfig>("root:\n  type: decision\n  options: {}\n", "options");
        }

        #[test]
        fn test_bayesian_and_scenario_unknown_keys() {
            accepts::<BayesianConfig>(
                "nodes:\n  rain: {type: discrete, states: [y, n], prior: [0.2, 0.8]}\n",
            );
            rejects::<BayesianConfig>(
                "nodes:\n  rain: {type: discrete, states: [y, n], priors: [0.2, 0.8]}\n",
                "priors",
            );
            accepts::<ScenarioDefinition>("probability: 0.3\nscalars: {growth: 0.1}\n");
            rejects::<ScenarioDefinition>("probability: 0.3\ngrowth: 0.1\n", "growth");
        }

        #[test]
        fn test_schema_rejects_unknown_scalar_keys() {
            let validate =
                |yaml: &str| validate_against_schema(&yaml_serde::from_str(yaml).unwrap());
            // A misspelt `formula` used to pass validation, and the scalar was never calculated
            for yaml in [
                "_forge_version: \"5.0.0\"\ns:\n  b: {value: null, fromula: \"=1 + 1\"}\n",
                "_forge_version: \"1.0.0\"\ns:\n  b: {value: null, fromula: \"=1 + 1\"}\n",
                "_forge_version: \"5.0.0\"\ns:\n  b: {value: 2, formula: null, expected: 2}\n",
                "_forge_version: \"5.0.0\"\nt:\n  x: {value: [1, 2], units: USD}\n",
                "_forge_version: \"5.0.0\"\nt:\n  x: [1, 2]\n  y: {formula: \"=x * 2\", note: double}\n",
            ] {
                assert!(validate(yaml).is_err(), "accepted:\n{yaml}");
            }
            for yaml in [
                "_forge_version: \"5.0.0\"\ns:\n  b: {value: 2, formula: null, unit: USD, notes: n, source: s, validation_status: VALIDATED, last_updated: \"2026-11-05\"}\n",
                "_forge_version: \"5.0.0\"\nt:\n  x: {value: [1, 2], unit: USD, notes: n}\n  y: {formula: \"=x * 2\", unit: USD}\n",
            ] {
                let verdict = validate(yaml);
                assert!(verdict.is_ok(), "rejected:\n{yaml}\n{verdict:?}");
            }
        }
    }

    mod properties {
        //! Rules that must hold for any model, checked on generated models with
        //! `proptest` (roadmap T1-PROPERTIES). No reference engine is needed: each
        //! property compares Forge with itself.
        //!
        //! 1. `format:` never changes a value (ADR-061).
        //! 2. `forge export` then `forge import` gives back the same numbers.
        //! 3. Calculating a model gives the same output every time, and calculating
        //!    the file it wrote changes nothing in it.
        //! 4. A simulation with a seed gives the same numbers every time
        //!    (docs/STABILITY.md, point 4).
        //!
        //! A generated model is a table of three to eight rows (two columns of
        //! numbers, one with empty cells, and a formula column) and scalars that read
        //! and aggregate it. Numbers run from 1e-6 to 1e12, negative and zero
        //! included. When a property fails, proptest shrinks the model and prints it;
        //! that model becomes an ordinary test (T1-TEST-RULE).
        //!
        //! The case counts keep the file to a few seconds in a debug build. CI fixes
        //! the draws (`PROPTEST_RNG_SEED=1` in ci.yml), so a pull request sees the
        //! same cases every time; the weekly run draws many more from a new seed it
        //! prints (`PROPTEST_CASES`, `PROPTEST_RNG_SEED`, weekly.yml). To replay a
        //! failure, run with the seed it printed:
        //! `PROPTEST_RNG_SEED=<seed> cargo test --lib -- tests::properties::`.

        use crate::cli::commands::results::CalculationResult;
        use crate::cli::SimulateOptions;
        use crate::cli::{calculate_core, export_core, import_core, simulate_core};
        use crate::core::precision;
        use proptest::prelude::*;
        use std::fmt::Write as _;
        use std::path::{Path, PathBuf};

        /// How many cases a property runs: `PROPTEST_CASES` when set, else `default`.
        fn cases(default: u32) -> u32 {
            cases_from(std::env::var("PROPTEST_CASES").ok(), default)
        }

        fn cases_from(set: Option<String>, default: u32) -> u32 {
            set.and_then(|v| v.parse().ok()).unwrap_or(default)
        }

        #[test]
        fn the_cases_of_a_property_are_what_the_environment_says_if_it_is_a_number() {
            assert_eq!(cases_from(Some("7".to_string()), 48), 7);
            assert_eq!(cases_from(Some("many".to_string()), 48), 48);
            assert_eq!(cases_from(None, 48), 48);
        }

        /// A number a model might hold: zero, small, ordinary or large, either sign.
        fn number() -> impl Strategy<Value = f64> {
            prop_oneof![
                Just(0.0),
                (-1_000_000i64..1_000_000).prop_map(|n| precision::whole(n) / 100.0),
                (-9i64..=9, -6i32..=12).prop_map(|(m, e)| precision::whole(m) * 10f64.powi(e)),
                (-100i64..100).prop_map(precision::whole),
            ]
        }

        /// The formula of the table's third column, read row by row.
        const ROW_FORMULAS: &[&str] = &[
            "=a + b",
            "=a * s.x - b",
            "=ROUND(a * b, 2)",
            "=IF(a > b, a, b)",
            "=ABS(a - b) + s.y",
            "=MAX(a, b) - MIN(a, s.x)",
        ];

        /// Number formats Forge writes as Excel does (ADR-061).
        const FORMATS: &[&str] = &[
            "#,##0",
            "#,##0.00",
            "0%",
            "0.0%",
            "$#,##0.00",
            "#,##0;(#,##0)",
        ];

        #[derive(Debug, Clone)]
        struct Model {
            x: f64,
            y: f64,
            a: Vec<f64>,
            b: Vec<Option<f64>>,
            row_formula: &'static str,
            /// A format for x, total, column a and column c, or none
            formats: [Option<&'static str>; 4],
        }

        fn model() -> impl Strategy<Value = Model> {
            (3usize..=8).prop_flat_map(|rows| {
                (
                    number(),
                    number(),
                    prop::collection::vec(number(), rows),
                    prop::collection::vec(prop::option::weighted(0.8, number()), rows),
                    prop::sample::select(ROW_FORMULAS),
                    prop::array::uniform4(prop::option::of(prop::sample::select(FORMATS))),
                )
                    .prop_map(|(x, y, a, b, row_formula, formats)| Model {
                        x,
                        y,
                        a,
                        b,
                        row_formula,
                        formats,
                    })
            })
        }

        fn list(values: impl Iterator<Item = String>) -> String {
            format!("[{}]", values.collect::<Vec<_>>().join(", "))
        }

        /// `value` alone, or a mapping with `format:` when there is one.
        fn with_format(key: &str, body: &str, format: Option<&str>) -> String {
            format.map_or_else(
                || format!("{body}\n"),
                |f| format!("{{{key}: {body}, format: \"{f}\"}}\n"),
            )
        }

        impl Model {
            /// The model as YAML, with its formats or without them.
            fn yaml(&self, formats: bool) -> String {
                let format_of = |i: usize| if formats { self.formats[i] } else { None };
                let total = format_of(1).map_or_else(
                    || "{formula: \"=SUM(t.c)\"}\n".to_string(),
                    |code| format!("{{formula: \"=SUM(t.c)\", format: \"{code}\"}}\n"),
                );
                let column_c = format_of(3).map_or_else(
                    || format!("\"{}\"\n", self.row_formula),
                    |code| {
                        format!(
                            "{{formula: \"{}\", format: \"{code}\"}}\n",
                            self.row_formula
                        )
                    },
                );
                let a = list(self.a.iter().map(ToString::to_string));
                let b = list(
                    self.b
                        .iter()
                        .map(|v| v.map_or_else(|| "null".to_string(), |n| n.to_string())),
                );
                let mut out = String::from("_forge_version: \"5.4.0\"\ns:\n");
                let _ = write!(
                    out,
                    "  x: {}",
                    with_format("value", &self.x.to_string(), format_of(0))
                );
                let _ = writeln!(out, "  y: {}", self.y);
                let _ = write!(out, "  total: {total}");
                out.push_str("  mean: {formula: \"=AVERAGE(t.a)\"}\n");
                out.push_str("  prod: {formula: \"=s.x * s.y + s.total\"}\n");
                out.push_str("  top: {formula: \"=MAX(t.c)\"}\n");
                out.push_str("t:\n");
                let _ = write!(out, "  a: {}", with_format("value", &a, format_of(2)));
                let _ = writeln!(out, "  b: {b}");
                let _ = write!(out, "  c: {column_c}");
                out
            }
        }

        fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            path
        }

        /// Every number a calculation gave, by name: scalars, then table cells.
        fn numbers(result: &CalculationResult) -> Vec<(String, Option<f64>)> {
            let mut out: Vec<(String, Option<f64>)> = result
                .scalars
                .iter()
                .map(|(name, value)| (name.clone(), *value))
                .collect();
            for (table, summary) in &result.tables {
                for (column, values) in &summary.values {
                    for (row, value) in values.iter().enumerate() {
                        out.push((format!("{table}.{column}[{row}]"), value.as_f64()));
                    }
                }
            }
            out
        }

        /// The same to the 15 significant digits a workbook keeps.
        fn close(a: Option<f64>, b: Option<f64>) -> bool {
            a.zip(b).map_or_else(
                || a.is_none() && b.is_none(),
                |(a, b)| (a - b).abs() <= 1e-14 * a.abs().max(b.abs()).max(1.0),
            )
        }

        proptest! {
            #![proptest_config(ProptestConfig { cases: cases(48), ..ProptestConfig::default() })]

            /// 1. A model with `format:` and the same model without them give the
            ///    same values, bit for bit.
            #[test]
            fn a_format_never_changes_a_value(m in model()) {
                let dir = tempfile::tempdir().unwrap();
                let plain = calculate_core(&write(dir.path(), "plain.yaml", &m.yaml(false)), true, None);
                let formatted = calculate_core(&write(dir.path(), "formatted.yaml", &m.yaml(true)), true, None);
                match (plain, formatted) {
                    (Ok(plain), Ok(formatted)) => {
                        let (p, f) = (numbers(&plain), numbers(&formatted));
                        prop_assert_eq!(p.len(), f.len());
                        for ((name, a), (_, b)) in p.iter().zip(&f) {
                            prop_assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits), "{} differs", name);
                        }
                    }
                    (Err(a), Err(b)) => prop_assert_eq!(a.to_string(), b.to_string()),
                    (a, b) => prop_assert!(false, "one calculated and one did not: {:?} / {:?}", a.err(), b.err()),
                }
            }

            /// 3. Calculating twice gives the same output, and calculating the file
            ///    Forge wrote leaves it as it was.
            #[test]
            fn calculating_again_changes_nothing(m in model()) {
                let dir = tempfile::tempdir().unwrap();
                let path = write(dir.path(), "m.yaml", &m.yaml(true));
                let Ok(dry) = calculate_core(&path, true, None) else { return Ok(()); };
                let again = calculate_core(&path, true, None).unwrap();
                prop_assert_eq!(serde_json::to_string(&dry).unwrap(), serde_json::to_string(&again).unwrap());

                calculate_core(&path, false, None).unwrap();
                let written = std::fs::read_to_string(&path).unwrap();
                let after = calculate_core(&path, false, None).unwrap();
                prop_assert_eq!(&written, &std::fs::read_to_string(&path).unwrap(), "a second calculate rewrote the file");
                for ((name, a), (_, b)) in numbers(&dry).iter().zip(&numbers(&after)) {
                    prop_assert_eq!(a.map(f64::to_bits), b.map(f64::to_bits), "{} changed once written", name);
                }
            }
        }

        proptest! {
            // Each case writes and reads a workbook
            #![proptest_config(ProptestConfig { cases: cases(24), ..ProptestConfig::default() })]

            /// 2. `forge export` then `forge import` gives back the same numbers.
            #[test]
            fn export_then_import_gives_the_same_numbers(m in model()) {
                let dir = tempfile::tempdir().unwrap();
                let path = write(dir.path(), "m.yaml", &m.yaml(true));
                let Ok(before) = calculate_core(&path, false, None) else { return Ok(()); };
                let xlsx = dir.path().join("m.xlsx");
                export_core(&path, &xlsx).unwrap();
                let back = dir.path().join("back.yaml");
                import_core(&xlsx, &back, false, false).unwrap();
                let after = calculate_core(&back, true, None).unwrap();
                let (b, a) = (numbers(&before), numbers(&after));
                let names = |v: &[(String, Option<f64>)]| v.iter().map(|(n, _)| n.clone()).collect::<Vec<_>>();
                prop_assert_eq!(names(&b), names(&a));
                for ((name, x), (_, y)) in b.iter().zip(&a) {
                    prop_assert!(close(*x, *y), "{} was {:?}, is {:?} after the round trip", name, x, y);
                }
            }
        }

        /// A Monte Carlo model with generated distributions and a seed.
        fn simulation() -> impl Strategy<Value = (String, u64)> {
            (
                (-1000i64..1000, 1i64..500),
                (0i64..100, 1i64..100, 1i64..100),
                any::<u64>(),
            )
                .prop_map(|((mean, sd), (lo, mode, hi), seed)| {
                    let (lo, mode, hi) = (lo, lo + mode, lo + mode + hi);
                    let yaml = format!(
                        "_forge_version: \"5.1.0\"\n\
                 monte_carlo:\n  iterations: 200\n  seed: {seed}\n  outputs:\n    - variable: r.out\n\
                 r:\n  a: {{formula: \"=MC.Normal({mean}, {sd})\"}}\n\
                 \x20 b: {{formula: \"=MC.Triangular({lo}, {mode}, {hi})\"}}\n\
                 \x20 out: {{formula: \"=r.a * 2 - r.b\"}}\n"
                    );
                    (yaml, seed)
                })
        }

        proptest! {
            #![proptest_config(ProptestConfig { cases: cases(24), ..ProptestConfig::default() })]

            /// 4. A seeded simulation gives the same numbers every time.
            #[test]
            fn a_seeded_simulation_repeats(sim in simulation()) {
                let (yaml, _) = sim;
                let dir = tempfile::tempdir().unwrap();
                let path = write(dir.path(), "mc.yaml", &yaml);
                let options = || SimulateOptions { iterations: None, seed: None, sampling: None, scenario: None, outputs: &[] };
                let first = simulate_core(&path, options()).unwrap();
                let second = simulate_core(&path, options()).unwrap();
                let bits = |r: &crate::monte_carlo::SimulationResult| {
                    r.outputs["r.out"].samples.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
                };
                prop_assert_eq!(first.iterations_completed, 200);
                prop_assert_eq!(bits(&first), bits(&second));
            }
        }
    }
}
