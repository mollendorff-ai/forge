//! Formula translation between Forge and Excel, token by token.
//!
//! Export and import share one picture of the workbook:
//! - a table is a sheet named after it, one column per data or formula column
//!   (in model order), header in row 1, values from row 2;
//! - a scalar group `g` (scalars named `g.name`) is a sheet `g` with `Name` in
//!   column A and `Value` in column B, one scalar per row (in model order);
//! - ungrouped scalars live on a sheet named `Scalars` in the same layout.
//!
//! References are resolved the way the calculator resolves them, so a formula
//! means the same thing in Excel as in Forge. When it cannot (a function Excel
//! does not have, array indexing, `IRR` over several separate cash flows), the
//! translation fails with a reason and the caller writes the computed value.

use crate::core::array_calculator::tokenizer::{tokenize, Token};
use std::collections::HashMap;

mod constants;
pub mod dates;
mod local_names;
mod previous_row;
mod reverse;
use local_names::Locals;
pub use reverse::{ReverseRefs, SheetKind};
use std::fmt::Write as _;

/// Sheet that holds scalars without a group.
pub const UNGROUPED_SHEET: &str = "Scalars";

/// Functions Excel evaluates differently from Forge: Forge's `VLOOKUP` and
/// `HLOOKUP` read a table of one column or one row, which a workbook's range
/// need not be (the export writes them where Excel's table is that too:
/// [`WorkbookRefs::one_line_lookup`]), `ROW`/`COLUMN`/`OFFSET`/`INDIRECT`
/// work on Forge names rather than sheet cells (`ROW()` alone is the table's
/// row and is translated: `previous_row`), `NOW` returns text instead of a
/// date serial, and `UNIQUE`/`SORT`/`FILTER`/`SEQUENCE` spill in Excel.
/// `forge import` does not read a formula that uses one either: it would
/// calculate something else in Forge.
/// (`LET`, `LAMBDA` called at once and `COUNTUNIQUE` of a column are written
/// as the Excel expression that computes the same thing: `local_names`. A
/// `TODAY` whose result is a text column is written as values by the
/// exporter, which knows the column's type.)
const DIFFERENT_IN_EXCEL: [&str; 12] = [
    "NOW",
    "UNIQUE",
    "SORT",
    "FILTER",
    "SEQUENCE",
    "RANDARRAY",
    "VLOOKUP",
    "HLOOKUP",
    "INDIRECT",
    "OFFSET",
    "ROW",
    "COLUMN",
];

/// An open parenthesis: the function called, if any; the argument being
/// written (0-based); how many arguments the call has
pub(super) type Call = (Option<String>, usize, usize);

/// Forge names for functions Excel calls otherwise (`E()` is `EXP(1)`).
fn excel_name(upper: &str) -> &str {
    match upper {
        "AVG" => "AVERAGE",
        "POW" => "POWER",
        "E" => "EXP",
        other => other,
    }
}

/// Where a formula is written.
#[derive(Debug, Clone, Copy)]
pub enum Context<'a> {
    /// A row formula of `table`, written in Excel row `row` (1-based).
    Row { table: &'a str, row: u32 },
    /// A scalar formula of a group (`None` for ungrouped scalars).
    Scalar { group: Option<&'a str> },
}

/// A translated Excel formula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translated {
    /// Formula text, starting with `=`
    pub formula: String,
    /// Whether it uses a multi-cell range (write it as a dynamic array formula)
    pub has_range: bool,
    /// Whether its result is a date: a day number in the workbook, to be
    /// shown as a date (ADR-054)
    pub is_date: bool,
}

/// Layout of the exported workbook.
#[derive(Debug, Clone, Default)]
pub struct WorkbookRefs {
    /// table -> (column -> letter)
    pub columns: HashMap<String, HashMap<String, String>>,
    /// table -> number of data rows
    pub rows: HashMap<String, usize>,
    /// scalar path -> (sheet, Excel row)
    pub scalars: HashMap<String, (String, u32)>,
    /// table -> its sheet, where that is not the table's name (a table of an
    /// included file: `@costs.prices` and `prices` in its own layout are on
    /// the sheet `costs.prices`)
    pub sheets: HashMap<String, String>,
    /// `table.column` (`@ns.table.column` for an include) -> whether the
    /// column holds text (`true`) or numbers (`false`); other columns are not
    /// listed. `COUNTUNIQUE` needs it: Forge compares text exactly, Excel's
    /// `UNIQUE` ignores case.
    pub text_columns: HashMap<String, bool>,
    /// table -> (column letter -> number of values), for a column shorter
    /// than its table: its range ends at its own last value, not the table's
    /// (`ROWS`, `COUNTA` and `INDEX` would read the empty cells)
    pub column_rows: HashMap<String, HashMap<String, usize>>,
    /// `table.column` (`@ns.table.column` for an include) of every column
    /// written as dates: each value a date Excel can hold (ADR-054)
    pub date_columns: std::collections::HashSet<String>,
    /// `table.column` of every column that is text in the workbook and holds
    /// a date Forge reads (`n/a` among dates, or a date before 1900-03-01).
    /// Forge reads its day number where a number is read; Excel reads text
    /// (ADR-054)
    pub mixed_columns: std::collections::HashSet<String>,
    /// The path of every scalar whose value is a date (`@ns.path` for an
    /// include's): a date cell, read as a cell of a date column is (ADR-060)
    pub date_scalars: std::collections::HashSet<String>,
}

