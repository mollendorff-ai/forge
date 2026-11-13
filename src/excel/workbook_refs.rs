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

pub mod dates;
mod local_names;
mod previous_row;
use local_names::Locals;
use std::fmt::Write as _;

/// Sheet that holds scalars without a group.
pub const UNGROUPED_SHEET: &str = "Scalars";

/// Functions Excel evaluates differently from Forge: Forge's `VLOOKUP` takes
/// one column and ignores the column index, `ROW`/`COLUMN`/`OFFSET`/`INDIRECT`
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
        #[allow(clippy::cast_possible_truncation)] // index % 26 < 26
        letters.push((b'A' + (index % 26) as u8) as char);
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
            for (i, path) in paths.into_iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)] // far below u32::MAX scalars
                let row = (i + 2) as u32;
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
        for (i, path) in paths.into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // far below u32::MAX scalars
            let row = (i + 2) as u32;
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
        for (i, path) in paths.into_iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)] // far below u32::MAX scalars
            let row = (i + 2) as u32;
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
    #[allow(clippy::too_many_lines)] // one match over the token kinds
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
            match token {
                Token::Number(n) => out.push_str(&number_text(*n)),
                Token::Text(t) => {
                    let _ = write!(out, "\"{}\"", t.replace('"', "\"\""));
                },
                Token::Boolean(b) => out.push_str(if *b { "TRUE" } else { "FALSE" }),
                // Forge's own functions, written out as the Excel expression
                // that computes the same thing
                Token::Identifier(word)
                    if next_is_paren && FORGE_FUNCTIONS.contains(&word.to_uppercase().as_str()) =>
                {
                    let close = matching_paren(tokens, i + 1)
                        .ok_or_else(|| format!("{word}: missing )"))?;
                    let args = split_arguments(&tokens[i + 2..close])
                        .into_iter()
                        .map(|arg| {
                            if arg.is_empty() {
                                return Err(format!("{word}: an argument is empty"));
                            }
                            self.translate(arg, ctx, &calls, locals, has_range)
                        })
                        .collect::<Result<Vec<String>, String>>()?;
                    let third = split_arguments(&tokens[i + 2..close])
                        .get(2)
                        .map(|t| t.to_vec());
                    out.push_str(&forge_function(word, &args, third.as_deref())?);
                    after_value = true;
                    i = close + 1;
                    continue;
                },
                // LET, LAMBDA called at once and COUNTUNIQUE: written as the
                // Excel expression that means the same (`local_names`)
                Token::Identifier(word)
                    if next_is_paren
                        && local_names::HANDLED.contains(&word.to_uppercase().as_str()) =>
                {
                    let (text, end) =
                        self.translate_special(tokens, i, ctx, &calls, locals, has_range)?;
                    out.push_str(&text);
                    after_value = true;
                    i = end;
                    continue;
                },
                // PREV and ROW(): the row above, and the table's row number
                Token::Identifier(word)
                    if next_is_paren
                        && (word.eq_ignore_ascii_case("PREV")
                            || word.eq_ignore_ascii_case("ROW")) =>
                {
                    let (text, end) = if word.eq_ignore_ascii_case("PREV") {
                        self.translate_prev(tokens, i, ctx, &calls, locals, has_range)?
                    } else {
                        Self::translate_row(tokens, i, ctx, locals)?
                    };
                    out.push_str(&text);
                    after_value = true;
                    i = end;
                    continue;
                },
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
                    let upper = word.to_uppercase();
                    if !super::FormulaTranslator::is_excel_function(word)
                        || DIFFERENT_IN_EXCEL.contains(&upper.as_str())
                    {
                        return Err(format!("function {word} has no Excel equivalent"));
                    }
                    if upper == "E" && !matches!(tokens.get(i + 2), Some(Token::CloseParen)) {
                        return Err("E takes no arguments".to_string());
                    }
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
                    // A range argument (ADR-058), or inside one
                    let whole_column = calls.iter().any(|(name, arg, count)| {
                        name.as_deref().is_some_and(|n| {
                            crate::core::array_calculator::whole_column::is_whole_column(
                                n, *count, *arg,
                            )
                        })
                    });
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
                    if let Some(call) = calls.last_mut() {
                        call.1 += 1;
                    }
                    out.push_str(", ");
                },
                Token::Colon => out.push(':'),
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
}

/// Whether a formula calls `function` (upper case).
#[must_use]
pub fn calls_function(formula: &str, function: &str) -> bool {
    tokenize(formula.trim().trim_start_matches('=')).is_ok_and(|tokens| {
        tokens.windows(2).any(|w| {
            matches!(&w[0], Token::Identifier(name) if name.eq_ignore_ascii_case(function))
                && matches!(w[1], Token::OpenParen)
        })
    })
}

