//! `forge simulate --output-var` shows only the outputs named (roadmap
//! T1-SIM-SHOW-ONE). The bagel simulation prints every output in
//! `monte_carlo.outputs`, about 80 lines, and an agent asking about the year's
//! profit had to search them. The draws do not depend on the outputs, so an
//! output's numbers must be the same with the flag as without it.

use mollendorff_forge::cli::{simulate_core, SimulateOptions};
use std::path::Path;

const BAGEL: &str = "examples/bagel/bagel_simulate.yaml";

fn run(outputs: &[String]) -> mollendorff_forge::monte_carlo::SimulationResult {
    simulate_core(
        Path::new(BAGEL),
        SimulateOptions {
            iterations: Some(500),
            outputs,
            ..SimulateOptions::default()
        },
    )
    .expect("simulates")
}

// The same draws give the same numbers, to the last bit
#[allow(clippy::float_cmp)]
#[test]
fn one_output_has_the_numbers_it_has_among_all() {
    let all = run(&[]);
    let one = run(&["year.profit".to_string()]);
    assert_eq!(one.outputs.keys().collect::<Vec<_>>(), ["year.profit"]);
    let (a, b) = (&all.outputs["year.profit"], &one.outputs["year.profit"]);
    assert_eq!(a.samples, b.samples);
    assert_eq!(a.statistics.mean, b.statistics.mean);
    assert_eq!(a.statistics.percentiles, b.statistics.percentiles);
    assert_eq!(a.threshold_probabilities, b.threshold_probabilities);
    assert_eq!(a.expected_shortfall, b.expected_shortfall);
    let drivers = |r: &mollendorff_forge::monte_carlo::OutputResult| {
        r.drivers
            .iter()
            .map(|d| (d.input.clone(), d.rank_correlation))
            .collect::<Vec<_>>()
    };
    assert_eq!(drivers(a), drivers(b));
}

#[test]
fn a_name_without_its_group_and_a_column_choose_their_outputs() {
    let worst = run(&["worst_month_profit".to_string()]);
    assert_eq!(
        worst.outputs.keys().collect::<Vec<_>>(),
        ["year.worst_month_profit"]
    );
    // A table column is all its rows, in the order configured
    let months = run(&["months.profit".to_string(), "year.profit".to_string()]);
    let names: Vec<&String> = months.outputs.keys().collect();
    assert_eq!(names.len(), 13, "{names:?}");
    assert_eq!(names[0], "year.profit");
    assert_eq!(names[1], "months.profit (row 1)");
    assert_eq!(names[12], "months.profit (row 12)");
}

#[test]
fn an_unknown_or_ambiguous_name_is_refused_with_the_outputs() {
    let err = |names: &[&str]| {
        let names: Vec<String> = names.iter().map(ToString::to_string).collect();
        simulate_core(
            Path::new(BAGEL),
            SimulateOptions {
                iterations: Some(10),
                outputs: &names,
                ..SimulateOptions::default()
            },
        )
        .expect_err("refused")
        .to_string()
    };
    assert_eq!(
        err(&["revenue"]),
        "Validation error: --output-var revenue: not an output of monte_carlo (outputs: \
         year.profit, year.worst_month_profit, months.profit)"
    );
    assert_eq!(
        err(&["profit"]),
        "Validation error: --output-var profit: more than one output ends with it \
         (year.profit, months.profit); write the whole name"
    );
}

#[cfg(not(coverage))]
mod command_line {
    use std::process::Command;

    fn forge(args: &[&str]) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(args)
            .output()
            .expect("forge runs")
    }

    #[test]
    fn the_flag_prints_only_that_output() {
        let out = forge(&[
            "simulate",
            super::BAGEL,
            "-n",
            "200",
            "--output-var",
            "year.profit",
        ]);
        assert!(out.status.success(), "{out:?}");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("year.profit:"), "{stdout}");
        assert!(stdout.contains("P(year.profit > 0) = "), "{stdout}");
        assert!(!stdout.contains("year.worst_month_profit:"), "{stdout}");
        assert!(!stdout.contains("months.profit by row"), "{stdout}");
    }

    #[test]
    fn an_unknown_name_exits_1_and_lists_the_outputs() {
        let out = forge(&["simulate", super::BAGEL, "--output-var", "nope"]);
        assert_eq!(out.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains(
                "--output-var nope: not an output of monte_carlo (outputs: year.profit, \
                 year.worst_month_profit, months.profit)"
            ),
            "{stderr}"
        );
    }
}
