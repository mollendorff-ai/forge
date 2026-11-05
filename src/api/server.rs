//! Forge API Server implementation
//!
//! HTTP REST API server using Axum for enterprise integrations.
//! Provides endpoints for validate, calculate, audit, export, import.

// During coverage builds, stubbed functions don't use all imports
#![cfg_attr(coverage, allow(unused_imports, dead_code))]

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use super::handlers;

/// API Server configuration
///
/// The API reads and writes files that requests name, with the permissions of
/// the user running `forge serve`. By default it only accepts requests from
/// this machine, rejects browser requests from other sites, and needs a token
/// before it will listen on any other address.
#[derive(Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    /// Bearer token required on `/api/v1/*` (`Authorization: Bearer <token>`)
    pub token: Option<String>,
    /// Browser origins allowed to call the API (none by default)
    pub cors_origins: Vec<String>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            token: None,
            cors_origins: Vec::new(),
        }
    }
}

impl ApiConfig {
    /// Whether `host` only accepts connections from this machine
    #[must_use]
    pub fn is_loopback(&self) -> bool {
        self.host == "localhost"
            || self
                .host
                .trim_matches(|c| c == '[' || c == ']')
                .parse::<IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    }

    /// `host:port`, with an IPv6 host in brackets
    #[must_use]
    pub fn socket_address(&self) -> String {
        if self.host.contains(':') && !self.host.starts_with('[') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }

    /// Check the configuration before the server starts.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty token, a wildcard or invalid CORS origin,
    /// or a non-loopback address without a token.
    pub fn check(&self) -> Result<(), String> {
        if self.token.as_deref() == Some("") {
            return Err("The API token must not be empty".to_string());
        }
        if !self.is_loopback() && self.token.is_none() {
            return Err(format!(
                "Refusing to listen on {} without a token: the API reads and writes files \
                 on this machine. Set FORGE_API_TOKEN (or --token), or use --host 127.0.0.1.",
                self.host
            ));
        }
        for origin in &self.cors_origins {
            if origin == "*" {
                return Err(
                    "--cors-origin '*' would let any website call the API; list the origins"
                        .to_string(),
                );
            }
            HeaderValue::from_str(origin).map_err(|_| format!("Invalid CORS origin: {origin}"))?;
        }
        Ok(())
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub version: String,
    /// Token required on `/api/v1/*`, if any
    pub token: Option<String>,
    /// Reject requests whose `Host` header is not this machine
    pub loopback_only: bool,
}

/// Whether a `Host` header names this machine (`localhost`, `127.0.0.1` or
/// `[::1]`, with or without a port)
fn is_local_host_header(host: &str) -> bool {
    let name = if host.starts_with('[') {
        host.split_inclusive(']').next().unwrap_or(host)
    } else {
        host.split(':').next().unwrap_or(host)
    };
    matches!(
        name.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "[::1]"
    )
}

/// Compare two tokens without stopping at the first difference
fn tokens_match(given: &[u8], expected: &[u8]) -> bool {
    given.len() == expected.len()
        && given
            .iter()
            .zip(expected)
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

fn reject(status: StatusCode, message: &str) -> Response {
    let mut response = (status, Json(serde_json::json!({ "error": message }))).into_response();
    if status == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    response
}

/// Reject requests with a foreign `Host` header (DNS rebinding) and, when a
/// token is configured, `/api/` requests without it.
async fn guard(State(state): State<Arc<AppState>>, request: Request, next: Next) -> Response {
    if state.loopback_only {
        let host = request
            .headers()
            .get(header::HOST)
            .and_then(|value| value.to_str().ok());
        if !host.is_some_and(is_local_host_header) {
            return reject(StatusCode::FORBIDDEN, "Host header must name this machine");
        }
    }
    if let Some(expected) = &state.token {
        if request.uri().path().starts_with("/api/") {
            let given = request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "));
            if !given.is_some_and(|token| tokens_match(token.as_bytes(), expected.as_bytes())) {
                return reject(StatusCode::UNAUTHORIZED, "Missing or invalid API token");
            }
        }
    }
    next.run(request).await
}

/// Build the router with the security layers for `config`.
///
/// # Errors
///
/// Returns an error if [`ApiConfig::check`] rejects the configuration.
pub fn build_router(config: &ApiConfig) -> Result<Router, String> {
    config.check()?;
    let state = Arc::new(AppState {
        version: env!("CARGO_PKG_VERSION").to_string(),
        token: config.token.clone(),
        loopback_only: config.is_loopback(),
    });

    let router = Router::new()
        // Health and info endpoints
        .route("/", get(handlers::root))
        .route("/health", get(handlers::health))
        .route("/version", get(handlers::version))
        // Core API endpoints
        .route("/api/v1/validate", post(handlers::validate))
        .route("/api/v1/calculate", post(handlers::calculate))
        .route("/api/v1/audit", post(handlers::audit))
        .route("/api/v1/export", post(handlers::export))
        .route("/api/v1/import", post(handlers::import_excel))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state);

    // Without a CORS layer, browsers refuse cross-origin JSON requests
    if config.cors_origins.is_empty() {
        return Ok(router);
    }
    let origins = config
        .cors_origins
        .iter()
        .map(|origin| {
            HeaderValue::from_str(origin).map_err(|_| format!("Invalid CORS origin: {origin}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(router.layer(
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(origins))
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION]),
    ))
}

