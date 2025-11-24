use royalbit_forge::parser::parse_model;
use std::path::Path;

#[test]
fn test_parse_saas_unit_economics() {
    let path = Path::new("test-data/v1.0/saas_unit_economics.yaml");
    let result = parse_model(path);

    match &result {
        Ok(model) => {
            assert_eq!(model.version, royalbit_forge::types::ForgeVersion::V1_0_0);
            println!("Tables: {}", model.tables.len());
            for (name, table) in &model.tables {
                println!(
                    "  - {}: {} columns, {} rows",
                    name,
                    table.columns.len(),
                    table.row_count()
                );
            }
            println!("Scalars: {}", model.scalars.len());
        }
        Err(e) => {
            panic!("Failed to parse saas_unit_economics.yaml: {}", e);
        }
    }

    assert!(result.is_ok());
}

#[test]
fn test_parse_quarterly_pl() {
    let path = Path::new("test-data/v1.0/quarterly_pl.yaml");
    let result = parse_model(path);

    match &result {
        Ok(model) => {
            assert_eq!(model.version, royalbit_forge::types::ForgeVersion::V1_0_0);
            println!("Tables: {}", model.tables.len());
            for (name, table) in &model.tables {
                println!(
                    "  - {}: {} columns, {} rows",
                    name,
                    table.columns.len(),
                    table.row_count()
                );
            }
            println!("Scalars: {}", model.scalars.len());
        }
        Err(e) => {
            panic!("Failed to parse quarterly_pl.yaml: {}", e);
        }
    }

    assert!(result.is_ok());
}

#[test]
fn test_parse_budget_vs_actual() {
    let path = Path::new("test-data/v1.0/budget_vs_actual.yaml");
    let result = parse_model(path);

    match &result {
        Ok(model) => {
            assert_eq!(model.version, royalbit_forge::types::ForgeVersion::V1_0_0);
            println!("Tables: {}", model.tables.len());
            for (name, table) in &model.tables {
                println!(
                    "  - {}: {} columns, {} rows",
                    name,
                    table.columns.len(),
                    table.row_count()
                );
            }
            println!("Scalars: {}", model.scalars.len());
        }
        Err(e) => {
            panic!("Failed to parse budget_vs_actual.yaml: {}", e);
        }
    }

    assert!(result.is_ok());
}
