//! Function Registry - Single Source of Truth for all Forge functions
//!
//! This module defines all supported functions with metadata including:
//! - Function name
//! - Category
//! - Description
//! - Syntax
//! - Demo availability (demo: true = included in demo build)
//!
//! See ADR-013 for the design rationale.

/// Function category for grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Math,
    Aggregation,
    Logical,
    Text,
    Date,
    Lookup,
    Financial,
    Statistical,
    Trigonometric,
    Information,
    Conditional,
    Array,
    Advanced,
    ForgeNative,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Category::Math => write!(f, "Math"),
            Category::Aggregation => write!(f, "Aggregation"),
            Category::Logical => write!(f, "Logical"),
            Category::Text => write!(f, "Text"),
            Category::Date => write!(f, "Date"),
            Category::Lookup => write!(f, "Lookup"),
            Category::Financial => write!(f, "Financial"),
            Category::Statistical => write!(f, "Statistical"),
            Category::Trigonometric => write!(f, "Trigonometric"),
            Category::Information => write!(f, "Information"),
            Category::Conditional => write!(f, "Conditional"),
            Category::Array => write!(f, "Array"),
            Category::Advanced => write!(f, "Advanced"),
            Category::ForgeNative => write!(f, "Forge Native"),
        }
    }
}

/// Function definition with all metadata
#[derive(Debug, Clone)]
pub struct FunctionDef {
    /// Function name (e.g., "SUM")
    pub name: &'static str,
    /// Category for grouping
    pub category: Category,
    /// Short description
    pub description: &'static str,
    /// Usage syntax (e.g., "=SUM(value1, value2, ...)")
    pub syntax: &'static str,
    /// Available in demo build (false = enterprise only)
    pub demo: bool,
}

