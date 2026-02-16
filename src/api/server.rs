//! Forge API Server implementation
//!
//! HTTP REST API server using Axum for enterprise integrations.
//! Provides endpoints for validate, calculate, audit, export, import.

// During coverage builds, stubbed functions don't use all imports
#![cfg_attr(coverage, allow(unused_imports, dead_code))]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use super::handlers;

/// API Server configuration
#[derive(Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub version: String,
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

    let state = Arc::new(AppState {
        version: env!("CARGO_PKG_VERSION").to_string(),
    });

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Build router
    let app = Router::new()
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
        // State and middleware
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    info!("🔥 Forge API Server starting on http://{}", addr);
    info!("   Endpoints: /api/v1/validate, /api/v1/calculate, /api/v1/audit, /api/v1/export, /api/v1/import");
    info!("   Health: /health, Version: /version");

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
        };
        assert_eq!(state.version, "2.0.0");
    }

    #[test]
    fn test_app_state_clone() {
        let state1 = AppState {
            version: "2.0.0".to_string(),
        };
        let state2 = state1.clone();
        assert_eq!(state1.version, state2.version);
    }

    #[test]
    fn test_app_state_in_arc() {
        let state = Arc::new(AppState {
            version: "2.0.0".to_string(),
        });
        let state_clone = Arc::clone(&state);
        assert_eq!(state.version, state_clone.version);
        assert_eq!(Arc::strong_count(&state), 2);
    }

    // ==================== Router Building Tests ====================

    #[test]
    fn test_build_router() {
        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
        });

        // Build the router with explicit type
        let _app: Router = Router::new()
            .route("/", get(handlers::root))
            .route("/health", get(handlers::health))
            .route("/version", get(handlers::version))
            .route("/api/v1/validate", post(handlers::validate))
            .route("/api/v1/calculate", post(handlers::calculate))
            .route("/api/v1/audit", post(handlers::audit))
            .route("/api/v1/export", post(handlers::export))
            .route("/api/v1/import", post(handlers::import_excel))
            .with_state(state);

        // If we get here, router was built successfully
    }

    #[test]
    fn test_socket_addr_parsing() {
        let config = ApiConfig::default();
        let addr_str = format!("{}:{}", config.host, config.port);
        let addr: Result<SocketAddr, _> = addr_str.parse();
        assert!(addr.is_ok());
        assert_eq!(addr.unwrap().port(), 8080);
    }

    #[test]
    fn test_socket_addr_parsing_ipv6() {
        let config = ApiConfig {
            host: "::1".to_string(),
            port: 8080,
        };
        let addr_str = format!("[{}]:{}", config.host, config.port);
        let addr: Result<SocketAddr, _> = addr_str.parse();
        assert!(addr.is_ok());
    }

    #[test]
    fn test_cors_layer_creation() {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);
        // If we get here, CORS layer was created successfully
        let _ = cors;
    }

    // ==================== Integration Test - Router with Tower ====================

    #[tokio::test]
    async fn test_router_health_endpoint() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
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
