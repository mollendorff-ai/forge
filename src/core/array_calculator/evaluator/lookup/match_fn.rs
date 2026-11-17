//! MATCH and XMATCH function implementations

// Match casts: f64 match type to i32, array position to f64 (small bounded integers).
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]

use crate::core::array_calculator::evaluator::{
    evaluate, require_args_range, values_equal, EvalContext, EvalError, Expr, Value,
};
use crate::core::array_calculator::evaluator::{excel_date_serial, number_or, wildcard};

/// Evaluate MATCH function
/// `MATCH(lookup_value`, `lookup_array`, [`match_type`])
/// Returns the relative position of an item in an array
pub fn eval_match(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("MATCH", args, 2, 3)?;

    let lookup_value = evaluate(&args[0], ctx)?;

    let lookup_array = evaluate(&args[1], ctx)?;

    // A fraction is cut to a whole number (0.5 is an exact match), as
    // LibreOffice does; any other type counts by its sign, as Excel does:
    // MATCH(3, {1,3,5}, 2) is 2, an approximate match (checked in Excel,
    // 2026-11-14)
    let match_type = if args.len() > 2 {
        number_or(&args[2], ctx, 1.0)?.trunc()
    } else {
        1.0
    };

    let Value::Array(arr) = lookup_array else {
        return Err(EvalError::new("MATCH requires an array"));
    };

    let found = if match_type == 0.0 {
        arr.iter()
            .position(|val| exact_match(&lookup_value, val, true))
    } else {
        approximate_match(&lookup_value, &arr, match_type < 0.0)
    };
    found
        .map(|i| Value::Number((i + 1) as f64))
        .ok_or_else(|| EvalError::new("#N/A: MATCH: value not found"))
}

