//! Forge API Server binary
//!
//! Enterprise HTTP REST API for Forge.
//! Provides validate, calculate, audit, export, import endpoints.

use clap::Parser;
use royalbit_forge::api::{run_api_server, server::ApiConfig};

#[derive(Parser, Debug)]
#[command(name = "forge-server")]
#[command(version)]
#[command(author = "RoyalBit Inc. <admin@royalbit.ca>")]
#[command(about = "Forge API Server - Enterprise HTTP REST API for YAML formula calculations")]
#[command(long_about = r#"
Forge API Server - Enterprise HTTP REST API

Provides RESTful endpoints for all Forge operations:
  - POST /api/v1/validate  - Validate YAML model files
  - POST /api/v1/calculate - Calculate formulas (with dry-run support)
  - POST /api/v1/audit     - Audit variable dependency trees
  - POST /api/v1/export    - Export YAML to Excel (.xlsx)
  - POST /api/v1/import    - Import Excel to YAML

Additional endpoints:
  - GET  /health           - Health check
  - GET  /version          - Server version info
  - GET  /                  - API documentation

Features:
  - CORS enabled for cross-origin requests
  - Graceful shutdown on SIGINT/SIGTERM
  - JSON response format with request IDs
  - Tracing and structured logging

Example usage:
  forge-server                           # Start on localhost:8080
  forge-server --host 0.0.0.0 --port 3000

  curl -X POST http://localhost:8080/api/v1/validate \
    -H "Content-Type: application/json" \
    -d '{"file_path": "model.yaml"}'
"#)]
struct Args {
    /// Host address to bind to (use 0.0.0.0 for all interfaces)
    #[arg(short = 'H', long, default_value = "127.0.0.1", env = "FORGE_HOST")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value = "8080", env = "FORGE_PORT")]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config = ApiConfig {
        host: args.host,
        port: args.port,
    };

    run_api_server(config).await
}
