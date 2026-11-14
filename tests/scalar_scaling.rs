//! `forge calculate` took time growing with the cube of the number of
//! scalars: each formula laid every short name of the model over the
//! context, and checked each against every name laid before it. 1,000
//! scalars took 0.3 s, 4,000 took 14 s, and 26,600 (the generated cases of
//! ADR-062) did not finish in 10 minutes. Roadmap T1-SCALAR-SCALING.
//! The short names of each section were also worked out by going through
//! every scalar, so a model of many small sections grew with the square.

use mollendorff_forge::cli::calculate_core;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

fn model(n: usize, chained: bool) -> String {
    let mut text = String::from("_forge_version: \"5.4.0\"\nt:\n  c0: 1\n");
    for i in 1..n {
        let formula = if chained {
            format!("=c{} + 1", i - 1)
        } else {
            format!("=ROUND({i} * 1.5, 2)")
        };
        writeln!(text, "  c{i}: {{formula: \"{formula}\"}}").unwrap();
    }
    text
}

/// Under 2 s in a release build; a debug build is slower, so it gets ten
/// times as long, which the old cubic time still exceeded many times over.
fn limit() -> Duration {
    if cfg!(debug_assertions) {
        Duration::from_secs(20)
    } else {
        Duration::from_secs(2)
    }
}

fn time(n: usize, chained: bool) -> (Duration, Option<f64>) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("many.yaml");
    std::fs::write(&path, model(n, chained)).unwrap();
    let start = Instant::now();
    let result = calculate_core(&path, true, None).unwrap();
    (start.elapsed(), result.scalars[&format!("t.c{}", n - 1)])
}

#[test]
#[allow(clippy::float_cmp)] // small whole numbers
fn ten_thousand_scalars_calculate_in_time() {
    let (took, last) = time(10_000, false);
    assert!(took < limit(), "10,000 scalars took {took:?}");
    assert_eq!(last, Some(14998.5));
}

#[test]
#[allow(clippy::float_cmp)] // small whole numbers
fn ten_thousand_chained_scalars_calculate_in_time() {
    let (took, last) = time(10_000, true);
    assert!(took < limit(), "10,000 chained scalars took {took:?}");
    assert_eq!(last, Some(10000.0));
}

#[test]
#[allow(clippy::float_cmp)] // small whole numbers
fn a_thousand_sections_of_ten_calculate_in_time() {
    let mut text = String::from("_forge_version: \"5.4.0\"\n");
    for section in 0..1000 {
        writeln!(text, "s{section}:\n  v0: {{formula: \"={section} + 0\"}}").unwrap();
        for j in 1..10 {
            writeln!(text, "  v{j}: {{formula: \"=v{} + 1\"}}", j - 1).unwrap();
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sections.yaml");
    std::fs::write(&path, text).unwrap();
    let start = Instant::now();
    let result = calculate_core(&path, true, None).unwrap();
    let took = start.elapsed();
    assert!(took < limit(), "1,000 sections took {took:?}");
    assert_eq!(result.scalars["s999.v9"], Some(1008.0));
    assert_eq!(result.scalars["s0.v9"], Some(9.0));
}
