//! Forge API Server implementation
//!
//! HTTP REST API server using Axum for enterprise integrations.
//! Provides endpoints for validate, calculate, audit, export, import.

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
pub async fn run_api_server(config: ApiConfig) -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "forge_server=info,tower_http=info".into()),
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

/// Graceful shutdown signal handler
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
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, stopping server...");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ApiConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
    }
}
