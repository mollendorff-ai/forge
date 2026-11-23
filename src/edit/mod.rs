//! Forge writes a model's text (ADR-075).
//!
//! [`edit_files`] takes a model's files and a list of [`Change`]s, each named
//! and typed, and answers the new text of each file it changed. A change is
//! kept only when Forge reads the model back valid, with that change and
//! nothing else (decision 3); otherwise no file changes, and the message says
//! why. Every byte outside a change stays as it was (decision 4): `text`
//! finds the place in a lossless YAML tree and writes in the file's style.
//! The command line (`forge edit`), the MCP server (`forge_edit`) and Forge
//! Studio (forge-wasm's `edit`) share it.

mod check;
mod games;
mod insert;
mod rename;
pub mod templates;
pub mod text;
mod uncertain;

use crate::error::{ForgeError, ForgeResult};
use crate::types::ParsedModel;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

#[cfg(feature = "cli")]
use rmcp::schemars;

/// What a change does
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// An input's value (`value`)
    SetValue,
    /// A formula, a scalar's or a table column's (`formula`)
    SetFormula,
    /// A new input (`value`, and `unit`), formula (`formula`, and `unit`) or
    /// table column (`formula`), at the end of its group; at the top level,
    /// after the name `after`
    Add,
    /// An input, a formula or a column, with the lines under it
    Delete,
    /// A scalar, a group, a table, a column, a game, a player or a strategy
    /// called `new_name`, and every name that reads it renamed, in every file
    /// of the model: formulas, scenarios, expectations, a simulation's
    /// outputs, a tornado's inputs, a game's results
    Rename,
    /// An input becomes a distribution, its value kept as the plan
    /// (`formula`, a distribution; a triangle around the value when none)
    MakeUncertain,
    /// A distribution becomes an input (`value`: the figure to keep; the
    /// `value:` beside the formula, or else the distribution's plan figure)
    MakeCertain,
    /// A game's strategy, for every player (`sealed_bids.strategies.bid_50`) or
    /// for one (`inspection.strategies.human.ignore`), its cells 0
    AddStrategy,
    /// A game's strategy, every player's of that name or one player's, with
    /// its cells
    DeleteStrategy,
    /// A game's player (`price_war.players.shop`), last, with the last
    /// player's strategies and payoffs of 0; a tournament's
    /// (`rivalry.players.copycat`) with the habit it `plays`
    AddPlayer,
    /// A game's player, and the payoffs of its strategies but the first; or
    /// a tournament's
    DeletePlayer,
    /// A template's block, a game, added to `games:` (`path`: the template,
    /// from the `templates` tool; `new_name`: the name its game gets, or a
    /// free one)
    Insert,
}

impl Op {
    /// As a change names it
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SetValue => "set_value",
            Self::SetFormula => "set_formula",
            Self::Add => "add",
            Self::Delete => "delete",
            Self::Rename => "rename",
            Self::MakeUncertain => "make_uncertain",
            Self::MakeCertain => "make_certain",
            Self::AddStrategy => "add_strategy",
            Self::DeleteStrategy => "delete_strategy",
            Self::AddPlayer => "add_player",
            Self::DeletePlayer => "delete_player",
            Self::Insert => "insert",
        }
    }
}

/// A value to write: a number, or its text as YAML holds it (`1e-5`,
/// `0x10`, a date `2026-01-31`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
#[serde(untagged)]
pub enum Value {
    Number(f64),
    Text(String),
}

/// One change to a model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "cli", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Change {
    /// `set_value`, `set_formula`, `add`, `delete`, `rename`,
    /// `make_uncertain`, `make_certain`, `add_strategy`, `delete_strategy`,
    /// `add_player` or `delete_player`
    pub op: Op,
    /// The name as formulas read it: `price`, `year.price`, a table's column
    /// `months.sales`, or `@costs.rent` in the file included as `costs`; for
    /// `rename`, also a game `price_war`, its player `price_war.players.lune`
    /// (a tournament's too), and its strategy `price_war.strategies.hold`, or
    /// one player's `inspection.strategies.human.audit`; for `add_strategy`
    /// and `delete_strategy` a strategy so named, and for `add_player` and
    /// `delete_player` a player; for `insert` the template
    pub path: String,
    /// The input's value (`set_value`, `add`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// The formula, starting with `=` (`set_formula`, `add`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
    /// The unit of a new input or formula (`add`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// The top-level name a new top-level name follows (`add`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// The name it gets, one word: `cost` for `year.price` (`rename`), or a template's game (`insert`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_name: Option<String>,
    /// The habit a tournament's new player plays: `tit_for_tat`,
    /// `always_defect`, ... (`add_player`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plays: Option<String>,
}

