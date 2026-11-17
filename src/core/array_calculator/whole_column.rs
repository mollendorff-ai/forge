//! Which arguments of a function are whole columns in a row formula (ADR-058).
//!
//! In a row formula a name is the value in the current row, except in an
//! argument that Excel reads as a range: there it is the whole column, of
//! another table (`months.revenue`) or of the row's own (`revenue`). So
//! `=SUMIFS(months.revenue, months.fiscal_quarter, quarter)` sums the months
//! of the row's quarter, and `=revenue / SUM(revenue)` is a share of the total.
//!
//! This one table is read by the evaluator (which hides the row while it
//! evaluates such an argument), by the calculation order (`PREV` groups) and by
//! the export, which writes such an argument as an absolute range.

use super::parser::{Expr, Reference};
use std::collections::HashSet;

/// Whether argument `position` (0-based) of `function` (upper case), called
/// with `arg_count` arguments, is read as a whole column in a row formula.
#[must_use]
// Grouped as ADR-058's table is, by what the arguments mean
#[allow(clippy::match_same_arms)]
pub fn is_whole_column(function: &str, arg_count: usize, position: usize) -> bool {
    match function {
        // With several arguments these work row by row: `=MAX(0, ebit)`
        "SUM" | "AVERAGE" | "AVG" | "MAX" | "MIN" | "COUNT" | "MEDIAN" | "MODE" | "MODE.SNGL"
        | "GEOMEAN" => arg_count == 1,
        // range, criterion, [sum range]
        "SUMIF" | "COUNTIF" | "AVERAGEIF" => position == 0 || position == 2,
        // sum range, then pairs of criteria range and criterion
        "SUMIFS" | "AVERAGEIFS" | "MAXIFS" | "MINIFS" => {
            position == 0 || !position.is_multiple_of(2)
        },
        // pairs of criteria range and criterion
        "COUNTIFS" => position.is_multiple_of(2),
        "COUNTBLANK" => position == 0,
        "SUMPRODUCT" | "VAR" | "VAR.S" | "VAR.P" | "STDEV" | "STDEV.S" | "STDEV.P" => true,
        // the values, then k or the quartile (a different one in each row)
        "PERCENTILE" | "PERCENTILE.INC" | "PERCENTILE.EXC" | "QUARTILE" | "QUARTILE.INC"
        | "QUARTILE.EXC" => position == 0,
        "CORREL" | "SLOPE" | "INTERCEPT" | "RSQ" | "COVARIANCE.S" | "COVARIANCE.P" => position <= 1,
        // x, then the known y and x values
        "FORECAST" | "FORECAST.LINEAR" => position == 1 || position == 2,
        // the lookups that always read whole columns
        "INDEX" | "ROWS" => position == 0,
        "MATCH" | "XMATCH" => position == 1,
        "XLOOKUP" => position == 1 || position == 2,
        _ => false,
    }
}

/// Whether some argument of `function` is a whole column with `arg_count` arguments.
#[must_use]
pub fn has_whole_column(function: &str, arg_count: usize) -> bool {
    (0..arg_count).any(|position| is_whole_column(function, arg_count, position))
}

/// The names a formula reads as whole columns: `revenue` and `months.revenue`
/// as written, inside any argument that [`is_whole_column`].
#[must_use]
pub fn whole_column_names(ast: &Expr) -> HashSet<String> {
    fn walk(expr: &Expr, whole: bool, names: &mut HashSet<String>) {
        match expr {
            Expr::Reference(reference) => {
                if whole {
                    names.insert(match reference {
                        Reference::Scalar(name) => name.clone(),
                        Reference::TableColumn { table, column } => format!("{table}.{column}"),
                    });
                }
            },
            Expr::FunctionCall { name, args } => {
                let upper = name.to_uppercase();
                for (position, arg) in args.iter().enumerate() {
                    let inside = whole || is_whole_column(&upper, args.len(), position);
                    walk(arg, inside, names);
                }
            },
            Expr::CallResult { callable, args } => {
                walk(callable, whole, names);
                for arg in args {
                    walk(arg, whole, names);
                }
            },
            Expr::ArrayIndex { array, index } => {
                walk(array, whole, names);
                walk(index, whole, names);
            },
            Expr::BinaryOp { left, right, .. } => {
                walk(left, whole, names);
                walk(right, whole, names);
            },
            Expr::Range { start, end } => {
                walk(start, whole, names);
                walk(end, whole, names);
            },
            Expr::UnaryOp { operand, .. } => walk(operand, whole, names),
            Expr::Number(_) | Expr::Text(_) | Expr::Boolean(_) | Expr::Array { .. } => {},
        }
    }
    let mut names = HashSet::new();
    walk(ast, false, &mut names);
    names
}

#[cfg(test)]
mod tests {

    use super::is_whole_column;

