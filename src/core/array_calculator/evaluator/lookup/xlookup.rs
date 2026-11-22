//! XLOOKUP, VLOOKUP, HLOOKUP function implementations
//!
//! Functions

use super::match_fn::{approximate_match, exact_match, xmatch_index};
use crate::core::array_calculator::evaluator::{
    evaluate, require_args_range, EvalContext, EvalError, Expr, Value,
};
use crate::core::array_calculator::evaluator::{evaluate_without_row, number_or};

/// Evaluate XLOOKUP function
/// `XLOOKUP(lookup_value`, `lookup_array`, `return_array`, [`if_not_found`], [`match_mode`], [`search_mode`])
pub fn eval_xlookup(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args_range("XLOOKUP", args, 3, 6)?;

    let lookup_val = evaluate(&args[0], ctx)?;

    let lookup_arr = evaluate(&args[1], ctx)?;
    let return_arr = evaluate(&args[2], ctx)?;

    let if_not_found = if args.len() > 3 {
        Some(evaluate(&args[3], ctx)?)
    } else {
        None
    };
    let match_mode = if args.len() > 4 {
        number_or(&args[4], ctx, 0.0)?
    } else {
        0.0
    };
    let search_mode = if args.len() > 5 {
        number_or(&args[5], ctx, 1.0)?
    } else {
        1.0
    };

    let Value::Array(lookup_values) = lookup_arr else {
        return Err(EvalError::new("XLOOKUP lookup_array must be an array"));
    };

    let Value::Array(return_values) = return_arr else {
        return Err(EvalError::new("XLOOKUP return_array must be an array"));
    };

    if lookup_values.len() != return_values.len() {
        return Err(EvalError::new(format!(
            "XLOOKUP: lookup_array ({}) and return_array ({}) must have same length",
            lookup_values.len(),
            return_values.len()
        )));
    }

    let idx = xmatch_index(
        "XLOOKUP",
        &lookup_val,
        &lookup_values,
        match_mode,
        search_mode,
    )?;
    idx.map_or_else(
        || if_not_found.map_or_else(|| Err(EvalError::new("#N/A: XLOOKUP: no match found")), Ok),
        |i| Ok(return_values.get(i).cloned().unwrap_or(Value::Null)),
    )
}

/// Evaluate VLOOKUP function
/// `VLOOKUP(lookup_value`, `table_array`, `col_index`, [`range_lookup`])
pub fn eval_vlookup(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    first_line_lookup(&ONE_COLUMN, args, ctx)
}

/// Evaluate HLOOKUP function
/// `HLOOKUP(lookup_value`, `table_array`, `row_index`, [`range_lookup`])
pub fn eval_hlookup(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    first_line_lookup(&ONE_ROW, args, ctx)
}

/// What tells VLOOKUP and HLOOKUP apart in their messages
struct Orientation {
    function: &'static str,
    /// The third argument: `col_index` or `row_index`
    index: &'static str,
    /// What the table has one of: `column` or `row`
    line: &'static str,
}

const ONE_COLUMN: Orientation = Orientation {
    function: "VLOOKUP",
    index: "col_index",
    line: "column",
};

const ONE_ROW: Orientation = Orientation {
    function: "HLOOKUP",
    index: "row_index",
    line: "row",
};

