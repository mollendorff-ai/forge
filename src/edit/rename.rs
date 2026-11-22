//! `rename` (ADR-075, decision 2): a scalar, a group, a table or a column,
//! and every name that reads it, in every file of the model.
//!
//! What a name in a formula reads is found by the calculator's own reader
//! ([`Reader`]), where the formula is written: a column's bare name reads
//! its table's columns first, a scalar's short name its group first. A name
//! outside formulas (a scenario's, an expectation's, a simulation's output,
//! a tornado's input) is read as the command that reads it reads it: a full
//! name, or the last part of only one. Each is written again as it was, full
//! or short, and in full when the short name would read something else once
//! renamed. The model is then read back: every name reads what it read
//! before, renamed, and nothing else changed.

use super::{check, is_name, text, Change, Target};
use crate::core::array_calculator::references::{Named, Place, Reader};
use crate::core::array_calculator::tokenizer;
use crate::types::{ParsedModel, ResolvedInclude};
use indexmap::IndexMap;
use serde_json::Value as Json;
use std::collections::HashMap;
use std::ops::Range;

/// What is renamed, by its name in the model of its file
#[derive(Debug, Clone, PartialEq, Eq)]
enum Entity {
    Scalar(String),
    Group(String),
    Table(String),
    Column(String, String),
}

/// What a name reads: a scalar or a column, in the model of the file that
/// has it (never [`Named::Included`]), or nothing of the model
#[derive(Debug, Clone, PartialEq, Eq)]
struct Ref {
    file: String,
    named: Named,
}

/// A file of the model, and the model read from it
struct Node<'m> {
    file: String,
    model: &'m ParsedModel,
}

/// The file an include reads, by its name among the files
fn file_of(included: &ResolvedInclude) -> String {
    included.resolved_path.to_string_lossy().into_owned()
}

/// The model's files, each once, from the main one down
fn nodes<'m>(model: &'m ParsedModel, main: &str) -> Vec<Node<'m>> {
    let mut nodes = vec![Node {
        file: main.to_string(),
        model,
    }];
    let mut next = 0;
    while let Some(node) = nodes.get(next) {
        let model = node.model;
        let mut aliases: Vec<&String> = model.resolved_includes.keys().collect();
        aliases.sort();
        for alias in aliases {
            let included = &model.resolved_includes[alias];
            let file = file_of(included);
            if !nodes.iter().any(|n| n.file == file) {
                nodes.push(Node {
                    file,
                    model: &included.model,
                });
            }
        }
        next += 1;
    }
    nodes
}

/// The scalar or column of this full name in `model`
fn exact(model: &ParsedModel, name: &str) -> Named {
    if model.scalars.contains_key(name) {
        return Named::Scalar(name.to_string());
    }
    name.split_once('.')
        .filter(|(table, column)| {
            model.tables.get(*table).is_some_and(|t| {
                t.columns.contains_key(*column) || t.row_formulas.contains_key(*column)
            })
        })
        .map_or(Named::Other, |(table, column)| Named::Column {
            table: table.to_string(),
            column: column.to_string(),
        })
}

impl Node<'_> {
    /// What `named`, read in this file, is in the file that has it
    fn resolve(&self, named: Named) -> Ref {
        let Named::Included { alias, name } = named else {
            return Ref {
                file: self.file.clone(),
                named,
            };
        };
        self.model.resolved_includes.get(&alias).map_or_else(
            || Ref {
                file: self.file.clone(),
                named: Named::Other,
            },
            |included| Ref {
                file: file_of(included),
                named: exact(&included.model, &name),
            },
        )
    }

    /// What a name outside formulas reads, as scenarios and the analyses
    /// read it: a full name, `@alias.name`, or the last part of only one
    /// scalar
    fn outside(&self, name: &str) -> Ref {
        if let Some(rest) = name.strip_prefix('@') {
            let named =
                rest.split_once('.')
                    .map_or(Named::Other, |(alias, name)| Named::Included {
                        alias: alias.to_string(),
                        name: name.to_string(),
                    });
            return self.resolve(named);
        }
        let named = match exact(self.model, name) {
            Named::Other if !name.contains('.') => self
                .model
                .resolve_scalar_name(name)
                .map_or(Named::Other, Named::Scalar),
            named => named,
        };
        Ref {
            file: self.file.clone(),
            named,
        }
    }
}

/// Where a formula is written in a model, from its keys in the file
#[derive(Debug, Clone, PartialEq, Eq)]
enum Where {
    Scalar(String),
    Column(String),
    Elsewhere,
}

impl Where {
    /// # Panics
    ///
    /// Never: a site of a file is under a key of its top-level mapping.
    fn of(model: &ParsedModel, path: &[String]) -> Self {
        let top = &path[0];
        if model.tables.contains_key(top) {
            return Self::Column(top.clone());
        }
        if model.scalars.contains_key(top) {
            return Self::Scalar(top.clone());
        }
        path.get(1)
            .map(|name| format!("{top}.{name}"))
            .filter(|full| model.scalars.contains_key(full))
            .map_or(Self::Elsewhere, Self::Scalar)
    }

    fn place(&self) -> Place<'_> {
        match self {
            Self::Scalar(name) => Place::Scalar(name),
            Self::Column(table) => Place::Column(table),
            Self::Elsewhere => Place::Elsewhere,
        }
    }
}

