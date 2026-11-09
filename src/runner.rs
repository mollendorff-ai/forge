//! Test runner - executes E2E validation tests against Gnumeric.
//!
//! Pipeline:
//! 1. Load test specs from YAML files
//! 2. For each test, generate a minimal YAML with the formula
//! 3. Run forge export to create XLSX
//! 4. Use Gnumeric (ssconvert) to recalculate and export to CSV
//! 5. Compare results against expected values

use std::fmt::Write;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::engine::GnumericEngine;
use crate::types::{
    extract_skip_cases, extract_table_data_yaml, extract_test_cases, SkipCase, TestCase,
    TestResult, TestSpec,
};

/// Test runner for E2E validation.
pub struct TestRunner {
    /// Path to the forge binary.
    forge_binary: PathBuf,
    /// Gnumeric engine for validation.
    engine: GnumericEngine,
    /// Directory containing test spec files.
    tests_dir: PathBuf,
    /// All loaded test cases.
    test_cases: Vec<TestCase>,
    /// All loaded skip cases.
    skip_cases: Vec<SkipCase>,
}

impl TestRunner {
    /// Creates a new test runner.
    ///
    /// # Errors
    ///
    /// Returns an error if the tests directory does not exist or YAML files
    /// cannot be read.
    pub fn new(
        forge_binary: PathBuf,
        engine: GnumericEngine,
        tests_dir: PathBuf,
    ) -> anyhow::Result<Self> {
        let (test_cases, skip_cases) = Self::load_test_cases(&tests_dir)?;

        Ok(Self {
            forge_binary,
            engine,
            tests_dir,
            test_cases,
            skip_cases,
        })
    }

