//! Examples command - display runnable YAML examples for Forge capabilities

use crate::error::{ForgeError, ForgeResult};
use colored::Colorize;
use std::io::Write;
use std::process::Command;

/// Embedded example YAML files (compile-time)
const EXAMPLE_MONTE_CARLO: &str = include_str!("../../../examples/monte-carlo.yaml");
const EXAMPLE_SCENARIOS: &str = include_str!("../../../examples/scenarios.yaml");
const EXAMPLE_DECISION_TREE: &str = include_str!("../../../examples/decision-tree.yaml");
const EXAMPLE_REAL_OPTIONS: &str = include_str!("../../../examples/real-options.yaml");
const EXAMPLE_TORNADO: &str = include_str!("../../../examples/tornado.yaml");
const EXAMPLE_BOOTSTRAP: &str = include_str!("../../../examples/bootstrap.yaml");
const EXAMPLE_BAYESIAN: &str = include_str!("../../../examples/bayesian.yaml");
const EXAMPLE_VARIANCE: &str = include_str!("../../../examples/variance.yaml");
const EXAMPLE_BREAKEVEN: &str = include_str!("../../../examples/breakeven.yaml");

/// Example metadata
struct Example {
    name: &'static str,
    description: &'static str,
    command: &'static str,
    content: &'static str,
}

/// All available examples
const EXAMPLES: &[Example] = &[
    Example {
        name: "monte-carlo",
        description: "Probabilistic simulation with distributions",
        command: "forge simulate",
        content: EXAMPLE_MONTE_CARLO,
    },
    Example {
        name: "scenarios",
        description: "Probability-weighted scenario analysis",
        command: "forge scenarios",
        content: EXAMPLE_SCENARIOS,
    },
    Example {
        name: "decision-tree",
        description: "Sequential decisions with backward induction",
        command: "forge decision-tree",
        content: EXAMPLE_DECISION_TREE,
    },
    Example {
        name: "real-options",
        description: "Option pricing for managerial flexibility",
        command: "forge real-options",
        content: EXAMPLE_REAL_OPTIONS,
    },
    Example {
        name: "tornado",
        description: "One-at-a-time sensitivity analysis",
        command: "forge tornado",
        content: EXAMPLE_TORNADO,
    },
    Example {
        name: "bootstrap",
        description: "Non-parametric confidence intervals",
        command: "forge bootstrap",
        content: EXAMPLE_BOOTSTRAP,
    },
    Example {
        name: "bayesian",
        description: "Probabilistic graphical models",
        command: "forge bayesian",
        content: EXAMPLE_BAYESIAN,
    },
    Example {
        name: "variance",
        description: "Budget vs actual analysis",
        command: "forge calculate",
        content: EXAMPLE_VARIANCE,
    },
    Example {
        name: "breakeven",
        description: "Break-even calculations",
        command: "forge calculate",
        content: EXAMPLE_BREAKEVEN,
    },
];

/// Display example YAML models for Forge capabilities.
///
/// # Errors
///
/// Returns an error if the requested example name is not found,
/// or if execution of the example fails when `--run` is specified.
///
/// # Panics
///
/// Panics if `name` is `Some` and `.unwrap()` is called after the `None` check.
/// This is safe because we only unwrap after verifying `name.is_some()`.
pub fn examples(name: Option<String>, run: bool, json: bool) -> ForgeResult<()> {
    // JSON output mode for tooling
    if json {
        print_examples_json();
        return Ok(());
    }

    // No name specified - show list
    if name.is_none() {
        print_examples_list();
        return Ok(());
    }

    let name = name.unwrap();

    // Find the requested example
    let example = EXAMPLES.iter().find(|e| e.name == name).ok_or_else(|| {
        ForgeError::Validation(format!(
            "Unknown example '{name}'. Run 'forge examples' to see available examples.",
        ))
    })?;

    // Print the example
    println!("{}", format!("# {}", example.name).bold().green());
    println!("{}", format!("# {}", example.description).dimmed());
    println!(
        "{}",
        format!("# Run with: {} <file>", example.command).dimmed()
    );
    println!();
    println!("{}", example.content);

    // Run the example if requested
    if run {
        println!();
        println!("{}", "─".repeat(60).dimmed());
        println!("{}", "Running example...".bold().cyan());
        println!();
        run_example(example)?;
    }

    Ok(())
}

