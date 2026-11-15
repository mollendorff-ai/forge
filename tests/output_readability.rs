//! How the terminal shows the numbers (T2-VARIANCE-NM, T2-CLI-FORMAT): a
//! variance percentage that means nothing is n/m, and a scalar or column with
//! a `format:` (ADR-061) is printed as the format shows it, as `forge report`
//! and Studio do. Only the text changes: the values are the same.
//!
//! A coverage build stubs `main` (ADR-006), so the command-line tests run the
//! real binary only in an ordinary build.

use mollendorff_forge::cli::{pct_is_meaningful, variance_core};
use std::path::Path;

const PLAN: &str = "examples/bagel/bagel.yaml";
const ACTUAL: &str = "examples/bagel/actual.yaml";

#[test]
fn a_percentage_is_not_meaningful_on_a_zero_budget_or_across_zero() {
    // Zero (or as good as zero) budget: any change is an infinite share of it
    assert!(!pct_is_meaningful(0.0, 5.0));
    assert!(!pct_is_meaningful(0.000_01, 1.0));
    // The sign flips: a profit that became a loss, and back
    assert!(!pct_is_meaningful(786.32, -28_313.87));
    assert!(!pct_is_meaningful(-100.0, 50.0));
    // Same side of zero, or down to exactly zero (-100%): meaningful
    assert!(pct_is_meaningful(-95_579.52, -123_258.79));
    assert!(pct_is_meaningful(100.0, 0.0));
    assert!(pct_is_meaningful(4_800.0, 4_950.0));
}

/// The plan's profit is 786.32 and the actual -28,313.87: -3,700.8% says
/// nothing, 29,100 worse says it all. The rows whose sign flipped are n/m and
/// stay alerts; every other row keeps its percentage.
#[test]
fn bagel_variance_marks_the_rows_that_changed_sign() {
    let v = variance_core(Path::new(PLAN), Path::new(ACTUAL), 10.0).unwrap();
    let not_meaningful: Vec<&str> = v
        .results
        .iter()
        .filter(|r| !r.pct_meaningful)
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(
        not_meaningful,
        ["year.profit", "year.margin", "year.first_year_return"]
    );
    for name in &not_meaningful {
        let row = v.results.iter().find(|r| r.name == *name).unwrap();
        assert!(row.exceeds_threshold, "{name} is still an alert");
    }
    // Same five alerts as before the rule
    assert_eq!(v.alert_count, 5);
}

#[cfg(not(coverage))]
mod command_line {
    use std::process::Command;

    fn forge(args: &[&str]) -> Vec<String> {
        let out = Command::new(env!("CARGO_BIN_EXE_forge"))
            .args(args)
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env("NO_COLOR", "1")
            .env("COLUMNS", "200")
            .output()
            .expect("forge runs");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        // Words separated by single spaces, so column widths do not matter
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    }

    fn has(lines: &[String], wanted: &str) -> bool {
        lines.iter().any(|l| l == wanted)
    }

    #[test]
    fn variance_prints_n_m() {
        let out = forge(&["variance", super::PLAN, super::ACTUAL]);
        assert!(
            has(&out, "year.profit 786.32 -28,313.87 -29,100.19 n/m ⚠️ ❌"),
            "{out:#?}"
        );
        assert!(
            has(
                &out,
                "year.npv_first_year -95,579.52 -123,258.79 -27,679.27 -29.0% ⚠️ ❌"
            ),
            "{out:#?}"
        );
    }

    /// The text `forge report` shows for `name`, on a calculated copy of the
    /// plan (report reads the stored results).
    fn report_text(name: &str) -> String {
        let dir = tempfile::tempdir().unwrap();
        for file in ["bagel.yaml", "costs.yaml"] {
            std::fs::copy(
                std::path::Path::new("examples/bagel").join(file),
                dir.path().join(file),
            )
            .unwrap();
        }
        let model = dir.path().join("bagel.yaml");
        let model = model.to_str().unwrap();
        forge(&["calculate", model]);
        let out = forge(&["report", model, "--format", "markdown"]);
        let (group, key) = name.split_once('.').unwrap();
        let cells = |l: &str| -> Vec<String> {
            l.trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect()
        };
        let at = out
            .iter()
            .position(|l| cells(l).iter().any(|c| c == key) && l.starts_with("| group"))
            .expect("a header with the key");
        let column = cells(&out[at]).iter().position(|c| c == key).unwrap();
        let row = out[at + 2..]
            .iter()
            .find(|l| cells(l).first().map(String::as_str) == Some(group))
            .expect("the group's row");
        cells(row)[column].clone()
    }

    #[test]
    fn calculate_prints_the_format_and_the_full_value() {
        let out = forge(&["calculate", super::PLAN, "--dry-run"]);
        assert!(
            has(&out, "year.margin = 0.2% (0.00203274189215848)"),
            "{out:#?}"
        );
        assert!(
            has(&out, "assumptions.price_per_dozen = $16.00 (16)"),
            "{out:#?}"
        );
        // A scalar with no format prints as before
        assert!(has(&out, "year.profit = 786.320361712362"), "{out:#?}");
        // A column with a format, then its full values
        assert!(
            has(
                &out,
                "season = [85%; 125%; 80%; 80%; 90%; 95%; 105%; 115%; 125%; 125%; 105%; 100%]"
            ),
            "{out:#?}"
        );
        assert!(
            has(
                &out,
                "[0.85, 1.25, 0.8, 0.8, 0.9, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1]"
            ),
            "{out:#?}"
        );
    }

    #[test]
    fn scenarios_and_compare_print_what_report_prints() {
        let margin = report_text("year.margin");
        let price = report_text("assumptions.price_per_dozen");
        assert_eq!((margin.as_str(), price.as_str()), ("0.2%", "$16.00"));

        let scenarios = forge(&["scenarios", super::PLAN]);
        assert!(
            has(&scenarios, &format!("year.margin: {margin}")),
            "{scenarios:#?}"
        );

        let compare = forge(&["compare", super::PLAN, "--scenarios", "base,flour_shock"]);
        assert!(
            has(&compare, &format!("year.margin {margin} -4.4%")),
            "{compare:#?}"
        );
        assert!(
            has(
                &compare,
                &format!("assumptions.price_per_dozen {price} {price}")
            ),
            "{compare:#?}"
        );
    }

    /// CSV and JSON are for scripts: full values, no format
    #[test]
    fn compare_csv_keeps_the_full_numbers() {
        let out = forge(&[
            "compare",
            super::PLAN,
            "--scenarios",
            "base,flour_shock",
            "--format",
            "csv",
        ]);
        assert!(
            out.iter()
                .any(|l| l.starts_with("year.margin,0.00203274189215848,")),
            "{out:#?}"
        );
    }
}
