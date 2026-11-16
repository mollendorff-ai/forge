//! `forge mcp` keeps file paths inside the folder it starts in unless told
//! otherwise (evaluation of rc.27, fix 7): an agent, or a prompt injected
//! into one, cannot name any file the user can reach. `--any-path` lifts the
//! limit, and `--root DIR` names another folder.

#![cfg(not(coverage))]

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const MODEL: &str = r#"_forge_version: "5.4.0"
inputs:
  price: 4
out:
  total:
    formula: "=inputs.price * 2"
"#;

/// Part of the refusal for a path outside the folder (`src/path_root.rs`)
const REFUSED: &str = "may use";

/// What `forge mcp` answers to `forge_validate` on `file`, started in `cwd`
/// with `args` after `mcp`.
fn validate(cwd: &Path, args: &[&str], file: &Path) -> String {
    let mut child = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("mcp")
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("forge mcp starts");
    let mut stdin = child.stdin.take().expect("stdin");
    let requests = [
        serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": {"name": "test", "version": "0"}}}),
        serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
            "name": "forge_validate",
            "arguments": {"file_path": file.display().to_string()}}}),
    ];
    for request in requests {
        writeln!(stdin, "{request}").expect("request written");
    }
    let mut answer = String::new();
    for line in BufReader::new(child.stdout.take().expect("stdout")).lines() {
        let line = line.expect("a line");
        if line.contains("\"id\":2") {
            answer = line;
            break;
        }
    }
    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    answer
}

fn folders() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let top = tempfile::tempdir().expect("temp dir");
    let inside = top.path().join("here");
    let outside = top.path().join("elsewhere");
    std::fs::create_dir(&inside).unwrap();
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(inside.join("model.yaml"), MODEL).unwrap();
    std::fs::write(outside.join("model.yaml"), MODEL).unwrap();
    (top, inside, outside)
}

#[test]
fn by_default_a_path_outside_the_starting_folder_is_refused() {
    let (_top, inside, outside) = folders();
    let answer = validate(&inside, &[], &outside.join("model.yaml"));
    assert!(answer.contains(REFUSED), "{answer}");
    let answer = validate(&inside, &[], &inside.join("model.yaml"));
    assert!(!answer.contains(REFUSED), "{answer}");
}

#[test]
fn any_path_lifts_the_limit() {
    let (_top, inside, outside) = folders();
    let answer = validate(&inside, &["--any-path"], &outside.join("model.yaml"));
    assert!(!answer.is_empty() && !answer.contains(REFUSED), "{answer}");
}

#[test]
fn root_names_another_folder() {
    let (_top, inside, outside) = folders();
    let root = outside.display().to_string();
    let answer = validate(&inside, &["--root", &root], &outside.join("model.yaml"));
    assert!(!answer.is_empty() && !answer.contains(REFUSED), "{answer}");
    let answer = validate(&inside, &["--root", &root], &inside.join("model.yaml"));
    assert!(answer.contains(REFUSED), "{answer}");
}

#[test]
fn root_and_any_path_cannot_be_given_together() {
    let out = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["mcp", "--any-path", "--root", "."])
        .output()
        .expect("forge runs");
    assert!(!out.status.success());
}
