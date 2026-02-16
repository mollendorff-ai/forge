//! Forge API Server module (v2.0.0)
//!
//! Provides HTTP REST API for enterprise integration.
//! Run with `forge serve`.

pub mod handlers;
pub mod server;

pub use server::run_api_server;
