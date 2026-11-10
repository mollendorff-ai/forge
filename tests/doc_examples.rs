//! Every model in docs/EXAMPLES.md runs, and every number the doc shows as its
//! output is what Forge gives (roadmap T1-EXAMPLES-OUTPUT).
//!
//! The doc once showed invented outputs (`45678.91`, `245678.90`) and models
//! that could not run. Each `yaml` block with a `_forge_version` is run with
//! the command its sections call for; the first plain code block after it is
//! its output, and each number in that block must match a number in the
//! result at the precision the doc shows.

use mollendorff_forge::cli::{
    bootstrap_core, calculate_core, decision_tree_core, scenarios_core, simulate_core, tornado_core,
};
use std::path::Path;

/// A fenced code block: its language tag and body.
fn blocks(doc: &str) -> Vec<(String, String)> {
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

/// Run a model with the command its sections call for; the result as JSON text.
fn run(body: &str, dir: &Path, index: usize) -> Result<String, String> {
    let path = dir.join(format!("example_{index}.yaml"));
    std::fs::write(&path, body).unwrap();
    let json = |v: serde_json::Value| v.to_string();
    let has = |section: &str| body.lines().any(|l| l.starts_with(section));
    if has("monte_carlo:") {
        let r = simulate_core(&path, mollendorff_forge::cli::SimulateOptions::default())
            .map_err(|e| e.to_string())?;
        r.to_json().map_err(|e| e.to_string())
    } else if has("decision_tree:") {
        decision_tree_core(&path)
            .map(|r| json(serde_json::to_value(r).unwrap()))
            .map_err(|e| e.to_string())
    } else if has("tornado:") {
        tornado_core(&path, None)
            .map(|r| json(serde_json::to_value(r).unwrap()))
            .map_err(|e| e.to_string())
    } else if has("bootstrap:") {
        bootstrap_core(&path, None, None, None)
            .map(|r| json(serde_json::to_value(r).unwrap()))
            .map_err(|e| e.to_string())
    } else if has("scenarios:") && body.contains("probability:") {
        scenarios_core(&path, None)
            .map(|r| json(serde_json::to_value(r).unwrap()))
            .map_err(|e| e.to_string())
    } else {
        calculate_core(&path, true, None)
            .map(|r| json(serde_json::to_value(r).unwrap()))
            .map_err(|e| e.to_string())
    }
}

/// Every number in a JSON text.
fn numbers_in(json: &str) -> Vec<f64> {
    let mut out = Vec::new();
    collect(&serde_json::from_str(json).unwrap(), &mut out);
    out
}

fn collect(v: &serde_json::Value, out: &mut Vec<f64>) {
    match v {
        serde_json::Value::Number(n) => out.extend(n.as_f64()),
        serde_json::Value::Array(a) => a.iter().for_each(|x| collect(x, out)),
        serde_json::Value::Object(m) => m.values().for_each(|x| collect(x, out)),
        _ => {},
    }
}

/// The numbers a doc output block shows, with their decimals. Labels such as
/// `P10` and `p=50%` are not numbers; a percentage is read as a fraction.
fn shown_numbers(block: &str) -> Vec<(f64, usize)> {
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
        let negative = i > 0 && chars[i - 1] == '-';
        let mut j = i;
        while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
            j += 1;
        }
        let text: String = chars[i..j].iter().collect();
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

#[test]
fn every_example_runs_and_shows_its_real_output() {
    let doc =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/EXAMPLES.md"))
            .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let blocks = blocks(&doc);
    let mut ran = 0;
    let mut checked = 0;
    for (index, (lang, body)) in blocks.iter().enumerate() {
        if lang != "yaml" || !body.contains("_forge_version") {
            continue;
        }
        let result = run(body, dir.path(), index)
            .unwrap_or_else(|e| panic!("example {index} does not run: {e}\n{body}"));
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc = std::fs::read_to_string(root.join("docs/EXAMPLES.md")).unwrap();

    let options =
        mollendorff_forge::cli::real_options_core(&root.join("examples/real-options.yaml"))
            .unwrap();
    for option in options.options.values() {
        let millions = format!("${:.2}M", option.value / 1e6);
        let shown = if option.value < 10_000.0 {
            format!("${:.3}M", option.value / 1e6)
        } else {
            millions
        };
        assert!(
            doc.contains(&shown),
            "doc does not show {} = {shown}",
            option.name
        );
    }

    let bayes = |query: &str, evidence: &[&str]| {
        let ev: Vec<String> = evidence.iter().map(ToString::to_string).collect();
        let r = mollendorff_forge::cli::bayesian_core(
            &root.join("examples/bayesian.yaml"),
            Some(query),
            &ev,
        )
        .unwrap();
        let q = &r.queries[query];
        q.states
            .iter()
            .zip(&q.probabilities)
            .map(|(s, p)| format!("{s} {:.2}%", p * 100.0))
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
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let doc = std::fs::read_to_string(root.join("docs/EXAMPLES.md")).unwrap();
    let starter = std::fs::read_to_string(root.join("examples/starter.yaml")).unwrap();
    assert!(
        doc.contains(&format!("```yaml\n{starter}```")),
        "docs/EXAMPLES.md must show examples/starter.yaml as it is"
    );
}
