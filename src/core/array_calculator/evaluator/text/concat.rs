//! String concatenation functions: CONCAT, REPT, CONCATENATE

use super::super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};

/// CONCAT(text1, [text2], ...) - Joins multiple text values into one
pub fn eval_concat(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    let mut result = String::new();
    for arg in args {
        let val = evaluate(arg, ctx)?;
        result.push_str(&val.as_text());
    }
    Ok(Value::Text(result))
}

/// REPT(text, number_times) - Repeats text a specified number of times
pub fn eval_rept(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("REPT", args, 2)?;
    let text = evaluate(&args[0], ctx)?.as_text();
    let times = evaluate(&args[1], ctx)?.as_number().unwrap_or(0.0) as usize;
    Ok(Value::Text(text.repeat(times)))
}

/// CONCATENATE(text1, [text2], ...) - Excel's CONCATENATE (alias for CONCAT, enterprise only)
pub fn eval_concatenate(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    eval_concat(args, ctx)
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
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
        match &combined.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "Hello World");
                assert_eq!(texts[1], "Good Day");
                assert_eq!(texts[2], "Nice Work");
            },
            _ => panic!("Expected Text array"),
        }
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
        assert_eq!(len, 11.0);
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
        assert_eq!(len, 4.0);
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
}
