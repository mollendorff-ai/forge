//! A formula nested too deep is refused with a message, not calculated until
//! the stack overflows (roadmap T1-DEPTH). Before the limit, a formula of
//! 10,000 parentheses, or of 100,000 terms added, ended `forge calculate`
//! with a stack overflow (exit 134), and a `forge mcp` server died on the
//! same formula in a call, taking its session with it.

use mollendorff_forge::cli::calculate_core;
use mollendorff_forge::core::array_calculator::parser::{parse, MAX_DEPTH};
use mollendorff_forge::core::array_calculator::tokenizer::tokenize;

fn model(formula: &str) -> String {
    format!("_forge_version: \"5.1.0\"\nm:\n  x:\n    formula: \"={formula}\"\n")
}

fn accepted(formula: &str) -> bool {
    parse(tokenize(formula).unwrap()).is_ok()
}

/// What `m.x` calculates to, in a file of its own
fn calculated(formula: &str) -> Result<f64, String> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.yaml");
    std::fs::write(&path, model(formula)).unwrap();
    calculate_core(&path, true, None)
        .map(|result| result.scalars["m.x"].unwrap())
        .map_err(|e| e.to_string())
}

/// The longest chain `1+1+...` inside parentheses that is accepted as the
/// first operand of a longer one, and the longest longer one: the deepest
/// tree the limit lets through, nearly twice `MAX_DEPTH`.
fn deepest() -> (String, f64) {
    let chain = |n: usize| vec!["1"; n].join("+");
    let inner = (1..=MAX_DEPTH)
        .take_while(|&n| accepted(&format!("({})+1", chain(n))))
        .last()
        .unwrap();
    let outer = (1..=MAX_DEPTH)
        .take_while(|&n| accepted(&format!("({}){}", chain(inner), "+1".repeat(n))))
        .last()
        .unwrap();
    (
        format!("({}){}", chain(inner), "+1".repeat(outer)),
        (inner + outer) as f64,
    )
}

#[test]
fn a_formula_at_the_limit_calculates() {
    let pairs = MAX_DEPTH - 1;
    let parentheses = format!("{}7{}", "(".repeat(pairs), ")".repeat(pairs));
    assert_eq!(calculated(&parentheses), Ok(7.0));
    // The sign inside is one more level, so one call fewer
    let calls = format!("{}-3{}", "ABS(".repeat(pairs - 1), ")".repeat(pairs - 1));
    assert_eq!(calculated(&calls), Ok(3.0));
    let signs = format!("{}5", "-".repeat(pairs));
    assert_eq!(calculated(&signs), Ok(-5.0));
    let sum = vec!["1"; MAX_DEPTH].join("+");
    assert_eq!(calculated(&sum), Ok(MAX_DEPTH as f64));
}

#[test]
fn the_deepest_tree_the_limit_lets_through_calculates() {
    let (formula, value) = deepest();
    assert!(value > MAX_DEPTH as f64, "{value}: not the worst case");
    assert_eq!(calculated(&formula), Ok(value));
}

#[test]
fn a_formula_one_level_past_the_limit_is_refused_with_the_cell() {
    let pairs = MAX_DEPTH;
    let formula = format!("{}7{}", "(".repeat(pairs), ")".repeat(pairs));
    let message = calculated(&formula).unwrap_err();
    assert!(message.contains("m.x"), "{message}");
    assert!(
        message.contains(&format!("nested more than {MAX_DEPTH} levels")),
        "{message}"
    );
    let sum = vec!["1"; MAX_DEPTH + 1].join("+");
    assert!(calculated(&sum).is_err());
}

#[test]
fn a_hundred_thousand_levels_are_refused() {
    let n = 100_000;
    for formula in [
        format!("{}1{}", "(".repeat(n), ")".repeat(n)),
        format!("{}1{}", "ABS(".repeat(n), ")".repeat(n)),
        format!("{}1", "-".repeat(n)),
        vec!["2"; n].join("^"),
        vec!["1"; n].join("+"),
        vec!["1"; n].join("&"),
        vec!["1"; n].join("="),
        format!("LAMBDA(q, q){}", "(1)".repeat(n)),
    ] {
        let message = calculated(&formula).unwrap_err();
        assert!(message.contains("nested more than"), "{message}");
    }
}

#[cfg(not(coverage))]
mod processes {
    use super::{deepest, model, MAX_DEPTH};
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    const N: usize = 100_000;

    fn deep() -> String {
        format!("{}1{}", "(".repeat(N), ")".repeat(N))
    }

    #[test]
    fn forge_calculate_exits_with_an_error_not_a_stack_overflow() {
        let dir = tempfile::tempdir().unwrap();
        for (name, formula) in [
            ("parentheses", deep()),
            ("sum", vec!["1"; N].join("+")),
            ("signs", format!("{}1", "-".repeat(N))),
        ] {
            let path = dir.path().join(format!("{name}.yaml"));
            std::fs::write(&path, model(&formula)).unwrap();
            let out = Command::new(env!("CARGO_BIN_EXE_forge"))
                .args(["calculate", "--dry-run"])
                .arg(&path)
                .output()
                .expect("forge runs");
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert_eq!(out.status.code(), Some(1), "{name}: {stderr}");
            assert!(stderr.contains("m.x"), "{name}: {stderr}");
            assert!(
                stderr.contains(&format!("nested more than {MAX_DEPTH} levels")),
                "{name}: {stderr}"
            );
        }
    }

    /// The answers of `forge mcp` to `forge_calculate` on each model, in
    /// order, with the process still alive after the last
    fn answers(models: &[String]) -> Vec<String> {
        let mut child = Command::new(env!("CARGO_BIN_EXE_forge"))
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("forge mcp starts");
        let mut stdin = child.stdin.take().expect("stdin");
        let mut lines = BufReader::new(child.stdout.take().expect("stdout")).lines();
        let mut send = |request: serde_json::Value| writeln!(stdin, "{request}").unwrap();
        send(
            serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"}}}),
        );
        lines.next().expect("initialized").expect("a line");
        send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        let mut answers = Vec::new();
        for (id, content) in (2..).zip(models) {
            send(
                serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
                "params": {"name": "forge_calculate",
                           "arguments": {"content": content, "dry_run": true}}}),
            );
            let line = lines
                .next()
                .expect("the server answered, not died")
                .expect("a line");
            answers.push(line);
        }
        assert!(child.try_wait().unwrap().is_none(), "the server is alive");
        let _ = child.kill();
        let _ = child.wait();
        answers
    }

    #[test]
    fn forge_mcp_answers_a_deep_formula_with_an_error_and_the_next_call() {
        let fine = model("1 + 2");
        let (worst, value) = deepest();
        let answers = answers(&[
            model(&deep()),
            fine,
            model(&vec!["1"; N].join("+")),
            model(&worst),
        ]);
        assert!(answers[0].contains("nested more than"), "{}", answers[0]);
        assert!(answers[1].contains("\\\"m.x\\\":3.0"), "{}", answers[1]);
        assert!(answers[2].contains("nested more than"), "{}", answers[2]);
        let expected = format!("\\\"m.x\\\":{value:.1}");
        assert!(answers[3].contains(&expected), "{}", answers[3]);
    }
}
