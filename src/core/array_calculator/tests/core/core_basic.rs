//! Core ArrayCalculator functionality tests
//!
//! Tests for basic operations, rowwise formulas, aggregations, scalar dependencies, array indexing

#![allow(clippy::approx_constant)]

use crate::core::array_calculator::ArrayCalculator;
#[allow(unused_imports)]
use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

#[test]
fn test_simple_rowwise_formula() {
    let mut model = ParsedModel::new();

    let mut table = Table::new("test".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![100.0, 200.0, 300.0]),
    ));
    table.add_column(Column::new(
        "expenses".to_string(),
        ColumnValue::Number(vec![60.0, 120.0, 180.0]),
    ));
    table.add_row_formula("profit".to_string(), "=revenue - expenses".to_string());

    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();

    let result_table = result.tables.get("test").unwrap();
    let profit_col = result_table.columns.get("profit").unwrap();

    match &profit_col.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums.len(), 3);
            assert_eq!(nums[0], 40.0);
            assert_eq!(nums[1], 80.0);
            assert_eq!(nums[2], 120.0);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_is_aggregation_formula() {
    let model = ParsedModel::new();
    let calc = ArrayCalculator::new(model);

    assert!(calc.is_aggregation_formula("=SUM(revenue)"));
    assert!(calc.is_aggregation_formula("=AVERAGE(profit)"));
    assert!(calc.is_aggregation_formula("=sum(revenue)")); // case insensitive
    assert!(!calc.is_aggregation_formula("=revenue - expenses"));
    assert!(!calc.is_aggregation_formula("=revenue * 0.3"));
}

#[test]
fn test_extract_column_references() {
    let model = ParsedModel::new();
    let calc = ArrayCalculator::new(model);

    let refs = calc
        .extract_column_references("=revenue - expenses")
        .unwrap();
    assert_eq!(refs.len(), 2);
    assert!(refs.contains(&"revenue".to_string()));
    assert!(refs.contains(&"expenses".to_string()));

    let refs2 = calc
        .extract_column_references("=revenue * 0.3 + fixed_cost")
        .unwrap();
    assert!(refs2.contains(&"revenue".to_string()));
    assert!(refs2.contains(&"fixed_cost".to_string()));
}

#[test]
fn test_array_indexing() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("quarterly".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![1000.0, 1200.0, 1500.0, 1800.0]),
    ));
    model.add_table(table);

    let q1_revenue = Variable::new(
        "q1_revenue".to_string(),
        None,
        Some("=quarterly.revenue[0]".to_string()),
    );
    model.add_scalar("q1_revenue".to_string(), q1_revenue);

    let q4_revenue = Variable::new(
        "q4_revenue".to_string(),
        None,
        Some("=quarterly.revenue[3]".to_string()),
    );
    model.add_scalar("q4_revenue".to_string(), q4_revenue);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();

    assert_eq!(
        result.scalars.get("q1_revenue").unwrap().value,
        Some(1000.0)
    );
    assert_eq!(
        result.scalars.get("q4_revenue").unwrap().value,
        Some(1800.0)
    );
}

#[test]
fn test_scalar_dependencies() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("pl".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![1000.0, 1200.0]),
    ));
    table.add_column(Column::new(
        "cogs".to_string(),
        ColumnValue::Number(vec![300.0, 360.0]),
    ));
    model.add_table(table);

    // total_revenue depends on table
    let total_revenue = Variable::new(
        "total_revenue".to_string(),
        None,
        Some("=SUM(pl.revenue)".to_string()),
    );
    model.add_scalar("total_revenue".to_string(), total_revenue);

    // total_cogs depends on table
    let total_cogs = Variable::new(
        "total_cogs".to_string(),
        None,
        Some("=SUM(pl.cogs)".to_string()),
    );
    model.add_scalar("total_cogs".to_string(), total_cogs);

    // gross_profit depends on total_revenue and total_cogs
    let gross_profit = Variable::new(
        "gross_profit".to_string(),
        None,
        Some("=total_revenue - total_cogs".to_string()),
    );
    model.add_scalar("gross_profit".to_string(), gross_profit);

    // gross_margin depends on gross_profit and total_revenue
    let gross_margin = Variable::new(
        "gross_margin".to_string(),
        None,
        Some("=gross_profit / total_revenue".to_string()),
    );
    model.add_scalar("gross_margin".to_string(), gross_margin);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().unwrap();

    assert_eq!(
        result.scalars.get("total_revenue").unwrap().value,
        Some(2200.0)
    );
    assert_eq!(result.scalars.get("total_cogs").unwrap().value, Some(660.0));
    assert_eq!(
        result.scalars.get("gross_profit").unwrap().value,
        Some(1540.0)
    );
    assert!((result.scalars.get("gross_margin").unwrap().value.unwrap() - 0.7).abs() < 0.0001);
}

