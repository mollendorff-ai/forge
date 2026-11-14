//! `forge serve --root DIR` (T1-MCP-ROOT): the REST API refuses a path
//! outside DIR, for reading and for writing, as the MCP server does.

use super::server::{build_router, ApiConfig};
use axum::body::Body;
use axum::http::{header, Method, Request};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use tempfile::TempDir;
use tower::ServiceExt;

const MODEL: &str = "_forge_version: \"5.1.0\"\na:\n  x: 2\n  y: {formula: \"=a.x * 3\"}\n";

async fn post(root: &Path, uri: &str, body: serde_json::Value) -> serde_json::Value {
    let config = ApiConfig {
        root: Some(root.to_path_buf()),
        ..ApiConfig::default()
    };
    let request = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::HOST, "localhost:8080")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = build_router(&config)
        .unwrap()
        .oneshot(request)
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// `models/` (the root, with `m.yaml` and a link `out` to `secret/`) and
/// `secret/s.yaml` beside it.
fn folders() -> (TempDir, std::path::PathBuf, std::path::PathBuf) {
    let base = TempDir::new().unwrap();
    let root = base.path().join("models");
    let secret = base.path().join("secret");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&secret).unwrap();
    fs::write(root.join("m.yaml"), MODEL).unwrap();
    fs::write(secret.join("s.yaml"), MODEL).unwrap();
    symlink(&secret, root.join("out")).unwrap();
    (base, root, secret)
}

fn says_outside(out: &serde_json::Value, field: &str) {
    let message = out["data"]["message"].as_str().unwrap_or_default();
    assert!(message.contains("outside"), "{out}");
    assert_eq!(out["data"][field], false, "{out}");
}

#[tokio::test]
async fn validate_reads_inside_the_root_and_refuses_outside() {
    let (_base, root, secret) = folders();
    let inside = post(
        &root,
        "/api/v1/validate",
        serde_json::json!({"file_path": "m.yaml"}),
    )
    .await;
    assert_eq!(inside["data"]["valid"], true, "{inside}");
    for path in [
        secret.join("s.yaml").display().to_string(),
        "../secret/s.yaml".to_string(),
        "out/s.yaml".to_string(),
    ] {
        let out = post(
            &root,
            "/api/v1/validate",
            serde_json::json!({"file_path": path}),
        )
        .await;
        says_outside(&out, "valid");
    }
}

#[tokio::test]
async fn calculate_and_audit_refuse_outside() {
    let (_base, root, secret) = folders();
    let path = secret.join("s.yaml").display().to_string();
    let out = post(
        &root,
        "/api/v1/calculate",
        serde_json::json!({"file_path": path}),
    )
    .await;
    says_outside(&out, "calculated");
    let out = post(
        &root,
        "/api/v1/audit",
        serde_json::json!({"file_path": path, "variable": "a.y"}),
    )
    .await;
    assert!(
        out["data"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("outside"),
        "{out}"
    );
}

#[tokio::test]
async fn export_and_import_cannot_write_outside() {
    let (_base, root, secret) = folders();
    let out = post(
        &root,
        "/api/v1/export",
        serde_json::json!({"yaml_path": "m.yaml", "excel_path": "out/x.xlsx"}),
    )
    .await;
    says_outside(&out, "exported");
    assert!(!secret.join("x.xlsx").exists());

    let out = post(
        &root,
        "/api/v1/export",
        serde_json::json!({"yaml_path": "m.yaml", "excel_path": "m.xlsx"}),
    )
    .await;
    assert_eq!(out["data"]["exported"], true, "{out}");

    let out = post(
        &root,
        "/api/v1/import",
        serde_json::json!({"excel_path": "m.xlsx", "yaml_path": "../secret/back.yaml"}),
    )
    .await;
    says_outside(&out, "imported");
    assert!(!secret.join("back.yaml").exists());
}

#[test]
fn a_root_that_is_not_a_folder_stops_the_server_from_starting() {
    let (_base, root, _secret) = folders();
    let config = ApiConfig {
        root: Some(root.join("m.yaml")),
        ..ApiConfig::default()
    };
    assert!(build_router(&config).is_err());
}