    /// Finds all YAML files in a directory recursively.
    #[allow(dead_code)]
    fn find_yaml_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(Self::find_yaml_files(&path)?);
            } else if path.extension().is_some_and(|e| e == "yaml") {
                files.push(path);
            }
        }
        Ok(files)
    }

    /// Loads all test cases from the tests directory.
    fn load_test_cases(tests_dir: &Path) -> anyhow::Result<(Vec<TestCase>, Vec<SkipCase>)> {
        let mut all_cases = Vec::new();
        let mut all_skips = Vec::new();

        if !tests_dir.exists() {
            anyhow::bail!("Tests directory does not exist: {}", tests_dir.display());
        }

        Self::load_test_cases_recursive(tests_dir, &mut all_cases, &mut all_skips)?;

        Ok((all_cases, all_skips))
    }

    fn load_test_cases_recursive(
        dir: &Path,
        all_cases: &mut Vec<TestCase>,
        all_skips: &mut Vec<SkipCase>,
    ) -> anyhow::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::load_test_cases_recursive(&path, all_cases, all_skips)?;
            } else if path.extension().is_some_and(|e| e == "yaml") {
                let content = fs::read_to_string(&path)?;
                match yaml_serde::from_str::<TestSpec>(&content) {
                    Ok(spec) => {
                        let cases = extract_test_cases(&spec, Some(&path));
                        let skips = extract_skip_cases(&spec);
                        all_cases.extend(cases);
                        all_skips.extend(skips);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {e}", path.display());
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns the total number of test cases.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Uses len() which isn't const
    pub fn total_tests(&self) -> usize {
        self.test_cases.len() + self.skip_cases.len()
    }

    #[allow(dead_code)]
    /// Returns test directory.
    #[must_use]
    pub const fn tests_dir(&self) -> &PathBuf {
        &self.tests_dir
    }

    /// Returns all test cases.
    #[must_use]
    pub fn test_cases(&self) -> &[TestCase] {
        &self.test_cases
    }

    /// Returns all skip cases.
    #[must_use]
    pub fn skip_cases(&self) -> &[SkipCase] {
        &self.skip_cases
    }

    /// Runs all tests and returns results.
    #[must_use]
    pub fn run_all(&self) -> Vec<TestResult> {
        self.skip_cases
            .iter()
            .map(|sc| TestResult::Skip {
                name: sc.name.clone(),
                reason: sc.reason.clone(),
            })
            .chain(self.test_cases.iter().map(|tc| self.run_test(tc)))
            .collect()
    }

    /// Runs all tests with streaming output via callback.
    pub fn run_all_streaming<F>(&self, mut on_result: F) -> Vec<TestResult>
    where
        F: FnMut(&TestResult),
    {
        let mut results = Vec::new();

        for skip_case in &self.skip_cases {
            let result = TestResult::Skip {
                name: skip_case.name.clone(),
                reason: skip_case.reason.clone(),
            };
            on_result(&result);
            results.push(result);
        }

        for tc in &self.test_cases {
            let result = self.run_test(tc);
            on_result(&result);
            results.push(result);
        }

        results
    }

    /// Runs all tests in batch mode (one workbook per source file, faster).
    ///
    /// Cases are grouped by the file they came from so that each batch
    /// carries that file's table data. Every case is checked twice: Forge's
    /// own `calculate` result and Gnumeric's recalculation of Forge's export.
    #[must_use]
    pub fn run_batch(&self) -> Vec<TestResult> {
        let mut results: Vec<TestResult> = self
            .skip_cases
            .iter()
            .map(|sc| TestResult::Skip {
                name: sc.name.clone(),
                reason: sc.reason.clone(),
            })
            .collect();

        let mut groups: Vec<(Option<PathBuf>, Vec<&TestCase>)> = Vec::new();
        for tc in &self.test_cases {
            match groups.iter_mut().find(|(src, _)| *src == tc.source_file) {
                Some((_, cases)) => cases.push(tc),
                None => groups.push((tc.source_file.clone(), vec![tc])),
            }
        }

        for (source, cases) in &groups {
            results.extend(self.run_group(source.as_deref(), cases));
        }

        results
    }

    /// Runs one group of cases that share a source file in a single workbook.
    fn run_group(&self, source: Option<&Path>, cases: &[&TestCase]) -> Vec<TestResult> {
        let fail_all = |error: String| -> Vec<TestResult> {
            cases
                .iter()
                .map(|tc| fail(tc, None, Some(error.clone())))
                .collect()
        };

        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => return fail_all(format!("Failed to create temp dir: {e}")),
        };
        let includes = match includes_for(source, temp_dir.path()) {
            Ok(block) => block,
            Err(e) => return fail_all(e),
        };
        let forge_version = cases
            .first()
            .map_or("1.0.0", |tc| tc.forge_version.as_str());
        let mut yaml_content = format!(
            "_forge_version: \"{forge_version}\"\n{includes}{}assumptions:\n",
            table_data_for(source)
        );
        for (i, tc) in cases.iter().enumerate() {
            let escaped_formula = tc.formula.replace('"', "\\\"");
            let _ = write!(
                yaml_content,
                "  test_{i}:\n    value: null\n    formula: \"{escaped_formula}\"\n"
            );
        }

        let yaml_path = temp_dir.path().join("batch.yaml");
        let xlsx_path = temp_dir.path().join("batch.xlsx");
        if let Err(e) = fs::write(&yaml_path, &yaml_content) {
            return fail_all(format!("Failed to write YAML: {e}"));
        }

        let forge_values = match self.forge_calculate(&yaml_path, "test_", cases.len()) {
            Ok(v) => v,
            Err(e) => vec![Err(e); cases.len()],
        };

        let gnumeric_values =
            match self.forge_export_and_recalc(&yaml_path, &xlsx_path, temp_dir.path()) {
                Ok((csv_files, as_values)) => Self::parse_batch_csv(&csv_files, cases.len())
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| not_calculated(&as_values, &format!("assumptions.test_{i}"), v))
                    .collect(),
                Err(e) => vec![Err(e); cases.len()],
            };

        let roundtrip_values = match self.forge_import_and_calculate(&xlsx_path, temp_dir.path()) {
            Ok(back) => match self.forge_calculate(&back, "test_", cases.len()) {
                Ok(v) => v,
                Err(e) => vec![Err(e); cases.len()],
            },
            Err(e) => vec![Err(e); cases.len()],
        };

        cases
            .iter()
            .enumerate()
            .map(|(i, tc)| {
                judge(
                    tc,
                    &forge_values[i],
                    &gnumeric_values[i],
                    &roundtrip_values[i],
                )
            })
            .collect()
    }

    /// Runs `forge calculate` on a copy of the spec and reads back the values
    /// of `{prefix}{i}` (or `test_result` when `count` is 0) from the
    /// `assumptions` group. This is Forge's own answer, with no Excel involved.
    fn forge_calculate(
        &self,
        yaml_path: &Path,
        prefix: &str,
        count: usize,
    ) -> Result<Vec<Result<f64, String>>, String> {
        let output = Command::new(&self.forge_binary)
            .arg("calculate")
            .arg(yaml_path)
            .output()
            .map_err(|e| format!("Failed to run forge calculate: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "forge calculate failed: {}{}",
                String::from_utf8_lossy(&output.stderr).trim(),
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .filter(|l| l.contains("Error"))
                    .collect::<Vec<_>>()
                    .join(" ")
            ));
        }

        let content = fs::read_to_string(yaml_path)
            .map_err(|e| format!("Failed to read calculated YAML: {e}"))?;
        let doc: yaml_serde::Value = yaml_serde::from_str(&content)
            .map_err(|e| format!("Failed to parse calculated YAML: {e}"))?;
        let group = doc
            .get("assumptions")
            .ok_or("forge calculate output has no assumptions group")?;

        // A scalar is `{value: ...}` (model format 5.0.0), or a plain number
        // in the 5.1.0 short form that `forge import` writes (Forge ADR-051).
        let read = |key: &str| -> Result<f64, String> {
            group
                .get(key)
                .and_then(|entry| entry.as_f64().or_else(|| entry.get("value")?.as_f64()))
                .ok_or_else(|| format!("forge calculate gave no numeric value for {key}"))
        };

        if count == 0 {
            Ok(vec![read("test_result")])
        } else {
            Ok((0..count).map(|i| read(&format!("{prefix}{i}"))).collect())
        }
    }

    /// Runs `forge export`, then has Gnumeric recalculate the workbook.
    fn forge_export_and_recalc(
        &self,
        yaml_path: &Path,
        xlsx_path: &Path,
        out_dir: &Path,
    ) -> Result<(Vec<PathBuf>, Vec<WrittenAsValue>), String> {
        let output = Command::new(&self.forge_binary)
            .arg("export")
            .arg(yaml_path)
            .arg(xlsx_path)
            .output()
            .map_err(|e| format!("Failed to run forge: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "forge export failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        // A formula Excel cannot express is written as its value, which
        // Gnumeric would only read back: list them (`- name: written as ...`).
        let written_as_values = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|l| l.trim().strip_prefix("- ")?.split_once(": written as "))
            .map(|(name, rest)| (name.to_string(), rest.to_string()))
            .collect();
        let csv_files = self
            .engine
            .xlsx_to_csv_all_sheets(xlsx_path, out_dir)
            .map_err(|e| format!("CSV conversion failed: {e}"))?;
        Ok((csv_files, written_as_values))
    }

    /// Runs `forge import` on Forge's own exported workbook, so the result can be
    /// calculated again: a formula that does not survive export and import shows up here.
    fn forge_import_and_calculate(
        &self,
        xlsx_path: &Path,
        out_dir: &Path,
    ) -> Result<PathBuf, String> {
        let back = out_dir.join("roundtrip.yaml");
        let output = Command::new(&self.forge_binary)
            .arg("import")
            .arg(xlsx_path)
            .arg(&back)
            .output()
            .map_err(|e| format!("Failed to run forge import: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "forge import failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(back)
    }

    /// Reads `test_<i>` rows out of the CSV sheets Gnumeric produced.
    fn parse_batch_csv(csv_files: &[PathBuf], count: usize) -> Vec<Result<f64, String>> {
        let mut results: Vec<Result<f64, String>> =
            vec![Err("Missing result in CSV output".to_string()); count];

        for csv_path in csv_files {
            let Ok(file) = fs::File::open(csv_path) else {
                continue;
            };
            for line in BufReader::new(file).lines() {
                let Ok(line) = line else { continue };
                let cells: Vec<&str> = line
                    .split(',')
                    .map(|s| s.trim_matches('"').trim())
                    .collect();
                if cells.len() < 2 {
                    continue;
                }
                let label = cells[0];
                let Some(idx) = label
                    .strip_prefix("assumptions.test_")
                    .or_else(|| label.strip_prefix("test_"))
                    .and_then(|i| i.parse::<usize>().ok())
                else {
                    continue;
                };
                if idx < count {
                    match cells[1].parse::<f64>() {
                        Ok(value) => results[idx] = Ok(value),
                        Err(_) => {
                            results[idx] =
                                Err(format!("Gnumeric cell is not a number: {}", cells[1]));
                        }
                    }
                }
            }
        }

        results
    }

    /// Runs a single test case.
    ///
    /// The case passes only when both Forge's own `calculate` result and
    /// Gnumeric's recalculation of Forge's exported workbook equal the
    /// expected value.
    #[must_use]
    pub fn run_test(&self, test_case: &TestCase) -> TestResult {
        let temp_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                return fail(
                    test_case,
                    None,
                    Some(format!("Failed to create temp dir: {e}")),
                )
            }
        };
        let includes = match includes_for(test_case.source_file.as_deref(), temp_dir.path()) {
            Ok(block) => block,
            Err(e) => return fail(test_case, None, Some(e)),
        };
        let escaped_formula = test_case.formula.replace('"', "\\\"");
        let forge_version = &test_case.forge_version;
        let yaml_content = format!(
            r#"_forge_version: "{forge_version}"
{includes}{table_data}assumptions:
  test_result:
    value: null
    formula: "{escaped_formula}"
"#,
            table_data = table_data_for(test_case.source_file.as_deref()),
        );

        let yaml_path = temp_dir.path().join("test.yaml");
        let xlsx_path = temp_dir.path().join("test.xlsx");

        if let Err(e) = fs::write(&yaml_path, &yaml_content) {
            return fail(test_case, None, Some(format!("Failed to write YAML: {e}")));
        }

        // Forge's own answer first: `export` does not run Forge's evaluator
        // on the formula, `calculate` does.
        let forge_value = self
            .forge_calculate(&yaml_path, "", 0)
            .and_then(|mut v| v.remove(0));

        let gnumeric_value = self
            .forge_export_and_recalc(&yaml_path, &xlsx_path, temp_dir.path())
            .and_then(|(files, as_values)| {
                let value = Self::find_result_in_csvs(&files);
                not_calculated(&as_values, "assumptions.test_result", value)
            });

        let roundtrip_value = self
            .forge_import_and_calculate(&xlsx_path, temp_dir.path())
            .and_then(|back| self.forge_calculate(&back, "", 0))
            .and_then(|mut v| v.remove(0));

        judge(test_case, &forge_value, &gnumeric_value, &roundtrip_value)
    }

    /// Finds the labelled `test_result` value in the CSV sheets.
    ///
    /// Only the cell next to the `test_result` label counts. A cell that
    /// happens to hold the expected number (a table value, say) is not a result.
    fn find_result_in_csvs(csv_files: &[PathBuf]) -> Result<f64, String> {
        for csv_path in csv_files {
            let file = fs::File::open(csv_path).map_err(|e| format!("Failed to open CSV: {e}"))?;
            for line in BufReader::new(file).lines() {
                let line = line.map_err(|e| format!("Failed to read line: {e}"))?;
                let cells: Vec<&str> = line
                    .split(',')
                    .map(|s| s.trim_matches('"').trim())
                    .collect();
                if cells.len() >= 2 && (cells[0] == "test_result" || cells[0] == "result") {
                    return cells[1]
                        .parse::<f64>()
                        .map_err(|_| format!("Gnumeric cell is not a number: {}", cells[1]));
                }
            }
        }
        Err("Could not find a labelled test_result row in any sheet".to_string())
    }
}

