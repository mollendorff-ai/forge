//! The headline numbers of examples/mortgage and examples/household, as
//! computed outside Forge.
//!
//! The mortgage, as a month-by-month amortisation loop (no closed forms):
//! ```python
//! r = (1 + 0.0479 / 2) ** (1 / 6) - 1        # semi-annual compounding
//! P, n = 450000, 300
//! pay = P * r / (1 - (1 + r) ** -n)            # 2563.6876295653
//! def run(pmt):
//!     bal, months, interest, y1i, y1p = P, 0, 0.0, 0.0, 0.0
//!     while bal > 1e-9:
//!         i = bal * r; p = min(pmt, bal + i)
//!         bal = bal + i - p; months += 1; interest += i
//!         if months <= 12: y1i += i; y1p += p - i
//!     return months, p, interest, y1i, y1p
//! run(pay)        # interest 319106.2888696
//! # the extra payment that saves 80,000 of interest, as in the README
//! brentq(lambda e: run(pay)[2] - run(pay + e)[2] - 80000, 1, 2000)  # 397.044208
//! run(pay + 300)  # 247 payments, the last 42.248775, interest 254509.4056479,
//!                 # first year interest 21056.173122, principal 13308.078432
//! ```
//!
//! The household: income 5200 + 400 = 5600 budgeted, 5200 + 150 = 5350
//! actual; expenses 3100 budgeted, 1650 + 812.43 + 96.50 + 163.20 + 120 +
//! 388.75 + 74.97 = 3305.85 actual; left over 2500 and 2044.15.

use mollendorff_forge::cli::{calculate_core, goal_seek_core, variance_core};
use std::path::Path;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

#[test]
fn the_mortgage_matches_an_amortisation_loop() {
    let result = calculate_core(Path::new("examples/mortgage/mortgage.yaml"), true, None).unwrap();
    let get = |name: &str| result.scalars[name].unwrap();
    for (name, expected) in [
        ("terms.payment", 2_563.687_629_565_3),
        ("as_agreed.total_interest", 319_106.288_869_6),
        ("with_extra.last_payment", 42.248_774_9),
        ("with_extra.total_interest", 254_509.405_647_9),
        ("saved.interest", 64_596.883_221_6),
        ("saved.months", 53.0),
        ("year_one.interest", 21_056.173_122_35),
        ("year_one.principal_paid", 13_308.078_432_43),
    ] {
        assert!(close(get(name), expected, 1e-5), "{name}: {}", get(name));
    }
    // 246 full payments and a smaller last one: 247, as in the loop
    assert!(close(get("with_extra.full_months"), 246.0, 0.0));
}

#[test]
fn the_extra_payment_that_saves_eighty_thousand() {
    let result = goal_seek_core(
        Path::new("examples/mortgage/mortgage.yaml"),
        "saved.interest",
        80_000.0,
        "loan.extra_per_month",
        (None, None),
        0.0001,
    )
    .unwrap();
    assert!(result.converged);
    assert!(
        close(result.solution, 397.044_208, 1e-4),
        "{}",
        result.solution
    );
}

#[test]
fn the_household_variance_has_the_right_directions_and_totals() {
    let report = variance_core(
        Path::new("examples/household/budget.yaml"),
        Path::new("examples/household/actual.yaml"),
        10.0,
    )
    .unwrap();
    let get = |name: &str| {
        report
            .results
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("{name}"))
    };
    for (name, budget, actual, better, favourable) in [
        ("income.salary", 5200.0, 5200.0, "higher", Some(true)),
        ("income.side_work", 400.0, 150.0, "higher", Some(false)),
        ("income.total", 5600.0, 5350.0, "higher", Some(false)),
        ("expenses.transport", 180.0, 96.5, "lower", Some(true)),
        ("expenses.eating_out", 250.0, 388.75, "lower", Some(false)),
        ("expenses.total", 3100.0, 3305.85, "lower", Some(false)),
        ("result.left_over", 2500.0, 2044.15, "higher", Some(false)),
    ] {
        let r = get(name);
        assert!(close(r.budget, budget, 1e-9), "{name} budget {}", r.budget);
        assert!(close(r.actual, actual, 1e-9), "{name} actual {}", r.actual);
        assert_eq!(r.better, better, "{name}");
        assert_eq!(r.is_favorable, favourable, "{name}");
    }
    // Over the 10% threshold: side work, groceries, transport, utilities,
    // eating out, subscriptions, left over and the savings rate
    let alerts: Vec<&str> = report
        .results
        .iter()
        .filter(|r| r.exceeds_threshold)
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(
        alerts,
        [
            "income.side_work",
            "expenses.groceries",
            "expenses.transport",
            "expenses.utilities",
            "expenses.eating_out",
            "expenses.subscriptions",
            "result.left_over",
            "result.savings_rate",
        ]
    );
}

#[test]
fn each_demo_calculates_in_under_a_second() {
    for model in [
        "examples/mortgage/mortgage.yaml",
        "examples/household/budget.yaml",
        "examples/household/actual.yaml",
        "examples/bagel/bagel.yaml",
    ] {
        let start = std::time::Instant::now();
        calculate_core(Path::new(model), true, None).unwrap();
        let took = start.elapsed();
        // A debug build is slower than a release build; a second is still ample
        assert!(took.as_secs_f64() < 1.0, "{model}: {took:?}");
    }
}

/// Every model under examples/ passes the schema of its format (the parser
/// checks it before anything else) and parses.
#[test]
fn every_example_passes_its_schema() {
    fn models(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                models(&path, out);
            } else if path.extension().is_some_and(|e| e == "yaml") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    models(Path::new("examples"), &mut files);
    assert!(files.len() >= 20, "{files:?}");
    for file in files {
        if let Err(e) = mollendorff_forge::parser::parse_model(&file) {
            panic!("{}: {e}", file.display());
        }
    }
}