/// The model's files after each change in turn.
///
/// Answers the new text of each file that changed, by its name in `files`.
/// `main` is the model; the others are the files it includes, by the path
/// `_includes` gives them.
///
/// # Errors
///
/// Returns why, naming the change, when a change cannot be made, or Forge
/// does not read the model back valid with exactly that change. Then no file
/// is changed.
pub fn edit_files<S: BuildHasher>(
    files: &HashMap<String, String, S>,
    main: &str,
    changes: &[Change],
) -> ForgeResult<BTreeMap<String, String>> {
    let mut now: HashMap<String, String> =
        files.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    for change in changes {
        apply(&mut now, main, change).map_err(|why| {
            ForgeError::Validation(format!(
                "{} {}: {why}. No file was changed",
                change.op.name(),
                change.path
            ))
        })?;
    }
    Ok(now
        .into_iter()
        .filter(|(name, text)| files.get(name) != Some(text))
        .collect())
}

/// What a name is in the model
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Input,
    Formula,
    /// A table's column with a formula
    Column,
    /// A table's column of values
    Values,
    Absent,
}

/// Where a change goes: the file, the model read from it, and the name in it
struct Target<'m> {
    file: String,
    model: &'m ParsedModel,
    name: String,
    /// The include aliases on the way, from the main model
    aliases: Vec<String>,
}

impl Target<'_> {
    fn kind(&self) -> Kind {
        if let Some(var) = self.model.scalars.get(&self.name) {
            return if var.formula.is_some() {
                Kind::Formula
            } else {
                Kind::Input
            };
        }
        let column = self.name.rsplit_once('.').and_then(|(table, column)| {
            let table = self.model.tables.get(table)?;
            Some(if table.row_formulas.contains_key(column) {
                Kind::Column
            } else if table.columns.contains_key(column) {
                Kind::Values
            } else {
                Kind::Absent
            })
        });
        column.unwrap_or(Kind::Absent)
    }
}

/// The file and model of `path`, following `@alias.` into included files
fn locate<'m>(model: &'m ParsedModel, main: &str, path: &str) -> Result<Target<'m>, String> {
    let mut target = Target {
        file: main.to_string(),
        model,
        name: path.to_string(),
        aliases: Vec::new(),
    };
    while let Some(rest) = target.name.strip_prefix('@') {
        let (alias, name) = rest
            .split_once('.')
            .ok_or("a name in an included file is written @alias.name")?;
        let included = target
            .model
            .resolved_includes
            .get(alias)
            .ok_or_else(|| format!("the model includes no file as {alias}"))?;
        target.aliases.push(alias.to_string());
        target.file = included.resolved_path.to_string_lossy().into_owned();
        target.model = &included.model;
        target.name = name.to_string();
    }
    Ok(target)
}

/// One change, kept in `files` only when Forge agrees
fn apply(files: &mut HashMap<String, String>, main: &str, change: &Change) -> Result<(), String> {
    let before =
        check::read(files, main).map_err(|e| format!("Forge cannot read the model: {e}"))?;
    fields(change)?;
    let target = locate(&before, main, &change.path)?;
    if change.op == Op::Insert {
        return insert::apply(files, main, &before, change);
    }
    if change.op == Op::Rename {
        return rename::apply(files, main, &before, &target, change);
    }
    if matches!(change.op, Op::MakeUncertain | Op::MakeCertain) {
        return uncertain::apply(files, main, &before, &target, change);
    }
    if matches!(
        change.op,
        Op::AddStrategy | Op::DeleteStrategy | Op::AddPlayer | Op::DeletePlayer
    ) {
        return games::apply(files, main, &before, &target, change);
    }
    let kind = target.kind();
    let text = write(&files[&target.file], &target, kind, change)
        .map_err(|why| format!("{why} ({})", target.file))?;
    let mut changed = files.clone();
    changed.insert(target.file.clone(), text);
    let after = check::read(&changed, main)
        .and_then(|after| check::valid(&after).map(|()| after))
        .map_err(|e| format!("Forge refuses the model with this change: {e}"))?;
    check::only(&before, &after, &target, kind, change)?;
    *files = changed;
    Ok(())
}