/// Excel column letter for a 0-based index (0 -> A, 26 -> AA).
#[must_use]
pub fn column_letter(mut index: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push(char::from(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ"[index % 26]));
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    letters.iter().rev().collect()
}

/// `'name'!` with quotes doubled, as Excel requires.
#[must_use]
pub fn sheet_prefix(sheet: &str) -> String {
    format!("'{}'!", sheet.replace('\'', "''"))
}

/// The sheet that holds an included file's scalars.
#[must_use]
pub fn include_sheet(namespace: &str) -> String {
    format!("{namespace}.Scalars")
}

/// Split `group.name`; a path without a dot is ungrouped.
#[must_use]
pub fn split_scalar_path(path: &str) -> (Option<&str>, &str) {
    path.split_once('.')
        .map_or((None, path), |(group, name)| (Some(group), name))
}

/// Excel text for a number literal.
fn number_text(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{n:.0}")
    } else {
        format!("{n}")
    }
}

impl WorkbookRefs {
    /// Build the layout for a set of tables and scalar paths.
    ///
    /// The order given is the order written: a table's columns get letters A,
    /// B, C... in the order listed (a repeated name keeps its first letter),
    /// and each scalar sheet gets rows 2, 3, 4... in the order its paths
    /// appear. The exporter passes model order and writes in the same order.
    #[must_use]
    pub fn new<'a>(
        tables: impl IntoIterator<Item = (&'a str, Vec<String>, usize)>,
        scalar_paths: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let mut refs = Self::default();
        for (table, columns, row_count) in tables {
            refs.add_table(table, columns, row_count);
        }
        let mut groups: HashMap<String, Vec<&str>> = HashMap::new();
        for path in scalar_paths {
            let sheet = split_scalar_path(path).0.unwrap_or(UNGROUPED_SHEET);
            groups.entry(sheet.to_string()).or_default().push(path);
        }
        for (sheet, paths) in groups {
            for (row, path) in (2u32..=u32::MAX).zip(paths) {
                refs.scalars.insert(path.to_string(), (sheet.clone(), row));
            }
        }
        refs
    }

    /// Make the scalars of an included file readable as `@{namespace}.{path}`.
    ///
    /// They live on the sheet `{namespace}.Scalars`, rows 2, 3, 4... in the
    /// order given, the layout `ExcelExporter` writes for an include.
    pub fn add_include<'a>(&mut self, namespace: &str, paths: impl IntoIterator<Item = &'a str>) {
        let sheet = include_sheet(namespace);
        for (row, path) in (2u32..=u32::MAX).zip(paths) {
            self.scalars
                .insert(format!("@{namespace}.{path}"), (sheet.clone(), row));
        }
    }

    /// Make the tables of an included file readable as
    /// `@{namespace}.{table}.{column}`, on the sheets `{namespace}.{table}`
    /// that `ExcelExporter` writes for them.
    pub fn add_include_tables(
        &mut self,
        namespace: &str,
        tables: impl IntoIterator<Item = (String, Vec<String>, usize)>,
    ) {
        for (table, columns, row_count) in tables {
            let key = format!("@{namespace}.{table}");
            self.add_table(&key, columns, row_count);
            self.sheets.insert(key, format!("{namespace}.{table}"));
        }
    }

    /// Lay out one table: columns A, B, C... in the order given (a repeated
    /// name keeps its first letter).
    fn add_table(&mut self, table: &str, columns: Vec<String>, row_count: usize) {
        let mut seen = std::collections::HashSet::new();
        let map = columns
            .into_iter()
            .filter(|name| seen.insert(name.clone()))
            .enumerate()
            .map(|(i, name)| (name, column_letter(i)))
            .collect();
        self.columns.insert(table.to_string(), map);
        self.rows.insert(table.to_string(), row_count);
    }

    /// Record which columns of a table hold text and which numbers, under
    /// `{table}.{column}` (`table` is `@ns.t` for a table of an include).
    pub fn add_column_types<'a>(
        &mut self,
        table: &str,
        columns: impl IntoIterator<Item = (&'a str, bool)>,
    ) {
        for (column, is_text) in columns {
            self.text_columns
                .insert(format!("{table}.{column}"), is_text);
        }
    }

    /// Record the columns of a table that are written as dates (`table` is
    /// `@ns.t` for a table of an include). To `COUNTUNIQUE` they are numbers.
    pub fn add_date_columns<'a>(
        &mut self,
        table: &str,
        columns: impl IntoIterator<Item = &'a str>,
    ) {
        for column in columns {
            self.date_columns.insert(format!("{table}.{column}"));
            self.text_columns.insert(format!("{table}.{column}"), false);
        }
    }

    /// Record the scalars whose value is a date (`@ns.path` for an include's).
    pub fn add_date_scalars<'a>(&mut self, paths: impl IntoIterator<Item = &'a str>) {
        self.date_scalars
            .extend(paths.into_iter().map(str::to_string));
    }

    /// Record the columns of a table that stay text in the workbook and hold
    /// a date in some row (`table` is `@ns.t` for a table of an include).
    pub fn add_mixed_columns<'a>(
        &mut self,
        table: &str,
        columns: impl IntoIterator<Item = &'a str>,
    ) {
        for column in columns {
            self.mixed_columns.insert(format!("{table}.{column}"));
        }
    }

    /// Record the length of each column of a table that is shorter than the
    /// table (`table` is `@ns.t` for a table of an include). An empty column
    /// is left out: a formula column of a model not calculated has no values
    /// yet.
    pub fn add_column_lengths<'a>(
        &mut self,
        table: &str,
        lengths: impl IntoIterator<Item = (&'a str, usize)>,
    ) {
        let rows = self.rows.get(table).copied().unwrap_or(0);
        for (column, length) in lengths {
            let letter = self.columns.get(table).and_then(|c| c.get(column)).cloned();
            if let (Some(letter), true) = (letter, length > 0 && length < rows) {
                self.column_rows
                    .entry(table.to_string())
                    .or_default()
                    .insert(letter, length);
            }
        }
    }

    /// The sheet a table is on.
    fn sheet_of<'s>(&'s self, table: &'s str) -> &'s str {
        self.sheets.get(table).map_or(table, String::as_str)
    }

    /// The layout for an included file's own formulas: its scalars by their
    /// own paths, on its `{namespace}.Scalars` sheet, and its tables by their
    /// own names, on the sheets `{namespace}.{table}`.
    #[must_use]
    pub fn for_include<'a>(
        namespace: &str,
        paths: impl IntoIterator<Item = &'a str>,
        tables: impl IntoIterator<Item = (String, Vec<String>, usize)>,
    ) -> Self {
        let sheet = include_sheet(namespace);
        let mut refs = Self::default();
        for (table, columns, row_count) in tables {
            refs.add_table(&table, columns, row_count);
            refs.sheets
                .insert(table.clone(), format!("{namespace}.{table}"));
        }
        for (row, path) in (2u32..=u32::MAX).zip(paths) {
            refs.scalars.insert(path.to_string(), (sheet.clone(), row));
        }
        refs
    }

    /// Resolve a scalar name the way the calculator does: the formula's own
    /// group first, then the exact name, then a unique last segment.
    fn resolve_scalar(&self, word: &str, group: Option<&str>) -> Result<Option<String>, String> {
        if let Some(group) = group {
            if !word.contains('.') {
                let scoped = format!("{group}.{word}");
                if self.scalars.contains_key(&scoped) {
                    return Ok(Some(scoped));
                }
            }
        }
        if self.scalars.contains_key(word) {
            return Ok(Some(word.to_string()));
        }
        if word.contains('.') {
            return Ok(None);
        }
        // A value from an included file is only ever named in full (`@ns.path`)
        let mut matches: Vec<&String> = self
            .scalars
            .keys()
            .filter(|p| !p.starts_with('@'))
            .filter(|p| p.rsplit_once('.').is_some_and(|(_, last)| last == word))
            .collect();
        matches.sort();
        match matches.len() {
            0 => Ok(None),
            1 => Ok(Some(matches[0].clone())),
            _ => Err(format!("name '{word}' is ambiguous")),
        }
    }

    /// Absolute single-cell reference to a scalar's value.
    fn scalar_cell(&self, path: &str, current_sheet: Option<&str>) -> String {
        let (sheet, row) = &self.scalars[path];
        let prefix = if current_sheet == Some(sheet.as_str()) {
            String::new()
        } else {
            sheet_prefix(sheet)
        };
        format!("{prefix}$B${row}")
    }

    /// The whole data range of a table, on its sheet: `'t'!$A$2:$C$n`.
    fn table_range(&self, table: &str) -> String {
        let width = self.columns.get(table).map_or(1, HashMap::len).max(1);
        let last = self.rows.get(table).copied().unwrap_or(0) + 1;
        format!(
            "{}$A$2:${}${last}",
            sheet_prefix(self.sheet_of(table)),
            column_letter(width - 1)
        )
    }

    /// `$L$2:$L$n` for a table column, with an optional sheet prefix: `n` is
    /// the column's own last row.
    fn column_range(&self, prefix: &str, table: &str, letter: &str) -> String {
        let rows = self.column_rows.get(table).and_then(|c| c.get(letter));
        let last = rows.or_else(|| self.rows.get(table)).copied().unwrap_or(0) + 1;
        format!("{prefix}${letter}$2:${letter}${last}")
    }

    /// Resolve one identifier to an Excel reference.
    fn reference(
        &self,
        word: &str,
        ctx: Context<'_>,
        whole_column: bool,
        has_range: &mut bool,
    ) -> Result<String, String> {
        // A column of a table of an included file: `@costs.prices.flour`
        if word.starts_with('@') && !self.scalars.contains_key(word) {
            if let Some((table, column)) = word.rsplit_once('.') {
                if let Some(letter) = self.columns.get(table).and_then(|c| c.get(column)) {
                    let prefix = sheet_prefix(self.sheet_of(table));
                    if let Context::Row { row, .. } = ctx {
                        if !whole_column {
                            return Ok(format!("{prefix}{letter}{row}"));
                        }
                    }
                    *has_range = true;
                    return Ok(self.column_range(&prefix, table, letter));
                }
            }
        }
        match ctx {
            Context::Row { table, row } => {
                // A column of this table, then a scalar, then `table.column`.
                if let Some(letter) = self.columns.get(table).and_then(|c| c.get(word)) {
                    if whole_column {
                        *has_range = true;
                        return Ok(self.column_range("", table, letter));
                    }
                    return Ok(format!("{letter}{row}"));
                }
                if let Some(path) = self.resolve_scalar(word, Some(table))? {
                    return Ok(self.scalar_cell(&path, None));
                }
                if let Some((other, column)) = word.split_once('.') {
                    if let Some(letter) = self.columns.get(other).and_then(|c| c.get(column)) {
                        // Row formulas work row by row, so another table's column
                        // is the cell in the same row (or the whole column where
                        // the function reads one).
                        let prefix = if other == table {
                            String::new()
                        } else {
                            sheet_prefix(self.sheet_of(other))
                        };
                        if whole_column {
                            *has_range = true;
                            return Ok(self.column_range(&prefix, other, letter));
                        }
                        return Ok(format!("{prefix}{letter}{row}"));
                    }
                }
            },
            Context::Scalar { group } => {
                let sheet = group.unwrap_or(UNGROUPED_SHEET);
                if let Some(path) = self.resolve_scalar(word, group)? {
                    return Ok(self.scalar_cell(&path, Some(sheet)));
                }
                if let Some((table, column)) = word.split_once('.') {
                    if let Some(letter) = self.columns.get(table).and_then(|c| c.get(column)) {
                        *has_range = true;
                        return Ok(self.column_range(
                            &sheet_prefix(self.sheet_of(table)),
                            table,
                            letter,
                        ));
                    }
                }
            },
        }
        Err(format!("unknown name '{word}'"))
    }

    /// `VLOOKUP` or `HLOOKUP` called at `tokens[at]` means in Excel what it
    /// means in Forge only where Excel's table is one column (`VLOOKUP`) or
    /// one row (`HLOOKUP`) too, since Forge reads every table that way
    /// (ADR-064): a table column, a range of one column in the workbook, or a
    /// column constant `{1;2;3}` for `VLOOKUP`; a row constant `{1,2,3}` for
    /// `HLOOKUP`. The index and both searches are then Excel's. Anything else
    /// (an `HLOOKUP` over a column, which Excel reads as a table of one column
    /// and many rows; a computed table) is an error, and the formula's values
    /// are written.
    fn one_line_lookup(
        &self,
        upper: &str,
        tokens: &[Token],
        at: usize,
        ctx: Context<'_>,
    ) -> Result<(), String> {
        let vertical = upper == "VLOOKUP";
        let table = matching_paren(tokens, at + 1)
            .map(|close| split_arguments(&tokens[at + 2..close]))
            .and_then(|args| args.get(1).copied());
        let across = if vertical {
            Token::Comma
        } else {
            Token::Semicolon
        };
        let one_line = match table {
            Some([Token::Identifier(word)]) if vertical => {
                let mut range = false;
                self.reference(word, ctx, true, &mut range).is_ok() && range
            },
            Some([Token::OpenBrace, items @ .., Token::CloseBrace]) => !items.contains(&across),
            _ => false,
        };
        if one_line {
            return Ok(());
        }
        Err(if vertical {
            "VLOOKUP's table in Excel is one column only when it is a table column or a \
             column constant {1;2;3}"
                .to_string()
        } else {
            "HLOOKUP's table in Excel is one row only when it is a row constant {1,2,3}: \
             a table column is a column of the sheet"
                .to_string()
        })
    }

    /// Translate a Forge formula to Excel.
    ///
    /// # Errors
    ///
    /// Returns the reason when the formula cannot be written as an Excel formula
    /// with the same meaning.
    pub fn to_excel(&self, formula: &str, ctx: Context<'_>) -> Result<Translated, String> {
        self.to_excel_as(formula, ctx, dates::Holds::Other)
    }

    /// Translate a Forge formula to Excel, given what its column holds in
    /// Forge: the cells of a column of dates must hold dates in the workbook
    /// too.
    ///
    /// # Errors
    ///
    /// Returns the reason when the formula cannot be written as an Excel formula
    /// with the same meaning.
    pub fn to_excel_as(
        &self,
        formula: &str,
        ctx: Context<'_>,
        holds: dates::Holds,
    ) -> Result<Translated, String> {
        let body = formula.trim().trim_start_matches('=');
        let tokens = tokenize(body).map_err(|e| e.to_string())?;
        check_irr_arguments(&tokens, ctx, self)?;
        // Dates are written the way Forge reads them in each place (`dates`)
        let dated = self.write_dates(&tokens, ctx, holds)?;
        let is_date = dated.as_ref().is_some_and(|d| d.is_date);
        let tokens = dated.map_or(tokens, |d| d.tokens);

        let mut has_range = false;
        // A LET or LAMBDA name never takes the name of a value of the model
        // that the formula reads: `forge import` would read that value back
        // as the local name.
        let model_names = tokens
            .iter()
            .enumerate()
            .filter_map(|(i, token)| match token {
                Token::Identifier(word)
                    if !matches!(tokens.get(i + 1), Some(Token::OpenParen))
                        && self.reference(word, ctx, false, &mut false).is_ok() =>
                {
                    Some(word.as_str())
                },
                _ => None,
            });
        let locals = Locals::avoiding(model_names);
        let out = self.translate(&tokens, ctx, &[], &locals, &mut has_range)?;
        Ok(Translated {
            formula: format!("={out}"),
            has_range,
            is_date,
        })
    }

    /// Translate a run of tokens; `outer` are the calls it sits inside.
    fn translate(
        &self,
        tokens: &[Token],
        ctx: Context<'_>,
        outer: &[Call],
        locals: &Locals,
        has_range: &mut bool,
    ) -> Result<String, String> {
        let mut out = String::new();
        let mut calls: Vec<Call> = outer.to_vec();
        let mut pending_function: Option<String> = None;
        // Whether the previous token ends a value, so `-` after it is binary.
        let mut after_value = false;
        let mut i = 0;
        while i < tokens.len() {
            let token = &tokens[i];
            let next_is_paren = matches!(tokens.get(i + 1), Some(Token::OpenParen));
            if let Some((text, end)) =
                self.translate_whole(tokens, i, ctx, &calls, locals, has_range)?
            {
                out.push_str(&text);
                after_value = true;
                i = end;
                continue;
            }
            match token {
                // An array constant (ADR-064), written as Excel writes it
                Token::OpenBrace => {
                    let (text, end) = constants::to_excel(tokens, i)?;
                    out.push_str(&text);
                    after_value = true;
                    i = end;
                    continue;
                },
                Token::CloseBrace | Token::Semicolon => {
                    return Err("{ } or ; outside an array constant".to_string());
                },
                Token::Number(n) => out.push_str(&number_text(*n)),
                Token::Text(t) => {
                    let _ = write!(out, "\"{}\"", t.replace('"', "\"\""));
                },
                Token::Boolean(b) => out.push_str(if *b { "TRUE" } else { "FALSE" }),
                Token::Identifier(word) if next_is_paren && locals.get(word).is_some() => {
                    return Err(format!(
                        "{word} is a name of a LET or LAMBDA, not a function"
                    ));
                },
                // A name of a LET or LAMBDA: `_xlpm.name`, as Excel stores it
                Token::Identifier(word) if locals.get(word).is_some() => {
                    let _ = write!(out, "_xlpm.{}", locals.get(word).unwrap_or(word));
                },
                Token::Identifier(word) if next_is_paren => {
                    let upper = self.call_head(word, tokens, i, ctx)?;
                    out.push_str(excel_name(&upper));
                    pending_function = Some(upper);
                },
                // `COLUMNS(table)`: Forge counts the table's columns
                Token::Identifier(word)
                    if self.columns.contains_key(word.as_str())
                        && matches!(calls.last(), Some((Some(f), 0, _)) if f == "COLUMNS") =>
                {
                    *has_range = true;
                    out.push_str(&self.table_range(word));
                },
                Token::Identifier(word) => {
                    let whole_column = Self::in_whole_column_call(&calls);
                    out.push_str(&self.reference(word, ctx, whole_column, has_range)?);
                },
                Token::Operator(op) => {
                    // `^` goes out unspaced: Gnumeric cannot read `2 ^ 10` in an
                    // imported workbook, while Excel accepts both forms.
                    if after_value && op != "^" {
                        let _ = write!(out, " {op} ");
                    } else {
                        out.push_str(op);
                    }
                },
                Token::OpenParen => {
                    let function = pending_function.take();
                    out.push('(');
                    if function.as_deref() == Some("E") {
                        out.push('1'); // EXP(1)
                    }
                    let count = matching_paren(tokens, i)
                        .map_or(0, |close| split_arguments(&tokens[i + 1..close]).len());
                    calls.push((function, 0, count));
                },
                Token::CloseParen => {
                    calls.pop();
                    out.push(')');
                },
                Token::Comma => {
                    let Some(call) = calls.last_mut() else {
                        return Err("a comma outside a call".to_string());
                    };
                    call.1 += 1;
                    out.push_str(", ");
                },
                // Forge reads `a:b` as the text of its two ends; Excel would
                // read a range
                Token::Colon => {
                    return Err("a range a:b has no Excel equivalent".to_string());
                },
                Token::OpenBracket | Token::CloseBracket => {
                    return Err("array indexing [ ] has no Excel equivalent".to_string());
                },
            }
            after_value = match token {
                Token::Number(_) | Token::Text(_) | Token::Boolean(_) | Token::CloseParen => true,
                Token::Identifier(_) => !next_is_paren,
                _ => false,
            };
            i += 1;
        }
        Ok(out)
    }

    /// A construct that is written out as a whole: one of Forge's own
    /// functions, `LET` and `LAMBDA` called at once, `COUNTUNIQUE`, `PREV` or
    /// `ROW()`. Its Excel text, and the index of the token after it; `None`
    /// when the token starts none of them.
    fn translate_whole(
        &self,
        tokens: &[Token],
        i: usize,
        ctx: Context<'_>,
        calls: &[Call],
        locals: &Locals,
        has_range: &mut bool,
    ) -> Result<Option<(String, usize)>, String> {
        let next_is_paren = matches!(tokens.get(i + 1), Some(Token::OpenParen));
        match &tokens[i] {
            // Forge's own functions, written out as the Excel expression
            // that computes the same thing
            Token::Identifier(word)
                if next_is_paren && FORGE_FUNCTIONS.contains(&word.to_uppercase().as_str()) =>
            {
                let close =
                    matching_paren(tokens, i + 1).ok_or_else(|| format!("{word}: missing )"))?;
                let args = split_arguments(&tokens[i + 2..close])
                    .into_iter()
                    .map(|arg| {
                        if arg.is_empty() {
                            return Err(format!("{word}: an argument is empty"));
                        }
                        self.translate(arg, ctx, calls, locals, has_range)
                    })
                    .collect::<Result<Vec<String>, String>>()?;
                let third = split_arguments(&tokens[i + 2..close])
                    .get(2)
                    .map(|t| t.to_vec());
                Ok(Some((
                    forge_function(word, &args, third.as_deref())?,
                    close + 1,
                )))
            },
            // LET, LAMBDA called at once and COUNTUNIQUE: written as the
            // Excel expression that means the same (`local_names`)
            Token::Identifier(word)
                if next_is_paren
                    && local_names::HANDLED.contains(&word.to_uppercase().as_str()) =>
            {
                let (text, end) =
                    self.translate_special(tokens, (word, i), ctx, calls, locals, has_range)?;
                Ok(Some((text, end)))
            },
            // PREV and ROW(): the row above, and the table's row number
            Token::Identifier(word)
                if next_is_paren
                    && (word.eq_ignore_ascii_case("PREV") || word.eq_ignore_ascii_case("ROW")) =>
            {
                let (text, end) = if word.eq_ignore_ascii_case("PREV") {
                    self.translate_prev(tokens, i, ctx, calls, locals, has_range)?
                } else {
                    Self::translate_row(tokens, i, ctx, locals)?
                };
                Ok(Some((text, end)))
            },
            _ => Ok(None),
        }
    }

    /// The name of a function being called, upper case, once it is known to
    /// have an Excel equivalent.
    fn call_head(
        &self,
        word: &str,
        tokens: &[Token],
        i: usize,
        ctx: Context<'_>,
    ) -> Result<String, String> {
        let upper = word.to_uppercase();
        if upper == "VLOOKUP" || upper == "HLOOKUP" {
            self.one_line_lookup(&upper, tokens, i, ctx)?;
        } else if !super::FormulaTranslator::is_excel_function(word)
            || DIFFERENT_IN_EXCEL.contains(&upper.as_str())
        {
            return Err(format!("function {word} has no Excel equivalent"));
        }
        if upper == "E" && !matches!(tokens.get(i + 2), Some(Token::CloseParen)) {
            return Err("E takes no arguments".to_string());
        }
        Ok(upper)
    }

    /// Whether a name here is a range argument (ADR-058), or inside one.
    fn in_whole_column_call(calls: &[Call]) -> bool {
        calls.iter().any(|(name, arg, count)| {
            name.as_deref().is_some_and(|n| {
                crate::core::array_calculator::whole_column::is_whole_column(n, *count, *arg)
            })
        })
    }
}

