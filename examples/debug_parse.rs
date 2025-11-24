use royalbit_forge::parser;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from("test-data/v1.0/quarterly_pl.yaml");
    let model = parser::parse_model(&path).unwrap();

    println!("=== Parsed Model ===");
    println!("Version: {:?}", model.version);
    println!("\nTables:");
    for (table_name, table) in &model.tables {
        println!("\n  Table: {}", table_name);
        println!(
            "    Columns: {:?}",
            table.columns.keys().collect::<Vec<_>>()
        );
        println!("    Row formulas: {:?}", table.row_formulas);
    }
}
