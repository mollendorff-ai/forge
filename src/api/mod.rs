//! Forge API Server module (v2.0.0)
//!
//! Provides an HTTP REST API for integration with other systems.
//! Run with `forge serve`.

pub mod handlers;
pub mod server;

pub use server::run_api_server;

#[cfg(test)]
mod security_tests;

#[cfg(test)]
mod inline_tests;
