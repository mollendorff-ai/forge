//! Forge Language Server
//!
//! Run with: forge-lsp
//!
//! This LSP server provides IDE features for Forge YAML files:
//! - Real-time formula validation
//! - Hover to see calculated values
//! - Autocomplete for variables and 50+ functions
//! - Go to definition
//! - Error diagnostics

use royalbit_forge::lsp::run_lsp_server;

#[tokio::main]
async fn main() {
    run_lsp_server().await;
}