/// Forge's own functions that are written to Excel as an expression.
const FORGE_FUNCTIONS: [&str; 5] = [
    "VARIANCE",
    "VARIANCE_PCT",
    "VARIANCE_STATUS",
    "BREAKEVEN_UNITS",
    "BREAKEVEN_REVENUE",
];

/// Index of the `)` that closes the `(` at `open`.
fn matching_paren(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (i, token) in tokens.iter().enumerate().skip(open) {
        match token {
            Token::OpenParen => depth += 1,
            Token::CloseParen => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            },
            _ => {},
        }
    }
    None
}

/// The arguments of a call: `tokens` between its parentheses, split at the
/// commas that are not inside another call.
fn split_arguments(tokens: &[Token]) -> Vec<&[Token]> {
    if tokens.is_empty() {
        return Vec::new();
    }
    let mut args = Vec::new();
    let (mut depth, mut start) = (0usize, 0);
    for (i, token) in tokens.iter().enumerate() {
        match token {
            Token::OpenParen | Token::OpenBracket | Token::OpenBrace => depth += 1,
            Token::CloseParen | Token::CloseBracket | Token::CloseBrace => {
                depth = depth.saturating_sub(1);
            },
            Token::Comma if depth == 0 => {
                args.push(&tokens[start..i]);
                start = i + 1;
            },
            _ => {},
        }
    }
    args.push(&tokens[start..]);
    args
}

