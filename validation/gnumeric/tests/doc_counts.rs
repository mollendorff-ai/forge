//! The counts of cases, `LibreOffice` cases and files written in the
//! docs are what the suite loads (roadmap T1-CLAIMS). A count that differs
//! fails here, with the text to write.

use forge_e2e_gnumeric::runner::TestRunner;
use std::path::Path;

fn group(n: usize) -> String {
    if n >= 1000 {
        format!("{},{:03}", n / 1000, n % 1000)
    } else {
        n.to_string()
    }
}

fn files(dir: &Path) -> usize {
    std::fs::read_dir(dir).map_or(0, |entries| {
        entries
            .flatten()
            .map(|e| {
                let path = e.path();
                if path.is_dir() {
                    files(&path)
                } else {
                    usize::from(path.extension().is_some_and(|x| x == "yaml"))
                }
            })
            .sum()
    })
}

#[test]
fn the_docs_count_the_cases_the_suite_loads() {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (cases, skips) = TestRunner::load_test_cases(&here.join("tests")).unwrap();
    let total = cases.len() + skips.len();
    let libreoffice = cases.iter().filter(|c| c.libreoffice.is_some()).count();
    let forge_only = cases.iter().filter(|c| c.gnumeric_skip.is_some()).count();
    let spec_files = files(&here.join("tests"));
    assert!(
        total > 1900 && libreoffice > 100,
        "{total} cases, {libreoffice} for LibreOffice"
    );

    let root = here.join("../..");
    let read = |p: &str| std::fs::read_to_string(root.join(p)).unwrap();
    let wanted = [
        (
            "README.md",
            format!("{} formula tests compare Forge with Gnumeric", group(total)),
        ),
        ("VALIDATION.md", format!("{} formula cases", group(total))),
        (
            "VALIDATION.md",
            format!("LibreOffice for {libreoffice} cases"),
        ),
        (
            "VALIDATION.md",
            format!("of which {libreoffice} are recalculated by LibreOffice"),
        ),
        (
            "VALIDATION.md",
            format!("{forge_only} cases are checked against Forge only"),
        ),
        (
            "validation/gnumeric/README.md",
            format!(
                "**{} formula tests** across {spec_files} YAML files",
                group(total)
            ),
        ),
        (
            "validation/gnumeric/README.md",
            format!("for the {libreoffice} cases Gnumeric 1.12 cannot judge"),
        ),
    ];
    let missing: Vec<String> = wanted
        .iter()
        .filter(|(path, text)| !read(path).contains(text.as_str()))
        .map(|(path, text)| format!("{path} must say: {text}"))
        .collect();
    assert!(
        missing.is_empty(),
        "{total} cases, {libreoffice} for LibreOffice, {spec_files} files: {missing:#?}"
    );
}