/// The spec's `_includes`, resolved: (file beside the spec, namespace).
fn includes_of(source: Option<&Path>) -> Vec<(PathBuf, String)> {
    let Some(path) = source else {
        return Vec::new();
    };
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    fs::read_to_string(path)
        .ok()
        .and_then(|content| yaml_serde::from_str::<TestSpec>(&content).ok())
        .map(|spec| {
            spec.includes
                .into_iter()
                .map(|inc| (dir.join(inc.file), inc.namespace))
                .collect()
        })
        .unwrap_or_default()
}

/// Copy the spec's included files into `dir`, the folder of the generated
/// model, and return its `_includes:` section naming those copies.
fn includes_for(source: Option<&Path>, dir: &Path) -> Result<String, String> {
    let mut block = String::new();
    for (file, namespace) in includes_of(source) {
        let name = file
            .file_name()
            .ok_or_else(|| format!("include without a file name: {}", file.display()))?;
        fs::copy(&file, dir.join(name))
            .map_err(|e| format!("Failed to copy include {}: {e}", file.display()))?;
        if block.is_empty() {
            block.push_str("_includes:\n");
        }
        let _ = writeln!(
            block,
            "  - file: {}\n    as: {namespace}",
            name.to_string_lossy()
        );
    }
    Ok(block)
}

