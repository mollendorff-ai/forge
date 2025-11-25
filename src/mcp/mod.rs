//! Forge MCP Server (v1.7.0)
//!
//! Model Context Protocol server for AI agent integration.
//! Enables Claude Code, Cursor, GitHub Copilot, and other AI tools
//! to interact with Forge models programmatically.
//!
//! ## Features
//! - `forge_validate` - Validate YAML model files for formula errors
//! - `forge_calculate` - Calculate formulas and update values
//! - `forge_audit` - Get dependency tree and value tracing
//! - `forge_export` - Export YAML to Excel
//! - `forge_import` - Import Excel to YAML
//!
//! ## Usage
//!
//! Configure in Claude Code settings:
//! ```json
//! {
//!   "mcpServers": {
//!     "forge": {
//!       "command": "forge-mcp"
//!     }
//!   }
//! }
//! ```

pub mod server;

pub use server::run_mcp_server_sync;
pub use server::ForgeMcpServer;
