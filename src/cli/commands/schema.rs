//! Schema command - display JSON schemas for Forge model formats

use crate::error::ForgeResult;
use colored::Colorize;

/// Embedded JSON schemas (compile-time)
const SCHEMA_V1: &str = include_str!("../../../schema/forge-v1.0.0.schema.json");
const SCHEMA_V5: &str = include_str!("../../../schema/forge-v5.0.0.schema.json");

/// Display JSON schema for Forge model formats
pub fn schema(version: Option<String>, list: bool) -> ForgeResult<()> {
    if list || version.is_none() {
        print_schema_list();
        return Ok(());
    }

    let schema = match version.as_deref() {
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
        let result = schema(Some("v1".to_string()), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_v5_output() {
        let result = schema(Some("v5".to_string()), false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_version_aliases() {
        // All these should work for v1
        for alias in ["v1", "v1.0.0", "1", "1.0.0"] {
            let result = schema(Some(alias.to_owned()), false);
            assert!(result.is_ok(), "Alias '{alias}' should work for v1");
        }

        // All these should work for v5
        for alias in ["v5", "v5.0.0", "5", "5.0.0"] {
            let result = schema(Some(alias.to_owned()), false);
            assert!(result.is_ok(), "Alias '{alias}' should work for v5");
        }
    }
}
