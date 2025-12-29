//! MINIFS and MAXIFS function implementations
//!
//! MINIFS returns the minimum value among cells that meet multiple criteria.
//! MAXIFS returns the maximum value among cells that meet multiple criteria.
//!
//! Syntax:
//! - MINIFS(min_range, criteria_range1, criteria1, [criteria_range2, criteria2], ...)
//! - MAXIFS(max_range, criteria_range1, criteria1, [criteria_range2, criteria2], ...)

use super::super::{
    collect_values_as_vec, evaluate, matches_criteria, EvalContext, EvalError, Expr, Value,
};

/// Evaluate MAXIFS function
pub fn eval_maxifs(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    if args.len() < 3 || args.len().is_multiple_of(2) {
        return Err(EvalError::new(
            "MAXIFS requires max_range, criteria_range1, criteria1, ...",
        ));
    }
    let max_range = collect_values_as_vec(&args[0], ctx)?;
    let mut matches = vec![true; max_range.len()];
    for pair in args[1..].chunks(2) {
        let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
        let criteria = evaluate(&pair[1], ctx)?;
        for (i, val) in criteria_range.iter().enumerate() {
            if i < matches.len() && !matches_criteria(val, &criteria) {
                matches[i] = false;
            }
        }
    }
    let matching: Vec<f64> = max_range
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < matches.len() && matches[*i])
        .filter_map(|(_, v)| v.as_number())
        .collect();
    if matching.is_empty() {
        Ok(Value::Number(0.0))
    } else {
        Ok(Value::Number(
            matching.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ))
    }
}

/// Evaluate MINIFS function
pub fn eval_minifs(args: &[Expr], ctx: &EvalContext) -> Result<Value, EvalError> {
    if args.len() < 3 || args.len().is_multiple_of(2) {
        return Err(EvalError::new(
            "MINIFS requires min_range, criteria_range1, criteria1, ...",
        ));
    }
    let min_range = collect_values_as_vec(&args[0], ctx)?;
    let mut matches = vec![true; min_range.len()];
    for pair in args[1..].chunks(2) {
        let criteria_range = collect_values_as_vec(&pair[0], ctx)?;
        let criteria = evaluate(&pair[1], ctx)?;
        for (i, val) in criteria_range.iter().enumerate() {
            if i < matches.len() && !matches_criteria(val, &criteria) {
                matches[i] = false;
            }
        }
    }
    let matching: Vec<f64> = min_range
        .iter()
        .enumerate()
        .filter(|(i, _)| *i < matches.len() && matches[*i])
        .filter_map(|(_, v)| v.as_number())
        .collect();
    if matching.is_empty() {
        Ok(Value::Number(0.0))
    } else {
        Ok(Value::Number(
            matching.iter().copied().fold(f64::INFINITY, f64::min),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_maxifs_multiple_criteria() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec![
                "North".to_string(),
                "South".to_string(),
                "North".to_string(),
                "North".to_string(),
            ]),
        ));
        table.add_column(Column::new(
            "quarter".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 2.0, 2.0]),
        ));
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![1000.0, 2000.0, 1500.0, 1800.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "max_result".to_string(),
            Variable::new(
                "max_result".to_string(),
                None,
                Some(
                    "=MAXIFS(sales.revenue, sales.region, \"North\", sales.quarter, \"2\")"
                        .to_string(),
                ),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();
        assert_eq!(
            result.scalars.get("max_result").unwrap().value,
            Some(1800.0)
        );
    }

    #[test]
    fn test_maxifs_scalar() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("products".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "A".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![10.0, 50.0, 30.0, 20.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "max_a_price".to_string(),
            Variable::new(
                "max_a_price".to_string(),
                None,
                Some("=MAXIFS(products.price, products.category, \"A\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();
        assert_eq!(result.scalars.get("max_a_price").unwrap().value, Some(30.0));
    }

    #[test]
    fn test_minifs_multiple_criteria() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("inventory".to_string());
        table.add_column(Column::new(
            "product".to_string(),
            ColumnValue::Text(vec![
                "Widget".to_string(),
                "Gadget".to_string(),
                "Widget".to_string(),
                "Widget".to_string(),
            ]),
        ));
        table.add_column(Column::new(
            "quantity".to_string(),
            ColumnValue::Number(vec![100.0, 50.0, 75.0, 120.0]),
        ));
        table.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![10.0, 15.0, 9.0, 11.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "min_result".to_string(),
            Variable::new(
                "min_result".to_string(),
                None,
                Some(
                    "=MINIFS(inventory.price, inventory.product, \"Widget\", inventory.quantity, \">=75\")"
                        .to_string(),
                ),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();
        assert_eq!(result.scalars.get("min_result").unwrap().value, Some(9.0));
    }

    #[test]
    fn test_minifs_criteria() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("products".to_string());
        data.add_column(Column::new(
            "category".to_string(),
            ColumnValue::Text(vec![
                "A".to_string(),
                "B".to_string(),
                "A".to_string(),
                "A".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![10.0, 50.0, 30.0, 20.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "min_a_price".to_string(),
            Variable::new(
                "min_a_price".to_string(),
                None,
                Some("=MINIFS(products.price, products.category, \"A\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().unwrap();
        assert_eq!(result.scalars.get("min_a_price").unwrap().value, Some(10.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_maxifs_basic() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MAXIFS(data.values, data.cat, \"=1\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(30.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_minifs_basic() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0, 40.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 1.0, 2.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MINIFS(data.values, data.cat, \"=1\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(10.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_maxifs_empty_result() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MAXIFS(data.values, data.cat, \"=5\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[cfg(not(feature = "demo"))]
    #[test]
    fn test_minifs_empty_result() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        table.add_column(Column::new(
            "cat".to_string(),
            ColumnValue::Number(vec![1.0, 1.0, 1.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=MINIFS(data.values, data.cat, \"=5\")".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }
}
