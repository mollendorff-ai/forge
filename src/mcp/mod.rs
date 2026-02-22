//! Forge MCP Server (v10.0.0-beta.3)
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
//! - `forge_functions` - List 173 Excel-compatible functions
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
