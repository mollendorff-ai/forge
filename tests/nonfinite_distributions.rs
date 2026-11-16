//! A distribution with a parameter that is not a finite number is refused,
//! naming the scalar and the argument (roadmap T1-NONFINITE). `1E+400`, `inf`
//! and `NaN` are read as numbers: before this, `MC.PERT(0, 1, 1E+400)` ended
//! `forge simulate` on a panic (exit 134, as the release build aborts on a
//! panic) and a `forge mcp` server on the same call, while
//! `MC.Triangular(0, 1, 1E+400)` and `MC.Lognormal(1, 1E+400)` printed NaN
//! percentiles with exit 0.

#![cfg(not(coverage))]

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// How long to wait for an answer. A debug build unwinds a panic instead of
/// aborting, so before the fix its server never answered the call that
/// panicked; the test fails after this instead of hanging.
const ANSWER_TIMEOUT: Duration = Duration::from_mins(1);

/// Each distribution of the audit, and the start of the message it gives
const REFUSED: [(&str, &str); 8] = [
    (
        "MC.PERT(0, 1, 1E+400)",
        "MC.PERT max must be a finite number, not inf",
    ),
    (
        "MC.Normal(0, nan)",
        "MC.Normal stdev must be a finite number, not NaN",
    ),
    (
        "MC.Triangular(0, 1, 1E+400)",
        "MC.Triangular max must be a finite number, not inf",
    ),
    (
        "MC.Lognormal(1, 1E+400)",
        "MC.Lognormal stdev must be a finite number, not inf",
    ),
    (
        "MC.Uniform(0, inf)",
        "MC.Uniform max must be a finite number, not inf",
    ),
    (
        "MC.PERT(-1E+308, 1E+308, 1E+308)",
        "MC.PERT max - min is too large to calculate with",
    ),
    (
        "MC.Normal(0, 1E+308)",
        "MC.Normal: its arguments are too large to calculate with",
    ),
    (
        "MC.Lognormal(1, 1E+200)",
        "MC.Lognormal: its arguments are too large to calculate with",
    ),
];

fn model(distribution: &str) -> String {
    format!(
        "_forge_version: \"5.1.0\"\nmonte_carlo:\n  iterations: 200\n  seed: 1\n  outputs:\n    \
         - variable: m.out\n      percentiles: [50]\nm:\n  x:\n    formula: \"={distribution}\"\n  \
         out:\n    formula: \"=x * 2\"\n"
    )
}

#[test]
fn forge_simulate_exits_1_naming_the_scalar_and_the_argument() {
    let dir = tempfile::tempdir().unwrap();
    for (distribution, message) in REFUSED {
        let path = dir.path().join("m.yaml");
        std::fs::write(&path, model(distribution)).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_forge"))
            .arg("simulate")
            .arg(&path)
            .output()
            .expect("forge runs");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{distribution}: {stderr}");
        assert!(
            stderr.contains(&format!("m.x: {message}")),
            "{distribution}: {stderr}"
        );
        assert!(!String::from_utf8_lossy(&out.stdout).contains("NaN"));
    }
}

#[test]
fn a_wide_but_finite_distribution_still_simulates() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("m.yaml");
    std::fs::write(&path, model("MC.Triangular(-1E+307, 0, 1E+307)")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("simulate")
        .arg(&path)
        .output()
        .expect("forge runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The draws, the mean, the percentiles and the extremes are finite. The
    // standard deviation of values near 1e307 overflows when squared, as for
    // any output above about 1e154: roadmap T1-HUGE-STATS, not this item.
    assert!(!stdout.contains("NaN"), "{stdout}");
    let infinite: Vec<&str> = stdout.lines().filter(|l| l.contains("inf")).collect();
    assert_eq!(infinite.len(), 1, "{infinite:?}");
    assert!(
        infinite[0].trim_start().starts_with("Std Dev:"),
        "{infinite:?}"
    );
}

#[test]
fn forge_mcp_answers_each_with_an_error_and_then_simulates() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("forge mcp starts");
    let mut stdin = child.stdin.take().expect("stdin");
    let (tx, rx) = mpsc::channel();
    let stdout = child.stdout.take().expect("stdout");
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let next = || {
        rx.recv_timeout(ANSWER_TIMEOUT)
            .expect("the server answered in time, and did not die")
    };
    let mut send = |request: serde_json::Value| writeln!(stdin, "{request}").unwrap();
    send(
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {},
        "clientInfo": {"name": "test", "version": "0"}}}),
    );
    next();
    send(serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
    let mut call = |id: u64, distribution: &str| {
        send(
            serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "forge_simulate",
                       "arguments": {"content": model(distribution)}}}),
        );
        next()
    };
    for ((distribution, message), id) in REFUSED.into_iter().zip(2..) {
        let answer = call(id, distribution);
        assert!(
            answer.contains("\"isError\":true"),
            "{distribution}: {answer}"
        );
        assert!(
            answer.contains(&format!("m.x: {message}")),
            "{distribution}: {answer}"
        );
    }
    let answer = call(100, "MC.PERT(0, 5, 10)");
    assert!(answer.contains("\"isError\":false"), "{answer}");
    assert!(answer.contains("monte_carlo_results"), "{answer}");
    assert!(child.try_wait().unwrap().is_none(), "the server is alive");
    let _ = child.kill();
    let _ = child.wait();
}
