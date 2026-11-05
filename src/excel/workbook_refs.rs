//! Formula translation between Forge and Excel, token by token.
//!
//! Export and import share one picture of the workbook:
//! - a table is a sheet named after it, one column per data or formula column
//!   (alphabetical), header in row 1, values from row 2;
//! - a scalar group `g` (scalars named `g.name`) is a sheet `g` with `Name` in
//!   column A and `Value` in column B, one scalar per row (alphabetical);
//! - ungrouped scalars live on a sheet named `Scalars` in the same layout.
//!
//! References are resolved the way the calculator resolves them, so a formula
//! means the same thing in Excel as in Forge. When it cannot (a function Excel
//! does not have, array indexing, `IRR` over several separate cash flows), the
//! translation fails with a reason and the caller writes the computed value.

use crate::core::array_calculator::tokenizer::{tokenize, Token};
use std::collections::HashMap;
use std::fmt::Write as _;

/// Sheet that holds scalars without a group.
pub const UNGROUPED_SHEET: &str = "Scalars";

/// Functions Forge accepts that Excel either lacks (`POW`, `COUNTUNIQUE`) or
/// evaluates differently: Forge's `VLOOKUP` takes one column and ignores the
/// column index, `ROW`/`COLUMN`/`OFFSET`/`INDIRECT` work on Forge names rather
/// than sheet cells, `TODAY`/`NOW` return text instead of a date serial, and
/// `UNIQUE`/`SORT`/`FILTER`/`SEQUENCE` spill in Excel.
const DIFFERENT_IN_EXCEL: [&str; 18] = [
    "POW",
    "TODAY",
    "NOW",
    "E",
    "COUNTUNIQUE",
    "UNIQUE",
    "SORT",
    "FILTER",
    "SEQUENCE",
    "RANDARRAY",
    "VLOOKUP",
    "HLOOKUP",
    "LAMBDA",
    "INDIRECT",
    "OFFSET",
    "ADDRESS",
    "ROW",
    "COLUMN",
];

