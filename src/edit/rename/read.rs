//! What a name reads, in a formula and outside one, in each file of the
//! model (ADR-075, `rename`): a scalar, a column, or a game's result, in the
//! model of the file that has it.

use crate::core::array_calculator::references::{Named, Place};
use crate::types::{ParsedModel, ResolvedInclude};
use indexmap::IndexMap;
use serde_json::Value as Json;

/// What a name reads: a scalar or a column, in the model of the file that
/// has it (never [`Named::Included`]), or nothing of the model
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Ref {
    pub(super) file: String,
    pub(super) named: Named,
}

/// A file of the model, and the model read from it
pub(super) struct Node<'m> {
    pub(super) file: String,
    pub(super) model: &'m ParsedModel,
}

/// The file an include reads, by its name among the files
pub(super) fn file_of(included: &ResolvedInclude) -> String {
    included.resolved_path.to_string_lossy().into_owned()
}

/// The model's files, each once, from the main one down
pub(super) fn nodes<'m>(model: &'m ParsedModel, main: &str) -> Vec<Node<'m>> {
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
pub(super) fn exact(model: &ParsedModel, name: &str) -> Named {
    if model.scalars.contains_key(name) {
        return Named::Scalar(name.to_string());
    }
    if let Some((game, rest)) = name
        .split_once('.')
        .filter(|(g, _)| model.games.contains_key(*g))
    {
        return Named::Game {
            game: game.to_string(),
            rest: rest.to_string(),
        };
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
    pub(super) fn resolve(&self, named: Named) -> Ref {
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
    pub(super) fn outside(&self, name: &str) -> Ref {
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
pub(super) enum Where {
    Scalar(String),
    Column(String),
    Elsewhere,
}

impl Where {
    /// # Panics
    ///
    /// Never: a site of a file is under a key of its top-level mapping.
    pub(super) fn of(model: &ParsedModel, path: &[String]) -> Self {
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

    pub(super) fn place(&self) -> Place<'_> {
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
pub(super) fn is_outside(model: &ParsedModel, path: &[String], key: bool) -> bool {
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

/// A name as written in full: `group.name`, `table.column`, `game.result`
pub(super) fn full(named: &Named) -> Option<String> {
    match named {
        Named::Scalar(name) => Some(name.clone()),
        Named::Column { table, column } => Some(format!("{table}.{column}")),
        Named::Game { game, rest } => Some(format!("{game}.{rest}")),
        _ => None,
    }
}

/// A name as written short: its last part (a game's result is always
/// written in full, so it is not written short)
pub(super) fn short(named: &Named) -> Option<String> {
    full(named).map(|name| name.rsplit('.').next().unwrap_or_default().to_string())
}

/// The keys whose values are text, never formulas, even when they start
/// with `=`
pub(super) const TEXT_KEYS: [&str; 8] = [
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
pub(super) fn is_formula(path: &[String], text: &str) -> bool {
    text.trim_start().starts_with('=')
        && !path
            .last()
            .is_some_and(|key| TEXT_KEYS.contains(&key.as_str()))
}

/// The top-level names of a file that are sections, not names of the model
pub(super) const SECTIONS: [&str; 12] = [
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
pub(super) fn rekey<V>(map: IndexMap<String, V>, old: &str, new: &str) -> IndexMap<String, V> {
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

/// The names a file's analyses read, as YAML reads the file: a
/// simulation's outputs and correlated inputs, a tornado's output and inputs
pub(super) fn analysis_names(text: &str) -> Vec<String> {
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

/// `json` without the keys `keys`, at any depth
pub(super) fn without_keys(json: &mut Json, keys: &[&str]) {
    match json {
        Json::Array(items) => items.iter_mut().for_each(|j| without_keys(j, keys)),
        Json::Object(map) => {
            map.retain(|key, _| !keys.contains(&key.as_str()));
            map.values_mut().for_each(|j| without_keys(j, keys));
        },
        _ => {},
    }
}

/// Each formula in `json` rewritten by `formula`: a text starting with `=`,
/// but for the text of [`TEXT_KEYS`]
pub(super) fn formulas_in(json: &mut Json, formula: &dyn Fn(&str) -> String) {
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

#[cfg(test)]
mod tests {
    use super::super::tests::files;
    use super::*;

    #[test]
    fn a_name_of_an_alias_the_model_does_not_have_reads_nothing() {
        let model = crate::edit::check::read(&files(), "model.yaml").unwrap();
        let node = Node {
            file: "model.yaml".to_string(),
            model: &model,
        };
        for name in ["@rates.x", "@costs", "@costs.nothing"] {
            assert_eq!(node.outside(name).named, Named::Other, "{name}");
        }
        assert_eq!(
            node.outside("@costs.rent"),
            Ref {
                file: "costs.yaml".to_string(),
                named: Named::Scalar("rent".to_string())
            }
        );
    }
}