/// Table sections of a spec file, as YAML to prepend to a generated model.
fn table_data_for(source: Option<&Path>) -> String {
    source
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|content| yaml_serde::from_str::<TestSpec>(&content).ok())
        .map(|spec| extract_table_data_yaml(&spec))
        .unwrap_or_default()
}

/// A formula `forge export` wrote as its value: (name, Forge's reason).
type WrittenAsValue = (String, String);

/// What Gnumeric gave for a formula `forge export` wrote as a value.
const NOT_CALCULATED: &str =
    "forge export wrote this formula as a value, so Gnumeric did not calculate it";

/// Gnumeric's value, unless Forge wrote the formula as a value: then
/// Gnumeric only read back Forge's number, and judged nothing, so the case
/// fails unless it has a `gnumeric_skip` saying why.
fn not_calculated(
    written_as_values: &[WrittenAsValue],
    name: &str,
    value: Result<f64, String>,
) -> Result<f64, String> {
    match written_as_values.iter().find(|(n, _)| n == name) {
        Some((_, reason)) => Err(format!("{NOT_CALCULATED}: written as {reason}")),
        None => value,
    }
}

/// Builds a failing result.
fn fail(tc: &TestCase, actual: Option<f64>, error: Option<String>) -> TestResult {
    TestResult::Fail {
        name: tc.name.clone(),
        formula: tc.formula.clone(),
        expected: tc.expected,
        actual,
        error,
    }
}