#[test]
fn test_math_functions_combined() {
    let mut model = ParsedModel::new();
    let mut table = Table::new("data".to_string());

    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.567, 20.234, 30.899]),
    ));
    table.add_row_formula("complex".to_string(), "=ROUND(SQRT(values), 2)".to_string());

    model.add_table(table);
    let calculator = ArrayCalculator::new(model);
    let result = calculator
        .calculate_all()
        .expect("Calculation should succeed");
    let result_table = result.tables.get("data").unwrap();

    let complex = result_table.columns.get("complex").unwrap();
    match &complex.values {
        ColumnValue::Number(nums) => {
            assert!((nums[0] - 3.25).abs() < 0.01);
            assert!((nums[1] - 4.50).abs() < 0.01);
            assert!((nums[2] - 5.56).abs() < 0.01);
        }
        _ => panic!("Expected Number array"),
    }
}

#[test]
fn test_cross_table_sum() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Create two tables
    let mut revenue_table = Table::new("revenue".to_string());
    revenue_table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![1000.0, 2000.0, 3000.0]),
    ));
    model.add_table(revenue_table);

    let mut costs_table = Table::new("costs".to_string());
    costs_table.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![500.0, 750.0, 1000.0]),
    ));
    model.add_table(costs_table);

    // Single table aggregation
    model.add_scalar(
        "total_revenue".to_string(),
        Variable::new(
            "total_revenue".to_string(),
            None,
            Some("=SUM(revenue.amount)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // 1000+2000+3000 = 6000
    let total = result.scalars.get("total_revenue").unwrap().value.unwrap();
    assert_eq!(total, 6000.0);
}

#[test]
fn test_circular_dependency_error() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Create circular dependency: a depends on b, b depends on a
    model.add_scalar(
        "a".to_string(),
        Variable::new("a".to_string(), None, Some("=b + 1".to_string())),
    );
    model.add_scalar(
        "b".to_string(),
        Variable::new("b".to_string(), None, Some("=a + 1".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Circular") || err.contains("Unable to resolve"));
}

#[test]
fn test_undefined_reference_error() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "result".to_string(),
        Variable::new(
            "result".to_string(),
            None,
            Some("=nonexistent_variable * 2".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    assert!(result.is_err());
}

#[test]
fn test_scalar_chain_calculation() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "base".to_string(),
        Variable::new("base".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "doubled".to_string(),
        Variable::new("doubled".to_string(), None, Some("=base * 2".to_string())),
    );
    model.add_scalar(
        "final".to_string(),
        Variable::new("final".to_string(), None, Some("=doubled + 50".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert_eq!(result.scalars.get("doubled").unwrap().value.unwrap(), 200.0);
    assert_eq!(result.scalars.get("final").unwrap().value.unwrap(), 250.0);
}

#[test]
fn test_empty_table_handling() {
    let mut model = ParsedModel::new();

    let empty_table = Table::new("empty".to_string());
    model.add_table(empty_table);

    let calculator = ArrayCalculator::new(model);
    // Should not panic with empty table
    let result = calculator.calculate_all();
    assert!(result.is_ok());
}

#[test]
fn test_profit_calculation() {
    let mut model = ParsedModel::new();

    let mut income = Table::new("income".to_string());
    income.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![1000.0, 2000.0, 1500.0]),
    ));
    income.add_column(Column::new(
        "cost".to_string(),
        ColumnValue::Number(vec![600.0, 1400.0, 900.0]),
    ));
    income
        .row_formulas
        .insert("profit".to_string(), "=revenue - cost".to_string());
    model.add_table(income);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let profit = result
        .tables
        .get("income")
        .unwrap()
        .columns
        .get("profit")
        .unwrap();
    if let ColumnValue::Number(values) = &profit.values {
        assert_eq!(values[0], 400.0);
        assert_eq!(values[1], 600.0);
        assert_eq!(values[2], 600.0);
    }
}

#[test]
fn test_multi_column_operations() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "a".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    data.add_column(Column::new(
        "b".to_string(),
        ColumnValue::Number(vec![5.0, 10.0]),
    ));
    data.row_formulas
        .insert("sum".to_string(), "=a + b".to_string());
    data.row_formulas
        .insert("diff".to_string(), "=a - b".to_string());
    data.row_formulas
        .insert("prod".to_string(), "=a * b".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let table = result.tables.get("data").unwrap();

    if let ColumnValue::Number(values) = &table.columns.get("sum").unwrap().values {
        assert_eq!(values[0], 15.0);
        assert_eq!(values[1], 30.0);
    }

    if let ColumnValue::Number(values) = &table.columns.get("diff").unwrap().values {
        assert_eq!(values[0], 5.0);
        assert_eq!(values[1], 10.0);
    }

    if let ColumnValue::Number(values) = &table.columns.get("prod").unwrap().values {
        assert_eq!(values[0], 50.0);
        assert_eq!(values[1], 200.0);
    }
}

#[test]
fn test_multiple_tables_same_columns() {
    let mut model = ParsedModel::new();

    let mut table1 = Table::new("jan".to_string());
    table1.add_column(Column::new(
        "sales".to_string(),
        ColumnValue::Number(vec![100.0, 200.0]),
    ));
    model.add_table(table1);

    let mut table2 = Table::new("feb".to_string());
    table2.add_column(Column::new(
        "sales".to_string(),
        ColumnValue::Number(vec![150.0, 250.0]),
    ));
    model.add_table(table2);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Both tables should exist independently
    assert!(result.tables.contains_key("jan"));
    assert!(result.tables.contains_key("feb"));
}

#[test]
fn test_division_operation() {
    let mut model = ParsedModel::new();

    let mut data = Table::new("data".to_string());
    data.add_column(Column::new(
        "numerator".to_string(),
        ColumnValue::Number(vec![100.0, 50.0, 75.0]),
    ));
    data.add_column(Column::new(
        "denominator".to_string(),
        ColumnValue::Number(vec![2.0, 5.0, 3.0]),
    ));
    data.row_formulas
        .insert("result".to_string(), "=numerator / denominator".to_string());
    model.add_table(data);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("data")
        .unwrap()
        .columns
        .get("result")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values[0], 50.0);
        assert_eq!(values[1], 10.0);
        assert_eq!(values[2], 25.0);
    }
}

#[test]
fn test_get_table_calculation_order_simple() {
    let mut model = ParsedModel::new();

    let mut table1 = Table::new("revenue".to_string());
    table1.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![100.0, 200.0]),
    ));
    model.add_table(table1);

    let mut table2 = Table::new("expenses".to_string());
    table2.add_column(Column::new(
        "amount".to_string(),
        ColumnValue::Number(vec![50.0, 100.0]),
    ));
    model.add_table(table2);

    let calc = ArrayCalculator::new(model);
    let table_names: Vec<String> = vec!["revenue".to_string(), "expenses".to_string()];
    let order = calc.get_table_calculation_order(&table_names).unwrap();

    // Both tables should be in the order (no dependencies)
    assert_eq!(order.len(), 2);
    assert!(order.contains(&"revenue".to_string()));
    assert!(order.contains(&"expenses".to_string()));
}

