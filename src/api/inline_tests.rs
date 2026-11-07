//! The REST API accepts a model as text, like the MCP server does

use super::server::{build_router, ApiConfig};
use axum::body::Body;
use axum::http::{header, Method, Request};
use tower::ServiceExt;

const MODEL: &str = "_forge_version: \"5.0.0\"\nsales:\n  amount: [100, 200]\n  tax: \"=amount * 0.1\"\nsummary:\n  total:\n    value: null\n    formula: \"=SUM(sales.tax)\"\n";

async fn post(uri: &str, body: serde_json::Value) -> serde_json::Value {
    let request = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::HOST, "localhost:8080")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = build_router(&ApiConfig::default())
        .unwrap()
        .oneshot(request)
        .await
        .unwrap();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn test_calculate_inline_returns_the_numbers() {
    let out = post("/api/v1/calculate", serde_json::json!({ "content": MODEL })).await;
    let data = &out["data"];
    assert_eq!(data["calculated"], true, "{out}");
    let content = data["calculated_content"].as_str().unwrap();
    assert!(content.contains("30"), "{content}");
    assert!(data["result"].is_object(), "{out}");
}

#[tokio::test]
async fn test_validate_inline() {
    let out = post("/api/v1/validate", serde_json::json!({ "content": MODEL })).await;
    assert_eq!(out["data"]["valid"], true, "{out}");
    assert_eq!(out["data"]["result"]["scalar_count"], 1);
}

#[tokio::test]
async fn test_audit_inline() {
    let out = post(
        "/api/v1/audit",
        serde_json::json!({ "content": MODEL, "variable": "summary.total" }),
    )
    .await;
    assert_eq!(out["data"]["audited"], true, "{out}");
    assert_eq!(out["data"]["result"]["calculated_value"], 30.0, "{out}");
    let deps = out["data"]["result"]["dependencies"].as_array().unwrap();
    assert!(!deps.is_empty(), "{out}");
}

#[tokio::test]
async fn test_inline_includes_work() {
    let model = "_forge_version: \"5.0.0\"\n_includes:\n  - file: rates.yaml\n    as: rates\nsales:\n  amount: [100]\n  eur: \"=amount * @rates.fx.usd_eur\"\n";
    let rates = "_forge_version: \"5.0.0\"\nfx:\n  usd_eur:\n    value: 0.92\n    formula: null\n";
    let out = post(
        "/api/v1/calculate",
        serde_json::json!({ "content": model, "includes": { "rates": rates }, "dry_run": true }),
    )
    .await;
    assert_eq!(out["data"]["calculated"], true, "{out}");
}

#[tokio::test]
async fn test_inline_include_cannot_leave_the_temp_directory() {
    let model = "_forge_version: \"5.0.0\"\n_includes:\n  - file: ../../etc/passwd\n    as: rates\nx:\n  v:\n    value: 1\n    formula: null\n";
    let out = post(
        "/api/v1/calculate",
        serde_json::json!({ "content": model, "includes": { "rates": "a: 1" } }),
    )
    .await;
    assert_eq!(out["data"]["calculated"], false, "{out}");
    assert!(out["data"]["message"]
        .as_str()
        .unwrap()
        .contains("not allowed"));
}

#[tokio::test]
async fn test_file_path_and_content_together_is_an_error() {
    let out = post(
        "/api/v1/validate",
        serde_json::json!({ "file_path": "a.yaml", "content": MODEL }),
    )
    .await;
    assert_eq!(out["data"]["valid"], false);
    assert!(out["data"]["message"]
        .as_str()
        .unwrap()
        .contains("not both"));
}

#[tokio::test]
async fn test_neither_file_path_nor_content_is_an_error() {
    let out = post("/api/v1/validate", serde_json::json!({})).await;
    assert_eq!(out["data"]["valid"], false);
    assert!(out["data"]["message"]
        .as_str()
        .unwrap()
        .contains("required"));
}
