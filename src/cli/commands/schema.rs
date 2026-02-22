//! Schema command - display JSON schemas for Forge model formats

use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;

/// Embedded JSON schemas (compile-time)
const SCHEMA_V1: &str = include_str!("../../../schema/forge-v1.0.0.schema.json");
const SCHEMA_V5: &str = include_str!("../../../schema/forge-v5.0.0.schema.json");

/// Return a JSON schema as a string (no printing).
///
/// # Errors
///
/// Returns an error if an unsupported schema version is requested.
pub fn schema_core(version: Option<&str>) -> ForgeResult<String> {
    match version {
        Some("v1" | "v1.0.0" | "1" | "1.0.0") => Ok(SCHEMA_V1.to_string()),
        Some("v5" | "v5.0.0" | "5" | "5.0.0") => Ok(SCHEMA_V5.to_string()),
        Some(v) => Err(ForgeError::Validation(format!(
            "Unknown schema version '{v}'. Use 'v1' or 'v5'."
        ))),
        None => {
            // Return list of available versions as JSON
            Ok(serde_json::json!({
                "available_versions": [
                    {"version": "v1.0.0", "description": "Scalar-only models (simple key-value pairs)"},
                    {"version": "v5.0.0", "description": "Full enterprise support (arrays, tables, Monte Carlo, etc.)"}
                ]
            })
            .to_string())
        },
    }
}

/// Display JSON schema for Forge model formats.
///
/// # Errors
///
/// Returns an error if an unsupported schema version is requested.
pub fn schema(version: Option<&str>, list: bool) -> ForgeResult<()> {
    if list || version.is_none() {
        print_schema_list();
        return Ok(());
    }

    let schema = match version {
        Some("v1" | "v1.0.0" | "1" | "1.0.0") => SCHEMA_V1,
        Some("v5" | "v5.0.0" | "5" | "5.0.0") => SCHEMA_V5,
        Some(v) => {
            eprintln!(
                "{}: Unknown schema version '{}'. Use 'v1' or 'v5'.",
                "Error".red().bold(),
                v
            );
            eprintln!();
            eprintln!(
                "Run {} to see available versions.",
                "forge schema --list".yellow()
            );
            std::process::exit(1);
        },
        None => {
            print_schema_list();
            return Ok(());
        },
    };

    println!("{schema}");
    Ok(())
}

/// Print list of available schema versions
fn print_schema_list() {
    println!("{}", "Available Forge Schema Versions".bold().green());
    println!();
    println!(
        "  {}  Scalar-only models (simple key-value pairs)",
        "v1.0.0".cyan().bold()
    );
    println!("         - Named variables with value/formula/metadata");
    println!("         - Scenarios for what-if analysis");
    println!("         - No arrays or tables");
    println!();
    println!("  {}  Full enterprise support", "v5.0.0".cyan().bold());
    println!("         - Arrays and tables with row-wise formulas");
    println!("         - inputs/outputs separation");
    println!("         - Monte Carlo, Decision Trees, Real Options");
    println!("         - Cross-file includes");
    println!();
    println!(
        "Usage: {} or {}",
        "forge schema v1".yellow(),
        "forge schema v5".yellow()
    );
    println!();
    println!("Pipe to file: {}", "forge schema v5 > schema.json".dimmed());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_v1_is_valid_json() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(SCHEMA_V1);
        assert!(parsed.is_ok(), "v1.0.0 schema should be valid JSON");
    }

    #[test]
    fn test_schema_v5_is_valid_json() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(SCHEMA_V5);
        assert!(parsed.is_ok(), "v5.0.0 schema should be valid JSON");
    }

    #[test]
    fn test_schema_list_mode() {
        // Just ensure it doesn't panic
        let result = schema(None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_v1_output() {
        let result = schema(Some("v1"), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_v5_output() {
        let result = schema(Some("v5"), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_version_aliases() {
        // All these should work for v1
        for alias in ["v1", "v1.0.0", "1", "1.0.0"] {
            let result = schema(Some(alias), false);
            assert!(result.is_ok(), "Alias '{alias}' should work for v1");
        }

        // All these should work for v5
        for alias in ["v5", "v5.0.0", "5", "5.0.0"] {
            let result = schema(Some(alias), false);
            assert!(result.is_ok(), "Alias '{alias}' should work for v5");
        }
    }
}