#[test]
fn test_extract_table_dependencies_from_formula() {
    let mut model = ParsedModel::new();

    let mut source = Table::new("source".to_string());
    source.add_column(Column::new(
        "value".to_string(),
        ColumnValue::Number(vec![10.0]),
    ));
    model.add_table(source);

    let calc = ArrayCalculator::new(model);

    // Formula referencing source.value should extract "source" as dependency
    let deps = calc
        .extract_table_dependencies_from_formula("=source.value * 2")
        .unwrap();
    assert!(deps.contains(&"source".to_string()));

    // Formula with no table references
    let deps2 = calc
        .extract_table_dependencies_from_formula("=10 * 2")
        .unwrap();
    assert!(deps2.is_empty());
}

#[test]
fn test_is_aggregation_formula_all_functions() {
    let model = ParsedModel::new();
    let calc = ArrayCalculator::new(model);

    // Standard aggregations that ARE supported
    assert!(calc.is_aggregation_formula("=SUM(data.values)"));
    assert!(calc.is_aggregation_formula("=AVERAGE(data.values)"));
    assert!(calc.is_aggregation_formula("=AVG(data.values)"));
    assert!(calc.is_aggregation_formula("=COUNT(data.values)"));
    assert!(calc.is_aggregation_formula("=MIN(data.values)"));
    assert!(calc.is_aggregation_formula("=MAX(data.values)"));
    assert!(calc.is_aggregation_formula("=MEDIAN(data.values)"));
    assert!(calc.is_aggregation_formula("=STDEV(data.values)"));
    assert!(calc.is_aggregation_formula("=STDEV.S(data.values)"));
    assert!(calc.is_aggregation_formula("=STDEV.P(data.values)"));
    assert!(calc.is_aggregation_formula("=VAR(data.values)"));
    assert!(calc.is_aggregation_formula("=VAR.S(data.values)"));
    assert!(calc.is_aggregation_formula("=VAR.P(data.values)"));
    assert!(calc.is_aggregation_formula("=PERCENTILE(data.values, 0.5)"));
    assert!(calc.is_aggregation_formula("=QUARTILE(data.values, 2)"));
    assert!(calc.is_aggregation_formula("=CORREL(data.x, data.y)"));

    // Conditional aggregations
    assert!(calc.is_aggregation_formula("=SUMIF(data.cat, \"A\", data.val)"));
    assert!(calc.is_aggregation_formula("=COUNTIF(data.values, \">0\")"));
    assert!(calc.is_aggregation_formula("=AVERAGEIF(data.cat, \"A\", data.val)"));
    assert!(calc.is_aggregation_formula("=SUMIFS(data.val, data.cat, \"A\")"));
    assert!(calc.is_aggregation_formula("=COUNTIFS(data.cat, \"A\", data.val, \">0\")"));
    assert!(calc.is_aggregation_formula("=AVERAGEIFS(data.val, data.cat, \"A\")"));
    assert!(calc.is_aggregation_formula("=MAXIFS(data.val, data.cat, \"A\")"));
    assert!(calc.is_aggregation_formula("=MINIFS(data.val, data.cat, \"A\")"));

    // Not aggregations
    assert!(!calc.is_aggregation_formula("=revenue - expenses"));
    assert!(!calc.is_aggregation_formula("=price * quantity"));
    assert!(!calc.is_aggregation_formula("=PRODUCT(data.values)")); // Not supported
}

