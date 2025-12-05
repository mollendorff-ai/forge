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
//!
//! # Coverage Exclusion (ADR-006)
//! Binary entry point - reads stdin forever. Library code tested in mcp/server.rs

// During coverage builds, stubbed main doesn't use imports
#![cfg_attr(coverage, allow(unused_imports))]

use royalbit_forge::mcp::run_mcp_server_sync;

/// Binary entry point - excluded from coverage (ADR-006)
/// Thin wrapper that calls library function. Cannot unit test binary main().
#[cfg(not(coverage))]
fn main() {
    run_mcp_server_sync();
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
fn main() {}