/// Print list of available examples
fn print_examples_list() {
    println!("{}", "Available Forge Examples".bold().green());
    println!();
    println!(
        "{}",
        "Forge-specific capabilities beyond Excel functions:".dimmed()
    );
    println!();

    for example in EXAMPLES {
        println!(
            "  {:16} {}",
            example.name.cyan().bold(),
            example.description
        );
    }

    println!();
    println!("Usage: {}", "forge examples <name>".yellow());
    println!("       {}", "forge examples <name> --run".yellow());
    println!();
    println!("Example: {}", "forge examples monte-carlo".dimmed());
}

/// Print examples as JSON for tooling
fn print_examples_json() {
    let examples_json: Vec<serde_json::Value> = EXAMPLES
        .iter()
        .map(|e| {
            serde_json::json!({
                "name": e.name,
                "description": e.description,
                "command": e.command
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&examples_json).unwrap());
}

/// Run an example by writing to temp file and executing
fn run_example(example: &Example) -> ForgeResult<()> {
    // Write example to temp file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("forge-example-{}.yaml", example.name));

    let mut file = std::fs::File::create(&temp_file).map_err(|e| {
        ForgeError::Io(std::io::Error::other(format!(
            "Failed to create temp file: {e}"
        )))
    })?;

    file.write_all(example.content.as_bytes()).map_err(|e| {
        ForgeError::Io(std::io::Error::other(format!(
            "Failed to write temp file: {e}"
        )))
    })?;

    // Determine the command to run
    let (cmd, args) = match example.name {
        "monte-carlo" => ("forge", vec!["simulate", temp_file.to_str().unwrap()]),
        "scenarios" => ("forge", vec!["scenarios", temp_file.to_str().unwrap()]),
        "decision-tree" => ("forge", vec!["decision-tree", temp_file.to_str().unwrap()]),
        "real-options" => ("forge", vec!["real-options", temp_file.to_str().unwrap()]),
        "tornado" => ("forge", vec!["tornado", temp_file.to_str().unwrap()]),
        "bootstrap" => ("forge", vec!["bootstrap", temp_file.to_str().unwrap()]),
        "bayesian" => ("forge", vec!["bayesian", temp_file.to_str().unwrap()]),
        _ => ("forge", vec!["calculate", temp_file.to_str().unwrap()]),
    };

    // Execute the command
    let output = Command::new(cmd).args(&args).output().map_err(|e| {
        ForgeError::Io(std::io::Error::other(format!(
            "Failed to execute command: {e}"
        )))
    })?;

    // Print output
    if !output.stdout.is_empty() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    if output.status.success() {
        Ok(())
    } else {
        Err(ForgeError::Validation(format!(
            "Example execution failed with exit code: {:?}",
            output.status.code()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_examples_are_valid_yaml() {
        for example in EXAMPLES {
            let parsed: Result<serde_yaml_ng::Value, _> = serde_yaml_ng::from_str(example.content);
            assert!(
                parsed.is_ok(),
                "Example '{}' should be valid YAML: {:?}",
                example.name,
                parsed.err()
            );
        }
    }

    #[test]
    fn test_examples_list_mode() {
        // Just ensure it doesn't panic
        let result = examples(None, false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_examples_json_mode() {
        let result = examples(None, false, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_examples_show_specific() {
        for example in EXAMPLES {
            let result = examples(Some(example.name.to_string()), false, false);
            assert!(result.is_ok(), "Should show example '{}'", example.name);
        }
    }

    #[test]
    fn test_examples_unknown_name() {
        let result = examples(Some("nonexistent".to_string()), false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_example_count() {
        assert_eq!(EXAMPLES.len(), 9, "Should have exactly 9 examples");
    }

    #[test]
    fn test_all_examples_have_forge_version() {
        for example in EXAMPLES {
            assert!(
                example.content.contains("_forge_version:"),
                "Example '{}' should have _forge_version field",
                example.name
            );
        }
    }
}
