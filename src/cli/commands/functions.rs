//! Functions command - list all supported Excel-compatible functions
//!
//! DEMO: 33 functions
//! ENTERPRISE: 154 functions

use crate::error::ForgeResult;
use colored::Colorize;

/// Function category with functions and descriptions
pub struct FunctionCategory {
    pub name: &'static str,
    pub functions: Vec<(&'static str, &'static str)>,
}

/// Execute the functions command - list all supported Excel-compatible functions
pub fn functions(json_output: bool) -> ForgeResult<()> {
    let categories = build_function_categories();

    // Count total functions
    let total: usize = categories.iter().map(|c| c.functions.len()).sum();

    if json_output {
        // JSON output for tooling
        let json = serde_json::json!({
            "total": total,
            "edition": if cfg!(feature = "full") { "enterprise" } else { "demo" },
            "categories": categories.iter().map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "count": c.functions.len(),
                    "functions": c.functions.iter().map(|(name, desc)| {
                        serde_json::json!({
                            "name": name,
                            "description": desc
                        })
                    }).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&json).unwrap());
    } else {
        // Human-readable output
        #[cfg(feature = "full")]
        {
            println!(
                "{}",
                "🔥 Forge Enterprise - Supported Functions".bold().green()
            );
            println!();
            println!(
                "{}",
                format!(
                    "   {} Excel-compatible functions for financial modeling",
                    total
                )
                .bright_white()
            );
        }
        #[cfg(not(feature = "full"))]
        {
            println!("{}", "🔥 Forge Demo - Supported Functions".bold().green());
            println!();
            println!(
                "{}",
                format!("   {} core functions (Enterprise: 154 functions)", total).bright_white()
            );
        }
        println!();
        println!("{}", "═".repeat(70));

        for category in &categories {
            if category.functions.is_empty() {
                continue;
            }
            println!();
            println!(
                "{} ({})",
                category.name.bold().cyan(),
                category.functions.len()
            );
            println!("{}", "─".repeat(70));

            for (name, desc) in &category.functions {
                println!("  {:12} {}", name.bold().yellow(), desc.bright_white());
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

/// Build function categories based on current feature flags
#[allow(unused_mut)]
fn build_function_categories() -> Vec<FunctionCategory> {
    vec![
        // ═══════════════════════════════════════════════════════════════════════════
        // FINANCIAL - Enterprise only (0 demo, 13 enterprise)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(feature = "full")]
        FunctionCategory {
            name: "Financial",
            functions: vec![
                ("NPV", "Net Present Value - =NPV(rate, cashflow1, cashflow2, ...)"),
                ("IRR", "Internal Rate of Return - =IRR(values, [guess])"),
                ("MIRR", "Modified IRR - =MIRR(values, finance_rate, reinvest_rate)"),
                ("XNPV", "NPV with irregular dates - =XNPV(rate, values, dates)"),
                ("XIRR", "IRR with irregular dates - =XIRR(values, dates, [guess])"),
                ("PMT", "Payment for a loan - =PMT(rate, nper, pv, [fv], [type])"),
                ("PV", "Present Value - =PV(rate, nper, pmt, [fv], [type])"),
                ("FV", "Future Value - =FV(rate, nper, pmt, [pv], [type])"),
                ("RATE", "Interest rate - =RATE(nper, pmt, pv, [fv], [type], [guess])"),
                ("NPER", "Number of periods - =NPER(rate, pmt, pv, [fv], [type])"),
                ("SLN", "Straight-line depreciation - =SLN(cost, salvage, life)"),
                ("DB", "Declining balance depreciation - =DB(cost, salvage, life, period)"),
                ("DDB", "Double declining balance - =DDB(cost, salvage, life, period)"),
            ],
        },
        #[cfg(not(feature = "full"))]
        FunctionCategory {
            name: "Financial",
            functions: vec![], // Enterprise only
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // STATISTICAL - Enterprise only (0 demo, 12 enterprise)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(feature = "full")]
        FunctionCategory {
            name: "Statistical",
            functions: vec![
                ("MEDIAN", "Middle value - =MEDIAN(array)"),
                ("VAR", "Variance (sample) - =VAR(array)"),
                ("VAR.S", "Variance (sample) - =VAR.S(array)"),
                ("VAR.P", "Variance (population) - =VAR.P(array)"),
                ("VARP", "Variance (population) - =VARP(array)"),
                ("STDEV", "Standard deviation (sample) - =STDEV(array)"),
                ("STDEV.S", "Standard deviation (sample) - =STDEV.S(array)"),
                ("STDEV.P", "Standard deviation (population) - =STDEV.P(array)"),
                ("STDEVP", "Standard deviation (population) - =STDEVP(array)"),
                ("PERCENTILE", "Percentile value - =PERCENTILE(array, k)"),
                ("QUARTILE", "Quartile value - =QUARTILE(array, quart)"),
                ("CORREL", "Correlation coefficient - =CORREL(array1, array2)"),
                ("LARGE", "K-th largest value - =LARGE(array, k)"),
                ("SMALL", "K-th smallest value - =SMALL(array, k)"),
                ("RANK", "Rank of value - =RANK(number, array, [order])"),
                ("RANK.EQ", "Rank of value - =RANK.EQ(number, array, [order])"),
            ],
        },
        #[cfg(not(feature = "full"))]
        FunctionCategory {
            name: "Statistical",
            functions: vec![], // Enterprise only
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // MATH - 9 demo, 10 enterprise
        // ═══════════════════════════════════════════════════════════════════════════
        FunctionCategory {
            name: "Math",
            functions: {
                let mut funcs = vec![
                    // DEMO (9)
                    ("ABS", "Absolute value - =ABS(value)"),
                    ("SQRT", "Square root - =SQRT(value)"),
                    ("ROUND", "Round to digits - =ROUND(value, digits)"),
                    ("ROUNDUP", "Round up - =ROUNDUP(value, digits)"),
                    ("ROUNDDOWN", "Round down - =ROUNDDOWN(value, digits)"),
                    ("FLOOR", "Round down to significance - =FLOOR(value, significance)"),
                    ("CEILING", "Round up to significance - =CEILING(value, significance)"),
                    ("MOD", "Modulo/remainder - =MOD(value, divisor)"),
                    ("POWER", "Power/exponent - =POWER(base, exponent)"),
                ];
                #[cfg(feature = "full")]
                {
                    // ENTERPRISE (10)
                    funcs.extend(vec![
                        ("EXP", "Exponential - =EXP(value)"),
                        ("LN", "Natural logarithm - =LN(value)"),
                        ("LOG", "Base-10 logarithm - =LOG(value)"),
                        ("LOG10", "Base-10 logarithm - =LOG10(value)"),
                        ("INT", "Integer part - =INT(value)"),
                        ("POW", "Power (alias) - =POW(base, exponent)"),
                        ("SIGN", "Sign of number - =SIGN(value)"),
                        ("TRUNC", "Truncate number - =TRUNC(value, [decimals])"),
                        ("PI", "Pi constant - =PI()"),
                        ("E", "Euler's number - =E()"),
                    ]);
                }
                funcs
            },
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // AGGREGATION - 5 demo, 4 enterprise
        // ═══════════════════════════════════════════════════════════════════════════
        FunctionCategory {
            name: "Aggregation",
            functions: {
                let mut funcs = vec![
                    // DEMO (5)
                    ("SUM", "Sum values - =SUM(value1, value2, ...)"),
                    ("AVERAGE", "Average values - =AVERAGE(value1, value2, ...)"),
                    ("MIN", "Minimum value - =MIN(value1, value2, ...)"),
                    ("MAX", "Maximum value - =MAX(value1, value2, ...)"),
                    ("COUNT", "Count numeric values - =COUNT(array)"),
                ];
                #[cfg(feature = "full")]
                {
                    // ENTERPRISE (4)
                    funcs.extend(vec![
                        ("AVG", "Average (alias) - =AVG(value1, value2, ...)"),
                        ("PRODUCT", "Product of values - =PRODUCT(value1, value2, ...)"),
                        ("COUNTA", "Count non-empty values - =COUNTA(array)"),
                        ("MEDIAN", "Median value - =MEDIAN(array)"),
                    ]);
                }
                funcs
            },
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // LOGICAL - 5 demo, 5 enterprise
        // ═══════════════════════════════════════════════════════════════════════════
        FunctionCategory {
            name: "Logical",
            functions: {
                let mut funcs = vec![
                    // DEMO (5)
                    ("IF", "Conditional - =IF(condition, true_value, false_value)"),
                    ("AND", "Logical AND - =AND(condition1, condition2, ...)"),
                    ("OR", "Logical OR - =OR(condition1, condition2, ...)"),
                    ("NOT", "Logical NOT - =NOT(condition)"),
                    ("IFERROR", "Error handling - =IFERROR(value, value_if_error)"),
                ];
                #[cfg(feature = "full")]
                {
                    // ENTERPRISE (5)
                    funcs.extend(vec![
                        ("IFNA", "Handle #N/A - =IFNA(value, value_if_na)"),
                        ("XOR", "Exclusive OR - =XOR(condition1, condition2, ...)"),
                        ("TRUE", "Boolean true - =TRUE()"),
                        ("FALSE", "Boolean false - =FALSE()"),
                        ("IFS", "Multiple conditions - =IFS(cond1, val1, cond2, val2, ...)"),
                    ]);
                }
                funcs
            },
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // TEXT - 8 demo, 8 enterprise
        // ═══════════════════════════════════════════════════════════════════════════
        FunctionCategory {
            name: "Text",
            functions: {
                let mut funcs = vec![
                    // DEMO (8)
                    ("CONCAT", "Concatenate strings - =CONCAT(text1, text2, ...)"),
                    ("UPPER", "Convert to uppercase - =UPPER(text)"),
                    ("LOWER", "Convert to lowercase - =LOWER(text)"),
                    ("TRIM", "Remove extra spaces - =TRIM(text)"),
                    ("LEN", "Length of text - =LEN(text)"),
                    ("LEFT", "Left characters - =LEFT(text, [num_chars])"),
                    ("RIGHT", "Right characters - =RIGHT(text, [num_chars])"),
                    ("MID", "Extract substring - =MID(text, start, length)"),
                ];
                #[cfg(feature = "full")]
                {
                    // ENTERPRISE (8)
                    funcs.extend(vec![
                        ("CONCATENATE", "Concatenate (alias) - =CONCATENATE(text1, text2, ...)"),
                        ("TEXT", "Format number as text - =TEXT(value, format)"),
                        ("VALUE", "Convert text to number - =VALUE(text)"),
                        ("FIND", "Find text (case-sensitive) - =FIND(find, within, [start])"),
                        ("SEARCH", "Search text (case-insensitive) - =SEARCH(find, within, [start])"),
                        ("REPLACE", "Replace by position - =REPLACE(old, start, num, new)"),
                        ("SUBSTITUTE", "Replace text - =SUBSTITUTE(text, old, new, [instance])"),
                    ]);
                }
                funcs
            },
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // DATE - 6 demo, 12 enterprise
        // ═══════════════════════════════════════════════════════════════════════════
        FunctionCategory {
            name: "Date",
            functions: {
                let mut funcs = vec![
                    // DEMO (6)
                    ("TODAY", "Current date - =TODAY()"),
                    ("DATE", "Create date - =DATE(year, month, day)"),
                    ("YEAR", "Extract year - =YEAR(date)"),
                    ("MONTH", "Extract month - =MONTH(date)"),
                    ("DAY", "Extract day - =DAY(date)"),
                    ("DATEDIF", "Date difference - =DATEDIF(start, end, unit)"),
                ];
                #[cfg(feature = "full")]
                {
                    // ENTERPRISE (12)
                    funcs.extend(vec![
                        ("NOW", "Current date and time - =NOW()"),
                        ("WEEKDAY", "Day of week - =WEEKDAY(date, [type])"),
                        ("HOUR", "Extract hour - =HOUR(time)"),
                        ("MINUTE", "Extract minute - =MINUTE(time)"),
                        ("SECOND", "Extract second - =SECOND(time)"),
                        ("TIME", "Create time - =TIME(hour, minute, second)"),
                        ("DAYS", "Days between dates - =DAYS(end, start)"),
                        ("WORKDAY", "Date after N working days - =WORKDAY(start, days)"),
                        ("EDATE", "Add months to date - =EDATE(start, months)"),
                        ("EOMONTH", "End of month - =EOMONTH(start, months)"),
                        ("NETWORKDAYS", "Working days between dates - =NETWORKDAYS(start, end)"),
                        ("YEARFRAC", "Fraction of year - =YEARFRAC(start, end, [basis])"),
                    ]);
                }
                funcs
            },
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // LOOKUP - 3 demo, 10 enterprise
        // ═══════════════════════════════════════════════════════════════════════════
        FunctionCategory {
            name: "Lookup",
            functions: {
                let mut funcs = vec![
                    // DEMO (3)
                    ("INDEX", "Get value by position - =INDEX(array, row, [col])"),
                    ("MATCH", "Find position in array - =MATCH(value, array, [type])"),
                    ("CHOOSE", "Pick nth value - =CHOOSE(index, value1, value2, ...)"),
                ];
                #[cfg(feature = "full")]
                {
                    // ENTERPRISE (10)
                    funcs.extend(vec![
                        ("INDIRECT", "String to reference - =INDIRECT(\"table.column\")"),
                        ("XLOOKUP", "Modern lookup - =XLOOKUP(value, lookup, return, [not_found], [match])"),
                        ("VLOOKUP", "Vertical lookup - =VLOOKUP(value, table, col, [approx])"),
                        ("HLOOKUP", "Horizontal lookup - =HLOOKUP(value, table, row, [approx])"),
                        ("OFFSET", "Offset reference - =OFFSET(ref, rows, cols, [height], [width])"),
                        ("ADDRESS", "Cell address - =ADDRESS(row, col, [abs], [a1])"),
                        ("ROW", "Row number - =ROW([reference])"),
                        ("COLUMN", "Column number - =COLUMN([reference])"),
                        ("ROWS", "Number of rows - =ROWS(array)"),
                        ("COLUMNS", "Number of columns - =COLUMNS(array)"),
                    ]);
                }
                funcs
            },
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // CONDITIONAL - Enterprise only (0 demo, 8 enterprise)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(feature = "full")]
        FunctionCategory {
            name: "Conditional",
            functions: vec![
                ("SUMIF", "Sum if condition - =SUMIF(range, criteria, [sum_range])"),
                ("COUNTIF", "Count if condition - =COUNTIF(range, criteria)"),
                ("AVERAGEIF", "Average if condition - =AVERAGEIF(range, criteria, [avg_range])"),
                ("SUMIFS", "Sum with multiple conditions - =SUMIFS(sum_range, range1, criteria1, ...)"),
                ("COUNTIFS", "Count with multiple conditions - =COUNTIFS(range1, criteria1, ...)"),
                ("AVERAGEIFS", "Average with multiple conditions - =AVERAGEIFS(avg_range, range1, criteria1, ...)"),
                ("MAXIFS", "Max with conditions - =MAXIFS(max_range, range1, criteria1, ...)"),
                ("MINIFS", "Min with conditions - =MINIFS(min_range, range1, criteria1, ...)"),
            ],
        },
        #[cfg(not(feature = "full"))]
        FunctionCategory {
            name: "Conditional",
            functions: vec![], // Enterprise only
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // ARRAY - Enterprise only (0 demo, 4 enterprise)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(feature = "full")]
        FunctionCategory {
            name: "Array",
            functions: vec![
                ("UNIQUE", "Get unique values - =UNIQUE(array)"),
                ("COUNTUNIQUE", "Count unique values - =COUNTUNIQUE(array)"),
                ("FILTER", "Filter by criteria - =FILTER(array, include)"),
                ("SORT", "Sort values - =SORT(array, [order])"),
            ],
        },
        #[cfg(not(feature = "full"))]
        FunctionCategory {
            name: "Array",
            functions: vec![], // Enterprise only
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // ADVANCED - Enterprise only (LET, SWITCH, LAMBDA)
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(feature = "full")]
        FunctionCategory {
            name: "Advanced",
            functions: vec![
                ("LET", "Named variables - =LET(name, value, ..., calculation)"),
                ("SWITCH", "Multi-match - =SWITCH(expr, val1, result1, ..., [default])"),
                ("LAMBDA", "Anonymous function - =LAMBDA(x, x*2)(5)"),
            ],
        },
        #[cfg(not(feature = "full"))]
        FunctionCategory {
            name: "Advanced",
            functions: vec![], // Enterprise only
        },
        // ═══════════════════════════════════════════════════════════════════════════
        // FORGE-NATIVE - Enterprise only
        // ═══════════════════════════════════════════════════════════════════════════
        #[cfg(feature = "full")]
        FunctionCategory {
            name: "Forge-Native",
            functions: vec![
                ("SCENARIO", "Get scenario value - =SCENARIO(name, variable)"),
                ("VARIANCE", "Budget variance - =VARIANCE(actual, budget)"),
                ("VARIANCE_PCT", "Variance percent - =VARIANCE_PCT(actual, budget)"),
                ("VARIANCE_STATUS", "Variance status - =VARIANCE_STATUS(actual, budget, [type])"),
                ("BREAKEVEN_UNITS", "Break-even units - =BREAKEVEN_UNITS(fixed, price, var_cost)"),
                ("BREAKEVEN_REVENUE", "Break-even revenue - =BREAKEVEN_REVENUE(fixed, margin_pct)"),
            ],
        },
        #[cfg(not(feature = "full"))]
        FunctionCategory {
            name: "Forge-Native",
            functions: vec![], // Enterprise only
        },
    ]
}
