//! API integration tests
//! ADR-004: 100% coverage required
//! Enterprise-only: API server is gated behind "full" feature
#![cfg(feature = "full")]

use royalbit_forge::api::handlers::{
    ApiResponse, AuditRequest, AuditResponse, CalculateRequest, CalculateResponse, EndpointInfo,
    ExportRequest, ExportResponse, HealthResponse, ImportRequest, ImportResponse, RootResponse,
    ValidateRequest, ValidateResponse, VersionResponse,
};

use royalbit_forge::api::server::{ApiConfig, AppState};

// ═══════════════════════════════════════════════════════════════════════════
// CONFIG TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_config_default() {
    let config = ApiConfig::default();
    assert_eq!(config.host, "127.0.0.1");
    assert_eq!(config.port, 8080);
}

#[test]
fn test_config_custom() {
    let config = ApiConfig {
        host: "0.0.0.0".to_string(),
        port: 3000,
    };
    assert_eq!(config.host, "0.0.0.0");
    assert_eq!(config.port, 3000);
}

#[test]
fn test_config_clone() {
    let config = ApiConfig::default();
    let cloned = config.clone();
    assert_eq!(config.host, cloned.host);
    assert_eq!(config.port, cloned.port);
}

// ═══════════════════════════════════════════════════════════════════════════
// APP STATE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_app_state_version() {
    let state = AppState {
        version: "1.0.0".to_string(),
    };
    assert_eq!(state.version, "1.0.0");
}

#[test]
fn test_app_state_clone() {
    let state = AppState {
        version: "2.0.0".to_string(),
    };
    let cloned = state.clone();
    assert_eq!(state.version, cloned.version);
}

#[test]
fn test_app_state_in_arc() {
    use std::sync::Arc;
    let state = Arc::new(AppState {
        version: "2.0.0".to_string(),
    });
    let state_clone = Arc::clone(&state);
    assert_eq!(state.version, state_clone.version);
}

// ═══════════════════════════════════════════════════════════════════════════
// API RESPONSE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_api_response_ok() {
    let response: ApiResponse<String> = ApiResponse::ok("test".to_string());
    assert!(response.success);
    assert_eq!(response.data, Some("test".to_string()));
    assert!(response.error.is_none());
    // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    assert_eq!(response.request_id.len(), 36);
}

#[test]
fn test_api_response_err() {
    let response: ApiResponse<String> = ApiResponse::err("error message");
    assert!(!response.success);
    assert!(response.data.is_none());
    assert_eq!(response.error, Some("error message".to_string()));
}

#[test]
fn test_api_response_unique_ids() {
    let r1: ApiResponse<i32> = ApiResponse::ok(1);
    let r2: ApiResponse<i32> = ApiResponse::ok(2);
    assert_ne!(r1.request_id, r2.request_id);
}

// ═══════════════════════════════════════════════════════════════════════════
// RESPONSE STRUCT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_health_response_fields() {
    let response = HealthResponse {
        status: "healthy".to_string(),
        uptime_message: "Running for 1 hour".to_string(),
    };
    assert_eq!(response.status, "healthy");
    assert_eq!(response.uptime_message, "Running for 1 hour");
}

#[test]
fn test_version_response_fields() {
    let response = VersionResponse {
        version: "1.0.0".to_string(),
        features: vec!["validate".to_string(), "calculate".to_string()],
    };
    assert_eq!(response.version, "1.0.0");
    assert_eq!(response.features.len(), 2);
}

#[test]
fn test_validate_response_fields() {
    let response = ValidateResponse {
        valid: true,
        file_path: "/path/to/file.yaml".to_string(),
        message: "Success".to_string(),
    };
    assert!(response.valid);
    assert_eq!(response.file_path, "/path/to/file.yaml");
}

#[test]
fn test_calculate_response_fields() {
    let response = CalculateResponse {
        calculated: true,
        file_path: "model.yaml".to_string(),
        dry_run: true,
        message: "Dry run completed".to_string(),
    };
    assert!(response.calculated);
    assert!(response.dry_run);
}