/// The Excel expression for one of [`FORGE_FUNCTIONS`], from its translated
/// arguments. Each computes what Forge computes; where Forge returns an error
/// (a margin of zero or less), Excel returns #N/A. `third` is the third
/// argument's tokens, for `VARIANCE_STATUS`, whose meaning depends on its type.
fn forge_function(name: &str, args: &[String], third: Option<&[Token]>) -> Result<String, String> {
    let upper = name.to_uppercase();
    let want = |n: usize| {
        if args.len() == n {
            Ok(())
        } else {
            Err(format!("{name} takes {n} arguments, not {}", args.len()))
        }
    };
    let a = |k: usize| format!("({})", args[k]);
    Ok(match upper.as_str() {
        "VARIANCE" => {
            want(2)?;
            format!("({} - {})", a(0), a(1))
        },
        "VARIANCE_PCT" => {
            want(2)?;
            format!("(({} - {}) / ABS{})", a(0), a(1), a(1))
        },
        "BREAKEVEN_UNITS" => {
            want(3)?;
            let margin = format!("({} - {})", a(1), a(2));
            format!("IF({margin} <= 0, NA(), {} / {margin})", a(0))
        },
        "BREAKEVEN_REVENUE" => {
            want(2)?;
            format!("IF(OR({r} <= 0, {r} > 1), NA(), {} / {r})", a(0), r = a(1))
        },
        // VARIANCE_STATUS, the last of FORGE_FUNCTIONS: it is only ever
        // called with one of them
        _ => {
            if !(2..=3).contains(&args.len()) {
                return Err(format!("{name} takes 2 or 3 arguments, not {}", args.len()));
            }
            // A number is the threshold and the text "cost" flips the sign;
            // only a literal says which it is
            let (threshold, is_cost) = match third {
                None => ("0.01".to_string(), false),
                Some([Token::Text(t)]) => ("0.01".to_string(), t.eq_ignore_ascii_case("cost")),
                Some([Token::Number(_)] | [Token::Operator(_), Token::Number(_)]) => {
                    (a(2), false)
                },
                Some(_) => {
                    return Err(format!(
                        "{name}: the third argument must be a number or \"cost\" written in the formula"
                    ))
                },
            };
            let (better, worse) = if is_cost { ("-1", "1") } else { ("1", "-1") };
            let pct = format!("(({} - {}) / ABS{})", a(0), a(1), a(1));
            format!(
                "IF({b} = 0, SIGN{x}, IF(ABS({pct}) <= {threshold}, 0, IF({pct} > 0, {better}, {worse})))",
                b = a(1),
                x = a(0),
            )
        },
    })
}

