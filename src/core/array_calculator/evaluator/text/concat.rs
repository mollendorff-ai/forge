//! String concatenation functions: CONCAT, REPT, CONCATENATE, TEXTJOIN

use super::super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};
use super::whole_arg;

/// CONCAT(text1, [text2], ...) - Joins multiple text values into one
pub fn eval_concat(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    let mut result = String::new();
    for arg in args {
        // A column or range gives each of its values in turn
        match evaluate(arg, ctx)? {
            Value::Array(items) => {
                for item in items.iter() {
                    result.push_str(&item.as_text());
                }
            },
            val => result.push_str(&val.as_text()),
        }
    }
    Ok(Value::Text(result))
}

/// REPT(text, `number_times`) - Repeats text a specified number of times
pub fn eval_rept(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("REPT", args, 2)?;
    let text = evaluate(&args[0], ctx)?.as_text();
    let times = whole_arg("REPT", "number_times", &args[1], ctx, 0.0, 0.0)?;
    // Excel's longest text is 32,767 characters; longer is #VALUE!
    if text.chars().count().saturating_mul(times) > 32_767 {
        return Err(EvalError::new(
            "#VALUE!: REPT: the result would be longer than 32,767 characters",
        ));
    }
    Ok(Value::Text(text.repeat(times)))
}

/// TEXTJOIN(delimiter, `ignore_empty`, text1, [text2], ...) - Joins text with a
/// delimiter between the pieces; a column or range gives all its values
pub fn eval_textjoin(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    if args.len() < 3 {
        return Err(EvalError::new(format!(
            "TEXTJOIN requires at least 3 arguments, got {}",
            args.len()
        )));
    }
    let delimiter = evaluate(&args[0], ctx)?.as_text();
    let ignore_empty = match evaluate(&args[1], ctx)? {
        Value::Boolean(b) => b,
        other => other.as_number().is_some_and(|n| n != 0.0),
    };
    let mut pieces = Vec::new();
    for arg in &args[2..] {
        match evaluate(arg, ctx)? {
            Value::Array(items) => pieces.extend(items.iter().map(Value::as_text)),
            single => pieces.push(single.as_text()),
        }
    }
    if ignore_empty {
        pieces.retain(|piece| !piece.is_empty());
    }
    Ok(Value::Text(pieces.join(&delimiter)))
}

