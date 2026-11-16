//! `forge simulate` shows each uncertain input's plan value beside its mean,
//! and each output's value at plan beside its simulated mean (roadmap
//! T1-SIM-MEAN-VS-PLAN). `forge calculate` gives the bagel year 786.32 and
//! `forge simulate` a mean near -15,926 for the same plan; both are right,
//! because the uncertain inputs' means are above their plan values, and the
//! output did not say so.
//!
//! The figures are checked against closed forms, not against Forge:
//! - a triangular's mean is (min + mode + max) / 3, a uniform's (min + max) / 2
//!   and a PERT's (min + 4 mode + max) / 6;
//! - profit is linear in staff hours, the wage and the flour price, and they
//!   are drawn independently, so the expected profit is the plan's profit plus
//!   each input's (mean - plan) times its coefficient:
//!   labour: (E[hours] E[wage] - 32 x 18.50) x 30 days x 1.15 load x 12 months;
//!   flour: dozens of the plan's year x 12 / 9 bagels a kg x (E[flour] - 1.15).
//!   Opening demand is a symmetric PERT (mean 55 = plan), so it adds nothing.

use mollendorff_forge::cli::{calculate_core, simulate_core, SimulateOptions};
use std::path::Path;

const SIMULATE: &str = "examples/bagel/bagel_simulate.yaml";

fn bagel() -> mollendorff_forge::monte_carlo::SimulationResult {
    simulate_core(
        Path::new(SIMULATE),
        SimulateOptions {
            outputs: &["year.profit".to_string()],
            ..SimulateOptions::default()
        },
    )
    .expect("simulates")
}

#[test]
fn the_value_at_plan_is_the_plans_profit() {
    let result = bagel();
    let at_plan = result.outputs["year.profit"].at_plan.expect("calculated");
    let plan = calculate_core(Path::new("examples/bagel/bagel.yaml"), true, None).unwrap();
    let planned = plan.scalars["year.profit"].unwrap();
    assert!(
        (at_plan - planned).abs() < 1e-9,
        "{at_plan} against {planned}"
    );
    assert!((at_plan - 786.320_361_7).abs() < 1e-6, "{at_plan}");
}

#[test]
fn the_inputs_plan_values_and_means_are_the_closed_forms() {
    let result = bagel();
    let mean = |name: &str| {
        let s = &result.input_samples[name];
        #[allow(clippy::cast_precision_loss)] // 5,000 samples
        let n = s.len() as f64;
        s.iter().sum::<f64>() / n
    };
    let cases = [
        // (input, plan, closed-form mean)
        (
            "assumptions.staff_hours_per_day",
            32.0,
            (28.0 + 32.0 + 38.0) / 3.0,
        ),
        (
            "@costs.ingredients.flour_per_kg",
            1.15,
            (1.05 + 1.15 + 1.70) / 3.0,
        ),
        (
            "@costs.labour.wage_per_hour",
            18.50,
            f64::midpoint(17.50, 20.50),
        ),
        (
            "assumptions.opening_dozens_per_day",
            55.0,
            4.0f64.mul_add(55.0, 40.0 + 70.0) / 6.0,
        ),
    ];
    for (name, plan, closed_form) in cases {
        let p = &result.plan_inputs[name];
        assert!((p.value - plan).abs() < 1e-12, "{name}: plan {}", p.value);
        assert_eq!(p.from, "value", "{name}");
        // Latin hypercube with 5,000 draws: the mean is within 0.1%
        let m = mean(name);
        assert!(
            (m - closed_form).abs() <= closed_form * 1e-3,
            "{name}: sampled mean {m}, closed form {closed_form}"
        );
    }
}

#[test]
fn the_gap_between_mean_and_plan_is_the_inputs_means() {
    let result = bagel();
    let year = &result.outputs["year.profit"];
    let at_plan = year.at_plan.expect("calculated");
    let hours: f64 = (28.0 + 32.0 + 38.0) / 3.0;
    let wage = f64::midpoint(17.50, 20.50);
    let labour = hours.mul_add(wage, -32.0 * 18.50) * 30.0 * 1.15 * 12.0;
    // The plan's year of dozens, from the independent Python in bagel_example.rs
    let dozens = 24_176.715_596_114_2;
    let flour = dozens * 12.0 / 9.0 * ((1.05 + 1.15 + 1.70) / 3.0 - 1.15);
    let expected_gap = -(labour + flour); // -16,703.34
    assert!((expected_gap + 16_703.34).abs() < 0.01, "{expected_gap}");
    let gap = year.statistics.mean - at_plan;
    // Sampling error of the mean: under 0.1% of a standard deviation of 38,058
    assert!(
        (gap - expected_gap).abs() < 40.0,
        "simulated gap {gap}, closed form {expected_gap}"
    );
}

#[cfg(not(coverage))]
mod command_line {
    use std::process::Command;

    #[test]
    fn simulate_prints_the_plan_beside_the_mean() {
        let out = Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(["simulate", super::SIMULATE, "--output-var", "year.profit"])
            .output()
            .expect("forge runs");
        assert!(out.status.success(), "{out:?}");
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in [
            "assumptions.staff_hours_per_day mean=32.67 plan=32.00 (value)",
            "@costs.ingredients.flour_per_kg mean=1.30 plan=1.15 (value)",
            "@costs.labour.wage_per_hour mean=19.00 plan=18.50 (value)",
            "At plan:   786.32  (each uncertain input at its plan value)",
            "the uncertain inputs' means are not their plan values",
        ] {
            assert!(stdout.contains(line), "missing {line:?} in:\n{stdout}");
        }
    }
}
