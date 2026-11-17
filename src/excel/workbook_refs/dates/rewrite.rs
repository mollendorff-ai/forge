//! The walk over a formula that writes its dates for the workbook: what each
//! part holds (a date, a number, a text), and where a date must become a day
//! number or a day number a date.

use super::super::{Context, WorkbookRefs};
use super::excel_date;
use super::kinds::{
    join, passed_on, switch_results, Kind, Read, ANY_VALUES, CRITERIA, GIVE_A_DATE, GIVE_TEXT,
    READ_NUMBERS,
};
use crate::core::array_calculator::parser::{Expr, Reference};
use crate::core::array_calculator::whole_column::is_whole_column;

pub(super) struct Dates<'a> {
    pub(super) refs: &'a WorkbookRefs,
    pub(super) ctx: Context<'a>,
    /// Names of an enclosing LET, innermost last
    pub(super) locals: Vec<(String, Kind)>,
}

impl Dates<'_> {
    pub(super) fn word(reference: &Reference) -> String {
        match reference {
            Reference::Scalar(name) => name.clone(),
            Reference::TableColumn { table, column } => format!("{table}.{column}"),
        }
    }

    /// The `table.column` a name is, resolved as `WorkbookRefs::reference`
    /// resolves it; `None` for a scalar or an unknown name.
    pub(super) fn column_of(&self, word: &str) -> Option<String> {
        let known = |table: &str, column: &str| {
            self.refs
                .columns
                .get(table)
                .is_some_and(|c| c.contains_key(column))
                .then(|| format!("{table}.{column}"))
        };
        if word.starts_with('@') && !self.refs.scalars.contains_key(word) {
            if let Some((table, column)) = word.rsplit_once('.') {
                if let Some(found) = known(table, column) {
                    return Some(found);
                }
            }
        }
        match self.ctx {
            Context::Row { table, .. } => {
                if let Some(found) = known(table, word) {
                    return Some(found);
                }
                if matches!(self.refs.resolve_scalar(word, Some(table)), Ok(Some(_))) {
                    return None;
                }
                word.split_once('.').and_then(|(t, c)| known(t, c))
            },
            Context::Scalar { group } => {
                if matches!(self.refs.resolve_scalar(word, group), Ok(Some(_))) {
                    return None;
                }
                word.split_once('.').and_then(|(t, c)| known(t, c))
            },
        }
    }

    pub(super) fn kind_of_name(&self, word: &str) -> Kind {
        if let Some((_, kind)) = self.locals.iter().rev().find(|(name, _)| name == word) {
            return *kind;
        }
        match self.column_of(word) {
            Some(column) if self.refs.date_columns.contains(&column) => Kind::Date,
            Some(column) if self.refs.mixed_columns.contains(&column) => Kind::Mixed,
            Some(column) => match self.refs.text_columns.get(&column) {
                Some(true) => Kind::Text,
                Some(false) => Kind::Number,
                None => Kind::Unknown,
            },
            // A scalar is a number or a date (ADR-060); an unknown name is
            // reported by the translator
            None => {
                let group = match self.ctx {
                    Context::Row { table, .. } => Some(table),
                    Context::Scalar { group } => group,
                };
                let path = match self.refs.resolve_scalar(word, group) {
                    Ok(Some(path)) => path,
                    _ => word.to_string(),
                };
                if self.refs.date_scalars.contains(&path) {
                    Kind::Date
                } else {
                    Kind::Number
                }
            },
        }
    }

    pub(super) fn kind(&self, expr: &Expr) -> Kind {
        match expr {
            Expr::Number(_) | Expr::Boolean(_) | Expr::UnaryOp { .. } => Kind::Number,
            Expr::Text(text) => {
                if excel_date(text).is_some() {
                    Kind::Written
                } else {
                    Kind::Text
                }
            },
            Expr::Reference(reference) => self.kind_of_name(&Self::word(reference)),
            Expr::BinaryOp { op, .. } => {
                if op == "&" {
                    Kind::Text
                } else {
                    Kind::Number
                }
            },
            Expr::FunctionCall { name, args } => self.kind_of_call(&name.to_uppercase(), args),
            Expr::ArrayIndex { .. }
            | Expr::CallResult { .. }
            | Expr::Range { .. }
            | Expr::Array { .. } => Kind::Unknown,
        }
    }

    pub(super) fn kind_of_call(&self, name: &str, args: &[Expr]) -> Kind {
        if GIVE_A_DATE.contains(&name) {
            return Kind::Date;
        }
        if GIVE_TEXT.contains(&name) {
            return Kind::Text;
        }
        if let Some(values) = passed_on(name, args.len()) {
            return join(values.into_iter().map(|i| self.kind(&args[i])));
        }
        match name {
            "INDEX" => args.first().map_or(Kind::Unknown, |a| self.kind(a)),
            "PREV" => join(args.iter().take(2).map(|a| self.kind(a))),
            "XLOOKUP" => join(args.iter().skip(2).take(2).map(|a| self.kind(a))),
            "SWITCH" => join(
                switch_results(args.len())
                    .into_iter()
                    .map(|i| self.kind(&args[i])),
            ),
            "LET" => {
                // The kind of the body, with the names bound before it
                let mut inner = Dates {
                    refs: self.refs,
                    ctx: self.ctx,
                    locals: self.locals.clone(),
                };
                for pair in args[..args.len().saturating_sub(1)].chunks(2) {
                    if let [Expr::Reference(name), value] = pair {
                        let kind = inner.kind(value);
                        inner.locals.push((Self::word(name), kind));
                    }
                }
                args.last().map_or(Kind::Unknown, |body| inner.kind(body))
            },
            "LAMBDA" | "NA" | "SCENARIO" | "OFFSET" | "INDIRECT" | "VLOOKUP" | "HLOOKUP"
            | "UNIQUE" | "SORT" | "FILTER" | "SEQUENCE" => Kind::Unknown,
            _ => Kind::Number,
        }
    }

    /// Whether the value of a part of a formula may be a date: a date, or a
    /// value passed on by `IF` and its like that is a date in some cases.
    pub(super) fn may_be_a_date(&self, expr: &Expr) -> bool {
        if self.kind(expr).is_date() {
            return true;
        }
        let Expr::FunctionCall { name, args } = expr else {
            return false;
        };
        let upper = name.to_uppercase();
        let values: Vec<usize> = match upper.as_str() {
            "PREV" => (0..args.len().min(2)).collect(),
            "LET" => args.len().checked_sub(1).into_iter().collect(),
            "SWITCH" => switch_results(args.len()),
            other => passed_on(other, args.len()).unwrap_or_default(),
        };
        values.into_iter().any(|i| self.may_be_a_date(&args[i]))
    }

    /// Whether a part of a formula has a date in it at all.
    pub(super) fn touches_dates(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Number(_) | Expr::Boolean(_) => false,
            Expr::Text(text) => excel_date(text).is_some(),
            Expr::Array { items, .. } => items.iter().any(|item| self.touches_dates(item)),
            Expr::Reference(reference) => self
                .kind_of_name(&Self::word(reference))
                .may_be_read_as_a_date(),
            Expr::ArrayIndex { array, index } => {
                self.touches_dates(array) || self.touches_dates(index)
            },
            Expr::FunctionCall { name, args } => {
                GIVE_A_DATE.contains(&name.to_uppercase().as_str())
                    || args.iter().any(|a| self.touches_dates(a))
            },
            Expr::CallResult { callable, args } => {
                self.touches_dates(callable) || args.iter().any(|a| self.touches_dates(a))
            },
            Expr::BinaryOp { left, right, .. } => {
                self.touches_dates(left) || self.touches_dates(right)
            },
            Expr::UnaryOp { operand, .. } => self.touches_dates(operand),
            Expr::Range { start, end } => self.touches_dates(start) || self.touches_dates(end),
        }
    }

    /// `expr` as the workbook must hold it for a reader that reads a date as
    /// `read`. `whole` says a column named here is the whole column, not the
    /// cell of the row.
    pub(super) fn write(&mut self, expr: &Expr, read: Read, whole: bool) -> Result<Expr, String> {
        if read == Read::Number && self.kind(expr) == Kind::Mixed {
            return Err(match expr {
                Expr::Reference(reference) => format!(
                    "{} is text in the workbook, and Forge reads its dates as numbers here",
                    Self::word(reference)
                ),
                _ => "text that may be a date is read as a number".to_string(),
            });
        }
        if read == Read::Number && self.is_text_value(expr) {
            return Err(match expr {
                Expr::Reference(reference) => format!(
                    "{} is text, read here as a number; Forge and Excel turn text into numbers differently",
                    Self::word(reference)
                ),
                _ => "text is read here as a number; Forge and Excel turn text into numbers differently"
                    .to_string(),
            });
        }
        match expr {
            Expr::Number(_) | Expr::Boolean(_) => Ok(expr.clone()),
            // Excel reads a date written as text in braces as text, Forge as a
            // date (ADR-060), so such a constant is not written as a formula
            Expr::Array { items, .. } => {
                if items
                    .iter()
                    .any(|item| matches!(item, Expr::Text(text) if excel_date(text).is_some()))
                {
                    return Err(
                        "an array constant holds a date as text, which Excel reads as text"
                            .to_string(),
                    );
                }
                Ok(expr.clone())
            },
            Expr::Text(text) => Ok(match (read, as_number(text)) {
                (Read::Number, Some(date)) => date,
                _ => expr.clone(),
            }),
            Expr::Reference(reference) => {
                if read == Read::Text && self.kind_of_name(&Self::word(reference)) == Kind::Date {
                    let is_column = self.column_of(&Self::word(reference)).is_some();
                    if is_column && (whole || matches!(self.ctx, Context::Scalar { .. })) {
                        return Err(format!(
                            "the dates of {} are read as text, a whole column at once",
                            Self::word(reference)
                        ));
                    }
                    return Ok(as_text(expr));
                }
                Ok(expr.clone())
            },
            Expr::UnaryOp { op, operand } => Ok(Expr::UnaryOp {
                op: op.clone(),
                operand: Box::new(self.write(operand, Read::Number, whole)?),
            }),
            Expr::BinaryOp { op, left, right } => {
                if ["<", ">", "<=", ">="].contains(&op.as_str())
                    && [left, right]
                        .iter()
                        .any(|side| matches!(self.kind(side), Kind::Text | Kind::Mixed))
                {
                    return Err(format!(
                        "{op} compares text; Excel puts any text above every number and orders text by its letters, and Forge does neither"
                    ));
                }
                let read = match op.as_str() {
                    "&" => Read::Text,
                    "=" | "<>" => self.equality(left, right)?,
                    _ => Read::Number,
                };
                Ok(binary(
                    op,
                    self.write(left, read, whole)?,
                    self.write(right, read, whole)?,
                ))
            },
            Expr::FunctionCall { name, args } => self.write_call(name, args, read, whole),
            Expr::ArrayIndex { .. } | Expr::CallResult { .. } | Expr::Range { .. } => {
                if self.touches_dates(expr) {
                    Err("a date inside [ ], a range or a LAMBDA call".to_string())
                } else {
                    Ok(expr.clone())
                }
            },
        }
    }

    /// Whether `expr` is text made in the formula that a reader would turn
    /// into a number: a function that gives text, or `&`. Text written in the
    /// formula is what it shows, and `IF` and its like are judged by the
    /// values they pass on.
    pub(super) fn is_text_value(&self, expr: &Expr) -> bool {
        if self.kind(expr) != Kind::Text {
            return false;
        }
        // A column of text is judged by its values (`mixed_columns`): a
        // column of words reads the same in both
        match expr {
            Expr::BinaryOp { op, .. } => op == "&",
            Expr::FunctionCall { name, args } => {
                let upper = name.to_uppercase();
                passed_on(&upper, args.len()).is_none()
                    && !["PREV", "SWITCH", "LET", "XLOOKUP"].contains(&upper.as_str())
            },
            _ => false,
        }
    }

    /// How the formula's own result is written: a text result is text, what
    /// the column holds aside (an imported column of `LEFT(d, 4)` may hold
    /// `2026` as a number). Only what the formula reads is judged.
    pub(super) fn result_read(&self, expr: &Expr, read: Read) -> Read {
        if self.is_text_value(expr) {
            Read::Text
        } else {
            read
        }
    }

    /// How a column handed to a function as it is (the array of `INDEX` or
    /// `MATCH`, the range of a criterion, a column to count) is written: the
    /// cells themselves. Text that may be a date is not read as a number here.
    pub(super) fn handed_on(&self, expr: &Expr) -> Read {
        if matches!(self.kind(expr), Kind::Mixed | Kind::Text) {
            Read::Text
        } else {
            Read::Number
        }
    }

    /// How both sides of `=` or `<>` are read. Forge compares two dates, or a
    /// date and a number, as numbers, and a date with other text as text.
    pub(super) fn equality(&self, left: &Expr, right: &Expr) -> Result<Read, String> {
        let (l, r) = (self.kind(left), self.kind(right));
        if l == Kind::Mixed || r == Kind::Mixed {
            // Text with text in Forge and in the workbook, a date as its
            // text. With a number, Forge finds a date equal to its day
            // number and Excel finds text unequal to any number.
            let other = if l == Kind::Mixed { r } else { l };
            return match other {
                Kind::Number | Kind::Unknown => {
                    Err("text that may be a date is compared with a number".to_string())
                },
                _ => Ok(Read::Text),
            };
        }
        if !l.is_date() && !r.is_date() {
            return Ok(Read::Text);
        }
        let other = if l.is_date() { r } else { l };
        match other {
            // Two dates written in the formula compare as the text they are
            Kind::Written if l == r => Ok(Read::Text),
            Kind::Date | Kind::Written | Kind::Number => Ok(Read::Number),
            // Mixed is settled above: the arm keeps the match whole
            Kind::Text | Kind::Mixed => Ok(Read::Text),
            Kind::Unknown => {
                Err("a date is compared with a value that may be text or a number".to_string())
            },
        }
    }

    /// A value looked up in an array (`MATCH`, `XLOOKUP`): how the value is
    /// read, given what the array holds.
    pub(super) fn looked_up(&self, value: &Expr, array: &Expr) -> Result<Read, String> {
        let (v, a) = (self.kind(value), self.kind(array));
        match (v, a) {
            // Text that may be a date matches a number in Forge (a date is
            // its day number) and never in Excel
            (Kind::Number | Kind::Unknown, Kind::Mixed)
            | (Kind::Mixed, Kind::Number | Kind::Unknown) => Err(
                "text that may be a date is looked up among numbers, or a number in it".to_string(),
            ),
            (Kind::Date | Kind::Written, Kind::Date | Kind::Number) => Ok(Read::Number),
            (Kind::Date | Kind::Written, Kind::Text | Kind::Mixed) | (Kind::Number, _) => {
                Ok(Read::Text)
            },
            (Kind::Text | Kind::Unknown | Kind::Mixed, Kind::Date) => {
                Err("a value that is not a date is looked up in a column of dates".to_string())
            },
            (Kind::Date, _) => Err("a date is looked up in values Forge cannot type".to_string()),
            _ => Ok(Read::Text),
        }
    }

    /// Whether a criterion is text written in the formula that is compared as
    /// text and nothing else: no operator, not a number, not a date.
    pub(super) fn plain_text(criterion: &Expr) -> bool {
        let Expr::Text(text) = criterion else {
            return false;
        };
        let text = text.trim();
        !text.starts_with(['=', '<', '>'])
            && text.parse::<f64>().is_err()
            && crate::core::array_calculator::evaluator::date_serial(text).is_none()
    }

    /// A criterion with a date in it (`"2026-02-28"`, `">=2026-02-01"`,
    /// `">=" & start`), with the date as its day number. `None` for any other
    /// criterion.
    pub(super) fn criterion(&mut self, expr: &Expr, whole: bool) -> Result<Option<Expr>, String> {
        const OPERATORS: [&str; 6] = [">=", "<=", "<>", ">", "<", "="];
        match expr {
            Expr::Text(text) => {
                if let Some(date) = as_number(text) {
                    return Ok(Some(date));
                }
                let found = OPERATORS.iter().find_map(|op| {
                    let date = as_number(text.strip_prefix(op)?.trim())?;
                    Some(binary("&", Expr::Text((*op).to_string()), date))
                });
                Ok(found)
            },
            Expr::BinaryOp { op, left, right } if op == "&" => {
                let is_operator =
                    matches!(&**left, Expr::Text(text) if OPERATORS.contains(&text.trim()));
                if is_operator && self.kind(right).is_date() {
                    let date = self.write(right, Read::Number, whole)?;
                    return Ok(Some(binary("&", (**left).clone(), date)));
                }
                Ok(None)
            },
            _ => Ok(None),
        }
    }

    #[allow(clippy::too_many_lines)] // one rule per family of functions
    pub(super) fn write_call(
        &mut self,
        name: &str,
        args: &[Expr],
        read: Read,
        whole: bool,
    ) -> Result<Expr, String> {
        let upper = name.to_uppercase();
        let upper = upper.as_str();
        let whole_arg = |i: usize| whole || is_whole_column(upper, args.len(), i);
        let kind = self.kind_of_call(upper, args);

        // IF and its like pass a value on: every value is written the way the
        // reader of the result reads it.
        if let Some(values) = passed_on(upper, args.len()) {
            let inner = read;
            let written = args
                .iter()
                .enumerate()
                .map(|(i, arg)| {
                    let read = if values.contains(&i) {
                        inner
                    } else {
                        Read::Text
                    };
                    self.write(arg, read, whole)
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(call(name, written));
        }

        let mut written = Vec::with_capacity(args.len());
        match upper {
            "PREV" => {
                let inner = read;
                for (i, arg) in args.iter().enumerate() {
                    let read = if i < 2 { inner } else { Read::Number };
                    written.push(self.write(arg, read, whole)?);
                }
            },
            "SWITCH" => {
                let results = switch_results(args.len());
                let compared: Vec<Kind> = (0..args.len())
                    .filter(|i| !results.contains(i))
                    .map(|i| self.kind(&args[i]))
                    .collect();
                let compare = if compared.contains(&Kind::Mixed) {
                    // Text with text; with a number Forge may find a date
                    // equal to its day number
                    if compared
                        .iter()
                        .all(|k| matches!(k, Kind::Mixed | Kind::Text | Kind::Written))
                    {
                        Read::Text
                    } else {
                        return Err(
                            "SWITCH compares text that may be a date with a number".to_string()
                        );
                    }
                } else if compared.iter().any(|k| k.is_date()) {
                    if compared.iter().all(|k| k.is_date() || *k == Kind::Number) {
                        Read::Number
                    } else {
                        return Err(
                            "SWITCH compares a date with values that are not dates".to_string()
                        );
                    }
                } else {
                    Read::Text
                };
                for (i, arg) in args.iter().enumerate() {
                    let read = if results.contains(&i) { read } else { compare };
                    written.push(self.write(arg, read, whole)?);
                }
            },
            "INDEX" => {
                for (i, arg) in args.iter().enumerate() {
                    let read = if i == 0 {
                        self.handed_on(arg)
                    } else {
                        Read::Number
                    };
                    written.push(self.write(arg, read, whole_arg(i))?);
                }
                let result = call(name, written);
                return Ok(if kind == Kind::Date && read == Read::Text {
                    as_text(&result)
                } else {
                    result
                });
            },
            "MATCH" | "XLOOKUP" => {
                let [value, array, ..] = args else {
                    return Err(format!("{upper} needs a value and an array"));
                };
                let value_read = self.looked_up(value, array)?;
                written.push(self.write(value, value_read, whole_arg(0))?);
                let array_read = self.handed_on(array);
                written.push(self.write(array, array_read, whole_arg(1))?);
                if upper == "XLOOKUP" {
                    let found = args.get(2).map_or(Kind::Unknown, |a| self.kind(a));
                    if found.is_date() && kind != Kind::Date {
                        return Err(
                            "XLOOKUP gives a date, or a value that is not a date when none is found"
                                .to_string(),
                        );
                    }
                }
                // What is found, and what is given when nothing is: dates as
                // day numbers if both are dates, otherwise as they are
                let rest = if kind == Kind::Date {
                    Read::Number
                } else {
                    Read::Text
                };
                for (i, arg) in args.iter().enumerate().skip(2) {
                    written.push(self.write(arg, rest, whole_arg(i))?);
                }
                let result = call(name, written);
                return Ok(
                    if upper == "XLOOKUP" && kind == Kind::Date && read == Read::Text {
                        as_text(&result)
                    } else {
                        result
                    },
                );
            },
            "TEXT" => {
                // TEXT formats the day number of a date, in Forge as in Excel
                for (i, arg) in args.iter().enumerate() {
                    let read = if i == 0 { Read::Number } else { Read::Text };
                    written.push(self.write(arg, read, whole)?);
                }
            },
            "LET" => {
                // A name bound to a date is the day number; where the body
                // reads it as text it is written as text there
                let before = self.locals.len();
                let last = args.len().saturating_sub(1);
                for (i, arg) in args.iter().enumerate() {
                    if i < last && i % 2 == 0 {
                        written.push(arg.clone());
                    } else if i < last {
                        let value_kind = self.kind(arg);
                        let read = if value_kind.is_date() {
                            Read::Number
                        } else {
                            Read::Text
                        };
                        written.push(self.write(arg, read, whole)?);
                        if let Expr::Reference(bound) = &args[i - 1] {
                            let kind = if value_kind.is_date() {
                                Kind::Date
                            } else {
                                value_kind
                            };
                            self.locals.push((Self::word(bound), kind));
                        }
                    } else {
                        written.push(self.write(arg, read, whole)?);
                    }
                }
                self.locals.truncate(before);
            },
            _ if CRITERIA.iter().any(|(n, _)| *n == upper) => {
                let first = CRITERIA
                    .iter()
                    .find(|(n, _)| *n == upper)
                    .map_or(0, |(_, first)| *first);
                // SUMIF(range, criterion, to add up): the third is added up
                let added_up = if first == 0 { 2 } else { 0 };
                let has_values = !upper.starts_with("COUNT");
                for (i, arg) in args.iter().enumerate() {
                    // Ranges are at first, first + 2, ...; a criterion follows each
                    let is_criterion = i > first && (i - first) % 2 == 1;
                    let read = if is_criterion {
                        let range = self.kind(&args[i - 1]);
                        if range == Kind::Mixed && !Self::plain_text(arg) {
                            return Err(format!(
                                "{upper}: the criterion is for text that may be a date, which Forge compares as a number"
                            ));
                        }
                        // Over dates or numbers, a criterion's date is its day
                        // number: Excel would read `">=2026-02-01"` in the
                        // language it runs in
                        if matches!(range, Kind::Date | Kind::Number) {
                            if let Some(criterion) = self.criterion(arg, whole_arg(i))? {
                                written.push(criterion);
                                continue;
                            }
                        }
                        match (self.kind(arg), range) {
                            (Kind::Date, Kind::Date | Kind::Number) => Read::Number,
                            (Kind::Date, Kind::Unknown | Kind::Written) => {
                                return Err(format!(
                                    "{upper}: a date is the criterion for values Forge cannot type"
                                ))
                            },
                            // Excel would read the text in the language it runs in
                            (Kind::Mixed, Kind::Date | Kind::Number | Kind::Unknown) => {
                                return Err(format!(
                                    "{upper}: text that may be a date is the criterion for dates or numbers"
                                ))
                            },
                            // A date as the criterion for text is that text
                            _ => Read::Text,
                        }
                    } else if has_values && i == added_up {
                        Read::Number
                    } else {
                        self.handed_on(arg)
                    };
                    written.push(self.write(arg, read, whole_arg(i))?);
                }
            },
            _ if READ_NUMBERS.contains(&upper) => {
                for (i, arg) in args.iter().enumerate() {
                    written.push(self.write(arg, Read::Number, whole_arg(i))?);
                }
            },
            _ if ANY_VALUES.contains(&upper) => {
                for (i, arg) in args.iter().enumerate() {
                    let read = self.handed_on(arg);
                    written.push(self.write(arg, read, whole_arg(i))?);
                }
            },
            "SUMPRODUCT" => {
                for (i, arg) in args.iter().enumerate() {
                    if self.kind(arg) == Kind::Date {
                        return Err(
                            "SUMPRODUCT reads a date as 0 in Forge, and as its day number in Excel"
                                .to_string(),
                        );
                    }
                    written.push(self.write(arg, Read::Number, whole_arg(i))?);
                }
            },
            "LAMBDA" => {
                if args.iter().any(|a| self.touches_dates(a)) {
                    return Err("a date inside a LAMBDA".to_string());
                }
                written.extend(args.iter().cloned());
            },
            // What is found is the table's own value: a date in it would be a
            // day number in the workbook and text in Forge, and a date looked
            // up would be compared as one or the other. No rule yet.
            "VLOOKUP" | "HLOOKUP" => {
                if args.iter().any(|a| self.touches_dates(a)) {
                    return Err(format!("a date in {upper}"));
                }
                written.extend(args.iter().cloned());
            },
            _ => {
                // Any other function reads its arguments as Forge holds them:
                // a date as its text, which is what the workbook held for it
                // before dates were written as dates.
                for (i, arg) in args.iter().enumerate() {
                    written.push(self.write(arg, Read::Text, whole_arg(i))?);
                }
            },
        }
        let result = call(name, written);
        // EDATE and its like give a day number in Excel: where the reader
        // wants Forge's text, it is written as text
        Ok(if GIVE_A_DATE.contains(&upper) && read == Read::Text {
            as_text(&result)
        } else {
            result
        })
    }
}

pub(super) fn call(name: &str, args: Vec<Expr>) -> Expr {
    Expr::FunctionCall {
        name: name.to_string(),
        args,
    }
}

pub(super) fn binary(op: &str, left: Expr, right: Expr) -> Expr {
    Expr::BinaryOp {
        op: op.to_string(),
        left: Box::new(left),
        right: Box::new(right),
    }
}

/// The date text Forge holds, from the day number the workbook holds.
pub(super) fn as_text(date: &Expr) -> Expr {
    let part = |function: &str, times: f64| {
        binary("*", call(function, vec![date.clone()]), Expr::Number(times))
    };
    call(
        "TEXT",
        vec![
            binary(
                "+",
                binary("+", part("YEAR", 10000.0), part("MONTH", 100.0)),
                call("DAY", vec![date.clone()]),
            ),
            Expr::Text("0000-00-00".to_string()),
        ],
    )
}

/// `"2026-01-31"` as `DATE(2026, 1, 31)`.
pub(super) fn as_number(text: &str) -> Option<Expr> {
    let (year, month, day) = excel_date(text)?;
    Some(call(
        "DATE",
        vec![
            Expr::Number(f64::from(year)),
            Expr::Number(f64::from(month)),
            Expr::Number(f64::from(day)),
        ],
    ))
}

/// `expr` with every part replaced by what `change` gives for it, parts
/// before the whole.
pub(super) fn map(expr: &Expr, change: &impl Fn(Expr) -> Expr) -> Expr {
    let all = |items: &[Expr]| items.iter().map(|e| map(e, change)).collect::<Vec<_>>();
    let inner = match expr {
        Expr::Number(_)
        | Expr::Text(_)
        | Expr::Boolean(_)
        | Expr::Array { .. }
        | Expr::Reference(_) => expr.clone(),
        Expr::ArrayIndex { array, index } => Expr::ArrayIndex {
            array: Box::new(map(array, change)),
            index: Box::new(map(index, change)),
        },
        Expr::FunctionCall { name, args } => Expr::FunctionCall {
            name: name.clone(),
            args: all(args),
        },
        Expr::CallResult { callable, args } => Expr::CallResult {
            callable: Box::new(map(callable, change)),
            args: all(args),
        },
        Expr::BinaryOp { op, left, right } => Expr::BinaryOp {
            op: op.clone(),
            left: Box::new(map(left, change)),
            right: Box::new(map(right, change)),
        },
        Expr::UnaryOp { op, operand } => Expr::UnaryOp {
            op: op.clone(),
            operand: Box::new(map(operand, change)),
        },
        Expr::Range { start, end } => Expr::Range {
            start: Box::new(map(start, change)),
            end: Box::new(map(end, change)),
        },
    };
    change(inner)
}

pub(super) fn is_call<'e>(expr: &'e Expr, function: &str) -> Option<&'e [Expr]> {
    match expr {
        Expr::FunctionCall { name, args } if name.eq_ignore_ascii_case(function) => Some(args),
        _ => None,
    }
}

