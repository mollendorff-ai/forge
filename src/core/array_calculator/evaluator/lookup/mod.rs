//! Lookup functions: INDEX, MATCH, CHOOSE, XLOOKUP, INDIRECT, VLOOKUP, HLOOKUP, OFFSET, ADDRESS, ROW, COLUMN, ROWS, COLUMNS
//!
//! DEMO functions (3): INDEX, MATCH, CHOOSE
//! ENTERPRISE functions: XLOOKUP, INDIRECT, VLOOKUP, HLOOKUP, OFFSET, ADDRESS, ROW, COLUMN, ROWS, COLUMNS

mod choose;
mod index;
mod match_fn;
mod reference;
mod xlookup;

use super::{EvalContext, EvalError, Expr, Value};

/// Try to evaluate a lookup function. Returns None if function not recognized.
pub fn try_evaluate(
    name: &str,
    args: &[Expr],
    ctx: &EvalContext,
) -> Result<Option<Value>, EvalError> {
    // ═══════════════════════════════════════════════════════════════════════════
    // DEMO FUNCTIONS (always available)
    // ═══════════════════════════════════════════════════════════════════════════
    let result = match name {
        "INDEX" => index::eval_index(args, ctx)?,
        "MATCH" => match_fn::eval_match(args, ctx)?,
        "CHOOSE" => choose::eval_choose(args, ctx)?,

        // ═══════════════════════════════════════════════════════════════════════════
        // ENTERPRISE FUNCTIONS (only in full build)
        // ═══════════════════════════════════════════════════════════════════════════
        "INDIRECT" => reference::eval_indirect(args, ctx)?,

        "XLOOKUP" => xlookup::eval_xlookup(args, ctx)?,

        "VLOOKUP" => xlookup::eval_vlookup(args, ctx)?,

        "HLOOKUP" => xlookup::eval_hlookup(args, ctx)?,

        "OFFSET" => reference::eval_offset(args, ctx)?,

        "ADDRESS" => reference::eval_address(args, ctx)?,

        "ROW" => reference::eval_row(args, ctx)?,

        "COLUMN" => reference::eval_column(args, ctx)?,

        "ROWS" => reference::eval_rows(args, ctx)?,

        "COLUMNS" => reference::eval_columns(args, ctx)?,

        _ => return Ok(None),
    };

    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    // Tests that involve multiple functions together

    #[test]
    fn test_index_match_combined() {
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
        products.add_column(Column::new(
            "price".to_string(),
            ColumnValue::Number(vec![10.0, 20.0, 30.0]),
        ));
        model.add_table(products);

        let mut sales = Table::new("sales".to_string());
        sales.add_column(Column::new(
            "product_id".to_string(),
            ColumnValue::Number(vec![102.0, 101.0, 103.0]),
        ));
        sales.add_row_formula(
            "product_name".to_string(),
            "=INDEX(products.product_name, MATCH(product_id, products.product_id, 0))".to_string(),
        );
        sales.add_row_formula(
            "price".to_string(),
            "=INDEX(products.price, MATCH(product_id, products.product_id, 0))".to_string(),
        );
        model.add_table(sales);

        let calculator = ArrayCalculator::new(model);
        let result = calculator
            .calculate_all()
            .expect("Calculation should succeed");
        let result_table = result.tables.get("sales").unwrap();

        let product_name = result_table.columns.get("product_name").unwrap();
        match &product_name.values {
            ColumnValue::Text(texts) => {
                assert_eq!(texts[0], "Widget B");
                assert_eq!(texts[1], "Widget A");
                assert_eq!(texts[2], "Widget C");
            },
            _ => panic!("Expected Text array"),
        }

        let price = result_table.columns.get("price").unwrap();
        match &price.values {
            ColumnValue::Number(nums) => {
                assert_eq!(nums[0], 20.0);
                assert_eq!(nums[1], 10.0);
                assert_eq!(nums[2], 30.0);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_cross_table_row_count_mismatch_error() {
        let mut model = ParsedModel::new();

        let mut table1 = Table::new("table1".to_string());
        table1.add_column(Column::new(
            "a".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        model.add_table(table1);

        let mut table2 = Table::new("table2".to_string());
        table2.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        table2
            .row_formulas
            .insert("result".to_string(), "=table1.a + x".to_string());
        model.add_table(table2);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("rows"));
    }

    #[test]
    fn test_column_row_count_mismatch_local() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "a".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        data.columns.insert(
            "b".to_string(),
            Column::new("b".to_string(), ColumnValue::Number(vec![10.0, 20.0])),
        );
        data.row_formulas
            .insert("result".to_string(), "=a + b".to_string());
        model.add_table(data);

        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(result.is_err());
    }

    #[test]
    fn test_index_match_combination() {
        let mut model = ParsedModel::new();
        let mut data = Table::new("data".to_string());
        data.add_column(Column::new(
            "names".to_string(),
            ColumnValue::Text(vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Carol".to_string(),
            ]),
        ));
        data.add_column(Column::new(
            "scores".to_string(),
            ColumnValue::Number(vec![85.0, 92.0, 78.0]),
        ));
        model.add_table(data);
        model.add_scalar(
            "score".to_string(),
            Variable::new(
                "score".to_string(),
                None,
                Some("=INDEX(data.scores, MATCH(\"Bob\", data.names, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all();
        assert!(
            result.is_ok(),
            "INDEX/MATCH combination should calculate successfully"
        );
        let model_result = result.unwrap();
        let val = model_result.scalars.get("score").unwrap().value.unwrap();
        assert_eq!(val, 92.0);
    }

    #[test]
    fn test_index_match_basic_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, MATCH(2, data.keys, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(200.0));
    }

    #[test]
    fn test_index_match_first_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, MATCH(1, data.keys, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(100.0));
    }

    #[test]
    fn test_index_match_last_edge() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("data".to_string());
        table.add_column(Column::new(
            "keys".to_string(),
            ColumnValue::Number(vec![1.0, 2.0, 3.0]),
        ));
        table.add_column(Column::new(
            "values".to_string(),
            ColumnValue::Number(vec![100.0, 200.0, 300.0]),
        ));
        model.add_table(table);
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=INDEX(data.values, MATCH(3, data.keys, 0))".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(300.0));
    }
}
