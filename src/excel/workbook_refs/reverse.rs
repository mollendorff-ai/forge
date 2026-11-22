//! Excel formulas read back as Forge formulas: the workbook's own tokens,
//! its cells and sheets, and how each reference becomes a Forge name.

use super::{constants, local_names, Call, DIFFERENT_IN_EXCEL};
use std::collections::HashMap;
use std::fmt::Write as _;

/// What a sheet holds, read back from a workbook.
#[derive(Debug, Clone)]
pub enum SheetKind {
    /// A table: column letter -> column name. `include` is the namespace when
    /// the sheet holds a table of an included file (`{namespace}.{table}`):
    /// `name` is then the file's own name for it, and the main model reads
    /// its columns as `@{namespace}.{name}.{column}`.
    ///
    /// `rows` are the Excel rows of the table's data, in order (a sheet
    /// someone else wrote may have rows above its header, a blank row inside
    /// the table, or a total row below it), and `ends` the Excel row of each
    /// column's last cell with something in it, by letter, where that is above
    /// the table's last row. A range is the column only from its first row to
    /// its end; any other range is part of it.
    Table {
        name: String,
        columns: HashMap<String, String>,
        include: Option<String>,
        rows: Vec<u32>,
        ends: HashMap<String, u32>,
        /// A total row under the table: its Excel row, and the group its
        /// cells are scalars of (`monthly_total.revenue`)
        total: Option<(u32, String)>,
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
    /// The workbook's names (in lower case, as Excel compares them) -> what
    /// each stands for (`list_price` -> `Assumptions!$B$7`)
    pub names: HashMap<String, String>,
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
    /// An array constant (ADR-064), already as Forge text
    Constant(String),
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
            '{' => tokens.push(XlToken::Constant(constants::from_excel(&chars, &mut i)?)),
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

    /// [`ReverseRefs::to_forge`] for a cell of a total row under a table on
    /// `sheet`: a scalar, which reads the table's columns by their names.
    ///
    /// # Errors
    ///
    /// Returns the reason when the formula cannot be expressed in Forge.
    pub fn total_to_forge(&self, formula: &str, sheet: &str) -> Result<String, String> {
        let scalar = SheetKind::Scalars {
            rows: HashMap::new(),
            include: None,
        };
        self.translate_on(&scalar, formula, sheet, (0, false))
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
        self.translate_on(current, formula, sheet, (row, above))
    }

    /// A workbook's names in `formula` replaced by what they stand for.
    fn expand_names(&self, formula: &str) -> String {
        let chars: Vec<char> = formula.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            if c == '"' || c == '\'' {
                let start = i;
                let _ = read_quoted(&chars, &mut i, c);
                out.extend(&chars[start..i.min(chars.len())]);
                continue;
            }
            if !(c.is_alphabetic() || c == '_') {
                out.push(c);
                i += 1;
                continue;
            }
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || matches!(chars[i], '_' | '.')) {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let part_of_a_reference = start > 0 && matches!(chars[start - 1], '$' | '!' | ':');
            let called = matches!(chars.get(i), Some('(' | '!'));
            match self.names.get(&word.to_lowercase()) {
                Some(target) if !part_of_a_reference && !called => out.push_str(target),
                _ => out.push_str(&word),
            }
        }
        out
    }