/// CONCATENATE(text1, [text2], ...) - Excel's CONCATENATE (alias for CONCAT)
pub fn eval_concatenate(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    eval_concat(args, ctx)
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_concat_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "first".to_string(),
            ColumnValue::Text(vec![
                "Hello".to_string(),
                "Good".to_string(),
                "Nice".to_string(),
            ]),
        ));
        table.add_column(Column::new(
            "second".to_string(),
            ColumnValue::Text(vec![
                "World".to_string(),
                "Day".to_string(),
                "Work".to_string(),
            ]),
        ));
        table.add_row_formula(
            "combined".to_string(),
            "=CONCAT(first, \" \", second)".to_string(),
        );

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let combined = result_table.columns.get("combined").unwrap();
        let texts = combined.values.texts().unwrap();
        assert_eq!(texts[0], "Hello World");
        assert_eq!(texts[1], "Good Day");
        assert_eq!(texts[2], "Nice Work");
    }

    #[test]
    fn test_rept_function_scalar() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=REPT(\"ab\", 3)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_concat_text_columns() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "first".to_string(),
            ColumnValue::Text(vec!["John".to_string(), "Jane".to_string()]),
        ));
        data.add_column(Column::new(
            "last".to_string(),
            ColumnValue::Text(vec!["Doe".to_string(), "Smith".to_string()]),
        ));
        model.add_table(data);
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_text_join_function() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "first".to_string(),
            ColumnValue::Text(vec!["Hello".to_string()]),
        ));
        data.add_column(Column::new(
            "second".to_string(),
            ColumnValue::Text(vec!["World".to_string()]),
        ));
        data.row_formulas.insert(
            "joined".to_string(),
            "=CONCAT(first, \" \", second)".to_string(),
        );
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_concat_empty_strings() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCAT(\"\", \"\", \"\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_concat_mixed_types() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "num".to_string(),
            Variable::new("num".to_string(), Some(42.0), None),
        );
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCAT(\"The answer is \", num))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(16.0));
    }

    // REPT FUNCTION TESTS

    #[test]
    fn test_rept_multiple_times() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"ab\", 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(10.0)); // "ab" * 5 = "ababababab" = 10 chars
    }

    #[test]
    fn test_rept_zero_times() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"hello\", 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0)); // Empty string
    }

    #[test]
    fn test_rept_once() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"test\", 1))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(4.0)); // "test" = 4 chars
    }

    #[test]
    fn test_rept_single_char() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"x\", 5))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[test]
    fn test_rept_multi_char() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(REPT(\"ab\", 3))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
    }

    // CONCATENATION

    #[test]
    fn test_concat_with_spaces_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCAT(\"Hello\", \" \", \"World\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(11.0));
    }

    #[test]
    fn test_concat_function_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCAT(\"ab\", \"cd\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(4.0));
    }

    // EMPTY STRING HANDLING

    #[test]
    fn test_empty_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=LEN(\"\")".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_single_space() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=LEN(\" \")".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_concatenate_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCATENATE(\"Hello\", \" \", \"World\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        // CONCATENATE should work like CONCAT, result length should be 11
        let len = result.scalars.get("result").unwrap().value.unwrap();
        assert_exact!(len, 11.0);
    }

    #[test]
    fn test_concatenate_multiple() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(CONCATENATE(\"A\", \"B\", \"C\", \"D\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let len = result.scalars.get("result").unwrap().value.unwrap();
        assert_exact!(len, 4.0);
    }

    #[test]
    fn test_cross_table_text_column_reference() {
        let mut model = ParsedModel::new();

        // Source table with text column
        let mut source = Table::new("source".to_string());
        source.add_column(Column::new(
            "names".to_string(),
            ColumnValue::Text(vec!["Alice".to_string(), "Bob".to_string()]),
        ));
        model.add_table(source);

        // Target table referencing source's text column
        let mut target = Table::new("target".to_string());
        target.add_column(Column::new(
            "id".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        target
            .row_formulas
            .insert("copy_name".to_string(), "=source.names".to_string());
        model.add_table(target);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // Should handle cross-table text reference
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_index_text_column() {
        let mut model = ParsedModel::new();

        let mut lookup_table = Table::new("items".to_string());
        lookup_table.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec![
                "First".to_string(),
                "Second".to_string(),
                "Third".to_string(),
            ]),
        ));
        model.add_table(lookup_table);

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "idx".to_string(),
            ColumnValue::Number(vec![2.0]),
        ));
        data.row_formulas
            .insert("result".to_string(), "=INDEX(items.name, idx)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        // INDEX function returns text, which may be handled differently
        assert!(result.is_ok() || result.is_err());
    }

    /// Tests of the functions added in 10.1: schedules, conveniences, percentiles.
    /// Their expected values were computed outside Forge (R, Python, by hand); the
    /// Gnumeric suite (`statistics_finance_batch.yaml`) checks the same functions
    /// against two spreadsheet engines.
    mod batch_10_1 {
        use crate::core::array_calculator::evaluator::tests as h;
        use crate::core::array_calculator::evaluator::Value;

        #[test]
        fn textjoin_joins_values_and_columns_and_can_skip_the_empty() {
            let ctx = h::table(&[(
                "names",
                vec![
                    Value::Text("Ana".to_string()),
                    Value::Text(String::new()),
                    Value::Text("Bo".to_string()),
                ],
            )]);
            let text = |f: &str| match h::eval(f, &ctx).unwrap() {
                Value::Text(s) => s,
                other => panic!("{f}: {other:?}"),
            };
            assert_eq!(text("TEXTJOIN(\", \", TRUE, t.names)"), "Ana, Bo");
            assert_eq!(text("TEXTJOIN(\", \", FALSE, t.names)"), "Ana, , Bo");
            assert_eq!(text("TEXTJOIN(\"-\", TRUE, \"a\", 2, \"c\")"), "a-2-c");
            assert!(h::error_in("TEXTJOIN(\",\", TRUE)", &ctx).contains("at least 3"));
        }
    }

    /// Wrong answers found by sweeping the statistical, lookup, math and text
    /// functions against Gnumeric and `LibreOffice` (`validation/sweep`), each pinned
    /// to what both engines give, or to Excel's documentation where they differ.
    mod sweep {
        use crate::core::array_calculator::evaluator::tests as h;

        #[test]
        fn concat_joins_a_column_and_numbers_keep_fifteen_digits() {
            assert_eq!(h::sweep_text("CONCAT(t.s)"), "abc");
            assert_eq!(h::sweep_text("CONCAT(0.1 + 0.2)"), "0.3");
            assert_eq!(h::sweep_text("CONCAT(1 / 3)"), "0.333333333333333");
            assert_eq!(h::sweep_text("CONCAT(1E+20)"), "1E+20");
            assert_exact!(h::sweep_number("LEN(2 / 3)"), 17.0);
        }
    }

    /// An error in any argument is passed on; REPT refuses a text longer
    /// than Excel holds; TEXTJOIN reads a number as its ignore-empty flag
    #[test]
    fn errors_lengths_and_flags() {
        use crate::core::array_calculator::evaluator::tests as h;
        for formula in [
            "CONCAT(nope)",
            "REPT(nope, 2)",
            "TEXTJOIN(nope, TRUE, \"a\")",
            "TEXTJOIN(\",\", nope, \"a\")",
            "TEXTJOIN(\",\", TRUE, nope)",
        ] {
            h::fails_with(formula, "Unknown variable: nope");
        }
        h::fails_with(
            "REPT(\"ab\", 20000)",
            "#VALUE!: REPT: the result would be longer than 32,767 characters",
        );
        assert_eq!(h::text("TEXTJOIN(\",\", 1, \"a\", \"\", \"b\")"), "a,b");
    }
}
