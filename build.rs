//! Stamps `forge --version` with the commit a build came from.
//!
//! A release says `10.0.0-rc.13`. A build of any other commit says
//! `10.0.0-rc.13 (a8ec1db)`, or `10.0.0-rc.13 (a8ec1db, modified)` with
//! uncommitted changes, so a bug report says which code it came from. It is
//! plain text, not a `-dev` version: Auto Release tags a version change.
//!
//! A build with no Git history around it (crates.io, a source archive) is
//! plain, and so is a build with `FORGE_RELEASE` set (the release workflow).

use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn stamp(version: &str) -> Option<String> {
    if std::env::var_os("FORGE_RELEASE").is_some() {
        return None;
    }
    let commit = git(&["rev-parse", "--short=7", "HEAD"])?;
    let release_tag = format!("v{version}");
    let tags = git(&["tag", "--points-at", "HEAD"])?;
    let modified = git(&["status", "--porcelain", "--untracked-files=no"])
        .is_some_and(|status| !status.is_empty());
    if !modified && tags.lines().any(|t| t == release_tag) {
        return None;
    }
    Some(if modified {
        format!("{commit}, modified")
    } else {
        commit
    })
}

fn main() {
    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    let text = stamp(&version).map_or_else(|| version.clone(), |s| format!("{version} ({s})"));
    println!("cargo:rustc-env=FORGE_VERSION_TEXT={text}");
    println!("cargo:rerun-if-env-changed=FORGE_RELEASE");
    println!("cargo:rerun-if-changed=build.rs");
    // A new commit, a checkout, a tag: the files Git moves
    for path in [".git/HEAD", ".git/index", ".git/refs", ".git/packed-refs"] {
        if std::path::Path::new(path).exists() {
            println!("cargo:rerun-if-changed={path}");
        }
    }
}
