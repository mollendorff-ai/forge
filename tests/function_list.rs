//! `forge functions` lists only what a formula can call (roadmap
//! T1-FUNCTION-LIST). It listed Y, M, D, YD, YM and MD, which are units of
//! `DATEDIF`; `=YD(a, b)` was an unknown function.

use mollendorff_forge::cli::calculate_core;
use mollendorff_forge::functions::registry::all_functions;

#[test]
fn every_listed_function_is_known_to_the_calculator() {
    let dir = tempfile::tempdir().unwrap();
    let mut unknown = Vec::new();
    let mut panicked: Vec<&str> = Vec::new();
    for f in all_functions() {
        let path = dir
            .path()
            .join(format!("{}.yaml", f.name.replace('.', "_")));
        std::fs::write(
            &path,
            format!(
                "_forge_version: \"5.2.0\"\nx:\n  a: {{formula: \"={}()\"}}\n",
                f.name
            ),
        )
        .unwrap();
        // Calling with no arguments is usually an error; only "unknown" matters.
        // A function that panics on no arguments is known to the calculator.
        let outcome = std::panic::catch_unwind(|| calculate_core(&path, true, None));
        if let Ok(Err(e)) = outcome {
            if e.to_string().contains("Unknown function") {
                unknown.push(f.name);
            }
        } else if outcome.is_err() {
            panicked.push(f.name);
        }
    }
    assert_eq!(panicked, Vec::<&str>::new(), "panic on no arguments");
    assert_eq!(unknown, Vec::<&str>::new(), "listed but not callable");
}

#[test]
fn the_datedif_units_are_not_listed() {
    let names: Vec<&str> = all_functions().map(|f| f.name).collect();
    for unit in ["Y", "M", "D", "YD", "YM", "MD"] {
        assert!(!names.contains(&unit), "{unit} is a DATEDIF unit");
    }
    assert!(names.contains(&"DATEDIF"));
}
