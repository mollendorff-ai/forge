//! Case conversion and whitespace functions: UPPER, LOWER, TRIM

use super::super::{evaluate, require_args, EvalContext, EvalError, Expr, Value};

/// UPPER(text) - Converts text to uppercase
pub fn eval_upper(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("UPPER", args, 1)?;
    let val = evaluate(&args[0], ctx)?;
    Ok(Value::Text(val.as_text().to_uppercase()))
}

/// LOWER(text) - Converts text to lowercase
pub fn eval_lower(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("LOWER", args, 1)?;
    let val = evaluate(&args[0], ctx)?;
    Ok(Value::Text(val.as_text().to_lowercase()))
}

/// TRIM(text) - Removes leading/trailing spaces and collapses internal spaces
pub fn eval_trim(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("TRIM", args, 1)?;
    let val = evaluate(&args[0], ctx)?;
    // Excel TRIM: removes leading/trailing spaces AND collapses multiple internal spaces
    let text = val.as_text();
    let trimmed: String = text.split_whitespace().collect::<Vec<&str>>().join(" ");
    Ok(Value::Text(trimmed))
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_upper_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec![
                "hello".to_string(),
                "world".to_string(),
                "Test".to_string(),
            ]),
        ));
        table.add_row_formula("upper".to_string(), "=UPPER(text)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let upper = result_table.columns.get("upper").unwrap();
        let texts = upper.values.texts().unwrap();
        assert_eq!(texts[0], "HELLO");
        assert_eq!(texts[1], "WORLD");
        assert_eq!(texts[2], "TEST");
    }

    #[test]
    fn test_lower_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec![
                "HELLO".to_string(),
                "WORLD".to_string(),
                "Test".to_string(),
            ]),
        ));
        table.add_row_formula("lower".to_string(), "=LOWER(text)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let lower = result_table.columns.get("lower").unwrap();
        let texts = lower.values.texts().unwrap();
        assert_eq!(texts[0], "hello");
        assert_eq!(texts[1], "world");
        assert_eq!(texts[2], "test");
    }

    #[test]
    fn test_trim_function_arraycalc() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec![
                "  Hello  ".to_string(),
                " World ".to_string(),
                "  Test".to_string(),
            ]),
        ));
        table.add_row_formula("trimmed".to_string(), "=TRIM(text)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let trimmed = result_table.columns.get("trimmed").unwrap();
        let texts = trimmed.values.texts().unwrap();
        assert_eq!(texts[0], "Hello");
        assert_eq!(texts[1], "World");
        assert_eq!(texts[2], "Test");
    }

    #[test]
    fn test_text_functions_combined() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec!["  hello  ".to_string(), "  WORLD  ".to_string()]),
        ));
        table.add_row_formula("processed".to_string(), "=UPPER(TRIM(text))".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let processed = result_table.columns.get("processed").unwrap();
        let texts = processed.values.texts().unwrap();
        assert_eq!(texts[0], "HELLO");
        assert_eq!(texts[1], "WORLD");
    }

    #[test]
    fn test_trim_function_whitespace() {
        let mut model = ParsedModel::new();

        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "text".to_string(),
            ColumnValue::Text(vec!["  hello  ".to_string(), " world ".to_string()]),
        ));
        table.add_row_formula("trimmed".to_string(), "=TRIM(text)".to_string());
        model.add_table(table);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let table = result.tables.get("data").unwrap();
        let trimmed = table.columns.get("trimmed").unwrap();
        assert_eq!(
            trimmed.values,
            ColumnValue::Text(vec!["hello".to_string(), "world".to_string()])
        );
    }

    #[test]
    fn test_text_column_result() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec!["alice".to_string(), "bob".to_string()]),
        ));
        data.row_formulas
            .insert("upper_name".to_string(), "=UPPER(name)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let col = result
            .tables
            .get("data")
            .unwrap()
            .columns
            .get("upper_name")
            .unwrap();
        assert_eq!(
            col.values,
            ColumnValue::Text(vec!["ALICE".to_string(), "BOB".to_string()])
        );
    }

    #[test]
    fn test_text_column_in_rowwise_formula() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec!["Alice".to_string(), "Bob".to_string()]),
        ));
        data.add_column(Column::new(
            "score".to_string(),
            ColumnValue::Number(vec![100.0, 90.0]),
        ));
        // Use UPPER function on text column
        data.row_formulas
            .insert("upper_name".to_string(), "=UPPER(name)".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        let model = result.unwrap();
        assert_eq!(
            model.tables["data"].columns["upper_name"].values,
            ColumnValue::Text(vec!["ALICE".to_string(), "BOB".to_string()])
        );
    }

    // Additional tests for functions with low coverage (v6.0.0 Phase 2)

    #[test]
    fn test_upper_empty_string_arraycalc() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=UPPER(\"\")".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        // Just verify calculation succeeds
        // A scalar holds a number, so a formula giving text is refused there
        let error = calculator.calculate_all().unwrap_err().to_string();
        assert!(error.contains("result is not a number"), "{error}");
        assert_eq!(
            crate::core::array_calculator::evaluator::tests::text("UPPER(\"\")"),
            ""
        );
    }

    #[test]
    fn test_upper_numbers_unchanged() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=UPPER(\"abc123xyz\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        // A scalar holds a number, so a formula giving text is refused there
        let error = calculator.calculate_all().unwrap_err().to_string();
        assert!(error.contains("result is not a number"), "{error}");
        assert_eq!(
            crate::core::array_calculator::evaluator::tests::text("UPPER(\"abc123xyz\")"),
            "ABC123XYZ"
        );
    }

    #[test]
    fn test_upper_special_chars() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=UPPER(\"hello-world_test\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        // A scalar holds a number, so a formula giving text is refused there
        let error = calculator.calculate_all().unwrap_err().to_string();
        assert!(error.contains("result is not a number"), "{error}");
        assert_eq!(
            crate::core::array_calculator::evaluator::tests::text("UPPER(\"hello-world_test\")"),
            "HELLO-WORLD_TEST"
        );
    }

    #[test]
    fn test_lower_empty_string_arraycalc() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new("result".to_string(), None, Some("=LOWER(\"\")".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        // A scalar holds a number, so a formula giving text is refused there
        let error = calculator.calculate_all().unwrap_err().to_string();
        assert!(error.contains("result is not a number"), "{error}");
        assert_eq!(
            crate::core::array_calculator::evaluator::tests::text("LOWER(\"\")"),
            ""
        );
    }

    #[test]
    fn test_lower_numbers_unchanged() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LOWER(\"ABC123XYZ\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        // A scalar holds a number, so a formula giving text is refused there
        let error = calculator.calculate_all().unwrap_err().to_string();
        assert!(error.contains("result is not a number"), "{error}");
        assert_eq!(
            crate::core::array_calculator::evaluator::tests::text("LOWER(\"ABC123XYZ\")"),
            "abc123xyz"
        );
    }

    #[test]
    fn test_lower_mixed_case() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LOWER(\"HeLLo WoRLd\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        // A scalar holds a number, so a formula giving text is refused there
        let error = calculator.calculate_all().unwrap_err().to_string();
        assert!(error.contains("result is not a number"), "{error}");
        assert_eq!(
            crate::core::array_calculator::evaluator::tests::text("LOWER(\"HeLLo WoRLd\")"),
            "hello world"
        );
    }

    #[test]
    fn test_trim_multiple_spaces() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"     hello     \"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(5.0));
    }

    #[test]
    fn test_trim_tabs_and_newlines() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"\t\nhello\t\n\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(5.0));
    }

    #[test]
    fn test_trim_only_spaces_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"     \"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // All spaces should be trimmed, leaving empty string
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_upper_empty_string_verified() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(UPPER(\"\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_lower_empty_string_verified() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LOWER(\"\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_trim_empty_string_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        assert_eq!(var.value, Some(0.0));
    }

    #[test]
    fn test_upper_with_numbers_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(UPPER(\"test123\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Numbers should remain unchanged
        assert_eq!(var.value, Some(7.0));
    }

    #[test]
    fn test_lower_with_numbers_edge() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LOWER(\"TEST123\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        let var = result.scalars.get("result").unwrap();
        // Numbers should remain unchanged
        assert_eq!(var.value, Some(7.0));
    }

    // TRIM BEHAVIOR

    #[test]
    fn test_trim_internal_spaces() {
        // Excel TRIM collapses multiple internal spaces to single space
        // "  a  b  " -> "a b" (length 3)
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"  a  b  \"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_trim_only_spaces_string() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(TRIM(\"   \"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    // UPPER/LOWER

    #[test]
    fn test_upper_length() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(UPPER(\"abc\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_lower_length() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=LEN(LOWER(\"ABC\"))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_mixed_math_and_text_functions() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());

        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.234, 5.678, 9.012]),
        ));
        table.add_column(Column::new(
            "labels".to_string(),
            ColumnValue::Text(vec![
                "item".to_string(),
                "data".to_string(),
                "test".to_string(),
            ]),
        ));
        table.add_row_formula("rounded".to_string(), "=ROUND(values, 1)".to_string());
        table.add_row_formula("upper_labels".to_string(), "=UPPER(labels)".to_string());

        model.add_table(table);
        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("data").unwrap();

        let rounded = result_table.columns.get("rounded").unwrap();
        let nums = rounded.values.numbers().unwrap();
        assert_exact!(nums[0], 1.2);
        assert_exact!(nums[1], 5.7);
        assert_exact!(nums[2], 9.0);

        let upper_labels = result_table.columns.get("upper_labels").unwrap();
        let texts = upper_labels.values.texts().unwrap();
        assert_eq!(texts[0], "ITEM");
        assert_eq!(texts[1], "DATA");
        assert_eq!(texts[2], "TEST");
    }

    #[test]
    fn an_error_in_the_argument_is_the_error() {
        use crate::core::array_calculator::evaluator::tests::each_argument;
        for name in ["UPPER", "LOWER", "TRIM"] {
            each_argument(name, &[("\"a\"", None)]);
        }
    }
}