/// Whether a computed value equals the expected one.
fn matches(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1e-9_f64.max(expected.abs() * 1e-9)
}

/// Combines Forge's own result and Gnumeric's into one verdict.
fn judge(
    tc: &TestCase,
    forge: &Result<f64, String>,
    gnumeric: &Result<f64, String>,
    roundtrip: &Result<f64, String>,
) -> TestResult {
    let mut problems = Vec::new();
    let mut shown = None;

    match forge {
        Ok(v) if matches(*v, tc.expected) => {}
        Ok(v) => {
            shown = Some(*v);
            problems.push(format!(
                "forge calculate gave {v}, expected {}",
                tc.expected
            ));
        }
        Err(e) => problems.push(e.clone()),
    }
    match gnumeric {
        _ if tc.gnumeric_skip.is_some() => {}
        Ok(v) if matches(*v, tc.expected) => {}
        Ok(v) => {
            shown = shown.or(Some(*v));
            problems.push(format!(
                "Gnumeric recalculation of the export gave {v}, expected {}",
                tc.expected
            ));
        }
        Err(e) => problems.push(e.clone()),
    }
    match roundtrip {
        Ok(v) if matches(*v, tc.expected) => {}
        Ok(v) => {
            shown = shown.or(Some(*v));
            problems.push(format!(
                "after forge export and forge import, forge calculate gave {v}, expected {}",
                tc.expected
            ));
        }
        Err(e) => problems.push(format!("export and import: {e}")),
    }

    if problems.is_empty() {
        if let Ok(actual) = forge {
            return TestResult::Pass {
                name: tc.name.clone(),
                formula: tc.formula.clone(),
                expected: tc.expected,
                actual: *actual,
            };
        }
    }
    fail(tc, shown, Some(problems.join("; ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_empty_dir_returns_empty_cases() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = TestRunner::load_test_cases(temp_dir.path());
        assert!(result.is_ok());
        let (cases, skips) = result.unwrap();
        assert!(cases.is_empty());
        assert!(skips.is_empty());
    }

    #[test]
    fn load_nonexistent_dir_returns_error() {
        let result = TestRunner::load_test_cases(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn load_dir_with_yaml_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let yaml_content = r#"
_forge_version: "1.0.0"
assumptions:
  test_one:
    value: null
    formula: "=1+1"
    expected: 2
"#;
        fs::write(temp_dir.path().join("test.yaml"), yaml_content).unwrap();

        let result = TestRunner::load_test_cases(temp_dir.path());
        assert!(result.is_ok());
        let (cases, _) = result.unwrap();
        assert_eq!(cases.len(), 1);
    }
}
