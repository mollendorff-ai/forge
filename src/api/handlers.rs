//! API request handlers
//!
//! Handlers for all REST API endpoints.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cli::commands::results::{AuditResult, CalculationResult, ValidationResult};
use crate::cli::{
    audit_core, calculate_core, calculate_plan_core, export_core, import_core, validate_core,
};
use crate::error::ForgeError;
use crate::inline_model::resolve_model_input;

use super::server::AppState;

/// Standard API response wrapper. Every answer is `ok`: a model that fails
/// says so in `data` (`valid: false`, `exported: false`, ...), as MCP's tools
/// do (T1-INTERFACE-PARITY).
#[derive(Serialize)]
#[serde(bound = "T: Serialize")]
pub struct ApiResponse<T> {
    pub success: bool,
    pub request_id: String,
    /// Numbers to 15 significant digits (`core::precision`).
    #[serde(serialize_with = "crate::core::precision::serialize_rounded")]
    pub data: T,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            request_id: Uuid::new_v4().to_string(),
            data,
        }
    }
}

/// Root endpoint response
#[derive(Serialize)]
pub struct RootResponse {
    pub name: String,
    pub version: String,
    pub description: String,
    pub endpoints: Vec<EndpointInfo>,
}

#[derive(Serialize)]
pub struct EndpointInfo {
    pub path: String,
    pub method: String,
    pub description: String,
}

/// GET / - Root info
pub async fn root(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let response = RootResponse {
        name: "Forge API Server".to_string(),
        version: state.version.clone(),
        description: "HTTP API for YAML formula calculations".to_string(),
        endpoints: vec![
            EndpointInfo {
                path: "/health".to_string(),
                method: "GET".to_string(),
                description: "Health check endpoint".to_string(),
            },
            EndpointInfo {
                path: "/version".to_string(),
                method: "GET".to_string(),
                description: "Get server version".to_string(),
            },
            EndpointInfo {
                path: "/api/v1/validate".to_string(),
                method: "POST".to_string(),
                description: "Validate a YAML model file".to_string(),
            },
            EndpointInfo {
                path: "/api/v1/calculate".to_string(),
                method: "POST".to_string(),
                description: "Calculate formulas in a YAML model".to_string(),
            },
            EndpointInfo {
                path: "/api/v1/audit".to_string(),
                method: "POST".to_string(),
                description: "Audit a variable's dependency tree".to_string(),
            },
            EndpointInfo {
                path: "/api/v1/export".to_string(),
                method: "POST".to_string(),
                description: "Export YAML to Excel".to_string(),
            },
            EndpointInfo {
                path: "/api/v1/import".to_string(),
                method: "POST".to_string(),
                description: "Import Excel to YAML".to_string(),
            },
        ],
    };
    Json(ApiResponse::ok(response))
}

/// Health check response
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_message: String,
}

/// GET /health - Health check
pub async fn health() -> impl IntoResponse {
    Json(ApiResponse::ok(HealthResponse {
        status: "healthy".to_string(),
        uptime_message: "Server is running".to_string(),
    }))
}

/// Version response
#[derive(Serialize)]
pub struct VersionResponse {
    pub version: String,
    pub features: Vec<String>,
}

/// GET /version - Server version
pub async fn version(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(ApiResponse::ok(VersionResponse {
        version: state.version.clone(),
        features: vec![
            "validate".to_string(),
            "calculate".to_string(),
            "audit".to_string(),
            "export".to_string(),
            "import".to_string(),
        ],
    }))
}

/// Where a request gets its model from: a file on the machine running Forge,
/// or the YAML itself (with the text of any included files).
#[derive(Deserialize, Default)]
pub struct ModelSource {
    /// Path to a model on the machine running Forge
    pub file_path: Option<String>,
    /// The model as YAML text, instead of `file_path`
    pub content: Option<String>,
    /// Text for each `_includes` entry of inline content, keyed by its `as` name
    pub includes: Option<HashMap<String, String>>,
}

