//! Schema command - display JSON schemas for Forge model formats

use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;

/// Embedded JSON schemas (compile-time)
const SCHEMA_V1: &str = include_str!("../../../schema/forge-v1.0.0.schema.json");
const SCHEMA_V5: &str = include_str!("../../../schema/forge-v5.0.0.schema.json");
const SCHEMA_V5_1: &str = include_str!("../../../schema/forge-v5.1.0.schema.json");
const SCHEMA_V5_2: &str = include_str!("../../../schema/forge-v5.2.0.schema.json");
const SCHEMA_V5_3: &str = include_str!("../../../schema/forge-v5.3.0.schema.json");

/// Return a JSON schema as a string (no printing).
///
/// # Errors
///
/// Returns an error if an unsupported schema version is requested.
pub fn schema_core(version: Option<&str>) -> ForgeResult<String> {
    match version {
        Some("v1" | "v1.0.0" | "1" | "1.0.0") => Ok(SCHEMA_V1.to_string()),
        Some("v5" | "v5.0.0" | "5" | "5.0.0") => Ok(SCHEMA_V5.to_string()),
        Some("v5.1" | "v5.1.0" | "5.1" | "5.1.0") => Ok(SCHEMA_V5_1.to_string()),
        Some("v5.2" | "v5.2.0" | "5.2" | "5.2.0") => Ok(SCHEMA_V5_2.to_string()),
        Some("v5.3" | "v5.3.0" | "5.3" | "5.3.0") => Ok(SCHEMA_V5_3.to_string()),
        Some(v) => Err(ForgeError::Validation(format!(
            "Unknown model format '{v}'. Use 'v1', 'v5', 'v5.1', 'v5.2' or 'v5.3'."
        ))),
        None => {
            // Return list of available versions as JSON
            Ok(serde_json::json!({
                "available_versions": [
                    {"version": "v1.0.0", "description": "Scalar-only models (simple key-value pairs)"},
                    {"version": "v5.0.0", "description": "Full feature set (arrays, tables, Monte Carlo, etc.)"},
                    {"version": "v5.1.0", "description": "5.0.0 plus the short form (`price: 16`, `{formula: ...}` without nulls); use this for new models"},
                    {"version": "v5.2.0", "description": "5.1.0 plus the period column (`month: {start: 2026-12, count: 300, unit: month}`); needed only for a table with one"},
                    {"version": "v5.3.0", "description": "5.2.0 plus the empty cell (`actual: [985, 1032, null]`, ADR-059); needed only for a column with one"}
                ],
                "note": "These are model format versions (_forge_version), not Forge tool versions"
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
        Some("v5.1" | "v5.1.0" | "5.1" | "5.1.0") => SCHEMA_V5_1,
        Some("v5.2" | "v5.2.0" | "5.2" | "5.2.0") => SCHEMA_V5_2,
        Some("v5.3" | "v5.3.0" | "5.3" | "5.3.0") => SCHEMA_V5_3,
        Some(v) => {
            eprintln!(
                "{}: Unknown model format '{}'. Use 'v1', 'v5', 'v5.1', 'v5.2' or 'v5.3'.",
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
    println!("{}", "Forge Model Formats (_forge_version)".bold().green());
    println!("These are versions of the model file format, not of Forge.");
    println!();
    println!(
        "  {}  Scalar-only models (simple key-value pairs)",
        "v1.0.0".cyan().bold()
    );
    println!("         - Named variables with value/formula/metadata");
    println!("         - Scenarios for what-if analysis");
    println!("         - No arrays or tables");
    println!();
    println!("  {}  Full feature set", "v5.0.0".cyan().bold());
    println!("         - Arrays and tables with row-wise formulas");
    println!("         - inputs/outputs separation");
    println!("         - Monte Carlo, Decision Trees, Real Options");
    println!("         - Cross-file includes");
    println!();
    println!(
        "  {}  5.0.0 plus the short form (use for new models)",
        "v5.1.0".cyan().bold()
    );
    println!("         - Inputs as plain numbers: price: 16");
    println!("         - Formulas without value: null: {{formula: \"=a * b\"}}");
    println!("         - forge calculate writes each result next to its formula");
    println!("         - forge upgrade converts a 5.0.0 model, keeping comments");
    println!();
    println!(
        "  {}  5.1.0 plus the period column (ADR-053)",
        "v5.2.0".cyan().bold()
    );
    println!("         - month: {{start: 2026-12, count: 300, unit: month}}");
    println!("         - Needed only for a table with a period column");
    println!();
    println!(
        "  {}  5.2.0 plus the empty cell (ADR-059)",
        "v5.3.0".cyan().bold()
    );
    println!("         - actual: [985, 1032, null]: aggregates skip it, formulas read 0");
    println!("         - Needed only for a column with an empty cell");
    println!();
    println!(
        "Usage: {}, {}, {}, {} or {}",
        "forge schema v1".yellow(),
        "forge schema v5".yellow(),
        "forge schema v5.1".yellow(),
        "forge schema v5.2".yellow(),
        "forge schema v5.3".yellow()
    );
    println!();
    println!(
        "Pipe to file: {}",
        "forge schema v5.1 > schema.json".dimmed()
    );
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
    fn test_schema_v5_1_is_valid_json_and_listed() {
        let parsed: serde_json::Value = serde_json::from_str(SCHEMA_V5_1).unwrap();
        assert_eq!(parsed["properties"]["_forge_version"]["enum"][0], "5.1.0");
        assert!(schema_core(None).unwrap().contains("v5.1.0"));
        assert_eq!(schema_core(Some("v5.1")).unwrap(), SCHEMA_V5_1);
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