/// VLOOKUP and HLOOKUP on a table of one column or one row: Forge has no
/// two-dimensional values (ADR-064), so a table column, a list or an array
/// constant is the whole table, and the only column (or row) to return from
/// is the first. Excel's rules for the index (Microsoft's VLOOKUP page): a
/// fraction is cut to a whole number, below 1 is #VALUE!, past the table is
/// #REF!, both before the search. The approximate search (`range_lookup`
/// TRUE, the default) is MATCH's with match type 1, the exact one MATCH's
/// with match type 0, so VLOOKUP gives what INDEX and MATCH give.
fn first_line_lookup(
    o: &Orientation,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Value, EvalError> {
    require_args_range(o.function, args, 3, 4)?;

    let lookup_val = evaluate(&args[0], ctx)?;
    let table_array = evaluate_without_row(&args[1], ctx)?;
    let index = evaluate(&args[2], ctx)?
        .as_cell_number()
        .ok_or_else(|| {
            EvalError::new(format!(
                "#VALUE!: {}: {} must be a number",
                o.function, o.index
            ))
        })?
        .trunc();
    let range_lookup = if args.len() > 3 {
        evaluate(&args[3], ctx)?.is_truthy()
    } else {
        true
    };

    let Value::Array(lookup_arr) = table_array else {
        return Err(EvalError::new(format!(
            "{}: table_array must be an array",
            o.function
        )));
    };

    if index.is_nan() || index < 1.0 {
        return Err(EvalError::new(format!(
            "#VALUE!: {}: {} {index} is less than 1",
            o.function, o.index
        )));
    }
    if index > 1.0 {
        return Err(EvalError::new(format!(
            "#REF!: {f}: {i} {index} is past the table, which has one {l}: \
             Forge has no two-dimensional values. To return a value from \
             another column, use XLOOKUP(value, t.key, t.other) or \
             INDEX(t.other, MATCH(value, t.key, 0))",
            f = o.function,
            i = o.index,
            l = o.line
        )));
    }

    let idx = if range_lookup {
        approximate_match(&lookup_val, &lookup_arr, false)
    } else {
        lookup_arr
            .iter()
            .position(|v| exact_match(&lookup_val, v, true))
    };

    idx.map_or_else(
        || {
            Err(EvalError::new(format!(
                "#N/A: {}: value not found",
                o.function
            )))
        },
        |i| Ok(lookup_arr[i].clone()),
    )
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::evaluator::tests::eval;
    use crate::core::array_calculator::evaluator::{EvalContext, Value};
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};
    use std::collections::HashMap;

    #[test]
    fn test_vlookup() {
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

        // Exact match (range_lookup = FALSE)
        assert_eq!(
            eval("VLOOKUP(20, t.data, 1, FALSE())", &ctx).unwrap(),
            Value::Number(20.0)
        );
    }

    #[test]
    fn test_xlookup_exact_match() {
        let mut model = ParsedModel::new();

        let mut products = Table::new("products".to_string());
        products.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![101.0, 102.0, 103.0]),
        ));
        products.add_column(Column::new(
            "product_name".to_string(),
            ColumnValue::Text(vec![
                "Widget A".to_string(),
                "Widget B".to_string(),
                "Widget C".to_string(),
            ]),
        ));
        model.add_table(products);

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![102.0, 103.0, 101.0]),
        ));
        sales.add_row_formula(
            "product_name".to_string(),
            "=XLOOKUP(product_id, products.product_id, products.product_name)".to_string(),
        );
        model.add_table(sales);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("sales").unwrap();

        let product_name = result_table.columns.get("product_name").unwrap();
        let texts = product_name.values.texts().unwrap();
        assert_eq!(texts[0], "Widget B");
        assert_eq!(texts[1], "Widget C");
        assert_eq!(texts[2], "Widget A");
    }

    #[test]
    fn test_xlookup_with_if_not_found() {
        let mut model = ParsedModel::new();

        let mut products = Table::new("products".to_string());
        products.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![101.0, 102.0, 103.0]),
        ));
        products.add_column(Column::new(
            "product_name".to_string(),
            ColumnValue::Text(vec![
                "Widget A".to_string(),
                "Widget B".to_string(),
                "Widget C".to_string(),
            ]),
        ));
        model.add_table(products);

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![102.0, 999.0, 101.0]),
        ));
        sales.add_row_formula(
            "product_name".to_string(),
            "=XLOOKUP(product_id, products.product_id, products.product_name, \"Not Found\")"
                .to_string(),
        );
        model.add_table(sales);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("sales").unwrap();

        let product_name = result_table.columns.get("product_name").unwrap();
        let texts = product_name.values.texts().unwrap();
        assert_eq!(texts[0], "Widget B");
        assert_eq!(texts[1], "Not Found");
        assert_eq!(texts[2], "Widget A");
    }

    /// A whole table is not a value in Forge: VLOOKUP over one is refused by
    /// name, and a column of it is the table (one column), so its third
    /// column is #REF! and its first is the value found
    #[test]
    fn test_vlookup_exact_match() {
        let calculate = |formula: &str| {
            let mut model = products_and_data();
            model
                .tables
                .get_mut("data")
                .unwrap()
                .row_formulas
                .insert("found".to_string(), formula.to_string());
            ArrayCalculator::new(model).calculate_all()
        };
        let whole_table = calculate("=VLOOKUP(search_id, products, 3, FALSE)")
            .unwrap_err()
            .to_string();
        assert!(
            whole_table.contains("Unknown variable: products"),
            "{whole_table}"
        );
        let third = calculate("=VLOOKUP(search_id, products.id, 3, FALSE)")
            .unwrap_err()
            .to_string();
        assert!(third.contains("#REF!: VLOOKUP: col_index 3"), "{third}");
        let first = calculate("=VLOOKUP(search_id, products.id, 1, FALSE)").unwrap();
        assert_eq!(
            first.tables["data"].columns["found"].values,
            ColumnValue::Number(vec![102.0])
        );
    }

    fn products_and_data() -> ParsedModel {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("products".to_string());
        lookup_table.add_column(Column::new(
            "id".to_string(),
            ColumnValue::Number(vec![101.0, 102.0, 103.0]),
        ));
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
            ColumnValue::Number(vec![1.50, 0.75, 3.00]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "search_id".to_string(),
            ColumnValue::Number(vec![102.0]),
        ));
        model.add_table(data);
        model
    }

    #[test]
    fn test_xlookup_employee_salary() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("employees".to_string());
        lookup_table.add_column(Column::new(
            "id".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        lookup_table.add_column(Column::new(
            "salary".to_string(),
            ColumnValue::Number(vec![50000.0, 60000.0, 70000.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "emp_id".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        data.row_formulas.insert(
            "emp_salary".to_string(),
            "=XLOOKUP(emp_id, employees.id, employees.salary, 0, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        let col = table.columns.get("emp_salary").unwrap();
        let vals = col.values.numbers().unwrap();
        let vals = vals.as_ref();
        assert_exact!(vals[0], 60000.0);
    }

    #[test]
    fn test_xlookup_default_value() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("employees".to_string());
        lookup_table.add_column(Column::new(
            "id".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        lookup_table.add_column(Column::new(
            "salary".to_string(),
            ColumnValue::Number(vec![50000.0, 60000.0]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "emp_id".to_string(),
            ColumnValue::Number(vec![99.0]),
        ));
        data.row_formulas.insert(
            "emp_salary".to_string(),
            "=XLOOKUP(emp_id, employees.id, employees.salary, -1, 0)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok());
        let model = result.unwrap();
        let table = model.tables.get("data").unwrap();
        let col = table.columns.get("emp_salary").unwrap();
        let vals = col.values.numbers().unwrap();
        let vals = vals.as_ref();
        assert_exact!(vals[0], -1.0);
    }

    #[test]
    fn test_xlookup_not_found_fallback() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("items".to_string());
        data.add_column(Column::new(
            "code".to_string(),
            ColumnValue::Text(vec!["A1".to_string(), "B2".to_string(), "C3".to_string()]),
        ));
        data.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=XLOOKUP(\"D4\", items.code, items.value, -1)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "XLOOKUP not found fallback should calculate successfully"
        );
        let model_result = result.unwrap();
        let val = model_result.scalars.get("result").unwrap().value.unwrap();
        assert_exact!(val, -1.0);
    }

    #[test]
    fn test_xlookup_exact() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
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
                Some("=XLOOKUP(2, data.keys, data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(20.0));
    }

    #[test]
    fn test_xlookup_not_found() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
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
                Some("=XLOOKUP(5, data.keys, data.values, 0)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_xlookup_first_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
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
                Some("=XLOOKUP(1, data.keys, data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[test]
    fn test_xlookup_last_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
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
                Some("=XLOOKUP(3, data.keys, data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
    }

    /// The table argument of VLOOKUP, HLOOKUP and OFFSET in a row formula
    /// reads the whole column (it is evaluated with the row hidden, no
    /// longer in a copy of the context)
    #[test]
    fn test_lookups_in_a_row_formula_read_the_whole_column() {
        let mut model = ParsedModel::new();
        let mut t = Table::new("t".to_string());
        t.add_column(Column::new(
            "k".to_string(),
            ColumnValue::Number(vec![3.0, 1.0, 2.0]),
        ));
        t.add_column(Column::new(
            "v".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        for (name, formula) in [
            ("vl", "=VLOOKUP(k, t.k, 1, FALSE)"),
            ("hl", "=HLOOKUP(k, t.k, 1, FALSE)"),
            ("off", "=OFFSET(t.v, k - 1, 0)"),
            ("total", "=k + SUM(t.v)"),
        ] {
            t.row_formulas.insert(name.to_string(), formula.to_string());
        }
        model.add_table(t);
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        let column = |name: &str| result.tables["t"].columns[name].values.clone();
        let numbers = |values: &[f64]| ColumnValue::Number(values.to_vec());
        assert_eq!(column("vl"), numbers(&[3.0, 1.0, 2.0]));
        assert_eq!(column("hl"), numbers(&[3.0, 1.0, 2.0]));
        assert_eq!(column("off"), numbers(&[30.0, 10.0, 20.0]));
        assert_eq!(column("total"), numbers(&[63.0, 61.0, 62.0]));
    }

    /// Wrong answers found by sweeping the statistical, lookup, math and text
    /// functions against Gnumeric and `LibreOffice` (`validation/sweep`), each pinned
    /// to what both engines give, or to Excel's documentation where they differ.
    mod sweep {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn xlookup_searches_from_the_end_when_asked() {
            assert_exact!(h::sweep_number("XLOOKUP(2, t.lu, t.pr, -1, 0, -1)"), 40.0);
            assert_exact!(h::sweep_number("XLOOKUP(2, t.lu, t.pr, -1, 0, 1)"), 20.0);
            assert_exact!(h::sweep_number("XLOOKUP(3, t.lu, t.pr, -1, 1, -1)"), 60.0);
        }
    }

    /// VLOOKUP and HLOOKUP read their table as one column (VLOOKUP) or one
    /// row (HLOOKUP), since Forge has no two-dimensional values, so the only
    /// column or row to return from is the first. Each answer here was checked
    /// in `LibreOffice` 2026-11-17 on a range of cells; Excel's documentation
    /// gives #VALUE! below 1 and #REF! past the table, where `LibreOffice`
    /// gives Err:502 for both.
    mod one_column {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn an_index_past_the_table_is_ref() {
            let ctx = h::table(&[("v", h::numbers(&[1.0, 2.0, 3.0]))]);
            for formula in [
                "VLOOKUP(2, {1;2;3}, 5, FALSE)",
                "VLOOKUP(2, t.v, 2, FALSE)",
                "VLOOKUP(2.5, t.v, 2, TRUE)",
                "HLOOKUP(2, {1,2,3}, 2, FALSE)",
                "HLOOKUP(2, t.v, 3)",
            ] {
                h::fails_in(formula, &ctx, "#REF!");
            }
        }

        /// The index is checked before the search: a value that is not there
        /// is still #REF!, not #N/A
        #[test]
        fn the_index_is_checked_before_the_search() {
            let ctx = crate::core::array_calculator::evaluator::EvalContext::new();
            h::fails_in("VLOOKUP(9, {1;2;3}, 5, FALSE)", &ctx, "#REF!");
            h::fails_in("HLOOKUP(9, {1,2,3}, 2, FALSE)", &ctx, "#REF!");
            h::fails_in("VLOOKUP(9, {1;2;3}, 0, FALSE)", &ctx, "#VALUE!");
        }

        #[test]
        fn an_index_below_one_is_value() {
            let ctx = crate::core::array_calculator::evaluator::EvalContext::new();
            for index in ["0", "-1", "0.5", "-0.5"] {
                h::fails_in(
                    &format!("VLOOKUP(2, {{1;2;3}}, {index}, FALSE)"),
                    &ctx,
                    "#VALUE!",
                );
                h::fails_in(
                    &format!("HLOOKUP(2, {{1,2,3}}, {index}, FALSE)"),
                    &ctx,
                    "#VALUE!",
                );
            }
        }

        /// A fraction is cut, and TRUE and "1" are 1, as in `LibreOffice`
        #[test]
        fn an_index_is_cut_to_a_whole_number() {
            for index in ["1", "1.9", "TRUE", "\"1\""] {
                h::is_exactly(&format!("VLOOKUP(2, {{1;2;3}}, {index}, FALSE)"), 2.0);
                h::is_exactly(&format!("HLOOKUP(2, {{1,2,3}}, {index}, FALSE)"), 2.0);
            }
        }

        /// A column given for the index has no single value
        #[test]
        fn a_column_for_the_index_fails() {
            let ctx = h::table(&[("v", h::numbers(&[1.0, 2.0, 3.0]))]);
            h::fails_in(
                "VLOOKUP(2, t.v, t.v, FALSE)",
                &ctx,
                "col_index must be a number",
            );
            h::fails_in(
                "HLOOKUP(2, t.v, t.v, FALSE)",
                &ctx,
                "row_index must be a number",
            );
        }

        /// A single number is no table to search
        #[test]
        fn a_number_for_the_table_fails() {
            let ctx = crate::core::array_calculator::evaluator::EvalContext::new();
            h::fails_in(
                "VLOOKUP(2, 2, 1)",
                &ctx,
                "VLOOKUP: table_array must be an array",
            );
            h::fails_in(
                "HLOOKUP(2, 2, 1)",
                &ctx,
                "HLOOKUP: table_array must be an array",
            );
        }

        /// The approximate search is MATCH's with match type 1: the same
        /// halving search, so on values not sorted ascending it finds what
        /// MATCH finds. `LibreOffice` gives VLOOKUP(v, range, 1, TRUE) =
        /// INDEX(range, MATCH(v, range, 1)) on every one of these.
        #[test]
        fn the_approximate_search_is_match_type_1() {
            let ctx = h::table(&[
                ("a", h::numbers(&[5.0, 1.0, 9.0, 2.0])),
                ("b", h::numbers(&[4.0, 2.0, 9.0, 2.0, 7.0, 4.0])),
                ("s", h::numbers(&[1.0, 2.0, 3.0])),
            ]);
            for column in ["a", "b", "s"] {
                for v in [0.0, 1.0, 2.0, 2.5, 3.0, 4.0, 6.0, 8.0, 10.0] {
                    let via_match = format!("INDEX(t.{column}, MATCH({v}, t.{column}, 1))");
                    for lookup in [
                        format!("VLOOKUP({v}, t.{column}, 1, TRUE)"),
                        format!("VLOOKUP({v}, t.{column}, 1)"),
                        format!("HLOOKUP({v}, t.{column}, 1, TRUE)"),
                    ] {
                        match h::eval(&via_match, &ctx) {
                            Ok(want) => assert_eq!(h::eval(&lookup, &ctx), Ok(want), "{lookup}"),
                            Err(_) => h::fails_in(&lookup, &ctx, "#N/A"),
                        }
                    }
                }
            }
            // Where the two searches part: a search from the first finds 2,
            // the largest value at most 3; MATCH's halving search finds 1
            h::is_exactly_in("VLOOKUP(3, t.a, 1, TRUE)", &ctx, 1.0);
            // On sorted values, the largest at most the lookup value
            h::is_exactly_in("VLOOKUP(2.5, t.s, 1, TRUE)", &ctx, 2.0);
            h::fails_in("VLOOKUP(0, t.s, 1, TRUE)", &ctx, "#N/A");
        }

        /// Text takes part in the approximate search, ignoring case, as in
        /// `LibreOffice`: "b" finds "a", the last text before it
        #[test]
        fn text_is_found_approximately_and_exactly() {
            let ctx = h::table(&[("w", h::texts(&["a", "c"]))]);
            assert_eq!(h::text_in("VLOOKUP(\"b\", t.w, 1, TRUE)", &ctx), "a");
            assert_eq!(h::text_in("HLOOKUP(\"b\", t.w, 1)", &ctx), "a");
            assert_eq!(h::text_in("VLOOKUP(\"A\", t.w, 1, FALSE)", &ctx), "a");
            h::fails_in("VLOOKUP(\"b\", t.w, 1, FALSE)", &ctx, "#N/A");
        }
    }

    /// Text where a number goes, or an argument that is an error, is an error
    #[test]
    fn an_argument_that_is_no_number_fails() {
        use crate::core::array_calculator::evaluator::tests as h;
        let ctx = h::table(&[("v", h::numbers(&[1.0, 2.0, 3.0]))]);
        assert!(
            h::error_in("VLOOKUP(1, t.v, \"x\")", &ctx).contains("col_index must be a number"),
            "VLOOKUP(1, t.v, \"x\")"
        );
        assert!(
            h::error_in("HLOOKUP(1, t.v, \"x\")", &ctx).contains("row_index must be a number"),
            "HLOOKUP(1, t.v, \"x\")"
        );
    }

    /// An error in any argument is passed on, text for VLOOKUP's and
    /// HLOOKUP's index is refused, and what is looked in and returned from
    /// must be arrays of one length
    #[test]
    fn each_argument_is_checked() {
        use crate::core::array_calculator::evaluator::tests::{each_argument, fails_with};
        each_argument(
            "XLOOKUP",
            &[
                ("2", None),
                ("{1, 2, 3}", None),
                ("{10, 20, 30}", None),
                ("0", None),
                ("0", None),
                ("1", None),
            ],
        );
        for name in ["VLOOKUP", "HLOOKUP"] {
            let index = if name == "VLOOKUP" {
                "col_index"
            } else {
                "row_index"
            };
            let message = format!("#VALUE!: {name}: {index} must be a number");
            each_argument(
                name,
                &[
                    ("2", None),
                    ("{1, 2, 3}", None),
                    ("1", Some(&message)),
                    ("FALSE", None),
                ],
            );
        }
        fails_with(
            "XLOOKUP(1, 5, {1})",
            "XLOOKUP lookup_array must be an array",
        );
        fails_with(
            "XLOOKUP(1, {1}, 5)",
            "XLOOKUP return_array must be an array",
        );
        fails_with(
            "XLOOKUP(1, {1, 2}, {1})",
            "XLOOKUP: lookup_array (2) and return_array (1) must have same length",
        );
        fails_with("XLOOKUP(1, {1}, {1}, 0, 5)", "XLOOKUP");
    }
}
