//! A row formula that reads a whole column copied it for every row, so the
//! time grew with the square of the rows for the copies alone: 16,000 rows of
//! `=INDEX(d.v, MATCH(k, d.k, 0))` took 5.2 s, and `SUMIF` 6.3 s (release).
//! A column is now shared, not copied. Roadmap T1-COLUMN-COPIES.
//!
//! Each row still searches the column, as in Excel, so the time still grows
//! with the square of the rows; the copies were most of it.

use mollendorff_forge::cli::calculate_core;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

const ROWS: usize = 16_000;

/// A table `d` of keys and values, and a table `q` whose row formula reads
/// both columns of `d` whole in every row.
fn model(formula: &str) -> String {
    let list = |f: &dyn Fn(usize) -> usize| {
        (0..ROWS)
            .map(|i| f(i).to_string())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut text = String::from("_forge_version: \"5.1.0\"\nd:\n");
    writeln!(text, "  k: [{}]", list(&|i| i)).unwrap();
    writeln!(text, "  v: [{}]", list(&|i| i * 2)).unwrap();
    // Keys in a scattered order, every one of d.k once
    writeln!(text, "q:\n  k: [{}]", list(&|i| (i * 7919) % ROWS)).unwrap();
    writeln!(text, "  r: \"{formula}\"").unwrap();
    text
}

/// Under 1 s in a release build. A debug build is too slow to tell the copies
/// from the search (2 s here against 13 s before, and CI runners vary), so it
/// checks only the results; the Validation workflow runs this in release.
fn in_time(took: Duration, limit: Duration, what: &str) {
    if !cfg!(debug_assertions) {
        assert!(took < limit, "16,000 rows of {what} took {took:?}");
    }
}

/// The time, and the results of `q.r`.
fn run(formula: &str) -> (Duration, Vec<f64>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lookup.yaml");
    std::fs::write(&path, model(formula)).unwrap();
    let start = Instant::now();
    let result = calculate_core(&path, true, None).unwrap();
    let took = start.elapsed();
    let values = result.tables["q"].values["r"]
        .iter()
        .map(|v| serde_json::to_value(v).unwrap().as_f64().unwrap())
        .collect();
    (took, values)
}

/// Row i of q looks up key (i * 7919) % ROWS, whose value is twice the key.
#[allow(clippy::cast_precision_loss)] // whole numbers under 2^53
fn expected() -> Vec<f64> {
    (0..ROWS).map(|i| ((i * 7919) % ROWS * 2) as f64).collect()
}

#[test]
fn sixteen_thousand_rows_of_index_match_calculate_in_time() {
    let (took, values) = run("=INDEX(d.v, MATCH(k, d.k, 0))");
    in_time(took, Duration::from_secs(1), "INDEX/MATCH");
    assert_eq!(values, expected());
}

#[test]
fn sixteen_thousand_rows_of_sumif_calculate_in_time() {
    // SUMIF reads its ranges another way (collect_values_as_vec)
    let (took, values) = run("=SUMIF(d.k, k, d.v)");
    in_time(took, Duration::from_secs(2), "SUMIF");
    assert_eq!(values, expected());
}
