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
