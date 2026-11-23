//! The model read back after a rename: every name reads what it read
//! before, renamed, and nothing else changed (ADR-075, decision 3).

use super::read::{analysis_names, file_of, formulas_in, nodes, without_keys, Node, Ref};
use super::Rename;
use crate::core::array_calculator::references::{Named, Place, Reader};
use crate::core::array_calculator::tokenizer;
use crate::edit::text;
use crate::types::ParsedModel;
use serde_json::Value as Json;
use std::collections::HashMap;

impl Rename {
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
        // Where a game's payoff is written, and the counts a payoff of the
        // count form reads, are said for messages, from the names compared
        if let Some(games) = json.get_mut("games") {
            without_keys(games, &["at", "others"]);
        }
        // A game's payoffs and a network's probabilities read names too
        for section in ["games", "network"] {
            if let Some(section) = json.get_mut(section) {
                formulas_in(section, &|f| formula(f, Place::Elsewhere));
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
    pub(super) fn only(
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
