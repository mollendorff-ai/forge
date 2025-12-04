use royalbit_forge::parser::parse_model;
use std::path::Path;

// ========== Multi-Document YAML Tests (v4.4.2) ==========

#[test]
fn test_parse_multi_document_yaml() {
    let path = Path::new("test-data/test_multi_document.yaml");
    let result = parse_model(path);

    match &result {
        Ok(model) => {
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
            println!("Documents: {}", model.documents.len());

            // Multi-doc should merge all documents into one model
            // Tables should be prefixed with document name
            assert!(
                model.tables.contains_key("Sales.sales")
                    || model.tables.contains_key("sales")
                    || model.tables.len() >= 2,
                "Should parse multiple documents into tables"
            );

            // Documents list should contain document names
            assert!(
                !model.documents.is_empty(),
                "Should track document names, found: {:?}",
                model.documents
            );
        }
        Err(e) => {
            panic!("Failed to parse multi-document YAML: {}", e);
        }
    }

    assert!(result.is_ok());
}

#[test]
fn test_parse_model_with_includes() {
    let path = Path::new("test-data/v4_with_includes.yaml");
    let result = parse_model(path);

    match &result {
        Ok(model) => {
            println!("Tables: {}", model.tables.len());
            println!("Scalars: {}", model.scalars.len());
            println!("Resolved includes: {}", model.resolved_includes.len());

            // Should have resolved includes
            assert!(
                model.resolved_includes.contains_key("sources"),
                "Should resolve 'sources' include"
            );

            // Verify included model has expected content
            if let Some(resolved) = model.resolved_includes.get("sources") {
                println!("  - sources: {} scalars", resolved.model.scalars.len());
                assert!(
                    resolved.model.scalars.contains_key("pricing.unit_price")
                        || !resolved.model.scalars.is_empty(),
                    "Included model should have scalars"
                );
            }
        }
        Err(e) => {
            panic!("Failed to parse model with includes: {}", e);
        }
    }

    assert!(result.is_ok());
}

#[test]
fn test_parse_saas_unit_economics() {
    let path = Path::new("test-data/v1.0/saas_unit_economics.yaml");
    let result = parse_model(path);

    match &result {
        Ok(model) => {
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