#[test]
fn test_audit_response_fields() {
    let response = AuditResponse {
        audited: true,
        file_path: "model.yaml".to_string(),
        variable: "profit".to_string(),
        message: "Audit complete".to_string(),
    };
    assert!(response.audited);
    assert_eq!(response.variable, "profit");
}

#[test]
fn test_export_response_fields() {
    let response = ExportResponse {
        exported: true,
        yaml_path: "model.yaml".to_string(),
        excel_path: "output.xlsx".to_string(),
        message: "Exported".to_string(),
    };
    assert!(response.exported);
}

#[test]
fn test_import_response_fields() {
    let response = ImportResponse {
        imported: true,
        excel_path: "input.xlsx".to_string(),
        yaml_path: "output.yaml".to_string(),
        message: "Imported".to_string(),
    };
    assert!(response.imported);
}

// ═══════════════════════════════════════════════════════════════════════════
// REQUEST STRUCT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_validate_request_deserialize() {
    let json = r#"{"file_path": "test.yaml"}"#;
    let req: ValidateRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.file_path, "test.yaml");
}

#[test]
fn test_calculate_request_deserialize() {
    let json = r#"{"file_path": "test.yaml", "dry_run": true}"#;
    let req: CalculateRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.file_path, "test.yaml");
    assert!(req.dry_run);
}

#[test]
fn test_calculate_request_dry_run_defaults_false() {
    let json = r#"{"file_path": "test.yaml"}"#;
    let req: CalculateRequest = serde_json::from_str(json).unwrap();
    assert!(!req.dry_run);
}

#[test]
fn test_audit_request_deserialize() {
    let json = r#"{"file_path": "test.yaml", "variable": "revenue"}"#;
    let req: AuditRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.file_path, "test.yaml");
    assert_eq!(req.variable, "revenue");
}

#[test]
fn test_export_request_deserialize() {
    let json = r#"{"yaml_path": "model.yaml", "excel_path": "out.xlsx"}"#;
    let req: ExportRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.yaml_path, "model.yaml");
    assert_eq!(req.excel_path, "out.xlsx");
}

#[test]
fn test_import_request_deserialize() {
    let json = r#"{"excel_path": "in.xlsx", "yaml_path": "model.yaml"}"#;
    let req: ImportRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.excel_path, "in.xlsx");
    assert_eq!(req.yaml_path, "model.yaml");
}

// ═══════════════════════════════════════════════════════════════════════════
// ROOT RESPONSE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_root_response_structure() {
    let response = RootResponse {
        name: "Forge API".to_string(),
        version: "2.0.0".to_string(),
        description: "API Server".to_string(),
        endpoints: vec![EndpointInfo {
            path: "/health".to_string(),
            method: "GET".to_string(),
            description: "Health check".to_string(),
        }],
    };
    assert_eq!(response.name, "Forge API");
    assert_eq!(response.endpoints.len(), 1);
}

#[test]
fn test_endpoint_info_serialize() {
    let info = EndpointInfo {
        path: "/api/v1/test".to_string(),
        method: "POST".to_string(),
        description: "Test endpoint".to_string(),
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("\"/api/v1/test\""));
    assert!(json.contains("\"POST\""));
}

// ═══════════════════════════════════════════════════════════════════════════
// DEFAULT TRAIT TESTS
// ═══════════════════════════════════════════════════════════════════════════

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
}

#[test]
fn test_audit_response_default() {
    let response = AuditResponse::default();
    assert!(!response.audited);
    assert!(response.variable.is_empty());
}

#[test]
fn test_export_response_default() {
    let response = ExportResponse::default();
    assert!(!response.exported);
}

#[test]
fn test_import_response_default() {
    let response = ImportResponse::default();
    assert!(!response.imported);
}

// ═══════════════════════════════════════════════════════════════════════════
// SERIALIZATION TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_health_response_serialize() {
    let response = HealthResponse {
        status: "ok".to_string(),
        uptime_message: "running".to_string(),
    };
    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"status\":\"ok\""));
}

