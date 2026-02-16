//! Functions command - list all supported Excel-compatible functions
//!
//! Uses the function registry as the single source of truth.
//! See src/functions/registry.rs for the authoritative list.

use crate::error::ForgeResult;
use crate::functions::registry::{self, Category, FunctionDef};
use colored::Colorize;
use std::collections::BTreeMap;

/// Execute the functions command - list all supported Excel-compatible functions.
///
/// # Errors
///
/// Returns an error if function listing fails.
///
/// # Panics
///
/// Panics if JSON serialization of the function registry fails, which should
/// never happen with static data.
pub fn functions(json_output: bool) -> ForgeResult<()> {
    // Get all functions from registry
    let all_functions: Vec<&FunctionDef> = registry::enterprise_functions().collect();

    let total = all_functions.len();

    // Group by category
    let mut by_category: BTreeMap<String, Vec<&FunctionDef>> = BTreeMap::new();
    for func in &all_functions {
        by_category
            .entry(func.category.to_string())
            .or_default()
            .push(func);
    }

    if json_output {
        // JSON output for tooling
        let json = serde_json::json!({
            "total": total,
            "edition": "enterprise",
            "categories": by_category.iter().map(|(name, funcs)| {
                serde_json::json!({
                    "name": name,
                    "count": funcs.len(),
                    "functions": funcs.iter().map(|f| {
                        serde_json::json!({
                            "name": f.name,
                            "description": f.description,
                            "syntax": f.syntax,
                            "scalar": f.scalar
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        // Human-readable output
        println!(
            "{}",
            "🔥 Forge Enterprise - Supported Functions".bold().green()
        );
        println!();
        println!(
            "{}",
            format!("   {total} Excel-compatible functions for financial modeling").bright_white()
        );
        println!();
        println!("{}", "═".repeat(70));

        // Display order for categories
        let category_order = [
            Category::Financial,
            Category::Statistical,
            Category::Math,
            Category::Aggregation,
            Category::Logical,
            Category::Text,
            Category::Date,
            Category::Lookup,
            Category::Conditional,
            Category::Array,
            Category::Trigonometric,
            Category::Information,
            Category::Advanced,
            Category::ForgeNative,
            Category::MonteCarlo,
        ];

        for category in category_order {
            let cat_name = category.to_string();
            if let Some(funcs) = by_category.get(&cat_name) {
                if funcs.is_empty() {
                    continue;
                }
                println!();
                println!("{} ({})", cat_name.bold().cyan(), funcs.len());
                println!("{}", "─".repeat(70));

                for func in funcs {
                    println!(
                        "  {:12} {}",
                        func.name.bold().yellow(),
                        format!("{} - {}", func.description, func.syntax).bright_white()
                    );
                }
            }
        }

        println!();
        println!("{}", "═".repeat(70));
        println!();
        println!(
            "{}",
            "Use these functions in your YAML formulas: formula: \"=NPV(0.1, cashflows)\""
                .bright_black()
        );
        println!();
    }

    Ok(())
}