/// Run the API server
///
/// # Errors
///
/// Returns an error if the server address cannot be parsed, the TCP listener
/// fails to bind, or the server encounters a fatal runtime error.
///
/// # Coverage Exclusion (ADR-006)
/// This function binds to a real TCP port and runs forever until terminated.
/// Cannot be unit tested - verified via integration tests in `binary_integration_tests.rs`
#[cfg(not(coverage))]
pub async fn run_api_server(config: ApiConfig) -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "forge=info,tower_http=info".into()),
        )
        .init();

    let app = build_router(&config)
        .map_err(anyhow::Error::msg)?
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = config.socket_address().parse()?;
    info!("🔥 Forge API Server starting on http://{}", addr);
    info!("   Endpoints: /api/v1/validate, /api/v1/calculate, /api/v1/audit, /api/v1/export, /api/v1/import");
    info!("   Health: /health, Version: /version");
    info!(
        "   Token: {}. Browser origins allowed: {}",
        if config.token.is_some() {
            "required on /api/v1/*"
        } else {
            "none (local use only)"
        },
        if config.cors_origins.is_empty() {
            "none".to_string()
        } else {
            config.cors_origins.join(", ")
        }
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Forge API Server shutdown complete");
    Ok(())
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
pub async fn run_api_server(_config: ApiConfig) -> anyhow::Result<()> {
    Ok(())
}

/// Graceful shutdown signal handler
///
/// # Coverage Exclusion (ADR-006)
/// Waits for OS signals (Ctrl+C, SIGTERM) - cannot be triggered in unit tests
#[cfg(not(coverage))]
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("Shutdown signal received, stopping server...");
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
async fn shutdown_signal() {}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ApiConfig Tests ====================

    #[test]
    fn test_default_config() {
        let config = ApiConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
    }

    #[test]
    fn test_config_custom_values() {
        let config = ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            ..ApiConfig::default()
        };
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_config_clone() {
        let config1 = ApiConfig::default();
        let config2 = config1.clone();
        assert_eq!(config1.host, config2.host);
        assert_eq!(config1.port, config2.port);
    }

    #[test]
    fn test_config_address_format() {
        let config = ApiConfig {
            host: "192.168.1.100".to_string(),
            port: 9090,
            ..ApiConfig::default()
        };
        let addr_str = format!("{}:{}", config.host, config.port);
        assert_eq!(addr_str, "192.168.1.100:9090");

        // Verify it parses to SocketAddr
        let addr: SocketAddr = addr_str.parse().unwrap();
        assert_eq!(addr.port(), 9090);
    }

    // ==================== AppState Tests ====================

    #[test]
    fn test_app_state_version() {
        let state = AppState {
            version: "2.0.0".to_string(),
            token: None,
            loopback_only: true,
        };
        assert_eq!(state.version, "2.0.0");
    }

    #[test]
    fn test_app_state_clone() {
        let state1 = AppState {
            version: "2.0.0".to_string(),
            token: None,
            loopback_only: true,
        };
        let state2 = state1.clone();
        assert_eq!(state1.version, state2.version);
    }

    #[test]
    fn test_app_state_in_arc() {
        let state = Arc::new(AppState {
            version: "2.0.0".to_string(),
            token: None,
            loopback_only: true,
        });
        let state_clone = Arc::clone(&state);
        assert_eq!(state.version, state_clone.version);
        assert_eq!(Arc::strong_count(&state), 2);
    }

    // ==================== Router Building Tests ====================

    #[test]
    fn test_build_router() {
        assert!(build_router(&ApiConfig::default()).is_ok());
    }

    #[test]
    fn test_socket_addr_parsing() {
        let config = ApiConfig::default();
        let addr: Result<SocketAddr, _> = config.socket_address().parse();
        assert!(addr.is_ok());
        assert_eq!(addr.unwrap().port(), 8080);
    }

    #[test]
    fn test_socket_addr_parsing_ipv6() {
        let config = ApiConfig {
            host: "::1".to_string(),
            ..ApiConfig::default()
        };
        let addr_str = config.socket_address();
        let addr: Result<SocketAddr, _> = addr_str.parse();
        assert!(addr.is_ok());
    }

    // ==================== Integration Test - Router with Tower ====================

    #[tokio::test]
    async fn test_router_health_endpoint() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
        });

        let app = Router::new()
            .route("/health", get(handlers::health))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_router_version_endpoint() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
        });

        let app = Router::new()
            .route("/version", get(handlers::version))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/version")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_router_root_endpoint() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
        });

        let app = Router::new()
            .route("/", get(handlers::root))
            .with_state(state);

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_router_validate_endpoint() {
        use axum::body::Body;
        use axum::http::{header, Method, Request, StatusCode};
        use tower::ServiceExt;

        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
        });

        let app = Router::new()
            .route("/api/v1/validate", post(handlers::validate))
            .with_state(state);

        let body = r#"{"file_path": "test-data/budget.yaml"}"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/validate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_router_calculate_endpoint() {
        use axum::body::Body;
        use axum::http::{header, Method, Request, StatusCode};
        use tower::ServiceExt;

        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
        });

        let app = Router::new()
            .route("/api/v1/calculate", post(handlers::calculate))
            .with_state(state);

        let body = r#"{"file_path": "test-data/budget.yaml", "dry_run": true}"#;

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/v1/calculate")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_router_not_found() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
        });

        let app = Router::new()
            .route("/health", get(handlers::health))
            .with_state(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