/// MATCH with match type 1 (`descending` false: the last value at most the
/// lookup value, in values sorted ascending) or -1 (the last value at least
/// it, in values sorted descending). Excel finds it by halving the range,
/// as Gnumeric and `LibreOffice` do, so on values not sorted that way it can
/// find another position than a search from the first would:
/// MATCH(0, {1,3,5,7,9,11}, -1) is 6 in all three (checked in Excel,
/// 2026-11-14). On sorted values with repeats it finds the last of them.
/// Only values of the lookup value's kind take part: numbers (a date
/// counts as its day number, as it is one in the workbook) for a number,
/// texts (ignoring case) for a text.
fn approximate_match(lookup: &Value, arr: &[Value], descending: bool) -> Option<usize> {
    let lookup = Kind::of(lookup);
    let order = |val: &Value| -> Option<std::cmp::Ordering> {
        match (&lookup, Kind::of(val)) {
            (Kind::Number(x), Kind::Number(v)) => v.partial_cmp(x),
            (Kind::Text(x), Kind::Text(v)) if !v.is_empty() => {
                Some(v.to_lowercase().cmp(&x.to_lowercase()))
            },
            (Kind::Boolean(x), Kind::Boolean(v)) => Some(v.cmp(x)),
            _ => None,
        }
    };
    let candidates: Vec<(usize, std::cmp::Ordering)> = arr
        .iter()
        .enumerate()
        .filter_map(|(i, val)| order(val).map(|o| (i, o)))
        .collect();
    // The values that qualify come first in a sorted range: at most the
    // lookup value ascending, at least it descending
    let qualifies = |o: std::cmp::Ordering| {
        if descending {
            o != std::cmp::Ordering::Less
        } else {
            o != std::cmp::Ordering::Greater
        }
    };
    let (mut lo, mut hi) = (0, candidates.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if qualifies(candidates[mid].1) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo.checked_sub(1).map(|last| candidates[last].0)
}

/// A value as MATCH compares it: a date written as text is a number
enum Kind<'v> {
    Number(f64),
    Text(&'v str),
    Boolean(bool),
    Other,
}

impl<'v> Kind<'v> {
    fn of(value: &'v Value) -> Self {
        match value {
            Value::Number(n) => Self::Number(*n),
            Value::Text(t) => excel_date_serial(t).map_or(Self::Text(t), Self::Number),
            Value::Boolean(b) => Self::Boolean(*b),
            _ => Self::Other,
        }
    }
}

/// Evaluate XMATCH function
/// `XMATCH(lookup_value`, `lookup_array`, [`match_mode`], [`search_mode`])
/// `match_mode`: 0 exact (default), -1 exact or the next smaller, 1 exact or
/// the next larger, 2 a wildcard pattern. `search_mode`: 1 first to last
/// (default), -1 last to first, 2 and -2 a binary search of sorted values,
/// which on sorted values finds what a search from the first finds.
pub fn eval_xmatch(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("XMATCH", args, 2, 4)?;

    let lookup_value = evaluate(&args[0], ctx)?;
    let Value::Array(arr) = evaluate(&args[1], ctx)? else {
        return Err(EvalError::new("XMATCH requires an array"));
    };
    let match_mode = if args.len() > 2 {
        number_or(&args[2], ctx, 0.0)?
    } else {
        0.0
    };
    let search_mode = if args.len() > 3 {
        number_or(&args[3], ctx, 1.0)?
    } else {
        1.0
    };
    xmatch_index("XMATCH", &lookup_value, &arr, match_mode, search_mode)?.map_or_else(
        || Err(EvalError::new("#N/A: XMATCH: value not found")),
        |i| Ok(Value::Number((i + 1) as f64)),
    )
}

/// Whether `val` is what an exact match looks for: with wildcards allowed
/// and a text with `*`, `?` or `~`, a text matching it as a pattern
pub(super) fn exact_match(lookup: &Value, val: &Value, wildcards: bool) -> bool {
    if wildcards {
        if let Value::Text(pattern) = lookup {
            if wildcard::has_wildcards(pattern) {
                // An empty text is an empty cell in the workbook: no match
                return matches!(val, Value::Text(t) if !t.is_empty() && wildcard::matches(pattern, t));
            }
        }
    }
    values_equal(lookup, val)
}

/// The position XMATCH and XLOOKUP find, or None (#N/A)
pub(super) fn xmatch_index(
    name: &str,
    lookup_value: &Value,
    arr: &[Value],
    match_mode: f64,
    search_mode: f64,
) -> Result<Option<usize>, EvalError> {
    let match_mode = match_mode.trunc();
    let search_mode = search_mode.trunc();
    if ![-1.0, 0.0, 1.0, 2.0].contains(&match_mode) {
        return Err(EvalError::new(format!(
            "#VALUE!: {name}: match_mode must be 0, -1, 1 or 2, not {match_mode}"
        )));
    }
    if ![-2.0, -1.0, 1.0, 2.0].contains(&search_mode) {
        return Err(EvalError::new(format!(
            "#VALUE!: {name}: search_mode must be 1, -1, 2 or -2, not {search_mode}"
        )));
    }
    let (match_mode, search_mode) = (match_mode as i32, search_mode as i32);
    // Positions in the order they are searched
    let order: Vec<usize> = if search_mode == -1 {
        (0..arr.len()).rev().collect()
    } else {
        (0..arr.len()).collect()
    };
    let wildcards = match_mode == 2;
    if let Some(&i) = order
        .iter()
        .find(|&&i| exact_match(lookup_value, &arr[i], wildcards))
    {
        return Ok(Some(i));
    }
    if match_mode == -1 || match_mode == 1 {
        if let Some(wanted) = lookup_value.as_number() {
            // The closest number on the chosen side; the first met wins a tie
            let mut best: Option<(usize, f64)> = None;
            for &i in &order {
                let Some(v) = arr[i].as_number() else {
                    continue;
                };
                let side = if match_mode < 0 {
                    v < wanted
                } else {
                    v > wanted
                };
                let closer = best.is_none_or(|(_, b)| if match_mode < 0 { v > b } else { v < b });
                if side && closer {
                    best = Some((i, v));
                }
            }
            return Ok(best.map(|(i, _)| i));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)] // Exact float comparison validated against Excel/Gnumeric/R
    use super::*;
    use crate::core::array_calculator::evaluator::tests::eval;
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
    use std::collections::HashMap;

    #[test]
    fn test_match() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Number(10.0),
                Value::Number(20.0),
                Value::Number(30.0),
            ],
        );
        ctx.tables.insert(
            "t".to_string(),
            table.into_iter().map(|(k, v)| (k, v.into())).collect(),
        );

        // Exact match (match_type = 0)
        assert_eq!(
            eval("MATCH(20, t.data, 0)", &ctx).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn test_match_exact() {
        let mut model = ParsedModel::new();
        let mut products = Table::new("products".to_string());
        products.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![101.0, 102.0, 103.0, 104.0]),
        ));
        products.add_column(Column::new(
            "product_name".to_string(),
            ColumnValue::Text(vec![
                "Widget A".to_string(),
                "Widget B".to_string(),
                "Widget C".to_string(),
                "Widget D".to_string(),
            ]),
        ));
        model.add_table(products);
        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "lookup_id".to_string(),
            ColumnValue::Number(vec![102.0, 104.0, 101.0]),
        ));
        sales.add_row_formula(
            "position".to_string(),
            "=MATCH(lookup_id, products.product_id, 0)".to_string(),
        );
        model.add_table(sales);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("sales").unwrap();

        let position = result_table.columns.get("position").unwrap();
        match &position.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 2.0);
                assert_eq!(nums[1], 4.0);
                assert_eq!(nums[2], 1.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_match_function_basic() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0, 50.0]),
        ));
        model.add_table(table);

        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=MATCH(30, data.values, 0)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let val = result.scalars.get("pos").unwrap().value.unwrap();
        assert!((val - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_match_text_exact() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "names".to_string(),
            ColumnValue::Text(vec![
                "Apple".to_string(),
                "Banana".to_string(),
                "Cherry".to_string(),
            ]),
        ));
        model.add_table(data);

        model.add_scalar(
            "pos".to_string(),
            Variable::new(
                "pos".to_string(),
                None,
                Some("=MATCH(\"Banana\", data.names, 0)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let pos = result.scalars.get("pos").unwrap().value.unwrap();
        assert!((pos - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_match_exact_match_found() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("products".to_string());
        lookup_table.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec![
                "Apple".to_string(),
                "Banana".to_string(),
                "Cherry".to_string(),
            ]),
        ));
        lookup_table.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "search".to_string(),
            ColumnValue::Text(vec!["Banana".to_string()]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(search, products.name, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("position") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 2.0);
            }
        }
    }

    #[test]
    fn test_match_exact_match_not_found() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("products".to_string());
        lookup_table.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec!["Apple".to_string(), "Banana".to_string()]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "search".to_string(),
            ColumnValue::Text(vec!["Orange".to_string()]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(search, products.name, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_match_less_than_or_equal_ascending() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "threshold".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![25.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(value, ranges.threshold, 1)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("position") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 2.0);
            }
        }
    }

    #[test]
    fn test_match_greater_than_or_equal_descending() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "threshold".to_string(),
            ColumnValue::Number(vec![40.0, 30.0, 20.0, 10.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![25.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(value, ranges.threshold, -1)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        if let Some(col) = table.columns.get("position") {
            if let ColumnValue::Number(vals) = &col.values {
                assert_eq!(vals[0], 2.0);
            }
        }
    }

    #[test]
    fn test_match_type_2_counts_as_1() {
        // Excel counts a match type by its sign (checked 2026-11-14)
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![10.0, 20.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "search".to_string(),
            ColumnValue::Number(vec![15.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(search, ranges.value, 2)".to_string(),
        );
        model.add_table(data);

        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        let ColumnValue::Number(vals) = &result.tables["data"].columns["position"].values else {
            panic!("position is not a column of numbers");
        };
        // 15 is past 10 and before 20: position 1
        assert_eq!(vals, &vec![1.0]);
    }

    #[test]
    fn test_match_no_value_found_ascending() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "threshold".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![50.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(value, ranges.threshold, 1)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_match_no_value_found_descending() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("ranges".to_string());
        lookup_table.add_column(Column::new(
            "threshold".to_string(),
            ColumnValue::Number(vec![300.0, 200.0, 100.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![500.0]),
        ));
        data.row_formulas.insert(
            "position".to_string(),
            "=MATCH(value, ranges.threshold, -1)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_match_exact_first() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(10, data.values, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_match_exact_last() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(30, data.values, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_match_exact_middle() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(20, data.values, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_match_approx_less() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(25, data.values, 1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(2.0));
    }

    #[test]
    fn test_match_approx_greater() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![30.0, 20.0, 10.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MATCH(25, data.values, -1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    /// Tests of the functions added in 10.1: schedules, conveniences, percentiles.
    /// Their expected values were computed outside Forge (R, Python, by hand); the
    /// Gnumeric suite (`statistics_finance_batch.yaml`) checks the same functions
    /// against two spreadsheet engines.
    mod batch_10_1 {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn xmatch_exact_nearest_and_from_the_end() {
            let ctx = h::table(&[
                ("ten", h::numbers(&[10.0, 20.0, 30.0, 40.0, 50.0])),
                ("dup", h::numbers(&[5.0, 3.0, 5.0, 3.0, 5.0])),
            ]);
            assert!(h::near(h::number_in("XMATCH(30, t.ten)", &ctx), 3.0));
            // 35 is not there: the next smaller is 30 (position 3), the next larger 40 (position 4)
            assert!(h::near(h::number_in("XMATCH(35, t.ten, -1)", &ctx), 3.0));
            assert!(h::near(h::number_in("XMATCH(35, t.ten, 1)", &ctx), 4.0));
            assert!(h::error_in("XMATCH(35, t.ten)", &ctx).contains("not found"));
            // The first or the last of equal values
            assert!(h::near(h::number_in("XMATCH(5, t.dup)", &ctx), 1.0));
            assert!(h::near(h::number_in("XMATCH(5, t.dup, 0, -1)", &ctx), 5.0));
            // Of equal nearest values, the first met in the direction searched
            assert!(h::near(h::number_in("XMATCH(4, t.dup, -1)", &ctx), 2.0));
            assert!(h::near(h::number_in("XMATCH(4, t.dup, -1, -1)", &ctx), 4.0));
            // What is not supported says so
            // A wildcard match (mode 2) and a binary search of sorted values (2, -2)
            assert!(h::near(h::number_in("XMATCH(30, t.ten, 2)", &ctx), 3.0));
            assert!(h::near(h::number_in("XMATCH(30, t.ten, 0, 2)", &ctx), 3.0));
            assert!(h::error_in("XMATCH(30, t.ten, 3)", &ctx).contains("match_mode"));
            assert!(h::error_in("XMATCH(30, t.ten, 0, 3)", &ctx).contains("search_mode"));
        }
    }

    /// The five answers Excel gave differently from Forge when 40 formulas were
    /// checked in Excel on 2026-11-14
    /// (`validation/gnumeric/tests/edge/edge_excel_checks.yaml`), and what
    /// follows from each rule.
    mod checked_in_excel {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn match_counts_a_type_by_its_sign() {
            assert_eq!(h::number("MATCH(3, {1,3,5}, 2)"), 2.0); // Excel
            assert_eq!(h::number("MATCH(3, {5,3,1}, -2)"), 2.0); // Gnumeric
            assert_eq!(h::number("MATCH(3, {1,3,5}, 0.5)"), 2.0); // cut to 0: exact
        }

        #[test]
        fn match_halves_the_range_as_excel_does() {
            // Excel and both engines: 6, where a search from the first finds 1
            assert_eq!(h::number("MATCH(0, {1,3,5,7,9,11}, -1)"), 6.0);
            // The last of repeated values, in all three
            assert_eq!(h::number("MATCH(3, {1,3,3,5}, 1)"), 3.0);
            assert_eq!(h::number("MATCH(2, {1,2,2,2,3}, 1)"), 4.0);
            // Sorted values give what they gave
            assert_eq!(h::number("MATCH(6, {1,3,5,7,9,11}, 1)"), 3.0);
            assert_eq!(h::number("MATCH(6, {11,9,7,5,3,1}, -1)"), 3.0);
            assert_eq!(h::number("MATCH(\"b\", {\"a\",\"B\",\"c\"}, 1)"), 2.0);
            h::fails("MATCH(0, {1,3,5}, 1)");
            h::fails("MATCH(12, {11,9,7}, -1)");
            // A text among numbers is passed over (as Gnumeric does)
            assert_eq!(h::number("MATCH(4, {1,\"x\",3,5}, 1)"), 3.0);
            // A date is its day number
            assert_eq!(
                h::number(
                    "MATCH(DATE(2024,3,1), {\"2024-01-01\",\"2024-02-01\",\"2024-04-01\"}, 1)"
                ),
                2.0
            );
        }
    }
}
