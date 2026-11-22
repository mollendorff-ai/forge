//! Functions command - list all supported Excel-compatible functions
//!
//! Uses the function registry as the single source of truth.
//! See src/functions/registry.rs for the authoritative list.

use crate::cli::console::outln;
use crate::cli::Console;
use crate::error::ForgeResult;
use crate::functions::registry::{self, Category, FunctionDef};
use colored::Colorize;
use std::collections::BTreeMap;

/// Return function catalog as a JSON value (no printing).
#[must_use]
pub fn functions_core() -> serde_json::Value {
    catalog()
}

/// The function catalog as JSON: the total, and each category's functions.
fn catalog() -> serde_json::Value {
    let all_functions: Vec<&FunctionDef> = registry::all_functions().collect();
    let total = all_functions.len();

    let mut by_category: BTreeMap<String, Vec<&FunctionDef>> = BTreeMap::new();
    for func in &all_functions {
        by_category
            .entry(func.category.to_string())
            .or_default()
            .push(func);
    }

    serde_json::json!({
        "total": total,
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
    })
}

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
pub fn functions(c: &mut Console<'_>, json_output: bool) -> ForgeResult<()> {
    if json_output {
        // JSON output for tooling
        outln!(c, "{}", serde_json::to_string_pretty(&catalog()).unwrap());
        return Ok(());
    }

    // Get all functions from registry
    let all_functions: Vec<&FunctionDef> = registry::all_functions().collect();
    let total = all_functions.len();

    // Group by category
    let mut by_category: BTreeMap<String, Vec<&FunctionDef>> = BTreeMap::new();
    for func in &all_functions {
        by_category
            .entry(func.category.to_string())
            .or_default()
            .push(func);
    }

    // Human-readable output
    outln!(c, "{}", "🔥 Forge - Supported Functions".bold().green());
    outln!(c);
    outln!(
        c,
        "{}",
        format!("   {total} Excel-compatible functions for financial modeling").bright_white()
    );
    outln!(c);
    outln!(c, "{}", "═".repeat(70));

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
        // Every category has functions (the test of this command lists them all)
        let funcs = &by_category[&cat_name];
        outln!(c);
        outln!(c, "{} ({})", cat_name.bold().cyan(), funcs.len());
        outln!(c, "{}", "─".repeat(70));

        for func in funcs {
            outln!(
                c,
                "  {:12} {}",
                func.name.bold().yellow(),
                format!("{} - {}", func.description, func.syntax).bright_white()
            );
        }
    }

    outln!(c);
    outln!(c, "{}", "═".repeat(70));
    outln!(c);
    outln!(
        c,
        "{}",
        "Use these functions in your YAML formulas: formula: \"=NPV(0.1, cashflows)\""
            .bright_black()
    );
    outln!(c);
    outln!(
        c,
        "{}",
        "In a table column, an aggregate's range is the whole column and its criterion the\n\
         row's value: quarters.revenue: \"=SUMIFS(months.revenue, months.quarter, quarter)\".\n\
         SUM, MAX and the like read the whole column with one argument and work row by row\n\
         with several (=MAX(0, ebit))."
            .bright_black()
    );
    outln!(c);

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli::run::forge;

    /// `forge functions --json` is the catalog `functions_core` gives, and
    /// the listing names every function of it, under every category.
    #[test]
    fn the_listing_and_its_json_have_every_function() {
        let json: serde_json::Value =
            serde_json::from_str(&forge(&["functions", "--json"]).out).unwrap();
        let core = super::functions_core();
        assert_eq!(json, core);
        let listing = forge(&["functions"]).out;
        assert!(listing.contains(&format!("{} Excel-compatible functions", json["total"])));
        let categories = json["categories"].as_array().unwrap();
        assert_eq!(categories.len(), 15);
        for category in categories {
            let name = category["name"].as_str().unwrap();
            let count = category["count"].as_u64().unwrap();
            assert!(listing.contains(&format!("{name} ({count})")), "{name}");
            for function in category["functions"].as_array().unwrap() {
                assert!(listing.contains(function["name"].as_str().unwrap()));
            }
        }
    }
}
