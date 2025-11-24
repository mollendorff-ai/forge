use calamine::{open_workbook, Reader, Xlsx};

fn main() {
    let path = "/tmp/test_export_fixed.xlsx";
    let mut workbook: Xlsx<_> = open_workbook(path).unwrap();

    for sheet_name in ["pl_2025", "opex_2025", "final_pl"] {
        println!("\n=== Sheet: {} ===", sheet_name);

        if let Ok(data_range) = workbook.worksheet_range(&sheet_name) {
            println!("Headers:");
            for col in 0..data_range.get_size().1 {
                if let Some(cell) = data_range.get((0, col)) {
                    println!("  Col {}: {}", col, cell);
                }
            }
        }

        if let Ok(formula_range) = workbook.worksheet_formula(&sheet_name) {
            println!("\nFormulas (first data row):");
            for col in 0..formula_range.get_size().1 {
                if let Some(formula) = formula_range.get((1, col)) {
                    if !formula.is_empty() {
                        if let Ok(data_range) = workbook.worksheet_range(&sheet_name) {
                            if let Some(header) = data_range.get((0, col)) {
                                println!("  Col {} ({}): {}", col, header, formula);
                            }
                        }
                    }
                }
            }
        }
    }
}