/// Whether the scalar at `path` is a name outside formulas: a scenario's
/// override or expectation, the file's expectation, a simulation's output
/// or correlated input, a tornado's output or input
fn is_outside(model: &ParsedModel, path: &[String], key: bool) -> bool {
    let path: Vec<&str> = path.iter().map(String::as_str).collect();
    let scenario = |name: &str| model.scenarios.contains_key(name);
    match (key, path.as_slice()) {
        (true, ["scenarios", s, name]) => {
            scenario(s) && !["probability", "description", "scalars", "expect"].contains(name)
        },
        (true, ["scenarios", s, "scalars" | "expect", _]) => scenario(s),
        (true, ["expect", name]) => {
            !model.tables.contains_key("expect")
                && !model.scalars.contains_key(&format!("expect.{name}"))
        },
        (
            false,
            ["monte_carlo", "outputs", "#", "variable"]
            | ["monte_carlo", "correlations", "#", "variables", "#"]
            | ["tornado", "output"]
            | ["tornado", "inputs", "#", "name"],
        ) => true,
        _ => false,
    }
}

/// A name as written in full: `group.name`, `table.column`
fn full(named: &Named) -> Option<String> {
    match named {
        Named::Scalar(name) => Some(name.clone()),
        Named::Column { table, column } => Some(format!("{table}.{column}")),
        _ => None,
    }
}

/// A name as written short: its last part
fn short(named: &Named) -> Option<String> {
    full(named).map(|name| name.rsplit('.').next().unwrap_or_default().to_string())
}

/// The keys whose values are text, never formulas, even when they start
/// with `=`
const TEXT_KEYS: [&str; 8] = [
    "notes",
    "description",
    "unit",
    "source",
    "format",
    "label",
    "name",
    "validation_status",
];

/// Whether the value at `path`, `text`, is a formula
fn is_formula(path: &[String], text: &str) -> bool {
    text.trim_start().starts_with('=')
        && !path
            .last()
            .is_some_and(|key| TEXT_KEYS.contains(&key.as_str()))
}

/// The top-level names of a file that are sections, not names of the model
const SECTIONS: [&str; 12] = [
    "_forge_version",
    "_name",
    "_includes",
    "scenarios",
    "games",
    "expect",
    "monte_carlo",
    "tornado",
    "decision_tree",
    "real_options",
    "bootstrap",
    "bayesian_network",
];

/// `map` with the key `old` called `new`, in its place
fn rekey<V>(map: IndexMap<String, V>, old: &str, new: &str) -> IndexMap<String, V> {
    map.into_iter()
        .map(|(k, v)| {
            if k == old {
                (new.to_string(), v)
            } else {
                (k, v)
            }
        })
        .collect()
}

/// A rename: what, in which file, and its new name
pub(super) struct Rename {
    file: String,
    entity: Entity,
    new: String,
}

impl Rename {
    /// What `target` names, to be called `new`
    fn of(target: &Target<'_>, new: &str) -> Result<Self, String> {
        let model = target.model;
        let name = target.name.as_str();
        if !is_name(new) {
            return Err(format!(
                "{new} is not a name: a letter or _, then letters, digits and _"
            ));
        }
        let group = |g: &str| {
            let prefix = format!("{g}.");
            model.scalars.keys().any(|k| k.starts_with(&prefix))
        };
        let entity = if model.scalars.contains_key(name) {
            Entity::Scalar(name.to_string())
        } else if model.tables.contains_key(name) {
            Entity::Table(name.to_string())
        } else if model.games.contains_key(name) {
            return Err("it is a game, which edit does not rename yet".into());
        } else if !name.contains('.') && group(name) {
            Entity::Group(name.to_string())
        } else if let Named::Column { table, column } = exact(model, name) {
            Entity::Column(table, column)
        } else {
            return Err("it is not in the model".into());
        };
        let top_free = |top: &str| {
            !(model.scalars.contains_key(top)
                || model.tables.contains_key(top)
                || model.games.contains_key(top)
                || group(top)
                || SECTIONS.contains(&top))
        };
        let (was, free) = match &entity {
            Entity::Scalar(old) => match old.rsplit_once('.') {
                Some((g, last)) => (last, !model.scalars.contains_key(&format!("{g}.{new}"))),
                None => (old.as_str(), top_free(new)),
            },
            Entity::Group(old) | Entity::Table(old) => (old.as_str(), top_free(new)),
            Entity::Column(table, column) => (
                column.as_str(),
                !matches!(
                    exact(model, &format!("{table}.{new}")),
                    Named::Column { .. }
                ),
            ),
        };
        if !free && was != new {
            return Err(format!("{new} is already a name there"));
        }
        Ok(Self {
            file: target.file.clone(),
            entity,
            new: new.to_string(),
        })
    }

    /// The new full name of the scalar `name`, when it is renamed
    fn scalar(&self, name: &str) -> Option<String> {
        match &self.entity {
            Entity::Scalar(old) if old == name => Some(
                old.rsplit_once('.')
                    .map_or_else(|| self.new.clone(), |(g, _)| format!("{g}.{}", self.new)),
            ),
            Entity::Group(g) => name
                .split_once('.')
                .filter(|(group, _)| group == g)
                .map(|(_, rest)| format!("{}.{rest}", self.new)),
            _ => None,
        }
    }

