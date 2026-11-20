//! Forge MCP Server implementation
//!
//! Provides the MCP server that AI agents use to interact with Forge.
//! Uses the rmcp SDK for Model Context Protocol over stdin/stdout.

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerConfig};
use rmcp::{tool_handler, ServerHandler};

mod analysis_tools;
mod model_tools;
mod reference_tools;
mod uncertainty_tools;

// ═══════════════════════════════════════════════════════════════════════════
// SERVER
// ═══════════════════════════════════════════════════════════════════════════

/// Forge MCP Server
#[derive(Debug, Clone)]
pub struct ForgeMcpServer {
    tool_router: ToolRouter<Self>,
    /// The folder file paths must stay inside (`forge mcp --root`), if any
    root: Option<crate::path_root::Root>,
}

impl ForgeMcpServer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_router: Self::model_tools()
                + Self::analysis_tools()
                + Self::uncertainty_tools()
                + Self::reference_tools(),
            root: None,
        }
    }

    /// Refuse any file path outside `root` (T1-MCP-ROOT).
    #[must_use]
    pub fn with_root(mut self, root: crate::path_root::Root) -> Self {
        self.root = Some(root);
        self
    }

    /// A file a tool reads or writes, checked against the root if there is one.
    fn file(&self, path: &str) -> Result<std::path::PathBuf, String> {
        self.root
            .as_ref()
            .map_or_else(|| Ok(path.into()), |root| root.file(path))
    }
}

impl Default for ForgeMcpServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialize a successful result to JSON string, numbers to 15 significant
/// digits (`core::precision`).
pub(super) fn to_json<T: serde::Serialize>(result: &T) -> String {
    serde_json::to_string(&crate::core::precision::Rounded(result)).unwrap_or_default()
}

/// What the server tells the agent it serves.
const INSTRUCTIONS: &str = "Forge MCP Server - 21 tools for AI-native financial modeling. Core: validate, calculate, audit, export, import. Analysis: sensitivity, goal-seek, break-even, variance, compare. Engines: simulate (Monte Carlo), scenarios, decision-tree, real-options, tornado, bootstrap, bayesian, game (equilibria and tournaments). Discovery: schema, functions, examples. 203 Excel-compatible functions. All tools return structured JSON. A new model: start from forge_examples name=starter; _forge_version is the model format version (5.1.0), not the Forge version; in a group of scalars a formula is {formula: \"=...\"}, in a table a column formula is a bare \"=...\".";

