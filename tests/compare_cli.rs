//! `forge compare` on the command line: the formats and the width of the
//! table (Session Prompt, compare column overflow).
//!
//! A coverage build stubs `main` (ADR-006), so these run the real binary only
//! in an ordinary build.

#![cfg(not(coverage))]

use std::process::Command;

const MODEL: &str = r#"_forge_version: "5.1.0"
living:
  rent: 1200
  total: {formula: "=living.rent + 450.5"}
bankruptcy:
  income: 3100
  monthly_surplus_payment: {formula: "=MAX(0, (bankruptcy.income - living.total) * 0.5)"}
scenarios:
  rent_1100_prosupport_recognized:
    probability: 0.5
    scalars: {living.rent: 1100}
  rent_1500_prosupport_not_recognized:
    probability: 0.5
    scalars: {living.rent: 1500, bankruptcy.income: 2800}
"#;

const BOTH: &str = "rent_1100_prosupport_recognized,rent_1500_prosupport_not_recognized";

fn run(dir: &std::path::Path, columns: &str, args: &[&str]) -> String {
    let file = dir.join("m.yaml");
    std::fs::write(&file, MODEL).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("compare")
        .arg(&file)
        .args(["--scenarios", BOTH])
        .args(args)
        .env("COLUMNS", columns)
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
fn csv_is_the_numbers_alone_at_full_precision() {
    let dir = tempfile::tempdir().unwrap();
    // (3100 - 1550.5) * 0.5 = 774.75 and (2800 - 1950.5) * 0.5 = 424.75
    assert_eq!(
        run(dir.path(), "40", &["--format", "csv"]),
        "variable,rent_1100_prosupport_recognized,rent_1500_prosupport_not_recognized\n\
         living.rent,1100,1500\n\
         living.total,1550.5,1950.5\n\
         bankruptcy.income,3100,2800\n\
         bankruptcy.monthly_surplus_payment,774.75,424.75\n"
    );
}

#[test]
fn the_table_fits_the_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let wide = run(dir.path(), "200", &[]);
    assert!(wide.contains("rent_1100_prosupport_recognized  rent_1500_prosupport_not_recognized"));
    // 34 + 2 + 31 + 2 + 35 = 104 characters: two blocks in 80
    let narrow = run(dir.path(), "80", &[]);
    assert_eq!(narrow.matches("\nvariable ").count(), 2, "{narrow}");
    let table: Vec<&str> = narrow
        .lines()
        .filter(|l| l.starts_with("variable") || l.starts_with("scenario") || l.contains('.'))
        .filter(|l| !l.contains("File:") && !l.contains("Scenarios:"))
        .collect();
    assert!(!table.is_empty(), "{narrow}");
    for line in table {
        assert!(line.chars().count() <= 80, "{line}\n{narrow}");
    }
}
