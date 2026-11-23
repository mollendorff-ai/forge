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

mod check;
mod game;
mod read;

use super::{check as model_check, is_name, text, Change, Target};
use crate::core::array_calculator::references::{Named, Reader};
use crate::core::array_calculator::tokenizer;
use crate::types::ParsedModel;
use read::{
    exact, file_of, full, is_formula, is_outside, nodes, rekey, short, Node, Ref, Where, SECTIONS,
};
use std::collections::HashMap;
use std::ops::Range;

/// What is renamed, by its name in the model of its file
#[derive(Debug, Clone, PartialEq, Eq)]
enum Entity {
    Scalar(String),
    Group(String),
    Table(String),
    Column(String, String),
    /// A game in `games:`
    Game(String),
    /// A player of a game played once
    Player {
        game: String,
        player: String,
    },
    /// A player of a tournament
    Entrant {
        game: String,
        entrant: String,
    },
    /// A strategy of a game played once: every player's of that name, or
    /// one player's
    Strategy {
        game: String,
        player: Option<String>,
        strategy: String,
    },
}

/// Whether `top` can be a new name at the top level of `model`: no scalar,
/// table, game or group has it, and no section of the file
fn top_free(model: &ParsedModel, top: &str) -> bool {
    let prefix = format!("{top}.");
    !(model.scalars.contains_key(top)
        || model.tables.contains_key(top)
        || model.games.contains_key(top)
        || model.scalars.keys().any(|k| k.starts_with(&prefix))
        || SECTIONS.contains(&top))
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
        let (entity, was, free) = match game::entity(model, name, new)? {
            Some(found) => found,
            None => Self::model_entity(model, name, new)?,
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

    /// A scalar, a group, a table or a column of `model` named `name`; the
    /// name it has now, and whether `new` is free there
    fn model_entity(
        model: &ParsedModel,
        name: &str,
        new: &str,
    ) -> Result<(Entity, String, bool), String> {
        let entity = if model.scalars.contains_key(name) {
            Entity::Scalar(name.to_string())
        } else if model.tables.contains_key(name) {
            Entity::Table(name.to_string())
        } else if !name.contains('.')
            && model
                .scalars
                .keys()
                .any(|k| k.strip_prefix(name).is_some_and(|r| r.starts_with('.')))
        {
            Entity::Group(name.to_string())
        } else if let Named::Column { table, column } = exact(model, name) {
            Entity::Column(table, column)
        } else {
            return Err("it is not in the model".into());
        };
        let (was, free) = match &entity {
            Entity::Scalar(old) => match old.rsplit_once('.') {
                Some((g, last)) => (last, !model.scalars.contains_key(&format!("{g}.{new}"))),
                None => (old.as_str(), top_free(model, new)),
            },
            Entity::Column(table, column) => (
                column.as_str(),
                !matches!(
                    exact(model, &format!("{table}.{new}")),
                    Named::Column { .. }
                ),
            ),
            // A group or a table
            _ => (name, top_free(model, new)),
        };
        let was = was.to_string();
        Ok((entity, was, free))
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
            Named::Game { game, rest } => self.game_result(game, rest),
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
        self.rename_games(&mut model);
        model
    }

    /// Whether `site` of the renamed file names what is renamed itself: its
    /// key, or a game's, a player's or a strategy's places in `games:`
    fn names_it(&self, site: &text::Site, model: &ParsedModel) -> Result<bool, String> {
        let key = |path: Vec<String>| Ok(site.key && site.path == path);
        match &self.entity {
            Entity::Scalar(name) => key(name.split('.').map(str::to_string).collect()),
            Entity::Group(name) | Entity::Table(name) => key(vec![name.clone()]),
            Entity::Column(table, column) => key(vec![table.clone(), column.clone()]),
            _ => self.game_site(site, model),
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
        let mut keys = 0;
        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        for site in text::sites(text)? {
            let raw = &text[site.range.clone()];
            if own && self.names_it(&site, node.model)? {
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
        if own && keys == 0 {
            return Err("it is not written where edit can rename it".into());
        }
        Ok(text::splice(text, edits))
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
    let after = model_check::read(&changed, main)
        .and_then(|after| model_check::valid(&after).map(|()| after))
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

    pub(super) fn files() -> HashMap<String, String> {
        HashMap::from([
            ("model.yaml".to_string(), MODEL.to_string()),
            ("costs.yaml".to_string(), COSTS.to_string()),
        ])
    }

    pub(super) fn rename(path: &str, new: &str) -> Change {
        Change {
            op: Op::Rename,
            path: path.into(),
            value: None,
            formula: None,
            unit: None,
            after: None,
            new_name: Some(new.into()),
            plays: None,
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

    /// A name a network's prior reads is renamed there too (ADR-074):
    /// `examples/expected_loss.yaml`, whose economy reads `outlook.p_bad`
    #[test]
    fn a_scalar_a_networks_prior_reads_is_renamed_in_the_prior() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/expected_loss.yaml"),
        )
        .unwrap();
        let files = HashMap::from([("model.yaml".to_string(), text.clone())]);
        let changed = run(&files, rename("outlook.p_bad", "p_crash")).unwrap();
        let after = changed["model.yaml"].clone();
        assert!(
            after.contains(
                "prior: [\"=1 - outlook.p_neutral - outlook.p_crash\", \"=outlook.p_neutral\", \"=outlook.p_crash\"]"
            ),
            "{after}"
        );
        assert!(!after.contains("p_bad"), "{after}");
        // Renamed back, the file is as it was
        let now: HashMap<String, String> = changed.into_iter().collect();
        let back = run(&now, rename("outlook.p_crash", "p_bad")).unwrap();
        assert_eq!(back["model.yaml"], text);
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

    /// A network's prior is a formula over the model's scalars (ADR-074):
    /// a rename reaches it, and Forge reads the model back as it was.
    #[test]
    fn a_name_read_by_a_networks_prior_is_renamed_there_too() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/expected_loss.yaml"),
        )
        .unwrap();
        let one = HashMap::from([("m.yaml".to_string(), text.clone())]);
        let changed = edit_files(&one, "m.yaml", &[rename("outlook.p_bad", "p_down")]).unwrap();
        let prior = "\"=1 - outlook.p_neutral - outlook.p_bad\", \"=outlook.p_neutral\", \"=outlook.p_bad\"";
        assert!(text.contains(prior));
        assert!(
            changed["m.yaml"]
                .contains("\"=1 - outlook.p_neutral - outlook.p_down\", \"=outlook.p_neutral\", \"=outlook.p_down\""),
            "{}",
            changed["m.yaml"]
        );
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
                plays: None,
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
        // The players of two tournaments written once (`players: &roster`,
        // then `*roster`) are refused: one key would rename a player of both
        let (roster, failures): (Vec<&String>, Vec<&String>) =
            results.iter().flat_map(|(_, f)| f).partition(|f| {
                f.contains("noisy-tournament.yaml clean.players.")
                    || f.contains("noisy-tournament.yaml noisy.players.")
            });
        assert!(
            failures.is_empty(),
            "{} failed:\n{failures:#?}",
            failures.len()
        );
        assert_eq!(roster.len(), 10, "{roster:#?}");
        assert!(renamed > 1300, "{renamed} renamed");
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
                names.extend(super::game::tests::names_of_games(&model));
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