    fn translate_on(
        &self,
        current: &SheetKind,
        formula: &str,
        sheet: &str,
        (row, above): (u32, bool),
    ) -> Result<String, String> {
        let table_rows = match current {
            SheetKind::Table { rows, .. } => Some(rows.as_slice()),
            SheetKind::Scalars { .. } => None,
        };
        let mut out = String::from("=");
        let mut after_value = false;
        let tokens = excel_tokens(&self.expand_names(formula))?;
        let mut skip = 0;
        let mut only_in_count_unique = false;
        // Open parentheses, as in `translate` (ADR-058)
        let mut calls: Vec<Call> = Vec::new();
        for (i, token) in tokens.iter().enumerate() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            track_call(&tokens, i, &mut calls)?;
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
                        && table_rows.is_some()
                        && tokens.get(i + 1..i + 3) == Some(&[XlToken::Open, XlToken::Close]) =>
                {
                    out.push_str(&Self::excel_row_function(table_rows.unwrap_or_default())?);
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
                XlToken::Constant(text) => out.push_str(text),
            }
            after_value = matches!(
                token,
                XlToken::Number(_)
                    | XlToken::Constant(_)
                    | XlToken::Text(_)
                    | XlToken::Bool(_)
                    | XlToken::Local(_)
                    | XlToken::Reference(..)
                    | XlToken::Close
            );
        }
        if !calls.is_empty() {
            return Err("a ( with no ) after it".to_string());
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

    /// Excel's `ROW()` in a table as Forge's: the table's first data row is
    /// Forge's row 1. A table with a gap in its rows has no such formula.
    fn excel_row_function(rows: &[u32]) -> Result<String, String> {
        let start = rows.first().copied().unwrap_or(2);
        let gap = rows.windows(2).any(|pair| pair[1] != pair[0] + 1);
        if gap {
            return Err("ROW() in a table with a gap in its rows".to_string());
        }
        Ok(format!("(ROW() + {})", start - 1))
    }

    /// A range of a table's column is the column, as Forge reads `table.column`,
    /// only when it runs from the table's first data row to the column's last
    /// cell with something in it, or further down to the table's last row.
    fn check_column_range(
        rows: &[u32],
        end: Option<&u32>,
        first: &Cell,
        last: &Cell,
    ) -> Result<(), String> {
        let (top, bottom) = (first.row.min(last.row), first.row.max(last.row));
        let (Some(start), Some(stop)) = (rows.first(), rows.last()) else {
            return Err("a range in a table with no data rows".to_string());
        };
        let end = end.copied().unwrap_or(*stop);
        if top == *start && (end..=*stop).contains(&bottom) {
            return Ok(());
        }
        Err(format!(
            "a range over part of a column: rows {top} to {bottom} of a table in rows \
             {start} to {stop}"
        ))
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
                rows,
                ends,
                total,
            } => {
                // A cell of the total row is a scalar of its group
                if let Some((_, group)) = total.as_ref().filter(|(r, _)| *r == first.row) {
                    let column = columns
                        .get(&first.column)
                        .ok_or_else(|| format!("no column {} in '{name}'", first.column))?;
                    if last.is_some() {
                        return Err("a range in a total row".to_string());
                    }
                    return Ok(format!("{group}.{column}"));
                }
                if let Some(last) = last {
                    if last.column != first.column {
                        return Err("range over several columns".to_string());
                    }
                    Self::check_column_range(rows, ends.get(&first.column), first, last)?;
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
                // How many data rows above the formula's row the cell is
                let back = rows
                    .iter()
                    .position(|r| *r == row)
                    .zip(rows.iter().position(|r| *r == first.row))
                    .and_then(|(here, there)| here.checked_sub(there))
                    .filter(|back| *back > 0);
                match (current, last) {
                    // A single cell in a row formula is in the same row, or
                    // (where the caller reads it so) in a data row above
                    (SheetKind::Table { .. }, None) if above && back.is_some() => Ok(match back {
                        Some(1) => format!("PREV({name}, 0)"),
                        back => format!("PREV({name}, 0, {})", back.unwrap_or_default()),
                    }),
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

/// Keep `calls` up to date at token `i` of an Excel formula: a `(` opens a
/// call (with its function and number of arguments), a `)` closes one, a `,`
/// moves to the next argument. A `)` or a `,` with no call open is refused.
fn track_call(tokens: &[XlToken], i: usize, calls: &mut Vec<Call>) -> Result<(), String> {
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
            calls.pop().ok_or("a ) with no ( before it")?;
        },
        XlToken::Comma => {
            let call = calls.last_mut().ok_or("a , outside parentheses")?;
            call.1 += 1;
        },
        _ => {},
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    //! Tests of `ReverseRefs`: Excel formulas read back as Forge formulas.
    use crate::excel::workbook_refs::{Context, WorkbookRefs};

    use super::*;

    /// The layout `WorkbookRefs::new` produces, read back.
    fn reverse(refs: &WorkbookRefs) -> ReverseRefs {
        let mut sheets = HashMap::new();
        for (table, columns) in &refs.columns {
            let columns = columns
                .iter()
                .map(|(n, l)| (l.clone(), n.clone()))
                .collect();
            sheets.insert(
                table.clone(),
                SheetKind::Table {
                    name: table.clone(),
                    columns,
                    include: None,
                    rows: (2..2 + u32::try_from(refs.rows[table]).unwrap()).collect(),
                    ends: HashMap::new(),
                    total: None,
                },
            );
        }
        let mut scalars: HashMap<String, HashMap<u32, String>> = HashMap::new();
        for (path, (sheet, row)) in &refs.scalars {
            scalars
                .entry(sheet.clone())
                .or_default()
                .insert(*row, path.clone());
        }
        sheets.extend(scalars.into_iter().map(|(sheet, rows)| {
            (
                sheet,
                SheetKind::Scalars {
                    rows,
                    include: None,
                },
            )
        }));
        ReverseRefs {
            sheets,
            names: HashMap::new(),
        }
    }

    fn custom() -> ReverseRefs {
        let table =
            |name: &str, include: Option<&str>, total: Option<(u32, &str)>| SheetKind::Table {
                name: name.to_string(),
                columns: HashMap::from([
                    ("A".to_string(), "profit".to_string()),
                    ("B".to_string(), "rev".to_string()),
                ]),
                include: include.map(str::to_string),
                rows: vec![2, 3, 4],
                ends: HashMap::new(),
                total: total.map(|(r, g)| (r, g.to_string())),
            };
        ReverseRefs {
            sheets: HashMap::from([
                ("p".to_string(), table("p", None, Some((6, "p_total")))),
                ("costs.t".to_string(), table("t", Some("costs"), None)),
                (
                    "v".to_string(),
                    SheetKind::Scalars {
                        rows: HashMap::from([(2, "v.npv".to_string())]),
                        include: None,
                    },
                ),
            ]),
            names: HashMap::new(),
        }
    }

    /// Each Excel formula that cannot be read back, with why
    #[test]
    fn what_cannot_be_read_back_is_refused_with_why() {
        let back = custom();
        for (sheet, formula, why) in [
            // Not Excel as Forge writes it
            ("p", "=A99999999999", "unsupported name 'A99999999999'"),
            ("p", "=nope", "unsupported name 'nope'"),
            ("p", "=$1", "unsupported name '$1'"),
            ("p", "=ABCD1", "unsupported name 'ABCD1'"),
            ("p", "=a1", "unsupported name 'a1'"),
            ("p", "=SUM(B2:foo)", "unsupported range"),
            ("p", "=B2%", "unsupported character '%'"),
            ("p", "=\"abc", "missing closing \""),
            ("p", "='abc", "missing closing '"),
            ("p", "='p'B2", "quoted name without '!'"),
            ("p", "={1,2", "array constant without }"),
            ("p", "=\"a\"\"b\"", "text with a double quote"),
            ("p", "=p!", "sheet name not followed by a cell"),
            ("p", "=p!1", "sheet name not followed by a cell"),
            ("p", "=p!+B2", "sheet name not followed by a cell"),
            ("p", "=1, 2", "a , outside parentheses"),
            ("p", "=1)", "a ) with no ( before it"),
            ("p", "=SUM(1", "a ( with no ) after it"),
            ("p", "=SUM(B2:B4", "a ( with no ) after it"),
            // Sheets, columns and rows that are not there
            ("p", "=zz!B2", "unknown sheet 'zz'"),
            ("zz", "=B2", "unknown sheet 'zz'"),
            ("p", "=p!E2", "no column E in 'p'"),
            ("p", "=p!E6", "no column E in 'p'"),
            ("v", "=v!B9", "no scalar in row 9"),
            ("v", "=v!A2", "reference to a scalar sheet outside the Value column"),
            ("v", "=v!B2:B3", "reference to a scalar sheet outside the Value column"),
            // Ranges and cells that Forge reads differently
            (
                "p",
                "=B2:B4 + 1",
                "a range where Forge reads the row's value (a row formula reads a whole column only as an aggregate's range, ADR-058)",
            ),
            (
                "p",
                "=SUM(B2)",
                "one cell where Forge reads the whole column (ADR-058)",
            ),
            ("p", "=p!A2:B3", "range over several columns"),
            (
                "p",
                "=B2:B3",
                "a range over part of a column: rows 2 to 3 of a table in rows 2 to 4",
            ),
            ("p", "=p!B6:B7", "a range in a total row"),
            // An included file reads only itself
            (
                "costs.t",
                "=v!B2",
                "the included file 'costs' reads a value outside itself",
            ),
            (
                "costs.t",
                "=p!B2",
                "an included file reads a table outside itself",
            ),
        ] {
            assert_eq!(
                back.to_forge(formula, sheet, 2),
                Err(why.to_string()),
                "{sheet}: {formula}"
            );
        }
        assert_eq!(
            back.total_to_forge("=SUM(B2:B4)", "zz"),
            Err("unknown sheet 'zz'".to_string())
        );
    }

    /// What is read back: a total row's cell is a scalar of its group, an
    /// included table is read by its namespace, and the operators, numbers
    /// and `ROW()` of a row formula are as written
    #[test]
    fn what_can_be_read_back() {
        let back = custom();
        for (sheet, formula, forge) in [
            ("p", "=B6", "=p_total.rev"),
            ("costs.t", "=A2", "=profit"),
            ("v", "='costs.t'!B2:B4", "=@costs.t.rev"),
            ("p", "=TRUE + FALSE", "=TRUE + FALSE"),
            ("p", "=A2 <> B2", "=profit <> rev"),
            ("p", "=A2 < B2 > 1", "=profit < rev > 1"),
            ("p", "=2E5 + 1.5e-3 + 3E+2", "=2E5 + 1.5e-3 + 3E+2"),
            ("p", "=ROW()", "=(ROW() + 1)"),
        ] {
            assert_eq!(
                back.to_forge(formula, sheet, 2),
                Ok(forge.to_string()),
                "{sheet}: {formula}"
            );
        }
        assert_eq!(
            back.total_to_forge("=SUM(B2:B4)", "p"),
            Ok("=SUM(p.rev)".to_string())
        );
    }

    fn layout() -> WorkbookRefs {
        WorkbookRefs::new(
            [
                // In layout order: profit (A), rev (B), year (C); a.base (row 2), a.rate (row 3)
                (
                    "p",
                    vec!["profit".to_string(), "rev".to_string(), "year".to_string()],
                    3,
                ),
                ("q", vec!["x".to_string()], 3),
            ],
            ["a.base", "a.rate", "v.npv", "tax"],
        )
    }

    #[test]
    fn row_formulas_round_trip() {
        let refs = layout();
        let back = reverse(&refs);
        for f in [
            "=a.base * (1 + a.rate) ^ year",
            "=rev - q.x",
            "=IF(rev > 0, \"up\", FALSE)",
            "=-rev + 1.5",
            "=MATCH(rev, q.x, 0)",
            "=rev * tax",
        ] {
            let xl = refs
                .to_excel(f, Context::Row { table: "p", row: 2 })
                .unwrap();
            assert_eq!(
                back.to_forge(&xl.formula, "p", 2).unwrap(),
                f,
                "{}",
                xl.formula
            );
        }
    }

    #[test]
    fn scalar_formulas_round_trip() {
        let refs = layout();
        let back = reverse(&refs);
        for f in [
            "=NPV(a.rate, p.profit)",
            "=v.npv - a.base",
            "=SUM(p.rev) * tax",
        ] {
            let xl = refs
                .to_excel(f, Context::Scalar { group: Some("v") })
                .unwrap();
            assert_eq!(
                back.to_forge(&xl.formula, "v", 0).unwrap(),
                f,
                "{}",
                xl.formula
            );
        }
    }

    /// `@costs.x` is a cell on the include's sheet and comes back as
    /// `@costs.x`; the include's own formulas come back in its own names.
    #[test]
    fn include_references_round_trip() {
        let mut refs = layout();
        refs.add_include("costs", ["labour.wage", "labour.loaded"]);
        let mut back = reverse(&refs);
        let xl = refs
            .to_excel(
                "=a.base * @costs.labour.wage",
                Context::Scalar { group: Some("v") },
            )
            .unwrap();
        assert_eq!(xl.formula, "='a'!$B$2 * 'costs.Scalars'!$B$2");
        let xl_row = refs
            .to_excel(
                "=rev * @costs.labour.loaded",
                Context::Row { table: "p", row: 3 },
            )
            .unwrap();
        assert_eq!(xl_row.formula, "=B3 * 'costs.Scalars'!$B$3");
        // A bare last segment does not reach into an included file
        assert!(refs
            .to_excel("=wage", Context::Scalar { group: None })
            .unwrap_err()
            .contains("unknown name"));

        let own = WorkbookRefs::for_include(
            "costs",
            ["labour.wage", "labour.loaded"],
            std::iter::empty(),
        );
        let xl_own = own
            .to_excel(
                "=wage * 1.15",
                Context::Scalar {
                    group: Some("labour"),
                },
            )
            .unwrap();
        assert_eq!(xl_own.formula, "='costs.Scalars'!$B$2 * 1.15");

        // As the importer reads the sheet: the include's own paths
        back.sheets.insert(
            "costs.Scalars".to_string(),
            SheetKind::Scalars {
                rows: HashMap::from([
                    (2, "labour.wage".to_string()),
                    (3, "labour.loaded".to_string()),
                ]),
                include: Some("costs".to_string()),
            },
        );
        assert_eq!(
            back.to_forge(&xl.formula, "v", 0).unwrap(),
            "=a.base * @costs.labour.wage"
        );
        assert_eq!(
            back.to_forge(&xl_row.formula, "p", 3).unwrap(),
            "=rev * @costs.labour.loaded"
        );
        assert_eq!(
            back.to_forge(&xl_own.formula, "costs.Scalars", 0).unwrap(),
            "=labour.wage * 1.15"
        );
        assert!(back
            .to_forge("='a'!$B$2 * 2", "costs.Scalars", 0)
            .unwrap_err()
            .contains("outside itself"));
    }

    #[test]
    fn what_cannot_come_back() {
        let back = reverse(&layout());
        assert!(back
            .to_forge("=C3 - B2", "p", 2)
            .unwrap_err()
            .contains("another row"));
        assert!(back
            .to_forge("='p'!$A$2", "v", 0)
            .unwrap_err()
            .contains("single table cell"));
        assert!(back
            .to_forge("=MyRange * 2", "p", 2)
            .unwrap_err()
            .contains("unsupported name"));
        assert!(back
            .to_forge("=A2:B4", "p", 2)
            .unwrap_err()
            .contains("several columns"));
    }

    #[test]
    fn excel_prefixes_and_quotes() {
        let back = reverse(&layout());
        assert_eq!(
            back.to_forge("=_xlfn.STDEV.S('p'!$A$2:$A$4)", "v", 0)
                .unwrap(),
            "=STDEV.S(p.profit)"
        );
        assert_eq!(
            back.to_forge("=\"a\"\"b\"", "v", 0).unwrap_err(),
            "text with a double quote"
        );
    }

    /// A range of a table that has a header and no rows is not its column,
    /// and a range must start at the table's first row.
    #[test]
    fn a_range_is_the_column_only_from_its_first_row_to_its_end() {
        let table = |rows: Vec<u32>, ends: HashMap<String, u32>| SheetKind::Table {
            name: "t".to_string(),
            columns: HashMap::from([("A".to_string(), "x".to_string())]),
            include: None,
            rows,
            ends,
            total: None,
        };
        let refs = |kind: SheetKind| ReverseRefs {
            sheets: HashMap::from([
                ("t".to_string(), kind),
                (
                    "s".to_string(),
                    SheetKind::Scalars {
                        rows: HashMap::new(),
                        include: None,
                    },
                ),
            ]),
            names: HashMap::new(),
        };
        let empty = refs(table(Vec::new(), HashMap::new()));
        assert!(empty
            .to_forge("=SUM(t!A2:A3)", "s", 0)
            .unwrap_err()
            .contains("no data rows"));
        // Rows 2 to 6, column A filled to row 4: A2:A4 to A2:A6 are the column
        let short = refs(table(
            (2..=6).collect(),
            HashMap::from([("A".to_string(), 4)]),
        ));
        for whole in ["=SUM(t!A2:A4)", "=SUM(t!A2:A6)", "=SUM(t!A5:A2)"] {
            let forge = short.to_forge(whole, "s", 0);
            assert_eq!(forge.as_deref(), Ok("=SUM(t.x)"), "{whole}");
        }
        for part in ["=SUM(t!A2:A3)", "=SUM(t!A3:A6)", "=SUM(t!A2:A7)"] {
            let err = short.to_forge(part, "s", 0).unwrap_err();
            assert!(err.contains("part of a column"), "{part}: {err}");
        }
    }

    #[test]
    fn a_workbook_name_is_what_it_names() {
        let back = ReverseRefs {
            sheets: HashMap::from([(
                "a".to_string(),
                SheetKind::Scalars {
                    rows: HashMap::from([(2, "a.price".to_string())]),
                    include: None,
                },
            )]),
            names: HashMap::from([("list_price".to_string(), "a!$B$2".to_string())]),
        };
        assert_eq!(
            back.to_forge("=LIST_PRICE * 2", "a", 0).as_deref(),
            Ok("=a.price * 2")
        );
        // Not inside text, not a function of the same name, not a sheet
        assert_eq!(
            back.expand_names(r#"=IF(x = "list_price", list_price(1), 'list_price'!B2)"#),
            r#"=IF(x = "list_price", list_price(1), 'list_price'!B2)"#
        );
        assert_eq!(back.expand_names("=a!list_price"), "=a!list_price");
        assert_eq!(back.expand_names("=\"open"), "=\"open");
    }
}