/// Arguments that Forge reads as a whole column even inside a row formula
/// (function, 0-based argument position).
const WHOLE_COLUMN_ARGS: [(&str, usize); 5] = [
    ("INDEX", 0),
    ("MATCH", 1),
    ("ROWS", 0),
    ("XLOOKUP", 1),
    ("XLOOKUP", 2),
];

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
    #[must_use]
    pub fn new<'a>(
        tables: impl IntoIterator<Item = (&'a str, Vec<String>, usize)>,
        scalar_paths: impl IntoIterator<Item = &'a str>,
    ) -> Self {
        let mut refs = Self::default();
        for (table, mut columns, row_count) in tables {
            columns.sort();
            columns.dedup();
            let map = columns
                .into_iter()
                .enumerate()
                .map(|(i, name)| (name, column_letter(i)))
                .collect();
            refs.columns.insert(table.to_string(), map);
            refs.rows.insert(table.to_string(), row_count);
        }
        let mut groups: HashMap<String, Vec<&str>> = HashMap::new();
        for path in scalar_paths {
            let sheet = split_scalar_path(path).0.unwrap_or(UNGROUPED_SHEET);
            groups.entry(sheet.to_string()).or_default().push(path);
        }
        for (sheet, mut paths) in groups {
            paths.sort_unstable();
            for (i, path) in paths.into_iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)] // far below u32::MAX scalars
                let row = (i + 2) as u32;
                refs.scalars.insert(path.to_string(), (sheet.clone(), row));
            }
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
        let mut matches: Vec<&String> = self
            .scalars
            .keys()
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

    /// `$L$2:$L$n` for a table column, with an optional sheet prefix.
    fn column_range(&self, prefix: &str, table: &str, letter: &str) -> String {
        let last = self.rows.get(table).copied().unwrap_or(0) + 1;
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
                            sheet_prefix(other)
                        };
                        if whole_column {
                            *has_range = true;
                            return Ok(self.column_range(&prefix, other, letter));
                        }
                        return Ok(format!("{prefix}{letter}{row}"));
                    }
                }
                Err(format!("unknown name '{word}'"))
            },
            Context::Scalar { group } => {
                let sheet = group.unwrap_or(UNGROUPED_SHEET);
                if let Some(path) = self.resolve_scalar(word, group)? {
                    return Ok(self.scalar_cell(&path, Some(sheet)));
                }
                if let Some((table, column)) = word.split_once('.') {
                    if let Some(letter) = self.columns.get(table).and_then(|c| c.get(column)) {
                        *has_range = true;
                        return Ok(self.column_range(&sheet_prefix(table), table, letter));
                    }
                }
                Err(format!("unknown name '{word}'"))
            },
        }
    }

    /// Translate a Forge formula to Excel.
    ///
    /// # Errors
    ///
    /// Returns the reason when the formula cannot be written as an Excel formula
    /// with the same meaning.
    pub fn to_excel(&self, formula: &str, ctx: Context<'_>) -> Result<Translated, String> {
        let body = formula.trim().trim_start_matches('=');
        let tokens = tokenize(body).map_err(|e| e.to_string())?;
        check_irr_arguments(&tokens, ctx, self)?;

        let mut out = String::from("=");
        let mut has_range = false;
        // Open parentheses: (function name, if any; current argument position)
        let mut calls: Vec<(Option<String>, usize)> = Vec::new();
        let mut pending_function: Option<String> = None;
        // Whether the previous token ends a value, so `-` after it is binary.
        let mut after_value = false;
        for (i, token) in tokens.iter().enumerate() {
            let next_is_paren = matches!(tokens.get(i + 1), Some(Token::OpenParen));
            match token {
                Token::Number(n) => out.push_str(&number_text(*n)),
                Token::Text(t) => {
                    let _ = write!(out, "\"{}\"", t.replace('"', "\"\""));
                },
                Token::Boolean(b) => out.push_str(if *b { "TRUE" } else { "FALSE" }),
                Token::Identifier(word) if next_is_paren => {
                    let upper = word.to_uppercase();
                    if !super::FormulaTranslator::is_excel_function(word)
                        || DIFFERENT_IN_EXCEL.contains(&upper.as_str())
                    {
                        return Err(format!("function {word} has no Excel equivalent"));
                    }
                    out.push_str(&upper);
                    pending_function = Some(upper);
                },
                Token::Identifier(word) => {
                    let whole_column = calls.iter().any(|(name, arg)| {
                        name.as_deref()
                            .is_some_and(|n| WHOLE_COLUMN_ARGS.contains(&(n, *arg)))
                    });
                    out.push_str(&self.reference(word, ctx, whole_column, &mut has_range)?);
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
                    calls.push((pending_function.take(), 0));
                    out.push('(');
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
        }
        Ok(Translated {
            formula: out,
            has_range,
        })
    }
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

    fn refs() -> WorkbookRefs {
        WorkbookRefs::new(
            [
                (
                    "projections",
                    vec![
                        "year".to_string(),
                        "revenue".to_string(),
                        "total_profit".to_string(),
                    ],
                    3,
                ),
                ("other", vec!["x".to_string()], 3),
            ],
            [
                "assumptions.base_revenue",
                "assumptions.growth",
                "valuation.npv",
                "valuation.net",
                "tax",
            ],
        )
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
        // `year` is a column here, not YEAR(); the columns sort to
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

    #[test]
    fn untranslatable_formulas_say_why() {
        assert!(row("=MC.Normal(1, 2)")
            .unwrap_err()
            .contains("no Excel equivalent"));
        assert!(row("=revenue[0]").unwrap_err().contains("indexing"));
        assert!(row("=missing * 2").unwrap_err().contains("unknown name"));
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

    #[test]
    fn functions_that_differ_in_excel_are_not_translated() {
        for f in [
            "=VLOOKUP(1, revenue, 2)",
            "=COUNTUNIQUE(revenue)",
            "=POW(revenue, 2)",
            "=ROW()",
        ] {
            assert!(row(f).unwrap_err().contains("no Excel equivalent"), "{f}");
        }
    }

    #[test]
    fn sheet_names_with_quotes() {
        assert_eq!(sheet_prefix("Bob's"), "'Bob''s'!");
    }
}

/// What a sheet holds, read back from a workbook.
#[derive(Debug, Clone)]
pub enum SheetKind {
    /// A table: column letter -> column name
    Table {
        name: String,
        columns: HashMap<String, String>,
    },
    /// A scalar sheet (`Name` / `Value`): Excel row -> scalar path
    Scalars { rows: HashMap<u32, String> },
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
        let current = self
            .sheets
            .get(sheet)
            .ok_or_else(|| format!("unknown sheet '{sheet}'"))?;
        let mut out = String::from("=");
        let mut after_value = false;
        for token in excel_tokens(formula)? {
            match &token {
                XlToken::Number(n) => out.push_str(n),
                XlToken::Text(t) => {
                    if t.contains('"') {
                        return Err("text with a double quote".to_string());
                    }
                    let _ = write!(out, "\"{t}\"");
                },
                XlToken::Bool(b) => out.push_str(if *b { "TRUE" } else { "FALSE" }),
                XlToken::Function(name) => out.push_str(name),
                XlToken::Reference(target, first, last) => {
                    let target_name = target.as_deref().unwrap_or(sheet);
                    let kind = self
                        .sheets
                        .get(target_name)
                        .ok_or_else(|| format!("unknown sheet '{target_name}'"))?;
                    out.push_str(&Self::name_for(current, kind, first, last.as_ref(), row)?);
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
                    | XlToken::Reference(..)
                    | XlToken::Close
            );
        }
        Ok(out)
    }

    /// Forge name for a cell or range on `kind`, seen from `current`.
    fn name_for(
        current: &SheetKind,
        kind: &SheetKind,
        first: &Cell,
        last: Option<&Cell>,
        row: u32,
    ) -> Result<String, String> {
        match kind {
            SheetKind::Scalars { rows } => {
                if last.is_some() || first.column != "B" {
                    return Err("reference to a scalar sheet outside the Value column".to_string());
                }
                rows.get(&first.row)
                    .cloned()
                    .ok_or_else(|| format!("no scalar in row {}", first.row))
            },
            SheetKind::Table { name, columns } => {
                if let Some(last) = last {
                    if last.column != first.column {
                        return Err("range over several columns".to_string());
                    }
                }
                let column = columns
                    .get(&first.column)
                    .ok_or_else(|| format!("no column {} in '{name}'", first.column))?;
                let in_current_table =
                    matches!(current, SheetKind::Table { name: n, .. } if n == name);
                match (current, last) {
                    // A single cell in a row formula must be in the same row.
                    (SheetKind::Table { .. }, None) if first.row != row => {
                        Err("reference to another row".to_string())
                    },
                    // A single table cell in a scalar formula would be indexing.
                    (SheetKind::Scalars { .. }, None) => {
                        Err("reference to a single table cell".to_string())
                    },
                    _ if in_current_table => Ok(column.clone()),
                    _ => Ok(format!("{name}.{column}")),
                }
            },
        }
    }
}

#[cfg(test)]
mod reverse_tests {
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
                },
            );
        }
        for (path, (sheet, row)) in &refs.scalars {
            let entry = sheets
                .entry(sheet.clone())
                .or_insert_with(|| SheetKind::Scalars {
                    rows: HashMap::new(),
                });
            if let SheetKind::Scalars { rows } = entry {
                rows.insert(*row, path.clone());
            }
        }
        ReverseRefs { sheets }
    }

    fn layout() -> WorkbookRefs {
        WorkbookRefs::new(
            [
                (
                    "p",
                    vec!["year".to_string(), "rev".to_string(), "profit".to_string()],
                    3,
                ),
                ("q", vec!["x".to_string()], 3),
            ],
            ["a.rate", "a.base", "v.npv", "tax"],
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
}
