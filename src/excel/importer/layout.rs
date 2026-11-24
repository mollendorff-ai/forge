//! A sheet someone else wrote (roadmap T1-EXAMPLE-EXCEL-ROUNDTRIP): titles
//! above the header, a blank row between a table's rows, a total row under
//! them, and labels in sentences. `tidy` takes those rows out of the sheet,
//! keeping the Excel row each remaining row came from, so a formula that
//! reads the sheet is still translated by the rows it names.
//!
//! A sheet `forge export` wrote has its header in its first row, a formula
//! in every row of a formula column and no total row, so `tidy` leaves it
//! as it is.

use super::sheet::{number_of, Sheet};
use super::{ExcelImporter, Names};
use crate::excel::workbook_refs::dates::Holds;
use crate::excel::workbook_refs::{Context, ReverseRefs, SheetKind};
use crate::types::{ParsedModel, Variable};
use calamine::{Data, Dimensions, Range};

impl Sheet {
    /// Whether the cell at a 0-based `row` and `col` holds a value or a formula.
    pub(super) fn filled(&self, row: usize, col: usize) -> bool {
        self.formula(row, col).is_some()
            || !matches!(self.values.get((row, col)), None | Some(Data::Empty))
    }

    /// Whether any cell of `row` holds a value or a formula.
    fn filled_row(&self, row: usize) -> bool {
        let (_, width) = self.values.get_size();
        (0..width).any(|col| self.filled(row, col))
    }

    /// The columns of `row` that hold text typed in.
    fn texts(&self, row: usize) -> Vec<usize> {
        let (_, width) = self.values.get_size();
        (0..width)
            .filter(|col| matches!(self.values.get((row, *col)), Some(Data::String(t)) if !t.is_empty()))
            .collect()
    }

    /// Whether `row` holds a number, a date, a boolean, an error or a formula.
    pub(super) fn holds_data(&self, row: usize) -> bool {
        let (_, width) = self.values.get_size();
        (0..width).any(|col| {
            self.formula(row, col).is_some()
                || !matches!(
                    self.values.get((row, col)),
                    None | Some(Data::Empty | Data::String(_))
                )
        })
    }

    /// The text of a label in the first column, or of a header, as a name
    /// Forge reads: kept when `keep` (every name of the sheet already is one),
    /// otherwise in lower case with `_` for spaces and punctuation.
    pub(super) fn name_from(text: &str, keep: bool) -> String {
        if keep {
            return text.to_string();
        }
        let mut name = String::new();
        for c in text.trim().chars() {
            match c {
                '%' => name.push_str("_pct_"),
                '&' => name.push_str("_and_"),
                c if c.is_alphanumeric() => name.extend(c.to_lowercase()),
                _ => name.push('_'),
            }
        }
        let name = name
            .split('_')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("_");
        match name.chars().next() {
            Some(c) if c.is_alphabetic() => name,
            // A name starts with a letter
            _ => format!("n_{name}"),
        }
    }

    /// The name of column `col` of a table: its header, kept when every
    /// header of the sheet is a name already, otherwise made one; a column
    /// without a header is named by its place.
    pub(super) fn column_name(&self, col: usize) -> String {
        let (_, width) = self.values.get_size();
        let keep = (0..width).all(|c| self.header(c).is_none_or(|h| Self::is_name(&h)));
        self.header(col)
            .map_or_else(|| format!("col_{col}"), |h| Self::name_from(&h, keep))
    }

    /// A sheet of labels and values: text in the first column of every row
    /// with something in it, and `Value`, or nothing, over the second, or
    /// any header in any language when other sheets read its values one
    /// cell at a time (`cells_read`). Its labels are the names of scalars
    /// (`Opening volume` is `opening_volume`, `Coût fixe` `coût_fixe`).
    pub(super) fn is_label_sheet(&self) -> bool {
        let (height, width) = self.values.get_size();
        // A first row of a label and its value is read as the header
        // (`import_with_warnings` says so)
        let value = self.read_one_at_a_time
            || self.holds_data(0)
            || self
                .header(1)
                .is_none_or(|h| h.eq_ignore_ascii_case("value"));
        let rows: Vec<usize> = (1..height).filter(|row| self.filled_row(*row)).collect();
        let labelled = rows.iter().all(|row| {
            self.formula(*row, 0).is_none()
                && matches!(self.values.get((*row, 0)), Some(Data::String(t)) if !t.is_empty())
        });
        width >= 2 && value && labelled && rows.iter().any(|row| self.filled(*row, 1))
    }

