//! Forge YAML Extension for Zed
//!
//! Provides language support for Forge YAML formula files:
//! - Real-time formula validation
//! - Autocomplete for variables and 60+ Excel functions
//! - Hover to see calculated values
//! - Go to definition
//!
//! Requires: forge-lsp in PATH (install via `cargo install royalbit-forge`)

use zed_extension_api::{self as zed, LanguageServerId, Result};

struct ForgeExtension;

impl zed::Extension for ForgeExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        // Check for forge-lsp in worktree's PATH or system PATH
        let path = worktree
            .which("forge-lsp")
            .ok_or_else(|| "forge-lsp not found in PATH. Install with: cargo install royalbit-forge".to_string())?;

        Ok(zed::Command {
            command: path,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(ForgeExtension);