    /// The new name of the table `table`, when it is renamed
    fn table(&self, table: &str) -> Option<String> {
        match &self.entity {
            Entity::Table(t) if t == table => Some(self.new.clone()),
            _ => None,
        }
    }

    /// The new name of the column `column` of `table`, when it is renamed
    fn column(&self, table: &str, column: &str) -> Option<String> {
        match &self.entity {
            Entity::Column(t, c) if t == table && c == column => Some(self.new.clone()),
            _ => None,
        }
    }

    /// What `was` reads once renamed
    fn map(&self, was: &Ref) -> Ref {
        if was.file != self.file {
            return was.clone();
        }
        let named = match &was.named {
            Named::Scalar(name) => self.scalar(name).map(Named::Scalar),
            Named::Column { table, column } => self
                .table(table)
                .map(|t| (t, column.clone()))
                .or_else(|| self.column(table, column).map(|c| (table.clone(), c)))
                .map(|(table, column)| Named::Column { table, column }),
            _ => None,
        };
        named.map_or_else(
            || was.clone(),
            |named| Ref {
                file: was.file.clone(),
                named,
            },
        )
    }

    /// Where a formula of the renamed file is, once renamed
    fn map_where(&self, at: &Where) -> Where {
        match at {
            Where::Scalar(name) => Where::Scalar(self.scalar(name).unwrap_or_else(|| name.clone())),
            Where::Column(table) => {
                Where::Column(self.table(table).unwrap_or_else(|| table.clone()))
            },
            Where::Elsewhere => Where::Elsewhere,
        }
    }

    /// The model with the names renamed (and not what reads them)
    fn renamed(&self, model: &ParsedModel) -> ParsedModel {
        let mut model = model.clone();
        model.scalars = std::mem::take(&mut model.scalars)
            .into_iter()
            .map(|(name, mut var)| match self.scalar(&name) {
                Some(new) => {
                    var.path.clone_from(&new);
                    (new, var)
                },
                None => (name, var),
            })
            .collect();
        model.tables = std::mem::take(&mut model.tables)
            .into_iter()
            .map(|(name, mut table)| {
                if let Some(new) = self.table(&name) {
                    table.name.clone_from(&new);
                    return (new, table);
                }
                if let Entity::Column(t, c) = &self.entity {
                    if *t == name {
                        table.columns = rekey(std::mem::take(&mut table.columns), c, &self.new);
                        if let Some(column) = table.columns.get_mut(&self.new) {
                            column.name.clone_from(&self.new);
                        }
                        table.row_formulas =
                            rekey(std::mem::take(&mut table.row_formulas), c, &self.new);
                        table.formula_metadata =
                            rekey(std::mem::take(&mut table.formula_metadata), c, &self.new);
                    }
                }
                (name, table)
            })
            .collect();
        model
    }

    /// The keys down to the renamed name's own key in its file
    fn key_path(&self) -> Vec<String> {
        match &self.entity {
            Entity::Scalar(name) => name.split('.').map(str::to_string).collect(),
            Entity::Group(name) | Entity::Table(name) => vec![name.clone()],
            Entity::Column(table, column) => vec![table.clone(), column.clone()],
        }
    }

    /// `word`, which read `was`, written to read `now`: as it was written,
    /// full or short, or in full when that would read something else.
    /// `reads` says what a spelling reads once renamed, in the renamed
    /// file; elsewhere a name is written in full (`@alias.name`) already.
    fn spell(
        word: &str,
        was: &Ref,
        now: &Ref,
        reads: impl Fn(&str) -> Option<Ref>,
    ) -> Result<String, String> {
        let (prefix, name) = match word.strip_prefix('@').and_then(|r| r.split_once('.')) {
            Some((alias, name)) => (format!("@{alias}."), name),
            None => (String::new(), word),
        };
        let spelled = if was == now {
            Some(word.to_string())
        } else if full(&was.named).as_deref() == Some(name) {
            full(&now.named).map(|n| format!("{prefix}{n}"))
        } else {
            short(&now.named)
        };
        let fits = |spelling: &String| reads(spelling).is_none_or(|r| r == *now);
        spelled
            .filter(fits)
            .or_else(|| full(&now.named).filter(fits))
            .ok_or_else(|| {
                format!("{word} would read something else once renamed, written short or in full")
            })
    }