#[test]
fn test_invalid_column_reference() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();
    model.add_scalar(
        "bad_ref".to_string(),
        Variable::new(
            "bad_ref".to_string(),
            None,
            Some("=SUM(nonexistent_table.column)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();
    assert!(result.is_err());
}

#[cfg(not(feature = "demo"))]
#[test]
fn test_single_value_aggregations() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("single".to_string());
    table.add_column(Column::new(
        "val".to_string(),
        ColumnValue::Number(vec![42.0]),
    ));
    model.add_table(table);

    model.add_scalar(
        "sum".to_string(),
        Variable::new(
            "sum".to_string(),
            None,
            Some("=SUM(single.val)".to_string()),
        ),
    );
    model.add_scalar(
        "avg".to_string(),
        Variable::new(
            "avg".to_string(),
            None,
            Some("=AVERAGE(single.val)".to_string()),
        ),
    );
    model.add_scalar(
        "med".to_string(),
        Variable::new(
            "med".to_string(),
            None,
            Some("=MEDIAN(single.val)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("sum").unwrap().value.unwrap() - 42.0).abs() < 0.01);
    assert!((result.scalars.get("avg").unwrap().value.unwrap() - 42.0).abs() < 0.01);
    assert!((result.scalars.get("med").unwrap().value.unwrap() - 42.0).abs() < 0.01);
}

#[test]
fn test_nested_formula_evaluation() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    let mut table = Table::new("data".to_string());
    table.add_column(Column::new(
        "values".to_string(),
        ColumnValue::Number(vec![10.0, 20.0, 30.0]),
    ));
    model.add_table(table);

    // Nested: ROUND(SUM(...), 0)
    model.add_scalar(
        "rounded_sum".to_string(),
        Variable::new(
            "rounded_sum".to_string(),
            None,
            Some("=ROUND(SUM(data.values), 0)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let val = result.scalars.get("rounded_sum").unwrap().value.unwrap();
    assert!((val - 60.0).abs() < 0.01);
}

#[test]
fn test_scalar_arithmetic() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "a".to_string(),
        Variable::new("a".to_string(), Some(10.0), None),
    );
    model.add_scalar(
        "b".to_string(),
        Variable::new("b".to_string(), Some(3.0), None),
    );
    model.add_scalar(
        "sum".to_string(),
        Variable::new("sum".to_string(), None, Some("=a + b".to_string())),
    );
    model.add_scalar(
        "diff".to_string(),
        Variable::new("diff".to_string(), None, Some("=a - b".to_string())),
    );
    model.add_scalar(
        "prod".to_string(),
        Variable::new("prod".to_string(), None, Some("=a * b".to_string())),
    );
    model.add_scalar(
        "quot".to_string(),
        Variable::new("quot".to_string(), None, Some("=a / b".to_string())),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("sum").unwrap().value.unwrap() - 13.0).abs() < 0.01);
    assert!((result.scalars.get("diff").unwrap().value.unwrap() - 7.0).abs() < 0.01);
    assert!((result.scalars.get("prod").unwrap().value.unwrap() - 30.0).abs() < 0.01);
    assert!((result.scalars.get("quot").unwrap().value.unwrap() - 3.333).abs() < 0.01);
}

#[test]
fn test_cross_table_formula() {
    let mut model = ParsedModel::new();

    let mut prices = Table::new("prices".to_string());
    prices.add_column(Column::new(
        "unit_price".to_string(),
        ColumnValue::Number(vec![10.0, 20.0]),
    ));
    model.add_table(prices);

    let mut orders = Table::new("orders".to_string());
    orders.add_column(Column::new(
        "quantity".to_string(),
        ColumnValue::Number(vec![5.0, 3.0]),
    ));
    orders.row_formulas.insert(
        "total".to_string(),
        "=quantity * prices.unit_price".to_string(),
    );
    model.add_table(orders);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    let col = result
        .tables
        .get("orders")
        .unwrap()
        .columns
        .get("total")
        .unwrap();
    if let ColumnValue::Number(values) = &col.values {
        assert_eq!(values[0], 50.0); // 5 * 10
        assert_eq!(values[1], 60.0); // 3 * 20
    }
}

#[test]
fn test_formula_chain() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    model.add_scalar(
        "base".to_string(),
        Variable::new("base".to_string(), Some(100.0), None),
    );
    model.add_scalar(
        "tax_rate".to_string(),
        Variable::new("tax_rate".to_string(), Some(0.2), None),
    );
    model.add_scalar(
        "discount".to_string(),
        Variable::new("discount".to_string(), Some(10.0), None),
    );
    model.add_scalar(
        "subtotal".to_string(),
        Variable::new(
            "subtotal".to_string(),
            None,
            Some("=base - discount".to_string()),
        ),
    );
    model.add_scalar(
        "tax".to_string(),
        Variable::new(
            "tax".to_string(),
            None,
            Some("=subtotal * tax_rate".to_string()),
        ),
    );
    model.add_scalar(
        "total".to_string(),
        Variable::new(
            "total".to_string(),
            None,
            Some("=subtotal + tax".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    assert!((result.scalars.get("subtotal").unwrap().value.unwrap() - 90.0).abs() < 0.01);
    assert!((result.scalars.get("tax").unwrap().value.unwrap() - 18.0).abs() < 0.01);
    assert!((result.scalars.get("total").unwrap().value.unwrap() - 108.0).abs() < 0.01);
}

#[test]
fn test_large_dataset() {
    use crate::types::Variable;

    let mut model = ParsedModel::new();

    // Create a table with 100 rows
    let values: Vec<f64> = (1..=100).map(|i| i as f64).collect();
    let mut data = Table::new("big".to_string());
    data.add_column(Column::new("nums".to_string(), ColumnValue::Number(values)));
    data.row_formulas
        .insert("doubled".to_string(), "=nums * 2".to_string());
    model.add_table(data);

    model.add_scalar(
        "sum_all".to_string(),
        Variable::new(
            "sum_all".to_string(),
            None,
            Some("=SUM(big.doubled)".to_string()),
        ),
    );

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Should calculate");

    // Sum of 2 * (1..100) = 2 * 5050 = 10100
    let total = result.scalars.get("sum_all").unwrap().value.unwrap();
    assert!((total - 10100.0).abs() < 0.01);
}