/// Whether the change gives only the fields its op takes
fn fields(change: &Change) -> Result<(), String> {
    let given = [
        ("value", change.value.is_some()),
        ("formula", change.formula.is_some()),
        ("unit", change.unit.is_some()),
        ("after", change.after.is_some()),
        ("new_name", change.new_name.is_some()),
        ("plays", change.plays.is_some()),
    ];
    let takes: &[&str] = match change.op {
        Op::SetValue | Op::MakeCertain => &["value"],
        Op::SetFormula | Op::MakeUncertain => &["formula"],
        Op::Add => &["value", "formula", "unit", "after"],
        Op::Delete | Op::AddStrategy | Op::DeleteStrategy | Op::DeletePlayer => &[],
        Op::Rename | Op::Insert => &["new_name"],
        Op::AddPlayer => &["plays"],
    };
    given
        .iter()
        .find(|(f, g)| *g && !takes.contains(f))
        .map_or(Ok(()), |(field, _)| {
            Err(format!("{} does not take `{field}`", change.op.name()))
        })
}

/// The new text of the target's file, before Forge reads it back
fn write(text: &str, target: &Target<'_>, kind: Kind, change: &Change) -> Result<String, String> {
    let names: Vec<&str> = target.name.split('.').collect();
    match (change.op, kind) {
        (Op::SetValue, Kind::Input) => {
            text::set_value(text, &names, &value_text(change.value.as_ref())?)
        },
        (Op::SetFormula, Kind::Formula | Kind::Column) => {
            text::set_formula(text, &names, formula(change.formula.as_deref())?)
        },
        (Op::Add, Kind::Absent) => add(text, target, &names, change),
        (Op::Delete, k) if k != Kind::Absent => text::delete(text, &names),
        (Op::Add, _) => Err("it is already in the model".into()),
        (_, Kind::Absent) => Err("it is not in the model".into()),
        (_, Kind::Values) => {
            Err("it is a table's column of values, which edit does not change yet".into())
        },
        (Op::SetValue, _) => {
            Err("it is a formula, not an input: change it with set_formula".into())
        },
        _ => Err("it is an input, not a formula: change it with set_value".into()),
    }
}

/// `add`: which of the three it is, from the change and the group it goes in
fn add(text: &str, target: &Target<'_>, names: &[&str], change: &Change) -> Result<String, String> {
    let name = names.last().copied().unwrap_or_default();
    if !is_name(name) {
        return Err(format!(
            "{name} is not a name: a letter or _, then letters, digits and _"
        ));
    }
    if names.len() > 2 {
        return Err("edit adds a name to a group at the top level, or to the top level".into());
    }
    if names.len() == 2 && change.after.is_some() {
        return Err(
            "`after` is for a name at the top level; in a group, a name goes at its end".into(),
        );
    }
    let unit = change.unit.as_deref();
    let in_table = names.len() == 2 && target.model.tables.contains_key(names[0]);
    let new = match (&change.value, &change.formula) {
        (Some(_), Some(_)) => return Err("add takes `value` or `formula`, not both".into()),
        (None, None) => return Err("add needs `value` (an input) or `formula`".into()),
        (Some(_), None) if in_table => {
            return Err("a table's new column is added with a formula".into())
        },
        (None, Some(_)) if in_table && unit.is_some() => {
            return Err("edit writes no unit for a table's column".into())
        },
        (Some(_), None) => {
            return text::add(
                text,
                names,
                text::New::Input {
                    value: &value_text(change.value.as_ref())?,
                    unit,
                },
                change.after.as_deref(),
            )
        },
        (None, Some(f)) if in_table => text::New::Column {
            formula: formula(Some(f))?,
        },
        (None, Some(f)) => text::New::Formula {
            formula: formula(Some(f))?,
            unit,
        },
    };
    text::add(text, names, new, change.after.as_deref())
}

/// A name Forge reads in a formula: a letter (of any alphabet) or `_`, then
/// letters, digits and `_`
fn is_name(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// A formula as edit writes it: one line, starting with `=`
fn formula(formula: Option<&str>) -> Result<&str, String> {
    let formula = formula.ok_or("it needs `formula`")?;
    if !formula.starts_with('=') {
        return Err(format!(
            "{formula} is not a formula: a formula starts with ="
        ));
    }
    if formula.contains(['\n', '\r']) {
        return Err("a formula is written on one line".into());
    }
    Ok(formula)
}

/// A value as YAML text: a number as JavaScript writes it, or text made of
/// the characters of a number or a date only, so that it cannot be read as
/// anything else (a comment, a second key)
fn value_text(value: Option<&Value>) -> Result<String, String> {
    match value.ok_or("it needs `value`")? {
        Value::Number(n) if n.is_finite() => Ok(js_number(*n)),
        Value::Number(n) => Err(format!("{n} is not a value: a value is a finite number")),
        Value::Text(text)
            if !text.is_empty()
                && text
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() || "xXoObB.+-".contains(c)) =>
        {
            Ok(text.clone())
        },
        Value::Text(text) => Err(format!(
            "{text:?} is not written as a number or a date (2026-01-31)"
        )),
    }
}