impl ModelSource {
    fn resolve(&self, state: &AppState) -> Result<(PathBuf, Option<tempfile::TempDir>), String> {
        resolve_model_input(
            state.root.as_ref(),
            self.file_path.as_deref(),
            self.content.as_deref(),
            self.includes.as_ref(),
        )
    }

    /// The path to show in the response: empty for inline content.
    fn label(&self) -> String {
        self.file_path.clone().unwrap_or_default()
    }
}

/// Validate request
#[derive(Deserialize)]
pub struct ValidateRequest {
    #[serde(flatten)]
    pub source: ModelSource,
}

/// Validate response
#[derive(Serialize, Default)]
pub struct ValidateResponse {
    pub valid: bool,
    pub file_path: String,
    pub message: String,
    /// Counts and any value that differs from its formula
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ValidationResult>,
}

/// POST /api/v1/validate - Validate a YAML model
pub async fn validate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ValidateRequest>,
) -> impl IntoResponse {
    let file_path = req.source.label();
    let outcome = req
        .source
        .resolve(&state)
        .and_then(|(path, _tmp)| validate_core(&path).map_err(|e| e.to_string()));

    Json(ApiResponse::ok(validate_response(file_path, outcome)))
}

/// What `POST /api/v1/validate` answers: valid when every stored value
/// matches its formula and every expectation is met (ADR-074).
fn validate_response(
    file_path: String,
    outcome: Result<ValidationResult, String>,
) -> ValidateResponse {
    match outcome {
        Ok(result) => {
            let message = if !result.scalars_valid {
                "Some values differ from their formulas".to_string()
            } else if result.expectations_met {
                "Validation successful".to_string()
            } else {
                format!(
                    "{} of {} expected results differ",
                    result.expectation_differences.len(),
                    result.expectations_checked
                )
            };
            ValidateResponse {
                valid: result.tables_valid && result.scalars_valid && result.expectations_met,
                file_path,
                message,
                result: Some(result),
            }
        },
        Err(e) => ValidateResponse {
            valid: false,
            file_path,
            message: e,
            result: None,
        },
    }
}

/// Calculate request
#[derive(Deserialize)]
pub struct CalculateRequest {
    #[serde(flatten)]
    pub source: ModelSource,
    #[serde(default)]
    pub dry_run: bool,
    /// Scenario to apply
    pub scenario: Option<String>,
    /// Each uncertain input at its plan figure (ADR-076); never writes
    #[serde(default)]
    pub plan: bool,
}

/// Calculate response
#[derive(Serialize, Default)]
pub struct CalculateResponse {
    pub calculated: bool,
    pub file_path: String,
    pub dry_run: bool,
    pub message: String,
    /// The calculated tables and scalars
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<CalculationResult>,
    /// For inline content: the model with its calculated values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calculated_content: Option<String>,
}

/// POST /api/v1/calculate - Calculate formulas
pub async fn calculate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CalculateRequest>,
) -> impl IntoResponse {
    let file_path = req.source.label();
    let dry_run = req.dry_run || req.plan;
    let is_inline = req.source.content.is_some();
    let outcome = req.source.resolve(&state).and_then(|(path, _tmp)| {
        let result = if req.plan {
            calculate_plan_core(&path, req.scenario.as_deref())
        } else {
            calculate_core(&path, dry_run, req.scenario.as_deref())
        }
        .map_err(|e| format!("Error: {e}"))?;
        let content = if is_inline && !dry_run {
            std::fs::read_to_string(&path).ok()
        } else {
            None
        };
        Ok((result, content))
    });

    Json(ApiResponse::ok(match outcome {
        Ok((result, calculated_content)) => CalculateResponse {
            calculated: true,
            file_path,
            dry_run,
            message: if dry_run {
                "Dry run completed".to_string()
            } else if is_inline {
                "Calculation completed".to_string()
            } else {
                "Calculation completed and file updated".to_string()
            },
            result: Some(result),
            calculated_content,
        },
        Err(e) => CalculateResponse {
            calculated: false,
            file_path,
            dry_run,
            message: e,
            result: None,
            calculated_content: None,
        },
    }))
}

