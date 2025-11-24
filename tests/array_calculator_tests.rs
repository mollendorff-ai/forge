use royalbit_forge::core::ArrayCalculator;
use royalbit_forge::parser::parse_model;
use royalbit_forge::types::{Column, ColumnValue, ForgeVersion, ParsedModel, Table};
use std::path::Path;

#[test]
fn test_simple_table_calculation() {
    let mut model = ParsedModel::new(ForgeVersion::V1_0_0);

    let mut table = Table::new("financials".to_string());
    table.add_column(Column::new(
        "revenue".to_string(),
        ColumnValue::Number(vec![1000.0, 1200.0, 1500.0, 1800.0]),
    ));
    table.add_column(Column::new(
        "cogs".to_string(),
        ColumnValue::Number(vec![300.0, 360.0, 450.0, 540.0]),
    ));
    table.add_row_formula("gross_profit".to_string(), "=revenue - cogs".to_string());
    table.add_row_formula(
        "gross_margin".to_string(),
        "=gross_profit / revenue".to_string(),
    );

    model.add_table(table);

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all().expect("Calculation should succeed");

    let result_table = result.tables.get("financials").unwrap();

    // Check gross_profit
    let gross_profit = result_table.columns.get("gross_profit").unwrap();
    match &gross_profit.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums.len(), 4);
            assert_eq!(nums[0], 700.0);
            assert_eq!(nums[1], 840.0);
            assert_eq!(nums[2], 1050.0);
            assert_eq!(nums[3], 1260.0);
        }
        _ => panic!("Expected Number array"),
    }

    // Check gross_margin
    let gross_margin = result_table.columns.get("gross_margin").unwrap();
    match &gross_margin.values {
        ColumnValue::Number(nums) => {
            assert_eq!(nums.len(), 4);
            assert!((nums[0] - 0.7).abs() < 0.0001);
            assert!((nums[1] - 0.7).abs() < 0.0001);
            assert!((nums[2] - 0.7).abs() < 0.0001);
            assert!((nums[3] - 0.7).abs() < 0.0001);
        }
        _ => panic!("Expected Number array"),
    }

    println!("✓ Simple table calculation succeeded");
}

#[test]
#[ignore] // TODO: Enable once cross-table references are implemented
fn test_calculate_quarterly_pl() {
    let path = Path::new("test-data/v1.0/quarterly_pl.yaml");
    let model = parse_model(path).expect("Failed to parse");

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all();

    match result {
        Ok(calculated_model) => {
            println!("✓ Calculation succeeded");
            for (name, table) in &calculated_model.tables {
                println!("  Table '{}': {} columns", name, table.columns.len());
                for (col_name, col) in &table.columns {
                    println!("    - {}: {} rows", col_name, col.values.len());
                }
            }
        }
        Err(e) => {
            println!("✗ Calculation failed: {}", e);
            panic!("Calculation failed: {}", e);
        }
    }
}