    /// A formula of `node` at `at`, its names that read what is renamed
    /// renamed; `after` reads the renamed file
    fn formula(
        &self,
        formula: &str,
        at: &Where,
        node: &Node<'_>,
        before: &Reader,
        after: Option<&(Reader, Node<'_>)>,
    ) -> Result<String, String> {
        // A formula Forge cannot read has no names: a model Forge reads has none
        let names = tokenizer::names(formula).unwrap_or_default();
        let at_after = self.map_where(at);
        let mut edits = Vec::new();
        for (start, word) in names {
            let was = node.resolve(before.read(&word, at.place()));
            let now = self.map(&was);
            let reads = |w: &str| {
                after.map(|(reader, node)| node.resolve(reader.read(w, at_after.place())))
            };
            let spelled = Self::spell(&word, &was, &now, reads)
                .map_err(|why| format!("in {formula}, {why}"))?;
            if spelled != word {
                edits.push((start..start + word.len(), spelled));
            }
        }
        Ok(text::splice(formula, edits))
    }

    /// The new text of one file of the model
    fn write_file(&self, text: &str, node: &Node<'_>) -> Result<String, String> {
        let own = node.file == self.file;
        if !own
            && !node
                .model
                .resolved_includes
                .values()
                .any(|i| file_of(i) == self.file)
        {
            return Ok(text.to_string());
        }
        let before = Reader::new(node.model.clone());
        let renamed = own.then(|| self.renamed(node.model));
        let after = renamed.as_ref().map(|model| {
            (
                Reader::new(model.clone()),
                Node {
                    file: node.file.clone(),
                    model,
                },
            )
        });
        let key_path = self.key_path();
        let mut keys = 0;
        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        for site in text::sites(text)? {
            let raw = &text[site.range.clone()];
            if own && site.key && site.path == key_path {
                keys += 1;
                edits.push((site.range, text::name_text(raw, &self.new)));
            } else if !site.key && is_formula(&site.path, &site.text) {
                let at = Where::of(node.model, &site.path);
                let new = self.formula(&site.text, &at, node, &before, after.as_ref())?;
                if new != site.text {
                    let written = text::renamed_formula(raw, &site.text, &new).ok_or_else(|| {
                        format!(
                            "{} is written over several lines; edit renames in a formula on one line",
                            site.path.join(".")
                        )
                    })?;
                    edits.push((site.range, written));
                }
            } else if is_outside(node.model, &site.path, site.key) {
                let was = node.outside(&site.text);
                let now = self.map(&was);
                let reads = |w: &str| after.as_ref().map(|(_, node)| node.outside(w));
                let new = Self::spell(&site.text, &was, &now, reads)
                    .map_err(|why| format!("at {}, {why}", site.path.join(".")))?;
                if new != site.text {
                    edits.push((site.range, text::name_text(raw, &new)));
                }
            }
        }
        if own && keys != 1 {
            return Err("it is not written where edit can rename it".into());
        }
        Ok(text::splice(text, edits))
    }

    /// The model as JSON, with each name a formula or a section reads
    /// written as what it reads: renamed (`mapped`) or as it is. Two models
    /// read the same when these are equal.
    fn canonical(&self, model: &ParsedModel, file: &str, mapped: bool) -> Json {
        let node = Node {
            file: file.to_string(),
            model,
        };
        let reader = Reader::new(model.clone());
        let token = |was: Ref, word: &str| {
            let now = if mapped { self.map(&was) } else { was };
            match now.named {
                Named::Other => word.to_string(),
                named => format!("\u{27e6}{}|{named:?}\u{27e7}", now.file),
            }
        };
        let formula = |f: &str, place: Place<'_>| {
            let edits = tokenizer::names(f)
                .unwrap_or_default()
                .into_iter()
                .map(|(at, w)| {
                    let read = token(node.resolve(reader.read(&w, place)), &w);
                    (at..at + w.len(), read)
                })
                .collect();
            text::splice(f, edits)
        };
        let outside = |name: &str| token(node.outside(name), name);
        let mut read = model.clone();
        for (name, var) in &mut read.scalars {
            if let Some(f) = &mut var.formula {
                *f = formula(f, Place::Scalar(name));
            }
        }
        for table in read.tables.values_mut() {
            let name = table.name.clone();
            for f in table.row_formulas.values_mut() {
                *f = formula(f, Place::Column(&name));
            }
        }
        for scenario in read.scenarios.values_mut() {
            scenario.overrides = std::mem::take(&mut scenario.overrides)
                .into_iter()
                .map(|(name, value)| (outside(&name), value))
                .collect();
            scenario.dates = std::mem::take(&mut scenario.dates)
                .into_iter()
                .map(|name| outside(&name))
                .collect();
            for expected in &mut scenario.expect {
                expected.name = outside(&expected.name);
            }
        }
        for expected in &mut read.expect {
            expected.name = outside(&expected.name);
        }
        if mapped && file == self.file {
            read = self.renamed(&read);
        }
        let mut json = serde_json::to_value(&read).expect("a model is written as JSON");
        // A game's formulas and a network's probabilities read names too
        for section in ["games", "network"] {
            if let Some(found) = json.get_mut(section) {
                formulas_in(found, &|f| formula(f, Place::Elsewhere));
            }
        }
        let includes: serde_json::Map<String, Json> = model
            .resolved_includes
            .iter()
            .map(|(alias, included)| {
                (
                    alias.clone(),
                    self.canonical(&included.model, &file_of(included), mapped),
                )
            })
            .collect();
        serde_json::json!({"model": json, "includes": includes})
    }

