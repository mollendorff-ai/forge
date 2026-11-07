//! `forge functions | head` must end quietly, without a panic message.

#![cfg(unix)]

use std::process::{Command, Stdio};

#[test]
fn closed_pipe_ends_without_a_panic() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_forge"))
        .arg("functions")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("forge starts");

    // Close the read end before the command has written its list.
    drop(child.stdout.take());

    let out = child.wait_with_output().expect("forge ends");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("panicked"), "stderr was: {err}");
}
