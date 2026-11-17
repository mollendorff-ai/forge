//! Forge MCP Server
//!
//! Model Context Protocol server for AI-Finance integration.
//! Uses the rmcp SDK for typed tool definitions and transport abstraction.
//!
//! ## Features
//!
//! ### Core Tools
//! - `forge_validate` - Validate YAML model files for formula errors
//! - `forge_calculate` - Calculate formulas and update values
//! - `forge_audit` - Get dependency tree and value tracing
//! - `forge_export` - Export YAML to Excel
//! - `forge_import` - Import Excel to YAML
//!
//! ### Financial Analysis Tools
//! - `forge_sensitivity` - What-if analysis (1D/2D data tables)
//! - `forge_goal_seek` - Find input value for target output
//! - `forge_break_even` - Find where output = 0
//! - `forge_variance` - Budget vs actual analysis
//! - `forge_compare` - Multi-scenario comparison
//!
//! ### Analysis Engines
//! - `forge_simulate` - Monte Carlo simulation
//! - `forge_scenarios` - Probability-weighted scenario analysis
//! - `forge_decision_tree` - Decision tree analysis
//! - `forge_real_options` - Real options valuation
//! - `forge_tornado` - Tornado sensitivity diagrams
//! - `forge_bootstrap` - Bootstrap resampling
//! - `forge_bayesian` - Bayesian network inference
//!
//! ### Discovery Tools
//! - `forge_schema` - JSON Schema for model validation
//! - `forge_functions` - List 203 Excel-compatible functions
//! - `forge_examples` - Runnable YAML examples
//!
//! ## Usage
//!
//! Configure in Claude Code settings:
//! ```json
//! {
//!   "mcpServers": {
//!     "forge": {
//!       "command": "forge",
//!       "args": ["mcp"]
//!     }
//!   }
//! }
//! ```

pub mod server;
pub mod types;

pub use server::ForgeMcpServer;

/// Serve MCP over stdin and stdout until the client disconnects, with file
/// paths kept inside `root` if one is given (`forge mcp --root`).
///
/// # Errors
///
/// As [`serve_on`].
pub fn run_stdio(root: Option<&std::path::Path>) -> crate::error::ForgeResult<()> {
    serve_on(root, rmcp::transport::io::stdio())
}

/// Serve MCP on `transport`, a reader and a writer, until the client
/// disconnects, with file paths kept inside `root` if one is given.
///
/// # Errors
///
/// Returns an error if `root` is not a folder, the runtime cannot start or
/// the connection fails.
pub fn serve_on<R, W>(
    root: Option<&std::path::Path>,
    transport: (R, W),
) -> crate::error::ForgeResult<()>
where
    R: tokio::io::AsyncRead + Send + Unpin + 'static,
    W: tokio::io::AsyncWrite + Send + Unpin + 'static,
{
    use crate::error::ForgeError;
    use rmcp::ServiceExt;
    let mut server = ForgeMcpServer::new();
    if let Some(dir) = root {
        server =
            server.with_root(crate::path_root::Root::new(dir).map_err(ForgeError::Validation)?);
    }
    let failed =
        |e: &dyn std::fmt::Display| ForgeError::Validation(format!("MCP server error: {e}"));
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| ForgeError::Validation(format!("Failed to create runtime: {e}")))?;
    runtime.block_on(async {
        let service = server.serve(transport).await.map_err(|e| failed(&e))?;
        service.waiting().await.map_err(|e| failed(&e))?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    const INITIALIZE: &str = concat!(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#,
        "\n",
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
        "\n",
    );

    /// What the server writes back to a client that sends `input` and leaves.
    fn session(
        root: Option<&std::path::Path>,
        input: &str,
    ) -> (crate::error::ForgeResult<()>, String) {
        let (server_side, mut client_side) = tokio::io::duplex(1 << 20);
        let reader = std::io::Cursor::new(input.as_bytes().to_vec());
        let result = serve_on(root, (reader, server_side));
        let mut answer = String::new();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(client_side.read_to_string(&mut answer))
            .unwrap();
        (result, answer)
    }

    #[test]
    fn serve_on_answers_initialize_and_ends_when_the_client_leaves() {
        let dir = tempfile::tempdir().unwrap();
        let (result, answer) = session(Some(dir.path()), INITIALIZE);
        result.unwrap();
        let reply: serde_json::Value =
            serde_json::from_str(answer.lines().next().unwrap()).unwrap();
        assert_eq!(reply["id"], 1);
        assert_eq!(reply["result"]["serverInfo"]["name"], "forge");
    }

    #[test]
    fn serve_on_fails_when_the_client_leaves_before_initializing() {
        let (result, answer) = session(None, "");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("MCP server error"), "{err}");
        assert_eq!(answer, "");
    }

    #[test]
    fn serve_on_refuses_a_root_that_is_not_a_folder() {
        let (result, answer) = session(Some(std::path::Path::new("nowhere-at-all")), INITIALIZE);
        let err = result.unwrap_err().to_string();
        assert!(err.contains("--root nowhere-at-all"), "{err}");
        assert_eq!(answer, "");
    }
}