/// Forge's own functions that are written to Excel as an expression.
const FORGE_FUNCTIONS: [&str; 5] = [
    "VARIANCE",
    "VARIANCE_PCT",
    "VARIANCE_STATUS",
    "BREAKEVEN_UNITS",
    "BREAKEVEN_REVENUE",
];

/// Keep `calls` up to date at token `i` of an Excel formula: a `(` opens a
/// call (with its function and number of arguments), a `)` closes one, a `,`
/// moves to the next argument.
fn track_call(tokens: &[XlToken], i: usize, calls: &mut Vec<Call>) {
    match &tokens[i] {
        XlToken::Open => {
            let function = match i.checked_sub(1).and_then(|p| tokens.get(p)) {
                Some(XlToken::Function(name)) => Some(
                    name.trim_start_matches("_xlfn.")
                        .trim_start_matches("_xlws.")
                        .to_uppercase(),
                ),
                _ => None,
            };
            calls.push((function, 0, excel_argument_count(tokens, i)));
        },
        XlToken::Close => {
            calls.pop();
        },
        XlToken::Comma => {
            if let Some(call) = calls.last_mut() {
                call.1 += 1;
            }
        },
        _ => {},
    }
}

/// The number of arguments of the Excel call whose `(` is at `open`.
fn excel_argument_count(tokens: &[XlToken], open: usize) -> usize {
    // Depth inside the call's own parentheses
    let mut depth = 0usize;
    let mut count = 0;
    for token in tokens.iter().skip(open + 1) {
        match token {
            XlToken::Close if depth == 0 => return count,
            XlToken::Close => depth -= 1,
            XlToken::Comma if depth == 0 => count = count.max(1) + 1,
            XlToken::Open => {
                count = count.max(1);
                depth += 1;
            },
            _ => count = count.max(1),
        }
    }
    count
}

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
            Token::OpenParen | Token::OpenBracket => depth += 1,
            Token::CloseParen | Token::CloseBracket => depth = depth.saturating_sub(1),
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
        "VARIANCE_STATUS" => {
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
        _ => return Err(format!("function {name} has no Excel equivalent")),
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
#[path = "workbook_refs/forward_tests.rs"]
mod tests;

/// What a sheet holds, read back from a workbook.
#[derive(Debug, Clone)]
pub enum SheetKind {
    /// A table: column letter -> column name. `include` is the namespace when
    /// the sheet holds a table of an included file (`{namespace}.{table}`):
    /// `name` is then the file's own name for it, and the main model reads
    /// its columns as `@{namespace}.{name}.{column}`.
    Table {
        name: String,
        columns: HashMap<String, String>,
        include: Option<String>,
    },
    /// A scalar sheet (`Name` / `Value`): Excel row -> scalar path.
    ///
    /// `include` is the namespace when the sheet holds an included file's
    /// scalars (`{namespace}.Scalars`): its paths are the file's own, and the
    /// main model reads them as `@{namespace}.{path}`.
    Scalars {
        rows: HashMap<u32, String>,
        include: Option<String>,
    },
}

impl SheetKind {
    /// The namespace of an included file's sheet.
    pub(crate) fn include(&self) -> Option<&str> {
        match self {
            Self::Scalars { include, .. } | Self::Table { include, .. } => include.as_deref(),
        }
    }
}

/// Workbook layout for translating Excel formulas back to Forge.
#[derive(Debug, Clone, Default)]
pub struct ReverseRefs {
    /// Sheet name -> what it holds
    pub sheets: HashMap<String, SheetKind>,
}

/// One piece of an Excel formula.
#[derive(Debug, Clone, PartialEq)]
enum XlToken {
    Number(String),
    Text(String),
    Bool(bool),
    Function(String),
    /// A name of a LET (`_xlpm.x`, read as `x`)
    Local(String),
    /// Sheet (if given), first cell, last cell (for a range)
    Reference(Option<String>, Cell, Option<Cell>),
    Operator(String),
    Open,
    Close,
    Comma,
}

/// A cell address: column letters and 1-based row.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Cell {
    column: String,
    row: u32,
}

fn parse_cell(text: &str) -> Option<Cell> {
    let t = text.replace('$', "");
    let split = t.find(|c: char| c.is_ascii_digit())?;
    let (column, row) = t.split_at(split);
    if column.is_empty() || column.len() > 3 || !column.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    Some(Cell {
        column: column.to_string(),
        row: row.parse().ok()?,
    })
}

