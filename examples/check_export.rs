use calamine::{open_workbook, Reader, Xlsx};

fn main() {
    let path = "/tmp/test_export_fixed.xlsx";
    let mut workbook: Xlsx<_> = open_workbook(path).unwrap();

    println!("=== All sheets ===");
    for sheet_name in workbook.sheet_names() {
        println!("\nSheet: {}", sheet_name);

        if let Ok(data_range) = workbook.worksheet_range(&sheet_name) {
            println!("  Data range size: {:?}", data_range.get_size());
        }

        if let Ok(formula_range) = workbook.worksheet_formula(&sheet_name) {
            let (h, w) = formula_range.get_size();
            if h > 0 && w > 0 {
                println!("  Formula range size: {:?}", (h, w));
                println!("  Formulas found:");
                for row in 0..h {
                    for col in 0..w {
                        if let Some(cell) = formula_range.get((row, col)) {
                            if !cell.is_empty() {
                                println!("    [{},{}] = '{}'", row, col, cell);
                            }
                        }
                    }
                }
            } else {
                println!("  No formulas");
            }
        }
    }
}