#[test]
fn test_api_response_skips_none() {
    let response: ApiResponse<String> = ApiResponse::ok("data".to_string());
    let json = serde_json::to_string(&response).unwrap();
    // error should not appear when None
    assert!(!json.contains("error"));
}

#[test]
fn test_api_response_err_skips_data() {
    let response: ApiResponse<String> = ApiResponse::err("err");
    let json = serde_json::to_string(&response).unwrap();
    // data should not appear when None
    assert!(!json.contains("\"data\""));
}

// ═══════════════════════════════════════════════════════════════════════════
// ASYNC HANDLER TESTS
// ═══════════════════════════════════════════════════════════════════════════

use axum::{extract::State, response::IntoResponse, Json};
use royalbit_forge::api::handlers::{
    audit, calculate, export, health, import_excel, root, validate, version,
};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_health_handler() {
    let response = health().await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_root_handler() {
    let state = Arc::new(AppState {
        version: "5.0.0".to_string(),
    });
    let response = root(State(state)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_version_handler() {
    let state = Arc::new(AppState {
        version: "5.0.0".to_string(),
    });
    let response = version(State(state)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_validate_handler_success() {
    let req = ValidateRequest {
        file_path: "test-data/budget.yaml".to_string(),
    };
    let response = validate(Json(req)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_validate_handler_nonexistent() {
    let req = ValidateRequest {
        file_path: "/nonexistent/file.yaml".to_string(),
    };
    let response = validate(Json(req)).await;
    let json_response = response.into_response();
    // Should still return 200 with error in response body
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_calculate_handler_dry_run() {
    let req = CalculateRequest {
        file_path: "test-data/budget.yaml".to_string(),
        dry_run: true,
    };
    let response = calculate(Json(req)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_calculate_handler_nonexistent() {
    let req = CalculateRequest {
        file_path: "/nonexistent/file.yaml".to_string(),
        dry_run: true,
    };
    let response = calculate(Json(req)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_audit_handler() {
    let req = AuditRequest {
        file_path: "test-data/budget.yaml".to_string(),
        variable: "assumptions.profit".to_string(),
    };
    let response = audit(Json(req)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_audit_handler_nonexistent() {
    let req = AuditRequest {
        file_path: "/nonexistent/file.yaml".to_string(),
        variable: "test".to_string(),
    };
    let response = audit(Json(req)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_export_handler() {
    let temp_dir = TempDir::new().unwrap();
    let excel_path = temp_dir.path().join("api_export_test.xlsx");

    let req = ExportRequest {
        yaml_path: "test-data/budget.yaml".to_string(),
        excel_path: excel_path.to_str().unwrap().to_string(),
    };
    let response = export(Json(req)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_export_handler_nonexistent() {
    let req = ExportRequest {
        yaml_path: "/nonexistent/file.yaml".to_string(),
        excel_path: "/tmp/out.xlsx".to_string(),
    };
    let response = export(Json(req)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_import_handler() {
    let temp_dir = TempDir::new().unwrap();
    let excel_path = temp_dir.path().join("import_test.xlsx");
    let yaml_path = temp_dir.path().join("imported.yaml");

    // First export to create an Excel file
    let export_req = ExportRequest {
        yaml_path: "test-data/budget.yaml".to_string(),
        excel_path: excel_path.to_str().unwrap().to_string(),
    };
    let _ = export(Json(export_req)).await;

    // Then import it back
    let req = ImportRequest {
        excel_path: excel_path.to_str().unwrap().to_string(),
        yaml_path: yaml_path.to_str().unwrap().to_string(),
    };
    let response = import_excel(Json(req)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}

#[tokio::test]
async fn test_import_handler_nonexistent() {
    let req = ImportRequest {
        excel_path: "/nonexistent/file.xlsx".to_string(),
        yaml_path: "/tmp/out.yaml".to_string(),
    };
    let response = import_excel(Json(req)).await;
    let json_response = response.into_response();
    assert_eq!(json_response.status(), 200);
}