    /// The rows of a sheet of scalars that have a name, with the name: the
    /// label as it is on a sheet Forge wrote (`Name` over the labels), or
    /// made a name; a name met again gets `_2`, `_3`.
    pub(super) fn labels(&self) -> Vec<(usize, String)> {
        let (height, _) = self.values.get_size();
        let forge = self
            .header(0)
            .is_some_and(|h| h.eq_ignore_ascii_case("name"));
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut labels = Vec::new();
        for row in 1..height {
            let text = self
                .values
                .get((row, 0))
                .map(ToString::to_string)
                .unwrap_or_default();
            if text.is_empty() {
                continue;
            }
            let name = Self::name_from(&text, forge);
            let count = seen.entry(name.clone()).or_insert(0);
            *count += 1;
            labels.push((
                row,
                if *count == 1 {
                    name
                } else {
                    format!("{name}_{count}")
                },
            ));
        }
        labels
    }

    /// The Excel row of a total row under the table `table`, and the group
    /// its cells are scalars of: the table's name and the row's label
    /// (`monthly_total` for `Total`).
    pub(super) fn total_group(&self, table: &str) -> Option<(u32, String)> {
        let (row, cells) = self.total.as_ref()?;
        let label = match cells.first() {
            Some(Data::String(text)) if !text.is_empty() => Self::name_from(text, false),
            _ => "total".to_string(),
        };
        Some((row + 1, format!("{table}_{label}")))
    }

    /// Whether `text` is a name as Forge writes one: a letter or `_`, then
    /// letters, digits and `_`.
    pub(super) fn is_name(text: &str) -> bool {
        let mut chars = text.chars();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    }
}

impl ExcelImporter {
    /// The cells of a total row under a table, as scalars of its group
    /// (`Sheet::total_group`): a formula as its formula, a number typed in
    /// as the number. The label and empty cells are left out.
    pub(super) fn import_total(
        sheet: &Sheet,
        read: (&ReverseRefs, &Names),
        model: &mut ParsedModel,
        warnings: &mut Vec<String>,
    ) {
        let (refs, names) = read;
        let (
            Some((row, cells)),
            Some(SheetKind::Table {
                columns,
                total: Some((_, group)),
                ..
            }),
        ) = (&sheet.total, refs.sheets.get(&sheet.name))
        else {
            return;
        };
        let c0 = sheet.at(0, 0).1;
        for (col, cell) in cells.iter().enumerate() {
            let at = (*row, c0 + u32::try_from(col).unwrap_or(0));
            let formula = sheet.formula_at(at);
            if formula.is_none() && number_of(cell).is_none() {
                continue;
            }
            let path = format!("{group}.{}", columns[&sheet.letter(col)]);
            let formula = formula.and_then(|excel| {
                refs.total_to_forge(&excel, &sheet.name)
                    .and_then(|forge| {
                        names.of(refs, &sheet.name).read_dates(
                            &forge,
                            Context::Scalar { group: Some(group) },
                            Holds::Other,
                        )
                    })
                    .map_err(|reason| {
                        warnings.push(format!("{path}: imported as a value ({reason}): {excel}"));
                    })
                    .ok()
            });
            model.add_scalar(path.clone(), Variable::new(path, number_of(cell), formula));
        }
    }
}