    /// What the model in `texts` reads, as [`Rename::canonical`] gives it,
    /// with the names its analyses read, which their commands read from the
    /// file
    fn read_as(
        &self,
        model: &ParsedModel,
        texts: &HashMap<String, String>,
        main: &str,
        mapped: bool,
    ) -> Json {
        let analyses: serde_json::Map<String, Json> = nodes(model, main)
            .iter()
            .map(|node| {
                let names = analysis_names(&texts[&node.file])
                    .iter()
                    .map(|name| {
                        let was = node.outside(name);
                        let now = if mapped { self.map(&was) } else { was };
                        Json::String(format!("{now:?}"))
                    })
                    .collect();
                (node.file.clone(), Json::Array(names))
            })
            .collect();
        serde_json::json!({
            "model": self.canonical(model, main, mapped),
            "analyses": analyses,
        })
    }

    /// Whether `after`, read from `new`, is `before`, read from `old`, with
    /// this rename and nothing else
    fn only(
        &self,
        (before, old): (&ParsedModel, &HashMap<String, String>),
        (after, new): (&ParsedModel, &HashMap<String, String>),
        main: &str,
    ) -> Result<(), String> {
        if self.read_as(before, old, main, true) == self.read_as(after, new, main, false) {
            Ok(())
        } else {
            Err("Forge reads the new text as more than this rename, so it was not made".into())
        }
    }
}

/// The names a file's analyses read, as YAML reads the file: a
/// simulation's outputs and correlated inputs, a tornado's output and inputs
fn analysis_names(text: &str) -> Vec<String> {
    use yaml_serde::Value as Yaml;
    if !text.contains("monte_carlo") && !text.contains("tornado") {
        return Vec::new();
    }
    let yaml: Yaml = yaml_serde::from_str(text).unwrap_or_default();
    let items = |value: &Yaml| value.as_sequence().cloned().unwrap_or_default();
    let simulation = &yaml["monte_carlo"];
    let mut found: Vec<Yaml> = items(&simulation["outputs"])
        .iter()
        .map(|output| output["variable"].clone())
        .collect();
    for correlation in items(&simulation["correlations"]) {
        found.extend(items(&correlation["variables"]));
    }
    found.push(yaml["tornado"]["output"].clone());
    found.extend(
        items(&yaml["tornado"]["inputs"])
            .iter()
            .map(|input| input["name"].clone()),
    );
    found
        .iter()
        .filter_map(Yaml::as_str)
        .map(str::to_string)
        .collect()
}

/// Each formula in `json` rewritten by `formula`: a text starting with `=`,
/// but for the text of [`TEXT_KEYS`]
fn formulas_in(json: &mut Json, formula: &dyn Fn(&str) -> String) {
    match json {
        Json::String(text) if text.starts_with('=') => *text = formula(text),
        Json::Array(items) => items.iter_mut().for_each(|j| formulas_in(j, formula)),
        Json::Object(map) => map
            .iter_mut()
            .filter(|(key, _)| !TEXT_KEYS.contains(&key.as_str()))
            .for_each(|(_, j)| formulas_in(j, formula)),
        _ => {},
    }
}

