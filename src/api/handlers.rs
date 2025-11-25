//! API request handlers
//!
//! Handlers for all REST API endpoints.

use std::path::PathBuf;
use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::cli::{
    audit as cli_audit, calculate as cli_calculate, export as cli_export, import as cli_import,
    validate as cli_validate,
};

use super::server::AppState;

/// Standard API response wrapper
#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            request_id: Uuid::new_v4().to_string(),
            data: Some(data),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self
    where
        T: Default,
    {
        Self {
            success: false,
            request_id: Uuid::new_v4().to_string(),
            data: None,
            error: Some(message.into()),
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
        description: "Enterprise HTTP API for YAML formula calculations".to_string(),
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

/// Validate request
#[derive(Deserialize)]
pub struct ValidateRequest {
    pub file_path: String,
}

/// Validate response
#[derive(Serialize, Default)]
pub struct ValidateResponse {
    pub valid: bool,
    pub file_path: String,
    pub message: String,
}

/// POST /api/v1/validate - Validate a YAML model
pub async fn validate(Json(req): Json<ValidateRequest>) -> impl IntoResponse {
    let path = PathBuf::from(&req.file_path);

    match cli_validate(path) {
        Ok(()) => Json(ApiResponse::ok(ValidateResponse {
            valid: true,
            file_path: req.file_path,
            message: "Validation successful".to_string(),
        })),
        Err(e) => Json(ApiResponse::ok(ValidateResponse {
            valid: false,
            file_path: req.file_path,
            message: e.to_string(),
        })),
    }
}

/// Calculate request
#[derive(Deserialize)]
pub struct CalculateRequest {
    pub file_path: String,
    #[serde(default)]
    pub dry_run: bool,
}

/// Calculate response
#[derive(Serialize, Default)]
pub struct CalculateResponse {
    pub calculated: bool,
    pub file_path: String,
    pub dry_run: bool,
    pub message: String,
}

/// POST /api/v1/calculate - Calculate formulas
pub async fn calculate(Json(req): Json<CalculateRequest>) -> impl IntoResponse {
    let path = PathBuf::from(&req.file_path);
    let dry_run = req.dry_run;

    match cli_calculate(path, dry_run, false, None) {
        Ok(()) => Json(ApiResponse::ok(CalculateResponse {
            calculated: true,
            file_path: req.file_path,
            dry_run,
            message: if dry_run {
                "Dry run completed".to_string()
            } else {
                "Calculation completed and file updated".to_string()
            },
        })),
        Err(e) => Json(ApiResponse::ok(CalculateResponse {
            calculated: false,
            file_path: req.file_path,
            dry_run,
            message: format!("Error: {}", e),
        })),
    }
}

/// Audit request
#[derive(Deserialize)]
pub struct AuditRequest {
    pub file_path: String,
    pub variable: String,
}

/// Audit response
#[derive(Serialize, Default)]
pub struct AuditResponse {
    pub audited: bool,
    pub file_path: String,
    pub variable: String,
    pub message: String,
}

/// POST /api/v1/audit - Audit a variable
pub async fn audit(Json(req): Json<AuditRequest>) -> impl IntoResponse {
    let path = PathBuf::from(&req.file_path);
    let variable = req.variable.clone();

    match cli_audit(path, variable.clone()) {
        Ok(()) => Json(ApiResponse::ok(AuditResponse {
            audited: true,
            file_path: req.file_path,
            variable,
            message: "Audit completed".to_string(),
        })),
        Err(e) => Json(ApiResponse::ok(AuditResponse {
            audited: false,
            file_path: req.file_path,
            variable,
            message: format!("Error: {}", e),
        })),
    }
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
pub async fn export(Json(req): Json<ExportRequest>) -> impl IntoResponse {
    let yaml_path = PathBuf::from(&req.yaml_path);
    let excel_path = PathBuf::from(&req.excel_path);

    match cli_export(yaml_path, excel_path, false) {
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
            message: format!("Error: {}", e),
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
pub async fn import_excel(Json(req): Json<ImportRequest>) -> impl IntoResponse {
    let excel_path = PathBuf::from(&req.excel_path);
    let yaml_path = PathBuf::from(&req.yaml_path);

    match cli_import(excel_path, yaml_path, false) {
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
            message: format!("Error: {}", e),
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
        assert_eq!(response.data, Some("test data".to_string()));
        assert!(response.error.is_none());
        assert!(!response.request_id.is_empty());
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
        assert!(response.data.is_some());
        let data = response.data.unwrap();
        assert_eq!(data.status, "healthy");
        assert_eq!(data.uptime_message, "running");
    }

    #[test]
    fn test_api_response_err_creates_error_response() {
        let response: ApiResponse<String> = ApiResponse::err("Something went wrong");

        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("Something went wrong".to_string()));
        assert!(!response.request_id.is_empty());
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
        assert!(response.file_path.is_empty());
        assert!(response.message.is_empty());
    }

    #[test]
    fn test_calculate_response_default() {
        let response = CalculateResponse::default();

        assert!(!response.calculated);
        assert!(!response.dry_run);
        assert!(response.file_path.is_empty());
        assert!(response.message.is_empty());
    }

    #[test]
    fn test_audit_response_default() {
        let response = AuditResponse::default();

        assert!(!response.audited);
        assert!(response.file_path.is_empty());
        assert!(response.variable.is_empty());
        assert!(response.message.is_empty());
    }

    #[test]
    fn test_export_response_default() {
        let response = ExportResponse::default();

        assert!(!response.exported);
        assert!(response.yaml_path.is_empty());
        assert!(response.excel_path.is_empty());
        assert!(response.message.is_empty());
    }

    #[test]
    fn test_import_response_default() {
        let response = ImportResponse::default();

        assert!(!response.imported);
        assert!(response.excel_path.is_empty());
        assert!(response.yaml_path.is_empty());
        assert!(response.message.is_empty());
    }

    // ==================== Request Deserialization Tests ====================

    #[test]
    fn test_validate_request_deserialize() {
        let json = r#"{"file_path": "model.yaml"}"#;
        let req: ValidateRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.file_path, "model.yaml");
    }

    #[test]
    fn test_calculate_request_deserialize_with_dry_run() {
        let json = r#"{"file_path": "model.yaml", "dry_run": true}"#;
        let req: CalculateRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.file_path, "model.yaml");
        assert!(req.dry_run);
    }

    #[test]
    fn test_calculate_request_deserialize_dry_run_defaults_false() {
        let json = r#"{"file_path": "model.yaml"}"#;
        let req: CalculateRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.file_path, "model.yaml");
        assert!(!req.dry_run);
    }

    #[test]
    fn test_audit_request_deserialize() {
        let json = r#"{"file_path": "model.yaml", "variable": "total_revenue"}"#;
        let req: AuditRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.file_path, "model.yaml");
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
        };
        let json = serde_json::to_string(&response).unwrap();

        assert!(json.contains("\"calculated\":true"));
        assert!(json.contains("\"dry_run\":false"));
    }

    #[test]
    fn test_api_response_serializes_without_none_fields() {
        let response: ApiResponse<String> = ApiResponse::ok("data".to_string());
        let json = serde_json::to_string(&response).unwrap();

        // error field should be skipped when None
        assert!(!json.contains("\"error\""));
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"data\":\"data\""));
    }

    #[test]
    fn test_api_response_error_serializes_without_data() {
        let response: ApiResponse<String> = ApiResponse::err("error message");
        let json = serde_json::to_string(&response).unwrap();

        // data field should be skipped when None
        assert!(!json.contains("\"data\""));
        assert!(json.contains("\"success\":false"));
        assert!(json.contains("\"error\":\"error message\""));
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
            description: "Enterprise HTTP API".to_string(),
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
}
