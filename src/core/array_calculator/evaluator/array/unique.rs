//! UNIQUE and COUNTUNIQUE array functions

use crate::core::array_calculator::evaluator::{
    collect_values_as_vec, require_args, EvalContext, EvalError, Expr, Value,
};

/// Evaluate UNIQUE function - returns unique values from an array
pub fn eval_unique(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("UNIQUE", args, 1)?;
    let values = collect_values_as_vec(&args[0], ctx)?;
    let mut seen = Vec::new();
    for v in values {
        let text = v.as_text();
        if !seen.iter().any(|s: &Value| s.as_text() == text) {
            seen.push(v);
        }
    }
    Ok(Value::Array(seen))
}

/// Evaluate COUNTUNIQUE function - returns count of unique values
pub fn eval_countunique(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    require_args("COUNTUNIQUE", args, 1)?;
    let values = collect_values_as_vec(&args[0], ctx)?;
    let mut seen = std::collections::HashSet::new();
    for v in values {
        seen.insert(v.as_text());
    }
    Ok(Value::Number(seen.len() as f64))
}

#[cfg(test)]
mod tests {
    use super::super::super::tests::eval;
    use crate::core::array_calculator::evaluator::{EvalContext, Value};
    use std::collections::HashMap;

    #[test]
    fn test_unique() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Text("A".to_string()),
                Value::Text("B".to_string()),
                Value::Text("A".to_string()),
                Value::Text("C".to_string()),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        let result = eval("UNIQUE(t.data)", &ctx).unwrap();
        if let Value::Array(arr) = result {
            assert_eq!(arr.len(), 3); // A, B, C
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_countunique() {
        let mut ctx = EvalContext::new();
        let mut table = HashMap::new();
        table.insert(
            "data".to_string(),
            vec![
                Value::Text("A".to_string()),
                Value::Text("B".to_string()),
                Value::Text("A".to_string()),
                Value::Text("C".to_string()),
            ],
        );
        ctx.tables.insert("t".to_string(), table);

        assert_eq!(
            eval("COUNTUNIQUE(t.data)", &ctx).unwrap(),
            Value::Number(3.0)
        );
    }
}

#[cfg(test)]
mod integration_tests {
    #![allow(clippy::approx_constant)]

