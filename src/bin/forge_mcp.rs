//! Forge MCP Server binary
//!
//! Model Context Protocol server for AI agent integration.
//! Run with: `forge-mcp`
//!
//! Configure in Claude Code or other MCP clients:
//! ```json
//! {
//!   "mcpServers": {
//!     "forge": {
//!       "command": "forge-mcp"
//!     }
//!   }
//! }
//! ```

use royalbit_forge::mcp::run_mcp_server_sync;

fn main() {
    run_mcp_server_sync();
}
