use calamine::{open_workbook, Reader, Xlsx};

fn main() {
    let path = "/tmp/quarterly_pl_exported.xlsx";
    let mut workbook: Xlsx<_> = open_workbook(path).unwrap();

    // Check pl_2025 sheet
    println!("=== Checking pl_2025 sheet ===");

    if let Ok(data_range) = workbook.worksheet_range("pl_2025") {
        println!("Data range size: {:?}", data_range.get_size());
        println!("\nFirst few data cells:");
        for row in 0..5 {
            for col in 0..6 {
                if let Some(cell) = data_range.get((row, col)) {
                    println!("  Data[{row},{col}] = {cell:?}");
                }
            }
        }
    }

    if let Ok(formula_range) = workbook.worksheet_formula("pl_2025") {
        println!("\nFormula range size: {:?}", formula_range.get_size());
        println!("\nAll formula cells:");
        for row in 0..5 {
            for col in 0..6 {
                if let Some(cell) = formula_range.get((row, col)) {
                    if !cell.is_empty() {
                        println!("  Formula[{row},{col}] = '{cell}'");
                    }
                }
            }
        }
    } else {
        println!("\nNo formulas found in pl_2025");
    }
}
