//! Every model in docs/EXAMPLES.md runs, and every number the doc shows as its
//! output is what Forge gives (roadmap T1-EXAMPLES-OUTPUT).
//!
//! The doc once showed invented outputs (`45678.91`, `245678.90`) and models
//! that could not run. Each `yaml` block with a `_forge_version` is run with
//! the command its sections call for; the first plain code block after it is
//! its output, and each number in that block must match a number in the
//! result at the precision the doc shows.

use crate::forge;
use std::path::Path;

/// A fenced code block: its language tag and body.
#[must_use]
pub fn blocks(doc: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut lines = doc.lines();
    while let Some(line) = lines.next() {
        if let Some(lang) = line.strip_prefix("```") {
            let mut body = String::new();
            for inner in lines.by_ref() {
                if inner.starts_with("```") {
                    break;
                }
                body.push_str(inner);
                body.push('\n');
            }
            out.push((lang.trim().to_string(), body));
        }
    }
    out
}

/// The command a model's sections call for: an analysis for its section,
/// `calculate` otherwise.
#[must_use]
pub fn command_for(body: &str) -> &'static str {
    let has = |section: &str| body.lines().any(|l| l.starts_with(section));
    if has("monte_carlo:") {
        "simulate"
    } else if has("decision_tree:") {
        "decision-tree"
    } else if has("tornado:") {
        "tornado"
    } else if has("bootstrap:") {
        "bootstrap"
    } else if has("scenarios:") && body.contains("probability:") {
        "scenarios"
    } else {
        "calculate"
    }
}

/// Run a model as a reader would, with the command its sections call for.
///
/// The result is the JSON file an analysis writes with `-o`, or the values
/// `forge calculate --dry-run` prints; `Err` is what the command said when it
/// failed.
///
/// # Errors
///
/// Returns what the command wrote to standard error when it fails.
///
/// # Panics
///
/// Panics if the model or its result cannot be written or read in `dir`.
pub fn run(body: &str, dir: &Path, index: usize) -> Result<String, String> {
    let path = dir.join(format!("example_{index}.yaml"));
    std::fs::write(&path, body).expect("the model is written");
    let model = path.to_str().expect("a UTF-8 path");
    let command = command_for(body);
    if command == "calculate" {
        let ran = forge(&["calculate", model, "--dry-run"]);
        return if ran.success() {
            Ok(ran.out)
        } else {
            Err(ran.err)
        };
    }
    let result = dir.join(format!("example_{index}.json"));
    let ran = forge(&[command, model, "-o", result.to_str().expect("a UTF-8 path")]);
    if !ran.success() {
        return Err(ran.err);
    }
    Ok(std::fs::read_to_string(&result).expect("the result is written"))
}

/// Every number written in a text, as JSON writes them or as `forge` prints
/// them (`786.320361712362`, `-1.5e-7`).
#[must_use]
pub fn numbers_in(text: &str) -> Vec<f64> {
    let bytes = text.as_bytes();
    let digit = |k: usize| bytes.get(k).is_some_and(u8::is_ascii_digit);
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if !(digit(i) || (bytes[i] == b'-' && digit(i + 1))) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while digit(j) || bytes.get(j) == Some(&b'.') {
            j += 1;
        }
        if matches!(bytes.get(j), Some(b'e' | b'E')) {
            let sign = usize::from(matches!(bytes.get(j + 1), Some(b'-' | b'+')));
            if digit(j + 1 + sign) {
                j += 1 + sign;
                while digit(j) {
                    j += 1;
                }
            }
        }
        if let Ok(value) = text[i..j].trim_end_matches('.').parse() {
            out.push(value);
        }
        i = j;
    }
    out
}

/// Whether `chars[j]` is a comma between groups of three digits in the whole
/// part of the number that starts at `i`: the `,` of `4,272,727.27`.
pub fn thousands_separator(chars: &[char], i: usize, j: usize) -> bool {
    chars[j] == ','
        && !chars[i..j].contains(&'.')
        && chars
            .get(j + 1..j + 4)
            .is_some_and(|d| d.iter().all(char::is_ascii_digit))
        && !chars.get(j + 4).is_some_and(char::is_ascii_digit)
}

