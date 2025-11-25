//! Forge API Server binary
//!
//! Enterprise HTTP REST API for Forge.
//! Usage: forge-server [--host HOST] [--port PORT]

use clap::Parser;
use royalbit_forge::api::{run_api_server, server::ApiConfig};

#[derive(Parser, Debug)]
#[command(name = "forge-server")]
#[command(about = "Forge API Server - Enterprise HTTP REST API")]
#[command(version)]
struct Args {
    /// Host address to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    host: String,

    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
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
