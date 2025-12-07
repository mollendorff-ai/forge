//! Functions command - list all supported Excel-compatible functions

use crate::error::ForgeResult;
use colored::Colorize;

/// Function category with functions and descriptions
pub struct FunctionCategory {
    pub name: &'static str,
    pub functions: Vec<(&'static str, &'static str)>,
}

/// Execute the functions command - list all supported Excel-compatible functions
pub fn functions(json_output: bool) -> ForgeResult<()> {
    let categories = vec![
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
        FunctionCategory {
            name: "Lookup",
            functions: vec![
                ("MATCH", "Find position in array - =MATCH(value, array, [type])"),
                ("INDEX", "Get value by position - =INDEX(array, row, [col])"),
                ("VLOOKUP", "Vertical lookup - =VLOOKUP(value, table, col, [approx])"),
                ("XLOOKUP", "Modern lookup - =XLOOKUP(value, lookup, return, [not_found], [match], [search])"),
                ("CHOOSE", "Pick nth value - =CHOOSE(index, value1, value2, ...)"),
                ("OFFSET", "Dynamic range slice - =OFFSET(array, rows, [height])"),
            ],
        },
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
        FunctionCategory {
            name: "Array",
            functions: vec![
                ("UNIQUE", "Get unique values - =UNIQUE(array)"),
                ("COUNTUNIQUE", "Count unique values - =COUNTUNIQUE(array)"),
                ("FILTER", "Filter by criteria - =FILTER(array, include)"),
                ("SORT", "Sort values - =SORT(array, [order])"),
            ],
        },
        FunctionCategory {
            name: "Aggregation",
            functions: vec![
                ("SUM", "Sum values - =SUM(value1, value2, ...)"),
                ("AVERAGE", "Average values - =AVERAGE(value1, value2, ...) or =AVG(...)"),
                ("MIN", "Minimum value - =MIN(value1, value2, ...)"),
                ("MAX", "Maximum value - =MAX(value1, value2, ...)"),
                ("COUNT", "Count values - =COUNT(array)"),
            ],
        },
        FunctionCategory {
            name: "Math",
            functions: vec![
                ("ROUND", "Round to digits - =ROUND(value, digits)"),
                ("ROUNDUP", "Round up - =ROUNDUP(value, digits)"),
                ("ROUNDDOWN", "Round down - =ROUNDDOWN(value, digits)"),
                ("CEILING", "Round up to significance - =CEILING(value, significance)"),
                ("FLOOR", "Round down to significance - =FLOOR(value, significance)"),
                ("MOD", "Modulo/remainder - =MOD(value, divisor)"),
                ("SQRT", "Square root - =SQRT(value)"),
                ("POWER", "Power/exponent - =POWER(base, exponent)"),
                ("ABS", "Absolute value - =ABS(value)"),
            ],
        },
        FunctionCategory {
            name: "Text",
            functions: vec![
                ("CONCAT", "Concatenate strings - =CONCAT(text1, text2, ...)"),
                ("TRIM", "Remove extra spaces - =TRIM(text)"),
                ("UPPER", "Convert to uppercase - =UPPER(text)"),
                ("LOWER", "Convert to lowercase - =LOWER(text)"),
                ("LEN", "Length of text - =LEN(text)"),
                ("MID", "Extract substring - =MID(text, start, length)"),
            ],
        },
        FunctionCategory {
            name: "Date",
            functions: vec![
                ("TODAY", "Current date - =TODAY()"),
                ("DATE", "Create date - =DATE(year, month, day)"),
                ("YEAR", "Extract year - =YEAR(date)"),
                ("MONTH", "Extract month - =MONTH(date)"),
                ("DAY", "Extract day - =DAY(date)"),
                ("DATEDIF", "Date difference - =DATEDIF(start, end, unit)"),
                ("EDATE", "Add months to date - =EDATE(start, months)"),
                ("EOMONTH", "End of month - =EOMONTH(start, months)"),
                ("NETWORKDAYS", "Working days between dates - =NETWORKDAYS(start, end)"),
                ("WORKDAY", "Date after N working days - =WORKDAY(start, days)"),
                ("YEARFRAC", "Fraction of year - =YEARFRAC(start, end, [basis])"),
            ],
        },
        FunctionCategory {
            name: "Logic",
            functions: vec![
                ("IF", "Conditional - =IF(condition, true_value, false_value)"),
                ("AND", "Logical AND - =AND(condition1, condition2, ...)"),
                ("OR", "Logical OR - =OR(condition1, condition2, ...)"),
                ("LET", "Named variables - =LET(name, value, ..., calculation)"),
                ("SWITCH", "Multi-match - =SWITCH(expr, val1, result1, ..., [default])"),
                ("INDIRECT", "String to ref - =INDIRECT(\"table.column\")"),
                ("LAMBDA", "Anonymous func - =LAMBDA(x, x*2)(5)"),
            ],
        },
        FunctionCategory {
            name: "Statistical",
            functions: vec![
                ("MEDIAN", "Middle value - =MEDIAN(array)"),
                ("VAR", "Variance (sample) - =VAR(array)"),
                ("STDEV", "Standard deviation (sample) - =STDEV(array)"),
                ("PERCENTILE", "Percentile value - =PERCENTILE(array, k)"),
                ("QUARTILE", "Quartile value - =QUARTILE(array, quart)"),
                ("CORREL", "Correlation coefficient - =CORREL(array1, array2)"),
            ],
        },
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
    ];

    // Count total functions
    let total: usize = categories.iter().map(|c| c.functions.len()).sum();

    if json_output {
        // JSON output for tooling
        let json = serde_json::json!({
            "total": total,
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
        println!("{}", "🔥 Forge - Supported Functions".bold().green());
        println!();
        println!(
            "{}",
            format!(
                "   {} Excel-compatible functions for financial modeling",
                total
            )
            .bright_white()
        );
        println!();
        println!("{}", "═".repeat(70));

        for category in &categories {
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
