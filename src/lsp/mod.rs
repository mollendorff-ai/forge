//! LSP Server for Forge YAML files
//!
//! Provides Language Server Protocol support for:
//! - Real-time formula validation (diagnostics)
//! - Hover to see calculated values
//! - Autocomplete for variables and 50+ functions
//! - Go to definition for variable references
//! - Function signature hints
//!
//! This single LSP server powers ALL editor extensions:
//! VSCode, Zed, vim, emacs, JetBrains, etc.

pub mod capabilities;
pub mod document;
pub mod server;

pub use server::run_lsp_server;
pub use server::ForgeLsp;