/// All supported functions - THE SINGLE SOURCE OF TRUTH
///
/// Total: 159 functions (includes aliases)
/// Demo: 36 functions
/// Enterprise-only: 123 functions
pub static FUNCTIONS: &[FunctionDef] = &[
    // ══════════════════════════════════════════════════════════════════════════
    // MATH (9 demo + 10 enterprise = 19 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "ABS",
        category: Category::Math,
        description: "Absolute value",
        syntax: "=ABS(value)",
        demo: true,
    },
    FunctionDef {
        name: "SQRT",
        category: Category::Math,
        description: "Square root",
        syntax: "=SQRT(value)",
        demo: true,
    },
    FunctionDef {
        name: "ROUND",
        category: Category::Math,
        description: "Round to decimals",
        syntax: "=ROUND(value, decimals)",
        demo: true,
    },
    FunctionDef {
        name: "ROUNDUP",
        category: Category::Math,
        description: "Round up away from zero",
        syntax: "=ROUNDUP(value, decimals)",
        demo: true,
    },
    FunctionDef {
        name: "ROUNDDOWN",
        category: Category::Math,
        description: "Round down toward zero",
        syntax: "=ROUNDDOWN(value, decimals)",
        demo: true,
    },
    FunctionDef {
        name: "FLOOR",
        category: Category::Math,
        description: "Round down to multiple",
        syntax: "=FLOOR(value, significance)",
        demo: true,
    },
    FunctionDef {
        name: "CEILING",
        category: Category::Math,
        description: "Round up to multiple",
        syntax: "=CEILING(value, significance)",
        demo: true,
    },
    FunctionDef {
        name: "MOD",
        category: Category::Math,
        description: "Remainder after division",
        syntax: "=MOD(number, divisor)",
        demo: true,
    },
    FunctionDef {
        name: "POWER",
        category: Category::Math,
        description: "Number raised to power",
        syntax: "=POWER(base, exponent)",
        demo: true,
    },
    // Enterprise math
    FunctionDef {
        name: "EXP",
        category: Category::Math,
        description: "e raised to power",
        syntax: "=EXP(value)",
        demo: false,
    },
    FunctionDef {
        name: "LN",
        category: Category::Math,
        description: "Natural logarithm",
        syntax: "=LN(value)",
        demo: false,
    },
    FunctionDef {
        name: "LOG10",
        category: Category::Math,
        description: "Base-10 logarithm",
        syntax: "=LOG10(value)",
        demo: false,
    },
    FunctionDef {
        name: "INT",
        category: Category::Math,
        description: "Integer part",
        syntax: "=INT(value)",
        demo: false,
    },
    FunctionDef {
        name: "SIGN",
        category: Category::Math,
        description: "Sign of number (-1, 0, 1)",
        syntax: "=SIGN(value)",
        demo: false,
    },
    FunctionDef {
        name: "TRUNC",
        category: Category::Math,
        description: "Truncate to decimals",
        syntax: "=TRUNC(value, decimals)",
        demo: false,
    },
    FunctionDef {
        name: "POW",
        category: Category::Math,
        description: "Alias for POWER",
        syntax: "=POW(base, exp)",
        demo: false,
    },
    FunctionDef {
        name: "PI",
        category: Category::Math,
        description: "Pi constant",
        syntax: "=PI()",
        demo: false,
    },
    FunctionDef {
        name: "E",
        category: Category::Math,
        description: "Euler's number",
        syntax: "=E()",
        demo: false,
    },
    FunctionDef {
        name: "DEGREES",
        category: Category::Math,
        description: "Radians to degrees",
        syntax: "=DEGREES(radians)",
        demo: false,
    },
    FunctionDef {
        name: "RADIANS",
        category: Category::Math,
        description: "Degrees to radians",
        syntax: "=RADIANS(degrees)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // AGGREGATION (5 demo + 8 enterprise = 13 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "SUM",
        category: Category::Aggregation,
        description: "Sum of values",
        syntax: "=SUM(value1, value2, ...)",
        demo: true,
    },
    FunctionDef {
        name: "AVERAGE",
        category: Category::Aggregation,
        description: "Mean of values",
        syntax: "=AVERAGE(value1, value2, ...)",
        demo: true,
    },
    FunctionDef {
        name: "AVG",
        category: Category::Aggregation,
        description: "Alias for AVERAGE",
        syntax: "=AVG(value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "MIN",
        category: Category::Aggregation,
        description: "Minimum value",
        syntax: "=MIN(value1, value2, ...)",
        demo: true,
    },
    FunctionDef {
        name: "MAX",
        category: Category::Aggregation,
        description: "Maximum value",
        syntax: "=MAX(value1, value2, ...)",
        demo: true,
    },
    FunctionDef {
        name: "COUNT",
        category: Category::Aggregation,
        description: "Count of numbers",
        syntax: "=COUNT(value1, value2, ...)",
        demo: true,
    },
    // Enterprise aggregation
    FunctionDef {
        name: "COUNTA",
        category: Category::Aggregation,
        description: "Count non-empty",
        syntax: "=COUNTA(value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "COUNTUNIQUE",
        category: Category::Aggregation,
        description: "Count unique values",
        syntax: "=COUNTUNIQUE(value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "PRODUCT",
        category: Category::Aggregation,
        description: "Product of values",
        syntax: "=PRODUCT(value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "LARGE",
        category: Category::Aggregation,
        description: "Nth largest value",
        syntax: "=LARGE(array, n)",
        demo: false,
    },
    FunctionDef {
        name: "SMALL",
        category: Category::Aggregation,
        description: "Nth smallest value",
        syntax: "=SMALL(array, n)",
        demo: false,
    },
    FunctionDef {
        name: "MAXIFS",
        category: Category::Aggregation,
        description: "Max with conditions",
        syntax: "=MAXIFS(max_range, criteria_range, criteria)",
        demo: false,
    },
    FunctionDef {
        name: "MINIFS",
        category: Category::Aggregation,
        description: "Min with conditions",
        syntax: "=MINIFS(min_range, criteria_range, criteria)",
        demo: false,
    },
    FunctionDef {
        name: "RANK.EQ",
        category: Category::Aggregation,
        description: "Rank of value",
        syntax: "=RANK.EQ(value, array, order)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // LOGICAL (5 demo + 4 enterprise = 9 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "IF",
        category: Category::Logical,
        description: "Conditional value",
        syntax: "=IF(condition, true_value, false_value)",
        demo: true,
    },
    FunctionDef {
        name: "AND",
        category: Category::Logical,
        description: "All conditions true",
        syntax: "=AND(condition1, condition2, ...)",
        demo: true,
    },
    FunctionDef {
        name: "OR",
        category: Category::Logical,
        description: "Any condition true",
        syntax: "=OR(condition1, condition2, ...)",
        demo: true,
    },
    FunctionDef {
        name: "NOT",
        category: Category::Logical,
        description: "Negate condition",
        syntax: "=NOT(condition)",
        demo: true,
    },
    FunctionDef {
        name: "IFERROR",
        category: Category::Logical,
        description: "Handle errors",
        syntax: "=IFERROR(value, error_value)",
        demo: true,
    },
    // Enterprise logical
    FunctionDef {
        name: "XOR",
        category: Category::Logical,
        description: "Exclusive or",
        syntax: "=XOR(condition1, condition2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "IFNA",
        category: Category::Logical,
        description: "Handle #N/A errors",
        syntax: "=IFNA(value, na_value)",
        demo: false,
    },
    FunctionDef {
        name: "TRUE",
        category: Category::Logical,
        description: "Boolean TRUE",
        syntax: "=TRUE()",
        demo: false,
    },
    FunctionDef {
        name: "FALSE",
        category: Category::Logical,
        description: "Boolean FALSE",
        syntax: "=FALSE()",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // TEXT (8 demo + 7 enterprise = 15 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "CONCAT",
        category: Category::Text,
        description: "Join strings",
        syntax: "=CONCAT(text1, text2, ...)",
        demo: true,
    },
    FunctionDef {
        name: "CONCATENATE",
        category: Category::Text,
        description: "Alias for CONCAT",
        syntax: "=CONCATENATE(text1, text2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "LEFT",
        category: Category::Text,
        description: "Left characters",
        syntax: "=LEFT(text, num_chars)",
        demo: true,
    },
    FunctionDef {
        name: "RIGHT",
        category: Category::Text,
        description: "Right characters",
        syntax: "=RIGHT(text, num_chars)",
        demo: true,
    },
    FunctionDef {
        name: "MID",
        category: Category::Text,
        description: "Middle characters",
        syntax: "=MID(text, start, num_chars)",
        demo: true,
    },
    FunctionDef {
        name: "LEN",
        category: Category::Text,
        description: "Text length",
        syntax: "=LEN(text)",
        demo: true,
    },
    FunctionDef {
        name: "UPPER",
        category: Category::Text,
        description: "Uppercase text",
        syntax: "=UPPER(text)",
        demo: true,
    },
    FunctionDef {
        name: "LOWER",
        category: Category::Text,
        description: "Lowercase text",
        syntax: "=LOWER(text)",
        demo: true,
    },
    FunctionDef {
        name: "TRIM",
        category: Category::Text,
        description: "Remove extra spaces",
        syntax: "=TRIM(text)",
        demo: true,
    },
    // Enterprise text
    FunctionDef {
        name: "TEXT",
        category: Category::Text,
        description: "Format number as text",
        syntax: "=TEXT(value, format)",
        demo: false,
    },
    FunctionDef {
        name: "VALUE",
        category: Category::Text,
        description: "Convert text to number",
        syntax: "=VALUE(text)",
        demo: false,
    },
    FunctionDef {
        name: "FIND",
        category: Category::Text,
        description: "Find text position",
        syntax: "=FIND(find_text, within_text, start)",
        demo: false,
    },
    FunctionDef {
        name: "SEARCH",
        category: Category::Text,
        description: "Find text (case insensitive)",
        syntax: "=SEARCH(find_text, within_text, start)",
        demo: false,
    },
    FunctionDef {
        name: "REPLACE",
        category: Category::Text,
        description: "Replace characters",
        syntax: "=REPLACE(text, start, num_chars, new_text)",
        demo: false,
    },
    FunctionDef {
        name: "SUBSTITUTE",
        category: Category::Text,
        description: "Substitute text",
        syntax: "=SUBSTITUTE(text, old_text, new_text, instance)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // DATE (6 demo + 15 enterprise = 21 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "TODAY",
        category: Category::Date,
        description: "Current date",
        syntax: "=TODAY()",
        demo: true,
    },
    FunctionDef {
        name: "DATE",
        category: Category::Date,
        description: "Create date",
        syntax: "=DATE(year, month, day)",
        demo: true,
    },
    FunctionDef {
        name: "YEAR",
        category: Category::Date,
        description: "Extract year",
        syntax: "=YEAR(date)",
        demo: true,
    },
    FunctionDef {
        name: "MONTH",
        category: Category::Date,
        description: "Extract month",
        syntax: "=MONTH(date)",
        demo: true,
    },
    FunctionDef {
        name: "DAY",
        category: Category::Date,
        description: "Extract day",
        syntax: "=DAY(date)",
        demo: true,
    },
    FunctionDef {
        name: "DATEDIF",
        category: Category::Date,
        description: "Date difference",
        syntax: "=DATEDIF(start, end, unit)",
        demo: true,
    },
    // Enterprise date
    FunctionDef {
        name: "NOW",
        category: Category::Date,
        description: "Current date and time",
        syntax: "=NOW()",
        demo: false,
    },
    FunctionDef {
        name: "TIME",
        category: Category::Date,
        description: "Create time",
        syntax: "=TIME(hour, minute, second)",
        demo: false,
    },
    FunctionDef {
        name: "HOUR",
        category: Category::Date,
        description: "Extract hour",
        syntax: "=HOUR(time)",
        demo: false,
    },
    FunctionDef {
        name: "MINUTE",
        category: Category::Date,
        description: "Extract minute",
        syntax: "=MINUTE(time)",
        demo: false,
    },
    FunctionDef {
        name: "SECOND",
        category: Category::Date,
        description: "Extract second",
        syntax: "=SECOND(time)",
        demo: false,
    },
    FunctionDef {
        name: "WEEKDAY",
        category: Category::Date,
        description: "Day of week",
        syntax: "=WEEKDAY(date, type)",
        demo: false,
    },
    FunctionDef {
        name: "DAYS",
        category: Category::Date,
        description: "Days between dates",
        syntax: "=DAYS(end_date, start_date)",
        demo: false,
    },
    FunctionDef {
        name: "EDATE",
        category: Category::Date,
        description: "Add months to date",
        syntax: "=EDATE(date, months)",
        demo: false,
    },
    FunctionDef {
        name: "EOMONTH",
        category: Category::Date,
        description: "End of month",
        syntax: "=EOMONTH(date, months)",
        demo: false,
    },
    FunctionDef {
        name: "NETWORKDAYS",
        category: Category::Date,
        description: "Working days between",
        syntax: "=NETWORKDAYS(start, end, holidays)",
        demo: false,
    },
    FunctionDef {
        name: "WORKDAY",
        category: Category::Date,
        description: "Add working days",
        syntax: "=WORKDAY(start, days, holidays)",
        demo: false,
    },
    FunctionDef {
        name: "YEARFRAC",
        category: Category::Date,
        description: "Year fraction",
        syntax: "=YEARFRAC(start, end, basis)",
        demo: false,
    },
    FunctionDef {
        name: "Y",
        category: Category::Date,
        description: "Years between (shorthand)",
        syntax: "=Y(start, end)",
        demo: false,
    },
    FunctionDef {
        name: "M",
        category: Category::Date,
        description: "Months between (shorthand)",
        syntax: "=M(start, end)",
        demo: false,
    },
    FunctionDef {
        name: "D",
        category: Category::Date,
        description: "Days between (shorthand)",
        syntax: "=D(start, end)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // LOOKUP (3 demo + 9 enterprise = 12 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "INDEX",
        category: Category::Lookup,
        description: "Value by position",
        syntax: "=INDEX(array, row, col)",
        demo: true,
    },
    FunctionDef {
        name: "MATCH",
        category: Category::Lookup,
        description: "Find position",
        syntax: "=MATCH(lookup_value, array, type)",
        demo: true,
    },
    FunctionDef {
        name: "CHOOSE",
        category: Category::Lookup,
        description: "Pick by index",
        syntax: "=CHOOSE(index, value1, value2, ...)",
        demo: true,
    },
    // Enterprise lookup
    FunctionDef {
        name: "VLOOKUP",
        category: Category::Lookup,
        description: "Vertical lookup",
        syntax: "=VLOOKUP(lookup, table, col, exact)",
        demo: false,
    },
    FunctionDef {
        name: "HLOOKUP",
        category: Category::Lookup,
        description: "Horizontal lookup",
        syntax: "=HLOOKUP(lookup, table, row, exact)",
        demo: false,
    },
    FunctionDef {
        name: "XLOOKUP",
        category: Category::Lookup,
        description: "Extended lookup",
        syntax: "=XLOOKUP(lookup, lookup_array, return_array, not_found)",
        demo: false,
    },
    FunctionDef {
        name: "OFFSET",
        category: Category::Lookup,
        description: "Reference offset",
        syntax: "=OFFSET(ref, rows, cols, height, width)",
        demo: false,
    },
    FunctionDef {
        name: "INDIRECT",
        category: Category::Lookup,
        description: "Reference from text",
        syntax: "=INDIRECT(ref_text)",
        demo: false,
    },
    FunctionDef {
        name: "ADDRESS",
        category: Category::Lookup,
        description: "Cell address text",
        syntax: "=ADDRESS(row, col, abs_type)",
        demo: false,
    },
    FunctionDef {
        name: "ROW",
        category: Category::Lookup,
        description: "Row number",
        syntax: "=ROW(reference)",
        demo: false,
    },
    FunctionDef {
        name: "COLUMN",
        category: Category::Lookup,
        description: "Column number",
        syntax: "=COLUMN(reference)",
        demo: false,
    },
    FunctionDef {
        name: "ROWS",
        category: Category::Lookup,
        description: "Number of rows",
        syntax: "=ROWS(array)",
        demo: false,
    },
    FunctionDef {
        name: "COLUMNS",
        category: Category::Lookup,
        description: "Number of columns",
        syntax: "=COLUMNS(array)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // FINANCIAL (0 demo + 20 enterprise = 20 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "PMT",
        category: Category::Financial,
        description: "Loan payment",
        syntax: "=PMT(rate, nper, pv, fv, type)",
        demo: false,
    },
    FunctionDef {
        name: "PV",
        category: Category::Financial,
        description: "Present value",
        syntax: "=PV(rate, nper, pmt, fv, type)",
        demo: false,
    },
    FunctionDef {
        name: "FV",
        category: Category::Financial,
        description: "Future value",
        syntax: "=FV(rate, nper, pmt, pv, type)",
        demo: false,
    },
    FunctionDef {
        name: "NPV",
        category: Category::Financial,
        description: "Net present value",
        syntax: "=NPV(rate, value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "IRR",
        category: Category::Financial,
        description: "Internal rate of return",
        syntax: "=IRR(values, guess)",
        demo: false,
    },
    FunctionDef {
        name: "MIRR",
        category: Category::Financial,
        description: "Modified IRR",
        syntax: "=MIRR(values, finance_rate, reinvest_rate)",
        demo: false,
    },
    FunctionDef {
        name: "XNPV",
        category: Category::Financial,
        description: "NPV with dates",
        syntax: "=XNPV(rate, values, dates)",
        demo: false,
    },
    FunctionDef {
        name: "XIRR",
        category: Category::Financial,
        description: "IRR with dates",
        syntax: "=XIRR(values, dates, guess)",
        demo: false,
    },
    FunctionDef {
        name: "NPER",
        category: Category::Financial,
        description: "Number of periods",
        syntax: "=NPER(rate, pmt, pv, fv, type)",
        demo: false,
    },
    FunctionDef {
        name: "RATE",
        category: Category::Financial,
        description: "Interest rate",
        syntax: "=RATE(nper, pmt, pv, fv, type, guess)",
        demo: false,
    },
    FunctionDef {
        name: "DB",
        category: Category::Financial,
        description: "Declining balance depreciation",
        syntax: "=DB(cost, salvage, life, period, month)",
        demo: false,
    },
    FunctionDef {
        name: "DDB",
        category: Category::Financial,
        description: "Double declining balance",
        syntax: "=DDB(cost, salvage, life, period, factor)",
        demo: false,
    },
    FunctionDef {
        name: "SLN",
        category: Category::Financial,
        description: "Straight-line depreciation",
        syntax: "=SLN(cost, salvage, life)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // STATISTICAL (0 demo + 11 enterprise = 11 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "MEDIAN",
        category: Category::Statistical,
        description: "Median value",
        syntax: "=MEDIAN(value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "VAR.S",
        category: Category::Statistical,
        description: "Sample variance",
        syntax: "=VAR.S(value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "VARP",
        category: Category::Statistical,
        description: "Population variance",
        syntax: "=VARP(value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "STDEV.S",
        category: Category::Statistical,
        description: "Sample std deviation",
        syntax: "=STDEV.S(value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "STDEVP",
        category: Category::Statistical,
        description: "Population std deviation",
        syntax: "=STDEVP(value1, value2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "PERCENTILE",
        category: Category::Statistical,
        description: "Percentile value",
        syntax: "=PERCENTILE(array, k)",
        demo: false,
    },
    FunctionDef {
        name: "QUARTILE",
        category: Category::Statistical,
        description: "Quartile value",
        syntax: "=QUARTILE(array, quart)",
        demo: false,
    },
    FunctionDef {
        name: "CORREL",
        category: Category::Statistical,
        description: "Correlation coefficient",
        syntax: "=CORREL(array1, array2)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // TRIGONOMETRIC (0 demo + 11 enterprise = 11 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "SIN",
        category: Category::Trigonometric,
        description: "Sine",
        syntax: "=SIN(angle)",
        demo: false,
    },
    FunctionDef {
        name: "COS",
        category: Category::Trigonometric,
        description: "Cosine",
        syntax: "=COS(angle)",
        demo: false,
    },
    FunctionDef {
        name: "TAN",
        category: Category::Trigonometric,
        description: "Tangent",
        syntax: "=TAN(angle)",
        demo: false,
    },
    FunctionDef {
        name: "ASIN",
        category: Category::Trigonometric,
        description: "Arcsine",
        syntax: "=ASIN(value)",
        demo: false,
    },
    FunctionDef {
        name: "ACOS",
        category: Category::Trigonometric,
        description: "Arccosine",
        syntax: "=ACOS(value)",
        demo: false,
    },
    FunctionDef {
        name: "ATAN",
        category: Category::Trigonometric,
        description: "Arctangent",
        syntax: "=ATAN(value)",
        demo: false,
    },
    FunctionDef {
        name: "SINH",
        category: Category::Trigonometric,
        description: "Hyperbolic sine",
        syntax: "=SINH(value)",
        demo: false,
    },
    FunctionDef {
        name: "COSH",
        category: Category::Trigonometric,
        description: "Hyperbolic cosine",
        syntax: "=COSH(value)",
        demo: false,
    },
    FunctionDef {
        name: "TANH",
        category: Category::Trigonometric,
        description: "Hyperbolic tangent",
        syntax: "=TANH(value)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // INFORMATION (0 demo + 13 enterprise = 13 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "ISBLANK",
        category: Category::Information,
        description: "Is cell empty",
        syntax: "=ISBLANK(value)",
        demo: false,
    },
    FunctionDef {
        name: "ISERROR",
        category: Category::Information,
        description: "Is error value",
        syntax: "=ISERROR(value)",
        demo: false,
    },
    FunctionDef {
        name: "ISNA",
        category: Category::Information,
        description: "Is #N/A error",
        syntax: "=ISNA(value)",
        demo: false,
    },
    FunctionDef {
        name: "ISNUMBER",
        category: Category::Information,
        description: "Is numeric",
        syntax: "=ISNUMBER(value)",
        demo: false,
    },
    FunctionDef {
        name: "ISTEXT",
        category: Category::Information,
        description: "Is text",
        syntax: "=ISTEXT(value)",
        demo: false,
    },
    FunctionDef {
        name: "ISLOGICAL",
        category: Category::Information,
        description: "Is boolean",
        syntax: "=ISLOGICAL(value)",
        demo: false,
    },
    FunctionDef {
        name: "ISEVEN",
        category: Category::Information,
        description: "Is even number",
        syntax: "=ISEVEN(value)",
        demo: false,
    },
    FunctionDef {
        name: "ISODD",
        category: Category::Information,
        description: "Is odd number",
        syntax: "=ISODD(value)",
        demo: false,
    },
    FunctionDef {
        name: "ISREF",
        category: Category::Information,
        description: "Is reference",
        syntax: "=ISREF(value)",
        demo: false,
    },
    FunctionDef {
        name: "ISFORMULA",
        category: Category::Information,
        description: "Is formula",
        syntax: "=ISFORMULA(value)",
        demo: false,
    },
    FunctionDef {
        name: "NA",
        category: Category::Information,
        description: "Return #N/A",
        syntax: "=NA()",
        demo: false,
    },
    FunctionDef {
        name: "TYPE",
        category: Category::Information,
        description: "Type of value",
        syntax: "=TYPE(value)",
        demo: false,
    },
    FunctionDef {
        name: "N",
        category: Category::Information,
        description: "Convert to number",
        syntax: "=N(value)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // CONDITIONAL (0 demo + 7 enterprise = 7 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "IFS",
        category: Category::Conditional,
        description: "Multiple conditions",
        syntax: "=IFS(cond1, val1, cond2, val2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "SWITCH",
        category: Category::Conditional,
        description: "Switch case",
        syntax: "=SWITCH(expr, case1, val1, case2, val2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "SUMIF",
        category: Category::Conditional,
        description: "Sum with condition",
        syntax: "=SUMIF(range, criteria, sum_range)",
        demo: false,
    },
    FunctionDef {
        name: "SUMIFS",
        category: Category::Conditional,
        description: "Sum with conditions",
        syntax: "=SUMIFS(sum_range, range1, crit1, ...)",
        demo: false,
    },
    FunctionDef {
        name: "COUNTIF",
        category: Category::Conditional,
        description: "Count with condition",
        syntax: "=COUNTIF(range, criteria)",
        demo: false,
    },
    FunctionDef {
        name: "COUNTIFS",
        category: Category::Conditional,
        description: "Count with conditions",
        syntax: "=COUNTIFS(range1, crit1, range2, crit2, ...)",
        demo: false,
    },
    FunctionDef {
        name: "AVERAGEIF",
        category: Category::Conditional,
        description: "Average with condition",
        syntax: "=AVERAGEIF(range, criteria, avg_range)",
        demo: false,
    },
    FunctionDef {
        name: "AVERAGEIFS",
        category: Category::Conditional,
        description: "Average with conditions",
        syntax: "=AVERAGEIFS(avg_range, range1, crit1, ...)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // ARRAY (0 demo + 5 enterprise = 5 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "UNIQUE",
        category: Category::Array,
        description: "Unique values",
        syntax: "=UNIQUE(array)",
        demo: false,
    },
    FunctionDef {
        name: "FILTER",
        category: Category::Array,
        description: "Filter array",
        syntax: "=FILTER(array, include, if_empty)",
        demo: false,
    },
    FunctionDef {
        name: "SORT",
        category: Category::Array,
        description: "Sort array",
        syntax: "=SORT(array, sort_index, order)",
        demo: false,
    },
    FunctionDef {
        name: "SEQUENCE",
        category: Category::Array,
        description: "Generate sequence",
        syntax: "=SEQUENCE(rows, cols, start, step)",
        demo: false,
    },
    FunctionDef {
        name: "RANDARRAY",
        category: Category::Array,
        description: "Random array",
        syntax: "=RANDARRAY(rows, cols, min, max)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // ADVANCED (0 demo + 3 enterprise = 3 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "LET",
        category: Category::Advanced,
        description: "Define variables",
        syntax: "=LET(name1, val1, name2, val2, ..., calc)",
        demo: false,
    },
    FunctionDef {
        name: "LAMBDA",
        category: Category::Advanced,
        description: "Anonymous function",
        syntax: "=LAMBDA(param1, param2, ..., expression)",
        demo: false,
    },
    FunctionDef {
        name: "SCENARIO",
        category: Category::Advanced,
        description: "Scenario lookup",
        syntax: "=SCENARIO(name)",
        demo: false,
    },
    // ══════════════════════════════════════════════════════════════════════════
    // FORGE NATIVE (0 demo + 7 enterprise = 7 total)
    // ══════════════════════════════════════════════════════════════════════════
    FunctionDef {
        name: "VARIANCE",
        category: Category::ForgeNative,
        description: "Actual vs budget variance",
        syntax: "=VARIANCE(actual, budget)",
        demo: false,
    },
    FunctionDef {
        name: "VARIANCE_PCT",
        category: Category::ForgeNative,
        description: "Variance percentage",
        syntax: "=VARIANCE_PCT(actual, budget)",
        demo: false,
    },
    FunctionDef {
        name: "VARIANCE_STATUS",
        category: Category::ForgeNative,
        description: "Variance status",
        syntax: "=VARIANCE_STATUS(actual, budget)",
        demo: false,
    },
    FunctionDef {
        name: "BREAKEVEN_UNITS",
        category: Category::ForgeNative,
        description: "Break-even units",
        syntax: "=BREAKEVEN_UNITS(fixed, price, variable)",
        demo: false,
    },
    FunctionDef {
        name: "BREAKEVEN_REVENUE",
        category: Category::ForgeNative,
        description: "Break-even revenue",
        syntax: "=BREAKEVEN_REVENUE(fixed, margin_pct)",
        demo: false,
    },
    FunctionDef {
        name: "YD",
        category: Category::ForgeNative,
        description: "Years and days since",
        syntax: "=YD(start, end)",
        demo: false,
    },
    FunctionDef {
        name: "YM",
        category: Category::ForgeNative,
        description: "Years and months since",
        syntax: "=YM(start, end)",
        demo: false,
    },
    FunctionDef {
        name: "MD",
        category: Category::ForgeNative,
        description: "Months and days since",
        syntax: "=MD(start, end)",
        demo: false,
    },
];