/// Read a name, sheet prefix (`name!`), function (`NAME(`), cell or range.
fn read_word(
    chars: &[char],
    i: &mut usize,
    sheet: &mut Option<String>,
    tokens: &mut Vec<XlToken>,
) -> Result<(), String> {
    let start = *i;
    while *i < chars.len() && (chars[*i].is_alphanumeric() || matches!(chars[*i], '_' | '.' | '$'))
    {
        *i += 1;
    }
    let word: String = chars[start..*i].iter().collect();
    if chars.get(*i) == Some(&'!') {
        *i += 1;
        *sheet = Some(word);
        return Ok(());
    }
    if chars.get(*i) == Some(&'(') {
        let name = word
            .trim_start_matches("_xlfn.")
            .trim_start_matches("_xlws.")
            .to_string();
        tokens.push(XlToken::Function(name));
        return Ok(());
    }
    if let Some(name) = word.strip_prefix("_xlpm.") {
        tokens.push(XlToken::Local(name.to_string()));
        return Ok(());
    }
    if let Some(first) = parse_cell(&word) {
        let mut last = None;
        if chars.get(*i) == Some(&':') {
            let s = *i + 1;
            let mut e = s;
            while e < chars.len() && (chars[e].is_ascii_alphanumeric() || chars[e] == '$') {
                e += 1;
            }
            let second: String = chars[s..e].iter().collect();
            last = Some(parse_cell(&second).ok_or("unsupported range")?);
            *i = e;
        }
        tokens.push(XlToken::Reference(sheet.take(), first, last));
        return Ok(());
    }
    match word.to_uppercase().as_str() {
        "TRUE" => tokens.push(XlToken::Bool(true)),
        "FALSE" => tokens.push(XlToken::Bool(false)),
        _ => return Err(format!("unsupported name '{word}'")),
    }
    Ok(())
}

/// Read text up to the closing `quote` (doubled quotes stand for one) starting
/// after the opening quote at `chars[*i]`. Leaves `*i` after the closing quote.
fn read_quoted(chars: &[char], i: &mut usize, quote: char) -> Result<String, String> {
    let mut text = String::new();
    *i += 1;
    loop {
        match chars.get(*i) {
            Some(&c) if c == quote && chars.get(*i + 1) == Some(&quote) => {
                text.push(quote);
                *i += 2;
            },
            Some(&c) if c == quote => {
                *i += 1;
                return Ok(text);
            },
            Some(&c) => {
                text.push(c);
                *i += 1;
            },
            None => return Err(format!("missing closing {quote}")),
        }
    }
}

/// Split an Excel formula into tokens. Fails on anything this translator does
/// not handle (named ranges, structured references, `%`, R1C1).
fn excel_tokens(formula: &str) -> Result<Vec<XlToken>, String> {
    let chars: Vec<char> = formula.trim().trim_start_matches('=').chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut sheet: Option<String> = None;
    while i < chars.len() {
        let c = chars[i];
        let before = tokens.len();
        match c {
            ' ' | '\t' | '\n' => i += 1,
            '"' => tokens.push(XlToken::Text(read_quoted(&chars, &mut i, '"')?)),
            '\'' => {
                let name = read_quoted(&chars, &mut i, '\'')?;
                if chars.get(i) != Some(&'!') {
                    return Err("quoted name without '!'".to_string());
                }
                i += 1;
                sheet = Some(name);
            },
            '0'..='9' | '.' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_digit()
                        || chars[i] == '.'
                        || ((chars[i] == 'e' || chars[i] == 'E') && i + 1 < chars.len())
                        || ((chars[i] == '+' || chars[i] == '-')
                            && matches!(chars[i - 1], 'e' | 'E')))
                {
                    i += 1;
                }
                tokens.push(XlToken::Number(chars[start..i].iter().collect()));
            },
            c if c.is_alphabetic() || c == '_' || c == '$' => {
                read_word(&chars, &mut i, &mut sheet, &mut tokens)?;
            },
            '<' | '>' => {
                let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
                if matches!(two.as_str(), "<=" | ">=" | "<>") {
                    tokens.push(XlToken::Operator(two));
                    i += 2;
                } else {
                    tokens.push(XlToken::Operator(c.to_string()));
                    i += 1;
                }
            },
            '+' | '-' | '*' | '/' | '^' | '&' | '=' => {
                tokens.push(XlToken::Operator(c.to_string()));
                i += 1;
            },
            '(' => {
                tokens.push(XlToken::Open);
                i += 1;
            },
            ')' => {
                tokens.push(XlToken::Close);
                i += 1;
            },
            ',' => {
                tokens.push(XlToken::Comma);
                i += 1;
            },
            other => return Err(format!("unsupported character '{other}'")),
        }
        // A sheet name is consumed by the cell reference right after it.
        if sheet.is_some() && tokens.len() > before {
            return Err("sheet name not followed by a cell".to_string());
        }
    }
    if sheet.is_some() {
        return Err("sheet name not followed by a cell".to_string());
    }
    Ok(tokens)
}

