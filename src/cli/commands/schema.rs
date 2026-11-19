//! Schema command - display JSON schemas for Forge model formats

use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;

/// Embedded JSON schemas (compile-time)
const SCHEMA_V1: &str = include_str!("../../../schema/forge-v1.0.0.schema.json");
const SCHEMA_V5: &str = include_str!("../../../schema/forge-v5.0.0.schema.json");
const SCHEMA_V5_1: &str = include_str!("../../../schema/forge-v5.1.0.schema.json");
const SCHEMA_V5_2: &str = include_str!("../../../schema/forge-v5.2.0.schema.json");
const SCHEMA_V5_3: &str = include_str!("../../../schema/forge-v5.3.0.schema.json");
const SCHEMA_V5_4: &str = include_str!("../../../schema/forge-v5.4.0.schema.json");

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
        Some("v5.4" | "v5.4.0" | "5.4" | "5.4.0") => Ok(SCHEMA_V5_4.to_string()),
        Some(v) => Err(ForgeError::Validation(format!(
            "Unknown model format '{v}'. Use 'v1', 'v5', 'v5.1', 'v5.2', 'v5.3' or 'v5.4'."
        ))),
        None => {
            // Return list of available versions as JSON
            Ok(serde_json::json!({
                "available_versions": [
                    {"version": "v1.0.0", "description": "Scalar-only models (simple key-value pairs)"},
                    {"version": "v5.0.0", "description": "Full feature set (arrays, tables, Monte Carlo, etc.)"},
                    {"version": "v5.1.0", "description": "5.0.0 plus the short form (`price: 16`, `{formula: ...}` without nulls); use this for new models"},
                    {"version": "v5.2.0", "description": "5.1.0 plus the period column (`month: {start: 2026-12, count: 300, unit: month}`); needed only for a table with one"},
                    {"version": "v5.3.0", "description": "5.2.0 plus the empty cell (`actual: [985, 1032, null]`, ADR-059); needed only for a column with one"},
                    {"version": "v5.4.0", "description": "5.3.0 plus a date as a scalar (`last_actual: 2026-10-31`, ADR-060) and `format:`, an Excel number format beside `unit:` (ADR-061); needed only for a date scalar or a `format:`"}
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
pub fn schema(c: &mut Console<'_>, version: Option<&str>, list: bool) -> ForgeResult<()> {
    let Some(version) = version.filter(|_| !list) else {
        print_schema_list(c);
        return Ok(());
    };

    let schema = match version {
        "v1" | "v1.0.0" | "1" | "1.0.0" => SCHEMA_V1,
        "v5" | "v5.0.0" | "5" | "5.0.0" => SCHEMA_V5,
        "v5.1" | "v5.1.0" | "5.1" | "5.1.0" => SCHEMA_V5_1,
        "v5.2" | "v5.2.0" | "5.2" | "5.2.0" => SCHEMA_V5_2,
        "v5.3" | "v5.3.0" | "5.3" | "5.3.0" => SCHEMA_V5_3,
        "v5.4" | "v5.4.0" | "5.4" | "5.4.0" => SCHEMA_V5_4,
        v => {
            return Err(ForgeError::Validation(format!(
                "Unknown model format '{v}'. Use 'v1', 'v5', 'v5.1', 'v5.2', 'v5.3' or 'v5.4'.\n\n\
                 Run {} to see available versions.",
                "forge schema --list".yellow()
            )));
        },
    };

    outln!(c, "{schema}");
    Ok(())
}

/// Each model format: its version, what it is, and what it adds.
const FORMATS: [(&str, &str, &[&str]); 6] = [
    (
        "v1.0.0",
        "Scalar-only models (simple key-value pairs)",
        &[
            "Named variables with value/formula/metadata",
            "Scenarios for what-if analysis",
            "No arrays or tables",
        ],
    ),
    (
        "v5.0.0",
        "Full feature set",
        &[
            "Arrays and tables with row-wise formulas",
            "inputs/outputs separation",
            "Monte Carlo, Decision Trees, Real Options",
            "Cross-file includes",
        ],
    ),
    (
        "v5.1.0",
        "5.0.0 plus the short form (use for new models)",
        &[
            "Inputs as plain numbers: price: 16",
            "Formulas without value: null: {formula: \"=a * b\"}",
            "forge calculate writes each result next to its formula",
            "forge upgrade converts a 5.0.0 model, keeping comments",
        ],
    ),
    (
        "v5.2.0",
        "5.1.0 plus the period column (ADR-053)",
        &[
            "month: {start: 2026-12, count: 300, unit: month}",
            "Needed only for a table with a period column",
        ],
    ),
    (
        "v5.3.0",
        "5.2.0 plus the empty cell (ADR-059)",
        &[
            "actual: [985, 1032, null]: aggregates skip it, formulas read 0",
            "Needed only for a column with an empty cell",
        ],
    ),
    (
        "v5.4.0",
        "5.3.0 plus a date as a scalar (ADR-060) and a number format (ADR-061)",
        &[
            "last_actual: 2026-10-31: a date, read as a date column's cell is",
            "price: {value: 16, format: \"$#,##0.00\"}: how a number is shown, never its value",
            "Needed only for a scalar written as a date or a format:",
        ],
    ),
];

/// Print list of available schema versions
fn print_schema_list(c: &mut Console<'_>) {
    outln!(
        c,
        "{}",
        "Forge Model Formats (_forge_version)".bold().green()
    );
    outln!(
        c,
        "These are versions of the model file format, not of Forge."
    );
    for (version, summary, adds) in FORMATS {
        outln!(c);
        outln!(c, "  {}  {summary}", version.cyan().bold());
        for line in adds {
            outln!(c, "         - {line}");
        }
    }
    outln!(c);
    outln!(
        c,
        "Usage: {}, {}, {}, {}, {} or {}",
        "forge schema v1".yellow(),
        "forge schema v5".yellow(),
        "forge schema v5.1".yellow(),
        "forge schema v5.2".yellow(),
        "forge schema v5.3".yellow(),
        "forge schema v5.4".yellow()
    );
    outln!(c);
    outln!(
        c,
        "Pipe to file: {}",
        "forge schema v5.1 > schema.json".dimmed()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;

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
    fn the_list_names_every_format() {
        for (version, list) in [(None, false), (None, true), (Some("v1"), true)] {
            let (result, printed) = capture(|c| schema(c, version, list));
            result.unwrap();
            assert!(
                printed
                    .out
                    .starts_with("Forge Model Formats (_forge_version)\n"),
                "{}",
                printed.out
            );
            for format in ["v1.0.0", "v5.0.0", "v5.1.0", "v5.2.0", "v5.3.0", "v5.4.0"] {
                assert!(printed.out.contains(format), "{format}: {}", printed.out);
            }
            assert!(printed
                .out
                .ends_with("Pipe to file: forge schema v5.1 > schema.json\n"));
        }
    }

    #[test]
    fn each_name_of_a_format_prints_its_schema() {
        let formats = [
            (["v1", "v1.0.0", "1", "1.0.0"], SCHEMA_V1),
            (["v5", "v5.0.0", "5", "5.0.0"], SCHEMA_V5),
            (["v5.1", "v5.1.0", "5.1", "5.1.0"], SCHEMA_V5_1),
            (["v5.2", "v5.2.0", "5.2", "5.2.0"], SCHEMA_V5_2),
            (["v5.3", "v5.3.0", "5.3", "5.3.0"], SCHEMA_V5_3),
            (["v5.4", "v5.4.0", "5.4", "5.4.0"], SCHEMA_V5_4),
        ];
        for (names, expected) in formats {
            for name in names {
                let (result, printed) = capture(|c| schema(c, Some(name), false));
                result.unwrap();
                assert_eq!(printed.out, format!("{expected}\n"), "{name}");
            }
        }
    }

    #[test]
    fn an_unknown_format_is_refused_with_the_known_ones() {
        let (result, printed) = capture(|c| schema(c, Some("v9"), false));
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown model format 'v9'"), "{err}");
        assert!(err.contains("'v5.4'"), "{err}");
        assert!(err.contains("forge schema --list"), "{err}");
        assert_eq!(printed.both(), "");
    }
}