/// The numbers a doc output block shows, with their decimals. Labels such as
/// `P10` and `p=50%` are not numbers; a percentage is read as a fraction.
#[must_use]
pub fn shown_numbers(block: &str) -> Vec<(f64, usize)> {
    let mut out = Vec::new();
    let chars: Vec<char> = block.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let starts = chars[i].is_ascii_digit()
            && (i == 0
                || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_' || chars[i - 1] == '='));
        if !starts {
            i += 1;
            continue;
        }
        // A threshold in a label (`P(cost > 350000)`) is the condition, not a result
        let before = chars[..i].iter().rev().find(|c| **c != ' ');
        if matches!(before, Some('>' | '<')) {
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            continue;
        }
        // `-117` and `-$117,000.00`
        let negative =
            (i > 0 && chars[i - 1] == '-') || (i > 1 && chars[i - 1] == '$' && chars[i - 2] == '-');
        let mut j = i;
        while j < chars.len()
            && (chars[j].is_ascii_digit() || chars[j] == '.' || thousands_separator(&chars, i, j))
        {
            j += 1;
        }
        let text: String = chars[i..j].iter().filter(|c| **c != ',').collect();
        let text = text.trim_end_matches('.');
        let decimals = text.split_once('.').map_or(0, |(_, d)| d.len());
        let percent = chars.get(j) == Some(&'%');
        if let Ok(mut value) = text.parse::<f64>() {
            if negative {
                value = -value;
            }
            let (value, decimals) = if percent {
                (value / 100.0, decimals + 2)
            } else {
                (value, decimals)
            };
            // Small whole numbers are counts and labels (P50, 3 phases), not results
            if decimals > 0 || value.abs() >= 1000.0 {
                out.push((value, decimals));
            }
        }
        i = j;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_example_runs_and_shows_its_real_output() {
        let doc = crate::read("docs/EXAMPLES.md");
        let dir = tempfile::tempdir().unwrap();
        let blocks = blocks(&doc);
        let mut ran = 0;
        let mut checked = 0;
        for (index, (lang, body)) in blocks.iter().enumerate() {
            if lang != "yaml" || !body.contains("_forge_version") {
                continue;
            }
            let result = run(body, dir.path(), index);
            assert!(
                result.is_ok(),
                "example {index} does not run: {result:?}\n{body}"
            );
            let result = result.unwrap_or_default();
            ran += 1;
            // Its output: the next plain block before the next model
            let output = blocks[index + 1..]
                .iter()
                .take_while(|(l, b)| !(l == "yaml" && b.contains("_forge_version")))
                .find(|(l, _)| l.is_empty());
            let Some((_, output)) = output else { continue };
            let actual = numbers_in(&result);
            for (value, decimals) in shown_numbers(output) {
                let tolerance = 0.5f64.mul_add(10f64.powi(-i32::try_from(decimals).unwrap()), 1e-9);
                assert!(
                    actual.iter().any(|a| (a - value).abs() <= tolerance),
                    "example {index}: the doc shows {value} but Forge gives no such number\n{output}"
                );
                checked += 1;
            }
        }
        eprintln!("{ran} examples ran, {checked} output numbers checked");
        assert!(ran >= 15, "only {ran} examples found");
        assert!(checked >= 50, "only {checked} output numbers checked");
    }

    /// The real options and Bayesian sections show tables for files in
    /// examples/; their numbers must be what Forge gives for those files.
    #[test]
    fn tables_for_example_files_match() {
        let doc = crate::read("docs/EXAMPLES.md");

        let dir = tempfile::tempdir().unwrap();
        // What a command writes with `-o`, as JSON
        let written = |args: &[&str]| -> serde_json::Value {
            let path = dir.path().join("result.json");
            let mut all = args.to_vec();
            all.extend(["-o", path.to_str().unwrap()]);
            let printed = forge(&all).printed();
            assert!(printed.contains("Results written to"), "{printed}");
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
        };

        let options = written(&["real-options", "examples/real-options.yaml"]);
        for option in options["options"].as_object().unwrap().values() {
            let value = option["value"].as_f64().unwrap();
            let millions = format!("${:.2}M", value / 1e6);
            let shown = if value < 10_000.0 {
                format!("${:.3}M", value / 1e6)
            } else {
                millions
            };
            assert!(
                doc.contains(&shown),
                "doc does not show {} = {shown}",
                option["name"]
            );
        }

        let bayes = |query: &str, evidence: &[&str]| {
            let mut args = vec!["bayesian", "examples/bayesian.yaml", "-q", query];
            for e in evidence {
                args.extend(["-e", e]);
            }
            let q = written(&args);
            q["states"]
                .as_array()
                .unwrap()
                .iter()
                .zip(q["probabilities"].as_array().unwrap())
                .map(|(s, p)| {
                    format!(
                        "{} {:.2}%",
                        s.as_str().unwrap(),
                        p.as_f64().unwrap() * 100.0
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        for row in [
            bayes("default_probability", &[]),
            bayes("default_probability", &["economic_conditions=bad"]),
            bayes("economic_conditions", &["default_probability=high"]),
        ] {
            assert!(doc.contains(&row), "doc does not show: {row}");
        }
    }

    #[test]
    fn the_first_model_in_the_doc_is_the_starter_example() {
        let doc = crate::read("docs/EXAMPLES.md");
        let starter = crate::read("examples/starter.yaml");
        assert!(
            doc.contains(&format!("```yaml\n{starter}```")),
            "docs/EXAMPLES.md must show examples/starter.yaml as it is"
        );
    }

    #[test]
    fn an_example_that_fails_gives_what_the_command_said() {
        let dir = tempfile::tempdir().unwrap();
        // `calculate` for a model with no analysis section, `simulate` for one with
        for (index, body) in [(1, "x: [1,\n"), (2, "monte_carlo: [\n")] {
            let failed = run(body, dir.path(), index);
            assert!(failed.is_err(), "{body}: {failed:?}");
        }
    }

    #[test]
    fn numbers_are_read_from_what_forge_prints() {
        // An exponent with no digits is not part of the number, and a number that
        // does not parse is left out
        assert_eq!(numbers_in("1e x 1.2.3 -1.5e-7"), [1.0, -1.5e-7]);
        assert_eq!(shown_numbers("1.2.3"), Vec::<(f64, usize)>::new());
        // A count is not a result; a percentage is a fraction
        assert_eq!(
            shown_numbers("3 phases, 1.5 and 2,000 and 12%"),
            [(1.5, 1), (2000.0, 0), (0.12, 2)]
        );
    }
}