impl ReverseRefs {
    /// Translate an Excel formula written in `sheet` back to Forge.
    ///
    /// `row` is the Excel row of a table formula (references to other rows
    /// cannot be expressed in a Forge row formula); it is ignored for scalars.
    ///
    /// # Errors
    ///
    /// Returns the reason when the formula cannot be expressed in Forge.
    pub fn to_forge(&self, formula: &str, sheet: &str, row: u32) -> Result<String, String> {
        self.translate_back(formula, sheet, row, false)
    }

    /// [`ReverseRefs::to_forge`] for a table formula that may read the rows
    /// above: a cell of the row above is `PREV(column, 0)`, one `n` rows up
    /// `PREV(column, 0, n)`. The 0 stands for the first rows' value, which
    /// `forge import` reads from those rows (`previous_rows`).
    ///
    /// # Errors
    ///
    /// Returns the reason when the formula cannot be expressed in Forge.
    pub fn to_forge_reading_above(
        &self,
        formula: &str,
        sheet: &str,
        row: u32,
    ) -> Result<String, String> {
        self.translate_back(formula, sheet, row, true)
    }

    fn translate_back(
        &self,
        formula: &str,
        sheet: &str,
        row: u32,
        above: bool,
    ) -> Result<String, String> {
        let current = self
            .sheets
            .get(sheet)
            .ok_or_else(|| format!("unknown sheet '{sheet}'"))?;
        let mut out = String::from("=");
        let mut after_value = false;
        let tokens = excel_tokens(formula)?;
        let mut skip = 0;
        let mut only_in_count_unique = false;
        // Open parentheses, as in `translate` (ADR-058)
        let mut calls: Vec<Call> = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            track_call(&tokens, i, &mut calls);
            match token {
                XlToken::Number(n) => out.push_str(n),
                XlToken::Text(t) => {
                    if t.contains('"') {
                        return Err("text with a double quote".to_string());
                    }
                    let _ = write!(out, "\"{t}\"");
                },
                XlToken::Bool(b) => out.push_str(if *b { "TRUE" } else { "FALSE" }),
                // Excel's ROW() is the sheet's row; a table starts in row 2
                XlToken::Function(name)
                    if name.eq_ignore_ascii_case("ROW")
                        && matches!(current, SheetKind::Table { .. })
                        && tokens.get(i + 1..i + 3) == Some(&[XlToken::Open, XlToken::Close]) =>
                {
                    out.push_str("(ROW() + 1)");
                    skip = 2;
                    after_value = true;
                    continue;
                },
                // ROW(range) and UNIQUE are read only as part of what `forge
                // export` writes for COUNTUNIQUE (checked below)
                XlToken::Function(name)
                    if name.eq_ignore_ascii_case("ROW") || name.eq_ignore_ascii_case("UNIQUE") =>
                {
                    only_in_count_unique = true;
                    out.push_str(&name.to_uppercase());
                },
                XlToken::Function(name)
                    if DIFFERENT_IN_EXCEL.contains(&name.to_uppercase().as_str()) =>
                {
                    return Err(format!(
                        "function {name} works differently in Forge than in Excel"
                    ));
                },
                XlToken::Function(name) | XlToken::Local(name) => out.push_str(name),
                XlToken::Reference(target, first, last) => {
                    let target_name = target.as_deref().unwrap_or(sheet);
                    let kind = self
                        .sheets
                        .get(target_name)
                        .ok_or_else(|| format!("unknown sheet '{target_name}'"))?;
                    let name = Self::name_for(current, kind, first, last.as_ref(), (row, above))?;
                    Self::check_whole_column(current, kind, last.is_some(), &calls)?;
                    out.push_str(&name);
                },
                XlToken::Operator(op) => {
                    if after_value {
                        let _ = write!(out, " {op} ");
                    } else {
                        out.push_str(op);
                    }
                },
                XlToken::Open => out.push('('),
                XlToken::Close => out.push(')'),
                XlToken::Comma => out.push_str(", "),
            }
            after_value = matches!(
                token,
                XlToken::Number(_)
                    | XlToken::Text(_)
                    | XlToken::Bool(_)
                    | XlToken::Local(_)
                    | XlToken::Reference(..)
                    | XlToken::Close
            );
        }
        // What `forge export` writes for Forge's ROW()
        let out = out.replace("((ROW() + 1) - 1)", "ROW()");
        let out = local_names::restore_count_unique(&out);
        if only_in_count_unique {
            let rest = out.replace("ROW()", "").replace("COUNTUNIQUE(", "");
            if let Some(name) = ["ROW(", "UNIQUE("].iter().find(|f| rest.contains(**f)) {
                return Err(format!(
                    "function {} works differently in Forge than in Excel",
                    name.trim_end_matches('(')
                ));
            }
        }
        Ok(out)
    }

    /// A reference of a row formula read as Forge would read its name (ADR-058):
    /// a range only where Forge reads the whole column, one cell only where it
    /// reads the row's value. UNIQUE and ROW(range) are checked as COUNTUNIQUE.
    fn check_whole_column(
        current: &SheetKind,
        kind: &SheetKind,
        is_range: bool,
        calls: &[Call],
    ) -> Result<(), String> {
        if !matches!(current, SheetKind::Table { .. }) || !matches!(kind, SheetKind::Table { .. }) {
            return Ok(());
        }
        if calls
            .iter()
            .any(|(name, _, _)| matches!(name.as_deref(), Some("UNIQUE" | "ROW")))
        {
            return Ok(());
        }
        let whole = calls.iter().any(|(name, arg, count)| {
            name.as_deref().is_some_and(|n| {
                crate::core::array_calculator::whole_column::is_whole_column(n, *count, *arg)
            })
        });
        if is_range && !whole {
            return Err(
                "a range where Forge reads the row's value (a row formula reads a \
                        whole column only as an aggregate's range, ADR-058)"
                    .to_string(),
            );
        }
        if !is_range && whole {
            return Err("one cell where Forge reads the whole column (ADR-058)".to_string());
        }
        Ok(())
    }

    /// Forge name for a cell or range on `kind`, seen from `current`.
    fn name_for(
        current: &SheetKind,
        kind: &SheetKind,
        first: &Cell,
        last: Option<&Cell>,
        (row, above): (u32, bool),
    ) -> Result<String, String> {
        match kind {
            SheetKind::Scalars { rows, include } => {
                if last.is_some() || first.column != "B" {
                    return Err("reference to a scalar sheet outside the Value column".to_string());
                }
                let path = rows
                    .get(&first.row)
                    .cloned()
                    .ok_or_else(|| format!("no scalar in row {}", first.row))?;
                // An included file sees only its own names; the main model
                // reads an included file's names as `@namespace.path`.
                match (include.as_deref(), current.include()) {
                    (None, None) => Ok(path),
                    (Some(ns), Some(here)) if ns == here => Ok(path),
                    (Some(ns), None) => Ok(format!("@{ns}.{path}")),
                    (_, Some(here)) => Err(format!(
                        "the included file '{here}' reads a value outside itself"
                    )),
                }
            },
            SheetKind::Table { include, .. }
                if current.include().is_some() && include.as_deref() != current.include() =>
            {
                Err("an included file reads a table outside itself".to_string())
            },
            SheetKind::Table {
                name,
                columns,
                include,
            } => {
                if let Some(last) = last {
                    if last.column != first.column {
                        return Err("range over several columns".to_string());
                    }
                }
                let column = columns
                    .get(&first.column)
                    .ok_or_else(|| format!("no column {} in '{name}'", first.column))?;
                let in_current_table = matches!(current, SheetKind::Table { name: n, include: i, .. } if n == name && i == include);
                // The main model reads a table of an included file by namespace
                let qualified = match (include.as_deref(), current.include()) {
                    (Some(ns), None) => format!("@{ns}.{name}"),
                    _ => name.clone(),
                };
                let name = if in_current_table {
                    column.clone()
                } else {
                    format!("{qualified}.{column}")
                };
                match (current, last) {
                    // A single cell in a row formula is in the same row, or
                    // (where the caller reads it so) in a data row above
                    (SheetKind::Table { .. }, None) if above && (2..row).contains(&first.row) => {
                        Ok(match row - first.row {
                            1 => format!("PREV({name}, 0)"),
                            back => format!("PREV({name}, 0, {back})"),
                        })
                    },
                    (SheetKind::Table { .. }, None) if first.row != row => {
                        Err("reference to another row".to_string())
                    },
                    // A single table cell in a scalar formula would be indexing.
                    (SheetKind::Scalars { .. }, None) => {
                        Err("reference to a single table cell".to_string())
                    },
                    _ => Ok(name),
                }
            },
        }
    }
}

#[cfg(test)]
#[path = "workbook_refs/dates_tests.rs"]
mod dates_tests;
#[cfg(test)]
mod reverse_tests;