/// Take the titles above the header, blank rows inside a table and its total
/// row out of `sheet`. Rows that stay keep the Excel row they came from.
pub(super) fn tidy(sheet: Sheet, merged: &[Dimensions]) -> Sheet {
    let (height, width) = sheet.values.get_size();
    let header = header_row(&sheet, merged);
    // A blank row between two rows of formulas is a spacer: `forge export`
    // writes a formula column's formula in every row
    let formulas: Vec<usize> = (header + 1..height)
        .filter(|row| (0..width).any(|col| sheet.formula(*row, col).is_some()))
        .collect();
    let spacer = |row: usize| {
        !sheet.filled_row(row)
            && formulas.first().is_some_and(|first| *first < row)
            && formulas.last().is_some_and(|last| *last > row)
    };
    let total = (header + 2..height)
        .next_back()
        .filter(|last| is_total_row(&sheet, *last));
    let keep: Vec<usize> = std::iter::once(header)
        .chain((header + 1..height).filter(|row| !spacer(*row) && Some(*row) != total))
        .collect();
    if keep.len() == height {
        return sheet;
    }
    let c0 = sheet.values.start().map_or(0, |(_, c)| c);
    let last_col = c0 + u32::try_from(width).unwrap_or(1) - 1;
    let last_row = u32::try_from(keep.len()).unwrap_or(1) - 1;
    let mut values = Range::new((0, c0), (last_row, last_col));
    for (to, from) in keep.iter().enumerate() {
        for col in 0..width {
            let cell = sheet
                .values
                .get((*from, col))
                .cloned()
                .unwrap_or(Data::Empty);
            let at = (
                u32::try_from(to).unwrap_or(0),
                c0 + u32::try_from(col).unwrap_or(0),
            );
            values.set_value(at, cell);
        }
    }
    let total = total.map(|row| {
        let cells = (0..width)
            .map(|col| sheet.values.get((row, col)).cloned().unwrap_or(Data::Empty))
            .collect();
        (sheet.rows[row], cells)
    });
    let rows = keep.iter().map(|row| sheet.rows[*row]).collect();
    Sheet {
        values,
        rows,
        total,
        ..sheet
    }
}

/// The header of a sheet: its first row, unless that row and the rows under
/// it up to the header are titles. A title row holds one cell of text, or
/// only headings merged across several columns, or nothing; the header
/// after them holds text in two cells at least, and is the last row before
/// the first row of data (a number, a date, a formula).
fn header_row(sheet: &Sheet, merged: &[Dimensions]) -> usize {
    let (height, _) = sheet.values.get_size();
    let Some(first_data) = (0..height).find(|row| sheet.holds_data(*row)) else {
        return 0;
    };
    let heading = |row: usize, col: usize| {
        let at = sheet.at(row, col);
        merged.iter().any(|m| m.start == at && m.end.1 > m.start.1)
    };
    let is_title = |row: usize| {
        let texts = sheet.texts(row);
        texts.len() <= 1 || texts.iter().all(|col| heading(row, *col))
    };
    (1..first_data)
        .rev()
        .find(|row| sheet.texts(*row).len() >= 2 && (0..*row).all(is_title))
        .unwrap_or(0)
}

/// A table's last row is its total when one of its cells adds up, or
/// otherwise reads, a range of its own column (`=SUM(D4:D16)` in column D),
/// which no row of a table Forge wrote does: a row formula cannot read the
/// column it calculates.
fn is_total_row(sheet: &Sheet, row: usize) -> bool {
    let (_, width) = sheet.values.get_size();
    (0..width).any(|col| {
        sheet
            .formula(row, col)
            .is_some_and(|formula| reads_own_column(&formula, &sheet.letter(col)))
    })
}

/// Whether `formula` holds a range of the column `letter` on its own sheet
/// (`D4:D16`, `$D$4:$D$16`): not one after a sheet's `!`, nor inside a name.
fn reads_own_column(formula: &str, letter: &str) -> bool {
    let chars: Vec<char> = formula.chars().collect();
    (0..chars.len()).any(|at| {
        let after_a_name = at > 0 && {
            let before = chars[at - 1];
            before.is_ascii_alphanumeric() || matches!(before, '!' | '_' | '.' | '$')
        };
        !after_a_name && range_column(&chars[at..]).is_some_and(|column| column == letter)
    })
}

/// The letters of a cell (`$D$4`, `AB12`) at the start of `chars`, and how
/// many characters it takes.
fn cell_column(chars: &[char]) -> Option<(String, usize)> {
    let dollar = |at: usize| usize::from(chars.get(at) == Some(&'$'));
    let start = dollar(0);
    let letters = chars[start..]
        .iter()
        .take_while(|c| c.is_ascii_uppercase())
        .count();
    let row = start + letters + dollar(start + letters);
    let digits = chars[row..]
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .count();
    ((1..=3).contains(&letters) && digits > 0)
        .then(|| (chars[start..start + letters].iter().collect(), row + digits))
}

