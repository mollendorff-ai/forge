//! The tools that read, calculate, check, export and import a model.

use super::{to_json, ForgeMcpServer};
use crate::mcp::types::{
    AuditRequest, CalculateRequest, ExportRequest, ImportRequest, ValidateRequest,
};

use crate::inline_model::resolve_model_input;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::cli::calculate_core;
use crate::cli::{audit_core, export_buffer_core, export_core, import_core, validate_core};

#[tool_router(router = model_tools, vis = "pub(super)")]
impl ForgeMcpServer {
    #[tool(
        name = "forge_validate",
        description = "Validate a Forge YAML model file for formula errors, circular dependencies, and type mismatches."
    )]
    pub(super) fn validate(
        &self,
        Parameters(req): Parameters<ValidateRequest>,
    ) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        validate_core(&path)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Validation failed: {e}"))
    }

    #[tool(
        name = "forge_calculate",
        description = "Calculate all formulas in a Forge YAML model. Unless dry_run, writes each formula's result next to it in the file, changing nothing else (comments kept)."
    )]
    fn calculate(&self, Parameters(req): Parameters<CalculateRequest>) -> Result<String, String> {
        let is_inline = req.content.is_some();
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        let result = calculate_core(path.as_path(), req.dry_run, req.scenario.as_deref())
            .map_err(|e| format!("Calculation failed: {e}"))?;

        let mut json_val: serde_json::Value =
            serde_json::to_value(&result).map_err(|e| format!("Serialization failed: {e}"))?;

        // When using inline content with dry_run=false, read back the calculated file
        if is_inline && !req.dry_run {
            if let Ok(calculated) = std::fs::read_to_string(&path) {
                json_val["calculated_content"] = serde_json::Value::String(calculated);
            }
        }

        serde_json::to_string(&crate::core::precision::Rounded(&json_val))
            .map_err(|e| format!("Serialization failed: {e}"))
    }

    #[tool(
        name = "forge_audit",
        description = "Audit a specific variable to see its dependency tree and calculated value."
    )]
    fn audit(&self, Parameters(req): Parameters<AuditRequest>) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        audit_core(&path, &req.variable)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Audit failed: {e}"))
    }

    #[tool(
        name = "forge_export",
        description = "Export a Forge YAML model to an Excel workbook. \
            If `excel_path` is provided, writes the .xlsx file to disk. \
            If omitted, returns the workbook inline as base64 in the \
            `excel_base64` response field (for sandboxed clients)."
    )]
    pub(super) fn export(
        &self,
        Parameters(req): Parameters<ExportRequest>,
    ) -> Result<String, String> {
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.yaml_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        req.excel_path.as_ref().map_or_else(
            || {
                export_buffer_core(&path)
                    .map(|r| to_json(&r))
                    .map_err(|e| format!("Export failed: {e}"))
            },
            |excel_path| {
                export_core(&path, &self.file(excel_path)?)
                    .map(|r| to_json(&r))
                    .map_err(|e| format!("Export failed: {e}"))
            },
        )
    }

    #[tool(
        name = "forge_import",
        description = "Import an Excel workbook into a Forge YAML model."
    )]
    pub(super) fn import(
        &self,
        Parameters(req): Parameters<ImportRequest>,
    ) -> Result<String, String> {
        import_core(
            &self.file(&req.excel_path)?,
            &self.file(&req.yaml_path)?,
            false,
            false,
        )
        .map(|r| to_json(&r))
        .map_err(|e| format!("Import failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::server::tests::{err_text, ok_text, INLINE_YAML};
    use rmcp::handler::server::wrapper::Parameters;
    use std::collections::HashMap;

    use tempfile::TempDir;

    fn validate_inline_with_include(file: &str, include_content: &str) -> Result<String, String> {
        let yaml = format!(
                "_forge_version: \"5.0.0\"\n_includes:\n  - file: \"{file}\"\n    as: \"ext\"\nscalars:\n  total:\n    value: 100\n"
            );
        let mut includes = HashMap::new();
        includes.insert("ext".to_string(), include_content.to_string());
        ForgeMcpServer::new().validate(Parameters(ValidateRequest {
            file_path: None,
            content: Some(yaml),
            includes: Some(includes),
            verbose: false,
        }))
    }

    #[test]
    fn test_call_validate_success() {
        let server = ForgeMcpServer::new();
        let result = server.validate(Parameters(ValidateRequest {
            file_path: Some("test-data/budget.yaml".into()),
            content: None,
            includes: None,
            verbose: false,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_validate_nonexistent() {
        let server = ForgeMcpServer::new();
        let result = server.validate(Parameters(ValidateRequest {
            file_path: Some("nonexistent.yaml".into()),
            content: None,
            includes: None,
            verbose: false,
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_call_calculate_dry_run() {
        let server = ForgeMcpServer::new();
        let result = server.calculate(Parameters(CalculateRequest {
            file_path: Some("test-data/budget.yaml".into()),
            content: None,
            includes: None,
            dry_run: true,
            scenario: None,
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_calculate_nonexistent() {
        let server = ForgeMcpServer::new();
        let result = server.calculate(Parameters(CalculateRequest {
            file_path: Some("nonexistent.yaml".into()),
            content: None,
            includes: None,
            dry_run: true,
            scenario: None,
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_call_audit() {
        let server = ForgeMcpServer::new();
        // audit may succeed or fail, but should not panic
        let _ = server.audit(Parameters(AuditRequest {
            file_path: Some("test-data/budget.yaml".into()),
            content: None,
            includes: None,
            variable: "assumptions.profit".into(),
        }));
    }

    #[test]
    fn test_call_export() {
        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("mcp_test_export.xlsx");

        let server = ForgeMcpServer::new();
        let result = server.export(Parameters(ExportRequest {
            yaml_path: Some("test-data/budget.yaml".into()),
            content: None,
            includes: None,
            excel_path: Some(output.to_str().unwrap().into()),
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_call_import() {
        let temp_dir = TempDir::new().unwrap();
        let excel_path = temp_dir.path().join("import_test.xlsx");
        let yaml_path = temp_dir.path().join("imported.yaml");

        let server = ForgeMcpServer::new();

        // First export to create Excel file
        let _ = server.export(Parameters(ExportRequest {
            yaml_path: Some("test-data/budget.yaml".into()),
            content: None,
            includes: None,
            excel_path: Some(excel_path.to_str().unwrap().into()),
        }));

        // Then import
        let result = server.import(Parameters(ImportRequest {
            excel_path: excel_path.to_str().unwrap().into(),
            yaml_path: yaml_path.to_str().unwrap().into(),
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_returns_structured_json() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.validate(Parameters(ValidateRequest {
            file_path: Some("test-data/budget.yaml".into()),
            content: None,
            includes: None,
            verbose: false,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("tables_valid").is_some());
        assert!(parsed.get("scalars_valid").is_some());
        assert!(parsed.get("table_count").is_some());
    }

    #[test]
    fn test_calculate_returns_structured_json() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.calculate(Parameters(CalculateRequest {
            file_path: Some("test-data/budget.yaml".into()),
            content: None,
            includes: None,
            dry_run: true,
            scenario: None,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("tables").is_some());
        assert!(parsed.get("scalars").is_some());
        assert_eq!(parsed["dry_run"], true);
    }

    #[test]
    fn test_inline_validate() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.validate(Parameters(ValidateRequest {
            file_path: None,
            content: Some(INLINE_YAML.into()),
            includes: None,
            verbose: false,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("scalars_valid").is_some());
    }

    #[test]
    fn test_inline_calculate_dry_run() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.calculate(Parameters(CalculateRequest {
            file_path: None,
            content: Some(INLINE_YAML.into()),
            includes: None,
            dry_run: true,
            scenario: None,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("scalars").is_some());
        assert_eq!(parsed["dry_run"], true);
        // dry_run should not have calculated_content
        assert!(parsed.get("calculated_content").is_none());
    }

    #[test]
    fn test_inline_calculate_write_back() {
        let server = ForgeMcpServer::new();
        let text = ok_text(server.calculate(Parameters(CalculateRequest {
            file_path: None,
            content: Some(INLINE_YAML.into()),
            includes: None,
            dry_run: false,
            scenario: None,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        // Non-dry-run with inline content should include calculated_content
        assert!(parsed.get("calculated_content").is_some());
        let calculated = parsed["calculated_content"].as_str().unwrap();
        assert!(calculated.contains("profit"));
    }

    /// Every inline-content example in `docs/MCP.md` must run against the server,
    /// so the guide cannot drift from the model format again.
    #[test]
    fn test_mcp_doc_inline_examples_run() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/MCP.md");
        let doc = std::fs::read_to_string(path).expect("docs/MCP.md should exist");
        let mut ran = 0;
        for line in doc.lines().filter(|l| l.starts_with("{\"jsonrpc\"")) {
            let call: serde_json::Value =
                serde_json::from_str(line).expect("doc example should be valid JSON");
            let name = call["params"]["name"].as_str().expect("tool name");
            let args = call["params"]["arguments"].clone();
            if args.get("content").is_none() {
                continue; // file_path examples name files that do not exist here
            }
            let server = ForgeMcpServer::new();
            let result = match name {
                "forge_validate" => server.validate(Parameters(
                    serde_json::from_value(args).expect("validate arguments"),
                )),
                "forge_calculate" => server.calculate(Parameters(
                    serde_json::from_value(args).expect("calculate arguments"),
                )),
                other => panic!("docs/MCP.md has an inline example for {other}; add it here"),
            };
            if let Err(e) = result {
                panic!("docs/MCP.md example for {name} failed: {e}\n{line}");
            }
            ran += 1;
        }
        assert!(ran >= 3, "expected at least 3 inline examples, ran {ran}");
    }

    #[test]
    fn test_inline_with_includes() {
        let yaml_with_includes = r#"
_forge_version: "5.0.0"
_includes:
  - file: "pricing.yaml"
    as: "pricing"
scalars:
  total:
    value: 100
"#;
        let pricing_yaml = r#"
_forge_version: "5.0.0"
scalars:
  price:
    value: 50
"#;
        let mut includes = HashMap::new();
        includes.insert("pricing".into(), pricing_yaml.into());

        let server = ForgeMcpServer::new();
        let text = ok_text(server.validate(Parameters(ValidateRequest {
            file_path: None,
            content: Some(yaml_with_includes.into()),
            includes: Some(includes),
            verbose: false,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("scalars_valid").is_some());
    }

    #[test]
    fn test_inline_includes_missing_namespace() {
        let yaml_with_includes = r#"
_forge_version: "5.0.0"
_includes:
  - file: "pricing.yaml"
    as: "pricing"
scalars:
  total:
    value: 100
"#;
        // Provide includes map but with wrong namespace
        let mut includes = HashMap::new();
        includes.insert("wrong_namespace".into(), "content".into());

        let server = ForgeMcpServer::new();
        let result = server.validate(Parameters(ValidateRequest {
            file_path: None,
            content: Some(yaml_with_includes.into()),
            includes: Some(includes),
            verbose: false,
        }));
        let err = err_text(result);
        assert!(err.contains("pricing"));
    }

    #[test]
    fn test_inline_includes_no_map() {
        let yaml_with_includes = r#"
_forge_version: "5.0.0"
_includes:
  - file: "pricing.yaml"
    as: "pricing"
scalars:
  total:
    value: 100
"#;
        let server = ForgeMcpServer::new();
        let result = server.validate(Parameters(ValidateRequest {
            file_path: None,
            content: Some(yaml_with_includes.into()),
            includes: None,
            verbose: false,
        }));
        let err = err_text(result);
        assert!(err.contains("includes"));
    }

    #[test]
    fn test_inline_includes_nested_dir() {
        let yaml_with_includes = r#"
_forge_version: "5.0.0"
_includes:
  - file: "data/sub/pricing.yaml"
    as: "pricing"
scalars:
  total:
    value: 100
"#;
        let pricing_yaml = r#"
_forge_version: "5.0.0"
scalars:
  price:
    value: 25
"#;
        let mut includes = HashMap::new();
        includes.insert("pricing".into(), pricing_yaml.into());

        let server = ForgeMcpServer::new();
        let text = ok_text(server.validate(Parameters(ValidateRequest {
            file_path: None,
            content: Some(yaml_with_includes.into()),
            includes: Some(includes),
            verbose: false,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.get("scalars_valid").is_some());
    }

    #[test]
    fn test_inline_include_absolute_path_writes_nothing() {
        let outside = TempDir::new().unwrap();
        let target = outside.path().join("written.yaml");
        let err = err_text(validate_inline_with_include(
            target.to_str().unwrap(),
            "planted: true\n",
        ));
        assert!(err.contains("not allowed"), "unexpected error: {err}");
        assert!(!target.exists(), "include escaped to an absolute path");
    }

    #[test]
    fn test_inline_include_parent_path_writes_nothing() {
        let name = format!("forge-escape-{}.yaml", std::process::id());
        let err = err_text(validate_inline_with_include(
            &format!("../{name}"),
            "planted: true\n",
        ));
        assert!(err.contains("not allowed"), "unexpected error: {err}");
        assert!(!std::env::temp_dir().join(&name).exists());
    }

    #[test]
    fn test_inline_include_hidden_parent_path_rejected() {
        let err = err_text(validate_inline_with_include(
            "data/../../escape.yaml",
            "planted: true\n",
        ));
        assert!(err.contains("not allowed"), "unexpected error: {err}");
    }

    #[test]
    fn test_inline_nested_include_cannot_read_outside() {
        let outside = TempDir::new().unwrap();
        let secret = outside.path().join("secret.yaml");
        std::fs::write(
            &secret,
            "_forge_version: \"5.0.0\"\nscalars:\n  secret:\n    value: 42\n",
        )
        .unwrap();
        let content = format!(
            "_forge_version: \"5.0.0\"\n_includes:\n  - file: \"{}\"\n    as: \"s\"\n",
            secret.display()
        );
        let err = err_text(validate_inline_with_include("pricing.yaml", &content));
        assert!(err.contains("not allowed"), "unexpected error: {err}");
    }

    #[test]
    fn test_inline_include_dot_slash_path_allowed() {
        let content = "_forge_version: \"5.0.0\"\nscalars:\n  price:\n    value: 25\n";
        let text = ok_text(validate_inline_with_include("./data/pricing.yaml", content));
        assert!(text.contains("scalars_valid"));
    }

    #[test]
    fn test_inline_export() {
        let temp_dir = TempDir::new().unwrap();
        let output = temp_dir.path().join("inline_export.xlsx");

        let server = ForgeMcpServer::new();
        let result = server.export(Parameters(ExportRequest {
            yaml_path: None,
            content: Some(INLINE_YAML.into()),
            includes: None,
            excel_path: Some(output.to_str().unwrap().into()),
        }));
        assert!(result.is_ok());
    }

    #[test]
    fn test_inline_export_buffer() {
        // When excel_path is omitted, should return base64-encoded Excel content
        let server = ForgeMcpServer::new();
        let text = ok_text(server.export(Parameters(ExportRequest {
            yaml_path: None,
            content: Some(INLINE_YAML.into()),
            includes: None,
            excel_path: None,
        })));
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed["excel_base64"].as_str().is_some());
        assert!(parsed["byte_count"].as_u64().unwrap() > 0);
        // Verify it's valid base64 that decodes to an xlsx magic number (PK\x03\x04)
        let b64 = parsed["excel_base64"].as_str().unwrap();
        let bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64).unwrap();
        assert_eq!(
            &bytes[..4],
            b"PK\x03\x04",
            "Should be a valid ZIP/XLSX header"
        );
    }
}
