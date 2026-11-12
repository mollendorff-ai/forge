//! The last commands that printed raw numbers print them as `forge report`
//! does, and the smaller things the end-to-end pass of 2026-11-12 found:
//! break-even said it was goal-seek, a horizon of one said `1 years`, an
//! import did not say it had written an included file, and a program could
//! only read `forge sensitivity` by parsing its table.
//!
//! A coverage build stubs `main` (ADR-006), so these run the real binary only
//! in an ordinary build.

#![cfg(not(coverage))]

use std::process::Command;

fn run(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn simulate_prints_statistics_with_separators_and_two_decimals() {
    let text = run(&["simulate", "examples/bagel/bagel_simulate.yaml"]);
    for line in [
        "Mean:      -15,925.94",
        "Median:    -15,689.18",
        "P5: -78,477.56",
        "(expected shortfall): -91,760.43",
        "assumptions.opening_dozens_per_day mean=55.00 min=40.79 max=69.73",
    ] {
        assert!(text.contains(line), "{line}\n{text}");
    }
}

#[test]
fn variance_prints_budget_actual_and_variance_as_report_does() {
    let text = run(&[
        "variance",
        "examples/bagel/bagel.yaml",
        "examples/bagel/actual.yaml",
    ]);
    assert!(text.contains("assumptions.rent_per_month"), "{text}");
    for cell in ["4,800.00", "4,950.00", "150.00", "386,827.45", "-16,923.70"] {
        assert!(text.contains(cell), "{cell}\n{text}");
    }
    assert!(!text.contains("386827.449538"), "{text}");
}

#[test]
fn the_sensitivity_table_lines_up_under_its_header() {
    let text = run(&[
        "sensitivity",
        "examples/periods/dcf.yaml",
        "-v",
        "assumptions.wacc",
        "-r",
        "0.08,0.11,0.01",
        "-o",
        "valuation.equity_value",
    ]);
    let lines: Vec<&str> = text.lines().collect();
    let header = lines
        .iter()
        .position(|l| {
            l.contains("assumptions.wacc")
                && l.contains("valuation.equity_value")
                && !l.contains('→')
        })
        .unwrap();
    let header_line = lines[header];
    // Every row ends where the header ends, and its input where the header's input ends
    let first_end = header_line.find("assumptions.wacc").unwrap() + "assumptions.wacc".len();
    for row in &lines[header + 2..header + 6] {
        assert_eq!(row.chars().count(), header_line.chars().count(), "{row}");
        assert_eq!(&row[first_end..=first_end], " ", "{row}");
    }
    assert!(text.contains("7,693,647.27"), "{text}");
}

#[test]
fn sensitivity_can_print_json_with_the_full_numbers() {
    let text = run(&[
        "sensitivity",
        "examples/periods/dcf.yaml",
        "-v",
        "assumptions.wacc",
        "-r",
        "0.08,0.09,0.01",
        "-o",
        "valuation.equity_value",
        "--format",
        "json",
    ]);
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["vary"], "assumptions.wacc");
    let entries = json["data"]["OneVar"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0]["input"], 0.08);
    assert_eq!(entries[0]["output"], 7_693_647.272_727_27);
}

#[test]
fn break_even_has_a_header_of_its_own() {
    let text = run(&[
        "break-even",
        "examples/bagel/bagel.yaml",
        "--output",
        "year.profit",
        "--vary",
        "assumptions.price_per_dozen",
    ]);
    assert!(text.contains("Break-Even Analysis"), "{text}");
    assert!(text.contains("Finding where year.profit = 0"), "{text}");
    assert!(!text.contains("Goal Seek"), "{text}");
    assert!(
        text.contains("assumptions.price_per_dozen = 15.97"),
        "{text}"
    );
}

#[test]
fn a_horizon_of_one_is_a_year() {
    let text = run(&["real-options", "examples/bagel/wait_option.yaml"]);
    assert!(text.contains("Time horizon: 1 year\n"), "{text}");
    let text = run(&["real-options", "examples/real-options.yaml"]);
    assert!(text.contains("Time horizon: 3 years\n"), "{text}");
}

#[test]
fn an_import_says_which_included_file_it_wrote() {
    let dir = tempfile::tempdir().unwrap();
    let xlsx = dir.path().join("bagel.xlsx");
    let model = std::path::Path::new("examples/bagel/bagel.yaml");
    // The model reads examples/bagel/costs.yaml, so export from there
    run(&["export", model.to_str().unwrap(), xlsx.to_str().unwrap()]);
    let back = dir.path().join("back");
    std::fs::create_dir(&back).unwrap();
    let text = run(&[
        "import",
        xlsx.to_str().unwrap(),
        back.join("model.yaml").to_str().unwrap(),
    ]);
    assert!(text.contains("YAML file:"), "{text}");
    assert!(text.contains("Included files: costs.yaml"), "{text}");
    assert!(back.join("costs.yaml").exists());
    // Importing again finds the same values there and writes nothing more
    let again = run(&[
        "import",
        xlsx.to_str().unwrap(),
        back.join("model.yaml").to_str().unwrap(),
    ]);
    assert!(!again.contains("Included files"), "{again}");
}
