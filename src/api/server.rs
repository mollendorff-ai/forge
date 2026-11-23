//! Forge API Server implementation
//!
//! HTTP REST API server using Axum for integrations.
//! Provides endpoints for validate, calculate, audit, export, import.

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
    /// The folder file paths must stay inside (`--root`), if any
    pub root: Option<std::path::PathBuf>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            token: None,
            cors_origins: Vec::new(),
            root: None,
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
    /// or a non-loopback address without a token. Otherwise, the origins the
    /// browsers may call from, as headers.
    pub fn check(&self) -> Result<Vec<HeaderValue>, String> {
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
        self.cors_origins
            .iter()
            .map(|origin| {
                if origin == "*" {
                    return Err(
                        "--cors-origin '*' would let any website call the API; list the origins"
                            .to_string(),
                    );
                }
                HeaderValue::from_str(origin).map_err(|_| format!("Invalid CORS origin: {origin}"))
            })
            .collect()
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
    /// The folder file paths must stay inside (`--root`), if any
    pub root: Option<crate::path_root::Root>,
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
    let origins = config.check()?;
    let root = config
        .root
        .as_deref()
        .map(crate::path_root::Root::new)
        .transpose()?;
    let state = Arc::new(AppState {
        version: env!("CARGO_PKG_VERSION").to_string(),
        token: config.token.clone(),
        loopback_only: config.is_loopback(),
        root,
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
    if origins.is_empty() {
        return Ok(router);
    }
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
/// A configuration it refuses (a host other than this machine without a
/// token, any origin) returns its error before anything starts; a test runs
/// that part, and one serves on a free port until told to stop. It runs
/// until the process gets Ctrl+C or SIGTERM.
pub async fn run_api_server(config: ApiConfig) -> anyhow::Result<()> {
    serve_until(config, Box::pin(shutdown_signal())).await
}

/// What is logged when `RUST_LOG` says nothing: this crate's messages (its log
/// target is `mollendorff_forge`, not the binary's name) and each request
const DEFAULT_LOG_FILTER: &str = "mollendorff_forge=info,tower_http=info";

/// Serve until `shutdown` is ready
async fn serve_until(
    config: ApiConfig,
    shutdown: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
) -> anyhow::Result<()> {
    let app = build_router(&config)
        .map_err(anyhow::Error::msg)?
        .layer(TraceLayer::new_for_http());

    // The log starts once the configuration is accepted
    // A second server in one process finds the log started already
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| DEFAULT_LOG_FILTER.into()),
        )
        .try_init();

    let addr: SocketAddr = config.socket_address().parse()?;
    info!("🔥 Forge API Server starting on http://{}", addr);
    info!("   Endpoints: /api/v1/validate, /api/v1/calculate, /api/v1/audit, /api/v1/export, /api/v1/import");
    info!("   Health: /health, Version: /version");
    if let Some(root) = &config.root {
        let shown = root.display().to_string();
        info!("   Files: only inside {shown}");
    }
    let token = if config.token.is_some() {
        "required on /api/v1/*"
    } else {
        "none (local use only)"
    };
    let origins = if config.cors_origins.is_empty() {
        "none".to_string()
    } else {
        config.cors_origins.join(", ")
    };
    info!("   Token: {token}. Browser origins allowed: {origins}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .expect("axum's server does not fail once it is serving");

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
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("Shutdown signal received, stopping server...");
}

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
            root: None,
        };
        assert_eq!(state.version, "2.0.0");
    }

    #[test]
    fn test_app_state_clone() {
        let state1 = AppState {
            version: "2.0.0".to_string(),
            token: None,
            loopback_only: true,
            root: None,
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
            root: None,
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
            root: None,
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
            root: None,
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
            root: None,
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
            root: None,
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
            root: None,
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
            root: None,
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

    // ─────────────────────────────────────────────────────────────────────────
    // examples/bagel through the five endpoints (T1-INTERFACE-PARITY), called in
    // process through `build_router`: the numbers are the ones the command-line
    // tests assert (cli/commands/examples.rs, `bagel_example`, where the Python
    // is quoted). validate, calculate and audit take the model as text with
    // `costs.yaml` in `includes`; export and import name files.
    // ─────────────────────────────────────────────────────────────────────────

    mod bagel_over_http {
        use super::*;
        use axum::body::Body;
        use axum::http::{header, Method, Request, StatusCode};
        use serde_json::{json, Value};
        use tower::ServiceExt;

        const PLAN: &str = "examples/bagel/bagel.yaml";

        fn text(file: &str) -> String {
            std::fs::read_to_string(format!("examples/bagel/{file}")).unwrap()
        }

        /// The plan as text, and `costs.yaml` for its `_includes`
        fn inline() -> Value {
            json!({"content": text("bagel.yaml"), "includes": {"costs": text("costs.yaml")}})
        }

        /// POST `body` to `path` on the router `forge serve` builds, as a request
        /// from this machine; the `data` of the answer
        async fn post(path: &str, body: &Value) -> Value {
            let app = build_router(&ApiConfig::default()).unwrap();
            let response = app
                .oneshot(
                    Request::builder()
                        .method(Method::POST)
                        .uri(path)
                        .header(header::HOST, "localhost:8080")
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let answer: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(answer["success"], true);
            answer["data"].clone()
        }

        /// A dependency of the audited variable, wherever it is in the tree
        fn find<'a>(deps: &'a Value, name: &str) -> Option<&'a Value> {
            deps.as_array().into_iter().flatten().find_map(|d| {
                (d["name"] == name)
                    .then_some(d)
                    .or_else(|| find(&d["children"], name))
            })
        }

        fn close(a: &Value, b: f64, tolerance: f64) -> bool {
            (a.as_f64().unwrap() - b).abs() <= tolerance
        }

        #[tokio::test]
        async fn validate_counts_the_bagel_model() {
            let data = post("/api/v1/validate", &inline()).await;
            assert_eq!(data["valid"], true);
            assert_eq!(data["message"], "Validation successful");
            assert_eq!(data["file_path"], "");
            assert_eq!(data["result"]["table_count"], 1);
            assert_eq!(data["result"]["scalar_count"], 19);
            assert_eq!(data["result"]["mismatches"], json!([]));
        }

        #[tokio::test]
        async fn calculate_gives_the_years_profit_and_the_flour_shocks() {
            let mut body = inline();
            body["dry_run"] = json!(true);
            let data = post("/api/v1/calculate", &body).await;
            assert_eq!(data["calculated"], true);
            let scalars = &data["result"]["scalars"];
            assert!(close(&scalars["year.profit"], 786.320_361_7, 1e-6));
            assert!(close(&scalars["year.revenue"], 386_827.449_537_8, 1e-6));
            assert!(close(
                &scalars["year.first_year_return"],
                786.320_361_7 / 95_000.0,
                1e-9
            ));

            // The scenario sets the flour price, a value in the included file
            body["scenario"] = json!("flour_shock");
            let shock = post("/api/v1/calculate", &body).await;
            assert!(close(
                &shock["result"]["scalars"]["year.profit"],
                -16_943.271_075,
                1e-6
            ));
        }

        #[tokio::test]
        async fn calculate_without_a_dry_run_returns_the_model_with_its_values() {
            let data = post("/api/v1/calculate", &inline()).await;
            assert_eq!(data["message"], "Calculation completed");
            let model = data["calculated_content"].as_str().unwrap();
            assert!(model.contains("=SUM(months.profit)"), "{model}");
            assert!(model.contains("value: 786.3203617"), "{model}");
        }

        #[tokio::test]
        async fn audit_follows_the_profit_down_to_the_included_costs() {
            let mut body = inline();
            body["variable"] = json!("year.profit");
            let data = post("/api/v1/audit", &body).await;
            assert_eq!(data["audited"], true);
            assert_eq!(data["variable"], "year.profit");
            let result = &data["result"];
            assert!(close(&result["calculated_value"], 786.320_361_7, 1e-6));
            let deps = &result["dependencies"];
            assert_eq!(deps[0]["name"], "months.profit");
            assert_eq!(deps[0]["values"].as_array().map(Vec::len), Some(12));
            let wage = find(deps, "@costs.labour.loaded_wage").unwrap();
            assert_eq!(wage["dep_type"], "Included scalar");
            assert_eq!(wage["source"], "costs.yaml");
            assert!(close(&wage["value"], 21.275, 1e-9));
        }

        #[tokio::test]
        async fn export_writes_the_bagel_workbook() {
            let dir = tempfile::tempdir().unwrap();
            let excel = dir.path().join("bagel.xlsx");
            let data = post(
                "/api/v1/export",
                &json!({"yaml_path": PLAN, "excel_path": excel.to_str().unwrap()}),
            )
            .await;
            assert_eq!(data["exported"], true);
            assert_eq!(data["message"], "Export completed");
            let bytes = std::fs::read(&excel).unwrap();
            assert_eq!(&bytes[..4], b"PK\x03\x04", "an xlsx is a zip file");
        }

        /// A workbook of the plan, imported over HTTP, with `costs.yaml` beside the
        /// model: the model that comes back calculates the same year.
        #[tokio::test]
        async fn import_gives_the_bagel_year_back() {
            let dir = tempfile::tempdir().unwrap();
            let excel = dir.path().join("bagel.xlsx");
            crate::cli::export_core(std::path::Path::new(PLAN), &excel).unwrap();
            let yaml = dir.path().join("model.yaml");
            let data = post(
                "/api/v1/import",
                &json!({"excel_path": excel.to_str().unwrap(), "yaml_path": yaml.to_str().unwrap()}),
            )
            .await;
            assert_eq!(data["imported"], true);
            assert_eq!(data["message"], "Import completed");
            assert!(dir.path().join("costs.yaml").exists());

            let back = post(
                "/api/v1/calculate",
                &json!({"file_path": yaml.to_str().unwrap(), "dry_run": true}),
            )
            .await;
            assert!(close(
                &back["result"]["scalars"]["year.profit"],
                786.320_361_7,
                1e-6
            ));
        }

        /// examples/inherited-budget over HTTP: the workbook Forge did not write
        /// calculates to the net income `LibreOffice` stored in it.
        #[tokio::test]
        async fn import_reads_the_inherited_workbook() {
            let dir = tempfile::tempdir().unwrap();
            let yaml = dir.path().join("budget.yaml");
            let data = post(
                "/api/v1/import",
                &json!({"excel_path": "examples/inherited-budget/budget.xlsx", "yaml_path": yaml.to_str().unwrap()}),
            )
            .await;
            assert_eq!(data["imported"], true);
            let back = post(
                "/api/v1/calculate",
                &json!({"file_path": yaml.to_str().unwrap(), "dry_run": true}),
            )
            .await;
            assert!(close(
                &back["result"]["scalars"]["summary.net_income"],
                656_435.464_438_365,
                1e-9
            ));
        }
    }

    // ==================== Serving and stopping ====================

    /// A port nothing listens on now
    fn free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[tokio::test]
    async fn the_server_answers_until_told_to_stop() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let port = free_port();
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let dir = tempfile::tempdir().unwrap();
        let config = ApiConfig {
            port,
            token: Some("secret".to_string()),
            cors_origins: vec!["https://app.example".to_string()],
            root: Some(dir.path().to_path_buf()),
            ..ApiConfig::default()
        };
        let server = tokio::spawn(serve_until(
            config,
            Box::pin(async {
                let _ = stopped.await;
            }),
        ));

        let mut reply = String::new();
        for _ in 0..100 {
            if let Ok(mut stream) = tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                stream
                    .write_all(
                        format!(
                            "GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
                stream.read_to_string(&mut reply).await.unwrap();
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(reply.starts_with("HTTP/1.1 200"), "{reply}");

        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn the_server_without_a_token_or_a_root_stops_when_told_to() {
        // With its log off: each line of it is then not written, whatever the
        // other tests have set up for the process
        let _off = tracing::subscriber::set_default(tracing::subscriber::NoSubscriber::default());
        let config = ApiConfig {
            port: free_port(),
            ..ApiConfig::default()
        };
        serve_until(config, Box::pin(std::future::ready(())))
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn the_server_reports_an_address_it_cannot_use() {
        let bad_host = ApiConfig {
            host: "not an address".to_string(),
            token: Some("secret".to_string()),
            ..ApiConfig::default()
        };
        let err = serve_until(bad_host, Box::pin(std::future::ready(())))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("invalid socket address"), "{err}");

        let taken = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let busy = ApiConfig {
            port: taken.local_addr().unwrap().port(),
            ..ApiConfig::default()
        };
        let err = serve_until(busy, Box::pin(std::future::ready(())))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("in use"), "{err}");
    }

    /// Send this process `signal` once `shutdown_signal` is waiting for it
    async fn signal_the_shutdown(signal: &str) {
        let mut waiting = Box::pin(shutdown_signal());
        // Polling it installs the handlers; it is not ready, so it times out
        let polled =
            tokio::time::timeout(std::time::Duration::from_millis(200), &mut waiting).await;
        assert!(polled.is_err(), "it stopped with no signal");
        let sent = std::process::Command::new("kill")
            .arg(signal)
            .arg(std::process::id().to_string())
            .status()
            .unwrap();
        assert!(sent.success());
        tokio::time::timeout(std::time::Duration::from_secs(10), waiting)
            .await
            .expect("the signal ends the wait");
    }

    // One test: a signal goes to the whole process, so two waiting at once
    // would both be ended by the first
    #[cfg(unix)]
    #[tokio::test]
    async fn ctrl_c_and_sigterm_stop_the_server() {
        // One signal with the log on and one with it off: the line it logs
        // is written, or is not, whatever the other tests have set up
        let on = tracing_subscriber::fmt()
            .with_writer(std::io::sink)
            .finish();
        {
            let _on = tracing::subscriber::set_default(on);
            signal_the_shutdown("-INT").await;
        }
        let _off = tracing::subscriber::set_default(tracing::subscriber::NoSubscriber::default());
        signal_the_shutdown("-TERM").await;
    }

    /// A log a test can read back
    #[derive(Clone, Default)]
    struct Log(Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for Log {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            std::io::Write::write(&mut *self.0.lock().unwrap(), bytes)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            std::io::Write::flush(&mut *self.0.lock().unwrap())
        }
    }

    /// What `serve_until` logs for `config` under the log the server starts
    /// with when `RUST_LOG` says nothing
    async fn logged(config: ApiConfig) -> String {
        let log = Log::default();
        let writer = log.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new(DEFAULT_LOG_FILTER))
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        let _installed = tracing::subscriber::set_default(subscriber);
        let port = config.port;
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let server = tokio::spawn(serve_until(
            config,
            Box::pin(async {
                let _ = stopped.await;
            }),
        ));
        for _ in 0..200 {
            if tokio::net::TcpStream::connect(("127.0.0.1", port))
                .await
                .is_ok()
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        // The log is flushed to what the test reads
        std::io::Write::flush(&mut log.clone()).unwrap();
        let text = String::from_utf8(log.0.lock().unwrap().clone()).unwrap();
        text
    }

    #[tokio::test]
    async fn the_default_log_says_where_the_server_is_and_that_it_has_no_token() {
        let port = free_port();
        let text = logged(ApiConfig {
            port,
            ..ApiConfig::default()
        })
        .await;
        assert!(
            text.contains(&format!(
                "Forge API Server starting on http://127.0.0.1:{port}"
            )),
            "{text}"
        );
        assert!(
            text.contains("Token: none (local use only). Browser origins allowed: none"),
            "{text}"
        );
        assert!(
            text.contains("Forge API Server shutdown complete"),
            "{text}"
        );
    }

    #[tokio::test]
    async fn the_log_names_the_root_the_token_and_the_origins() {
        let root = tempfile::tempdir().unwrap();
        let text = logged(ApiConfig {
            port: free_port(),
            token: Some("s3cret".to_string()),
            cors_origins: vec![
                "http://localhost:3000".to_string(),
                "http://localhost:4000".to_string(),
            ],
            root: Some(root.path().to_path_buf()),
            ..ApiConfig::default()
        })
        .await;
        assert!(
            text.contains(&format!("Files: only inside {}", root.path().display())),
            "{text}"
        );
        assert!(
            text.contains(
                "Token: required on /api/v1/*. Browser origins allowed: http://localhost:3000, http://localhost:4000"
            ),
            "{text}"
        );
        assert!(
            !text.contains("s3cret"),
            "the token is never logged: {text}"
        );
    }
}