/// Audit request
#[derive(Deserialize)]
pub struct AuditRequest {
    #[serde(flatten)]
    pub source: ModelSource,
    pub variable: String,
}

/// Audit response
#[derive(Serialize, Default)]
pub struct AuditResponse {
    pub audited: bool,
    pub file_path: String,
    pub variable: String,
    pub message: String,
    /// The dependency tree and calculated value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<AuditResult>,
}

/// POST /api/v1/audit - Audit a variable
pub async fn audit(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AuditRequest>,
) -> impl IntoResponse {
    let file_path = req.source.label();
    let variable = req.variable.clone();
    let outcome = req
        .source
        .resolve(&state)
        .and_then(|(path, _tmp)| audit_core(&path, &variable).map_err(|e| format!("Error: {e}")));

    Json(ApiResponse::ok(match outcome {
        Ok(result) => AuditResponse {
            audited: true,
            file_path,
            variable,
            message: "Audit completed".to_string(),
            result: Some(result),
        },
        Err(e) => AuditResponse {
            audited: false,
            file_path,
            variable,
            message: e,
            result: None,
        },
    }))
}

/// Export request
#[derive(Deserialize)]
pub struct ExportRequest {
    pub yaml_path: String,
    pub excel_path: String,
}

/// Export response
#[derive(Serialize, Default)]
pub struct ExportResponse {
    pub exported: bool,
    pub yaml_path: String,
    pub excel_path: String,
    pub message: String,
}

/// POST /api/v1/export - Export YAML to Excel
pub async fn export(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ExportRequest>,
) -> impl IntoResponse {
    let outcome = state.root.as_ref().map_or_else(
        || {
            Ok((
                PathBuf::from(&req.yaml_path),
                PathBuf::from(&req.excel_path),
            ))
        },
        |root| Ok((root.model(&req.yaml_path)?, root.file(&req.excel_path)?)),
    );
    let outcome = outcome
        .map_err(ForgeError::Validation)
        .and_then(|(yaml_path, excel_path)| export_core(&yaml_path, &excel_path).map(drop));
    match outcome {
        Ok(()) => Json(ApiResponse::ok(ExportResponse {
            exported: true,
            yaml_path: req.yaml_path,
            excel_path: req.excel_path,
            message: "Export completed".to_string(),
        })),
        Err(e) => Json(ApiResponse::ok(ExportResponse {
            exported: false,
            yaml_path: req.yaml_path,
            excel_path: req.excel_path,
            message: format!("Error: {e}"),
        })),
    }
}

/// Import request
#[derive(Deserialize)]
pub struct ImportRequest {
    pub excel_path: String,
    pub yaml_path: String,
}

/// Import response
#[derive(Serialize, Default)]
pub struct ImportResponse {
    pub imported: bool,
    pub excel_path: String,
    pub yaml_path: String,
    pub message: String,
}

