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
//! `PROPTEST_RNG_SEED=<seed> cargo test --test properties`.

use mollendorff_forge::cli::commands::results::CalculationResult;
use mollendorff_forge::cli::SimulateOptions;
use mollendorff_forge::cli::{calculate_core, export_core, import_core, simulate_core};
use proptest::prelude::*;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// How many cases a property runs: `PROPTEST_CASES` when set, else `default`.
fn cases(default: u32) -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// A number a model might hold: zero, small, ordinary or large, either sign.
fn number() -> impl Strategy<Value = f64> {
    prop_oneof![
        Just(0.0),
        (-1_000_000i64..1_000_000).prop_map(|n| n as f64 / 100.0),
        (-9i64..=9, -6i32..=12).prop_map(|(m, e)| m as f64 * 10f64.powi(e)),
        (-100i64..100).prop_map(|n| n as f64),
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
    match (a, b) {
        (Some(a), Some(b)) => (a - b).abs() <= 1e-14 * a.abs().max(b.abs()).max(1.0),
        (None, None) => true,
        _ => false,
    }
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
        let bits = |r: &mollendorff_forge::monte_carlo::SimulationResult| {
            r.outputs["r.out"].samples.iter().map(|v| v.to_bits()).collect::<Vec<_>>()
        };
        prop_assert_eq!(first.iterations_completed, 200);
        prop_assert_eq!(bits(&first), bits(&second));
    }
}