#[allow(unknown_lints, clippy::unused_async_trait_impl)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for ForgeMcpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(self.root.as_ref().map_or_else(
                || INSTRUCTIONS.to_string(),
                |root| {
                    format!(
                        "{INSTRUCTIONS} Files: only inside {}; a relative path is relative to it.",
                        root.dir().display()
                    )
                },
            ))
            .with_server_info(rmcp::model::Implementation::new(
                "forge",
                env!("CARGO_PKG_VERSION"),
            ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_server_is_the_new_one_without_a_root() {
        assert!(ForgeMcpServer::default().root.is_none());
    }

    /// Helper: get Ok text or panic with the error string
    pub fn ok_text(result: Result<String, String>) -> String {
        result.expect("expected Ok result")
    }

    /// Helper: get Err text or panic
    pub fn err_text(result: Result<String, String>) -> String {
        result.expect_err("expected Err result")
    }

    /// Minimal valid YAML model for inline content tests
    pub const INLINE_YAML: &str = r#"
_forge_version: "5.0.0"
scalars:
  revenue:
    value: 100000
  costs:
    value: 60000
  profit:
    value: 0
    formula: "=revenue - costs"
"#;

    /// A file of an example (`dir`/`file` under examples/), as the text a client
    /// sends as inline content.
    pub fn example(dir: &str, file: &str) -> String {
        std::fs::read_to_string(format!("examples/{dir}/{file}")).expect("an example file")
    }

    /// How a request names a model of an example: its `file_path`, or its
    /// `content` and the `includes` map with the text of each file it includes,
    /// directly or through another included file (`namespace`, `file`).
    pub type Source = (
        Option<String>,
        Option<String>,
        Option<std::collections::HashMap<String, String>>,
    );

    /// The model `dir`/`file` named by its path, or sent as text.
    pub fn example_source(
        inline: bool,
        dir: &str,
        file: &str,
        includes: &[(&str, &str)],
    ) -> Source {
        if inline {
            (
                None,
                Some(example(dir, file)),
                Some(
                    includes
                        .iter()
                        .map(|(namespace, included)| {
                            (namespace.to_string(), example(dir, included))
                        })
                        .collect(),
                ),
            )
        } else {
            (Some(format!("examples/{dir}/{file}")), None, None)
        }
    }

    /// The bagel model `file`, by path or as text.
    pub fn bagel_source(inline: bool, file: &str, includes: &[(&str, &str)]) -> Source {
        example_source(inline, "bagel", file, includes)
    }

    /// A tool's answer as JSON.
    pub fn json(text: &str) -> serde_json::Value {
        serde_json::from_str(text).expect("JSON")
    }

    /// Whether two numbers differ by at most `tolerance`.
    pub fn close(a: f64, b: f64, tolerance: f64) -> bool {
        (a - b).abs() <= tolerance
    }

    #[test]
    fn test_server_get_info() {
        let server = ForgeMcpServer::new();
        let info = server.get_info();
        assert_eq!(info.server_info.name, "forge");
        assert!(info.instructions.is_some());
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_tool_count() {
        let server = ForgeMcpServer::new();
        let tools = server.tool_router.list_all();
        assert_eq!(tools.len(), 21, "Expected 21 tools, got {}", tools.len());
    }

    #[test]
    fn test_tool_names() {
        let server = ForgeMcpServer::new();
        let tools = server.tool_router.list_all();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

        // Core tools
        assert!(names.contains(&"forge_validate"));
        assert!(names.contains(&"forge_calculate"));
        assert!(names.contains(&"forge_audit"));
        assert!(names.contains(&"forge_export"));
        assert!(names.contains(&"forge_import"));
        // Financial analysis tools
        assert!(names.contains(&"forge_sensitivity"));
        assert!(names.contains(&"forge_goal_seek"));
        assert!(names.contains(&"forge_break_even"));
        assert!(names.contains(&"forge_variance"));
        assert!(names.contains(&"forge_compare"));
        // Analysis engine tools
        assert!(names.contains(&"forge_simulate"));
        assert!(names.contains(&"forge_scenarios"));
        assert!(names.contains(&"forge_decision_tree"));
        assert!(names.contains(&"forge_real_options"));
        assert!(names.contains(&"forge_tornado"));
        assert!(names.contains(&"forge_bootstrap"));
        assert!(names.contains(&"forge_bayesian"));
        assert!(names.contains(&"forge_game"));
        // Discovery tools
        assert!(names.contains(&"forge_schema"));
        assert!(names.contains(&"forge_functions"));
        assert!(names.contains(&"forge_examples"));
    }

    #[test]
    fn test_tool_schemas_are_objects() {
        let server = ForgeMcpServer::new();
        let tools = server.tool_router.list_all();

        for tool in &tools {
            assert_eq!(
                tool.input_schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "Tool {} schema missing type: object",
                tool.name
            );
        }
    }
}

#[cfg(all(test, unix))] // the tests use symbolic links
mod root_tests {
    //! `forge mcp --root DIR` (T1-MCP-ROOT): a path outside DIR is refused, for
    //! reading and for writing, through `..`, a symbolic link or an included file.
    //! Without a root nothing changes.
    use crate::mcp::types::{ExportRequest, ImportRequest, ValidateRequest, VarianceRequest};

    use super::*;
    use crate::path_root::Root;
    use rmcp::handler::server::wrapper::Parameters;
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    const MODEL: &str = "_forge_version: \"5.1.0\"\na:\n  x: 2\n  y: {formula: \"=a.x * 3\"}\n";

    /// `models/` is the root; `secret/` is beside it, outside.
    struct Folders {
        _base: TempDir,
        root: PathBuf,
        secret: PathBuf,
    }

    fn folders() -> Folders {
        let base = TempDir::new().unwrap();
        let root = base.path().join("models");
        let secret = base.path().join("secret");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&secret).unwrap();
        fs::write(root.join("m.yaml"), MODEL).unwrap();
        fs::write(secret.join("s.yaml"), MODEL).unwrap();
        // A link to a file outside, and a link to the folder outside
        symlink(secret.join("s.yaml"), root.join("link.yaml")).unwrap();
        symlink(&secret, root.join("out")).unwrap();
        // A model inside that includes a file outside
        fs::write(
            root.join("inc.yaml"),
            format!("{MODEL}_includes:\n  - file: ../secret/s.yaml\n    as: s\n"),
        )
        .unwrap();
        Folders {
            root,
            secret,
            _base: base,
        }
    }

    fn server(f: &Folders) -> ForgeMcpServer {
        ForgeMcpServer::new().with_root(Root::new(&f.root).unwrap())
    }

    fn s(path: &Path) -> String {
        path.to_str().unwrap().to_string()
    }

    fn validate(server: &ForgeMcpServer, path: &str) -> Result<String, String> {
        server.validate(Parameters(ValidateRequest {
            file_path: Some(path.to_string()),
            content: None,
            includes: None,
            verbose: false,
        }))
    }

    fn export(server: &ForgeMcpServer, yaml: &str, excel: &str) -> Result<String, String> {
        server.export(Parameters(ExportRequest {
            yaml_path: Some(yaml.to_string()),
            content: None,
            includes: None,
            excel_path: Some(excel.to_string()),
        }))
    }

    fn refused(result: Result<String, String>) -> String {
        let err = result.expect_err("a path outside the root must be refused");
        assert!(err.contains("outside"), "says why: {err}");
        err
    }

    #[test]
    fn a_model_inside_the_root_is_read_by_full_or_relative_path() {
        let f = folders();
        let server = server(&f);
        validate(&server, &s(&f.root.join("m.yaml"))).unwrap();
        validate(&server, "m.yaml").unwrap();
    }

    #[test]
    fn a_model_outside_the_root_is_refused() {
        let f = folders();
        let err = refused(validate(&server(&f), &s(&f.secret.join("s.yaml"))));
        assert!(err.contains("--root"), "names the setting: {err}");
    }

    #[test]
    fn dot_dot_cannot_leave_the_root() {
        let f = folders();
        let server = server(&f);
        refused(validate(&server, "../secret/s.yaml"));
        refused(validate(&server, &s(&f.root.join("../secret/s.yaml"))));
        // Through a folder that does not exist, too
        refused(validate(&server, "missing/../../secret/s.yaml"));
    }

    #[test]
    fn a_symbolic_link_cannot_leave_the_root() {
        let f = folders();
        let server = server(&f);
        refused(validate(&server, "link.yaml"));
        refused(validate(&server, "out/s.yaml"));
    }

    #[test]
    fn an_included_file_outside_the_root_is_refused() {
        let f = folders();
        let err = refused(validate(&server(&f), "inc.yaml"));
        assert!(err.contains("s.yaml"), "names the included file: {err}");
    }

    #[test]
    fn export_cannot_write_outside_the_root() {
        let f = folders();
        let server = server(&f);
        refused(export(&server, "m.yaml", &s(&f.secret.join("x.xlsx"))));
        refused(export(&server, "m.yaml", "../secret/x.xlsx"));
        refused(export(&server, "m.yaml", "out/x.xlsx"));
        assert!(!f.secret.join("x.xlsx").exists(), "nothing written outside");
        export(&server, "m.yaml", "m.xlsx").unwrap();
        assert!(f.root.join("m.xlsx").exists(), "relative to the root");
    }

    #[test]
    fn a_link_where_the_file_will_be_written_cannot_leave_the_root() {
        let f = folders();
        // A link that points outside to a file that does not exist yet
        symlink(f.secret.join("new.xlsx"), f.root.join("dangling.xlsx")).unwrap();
        refused(export(&server(&f), "m.yaml", "dangling.xlsx"));
        assert!(!f.secret.join("new.xlsx").exists());
    }

    #[test]
    fn import_cannot_read_or_write_outside_the_root() {
        let f = folders();
        let server = server(&f);
        export(&server, "m.yaml", "m.xlsx").unwrap();
        fs::copy(f.root.join("m.xlsx"), f.secret.join("m.xlsx")).unwrap();
        let import = |excel: &str, yaml: &str| {
            server.import(Parameters(ImportRequest {
                excel_path: excel.to_string(),
                yaml_path: yaml.to_string(),
            }))
        };
        refused(import(&s(&f.secret.join("m.xlsx")), "back.yaml"));
        refused(import("m.xlsx", &s(&f.secret.join("back.yaml"))));
        assert!(
            !f.secret.join("back.yaml").exists(),
            "nothing written outside"
        );
        import("m.xlsx", "back.yaml").unwrap();
        assert!(f.root.join("back.yaml").exists());
    }

    #[test]
    fn both_files_of_a_variance_are_checked() {
        let f = folders();
        let result = server(&f).variance(Parameters(VarianceRequest {
            budget_path: Some("m.yaml".into()),
            budget_content: None,
            actual_path: Some(s(&f.secret.join("s.yaml"))),
            actual_content: None,
            includes: None,
            threshold: None,
        }));
        refused(result);
    }

    #[test]
    fn inline_content_still_works_with_a_root() {
        let f = folders();
        server(&f)
            .validate(Parameters(ValidateRequest {
                file_path: None,
                content: Some(MODEL.into()),
                includes: None,
                verbose: false,
            }))
            .unwrap();
    }

    #[test]
    fn without_a_root_nothing_changes() {
        let f = folders();
        validate(&ForgeMcpServer::new(), &s(&f.secret.join("s.yaml"))).unwrap();
    }

    #[test]
    fn the_server_tells_the_agent_where_files_must_be() {
        let f = folders();
        let info = server(&f).get_info();
        let text = info.instructions.unwrap_or_default();
        let root = f.root.canonicalize().unwrap();
        assert!(text.contains(&s(&root)), "{text}");
    }

    #[test]
    fn a_root_that_is_not_a_folder_is_refused_at_start() {
        let f = folders();
        assert!(Root::new(&f.root.join("m.yaml")).is_err());
        assert!(Root::new(&f.root.join("missing")).is_err());
    }

    #[test]
    fn an_include_of_an_include_outside_the_root_is_refused() {
        let f = folders();
        fs::write(
            f.root.join("outer.yaml"),
            format!("{MODEL}_includes:\n  - file: inc.yaml\n    as: inner\n"),
        )
        .unwrap();
        refused(validate(&server(&f), "outer.yaml"));
    }

    #[test]
    fn a_document_of_a_multi_document_model_cannot_include_outside() {
        let f = folders();
        let second = "_forge_version: \"5.1.0\"\n_includes:\n  - file: ../secret/s.yaml\n    as: s\nb:\n  z: 1\n";
        fs::write(
            f.root.join("multi.yaml"),
            format!("---\n{MODEL}---\n{second}"),
        )
        .unwrap();
        refused(validate(&server(&f), "multi.yaml"));
    }
}
