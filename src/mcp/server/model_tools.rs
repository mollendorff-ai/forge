//! The tools that read, calculate, check, export and import a model.

use super::{to_json, ForgeMcpServer};
use crate::mcp::types::{
    AuditRequest, CalculateRequest, EditRequest, ExportRequest, ImportRequest, ValidateRequest,
};

use crate::inline_model::resolve_model_input;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::cli::commands::edit_core;
use crate::cli::{audit_core, export_buffer_core, export_core, import_core, validate_core};
use crate::cli::{calculate_core, calculate_plan_core};

#[tool_router(router = model_tools, vis = "pub(super)")]
impl ForgeMcpServer {
    #[tool(
        name = "forge_validate",
        description = "Validate a Forge YAML model file for formula errors, circular dependencies, and type mismatches, and check each stored value against its formula. When the model states its results (expect:, format 5.6.0), it calculates the file and each scenario that has one, and lists every result that differs, with the scenario, the name and both numbers."
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
        description = "Calculate all formulas in a Forge YAML model. Unless dry_run, writes each formula's result next to it in the file, changing nothing else (comments kept). An uncertain model (MC.* inputs) is refused unless plan, which calculates it with each uncertain input at its plan figure and writes nothing."
    )]
    pub(super) fn calculate(
        &self,
        Parameters(req): Parameters<CalculateRequest>,
    ) -> Result<String, String> {
        let is_inline = req.content.is_some();
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        let dry_run = req.dry_run || req.plan;
        let result = if req.plan {
            calculate_plan_core(path.as_path(), req.scenario.as_deref())
        } else {
            calculate_core(path.as_path(), dry_run, req.scenario.as_deref())
        }
        .map_err(|e| format!("Calculation failed: {e}"))?;

        let mut json_val = serde_json::to_value(&result).unwrap_or_default();

        // When using inline content with dry_run=false, read back the calculated
        // file; under a scenario nothing is written (ADR-074)
        if is_inline && !dry_run && req.scenario.is_none() {
            json_val["calculated_content"] =
                serde_json::Value::String(std::fs::read_to_string(&path).unwrap_or_default());
        }

        Ok(to_json(&json_val))
    }

    #[tool(
        name = "forge_edit",
        description = "Change a model's text: set an input's value (set_value) or a formula (set_formula), add an input, a formula or a table's column (add), delete one (delete), rename a scalar, a group, a table, a column, a game (path price_war), a player (price_war.players.lune) or a strategy (price_war.strategies.hold) to new_name with every name that reads it, a game's results included, in every file (rename), or make an input a distribution with its value kept as the plan, adding a monte_carlo section if there is none (make_uncertain), and back (make_certain), in the model or in a file it includes (@alias.name). Every byte outside the change stays as it was, comments and layout included. A change is made only when Forge reads the model back valid with that change and nothing else; if one cannot be made, none is. Answers the new text of each file that changed, by its path from the model's folder; with file_path it also writes them, each kept as it was in a .bak, unless dry_run. Results are not written: forge_calculate does."
    )]
    pub(super) fn edit(&self, Parameters(req): Parameters<EditRequest>) -> Result<String, String> {
        let inline = req.content.is_some();
        let (path, _tmpdir) = resolve_model_input(
            self.root.as_ref(),
            req.file_path.as_deref(),
            req.content.as_deref(),
            req.includes.as_ref(),
        )?;
        edit_core(&path, &req.changes, req.dry_run || inline)
            .map(|r| to_json(&r))
            .map_err(|e| format!("Edit failed: {e}"))
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
    use crate::mcp::server::tests::{
        bagel_source, close, err_text, example_source, json, ok_text, Source, INLINE_YAML,
    };
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
            plan: false,
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
            plan: false,
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
    fn validate_returns_each_expectation_that_differs() {
        // The same report as the command (ADR-074), as data, not a tool error
        let server = ForgeMcpServer::new();
        let text = ok_text(
            server.validate(Parameters(ValidateRequest {
                file_path: None,
                content: Some(
                    "_forge_version: \"5.6.0\"\nw: 0.6\nout:\n  twice: {formula: \"=w * 2\"}\n\
                 expect: {out.twice: 1.2}\n\
                 scenarios:\n  short:\n    scalars: {w: 0.4}\n    expect: {out.twice: =9/10}\n"
                        .into(),
                ),
                includes: None,
                verbose: false,
            })),
        );
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["expectations_checked"], 2);
        assert_eq!(parsed["expectations_met"], false);
        let d = &parsed["expectation_differences"][0];
        assert_eq!(d["scenario"], "short");
        assert_eq!(d["name"], "out.twice");
        assert_eq!(d["expected"], 0.9);
        assert_eq!(d["formula"], "=9/10");
        assert_eq!(d["got"], 0.8);
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
            plan: false,
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
            plan: false,
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
            plan: false,
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
        let mut ran: Vec<(String, Result<String, String>)> = Vec::new();
        for line in doc.lines().filter(|l| l.starts_with("{\"jsonrpc\"")) {
            let call: serde_json::Value =
                serde_json::from_str(line).expect("doc example should be valid JSON");
            let name = call["params"]["name"].as_str().expect("tool name");
            let args = call["params"]["arguments"].clone();
            if args.get("content").is_none() {
                continue; // file_path examples name files that do not exist here
            }
            let server = ForgeMcpServer::new();
            assert!(
                ["forge_validate", "forge_calculate"].contains(&name),
                "{name} is an inline example this test does not run"
            );
            let result = if name == "forge_validate" {
                server.validate(Parameters(
                    serde_json::from_value(args).expect("validate arguments"),
                ))
            } else {
                server.calculate(Parameters(
                    serde_json::from_value(args).expect("calculate arguments"),
                ))
            };
            ran.push((name.to_string(), result.map(|_| line.to_string())));
        }
        assert!(
            ran.len() >= 3,
            "expected at least 3 inline examples, ran {ran:?}"
        );
        assert!(
            ran.iter().all(|(_, result)| result.is_ok()),
            "an inline example of docs/MCP.md failed: {ran:?}"
        );
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

    // ─────────────────────────────────────────────────────────────────────────
    // The numbers of examples/bagel through the tools (T1-INTERFACE-PARITY):
    // each is asserted by the command-line tests in cli/commands/examples.rs
    // (`bagel_example`, the Python is quoted there), here by file path and as
    // inline content with `costs.yaml` passed in `includes`.
    // ─────────────────────────────────────────────────────────────────────────

    const BAGEL: &str = "examples/bagel/bagel.yaml";

    /// `forge_calculate` of the bagel plan, by path or inline, in a scenario
    fn bagel_calculate(inline: bool, scenario: Option<&str>) -> serde_json::Value {
        let (file_path, content, includes) = plan(inline);
        json(&ok_text(ForgeMcpServer::new().calculate(Parameters(
            CalculateRequest {
                file_path,
                content,
                includes,
                dry_run: true,
                scenario: scenario.map(Into::into),
                plan: false,
            },
        ))))
    }

    /// The bagel plan, by path or inline with `costs.yaml`
    fn plan(inline: bool) -> Source {
        bagel_source(inline, "bagel.yaml", &[("costs", "costs.yaml")])
    }

    /// A dependency of the audited variable, wherever it is in the tree
    fn find<'a>(deps: &'a serde_json::Value, name: &str) -> Option<&'a serde_json::Value> {
        deps.as_array().into_iter().flatten().find_map(|d| {
            (d["name"] == name)
                .then_some(d)
                .or_else(|| find(&d["children"], name))
        })
    }

    #[test]
    fn calculate_gives_the_years_profit_by_path_and_inline() {
        for inline in [false, true] {
            let result = bagel_calculate(inline, None);
            let scalars = &result["scalars"];
            let profit = scalars["year.profit"].as_f64().unwrap();
            assert!(
                close(profit, 786.320_361_7, 1e-6),
                "inline {inline}: {profit}"
            );
            let revenue = scalars["year.revenue"].as_f64().unwrap();
            assert!(
                close(revenue, 386_827.449_537_8, 1e-6),
                "inline {inline}: {revenue}"
            );
            let ret = scalars["year.first_year_return"].as_f64().unwrap();
            assert!(
                close(ret, 786.320_361_7 / 95_000.0, 1e-9),
                "inline {inline}: {ret}"
            );
            assert_eq!(result["formatted"]["year.margin"], "0.2%");
        }
    }

    /// The scenario sets a value in the included file: the flour price.
    #[test]
    fn a_scenario_overrides_a_value_in_the_included_file_by_path_and_inline() {
        for inline in [false, true] {
            let result = bagel_calculate(inline, Some("flour_shock"));
            let profit = result["scalars"]["year.profit"].as_f64().unwrap();
            assert!(
                close(profit, -16_943.271_075, 1e-6),
                "inline {inline}: {profit}"
            );
            let ret = result["scalars"]["year.first_year_return"]
                .as_f64()
                .unwrap();
            assert!(
                close(ret, -16_943.271_075 / 95_000.0, 1e-9),
                "inline {inline}: {ret}"
            );
        }
    }

    #[test]
    fn calculate_under_a_scenario_writes_nothing_back() {
        let server = ForgeMcpServer::new();
        let text = "_forge_version: \"5.5.0\"\nw: 0.6\nout:\n  twice: {formula: \"=w * 2\"}\n\
                    scenarios:\n  short:\n    scalars: {w: 0.4}\n";
        let answer = |scenario: Option<&str>| {
            json(&ok_text(server.calculate(Parameters(CalculateRequest {
                file_path: None,
                content: Some(text.into()),
                includes: None,
                dry_run: false,
                scenario: scenario.map(str::to_string),
                plan: false,
            }))))
        };
        let under = answer(Some("short"));
        assert_eq!(under["scalars"]["out.twice"], 0.8);
        assert_eq!(under["file_updated"], false);
        assert!(under.get("calculated_content").is_none(), "{under}");
        let plan = answer(None);
        assert!(plan["calculated_content"]
            .as_str()
            .unwrap()
            .contains("value: 1.2"));
    }

    #[test]
    fn validate_counts_the_bagel_model_by_path_and_inline() {
        let server = ForgeMcpServer::new();
        for inline in [false, true] {
            let (file_path, content, includes) = plan(inline);
            let result = json(&ok_text(server.validate(Parameters(ValidateRequest {
                file_path,
                content,
                includes,
                verbose: false,
            }))));
            assert_eq!(result["tables_valid"], true, "inline {inline}");
            assert_eq!(result["scalars_valid"], true, "inline {inline}");
            assert_eq!(result["table_count"], 1, "inline {inline}");
            assert_eq!(result["scalar_count"], 19, "inline {inline}");
            assert_eq!(
                result["mismatches"],
                serde_json::json!([]),
                "inline {inline}"
            );
        }
    }

    /// `bagel_profit_down_to_the_included_costs` in cli/commands/audit.rs
    #[test]
    fn audit_follows_the_profit_down_to_the_included_costs_by_path_and_inline() {
        let server = ForgeMcpServer::new();
        for inline in [false, true] {
            let (file_path, content, includes) = plan(inline);
            let result = json(&ok_text(server.audit(Parameters(AuditRequest {
                file_path,
                content,
                includes,
                variable: "year.profit".into(),
            }))));
            let profit = result["calculated_value"].as_f64().unwrap();
            assert!(
                close(profit, 786.320_361_7, 1e-6),
                "inline {inline}: {profit}"
            );
            let deps = &result["dependencies"];
            assert_eq!(deps[0]["name"], "months.profit");
            assert_eq!(deps[0]["values"].as_array().map(Vec::len), Some(12));
            let wage = find(deps, "@costs.labour.loaded_wage").unwrap();
            assert_eq!(wage["dep_type"], "Included scalar");
            assert_eq!(wage["source"], "costs.yaml");
            assert!(close(wage["value"].as_f64().unwrap(), 21.275, 1e-9));
            let rate = find(&wage["children"], "@costs.labour.wage_per_hour").unwrap();
            assert!(close(rate["value"].as_f64().unwrap(), 18.5, 1e-12));
        }
    }

    /// The bagel plan to a workbook and back through the tools alone: by path
    /// to a file, and inline as base64 written to a file here. The model that
    /// comes back, with its `costs.yaml`, calculates the same year.
    #[test]
    fn export_then_import_gives_the_bagel_year_back_by_path_and_inline() {
        let server = ForgeMcpServer::new();
        let dir = TempDir::new().unwrap();
        let by_path = dir.path().join("by_path.xlsx");
        let exported = json(&ok_text(server.export(Parameters(ExportRequest {
            yaml_path: Some(BAGEL.into()),
            content: None,
            includes: None,
            excel_path: Some(by_path.to_str().unwrap().into()),
        }))));
        assert_eq!(exported["table_count"], 1);
        assert_eq!(exported["scalar_count"], 19);

        let (_, content, includes) = plan(true);
        let inline = json(&ok_text(server.export(Parameters(ExportRequest {
            yaml_path: None,
            content,
            includes,
            excel_path: None,
        }))));
        assert_eq!(inline["table_count"], 1);
        assert_eq!(inline["scalar_count"], 19);
        let bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            inline["excel_base64"].as_str().unwrap(),
        )
        .unwrap();
        assert_eq!(bytes.len(), inline["byte_count"]);
        let by_content = dir.path().join("by_content.xlsx");
        std::fs::write(&by_content, bytes).unwrap();

        for (excel, name) in [(by_path, "path"), (by_content, "content")] {
            let folder = dir.path().join(name);
            std::fs::create_dir(&folder).unwrap();
            let yaml = folder.join("model.yaml");
            let imported = json(&ok_text(server.import(Parameters(ImportRequest {
                excel_path: excel.to_str().unwrap().into(),
                yaml_path: yaml.to_str().unwrap().into(),
            }))));
            assert_eq!(
                imported["included_files"],
                serde_json::json!(["costs.yaml"])
            );
            let back = json(&ok_text(server.calculate(Parameters(CalculateRequest {
                file_path: Some(yaml.to_str().unwrap().into()),
                content: None,
                includes: None,
                dry_run: true,
                scenario: None,
                plan: false,
            }))));
            let profit = back["scalars"]["year.profit"].as_f64().unwrap();
            assert!(close(profit, 786.320_361_7, 1e-6), "{name}: {profit}");
        }
    }

    /// examples/inherited-budget (`inherited_budget` in `cli/commands/excel_io`):
    /// the workbook Forge did not write, imported by the tool, calculates to the
    /// net income `LibreOffice` stored in it, and the tool names the three things
    /// the import did not find as Forge holds them.
    #[test]
    fn import_reads_the_inherited_workbook_to_its_own_net_income() {
        let server = ForgeMcpServer::new();
        let dir = TempDir::new().unwrap();
        let yaml = dir.path().join("budget.yaml");
        let imported = json(&ok_text(server.import(Parameters(ImportRequest {
            excel_path: "examples/inherited-budget/budget.xlsx".into(),
            yaml_path: yaml.to_str().unwrap().into(),
        }))));
        assert_eq!(imported["warnings"].as_array().unwrap().len(), 3);
        let back = json(&ok_text(server.calculate(Parameters(CalculateRequest {
            file_path: Some(yaml.to_str().unwrap().into()),
            content: None,
            includes: None,
            dry_run: true,
            scenario: None,
            plan: false,
        }))));
        let net = back["scalars"]["summary.net_income"].as_f64().unwrap();
        assert!(close(net, 656_435.464_438_365, 1e-9), "{net}");
    }

    /// examples/acquisition (`the_plan_matches_the_independent_calculation` in
    /// cli/commands/prediction/scenarios.rs, the Python is quoted there): the plan
    /// reads Valdane and Quillon from two included files.
    #[test]
    fn calculate_gives_the_value_the_takeover_creates_by_path_and_inline() {
        for inline in [false, true] {
            let (file_path, content, includes) = example_source(
                inline,
                "acquisition",
                "deal.yaml",
                &[("buyer", "buyer.yaml"), ("target", "target.yaml")],
            );
            let result = json(&ok_text(ForgeMcpServer::new().calculate(Parameters(
                CalculateRequest {
                    file_path,
                    content,
                    includes,
                    dry_run: true,
                    scenario: None,
                    plan: false,
                },
            ))));
            let value = result["scalars"]["valuation.value_created"]
                .as_f64()
                .unwrap();
            assert!(
                close(value, 224.507_933_418_290_12, 1e-9),
                "inline {inline}: {value}"
            );
        }
    }

    /// What every tool that reads a model says when it is given none
    const ASKS: &str = "Either file_path or content is required";

    fn request<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Parameters<T> {
        Parameters(serde_json::from_value(value).expect("a request"))
    }

    #[test]
    fn calculate_audit_and_export_ask_for_a_model_when_they_get_none() {
        use serde_json::json;
        let server = ForgeMcpServer::new();
        assert_eq!(server.calculate(request(json!({}))).unwrap_err(), ASKS);
        assert_eq!(
            server.audit(request(json!({"variable": "x"}))).unwrap_err(),
            ASKS
        );
        assert_eq!(server.export(request(json!({}))).unwrap_err(), ASKS);
    }

    #[test]
    fn edit_answers_the_new_text_and_writes_only_a_file_it_was_given() {
        let server = ForgeMcpServer::new();
        let text = "_forge_version: \"5.1.0\"\nyear:\n  price: 16 # p\n  revenue: {formula: \"=year.price * 2\"}\n";
        let changes: Vec<crate::edit::Change> = serde_json::from_value(serde_json::json!([
            {"op": "set_value", "path": "year.price", "value": 17},
            {"op": "set_formula", "path": "year.revenue", "formula": "=year.price * 3"}
        ]))
        .unwrap();
        let request = |file_path: Option<String>, content: Option<String>, dry_run| EditRequest {
            file_path,
            content,
            includes: None,
            changes: changes.clone(),
            dry_run,
        };
        let want = text.replace("16", "17").replace("* 2", "* 3");
        let inline = json(&ok_text(server.edit(Parameters(request(
            None,
            Some(text.into()),
            false,
        )))));
        assert_eq!(
            inline,
            serde_json::json!({"files": {"model.yaml": want}, "written": false})
        );

        let dir = TempDir::new().unwrap();
        let file = dir.path().join("m.yaml");
        std::fs::write(&file, text).unwrap();
        let path = Some(file.to_string_lossy().into_owned());
        let dry = json(&ok_text(server.edit(Parameters(request(
            path.clone(),
            None,
            true,
        )))));
        assert_eq!(dry["written"], false);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), text);
        let wrote = json(&ok_text(
            server.edit(Parameters(request(path, None, false))),
        ));
        assert_eq!(wrote["files"]["m.yaml"], want);
        assert_eq!(wrote["written"], true);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), want);

        let refused = err_text(
            server.edit(Parameters(EditRequest {
                changes: serde_json::from_value(
                    serde_json::json!([{"op": "delete", "path": "nothing"}]),
                )
                .unwrap(),
                ..request(None, Some(text.into()), false)
            })),
        );
        assert_eq!(
            refused,
            "Edit failed: Validation error: delete nothing: it is not in the model (model.yaml). No file was changed"
        );
        assert!(
            err_text(server.edit(Parameters(request(None, None, false))))
                .contains("Either file_path or content")
        );
    }

    const PRICED: &str =
        "_forge_version: \"5.1.0\"\nprice: 4\nout:\n  total: {formula: \"=price * 2\"}\n";

    #[test]
    fn a_tool_that_fails_says_which_and_why() {
        let server = ForgeMcpServer::new();
        let content = Some(PRICED.to_string());
        assert_eq!(
            err_text(server.audit(Parameters(AuditRequest {
                file_path: None,
                content: content.clone(),
                includes: None,
                variable: "nope".into(),
            }))),
            "Audit failed: Validation error: 'nope' is not a scalar or a table column of the model.\n  Scalars: price, out.total\n  Columns: "
        );
        let excel_path = Some("/nonexistent-folder/x.xlsx".to_string());
        assert_eq!(
            err_text(server.export(Parameters(ExportRequest {
                yaml_path: None,
                content,
                includes: None,
                excel_path,
            }))),
            "Export failed: IO error: Failed to save Excel file: No such file or directory (os error 2)"
        );
        // A table too wide for a worksheet is refused in memory too
        let columns: Vec<String> = (0..=16_384).map(|i| format!("  c{i}: [1]")).collect();
        let wide = format!("_forge_version: \"5.2.0\"\nt:\n{}\n", columns.join("\n"));
        assert_eq!(
            err_text(server.export(Parameters(ExportRequest {
                yaml_path: None,
                content: Some(wide),
                includes: None,
                excel_path: None,
            }))),
            "Export failed: Excel export error: t: 16385 columns do not fit in an Excel worksheet, which holds 16384"
        );
        assert_eq!(
            err_text(server.import(Parameters(ImportRequest {
                excel_path: "nowhere.xlsx".into(),
                yaml_path: "out.yaml".into(),
            }))),
            "Import failed: IO error: Failed to open Excel file: I/O error: No such file or directory (os error 2)"
        );
    }
}