/// A number as JavaScript writes it (`String(n)`).
///
/// ECMA-262's `Number::toString`: the shortest digits that read back to `n`,
/// without an exponent from 1e-6 to below 1e21. Forge Studio sends numbers,
/// so a change from Studio and one from an agent give the same bytes.
///
/// # Panics
///
/// Never: Rust writes every `f64` in scientific form with an exponent.
#[must_use]
pub fn js_number(n: f64) -> String {
    if n == 0.0 {
        return "0".into();
    }
    let scientific = format!("{n:e}");
    let (digits, exponent) = scientific
        .split_once('e')
        .expect("a number in scientific form has an exponent");
    let exponent: i32 = exponent.parse().expect("the exponent is a whole number");
    if (-6..21).contains(&exponent) {
        format!("{n}")
    } else {
        format!("{digits}e{}{exponent}", if exponent > 0 { "+" } else { "" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    const MODEL: &str = "_forge_version: \"5.1.0\"
_includes:
  - file: costs.yaml
    as: costs
rate: 0.05   # a rate
year:
  price: 16   # dollars
  units: 100
  revenue:
    formula: \"=year.price * year.units\"
    unit: CAD
  net:
    formula: \"=year.revenue - @costs.rent\"
months:
  month: [1, 2, 3]
  sales: \"=month * 10\"
";
    const COSTS: &str = "_forge_version: \"5.1.0\"\nrent: 900   # a month\nspare: 1\n";

    fn files() -> HashMap<String, String> {
        HashMap::from([
            ("model.yaml".to_string(), MODEL.to_string()),
            ("costs.yaml".to_string(), COSTS.to_string()),
        ])
    }

    fn change(op: Op, path: &str) -> Change {
        Change {
            op,
            path: path.into(),
            value: None,
            formula: None,
            unit: None,
            after: None,
            new_name: None,
            plays: None,
        }
    }

    fn value(op: Op, path: &str, v: Value) -> Change {
        Change {
            value: Some(v),
            ..change(op, path)
        }
    }

    fn formula_of(op: Op, path: &str, f: &str) -> Change {
        Change {
            formula: Some(f.into()),
            ..change(op, path)
        }
    }

    /// The files that changed, or the message
    fn run(changes: &[Change]) -> Result<BTreeMap<String, String>, String> {
        edit_files(&files(), "model.yaml", changes).map_err(|e| e.to_string())
    }

    fn only_model(text: String) -> BTreeMap<String, String> {
        BTreeMap::from([("model.yaml".to_string(), text)])
    }

    #[test]
    fn a_value_and_a_formula_change_alone() {
        assert_eq!(
            run(&[value(Op::SetValue, "year.price", Value::Number(17.5))]),
            Ok(only_model(MODEL.replace("price: 16", "price: 17.5")))
        );
        assert_eq!(
            run(&[value(Op::SetValue, "rate", Value::Text("5e-2".into()))]),
            Ok(only_model(MODEL.replace("rate: 0.05", "rate: 5e-2")))
        );
        assert_eq!(
            run(&[formula_of(
                Op::SetFormula,
                "year.revenue",
                "=year.price * year.units * 2"
            )]),
            Ok(only_model(
                MODEL.replace("year.units\"", "year.units * 2\"")
            ))
        );
        assert_eq!(
            run(&[formula_of(Op::SetFormula, "months.sales", "=month * 12")]),
            Ok(only_model(MODEL.replace("month * 10", "month * 12")))
        );
    }

    #[test]
    fn a_name_in_an_included_file_is_changed_there() {
        assert_eq!(
            run(&[value(Op::SetValue, "@costs.rent", Value::Number(950.0))]),
            Ok(BTreeMap::from([(
                "costs.yaml".to_string(),
                COSTS.replace("rent: 900", "rent: 950")
            )]))
        );
        let added = Change {
            after: Some("rent".into()),
            ..value(Op::Add, "@costs.fee", Value::Number(5.0))
        };
        assert_eq!(
            run(&[added]),
            Ok(BTreeMap::from([(
                "costs.yaml".to_string(),
                COSTS.replace("month\n", "month\nfee: 5\n")
            )]))
        );
    }

    #[test]
    fn each_kind_of_name_is_added_and_deleted_again() {
        let tax = Change {
            unit: Some("ratio".into()),
            ..value(Op::Add, "year.tax", Value::Number(0.15))
        };
        let share = Change {
            unit: Some("CAD".into()),
            ..formula_of(Op::Add, "year.share", "=year.net / 2")
        };
        let top = Change {
            after: Some("rate".into()),
            ..formula_of(Op::Add, "twice", "=rate * 2")
        };
        let cases = [
            (
                tax,
                "year.tax",
                "    unit: CAD\n",
                "  tax:\n    value: 0.15\n    unit: ratio\n",
                "formula: \"=year.revenue - @costs.rent\"\n",
            ),
            (
                share,
                "year.share",
                "",
                "  share:\n    formula: \"=year.net / 2\"\n    unit: CAD\n",
                "formula: \"=year.revenue - @costs.rent\"\n",
            ),
            (
                formula_of(Op::Add, "months.double", "=sales * 2"),
                "months.double",
                "",
                "  double: \"=sales * 2\"\n",
                "  sales: \"=month * 10\"\n",
            ),
            (
                top,
                "twice",
                "",
                "twice:\n  formula: \"=rate * 2\"\n",
                "rate: 0.05   # a rate\n",
            ),
        ];
        for (add, path, _, lines, after_line) in cases {
            let want = MODEL.replacen(after_line, &format!("{after_line}{lines}"), 1);
            let added = edit_files(&files(), "model.yaml", &[add]).unwrap();
            assert_eq!(added, only_model(want.clone()), "{path}");
            let mut now = files();
            now.insert("model.yaml".into(), want);
            let back = edit_files(&now, "model.yaml", &[change(Op::Delete, path)]).unwrap();
            assert_eq!(back, only_model(MODEL.to_string()), "{path}");
        }
    }

    #[test]
    fn several_changes_are_made_in_turn_or_none_is() {
        let both = [
            value(Op::SetValue, "year.price", Value::Number(20.0)),
            value(Op::SetValue, "@costs.rent", Value::Number(1000.0)),
            change(Op::Delete, "@costs.spare"),
        ];
        assert_eq!(
            run(&both),
            Ok(BTreeMap::from([
                (
                    "costs.yaml".to_string(),
                    "_forge_version: \"5.1.0\"\nrent: 1000   # a month\n".to_string()
                ),
                (
                    "model.yaml".to_string(),
                    MODEL.replace("price: 16", "price: 20")
                ),
            ]))
        );
        let second_fails = [
            value(Op::SetValue, "year.price", Value::Number(20.0)),
            change(Op::Delete, "year.nothing"),
        ];
        assert_eq!(
            run(&second_fails),
            Err("Validation error: delete year.nothing: it is not in the model (model.yaml). No file was changed".to_string())
        );
        // A value set to what it is already changes no file
        assert_eq!(
            run(&[value(Op::SetValue, "year.units", Value::Number(100.0))]),
            Ok(BTreeMap::new())
        );
    }

    #[test]
    fn a_change_forge_refuses_changes_nothing() {
        let refused = |c: Change, why: &str| {
            let message = run(&[c]).unwrap_err();
            assert!(message.starts_with(why), "{message}");
            assert!(message.ends_with(". No file was changed"), "{message}");
        };
        refused(
            change(Op::Delete, "year.units"),
            "Validation error: delete year.units: Forge refuses the model with this change: \
             Formula evaluation error: 1 formula failed:\n  - year.revenue: Unknown name: year.units.",
        );
        refused(
            formula_of(Op::SetFormula, "year.revenue", "=year.nothing"),
            "Validation error: set_formula year.revenue: Forge refuses the model with this change: \
             Formula evaluation error: 1 formula failed:\n  - year.revenue: Unknown name: year.nothing.",
        );
        // A file that no longer reads as a model
        refused(
            value(Op::SetValue, "rate", Value::Text("--".into())),
            "Validation error: set_value rate: Forge refuses the model with this change: ",
        );
    }

    #[test]
    fn a_change_that_does_not_fit_the_name_is_refused_with_the_reason() {
        let refused = |c: Change, why: &str| {
            let message = run(&[c]).unwrap_err();
            assert!(message.contains(why), "{message}");
            assert!(message.ends_with(". No file was changed"), "{message}");
        };
        refused(value(Op::SetValue, "year.revenue", Value::Number(1.0)), "set_value year.revenue: it is a formula, not an input: change it with set_formula (model.yaml)");
        refused(
            formula_of(Op::SetFormula, "year.price", "=1"),
            "it is an input, not a formula: change it with set_value",
        );
        refused(
            value(Op::SetValue, "months.month", Value::Number(1.0)),
            "it is a table's column of values, which edit does not change yet",
        );
        refused(
            formula_of(Op::SetFormula, "months.nothing", "=1"),
            "it is not in the model",
        );
        refused(
            value(Op::Add, "year.price", Value::Number(1.0)),
            "it is already in the model",
        );
        refused(change(Op::Delete, "nothing"), "it is not in the model");
        refused(
            Change {
                unit: Some("x".into()),
                ..value(Op::SetValue, "rate", Value::Number(1.0))
            },
            "set_value does not take `unit`",
        );
        refused(
            Change {
                after: Some("rate".into()),
                ..change(Op::Delete, "rate")
            },
            "delete does not take `after`",
        );
        refused(
            value(Op::SetFormula, "rate", Value::Number(1.0)),
            "set_formula does not take `value`",
        );
        refused(change(Op::SetValue, "rate"), "it needs `value`");
        refused(change(Op::SetFormula, "year.revenue"), "it needs `formula`");
        refused(
            formula_of(Op::SetFormula, "year.revenue", "year.price"),
            "year.price is not a formula: a formula starts with =",
        );
        refused(
            formula_of(Op::SetFormula, "year.revenue", "=a\n+ b"),
            "a formula is written on one line",
        );
        refused(
            value(Op::SetValue, "rate", Value::Text("1 # two".into())),
            "\"1 # two\" is not written as a number or a date (2026-01-31)",
        );
        refused(
            value(Op::SetValue, "rate", Value::Text(String::new())),
            "\"\" is not written as a number or a date",
        );
        refused(
            value(Op::SetValue, "rate", Value::Number(f64::INFINITY)),
            "inf is not a value: a value is a finite number",
        );
        refused(
            value(Op::SetValue, "@costs", Value::Number(1.0)),
            "a name in an included file is written @alias.name",
        );
        refused(
            value(Op::SetValue, "@rates.x", Value::Number(1.0)),
            "the model includes no file as rates",
        );
    }

    #[test]
    fn an_addition_is_refused_with_the_reason() {
        let refused = |c: Change, why: &str| {
            let message = run(&[c]).unwrap_err();
            assert!(message.contains(why), "{message}");
        };
        refused(
            value(Op::Add, "year.2x", Value::Number(1.0)),
            "2x is not a name: a letter or _, then letters, digits and _",
        );
        refused(
            value(Op::Add, "year.a-b", Value::Number(1.0)),
            "a-b is not a name",
        );
        refused(
            value(Op::Add, "year.deep.x", Value::Number(1.0)),
            "edit adds a name to a group at the top level, or to the top level",
        );
        refused(
            Change {
                after: Some("rate".into()),
                ..value(Op::Add, "year.x", Value::Number(1.0))
            },
            "`after` is for a name at the top level; in a group, a name goes at its end",
        );
        refused(
            Change {
                formula: Some("=1".into()),
                ..value(Op::Add, "year.x", Value::Number(1.0))
            },
            "add takes `value` or `formula`, not both",
        );
        refused(
            change(Op::Add, "year.x"),
            "add needs `value` (an input) or `formula`",
        );
        refused(
            value(Op::Add, "months.x", Value::Number(1.0)),
            "a table's new column is added with a formula",
        );
        refused(
            Change {
                unit: Some("CAD".into()),
                ..formula_of(Op::Add, "months.x", "=1")
            },
            "edit writes no unit for a table's column",
        );
        refused(formula_of(Op::Add, "months.x", "1"), "1 is not a formula");
        refused(formula_of(Op::Add, "year.x", "x"), "x is not a formula");
        refused(
            value(Op::Add, "year.x", Value::Text("x:".into())),
            "\"x:\" is not written as a number or a date",
        );
        refused(
            value(Op::Add, "x", Value::Number(1.0)),
            "a name at the top level needs `after`: the name it follows (model.yaml)",
        );
    }

    #[test]
    fn names_in_any_alphabet_are_changed_and_added() {
        let text = "_forge_version: \"5.1.0\"\nhypothèses:\n  coût_unitaire: 3   # €\n  quantité: 2\n假设:\n  单价: 5\n  总价:\n    formula: \"=假设.单价 * hypothèses.quantité\"\n";
        let one = HashMap::from([("m.yaml".to_string(), text.to_string())]);
        let changed = edit_files(
            &one,
            "m.yaml",
            &[
                value(Op::SetValue, "hypothèses.coût_unitaire", Value::Number(4.0)),
                formula_of(
                    Op::SetFormula,
                    "假设.总价",
                    "=假设.单价 * hypothèses.coût_unitaire",
                ),
                value(Op::Add, "假设.数量", Value::Number(7.0)),
            ],
        )
        .unwrap();
        assert_eq!(
            changed["m.yaml"],
            text.replace("coût_unitaire: 3", "coût_unitaire: 4")
                .replace("* hypothèses.quantité", "* hypothèses.coût_unitaire")
                + "  数量: 7\n"
        );
    }

    #[test]
    fn a_main_model_that_is_not_among_the_files_is_named() {
        assert_eq!(
            edit_files(&files(), "other.yaml", &[change(Op::Delete, "rate")]).map_err(|e| e.to_string()),
            Err("Validation error: delete rate: Forge cannot read the model: 'main' names other.yaml, which is not among the files. No file was changed".to_string())
        );
    }

    #[test]
    fn a_change_reads_as_json_in_the_tools_form() {
        let read: Vec<Change> = serde_json::from_str(
            r#"[{"op": "set_value", "path": "year.price", "value": 17},
                {"op": "set_value", "path": "rate", "value": "2026-01-31"},
                {"op": "add", "path": "year.x", "formula": "=1", "unit": "CAD"},
                {"op": "delete", "path": "rate"}]"#,
        )
        .unwrap();
        assert_eq!(
            read[0],
            value(Op::SetValue, "year.price", Value::Number(17.0))
        );
        assert_eq!(read[1].value, Some(Value::Text("2026-01-31".into())));
        assert_eq!(read[2].unit.as_deref(), Some("CAD"));
        assert_eq!(read[3], change(Op::Delete, "rate"));
        let unknown = serde_json::from_str::<Change>(r#"{"op": "delete", "path": "a", "to": 1}"#);
        assert!(unknown
            .unwrap_err()
            .to_string()
            .contains("unknown field `to`"));
        assert_eq!(
            serde_json::to_value(&read[0]).unwrap(),
            serde_json::json!({"op": "set_value", "path": "year.price", "value": 17.0})
        );
        for op in [
            Op::SetValue,
            Op::SetFormula,
            Op::Add,
            Op::Delete,
            Op::Rename,
        ] {
            assert_eq!(
                serde_json::to_value(op).unwrap(),
                serde_json::json!(op.name())
            );
        }
    }

    #[test]
    fn a_number_is_written_as_javascript_writes_it() {
        let cases = [
            (0.0, "0"),
            (-0.0, "0"),
            (17.0, "17"),
            (-4.0, "-4"),
            (0.04, "0.04"),
            (0.000_02, "0.00002"),
            (0.000_001, "0.000001"),
            (1e-7, "1e-7"),
            (-1.5e-7, "-1.5e-7"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e20, "100000000000000000000"),
            (1e21, "1e+21"),
            (1.234_5e25, "1.2345e+25"),
        ];
        for (n, text) in cases {
            assert_eq!(js_number(n), text, "{n}");
        }
    }

    /// Each folder of examples, its YAML files by their path in it
    pub(super) fn example_sets() -> Vec<(std::path::PathBuf, HashMap<String, String>)> {
        fn walk(dir: &Path, root: &Path, files: &mut HashMap<String, String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, root, files);
                } else if path.extension().is_some_and(|x| x == "yaml") {
                    let name = path
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    files.insert(name, std::fs::read_to_string(&path).unwrap());
                }
            }
        }
        let mut sets = Vec::new();
        let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("examples")];
        while let Some(dir) = dirs.pop() {
            let mut files = HashMap::new();
            walk(&dir, &dir, &mut files);
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                }
            }
            if !files.is_empty() {
                sets.push((dir, files));
            }
        }
        sets.sort_by(|a, b| a.0.cmp(&b.0));
        sets
    }

    /// Every input and every formula of every example: set to what it is
    /// (an input) or put in parentheses (a formula), each changes only its own
    /// file, and Forge reads back the model with that change alone
    #[test]
    fn every_input_and_formula_of_every_example_is_changed_alone() {
        let (mut inputs, mut formulas, mut models) = (0, 0, 0);
        for (dir, files) in example_sets() {
            let mut mains: Vec<&String> = files.keys().filter(|k| !k.contains('/')).collect();
            mains.sort();
            for main in mains {
                let model = check::read(&files, main).and_then(|m| check::valid(&m).map(|()| m));
                assert!(model.is_ok(), "{}/{main}: {:?}", dir.display(), model.err());
                let model = model.unwrap();
                models += 1;
                let columns = model.tables.values().flat_map(|t| {
                    t.row_formulas
                        .iter()
                        .map(|(c, f)| (format!("{}.{c}", t.name), f.clone()))
                });
                for (path, f) in columns {
                    formulas += 1;
                    let new = if f.starts_with("=MC.") {
                        f.clone()
                    } else {
                        format!("=({})", &f[1..])
                    };
                    let changed =
                        edit_files(&files, main, &[formula_of(Op::SetFormula, &path, &new)]);
                    assert!(
                        changed.is_ok(),
                        "{}/{main}: {:?}",
                        dir.display(),
                        changed.err()
                    );
                    assert_eq!(changed.unwrap().len(), usize::from(new != f), "{path}");
                }
                for (path, var) in &model.scalars {
                    // A distribution is read only as the whole formula: it
                    // is set to itself, which changes no file
                    let c = match &var.formula {
                        Some(f) if f.starts_with("=MC.") => formula_of(Op::SetFormula, path, f),
                        Some(f) => formula_of(Op::SetFormula, path, &format!("=({})", &f[1..])),
                        None => {
                            let number = || Value::Number(var.value.unwrap());
                            value(
                                Op::SetValue,
                                path,
                                var.date_text().map_or_else(number, Value::Text),
                            )
                        },
                    };
                    let same = c.formula.is_some() && c.formula.as_ref() == var.formula.as_ref();
                    if c.op == Op::SetValue {
                        inputs += 1;
                    } else {
                        formulas += 1;
                    }
                    let changed = edit_files(&files, main, &[c]);
                    assert!(
                        changed.is_ok(),
                        "{}/{main}: {:?}",
                        dir.display(),
                        changed.err()
                    );
                    let changed = changed.unwrap();
                    if same {
                        assert!(changed.is_empty(), "{path}");
                    }
                    let lines = |t: &str| t.lines().count();
                    for (name, text) in &changed {
                        assert_eq!(
                            lines(text),
                            lines(&files[name]),
                            "{}/{name} {path}",
                            dir.display()
                        );
                        let differ = text
                            .lines()
                            .zip(files[name].lines())
                            .filter(|(a, b)| a != b)
                            .count();
                        assert_eq!(differ, 1, "{}/{name} {path}", dir.display());
                    }
                }
            }
        }
        assert!(models > 50, "{models} models");
        assert!(
            inputs > 370 && formulas > 440,
            "{inputs} inputs, {formulas} formulas"
        );
    }

    /// Every group and every table of every example's own file: an input, a
    /// formula and a column added, each changing the file by its new lines
    /// only, and deleted again to the file as it was, byte for byte
    #[test]
    fn every_group_and_table_of_every_example_takes_a_name_and_gives_it_back() {
        let mut added = 0;
        for (dir, files) in example_sets() {
            let mut mains: Vec<&String> = files.keys().filter(|k| !k.contains('/')).collect();
            mains.sort();
            for main in mains {
                let model = check::read(&files, main).unwrap();
                let mut groups: Vec<String> = model
                    .scalars
                    .keys()
                    .filter_map(|k| k.split_once('.').map(|(g, _)| g.to_string()))
                    .collect();
                groups.sort();
                groups.dedup();
                let adds = groups
                    .iter()
                    .flat_map(|g| {
                        [
                            value(Op::Add, &format!("{g}.edit_probe"), Value::Number(1.0)),
                            Change {
                                unit: Some("CAD".into()),
                                ..formula_of(Op::Add, &format!("{g}.edit_sum"), "=1 + 1")
                            },
                        ]
                    })
                    .chain(
                        model
                            .tables
                            .keys()
                            .map(|t| formula_of(Op::Add, &format!("{t}.edit_col"), "=1")),
                    )
                    .collect::<Vec<_>>();
                for add in adds {
                    let where_ = format!("{}/{main} {}", dir.display(), add.path);
                    let changed = edit_files(&files, main, std::slice::from_ref(&add));
                    assert!(changed.is_ok(), "{where_}: {:?}", changed.err());
                    let text = &changed.unwrap()[main.as_str()];
                    let extra = text.lines().count() - files[main.as_str()].lines().count();
                    assert!((1..=3).contains(&extra), "{where_}");
                    let mut now = files.clone();
                    now.insert(main.clone(), text.clone());
                    let back = edit_files(&now, main, &[change(Op::Delete, &add.path)]);
                    assert!(back.is_ok(), "{where_}: {:?}", back.err());
                    assert_eq!(
                        back.unwrap().get(main.as_str()),
                        Some(&files[main.as_str()]),
                        "{where_}"
                    );
                    added += 1;
                }
            }
        }
        assert!(added > 300, "{added} added");
    }
}
