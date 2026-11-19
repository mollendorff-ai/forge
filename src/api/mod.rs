//! Forge API Server module (v2.0.0)
//!
//! Provides an HTTP REST API for integration with other systems.
//! Run with `forge serve`.

pub mod handlers;
pub mod server;

pub use server::run_api_server;

/// Run the REST API server (`forge serve`) until it is stopped.
///
/// # Errors
///
/// Returns an error if the runtime cannot start, or the server cannot bind
/// its address or fails while serving.
pub fn serve(config: server::ApiConfig) -> crate::error::ForgeResult<()> {
    use crate::error::ForgeError;
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| ForgeError::Validation(format!("Failed to create runtime: {e}")))?;
    runtime
        .block_on(run_api_server(config))
        .map_err(|e| ForgeError::Validation(format!("Server error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    mod inline {
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
            let rates =
                "_forge_version: \"5.0.0\"\nfx:\n  usd_eur:\n    value: 0.92\n    formula: null\n";
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
    }

    #[cfg(unix)]
    mod root {
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
    }

    mod security {
        //! The API must not be usable by other websites or other machines by default

        use super::server::{build_router, ApiConfig};
        use axum::body::Body;
        use axum::http::{header, Method, Request, StatusCode};
        use axum::response::Response;
        use tower::ServiceExt;

        const TOKEN: &str = "s3cret-token";

        async fn send(config: &ApiConfig, request: Request<Body>) -> Response {
            build_router(config)
                .unwrap()
                .oneshot(request)
                .await
                .unwrap()
        }

        fn validate_request(host: &str, auth: Option<&str>) -> Request<Body> {
            let mut builder = Request::builder()
                .method(Method::POST)
                .uri("/api/v1/validate")
                .header(header::HOST, host)
                .header(header::CONTENT_TYPE, "application/json");
            if let Some(auth) = auth {
                builder = builder.header(header::AUTHORIZATION, auth);
            }
            builder
                .body(Body::from(r#"{"file_path": "does-not-exist.yaml"}"#))
                .unwrap()
        }

        fn preflight(origin: &str) -> Request<Body> {
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/import")
                .header(header::HOST, "localhost:8080")
                .header(header::ORIGIN, origin)
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
                .body(Body::empty())
                .unwrap()
        }

        fn allowed_origin(response: &Response) -> Option<String> {
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .map(|value| value.to_str().unwrap().to_string())
        }

        #[tokio::test]
        async fn test_local_host_headers_are_accepted() {
            let config = ApiConfig::default();
            for host in [
                "localhost:8080",
                "127.0.0.1:8080",
                "[::1]:8080",
                "localhost",
            ] {
                let response = send(&config, validate_request(host, None)).await;
                assert_ne!(response.status(), StatusCode::FORBIDDEN, "{host}");
            }
        }

        #[tokio::test]
        async fn test_foreign_host_header_is_rejected_on_loopback() {
            // A DNS-rebinding page reaches 127.0.0.1 with its own name in Host
            let response = send(
                &ApiConfig::default(),
                validate_request("evil.example:8080", None),
            )
            .await;
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
        }

        #[tokio::test]
        async fn test_no_cross_origin_access_by_default() {
            let response = send(&ApiConfig::default(), preflight("https://evil.example")).await;
            assert_eq!(allowed_origin(&response), None);
        }

        #[tokio::test]
        async fn test_listed_cors_origin_only() {
            let config = ApiConfig {
                cors_origins: vec!["http://localhost:3000".to_string()],
                ..ApiConfig::default()
            };
            let allowed = send(&config, preflight("http://localhost:3000")).await;
            assert_eq!(
                allowed_origin(&allowed).as_deref(),
                Some("http://localhost:3000")
            );
            let other = send(&config, preflight("https://evil.example")).await;
            assert_eq!(allowed_origin(&other), None);
        }

        #[tokio::test]
        async fn test_token_is_required_on_api_routes() {
            let config = ApiConfig {
                token: Some(TOKEN.to_string()),
                ..ApiConfig::default()
            };
            for auth in [
                None,
                Some("Bearer wrong-token!"),
                Some("s3cret-token"),
                Some("Bearer s3cret-tok"),
            ] {
                let response = send(&config, validate_request("localhost", auth)).await;
                assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{auth:?}");
                assert!(response.headers().contains_key(header::WWW_AUTHENTICATE));
            }
            let response = send(
                &config,
                validate_request("localhost", Some("Bearer s3cret-token")),
            )
            .await;
            assert_ne!(response.status(), StatusCode::UNAUTHORIZED);

            // Health checks stay open
            let health = Request::builder()
                .uri("/health")
                .header(header::HOST, "localhost")
                .body(Body::empty())
                .unwrap();
            assert_eq!(send(&config, health).await.status(), StatusCode::OK);
        }

        #[test]
        fn test_non_loopback_host_needs_a_token() {
            let open = ApiConfig {
                host: "0.0.0.0".to_string(),
                ..ApiConfig::default()
            };
            assert!(open.check().unwrap_err().contains("without a token"));
            assert!(build_router(&open).is_err());

            let with_token = ApiConfig {
                token: Some(TOKEN.to_string()),
                ..open
            };
            assert!(with_token.check().is_ok());

            for host in ["127.0.0.1", "localhost", "::1", "[::1]", "127.0.0.2"] {
                let config = ApiConfig {
                    host: host.to_string(),
                    ..ApiConfig::default()
                };
                assert!(config.is_loopback(), "{host}");
                assert!(config.check().is_ok(), "{host}");
            }
        }

        #[test]
        fn test_bad_settings_are_rejected() {
            let empty_token = ApiConfig {
                token: Some(String::new()),
                ..ApiConfig::default()
            };
            assert!(empty_token.check().is_err());
            let wildcard = ApiConfig {
                cors_origins: vec!["*".to_string()],
                ..ApiConfig::default()
            };
            assert!(wildcard.check().unwrap_err().contains("any website"));
        }
    }
}