/// The date in `TEXT(YEAR(d)*10000+MONTH(d)*100+DAY(d), "0000-00-00")`, the
/// text of a date as [`as_text`] writes it.
pub(super) fn date_of_text(expr: &Expr) -> Option<Expr> {
    let [number, Expr::Text(format)] = is_call(expr, "TEXT")? else {
        return None;
    };
    if format != "0000-00-00" {
        return None;
    }
    let part = |expr: &Expr, function: &str, times: f64| -> Option<Expr> {
        let Expr::BinaryOp { op, left, right } = expr else {
            return None;
        };
        let [date] = is_call(left, function)? else {
            return None;
        };
        (op == "*" && **right == Expr::Number(times)).then(|| date.clone())
    };
    let Expr::BinaryOp { op, left, right } = number else {
        return None;
    };
    let Expr::BinaryOp {
        op: inner_op,
        left: year,
        right: month,
    } = &**left
    else {
        return None;
    };
    let [day] = is_call(right, "DAY")? else {
        return None;
    };
    let year = part(year, "YEAR", 10000.0)?;
    let month = part(month, "MONTH", 100.0)?;
    (op == "+" && inner_op == "+" && year == month && year == *day).then_some(year)
}

/// The date in `DATE(2026, 1, 31)`, as Forge writes it: `"2026-01-31"`.
pub(super) fn date_of_numbers(expr: &Expr) -> Option<Expr> {
    let [Expr::Number(year), Expr::Number(month), Expr::Number(day)] = is_call(expr, "DATE")?
    else {
        return None;
    };
    let whole = |n: &f64| n.fract() == 0.0 && (0.0..10000.0).contains(n);
    if !(whole(year) && whole(month) && whole(day)) {
        return None;
    }
    let text = format!("{year:04.0}-{month:02.0}-{day:02.0}");
    excel_date(&text).map(|_| Expr::Text(text))
}

pub(super) fn upper_names(expr: &Expr) -> Expr {
    map(expr, &|e| match e {
        Expr::FunctionCall { name, args } => Expr::FunctionCall {
            name: name.to_uppercase(),
            args,
        },
        other => other,
    })
}