    use crate::core::array_calculator::ArrayCalculator;
    #[allow(unused_imports)]
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_countunique_function() {
        let mut model = ParsedModel::new();

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "product".to_string(),
            ColumnValue::Text(vec![
                "Apple".to_string(),
                "Banana".to_string(),
                "Apple".to_string(),
                "Orange".to_string(),
                "Banana".to_string(),
            ]),
        ));
        sales.add_column(Column::new(
            "quantity".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 10.0, 30.0, 20.0]),
        ));
        model.add_table(sales);

        model.add_scalar(
            "unique_products".to_string(),
            Variable::new(
                "unique_products".to_string(),
                None,
                Some("=COUNTUNIQUE(sales.product)".to_string()),
            ),
        );

        model.add_scalar(
            "unique_quantities".to_string(),
            Variable::new(
                "unique_quantities".to_string(),
                None,
                Some("=COUNTUNIQUE(sales.quantity)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let unique_products = result
            .scalars
            .get("unique_products")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(
            unique_products, 3.0,
            "Should have 3 unique products, got {unique_products}"
        );

        let unique_quantities = result
            .scalars
            .get("unique_quantities")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(
            unique_quantities, 3.0,
            "Should have 3 unique quantities, got {unique_quantities}"
        );
    }

    #[test]
    fn test_unique_function_as_count() {
        let mut model = ParsedModel::new();

        let mut flags = Table::new("flags".to_string());
        flags.add_column(Column::new(
            "active".to_string(),
            ColumnValue::Boolean(vec![true, false, true, true, false]),
        ));
        model.add_table(flags);

        model.add_scalar(
            "unique_flags".to_string(),
            Variable::new(
                "unique_flags".to_string(),
                None,
                Some("=UNIQUE(flags.active)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let unique_flags = result.scalars.get("unique_flags").unwrap().value.unwrap();
        assert_eq!(
            unique_flags, 2.0,
            "Should have 2 unique boolean values (true, false), got {unique_flags}"
        );
    }
}

#[cfg(test)]
mod edge_case_tests {
    #![allow(clippy::approx_constant)]
    #![allow(clippy::float_cmp)]

    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_unique_all_same() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0, 5.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_unique_all_different() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[test]
    fn test_countunique_all_same() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(1.0));
    }

    #[test]
    fn test_countunique_all_diff() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(5.0));
    }

    #[test]
    fn test_countunique_with_dates() {
        let mut model = ParsedModel::new();

        let mut events = Table::new("events".to_string());
        events.add_column(Column::new(
            "date".to_string(),
            ColumnValue::Date(vec![
                "2024-01-15".to_string(),
                "2024-01-16".to_string(),
                "2024-01-15".to_string(),
                "2024-01-17".to_string(),
            ]),
        ));
        model.add_table(events);

        model.add_scalar(
            "unique_dates".to_string(),
            Variable::new(
                "unique_dates".to_string(),
                None,
                Some("=COUNTUNIQUE(events.date)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let unique_dates = result.scalars.get("unique_dates").unwrap().value.unwrap();
        assert_eq!(unique_dates, 3.0);
    }

    #[test]
    fn test_countunique_edge_cases() {
        let mut model = ParsedModel::new();

        let mut single = Table::new("single".to_string());
        single.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![42.0]),
        ));
        model.add_table(single);

        let mut same = Table::new("same".to_string());
        same.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![5.0, 5.0, 5.0, 5.0]),
        ));
        model.add_table(same);

        let mut different = Table::new("different".to_string());
        different.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0, 4.0, 5.0]),
        ));
        model.add_table(different);

        let mut floats = Table::new("floats".to_string());
        floats.add_column(Column::new(
            "value".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 2.0, 2.0]),
        ));
        model.add_table(floats);

        model.add_scalar(
            "single_unique".to_string(),
            Variable::new(
                "single_unique".to_string(),
                None,
                Some("=COUNTUNIQUE(single.value)".to_string()),
            ),
        );

        model.add_scalar(
            "same_unique".to_string(),
            Variable::new(
                "same_unique".to_string(),
                None,
                Some("=COUNTUNIQUE(same.value)".to_string()),
            ),
        );

        model.add_scalar(
            "different_unique".to_string(),
            Variable::new(
                "different_unique".to_string(),
                None,
                Some("=COUNTUNIQUE(different.value)".to_string()),
            ),
        );

        model.add_scalar(
            "floats_unique".to_string(),
            Variable::new(
                "floats_unique".to_string(),
                None,
                Some("=COUNTUNIQUE(floats.value)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        assert_eq!(
            result.scalars.get("single_unique").unwrap().value.unwrap(),
            1.0
        );
        assert_eq!(
            result.scalars.get("same_unique").unwrap().value.unwrap(),
            1.0
        );
        assert_eq!(
            result
                .scalars
                .get("different_unique")
                .unwrap()
                .value
                .unwrap(),
            5.0
        );
        assert_eq!(
            result.scalars.get("floats_unique").unwrap().value.unwrap(),
            2.0
        );
    }

    #[test]
    fn test_countunique_empty_text_values() {
        let mut model = ParsedModel::new();

        let mut mixed = Table::new("mixed".to_string());
        mixed.add_column(Column::new(
            "name".to_string(),
            ColumnValue::Text(vec![
                String::new(),
                "Alice".to_string(),
                String::new(),
                "Bob".to_string(),
                "Alice".to_string(),
            ]),
        ));
        model.add_table(mixed);

        model.add_scalar(
            "unique_names".to_string(),
            Variable::new(
                "unique_names".to_string(),
                None,
                Some("=COUNTUNIQUE(mixed.name)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let unique_names = result.scalars.get("unique_names").unwrap().value.unwrap();
        assert_eq!(unique_names, 3.0);
    }

    #[test]
    fn test_countunique_in_expression() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "C".to_string(),
            ]),
        ));
        model.add_table(data);

        model.add_scalar(
            "unique_times_10".to_string(),
            Variable::new(
                "unique_times_10".to_string(),
                None,
                Some("=COUNTUNIQUE(data.category) * 10".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");

        let result_val = result
            .scalars
            .get("unique_times_10")
            .unwrap()
            .value
            .unwrap();
        assert_eq!(result_val, 30.0);
    }

    #[test]
    fn test_countunique_numbers() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 1.0]),
        ));
        model.add_table(data);

        model.add_scalar(
            "unique".to_string(),
            Variable::new(
                "unique".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let unique = result.scalars.get("unique").unwrap().value.unwrap();
        assert!((unique - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_countunique_numbers_basic() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_unique_function() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "count".to_string(),
            Variable::new(
                "count".to_string(),
                None,
                Some("=COUNT(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let _ = calculator.calculate_all();
    }

    #[test]
    fn test_unique_some_dups() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNT(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }

    #[test]
    fn test_unique_sum() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=SUM(UNIQUE(data.values))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(6.0));
    }

    #[test]
    fn test_countunique_mixed() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 2.0, 3.0, 3.0, 3.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=COUNTUNIQUE(data.values)".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(3.0));
    }
}
