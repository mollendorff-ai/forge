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

/// A client of the MCP server for a test, as an agent is of `forge mcp`: the
/// server runs on a thread of its own on an in-memory connection
/// ([`serve_on`]), and the session has initialized it.
#[cfg(test)]
pub struct Session {
    runtime: tokio::runtime::Runtime,
    to_server: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    from_server:
        tokio::io::Lines<tokio::io::BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>>,
    server: std::thread::JoinHandle<crate::error::ForgeResult<()>>,
}

#[cfg(test)]
impl Session {
    /// How long to wait for an answer: a server that died or hangs fails the
    /// test instead of hanging it.
    const ANSWER_TIMEOUT: std::time::Duration = std::time::Duration::from_mins(1);

    /// A server with file paths kept inside `root`, if one is given, once it
    /// has answered `initialize`.
    ///
    /// # Panics
    ///
    /// Panics if the runtime cannot start or the server does not answer.
    #[must_use]
    pub fn start(root: Option<&std::path::Path>) -> Self {
        let (client, server_side) = tokio::io::duplex(1 << 20);
        let root = root.map(std::path::Path::to_path_buf);
        let server =
            std::thread::spawn(move || serve_on(root.as_deref(), tokio::io::split(server_side)));
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime");
        let (read, write) = tokio::io::split(client);
        let mut session = Self {
            runtime,
            to_server: write,
            from_server: tokio::io::AsyncBufReadExt::lines(tokio::io::BufReader::new(read)),
            server,
        };
        session.send(
            &serde_json::json!({"jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {"protocolVersion": "2025-06-18", "capabilities": {},
                       "clientInfo": {"name": "test", "version": "0"}}}),
        );
        let _ = session.answer(0);
        session.send(&serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        session
    }

    /// Send one JSON-RPC message.
    ///
    /// # Panics
    ///
    /// Panics if the server has closed the connection.
    pub fn send(&mut self, message: &serde_json::Value) {
        let line = format!("{message}\n");
        self.runtime
            .block_on(tokio::io::AsyncWriteExt::write_all(
                &mut self.to_server,
                line.as_bytes(),
            ))
            .expect("the server reads");
    }

    /// The server's answer to the request `id`, as the line it wrote.
    ///
    /// # Panics
    ///
    /// Panics if the server closes the connection, or does not answer within
    /// a minute.
    #[must_use]
    pub fn answer(&mut self, id: u64) -> String {
        loop {
            let lines = &mut self.from_server;
            let line = self
                .runtime
                .block_on(async move {
                    tokio::time::timeout(Self::ANSWER_TIMEOUT, lines.next_line()).await
                })
                .expect("the server answered in time")
                .expect("the connection reads")
                .expect("the server is still serving");
            let message: serde_json::Value = serde_json::from_str(&line).expect("JSON");
            if message["id"] == id {
                return line;
            }
        }
    }

    /// Call `tool` with `arguments` as request `id`: the server's answer.
    #[must_use]
    pub fn call(&mut self, id: u64, tool: &str, arguments: &serde_json::Value) -> String {
        self.send(
            &serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": tool, "arguments": arguments}}),
        );
        self.answer(id)
    }

    /// Whether the server is still serving.
    #[must_use]
    pub fn is_serving(&self) -> bool {
        !self.server.is_finished()
    }
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

    mod root {
        //! `forge mcp` keeps file paths inside the folder it starts in unless told
        //! otherwise (evaluation of rc.27, fix 7): an agent, or a prompt injected
        //! into one, cannot name any file the user can reach. `--any-path` lifts the
        //! limit, and `--root DIR` names another folder.
        //!
        //! The server is given the root `forge mcp` gives it: the folder it starts
        //! in, none for `--any-path`, or the one `--root` names (`McpArgs::root`,
        //! tested in `src/cli/run.rs`).

        use super::Session;
        use std::path::Path;

        const MODEL: &str = r#"_forge_version: "5.4.0"
inputs:
  price: 4
out:
  total:
    formula: "=inputs.price * 2"
"#;

        /// Part of the refusal for a path outside the folder (`src/path_root.rs`)
        const REFUSED: &str = "may use";

        /// What the server answers to `forge_validate` on `file`, its paths kept
        /// inside `root` if one is given
        fn validate(root: Option<&Path>, file: &Path) -> String {
            Session::start(root).call(
                2,
                "forge_validate",
                &serde_json::json!({"file_path": file.display().to_string()}),
            )
        }

        fn folders() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
            let top = tempfile::tempdir().expect("temp dir");
            let inside = top.path().join("here");
            let outside = top.path().join("elsewhere");
            std::fs::create_dir(&inside).unwrap();
            std::fs::create_dir(&outside).unwrap();
            std::fs::write(inside.join("model.yaml"), MODEL).unwrap();
            std::fs::write(outside.join("model.yaml"), MODEL).unwrap();
            (top, inside, outside)
        }

        #[test]
        fn by_default_a_path_outside_the_starting_folder_is_refused() {
            let (_top, inside, outside) = folders();
            let answer = validate(Some(&inside), &outside.join("model.yaml"));
            assert!(answer.contains(REFUSED), "{answer}");
            let answer = validate(Some(&inside), &inside.join("model.yaml"));
            assert!(!answer.contains(REFUSED), "{answer}");
        }

        #[test]
        fn any_path_lifts_the_limit() {
            let (_top, _inside, outside) = folders();
            let answer = validate(None, &outside.join("model.yaml"));
            assert!(!answer.is_empty() && !answer.contains(REFUSED), "{answer}");
        }

        #[test]
        fn root_names_another_folder() {
            let (_top, inside, outside) = folders();
            let answer = validate(Some(&outside), &outside.join("model.yaml"));
            assert!(!answer.is_empty() && !answer.contains(REFUSED), "{answer}");
            let answer = validate(Some(&outside), &inside.join("model.yaml"));
            assert!(answer.contains(REFUSED), "{answer}");
        }

        #[test]
        fn root_and_any_path_cannot_be_given_together() {
            let ran = crate::cli::forge(&["mcp", "--any-path", "--root", "."]);
            assert_eq!(ran.code, 2, "{ran:?}");
            assert!(ran.err.contains("cannot be used with"), "{}", ran.err);
        }
    }
}
