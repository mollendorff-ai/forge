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

    match cli_calculate(path, dry_run, false) {
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