/// Forge's `IRR(a, b)` treats `b` as more cash flows unless `a` is a column;
/// Excel always reads `b` as a guess. Only the forms that mean the same in both
/// are translated: `IRR(column)` and `IRR(column, guess)` in a scalar formula.
fn check_irr_arguments(
    tokens: &[Token],
    ctx: Context<'_>,
    refs: &WorkbookRefs,
) -> Result<(), String> {
    for (i, token) in tokens.iter().enumerate() {
        let Token::Identifier(name) = token else {
            continue;
        };
        if !name.eq_ignore_ascii_case("IRR") || !matches!(tokens.get(i + 1), Some(Token::OpenParen))
        {
            continue;
        }
        let Context::Scalar { group } = ctx else {
            return Err("IRR in a row formula has no Excel equivalent".to_string());
        };
        let first_is_column = match (tokens.get(i + 2), tokens.get(i + 3)) {
            (Some(Token::Identifier(arg)), Some(Token::Comma | Token::CloseParen)) => {
                refs.resolve_scalar(arg, group).ok().flatten().is_none()
                    && arg.split_once('.').is_some_and(|(t, c)| {
                        refs.columns.get(t).is_some_and(|cols| cols.contains_key(c))
                    })
            },
            _ => false,
        };
        if !first_is_column {
            return Err(
                "IRR with separate cash flows: Excel reads the second argument as a guess"
                    .to_string(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Columns and scalars are listed in the order the layout uses:
    /// revenue (A), `total_profit` (B), year (C); valuation.net (row 2), npv (row 3).
    fn refs() -> WorkbookRefs {
        WorkbookRefs::new(
            [
                (
                    "projections",
                    vec![
                        "revenue".to_string(),
                        "total_profit".to_string(),
                        "year".to_string(),
                    ],
                    3,
                ),
                ("other", vec!["x".to_string()], 3),
            ],
            [
                "assumptions.base_revenue",
                "assumptions.growth",
                "valuation.net",
                "valuation.npv",
                "tax",
            ],
        )
    }

    #[test]
    fn layout_keeps_the_order_given() {
        // Model order, not alphabetical: year (A), revenue (B), year again ignored;
        // b.second before b.first gives rows 2 and 3.
        let refs = WorkbookRefs::new(
            [(
                "t",
                vec![
                    "year".to_string(),
                    "revenue".to_string(),
                    "year".to_string(),
                ],
                3,
            )],
            ["b.second", "b.first"],
        );
        let columns = &refs.columns["t"];
        assert_eq!(columns["year"], "A");
        assert_eq!(columns["revenue"], "B");
        assert_eq!(columns.len(), 2);
        assert_eq!(refs.scalars["b.second"], ("b".to_string(), 2));
        assert_eq!(refs.scalars["b.first"], ("b".to_string(), 3));
    }

    fn row(f: &str) -> Result<String, String> {
        refs()
            .to_excel(
                f,
                Context::Row {
                    table: "projections",
                    row: 3,
                },
            )
            .map(|t| t.formula)
    }

    fn scalar(f: &str, group: Option<&str>) -> Result<Translated, String> {
        refs().to_excel(f, Context::Scalar { group })
    }

    #[test]
    fn column_letters() {
        assert_eq!(column_letter(0), "A");
        assert_eq!(column_letter(25), "Z");
        assert_eq!(column_letter(26), "AA");
        assert_eq!(column_letter(701), "ZZ");
        assert_eq!(column_letter(702), "AAA");
    }

    #[test]
    fn row_formula_columns_named_like_functions() {
        // `year` is a column here, not YEAR(); the fixture lists
        // revenue (A), total_profit (B), year (C).
        assert_eq!(
            row("=assumptions.base_revenue * (1 + assumptions.growth) ^ year").unwrap(),
            "='assumptions'!$B$2 * (1 + 'assumptions'!$B$3)^C3"
        );
    }

    #[test]
    fn row_formula_other_table_is_same_row() {
        assert_eq!(row("=revenue - other.x").unwrap(), "=A3 - 'other'!A3");
    }

    #[test]
    fn row_formula_bare_scalar_name() {
        assert_eq!(row("=revenue * tax").unwrap(), "=A3 * 'Scalars'!$B$2");
        assert_eq!(
            row("=revenue * growth").unwrap(),
            "=A3 * 'assumptions'!$B$3"
        );
    }

    #[test]
    fn unary_minus_and_literals() {
        assert_eq!(row("=-revenue + 1.5").unwrap(), "=-A3 + 1.5");
        assert_eq!(
            row("=IF(revenue > 0, \"up\", FALSE)").unwrap(),
            "=IF(A3 > 0, \"up\", FALSE)"
        );
    }

    #[test]
    fn scalar_formula_ranges_and_own_group() {
        let t = scalar(
            "=NPV(assumptions.growth, projections.total_profit)",
            Some("valuation"),
        )
        .unwrap();
        assert_eq!(
            t.formula,
            "=NPV('assumptions'!$B$3, 'projections'!$B$2:$B$4)"
        );
        assert!(t.has_range);

        // `npv` is in the formula's own group: same sheet, no prefix.
        let t = scalar("=npv - assumptions.base_revenue", Some("valuation")).unwrap();
        assert_eq!(t.formula, "=$B$3 - 'assumptions'!$B$2");
        assert!(!t.has_range);
    }

    #[test]
    fn irr_only_when_excel_means_the_same() {
        assert_eq!(
            scalar("=IRR(projections.total_profit)", Some("valuation"))
                .unwrap()
                .formula,
            "=IRR('projections'!$B$2:$B$4)"
        );
        assert!(scalar("=IRR(projections.total_profit, 0.1)", Some("valuation")).is_ok());
        let err = scalar(
            "=IRR(-assumptions.base_revenue, projections.total_profit)",
            Some("valuation"),
        )
        .unwrap_err();
        assert!(err.contains("guess"), "{err}");
    }

    /// A name `irr` that is not a call is a name like any other.
    #[test]
    fn a_name_irr_is_not_the_function() {
        let refs = WorkbookRefs::new([("t", vec!["irr".to_string()], 3)], ["v.irr"]);
        let row = Context::Row { table: "t", row: 2 };
        assert_eq!(refs.to_excel("=irr * 2", row).unwrap().formula, "=A2 * 2");
        let scalar = Context::Scalar { group: Some("v") };
        assert_eq!(
            refs.to_excel("=irr + 1", scalar).unwrap().formula,
            "=$B$2 + 1"
        );
    }

    #[test]
    fn untranslatable_formulas_say_why() {
        assert!(row("=MC.Normal(1, 2)")
            .unwrap_err()
            .contains("no Excel equivalent"));
        assert!(row("=revenue[0]").unwrap_err().contains("indexing"));
        assert!(row("=missing * 2").unwrap_err().contains("unknown name"));
    }

    #[test]
    fn a_range_or_a_comma_outside_a_call_is_not_written() {
        // Forge reads revenue:year as text; Excel would read a range
        assert_eq!(
            row("=SUM(revenue:year)"),
            Err("a range a:b has no Excel equivalent".to_string())
        );
        assert_eq!(row("=1, 2"), Err("a comma outside a call".to_string()));
    }

    #[test]
    fn braces_semicolons_and_local_names_out_of_place_are_refused() {
        assert_eq!(row("={1, 2"), Err("array constant without }".to_string()));
        for f in ["=1; 2", "=1 }"] {
            assert_eq!(
                row(f),
                Err("{ } or ; outside an array constant".to_string()),
                "{f}"
            );
        }
        assert_eq!(
            row("=LET(f, 2, f(1))"),
            Err("f is a name of a LET or LAMBDA, not a function".to_string())
        );
    }

    #[test]
    fn a_name_two_groups_end_with_is_ambiguous() {
        let mut refs = refs();
        refs.scalars.insert("a.rate".into(), ("a".into(), 2));
        refs.scalars.insert("b.rate".into(), ("b".into(), 2));
        let ambiguous = "name 'rate' is ambiguous";
        let in_row = Context::Row {
            table: "projections",
            row: 3,
        };
        assert_eq!(refs.to_excel("=rate", in_row).unwrap_err(), ambiguous);
        let in_scalar = Context::Scalar { group: None };
        assert_eq!(refs.to_excel("=rate", in_scalar).unwrap_err(), ambiguous);
    }

    #[test]
    fn an_included_column_in_a_row_formula() {
        let mut refs = refs();
        refs.add_include_tables("inc", [("t".to_string(), vec!["c".to_string()], 2)]);
        let in_row = Context::Row {
            table: "projections",
            row: 3,
        };
        // The cell in the same row, or the whole column where SUM reads one
        assert_eq!(
            refs.to_excel("=@inc.t.c", in_row).map(|t| t.formula),
            Ok("='inc.t'!A3".to_string())
        );
        let whole = refs.to_excel("=SUM(@inc.t.c)", in_row).unwrap();
        assert_eq!(whole.formula, "=SUM('inc.t'!$A$2:$A$3)");
        assert!(whole.has_range);
    }

    #[test]
    fn forge_functions_with_the_wrong_arguments_are_refused() {
        for (f, error) in [
            ("=VARIANCE(revenue)", "VARIANCE takes 2 arguments, not 1"),
            (
                "=VARIANCE_PCT(revenue)",
                "VARIANCE_PCT takes 2 arguments, not 1",
            ),
            (
                "=BREAKEVEN_UNITS(revenue, 1)",
                "BREAKEVEN_UNITS takes 3 arguments, not 2",
            ),
            (
                "=BREAKEVEN_REVENUE(revenue)",
                "BREAKEVEN_REVENUE takes 2 arguments, not 1",
            ),
            (
                "=VARIANCE_STATUS(revenue)",
                "VARIANCE_STATUS takes 2 or 3 arguments, not 1",
            ),
            ("=VARIANCE(revenue, 2", "VARIANCE: missing )"),
            ("=VARIANCE(revenue, )", "VARIANCE: an argument is empty"),
            ("=VARIANCE(nothing, 1)", "unknown name 'nothing'"),
        ] {
            assert_eq!(row(f), Err(error.to_string()), "{f}");
        }
    }

    #[test]
    fn variance_status_without_a_threshold_uses_one_percent() {
        assert_eq!(
            row("=VARIANCE_STATUS(revenue, 2)").unwrap(),
            "=IF((2) = 0, SIGN(A3), IF(ABS((((A3) - (2)) / ABS(2))) <= 0.01, 0, \
             IF((((A3) - (2)) / ABS(2)) > 0, 1, -1)))"
        );
    }

    #[test]
    fn irr_in_a_row_formula_is_refused() {
        assert_eq!(
            row("=IRR(revenue)"),
            Err("IRR in a row formula has no Excel equivalent".to_string())
        );
    }

    #[test]
    fn lookup_arguments_are_whole_columns_in_row_formulas() {
        // MATCH reads its second argument as the whole column; the first is per row.
        assert_eq!(
            row("=MATCH(revenue, other.x, 0)").unwrap(),
            "=MATCH(A3, 'other'!$A$2:$A$4, 0)"
        );
        assert_eq!(
            row("=INDEX(total_profit, 2) + year").unwrap(),
            "=INDEX($B$2:$B$4, 2) + C3"
        );
    }

    /// VLOOKUP and HLOOKUP are written where Excel's table is one column or
    /// one row too, and only there
    #[test]
    fn lookups_on_one_column_or_row_are_written() {
        assert_eq!(
            row("=VLOOKUP(revenue, other.x, 1, FALSE)").unwrap(),
            "=VLOOKUP(A3, 'other'!$A$2:$A$4, 1, FALSE)"
        );
        assert_eq!(
            row("=VLOOKUP(year, revenue, 1)").unwrap(),
            "=VLOOKUP(C3, $A$2:$A$4, 1)"
        );
        assert_eq!(
            scalar("=VLOOKUP(2, other.x, 1, TRUE)", Some("valuation"))
                .unwrap()
                .formula,
            "=VLOOKUP(2, 'other'!$A$2:$A$4, 1, TRUE)"
        );
        assert_eq!(
            row("=VLOOKUP(year, {1;2;3}, 1)").unwrap(),
            "=VLOOKUP(C3, {1;2;3}, 1)"
        );
        assert_eq!(
            row("=HLOOKUP(year, {1,2,3}, 1, FALSE)").unwrap(),
            "=HLOOKUP(C3, {1,2,3}, 1, FALSE)"
        );
        // A one-item constant is one row and one column
        assert_eq!(row("=HLOOKUP(1, {7}, 1)").unwrap(), "=HLOOKUP(1, {7}, 1)");
        for (f, why) in [
            // A table column is a column of the sheet: Excel's HLOOKUP would
            // search its first cell only
            ("=HLOOKUP(1, revenue, 1)", "HLOOKUP's table"),
            ("=HLOOKUP(1, other.x, 1)", "HLOOKUP's table"),
            ("=HLOOKUP(1, {1;2;3}, 1)", "HLOOKUP's table"),
            // A row constant is one row of three columns in Excel
            ("=VLOOKUP(1, {1,2,3}, 1)", "VLOOKUP's table"),
            // A scalar is one cell, and a computed table no range
            ("=VLOOKUP(1, tax, 1)", "VLOOKUP's table"),
            ("=VLOOKUP(1, other.x * 2, 1)", "VLOOKUP's table"),
            ("=VLOOKUP(1)", "VLOOKUP's table"),
        ] {
            assert!(row(f).unwrap_err().contains(why), "{f}");
        }
    }

    #[test]
    fn functions_that_differ_in_excel_are_not_translated() {
        for f in ["=NOW()", "=ROW(revenue)", "=COLUMN()"] {
            assert!(row(f).unwrap_err().contains("no Excel equivalent"), "{f}");
        }
        // COUNTUNIQUE has an Excel form only in a scalar (local_names)
        assert!(row("=COUNTUNIQUE(revenue)")
            .unwrap_err()
            .contains("row formula"));
    }

    /// Forge names Excel spells otherwise, functions Excel stores with a
    /// prefix, and `COLUMNS(table)`.
    #[test]
    fn functions_with_other_names_in_excel() {
        for (forge, excel) in [
            ("=POW(revenue, 2)", "=POWER(A3, 2)"),
            ("=AVG(revenue, 1)", "=AVERAGE(A3, 1)"),
            ("=ROUND(E() * revenue, 2)", "=ROUND(EXP(1) * A3, 2)"),
            ("=LN(E())", "=LN(EXP(1))"),
        ] {
            assert_eq!(row(forge).unwrap(), excel, "{forge}");
        }
        assert!(row("=E(1)").unwrap_err().contains("no arguments"));
        for (forge, excel) in [
            (
                "=STDEV.S(projections.revenue)",
                "=STDEV.S('projections'!$A$2:$A$4)",
            ),
            (
                "=RANK.EQ(assumptions.growth, projections.revenue, 1)",
                "=RANK.EQ('assumptions'!$B$3, 'projections'!$A$2:$A$4, 1)",
            ),
            ("=COLUMNS(projections)", "=COLUMNS('projections'!$A$2:$C$4)"),
            ("=RANDBETWEEN(1, 6)", "=RANDBETWEEN(1, 6)"),
        ] {
            assert_eq!(
                scalar(forge, Some("valuation")).unwrap().formula,
                excel,
                "{forge}"
            );
        }
    }

    /// Forge's own functions become the Excel expression that computes the
    /// same (forge-e2e-gnumeric recalculates each one in Gnumeric).
    #[test]
    fn forge_functions_as_excel_expressions() {
        for (forge, excel) in [
            ("=VARIANCE(revenue, 100)", "=((A3) - (100))"),
            ("=VARIANCE_PCT(revenue, year)", "=(((A3) - (C3)) / ABS(C3))"),
            (
                "=BREAKEVEN_UNITS(1000, revenue, 20)",
                "=IF(((A3) - (20)) <= 0, NA(), (1000) / ((A3) - (20)))",
            ),
            (
                "=BREAKEVEN_REVENUE(1000, 0.4)",
                "=IF(OR((0.4) <= 0, (0.4) > 1), NA(), (1000) / (0.4))",
            ),
            (
                "=VARIANCE_STATUS(revenue, 100, \"cost\")",
                "=IF((100) = 0, SIGN(A3), IF(ABS((((A3) - (100)) / ABS(100))) <= 0.01, 0, IF((((A3) - (100)) / ABS(100)) > 0, -1, 1)))",
            ),
            (
                "=VARIANCE_STATUS(revenue, 100, 0.05)",
                "=IF((100) = 0, SIGN(A3), IF(ABS((((A3) - (100)) / ABS(100))) <= (0.05), 0, IF((((A3) - (100)) / ABS(100)) > 0, 1, -1)))",
            ),
            (
                "=VARIANCE_STATUS(revenue, 100, -0.05)",
                "=IF((100) = 0, SIGN(A3), IF(ABS((((A3) - (100)) / ABS(100))) <= (-0.05), 0, IF((((A3) - (100)) / ABS(100)) > 0, 1, -1)))",
            ),
            (
                "=ROUND(VARIANCE(MAX(revenue, 1), 2) * 3, 0)",
                "=ROUND(((MAX(A3, 1)) - (2)) * 3, 0)",
            ),
        ] {
            assert_eq!(row(forge).unwrap(), excel, "{forge}");
        }
        assert!(row("=VARIANCE(revenue)").unwrap_err().contains("takes 2"));
        assert!(row("=VARIANCE_STATUS(revenue, 100, year)")
            .unwrap_err()
            .contains("number or"));
    }

    #[test]
    fn sheet_names_with_quotes() {
        assert_eq!(sheet_prefix("Bob's"), "'Bob''s'!");
    }
}