/// The column of a range `D4:D16` at the start of `chars`, when both ends
/// are in the same column.
fn range_column(chars: &[char]) -> Option<String> {
    let (first, taken) = cell_column(chars)?;
    (chars.get(taken) == Some(&':')).then_some(())?;
    let (last, _) = cell_column(&chars[taken + 1..])?;
    (first == last).then_some(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::excel::ExcelImporter;
    use calamine::{Reader, Xlsx};
    use rust_xlsxwriter::{Format, Formula, Workbook};
    use std::io::Cursor;

    fn read(mut book: Workbook) -> Vec<Sheet> {
        let bytes = book.save_to_buffer().unwrap();
        let mut workbook = Xlsx::new(Cursor::new(bytes)).unwrap();
        let names = workbook.sheet_names();
        names
            .into_iter()
            .map(|name| {
                let values = workbook.worksheet_range(&name).unwrap();
                let formulas = workbook.worksheet_formula(&name).ok();
                let merged = workbook.merge_cells_by_sheet_name(&name).unwrap();
                tidy(Sheet::new(name, values, formulas, None), &merged)
            })
            .collect()
    }

    fn texts(sheet: &Sheet, row: usize) -> Vec<String> {
        let (_, width) = sheet.values.get_size();
        (0..width)
            .map(|col| {
                sheet
                    .values
                    .get((row, col))
                    .map(ToString::to_string)
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Titles, a merged heading, a blank row between formulas and a total row
    /// come out; the rows that stay keep their Excel rows.
    #[test]
    fn titles_a_spacer_and_a_total_come_out() {
        let mut book = Workbook::new();
        let sheet = book.add_worksheet().set_name("Monthly").unwrap();
        sheet.write_string(0, 0, "Monthly P&L").unwrap();
        sheet
            .merge_range(1, 1, 1, 2, "Sales", &Format::new())
            .unwrap();
        for (col, text) in ["Month", "Volume", "Revenue"].iter().enumerate() {
            sheet
                .write_string(2, u16::try_from(col).unwrap(), *text)
                .unwrap();
        }
        // Rows 4-5, a blank row 6, row 7, a total row 8
        for row in [3, 4, 6] {
            sheet.write_string(row, 0, format!("m{row}")).unwrap();
            sheet.write_number(row, 1, f64::from(row)).unwrap();
            sheet
                .write_formula(row, 2, Formula::new(format!("=B{}*2", row + 1)))
                .unwrap();
        }
        sheet.write_string(7, 0, "Total").unwrap();
        sheet
            .write_formula(7, 1, Formula::new("=SUM(B4:B7)"))
            .unwrap();
        sheet
            .write_formula(7, 2, Formula::new("=SUM($C$4:$C$7)"))
            .unwrap();
        let sheets = read(book);
        let monthly = &sheets[0];
        assert_eq!(monthly.rows, [2, 3, 4, 6]);
        assert_eq!(texts(monthly, 0), ["Month", "Volume", "Revenue"]);
        assert_eq!(monthly.excel_row(3), 7);
        assert_eq!(monthly.formula(3, 2).as_deref(), Some("=B7*2"));
        let (row, cells) = monthly.total.as_ref().unwrap();
        assert_eq!(*row, 7);
        assert_eq!(cells[0], Data::String("Total".to_string()));
        assert_eq!(monthly.data_rows(), [4, 5, 7]);
    }

    /// A sheet as `forge export` writes it is left as it is: a one-column
    /// table whose header is one cell, a blank row in a table without
    /// formulas (an empty cell, ADR-059), and a last row of text.
    #[test]
    fn a_sheet_forge_wrote_is_left_as_it_is() {
        let mut book = Workbook::new();
        let one = book.add_worksheet().set_name("one").unwrap();
        one.write_string(0, 0, "amount").unwrap();
        one.write_string(1, 0, "n/a").unwrap();
        one.write_number(2, 0, 3.0).unwrap();
        let gaps = book.add_worksheet().set_name("gaps").unwrap();
        gaps.write_string(0, 0, "x").unwrap();
        gaps.write_number(1, 0, 1.0).unwrap();
        gaps.write_number(3, 0, 3.0).unwrap();
        let labels = book.add_worksheet().set_name("labels").unwrap();
        labels.write_string(0, 0, "line").unwrap();
        labels.write_string(0, 1, "n").unwrap();
        labels.write_string(1, 0, "a").unwrap();
        labels.write_number(1, 1, 2.0).unwrap();
        labels.write_string(2, 0, "Total").unwrap();
        labels
            .write_formula(2, 1, Formula::new("=SUM(A2:A2)"))
            .unwrap();
        let text = book.add_worksheet().set_name("text").unwrap();
        text.write_string(0, 0, "a").unwrap();
        text.write_string(1, 0, "b").unwrap();
        for sheet in read(book) {
            let (height, _) = sheet.values.get_size();
            let identity: Vec<u32> = (0..u32::try_from(height).unwrap()).collect();
            assert_eq!(sheet.rows, identity, "{}", sheet.name);
            assert!(sheet.total.is_none(), "{}", sheet.name);
        }
    }

    #[test]
    fn a_header_after_titles_has_two_cells_and_titles_have_one_or_are_merged() {
        let mut book = Workbook::new();
        // Two unmerged cells above the header: not titles, so the first row
        // is the header
        let two = book.add_worksheet().set_name("two").unwrap();
        two.write_string(0, 0, "a").unwrap();
        two.write_string(0, 1, "b").unwrap();
        two.write_string(1, 0, "c").unwrap();
        two.write_string(1, 1, "d").unwrap();
        two.write_number(2, 0, 1.0).unwrap();
        // A title, nothing, then the header
        let titled = book.add_worksheet().set_name("titled").unwrap();
        titled.write_string(0, 0, "A title").unwrap();
        titled.write_string(2, 0, "x").unwrap();
        titled.write_string(2, 1, "y").unwrap();
        titled.write_number(3, 0, 1.0).unwrap();
        titled.write_number(3, 1, 2.0).unwrap();
        let sheets = read(book);
        assert_eq!(sheets[0].rows, [0, 1, 2]);
        assert_eq!(sheets[1].rows, [2, 3]);
        assert_eq!(texts(&sheets[1], 0), ["x", "y"]);
    }

    #[test]
    fn a_range_of_its_own_column_makes_a_total() {
        assert!(reads_own_column("=SUM(D4:D16)", "D"));
        assert!(reads_own_column("=AVERAGE($D$4:$D$16)/2", "D"));
        assert!(!reads_own_column("=SUM(E4:E16)", "D"));
        assert!(!reads_own_column("=SUM(Other!D4:D16)", "D"));
        assert!(!reads_own_column("=D4+D16", "D"));
        // Two columns, no row, no letters, four letters, a name that ends like a cell
        assert!(!reads_own_column("=SUM(D4:E16)", "D"));
        assert!(!reads_own_column("=SUM(D:D)", "D"));
        assert!(!reads_own_column("=SUM(D4:16)", "D"));
        assert!(!reads_own_column("=SUM(4:D5)", "D"));
        assert!(!reads_own_column("=SUM(ABCD4:ABCD9)", "ABCD"));
        assert!(!reads_own_column("=SUM(x_D4:D9)", "D"));
        assert!(reads_own_column("=SUM(AB4:AB9)+1", "AB"));
    }

    #[test]
    fn a_label_becomes_a_name() {
        assert_eq!(
            Sheet::name_from("Gross margin %", false),
            "gross_margin_pct"
        );
        assert_eq!(Sheet::name_from("Volume (kg)", false), "volume_kg");
        assert_eq!(Sheet::name_from("R&D", false), "r_and_d");
        assert_eq!(Sheet::name_from("2027 plan", false), "n_2027_plan");
        assert_eq!(Sheet::name_from("EBITDA", false), "ebitda");
        assert_eq!(Sheet::name_from("EBITDA", true), "EBITDA");
        assert!(Sheet::is_name("year_1"));
        assert!(Sheet::is_name("_x"));
        assert!(!Sheet::is_name("Gross margin"));
        assert!(!Sheet::is_name("1st"));
        assert!(!Sheet::is_name(""));
    }

    fn import(mut book: Workbook) -> (ParsedModel, Vec<String>) {
        let bytes = book.save_to_buffer().unwrap();
        ExcelImporter::from_bytes(bytes)
            .import_with_warnings()
            .unwrap()
    }

    fn formula(model: &ParsedModel, path: &str) -> Option<String> {
        model.scalars[path].formula.clone()
    }

    /// A workbook laid out by hand: drivers with units, notes, another
    /// column and a label twice; a table under a title with a named range,
    /// `ROW()` and a total row without a label; labels without a header.
    #[test]
    fn a_workbook_laid_out_by_hand() {
        let mut book = Workbook::new();
        let inputs = book.add_worksheet().set_name("Inputs").unwrap();
        for (col, text) in ["Item", "Value", "Unit", "Notes", "Owner", "Empty"]
            .iter()
            .enumerate()
        {
            inputs
                .write_string(0, u16::try_from(col).unwrap(), *text)
                .unwrap();
        }
        inputs.write_string(1, 0, "Unit price").unwrap();
        inputs.write_number(1, 1, 2.0).unwrap();
        inputs.write_string(1, 2, "$").unwrap();
        inputs.write_string(1, 3, "list").unwrap();
        inputs.write_string(1, 4, "Ana").unwrap();
        inputs.write_string(2, 0, "Unit price").unwrap();
        inputs.write_number(2, 1, 3.0).unwrap();
        let sales = book.add_worksheet().set_name("Sales").unwrap();
        sales.write_string(0, 0, "Sales by month").unwrap();
        for (col, text) in ["Month", "Qty", "Amount", "Fee", "Row"].iter().enumerate() {
            sales
                .write_string(1, u16::try_from(col).unwrap(), *text)
                .unwrap();
        }
        for (i, qty) in [10.0, 20.0, 30.0].iter().enumerate() {
            let row = u32::try_from(i).unwrap() + 2;
            sales.write_string(row, 0, format!("m{i}")).unwrap();
            sales.write_number(row, 1, *qty).unwrap();
            let amount =
                Formula::new(format!("=B{}*price", row + 1)).set_result(format!("{}", qty * 2.0));
            sales.write_formula(row, 2, amount).unwrap();
            sales.write_number(row, 3, 1.0).unwrap();
            sales
                .write_formula(
                    row,
                    4,
                    Formula::new("=ROW()").set_result(format!("{}", row + 1)),
                )
                .unwrap();
        }
        // A total row with no label: a sum, a number typed in, part of a column
        sales
            .write_formula(5, 1, Formula::new("=SUM(B3:B5)").set_result("60"))
            .unwrap();
        sales.write_number(5, 2, 99.0).unwrap();
        sales
            .write_formula(5, 3, Formula::new("=SUM(D3:D4)").set_result("2"))
            .unwrap();
        let out = book.add_worksheet().set_name("Out").unwrap();
        out.write_string(0, 0, "Out").unwrap();
        out.write_string(2, 0, "Total qty").unwrap();
        out.write_formula(2, 1, Formula::new("=Sales!B6*2").set_result("120"))
            .unwrap();
        out.write_string(3, 0, "A range of the total").unwrap();
        out.write_formula(3, 1, Formula::new("=SUM(Sales!B6:B6)").set_result("60"))
            .unwrap();
        out.write_string(4, 0, "Text \"price\"").unwrap();
        out.write_formula(
            4,
            1,
            Formula::new("=LEN(\"price\")+'Inputs'!$B$2").set_result("7"),
        )
        .unwrap();
        // Excel's own name for it, `_xlnm.Print_Area`, is not one of the workbook's
        out.set_print_area(0, 0, 4, 1).unwrap();
        book.define_name("price", "=Inputs!$B$2").unwrap();

        let (model, warnings) = import(book);
        check_laid_out_by_hand(&model, &warnings.join("\n"));
    }

    fn check_laid_out_by_hand(model: &ParsedModel, warnings: &str) {
        // Inputs: a unit and notes kept, another column named, a label twice
        let price = &model.scalars["inputs.unit_price"];
        assert_eq!(price.value, Some(2.0));
        assert_eq!(price.metadata.unit.as_deref(), Some("$"));
        assert_eq!(price.metadata.notes.as_deref(), Some("list"));
        assert_eq!(model.scalars["inputs.unit_price_2"].value, Some(3.0));
        assert!(
            warnings.contains("Inputs: column E (Owner) is not imported"),
            "{warnings}"
        );
        assert!(!warnings.contains("Empty"), "{warnings}");
        // Sales: its headers are names, kept as they are; under its title, the named range read, ROW() from its first row
        let table = &model.tables["sales"];
        assert_eq!(table.row_formulas["Amount"], "=Qty * inputs.unit_price");
        assert_eq!(table.row_formulas["Row"], "=(ROW() + 2)");
        assert_eq!(
            formula(model, "sales_total.Qty").as_deref(),
            Some("=SUM(sales.Qty)")
        );
        assert_eq!(model.scalars["sales_total.Amount"].value, Some(99.0));
        assert_eq!(formula(model, "sales_total.Amount"), None);
        assert_eq!(formula(model, "sales_total.Fee"), None);
        assert_eq!(model.scalars["sales_total.Fee"].value, Some(2.0));
        assert!(
            warnings
                .contains("sales_total.Fee: imported as a value (a range over part of a column"),
            "{warnings}"
        );
        assert!(!model.scalars.contains_key("sales_total.Month"));
        // Out: labels under a title, a total cell, a range of it refused
        assert_eq!(
            formula(model, "out.total_qty").as_deref(),
            Some("=sales_total.Qty * 2")
        );
        assert!(warnings.contains("a range in a total row"), "{warnings}");
        assert_eq!(
            formula(model, "out.text_price").as_deref(),
            Some("=LEN(\"price\") + inputs.unit_price")
        );
    }

    /// `ROW()` in a table with a blank row inside is not Forge's `ROW()`.
    #[test]
    fn row_in_a_table_with_a_gap_is_its_value() {
        let mut book = Workbook::new();
        let t = book.add_worksheet().set_name("t").unwrap();
        t.write_string(0, 0, "a").unwrap();
        t.write_string(0, 1, "r").unwrap();
        for row in [1, 3] {
            t.write_number(row, 0, 1.0).unwrap();
            let r = Formula::new("=ROW()").set_result(format!("{}", row + 1));
            t.write_formula(row, 1, r).unwrap();
        }
        let (model, warnings) = import(book);
        assert!(
            warnings[0].contains("ROW() in a table with a gap in its rows"),
            "{warnings:?}"
        );
        assert_eq!(
            model.tables["t"].columns["r"].values,
            crate::types::ColumnValue::Number(vec![2.0, 4.0])
        );
    }

    #[test]
    fn a_sheet_of_labels_and_what_is_not_one() {
        let mut book = Workbook::new();
        // Text over the second column: a table
        let table = book.add_worksheet().set_name("table").unwrap();
        table.write_string(0, 0, "name").unwrap();
        table.write_string(0, 1, "amount").unwrap();
        table.write_string(1, 0, "a").unwrap();
        table.write_number(1, 1, 1.0).unwrap();
        // Labels with nothing beside them: a table of one text column
        let bare = book.add_worksheet().set_name("bare").unwrap();
        bare.write_string(0, 0, "Title").unwrap();
        bare.write_string(0, 1, "Value").unwrap();
        bare.write_string(1, 0, "a").unwrap();
        // A number where a label would be
        let numbered = book.add_worksheet().set_name("numbered").unwrap();
        numbered.write_string(0, 0, "x").unwrap();
        numbered.write_string(0, 1, "Value").unwrap();
        numbered.write_number(1, 0, 1.0).unwrap();
        numbered.write_number(1, 1, 2.0).unwrap();
        let sheets = read(book);
        assert!(!sheets[0].is_label_sheet());
        assert!(!sheets[1].is_label_sheet());
        assert!(!sheets[2].is_label_sheet());
        assert_eq!(sheets[0].column_name(1), "amount");
    }

    /// Labels from the first row: that row is read as the header, and the
    /// import says so, as its value is not in the model.
    #[test]
    fn a_first_row_that_holds_a_value_is_named() {
        let mut book = Workbook::new();
        let out = book.add_worksheet().set_name("Out").unwrap();
        out.write_string(0, 0, "Revenue").unwrap();
        out.write_number(0, 1, 10.0).unwrap();
        out.write_string(1, 0, "Cost").unwrap();
        out.write_number(1, 1, 4.0).unwrap();
        let (model, warnings) = import(book);
        assert_eq!(model.scalars["out.cost"].value, Some(4.0));
        assert_eq!(model.scalars.len(), 1);
        assert_eq!(
            warnings,
            [
                "Out: its first row holds a value or a formula, and is read as the header, so \
              that row is not in the model; give the sheet a header or a title row"
            ]
        );
    }
}