    #[test]
    fn criteria_are_the_rows_value_and_ranges_are_whole() {
        // SUMIFS(sum, range1, crit1, range2, crit2)
        let sumifs: Vec<bool> = (0..5).map(|p| is_whole_column("SUMIFS", 5, p)).collect();
        assert_eq!(sumifs, [true, true, false, true, false]);
        // COUNTIFS(range1, crit1, range2, crit2)
        let countifs: Vec<bool> = (0..4).map(|p| is_whole_column("COUNTIFS", 4, p)).collect();
        assert_eq!(countifs, [true, false, true, false]);
        let sumif: Vec<bool> = (0..3).map(|p| is_whole_column("SUMIF", 3, p)).collect();
        assert_eq!(sumif, [true, false, true]);
        assert!(is_whole_column("PERCENTILE.INC", 2, 0));
        assert!(!is_whole_column("PERCENTILE.INC", 2, 1));
        assert!(!is_whole_column("FORECAST", 3, 0));
        assert!(is_whole_column("FORECAST", 3, 2));
    }

    #[test]
    fn one_argument_aggregates_and_several_work_row_by_row() {
        assert!(is_whole_column("SUM", 1, 0));
        assert!(!is_whole_column("SUM", 2, 0));
        assert!(!is_whole_column("MAX", 2, 1));
        assert!(is_whole_column("GEOMEAN", 1, 0));
        // Functions not in the table keep reading the row's value
        assert!(!is_whole_column("NPV", 2, 1));
        assert!(!is_whole_column("IF", 3, 0));
    }

    mod row_aggregates {
        //! Aggregates in a row formula (ADR-058): one case per row of the ADR's
        //! table, with the numbers worked by hand.
        //!
        //! `months` has quarters 1, 1, 2, 2, 3 and revenue 10, 20, 30, 40, 50, so the
        //! quarters sum to 30, 70 and 50. The revenue's mean is 30, its squared
        //! deviations 400, 100, 0, 100, 400; against the quarters (mean 1.8) the sum
        //! of products of deviations is 50 and of squared quarter deviations 2.8, so
        //! the slope is 50 / 2.8 and the correlation 50 / sqrt(2.8 * 1000).

        use crate::cli::calculate_core;

        const MODEL: &str = r#"_forge_version: "5.1.0"
months:
  q: [1, 1, 2, 2, 3]
  cat: ["a", "b", "a", "b", "a"]
  revenue: [10, 20, 30, 40, 50]
  share: "=revenue / SUM(revenue)"
  rank: "=COUNTIF(revenue, \">\" & revenue) + 1"
  clipped: "=MAX(25, revenue)"
  weighted: "=SUM(revenue * q)"
quarters:
  quarter: [1, 2, 3]
  p: [0.25, 0.5, 0.75]
  sumifs: "=SUMIFS(months.revenue, months.q, quarter)"
  sumifs_two: "=SUMIFS(months.revenue, months.q, quarter, months.cat, \"a\")"
  averageifs: "=AVERAGEIFS(months.revenue, months.q, quarter)"
  maxifs: "=MAXIFS(months.revenue, months.q, quarter)"
  minifs: "=MINIFS(months.revenue, months.q, quarter)"
  countifs: "=COUNTIFS(months.q, quarter)"
  sumif: "=SUMIF(months.q, quarter, months.revenue)"
  countif: "=COUNTIF(months.q, quarter)"
  averageif: "=AVERAGEIF(months.q, quarter, months.revenue)"
  percentile: "=PERCENTILE.INC(months.revenue, p)"
  percentile_exc: "=PERCENTILE.EXC(months.revenue, p)"
  quartile: "=QUARTILE.INC(months.revenue, quarter)"
  forecast: "=FORECAST(quarter, months.revenue, months.q)"
  with_let: "=LET(k, quarter, SUMPRODUCT((months.q = k) * months.revenue))"
  sum: "=SUM(months.revenue)"
  average: "=AVERAGE(months.revenue)"
  max: "=MAX(months.revenue)"
  min: "=MIN(months.revenue)"
  count: "=COUNT(months.revenue)"
  median: "=MEDIAN(months.revenue)"
  mode: "=MODE(months.q)"
  geomean: "=GEOMEAN(months.revenue)"
  sumproduct: "=SUMPRODUCT(months.revenue, months.q)"
  var: "=VAR.S(months.revenue)"
  stdev: "=STDEV.P(months.revenue)"
  correl: "=CORREL(months.revenue, months.q)"
  slope: "=SLOPE(months.revenue, months.q)"
  intercept: "=INTERCEPT(months.revenue, months.q)"
"#;

        fn calculate(yaml: &str) -> crate::cli::commands::results::CalculationResult {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("model.yaml");
            std::fs::write(&path, yaml).unwrap();
            calculate_core(&path, true, None).unwrap()
        }

        fn column(
            result: &crate::cli::commands::results::CalculationResult,
            table: &str,
            name: &str,
        ) -> Vec<f64> {
            result.tables[table].values[name]
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect()
        }

