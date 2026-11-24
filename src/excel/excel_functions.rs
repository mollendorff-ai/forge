//! The functions Excel knows by name. The export writes a call to one of them
//! as it is; a call to any other function has no Excel equivalent.

/// Excel's function names, upper case and in byte order, so that a lookup can
/// bisect. A test keeps the order.
const EXCEL_FUNCTIONS: [&str; 193] = [
    "ABS",
    "ACCRINT",
    "ACOS",
    "ADDRESS",
    "AND",
    "ASIN",
    "ATAN",
    "AVERAGE",
    "AVERAGEIF",
    "AVERAGEIFS",
    "AVG",
    "CEILING",
    "CHOOSE",
    "COLUMN",
    "COLUMNS",
    "CONCAT",
    "CONCATENATE",
    "CORREL",
    "COS",
    "COSH",
    "COUNT",
    "COUNTA",
    "COUNTBLANK",
    "COUNTIF",
    "COUNTIFS",
    "COUNTUNIQUE",
    "COVARIANCE.P",
    "COVARIANCE.S",
    "CUMIPMT",
    "CUMPRINC",
    "DATE",
    "DATEDIF",
    "DAY",
    "DAYS",
    "DB",
    "DDB",
    "DEGREES",
    "E",
    "EDATE",
    "EFFECT",
    "EOMONTH",
    "EXP",
    "FALSE",
    "FILTER",
    "FIND",
    "FLOOR",
    "FORECAST",
    "FORECAST.LINEAR",
    "FV",
    "GEOMEAN",
    "HLOOKUP",
    "HOUR",
    "IF",
    "IFERROR",
    "IFNA",
    "IFS",
    "INDEX",
    "INDIRECT",
    "INT",
    "INTERCEPT",
    "IPMT",
    "IRR",
    "ISBLANK",
    "ISERROR",
    "ISEVEN",
    "ISFORMULA",
    "ISLOGICAL",
    "ISNA",
    "ISNUMBER",
    "ISODD",
    "ISOWEEKNUM",
    "ISREF",
    "ISTEXT",
    "LAMBDA",
    "LARGE",
    "LEFT",
    "LEN",
    "LET",
    "LN",
    "LOG",
    "LOG10",
    "LOWER",
    "MATCH",
    "MAX",
    "MAXIFS",
    "MEDIAN",
    "MID",
    "MIN",
    "MINIFS",
    "MINUTE",
    "MIRR",
    "MOD",
    "MODE",
    "MODE.SNGL",
    "MONTH",
    "MROUND",
    "N",
    "NA",
    "NETWORKDAYS",
    "NOMINAL",
    "NORM.DIST",
    "NORM.INV",
    "NORM.S.DIST",
    "NORM.S.INV",
    "NOT",
    "NOW",
    "NPER",
    "NPV",
    "OFFSET",
    "OR",
    "PERCENTILE",
    "PERCENTILE.EXC",
    "PERCENTILE.INC",
    "PI",
    "PMT",
    "POW",
    "POWER",
    "PPMT",
    "PRICEDISC",
    "PRODUCT",
    "PV",
    "QUARTILE",
    "QUARTILE.EXC",
    "QUARTILE.INC",
    "RADIANS",
    "RAND",
    "RANDARRAY",
    "RANDBETWEEN",
    "RANK",
    "RANK.EQ",
    "RATE",
    "REPLACE",
    "REPT",
    "RIGHT",
    "ROUND",
    "ROUNDDOWN",
    "ROUNDUP",
    "ROW",
    "ROWS",
    "RRI",
    "RSQ",
    "SEARCH",
    "SECOND",
    "SEQUENCE",
    "SIGN",
    "SIN",
    "SINH",
    "SLN",
    "SLOPE",
    "SMALL",
    "SORT",
    "SQRT",
    "STDEV",
    "STDEV.P",
    "STDEV.S",
    "STDEVP",
    "SUBSTITUTE",
    "SUM",
    "SUMIF",
    "SUMIFS",
    "SUMPRODUCT",
    "SWITCH",
    "SYD",
    "TAN",
    "TANH",
    "TEXT",
    "TEXTJOIN",
    "TIME",
    "TODAY",
    "TRIM",
    "TRUE",
    "TRUNC",
    "TYPE",
    "UNIQUE",
    "UPPER",
    "VALUE",
    "VAR",
    "VAR.P",
    "VAR.S",
    "VARP",
    "VDB",
    "VLOOKUP",
    "WEEKDAY",
    "WEEKNUM",
    "WORKDAY",
    "XIRR",
    "XLOOKUP",
    "XMATCH",
    "XNPV",
    "XOR",
    "YEAR",
    "YEARFRAC",
    "YIELDDISC",
];

/// Whether Excel knows `word` as a function, in any letter case.
pub(super) fn is_excel_function(word: &str) -> bool {
    EXCEL_FUNCTIONS
        .binary_search(&word.to_uppercase().as_str())
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_excel_function_is_known_to_the_export() {
        // A function in the list (`forge functions`) that the export does not
        // know would be written as its value, with a warning. Forge's own
        // functions and the Monte Carlo ones have no Excel name and are left out.
        use crate::functions::registry::{Category, FUNCTIONS};
        let own = [
            "SCENARIO",
            "PREV",
            "VARIANCE",
            "VARIANCE_PCT",
            "VARIANCE_STATUS",
        ];
        let missing: Vec<&str> = FUNCTIONS
            .iter()
            .filter(|f| !matches!(f.category, Category::MonteCarlo | Category::ForgeNative))
            .map(|f| f.name)
            .filter(|n| !own.contains(n))
            .filter(|n| !is_excel_function(n))
            .collect();
        assert!(missing.is_empty(), "not known to the export: {missing:?}");
    }

    /// The list is sorted and has no repeats, or the bisection would miss a name
    #[test]
    fn the_excel_function_names_are_sorted_and_distinct() {
        assert!(EXCEL_FUNCTIONS.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_function_is_known_in_any_letter_case_and_a_name_is_not() {
        assert!(is_excel_function("SUM"));
        assert!(is_excel_function("sum"));
        assert!(is_excel_function("Norm.S.Dist"));
        assert!(is_excel_function("XLOOKUP"));
        assert!(!is_excel_function("revenue"));
        assert!(!is_excel_function("coût_unitaire"));
        assert!(!is_excel_function("单价"));
    }
}