/// Get demo functions only
pub fn demo_functions() -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter().filter(|f| f.demo)
}

/// Get enterprise functions (all)
pub fn enterprise_functions() -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter()
}

/// Count demo functions
pub fn count_demo() -> usize {
    FUNCTIONS.iter().filter(|f| f.demo).count()
}

/// Count enterprise functions (total)
pub fn count_enterprise() -> usize {
    FUNCTIONS.len()
}

/// Get functions by category
pub fn by_category(category: Category) -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter().filter(move |f| f.category == category)
}

/// Get demo functions by category
pub fn demo_by_category(category: Category) -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS
        .iter()
        .filter(move |f| f.demo && f.category == category)
}

/// Check if function is available in demo build
pub fn is_demo_function(name: &str) -> bool {
    FUNCTIONS.iter().any(|f| f.name == name && f.demo)
}

/// Find function by name
pub fn find_function(name: &str) -> Option<&'static FunctionDef> {
    FUNCTIONS.iter().find(|f| f.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_count() {
        // 36 demo functions (9 Math, 5 Aggregation, 5 Logical, 8 Text, 6 Date, 3 Lookup)
        assert_eq!(count_demo(), 36, "Demo should have 36 functions");
    }

    #[test]
    fn test_enterprise_count() {
        // 159 total functions (includes aliases like AVG, CONCATENATE)
        assert_eq!(
            count_enterprise(),
            159,
            "Enterprise should have 159 functions"
        );
    }

    #[test]
    fn test_demo_functions_are_subset() {
        for f in demo_functions() {
            assert!(f.demo, "Demo function {} should have demo=true", f.name);
        }
    }

    #[test]
    fn test_math_demo_count() {
        let count = demo_by_category(Category::Math).count();
        assert_eq!(count, 9, "Math should have 9 demo functions");
    }

    #[test]
    fn test_find_function() {
        let sum = find_function("SUM");
        assert!(sum.is_some());
        assert!(sum.unwrap().demo);

        let npv = find_function("NPV");
        assert!(npv.is_some());
        assert!(!npv.unwrap().demo);
    }

    #[test]
    fn test_is_demo_function() {
        assert!(is_demo_function("SUM"));
        assert!(is_demo_function("IF"));
        assert!(!is_demo_function("NPV"));
        assert!(!is_demo_function("IRR"));
    }
}