/// `rename`: `target` called `change.new_name`, and every name that reads
/// it renamed, in every file; kept only when Forge reads the model back as
/// it was, but for the name
pub(super) fn apply(
    files: &mut HashMap<String, String>,
    main: &str,
    before: &ParsedModel,
    target: &Target<'_>,
    change: &Change,
) -> Result<(), String> {
    let new = change.new_name.as_deref().ok_or("it needs `new_name`")?;
    let rename = Rename::of(target, new).map_err(|why| format!("{why} ({})", target.file))?;
    let mut changed = files.clone();
    for node in nodes(before, main) {
        let text = rename
            .write_file(&files[&node.file], &node)
            .map_err(|why| format!("{why} ({})", node.file))?;
        changed.insert(node.file, text);
    }
    let after = check::read(&changed, main)
        .and_then(|after| check::valid(&after).map(|()| after))
        .map_err(|e| format!("Forge refuses the model with this change: {e}"))?;
    rename.only((before, files), (&after, &changed), main)?;
    *files = changed;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{edit_files, Change, Op};
    use std::collections::{BTreeMap, HashMap};

    const MODEL: &str = "_forge_version: \"5.6.0\"
_includes:
  - file: costs.yaml
    as: costs
rate: 0.05   # a rate
year:
  price: 16   # dollars
  units: 100
  start: 2026-01-31
  revenue:
    formula: \"=price * year.units\"   # short, in its group
    unit: CAD
    notes: \"=price is not renamed in a note\"
  net:
    formula: \"=year.revenue - @costs.rent * (1 + rate)\"
other:
  margin:
    formula: \"=revenue / 2\"   # the only revenue
  top: {formula: '=SUM(months.double)'}
months:
  month: [1, 2, 3]
  sales: =month * price
  double: \"=sales * 2\"
  scaled: =sales * units
scenarios:
  high:
    description: A high price
    scalars: {price: 20, start: 2027-01-31}
  low:
    description: A low price
    scalars:
      year.price: 12
    expect:
      year.revenue: 1200
expect:
  year.revenue: 1600
monte_carlo:
  iterations: 100
  outputs:
    - variable: year.revenue
tornado:
  output: year.net
  inputs:
    - name: price
      low: 10
      high: 20
";
    const COSTS: &str = "_forge_version: \"5.1.0\"\nrent: 900   # a month\nspare: 1\nscenarios:\n  dear:\n    rent: 950\n";

    fn files() -> HashMap<String, String> {
        HashMap::from([
            ("model.yaml".to_string(), MODEL.to_string()),
            ("costs.yaml".to_string(), COSTS.to_string()),
        ])
    }

    fn rename(path: &str, new: &str) -> Change {
        Change {
            op: Op::Rename,
            path: path.into(),
            value: None,
            formula: None,
            unit: None,
            after: None,
            new_name: Some(new.into()),
        }
    }

    fn run(
        files: &HashMap<String, String>,
        change: Change,
    ) -> Result<BTreeMap<String, String>, String> {
        edit_files(files, "model.yaml", &[change]).map_err(|e| e.to_string())
    }

    /// `text` with each of `pairs` replaced, each once and in turn
    fn replaced(text: &str, pairs: &[(&str, &str)]) -> String {
        pairs.iter().fold(text.to_string(), |text, (from, to)| {
            assert!(text.contains(from), "{from}");
            text.replacen(from, to, 1)
        })
    }

    /// `change` gives `want` for each file it changes, and renaming back
    /// gives the files as they were, byte for byte
    fn renamed_and_back(change: Change, back: Change, want: &[(&str, String)]) {
        let changed = run(&files(), change).unwrap();
        let want: BTreeMap<String, String> = want
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.clone()))
            .collect();
        assert_eq!(changed, want);
        let mut now = files();
        now.extend(changed);
        let again = run(&now, back).unwrap();
        for (name, text) in again {
            assert_eq!(text, files()[&name], "{name}");
        }
    }

    #[test]
    fn a_scalar_is_renamed_where_formulas_scenarios_and_analyses_read_it() {
        let want = replaced(
            MODEL,
            &[
                ("  price: 16", "  cost: 16"),
                ("=price * year.units", "=cost * year.units"),
                ("sales: =month * price", "sales: =month * cost"),
                ("{price: 20,", "{cost: 20,"),
                ("year.price: 12", "year.cost: 12"),
                ("name: price", "name: cost"),
            ],
        );
        renamed_and_back(
            rename("year.price", "cost"),
            rename("year.cost", "price"),
            &[("model.yaml", want)],
        );
        // A top-level scalar
        let want = replaced(
            MODEL,
            &[
                ("rate: 0.05", "growth: 0.05"),
                ("(1 + rate)", "(1 + growth)"),
            ],
        );
        renamed_and_back(
            rename("rate", "growth"),
            rename("growth", "rate"),
            &[("model.yaml", want)],
        );
    }

    #[test]
    fn a_name_of_an_included_file_is_renamed_there_and_where_the_model_reads_it() {
        renamed_and_back(
            rename("@costs.rent", "lease"),
            rename("@costs.lease", "rent"),
            &[
                ("costs.yaml", COSTS.replace("rent:", "lease:")),
                ("model.yaml", MODEL.replace("@costs.rent", "@costs.lease")),
            ],
        );
    }

    #[test]
    fn a_group_and_a_table_are_renamed_with_every_full_name_under_them() {
        let want = replaced(
            MODEL,
            &[
                ("\nyear:\n", "\nannual:\n"),
                ("=price * year.units", "=price * annual.units"),
                ("=year.revenue -", "=annual.revenue -"),
                ("year.price: 12", "annual.price: 12"),
                ("year.revenue: 1200", "annual.revenue: 1200"),
                ("year.revenue: 1600", "annual.revenue: 1600"),
                ("variable: year.revenue", "variable: annual.revenue"),
                ("output: year.net", "output: annual.net"),
            ],
        );
        renamed_and_back(
            rename("year", "annual"),
            rename("annual", "year"),
            &[("model.yaml", want)],
        );
        let want = replaced(
            MODEL,
            &[
                ("SUM(months.double)", "SUM(periods.double)"),
                ("\nmonths:\n", "\nperiods:\n"),
            ],
        );
        renamed_and_back(
            rename("months", "periods"),
            rename("periods", "months"),
            &[("model.yaml", want)],
        );
    }

    #[test]
    fn a_column_is_renamed_where_its_table_and_the_model_read_it() {
        let want = replaced(
            MODEL,
            &[
                ("months.double", "months.twice"),
                ("double: \"=sales", "twice: \"=sales"),
            ],
        );
        renamed_and_back(
            rename("months.double", "twice"),
            rename("months.twice", "double"),
            &[("model.yaml", want)],
        );
        let want = replaced(
            MODEL,
            &[
                ("sales: =month", "amount: =month"),
                ("\"=sales * 2\"", "\"=amount * 2\""),
                ("=sales * units", "=amount * units"),
            ],
        );
        renamed_and_back(
            rename("months.sales", "amount"),
            rename("months.amount", "sales"),
            &[("model.yaml", want)],
        );
    }

    #[test]
    fn a_short_name_that_would_read_something_else_is_written_in_full() {
        // `units` in the table now reads its new column: the scalar it read
        // is written in full
        let changed = run(&files(), rename("months.sales", "units")).unwrap();
        assert_eq!(
            changed["model.yaml"],
            replaced(
                MODEL,
                &[
                    ("sales: =month", "units: =month"),
                    ("\"=sales * 2\"", "\"=units * 2\""),
                    ("=sales * units", "=units * year.units"),
                ]
            )
        );
        // A second `price`: the short name no longer says which, outside its
        // group, in formulas and outside them
        let changed = run(&files(), rename("other.margin", "price")).unwrap();
        assert_eq!(
            changed["model.yaml"],
            replaced(
                MODEL,
                &[
                    ("  margin:", "  price:"),
                    ("sales: =month * price", "sales: =month * year.price"),
                    ("{price: 20,", "{year.price: 20,"),
                    ("name: price", "name: year.price"),
                ]
            )
        );
    }

    #[test]
    fn names_in_any_alphabet_and_in_a_game_are_renamed() {
        let text = "_forge_version: \"5.1.0\"\nhypothèses:\n  coût_unitaire: 3   # €\n假设:\n  单价: 5\n  总价:\n    formula: \"=假设.单价 * coût_unitaire\"\n";
        let one = HashMap::from([("m.yaml".to_string(), text.to_string())]);
        let changed = edit_files(
            &one,
            "m.yaml",
            &[
                rename("hypothèses.coût_unitaire", "prix"),
                rename("假设.单价", "价格"),
            ],
        )
        .unwrap();
        assert_eq!(
            changed["m.yaml"],
            text.replace("coût_unitaire", "prix")
                .replace("单价", "价格")
        );
        let game = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("examples/games/public-goods.yaml"),
        )
        .unwrap();
        let one = HashMap::from([("m.yaml".to_string(), game.clone())]);
        let changed = edit_files(&one, "m.yaml", &[rename("r", "multiplier")]).unwrap();
        let want = game
            .replace("\nr: 1.6", "\nmultiplier: 1.6")
            .replace("=r * endowment", "=multiplier * endowment")
            .replace(
                "endowment + r * endowment",
                "endowment + multiplier * endowment",
            )
            .replace("\"=r * endowment\"", "\"=multiplier * endowment\"")
            .replace("{r: 5}", "{multiplier: 5}")
            .replace("{r: 6}", "{multiplier: 6}");
        assert_eq!(changed["m.yaml"], want);
    }

    #[test]
    fn a_rename_that_cannot_be_made_is_refused_with_the_reason() {
        let refused = |change: Change, why: &str| {
            let message = run(&files(), change).unwrap_err();
            assert!(message.contains(why), "{message}");
            assert!(message.ends_with(". No file was changed"), "{message}");
        };
        refused(
            rename("year.price", "units"),
            "rename year.price: units is already a name there (model.yaml)",
        );
        refused(rename("rate", "year"), "year is already a name there");
        refused(
            rename("rate", "scenarios"),
            "scenarios is already a name there",
        );
        refused(rename("year", "months"), "months is already a name there");
        refused(
            rename("months.sales", "double"),
            "double is already a name there",
        );
        refused(
            rename("year.price", "2x"),
            "2x is not a name: a letter or _, then letters, digits and _",
        );
        refused(rename("nothing", "x"), "it is not in the model");
        refused(rename("months.nothing", "x"), "it is not in the model");
        refused(
            Change {
                new_name: None,
                ..rename("rate", "x")
            },
            "rename rate: it needs `new_name`",
        );
        refused(
            Change {
                op: Op::Delete,
                ..rename("rate", "x")
            },
            "delete does not take `new_name`",
        );
        // Renaming to its own name changes nothing
        assert_eq!(
            run(&files(), rename("year.price", "price")),
            Ok(BTreeMap::new())
        );
    }

    #[test]
    fn a_rename_forge_would_read_as_more_than_it_is_refused() {
        let one = |text: &str, change: Change| {
            let files = HashMap::from([("m.yaml".to_string(), text.to_string())]);
            edit_files(&files, "m.yaml", &[change]).map_err(|e| e.to_string())
        };
        let head = "_forge_version: \"5.1.0\"\n";
        // One key, read as two groups' names
        let alias = format!("{head}base: &b\n  p: 1\ng: *b\n");
        let message = one(&alias, rename("base.p", "q")).unwrap_err();
        assert!(
            message.contains("Forge reads the new text as more than this rename"),
            "{message}"
        );
        let message = one(&alias, rename("g.p", "q")).unwrap_err();
        assert!(
            message.contains("it is not written where edit can rename it (m.yaml)"),
            "{message}"
        );
        // A formula folded over lines
        let folded = format!("{head}rate: 1\nx:\n  formula: >\n    =rate *\n    2\n");
        let message = one(&folded, rename("rate", "r")).unwrap_err();
        assert!(
            message.contains("x.formula is written over several lines"),
            "{message}"
        );
        // A name of LET's that would read the renamed scalar
        let local = format!("{head}rate: 1\nx:\n  formula: \"=LET(k, 2, k * rate)\"\n");
        let message = one(&local, rename("rate", "k")).unwrap_err();
        assert!(
            message.contains("in =LET(k, 2, k * rate), k would read something else once renamed"),
            "{message}"
        );
        // A game is not renamed yet
        let game = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("examples/games/public-goods.yaml"),
        )
        .unwrap();
        let message = one(&game, rename("pg", "public")).unwrap_err();
        assert!(
            message.contains("it is a game, which edit does not rename yet"),
            "{message}"
        );
        // An expectation of a name the model does not have would read the
        // renamed scalar
        let unknown = "_forge_version: \"5.6.0\"\nrate: 1\nexpect:\n  nothing: 1\n";
        let message = one(unknown, rename("rate", "nothing")).unwrap_err();
        assert!(
            message.contains("at expect.nothing, nothing would read something else"),
            "{message}"
        );
        // A group with a key `value:` reads as one scalar
        let message = one(
            &format!("{head}g:\n  a: 1\n  b: 2\n"),
            rename("g.a", "value"),
        )
        .unwrap_err();
        assert!(
            message.contains("rename g.a: Forge refuses the model with this change: "),
            "{message}"
        );
    }

    #[test]
    fn a_file_included_twice_is_renamed_once_and_read_by_both_names() {
        let model = "_forge_version: \"5.1.0\"\n_includes:\n  - file: costs.yaml\n    as: costs\n  - file: costs.yaml\n    as: same\nboth:\n  formula: \"=@costs.rent + @same.rent\"\n";
        let files = HashMap::from([
            ("model.yaml".to_string(), model.to_string()),
            ("costs.yaml".to_string(), COSTS.to_string()),
        ]);
        let changed = run(&files, rename("@same.rent", "lease")).unwrap();
        assert_eq!(changed["model.yaml"], model.replace(".rent", ".lease"));
        assert_eq!(changed["costs.yaml"], COSTS.replace("rent:", "lease:"));
    }

    #[test]
    fn a_name_of_an_alias_the_model_does_not_have_reads_nothing() {
        let model = super::super::check::read(&files(), "model.yaml").unwrap();
        let node = super::Node {
            file: "model.yaml".to_string(),
            model: &model,
        };
        for name in ["@rates.x", "@costs", "@costs.nothing"] {
            assert_eq!(node.outside(name).named, super::Named::Other, "{name}");
        }
        assert_eq!(
            node.outside("@costs.rent"),
            super::Ref {
                file: "costs.yaml".to_string(),
                named: super::Named::Scalar("rent".to_string())
            }
        );
    }

    /// Every scalar, group, table and column of every example, and every
    /// scalar of a file one includes: renamed, which Forge reads back as the
    /// same model but for the name, and renamed back to the files as they
    /// were, byte for byte
    #[test]
    fn every_name_of_every_example_is_renamed_and_back() {
        let sets = super::super::tests::example_sets();
        let results: Vec<(usize, Vec<String>)> = std::thread::scope(|scope| {
            // Every folder's thread starts before the first is waited for
            let mut runs = Vec::new();
            for chunk in sets.chunks(sets.len().div_ceil(4)) {
                runs.push(
                    scope.spawn(move || chunk.iter().map(renamed_and_back_in).collect::<Vec<_>>()),
                );
            }
            runs.into_iter()
                .flat_map(|run| run.join().unwrap())
                .collect()
        });
        let renamed: usize = results.iter().map(|(n, _)| n).sum();
        let failures: Vec<&String> = results.iter().flat_map(|(_, f)| f).collect();
        assert!(
            failures.is_empty(),
            "{} failed:\n{failures:#?}",
            failures.len()
        );
        assert!(renamed > 1200, "{renamed} renamed");
    }

    /// Each name of one folder of examples renamed and back: how many were,
    /// and what failed
    fn renamed_and_back_in(
        (dir, files): &(std::path::PathBuf, HashMap<String, String>),
    ) -> (usize, Vec<String>) {
        let mut failures = Vec::new();
        let mut renamed = 0;
        {
            let mut mains: Vec<&String> = files.keys().filter(|k| !k.contains('/')).collect();
            mains.sort();
            for main in mains {
                let model = super::super::check::read(files, main).unwrap();
                let mut names: Vec<String> = model.scalars.keys().cloned().collect();
                let mut groups: Vec<String> = names
                    .iter()
                    .filter_map(|n| n.split_once('.').map(|(g, _)| g.to_string()))
                    .collect();
                groups.sort();
                groups.dedup();
                names.extend(groups);
                for (t, table) in &model.tables {
                    names.push(t.clone());
                    names.extend(table.column_names().iter().map(|c| format!("{t}.{c}")));
                }
                for (alias, included) in &model.resolved_includes {
                    names.extend(
                        included
                            .model
                            .scalars
                            .keys()
                            .map(|n| format!("@{alias}.{n}")),
                    );
                }
                for path in names {
                    let place = format!("{}/{main} {path}", dir.display());
                    let (head, last) = path
                        .rsplit_once('.')
                        .map_or(("", path.as_str()), |(h, l)| (h, l));
                    let new = if head.is_empty() {
                        "edit_renamed".to_string()
                    } else {
                        format!("{head}.edit_renamed")
                    };
                    let there = edit_files(files, main, &[rename(&path, "edit_renamed")]);
                    let there = match there {
                        Ok(there) if !there.is_empty() => there,
                        other => {
                            failures.push(format!("{place}: {other:?}"));
                            continue;
                        },
                    };
                    let mut now = files.clone();
                    now.extend(there);
                    match edit_files(&now, main, &[rename(&new, last)]) {
                        Ok(back) if back.iter().all(|(name, text)| files[name] == *text) => {
                            renamed += 1;
                        },
                        other => failures.push(format!("{place} back: {other:?}")),
                    }
                }
            }
        }
        (renamed, failures)
    }
}