        fn assert_close(got: &[f64], expected: &[f64], what: &str) {
            assert_eq!(got.len(), expected.len(), "{what}: {got:?}");
            for (g, e) in got.iter().zip(expected) {
                assert!(
                    (g - e).abs() <= 1e-12 * e.abs().max(1.0),
                    "{what}: {got:?}, expected {expected:?}"
                );
            }
        }

        #[test]
        fn each_function_reads_its_ranges_whole_and_its_criteria_in_the_row() {
            let result = calculate(MODEL);
            let slope: f64 = 50.0 / 2.8;
            let row = |v: f64| [v, v, v];
            for (name, expected) in [
                ("sumifs", [30.0, 70.0, 50.0]),
                ("sumifs_two", [10.0, 30.0, 50.0]),
                ("averageifs", [15.0, 35.0, 50.0]),
                ("maxifs", [20.0, 40.0, 50.0]),
                ("minifs", [10.0, 30.0, 50.0]),
                ("countifs", [2.0, 2.0, 1.0]),
                ("sumif", [30.0, 70.0, 50.0]),
                ("countif", [2.0, 2.0, 1.0]),
                ("averageif", [15.0, 35.0, 50.0]),
                // (n - 1) p and (n + 1) p into 10, 20, 30, 40, 50
                ("percentile", [20.0, 30.0, 40.0]),
                ("percentile_exc", [15.0, 30.0, 45.0]),
                ("quartile", [20.0, 30.0, 40.0]),
                (
                    "forecast",
                    [
                        slope.mul_add(1.0 - 1.8, 30.0),
                        slope.mul_add(2.0 - 1.8, 30.0),
                        slope.mul_add(3.0 - 1.8, 30.0),
                    ],
                ),
                ("with_let", [30.0, 70.0, 50.0]),
                ("sum", row(150.0)),
                ("average", row(30.0)),
                ("max", row(50.0)),
                ("min", row(10.0)),
                ("count", row(5.0)),
                ("median", row(30.0)),
                // 1 and 2 are both twice; the first met
                ("mode", row(1.0)),
                ("geomean", row(12_000_000f64.powf(0.2))),
                ("sumproduct", row(320.0)),
                ("var", row(250.0)),
                ("stdev", row(200f64.sqrt())),
                ("correl", row(50.0 / (2.8f64 * 1000.0).sqrt())),
                ("slope", row(slope)),
                ("intercept", row(slope.mul_add(-1.8, 30.0))),
            ] {
                assert_close(&column(&result, "quarters", name), &expected, name);
            }
        }

        #[test]
        fn a_column_reads_its_own_table_whole_and_several_arguments_stay_in_the_row() {
            let result = calculate(MODEL);
            let share: Vec<f64> = [10.0, 20.0, 30.0, 40.0, 50.0]
                .iter()
                .map(|r| r / 150.0)
                .collect();
            assert_close(&column(&result, "months", "share"), &share, "share");
            assert_close(
                &column(&result, "months", "rank"),
                &[5.0, 4.0, 3.0, 2.0, 1.0],
                "rank",
            );
            assert_close(
                &column(&result, "months", "clipped"),
                &[25.0, 25.0, 30.0, 40.0, 50.0],
                "clipped",
            );
            assert_close(
                &column(&result, "months", "weighted"),
                &[320.0; 5],
                "weighted",
            );
        }

        /// One table aggregating another of `rows` rows, `rows` times
        fn sumifs_model(rows: usize) -> String {
            let keys: Vec<String> = (0..rows).map(|i| (i % 50).to_string()).collect();
            let values: Vec<String> = (0..rows).map(|i| i.to_string()).collect();
            format!(
                "_forge_version: \"5.1.0\"\na:\n  k: [{keys}]\n  v: [{values}]\nb:\n  k: [{keys}]\n  total: \"=SUMIFS(a.v, a.k, k)\"\n",
                keys = keys.join(", "),
                values = values.join(", ")
            )
        }

        #[test]
        fn a_thousand_rows_aggregating_a_thousand_rows_take_under_a_second() {
            let start = std::time::Instant::now();
            let result = calculate(&sumifs_model(1_000));
            let took = start.elapsed();
            // Key 0 is in rows 0, 50, ..., 950: 0 + 50 + ... + 950 = 9,500
            assert_exact!(column(&result, "b", "total")[0], 9_500.0);
            assert!(took.as_secs_f64() < 1.0, "{took:?}");
        }

        /// ADR-058's target. Timed in a release build only (`cargo test --release`,
        /// as the weekly workflow runs); a debug build is several times slower.
        #[test]
        #[cfg_attr(debug_assertions, ignore = "timed in a release build")]
        fn five_thousand_rows_aggregating_five_thousand_rows_take_under_a_second() {
            let model = sumifs_model(5_000);
            let start = std::time::Instant::now();
            let result = calculate(&model);
            let took = start.elapsed();
            // Key 0 is in rows 0, 50, ..., 4,950: 50 * (0 + 99) * 100 / 2 = 247,500
            assert_exact!(column(&result, "b", "total")[0], 247_500.0);
            assert!(took.as_secs_f64() < 1.0, "{took:?}");
        }
    }
}
