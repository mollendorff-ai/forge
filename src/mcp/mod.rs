//! Forge MCP Server (v3.0.0)
//!
//! Model Context Protocol server for AI-Finance integration.
//! Enables Claude Code, Cursor, GitHub Copilot, and other AI tools
//! to interact with Forge models programmatically.
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
//! ### Financial Analysis Tools (v3.0.0)
//! - `forge_sensitivity` - What-if analysis (1D/2D data tables)
//! - `forge_goal_seek` - Find input value for target output
//! - `forge_break_even` - Find where output = 0
//! - `forge_variance` - Budget vs actual analysis
//! - `forge_compare` - Multi-scenario comparison
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