/// POST /api/v1/import - Import Excel to YAML
pub async fn import_excel(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ImportRequest>,
) -> impl IntoResponse {
    let outcome = state.root.as_ref().map_or_else(
        || {
            Ok((
                PathBuf::from(&req.excel_path),
                PathBuf::from(&req.yaml_path),
            ))
        },
        |root| Ok((root.file(&req.excel_path)?, root.file(&req.yaml_path)?)),
    );
    let outcome = outcome
        .map_err(ForgeError::Validation)
        .and_then(|(excel_path, yaml_path)| {
            import_core(&excel_path, &yaml_path, false, false).map(drop)
        });
    match outcome {
        Ok(()) => Json(ApiResponse::ok(ImportResponse {
            imported: true,
            excel_path: req.excel_path,
            yaml_path: req.yaml_path,
            message: "Import completed".to_string(),
        })),
        Err(e) => Json(ApiResponse::ok(ImportResponse {
            imported: false,
            excel_path: req.excel_path,
            yaml_path: req.yaml_path,
            message: format!("Error: {e}"),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ApiResponse Tests ====================

    #[test]
    fn test_api_response_ok_creates_success_response() {
        let response: ApiResponse<String> = ApiResponse::ok("test data".to_string());

        assert!(response.success);
        assert_eq!(response.data, "test data");
        assert_ne!(response.request_id, "");
        // Verify UUID format (8-4-4-4-12)
        assert_eq!(response.request_id.len(), 36);
    }

    #[test]
    fn test_api_response_ok_with_struct() {
        let health = HealthResponse {
            status: "healthy".to_string(),
            uptime_message: "running".to_string(),
        };
        let response = ApiResponse::ok(health);

        assert!(response.success);
        let data = response.data;
        assert_eq!(data.status, "healthy");
        assert_eq!(data.uptime_message, "running");
    }

    #[test]
    fn test_api_response_request_id_is_unique() {
        let response1: ApiResponse<String> = ApiResponse::ok("test1".to_string());
        let response2: ApiResponse<String> = ApiResponse::ok("test2".to_string());

        assert_ne!(response1.request_id, response2.request_id);
    }

    // ==================== Response Struct Default Tests ====================

    #[test]
    fn test_validate_response_default() {
        let response = ValidateResponse::default();

        assert!(!response.valid);
        assert_eq!(response.file_path, "");
        assert_eq!(response.message, "");
    }

    #[test]
    fn test_calculate_response_default() {
        let response = CalculateResponse::default();

        assert!(!response.calculated);
        assert!(!response.dry_run);
        assert_eq!(response.file_path, "");
        assert_eq!(response.message, "");
    }

    #[test]
    fn test_audit_response_default() {
        let response = AuditResponse::default();

        assert!(!response.audited);
        assert_eq!(response.file_path, "");
        assert_eq!(response.variable, "");
        assert_eq!(response.message, "");
    }

    #[test]
    fn test_export_response_default() {
        let response = ExportResponse::default();

        assert!(!response.exported);
        assert_eq!(response.yaml_path, "");
        assert_eq!(response.excel_path, "");
        assert_eq!(response.message, "");
    }

    #[test]
    fn test_import_response_default() {
        let response = ImportResponse::default();

        assert!(!response.imported);
        assert_eq!(response.excel_path, "");
        assert_eq!(response.yaml_path, "");
        assert_eq!(response.message, "");
    }

    // ==================== Request Deserialization Tests ====================

    #[test]
    fn test_validate_request_deserialize() {
        let json = r#"{"file_path": "model.yaml"}"#;
        let req: ValidateRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.source.file_path.as_deref(), Some("model.yaml"));
    }

    #[test]
    fn test_calculate_request_deserialize_with_dry_run() {
        let json = r#"{"file_path": "model.yaml", "dry_run": true}"#;
        let req: CalculateRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.source.file_path.as_deref(), Some("model.yaml"));
        assert!(req.dry_run);
    }

    #[test]
    fn test_calculate_request_deserialize_dry_run_defaults_false() {
        let json = r#"{"file_path": "model.yaml"}"#;
        let req: CalculateRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.source.file_path.as_deref(), Some("model.yaml"));
        assert!(!req.dry_run);
    }

    #[test]
    fn test_audit_request_deserialize() {
        let json = r#"{"file_path": "model.yaml", "variable": "total_revenue"}"#;
        let req: AuditRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.source.file_path.as_deref(), Some("model.yaml"));
        assert_eq!(req.variable, "total_revenue");
    }

    #[test]
    fn test_export_request_deserialize() {
        let json = r#"{"yaml_path": "model.yaml", "excel_path": "output.xlsx"}"#;
        let req: ExportRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.yaml_path, "model.yaml");
        assert_eq!(req.excel_path, "output.xlsx");
    }

    #[test]
    fn test_import_request_deserialize() {
        let json = r#"{"excel_path": "input.xlsx", "yaml_path": "output.yaml"}"#;
        let req: ImportRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.excel_path, "input.xlsx");
        assert_eq!(req.yaml_path, "output.yaml");
    }

    // ==================== Response Serialization Tests ====================

    #[test]
    fn test_health_response_serialize() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            uptime_message: "Server is running".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"status\":\"healthy\""));
        assert!(json.contains("\"uptime_message\":\"Server is running\""));
    }

    #[test]
    fn test_version_response_serialize() {
        let response = VersionResponse {
            version: "2.0.0".to_string(),
            features: vec!["validate".to_string(), "calculate".to_string()],
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"version\":\"2.0.0\""));
        assert!(json.contains("\"features\":[\"validate\",\"calculate\"]"));
    }

    #[test]
    fn test_validate_response_serialize() {
        let response = ValidateResponse {
            valid: true,
            file_path: "model.yaml".to_string(),
            message: "Validation successful".to_string(),
            result: None,
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("\"file_path\":\"model.yaml\""));
        assert!(json.contains("\"message\":\"Validation successful\""));
    }

    #[test]
    fn test_calculate_response_serialize() {
        let response = CalculateResponse {
            calculated: true,
            file_path: "model.yaml".to_string(),
            dry_run: false,
            message: "Calculation completed".to_string(),
            result: None,
            calculated_content: None,
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"calculated\":true"));
        assert!(json.contains("\"dry_run\":false"));
    }

    #[test]
    fn an_answer_is_success_a_request_id_and_data_only() {
        let response = ApiResponse::ok(0.1_f64 + 0.2);
        let json: serde_json::Value = serde_json::to_value(&response).unwrap();
        let keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, ["data", "request_id", "success"]);
        assert_eq!(json["success"], true);
        // Rounded to 15 significant digits
        assert_eq!(json["data"], 0.3);
    }

    // ==================== EndpointInfo Tests ====================

    #[test]
    fn test_endpoint_info_serialize() {
        let info = EndpointInfo {
            path: "/api/v1/validate".to_string(),
            method: "POST".to_string(),
            description: "Validate a YAML model".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();

        assert!(json.contains("\"path\":\"/api/v1/validate\""));
        assert!(json.contains("\"method\":\"POST\""));
        assert!(json.contains("\"description\":\"Validate a YAML model\""));
    }

    #[test]
    fn test_root_response_has_all_endpoints() {
        let response = RootResponse {
            name: "Forge API Server".to_string(),
            version: "2.0.0".to_string(),
            description: "HTTP API".to_string(),
            endpoints: vec![
                EndpointInfo {
                    path: "/health".to_string(),
                    method: "GET".to_string(),
                    description: "Health check".to_string(),
                },
                EndpointInfo {
                    path: "/api/v1/validate".to_string(),
                    method: "POST".to_string(),
                    description: "Validate".to_string(),
                },
            ],
        };

        assert_eq!(response.endpoints.len(), 2);
        assert_eq!(response.endpoints[0].path, "/health");
        assert_eq!(response.endpoints[1].path, "/api/v1/validate");
    }

    // ==================== Async Handler Tests ====================

    #[tokio::test]
    async fn test_health_handler() {
        use axum::response::IntoResponse;

        let response = health().await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    /// The state of a server without `--root`
    fn no_root() -> State<Arc<AppState>> {
        State(Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
            root: None,
        }))
    }

    #[tokio::test]
    async fn test_version_handler() {
        use axum::response::IntoResponse;

        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
            root: None,
        });

        let response = version(State(state)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_root_handler() {
        use axum::response::IntoResponse;

        let state = Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
            root: None,
        });

        let response = root(State(state)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_validate_handler_nonexistent_file() {
        use axum::response::IntoResponse;

        let req = ValidateRequest {
            source: ModelSource {
                file_path: Some("/nonexistent/file.yaml".to_string()),
                ..Default::default()
            },
        };

        let response = validate(no_root(), Json(req)).await;
        let response = response.into_response();

        // Should return 200 with error in body (API convention)
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[test]
    fn a_validate_response_counts_the_expectations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        let text = "_forge_version: \"5.6.0\"\nw: 0.6\nout:\n  twice: {formula: \"=w * 2\"}\n\
                    scenarios:\n  short:\n    scalars: {w: 0.4}\n    expect: {out.twice: 0.8}\n";
        let answer = |text: &str| {
            std::fs::write(&path, text).unwrap();
            validate_response(
                String::new(),
                validate_core(&path).map_err(|e| e.to_string()),
            )
        };
        let met = answer(text);
        assert!(met.valid);
        assert_eq!(met.message, "Validation successful");
        let differ = answer(&text.replace("0.8}", "0.9}"));
        assert!(!differ.valid);
        assert_eq!(differ.message, "1 of 1 expected results differ");
        let d = &differ.result.unwrap().expectation_differences[0];
        assert_eq!((d.scenario.as_deref(), d.got), (Some("short"), Some(0.8)));
        let stale = answer(&text.replace("=w * 2\"}", "=w * 2\", value: 3}"));
        assert!(!stale.valid);
        assert_eq!(stale.message, "Some values differ from their formulas");
    }

    #[tokio::test]
    async fn test_validate_handler_valid_file() {
        use axum::response::IntoResponse;

        let req = ValidateRequest {
            source: ModelSource {
                file_path: Some("test-data/budget.yaml".to_string()),
                ..Default::default()
            },
        };

        let response = validate(no_root(), Json(req)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_calculate_handler_dry_run() {
        use axum::response::IntoResponse;

        let req = CalculateRequest {
            source: ModelSource {
                file_path: Some("test-data/budget.yaml".to_string()),
                ..Default::default()
            },
            dry_run: true,
            scenario: None,
            plan: false,
        };

        let response = calculate(no_root(), Json(req)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    /// `plan`: the simulation's model at its plan figures, written nowhere
    #[tokio::test]
    async fn the_calculate_handler_calculates_an_uncertain_model_at_plan() {
        use axum::response::IntoResponse;
        let request = |plan: bool| CalculateRequest {
            source: ModelSource {
                file_path: Some("examples/bagel/bagel_simulate.yaml".to_string()),
                ..Default::default()
            },
            dry_run: false,
            scenario: None,
            plan,
        };
        let body = |response: axum::response::Response| async move {
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()
        };
        let at_plan = body(
            calculate(no_root(), Json(request(true)))
                .await
                .into_response(),
        )
        .await;
        let profit = at_plan["data"]["result"]["scalars"]["year.profit"]
            .as_f64()
            .unwrap();
        assert!((profit - 786.320_361_7).abs() < 1e-6, "{at_plan}");
        assert_eq!(at_plan["data"]["dry_run"], true, "{at_plan}");
        let refused = body(
            calculate(no_root(), Json(request(false)))
                .await
                .into_response(),
        )
        .await;
        assert!(refused.to_string().contains("--plan"), "{refused}");
    }

    #[tokio::test]
    async fn test_calculate_handler_nonexistent() {
        use axum::response::IntoResponse;

        let req = CalculateRequest {
            source: ModelSource {
                file_path: Some("/nonexistent/file.yaml".to_string()),
                ..Default::default()
            },
            dry_run: true,
            scenario: None,
            plan: false,
        };

        let response = calculate(no_root(), Json(req)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_audit_handler() {
        use axum::response::IntoResponse;

        let req = AuditRequest {
            source: ModelSource {
                file_path: Some("test-data/budget.yaml".to_string()),
                ..Default::default()
            },
            variable: "profit".to_string(),
        };

        let response = audit(no_root(), Json(req)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_audit_handler_nonexistent() {
        use axum::response::IntoResponse;

        let req = AuditRequest {
            source: ModelSource {
                file_path: Some("/nonexistent/file.yaml".to_string()),
                ..Default::default()
            },
            variable: "test".to_string(),
        };

        let response = audit(no_root(), Json(req)).await;
        let response = response.into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    /// An answer's JSON with its `request_id`, a fresh UUID, left out
    async fn answer(response: impl IntoResponse) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_response().into_body(), usize::MAX)
            .await
            .unwrap();
        let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let id = json.as_object_mut().unwrap().remove("request_id").unwrap();
        assert_eq!(id.as_str().unwrap().len(), 36);
        json
    }

    /// Export and import call their cores (T3-API): the answers are the ones
    /// they gave when they ran the commands' printing functions
    #[tokio::test]
    async fn the_export_and_import_handlers_write_the_files_and_say_so() {
        let dir = tempfile::TempDir::new().unwrap();
        let excel = dir.path().join("budget.xlsx").to_string_lossy().to_string();
        let yaml = dir.path().join("budget.yaml").to_string_lossy().to_string();

        let exported = answer(
            export(
                no_root(),
                Json(ExportRequest {
                    yaml_path: "test-data/budget.yaml".to_string(),
                    excel_path: excel.clone(),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(
            exported,
            serde_json::json!({"success": true, "data": {
                "exported": true,
                "yaml_path": "test-data/budget.yaml",
                "excel_path": excel,
                "message": "Export completed",
            }})
        );

        let imported = answer(
            import_excel(
                no_root(),
                Json(ImportRequest {
                    excel_path: excel.clone(),
                    yaml_path: yaml.clone(),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(
            imported,
            serde_json::json!({"success": true, "data": {
                "imported": true,
                "excel_path": excel,
                "yaml_path": yaml,
                "message": "Import completed",
            }})
        );
        let model = crate::parser::parse_model(std::path::Path::new(&yaml)).unwrap();
        assert_eq!(
            model.scalars["assumptions.profit"].formula.as_deref(),
            Some("=assumptions.revenue - assumptions.expenses - assumptions.cost_of_goods")
        );
    }

    /// A failure is not an HTTP error: `success` is true and the flag false,
    /// with Forge's message (T1-INTERFACE-PARITY)
    #[tokio::test]
    async fn the_export_and_import_handlers_say_why_they_failed() {
        let exported = answer(
            export(
                no_root(),
                Json(ExportRequest {
                    yaml_path: "/nonexistent/file.yaml".to_string(),
                    excel_path: "/nonexistent/test.xlsx".to_string(),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(exported["success"], true);
        assert_eq!(exported["data"]["exported"], false);
        let message = exported["data"]["message"].as_str().unwrap();
        assert!(message.starts_with("Error: "), "{message}");

        let imported = answer(
            import_excel(
                no_root(),
                Json(ImportRequest {
                    excel_path: "/nonexistent/file.xlsx".to_string(),
                    yaml_path: "/nonexistent/test.yaml".to_string(),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(imported["success"], true);
        assert_eq!(imported["data"]["imported"], false);
        let message = imported["data"]["message"].as_str().unwrap();
        assert!(message.starts_with("Error: "), "{message}");
    }

    /// The state of a server with `--root`
    fn rooted(dir: &std::path::Path) -> State<Arc<AppState>> {
        State(Arc::new(AppState {
            version: "5.0.0".to_string(),
            token: None,
            loopback_only: true,
            root: Some(crate::path_root::Root::new(dir).unwrap()),
        }))
    }

    /// With `--root`, export and import refuse a file outside it, each of the
    /// two paths on its own, and say so in `message`; inside it, they work
    #[tokio::test]
    async fn export_and_import_stay_inside_the_root() {
        let root = tempfile::TempDir::new().unwrap();
        let outside = tempfile::TempDir::new().unwrap();
        std::fs::copy("test-data/budget.yaml", root.path().join("budget.yaml")).unwrap();
        std::fs::copy("test-data/budget.yaml", outside.path().join("budget.yaml")).unwrap();
        let outside_yaml = outside
            .path()
            .join("budget.yaml")
            .to_string_lossy()
            .to_string();
        let outside_xlsx = outside
            .path()
            .join("out.xlsx")
            .to_string_lossy()
            .to_string();

        let export_of = |yaml_path: String, excel_path: String| async {
            answer(
                export(
                    rooted(root.path()),
                    Json(ExportRequest {
                        yaml_path,
                        excel_path,
                    }),
                )
                .await,
            )
            .await
        };
        let import_of = |excel_path: String, yaml_path: String| async {
            answer(
                import_excel(
                    rooted(root.path()),
                    Json(ImportRequest {
                        excel_path,
                        yaml_path,
                    }),
                )
                .await,
            )
            .await
        };

        for (yaml, excel) in [
            (outside_yaml.clone(), "out.xlsx".to_string()),
            ("budget.yaml".to_string(), outside_xlsx.clone()),
        ] {
            let refused = export_of(yaml, excel).await;
            assert_eq!(refused["data"]["exported"], false, "{refused}");
            assert!(refused["data"]["message"]
                .as_str()
                .unwrap()
                .starts_with("Error: "));
        }
        assert!(!outside.path().join("out.xlsx").exists());

        let exported = export_of("budget.yaml".to_string(), "out.xlsx".to_string()).await;
        assert_eq!(exported["data"]["exported"], true, "{exported}");
        assert!(root.path().join("out.xlsx").exists());

        for (excel, yaml) in [
            (outside_xlsx, "back.yaml".to_string()),
            ("out.xlsx".to_string(), outside_yaml),
        ] {
            let refused = import_of(excel, yaml).await;
            assert_eq!(refused["data"]["imported"], false, "{refused}");
            assert!(refused["data"]["message"]
                .as_str()
                .unwrap()
                .starts_with("Error: "));
        }

        let imported = import_of("out.xlsx".to_string(), "back.yaml".to_string()).await;
        assert_eq!(imported["data"]["imported"], true, "{imported}");
        assert!(root.path().join("back.yaml").exists());
    }

    /// A model named by `file_path` is calculated and written back, and the
    /// answer says so; inline content is not (its text comes back instead)
    #[tokio::test]
    async fn calculating_a_file_says_the_file_was_updated() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("budget.yaml");
        std::fs::copy("test-data/budget.yaml", &file).unwrap();
        let response = answer(
            calculate(
                no_root(),
                Json(CalculateRequest {
                    source: ModelSource {
                        file_path: Some(file.to_string_lossy().to_string()),
                        ..Default::default()
                    },
                    dry_run: false,
                    scenario: None,
                    plan: false,
                }),
            )
            .await,
        )
        .await;
        assert_eq!(response["data"]["calculated"], true, "{response}");
        assert_eq!(
            response["data"]["message"],
            "Calculation completed and file updated"
        );
    }

    /// A file that is not a model is `valid: false` with Forge's reason, not
    /// an HTTP error
    #[tokio::test]
    async fn validating_a_file_that_is_not_a_model_says_why() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("broken.yaml");
        std::fs::write(&file, "scalars: [unclosed").unwrap();
        let response = answer(
            validate(
                no_root(),
                Json(ValidateRequest {
                    source: ModelSource {
                        file_path: Some(file.to_string_lossy().to_string()),
                        ..Default::default()
                    },
                }),
            )
            .await,
        )
        .await;
        assert_eq!(response["success"], true, "{response}");
        assert_eq!(response["data"]["valid"], false, "{response}");
        assert_ne!(response["data"]["message"], "", "{response}");
    }
}
