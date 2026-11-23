//! `forge edit`: Forge writes a model's text, comments kept (ADR-075). The
//! changes are made by `crate::edit` on the model's files read from disk; the
//! files that changed are written back, each with a `.bak` of how it was.

use crate::cli::console::outln;
use crate::cli::Console;
use crate::edit::{edit_files, Change, Op, Value};
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::types::ParsedModel;
use colored::Colorize;
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// What `forge edit` and `forge_edit` answer
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct EditResult {
    /// The new text of each file that changed, by its path from the model's
    /// folder (`model.yaml`, `costs.yaml`)
    pub files: BTreeMap<String, String>,
    /// Whether they were written (not under `dry_run`)
    pub written: bool,
}

/// Make `changes` to the model at `file` and the files it includes, and,
/// unless `dry_run`, write each file that changed, keeping the file as it was
/// next to it (`model.yaml.bak`).
///
/// # Errors
///
/// Returns why when the model cannot be read, a change cannot be made (then
/// no file is written), or a file cannot be written.
pub fn edit_core(file: &Path, changes: &[Change], dry_run: bool) -> ForgeResult<EditResult> {
    let dir = file.parent().unwrap_or_else(|| Path::new(""));
    let main = file
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let model = parser::parse_model(file)?;
    let mut files = HashMap::from([(main.clone(), parser::read_model_text(file)?)]);
    included_files(&model, &main, dir, &mut files);
    let new_texts = edit_files(&files, &main, changes)?;
    if !dry_run {
        for (name, text) in &new_texts {
            let path = dir.join(name);
            std::fs::copy(&path, path.with_extension("yaml.bak"))
                .and_then(|_| std::fs::write(&path, text))?;
        }
    }
    Ok(EditResult {
        written: !dry_run && !new_texts.is_empty(),
        files: new_texts,
    })
}

/// The text of each file `model` (the file `name`) includes, and of the files
/// those include, by the path the parser gives them: from the main model's
/// folder `dir`. A file that cannot be read now is empty, and the edit then
/// says that Forge cannot read the model.
fn included_files(
    model: &ParsedModel,
    name: &str,
    dir: &Path,
    files: &mut HashMap<String, String>,
) {
    let from = Path::new(name).parent().unwrap_or_else(|| Path::new(""));
    for include in &model.includes {
        let path = parser::normalize(&from.join(&include.file));
        let text = std::fs::read_to_string(dir.join(&path)).unwrap_or_default();
        files.insert(path.clone(), text);
        if let Some(included) = model.resolved_includes.get(&include.namespace) {
            included_files(&included.model, &path, dir, files);
        }
    }
}

/// `forge edit`: the changes, then each file written, or under `dry_run` its
/// new text.
///
/// # Errors
///
/// As [`edit_core`].
pub fn edit(
    c: &mut Console<'_>,
    file: &Path,
    changes: &[Change],
    dry_run: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "✏️  Forge - Editing".bold().green());
    outln!(c, "   File: {}", file.display());
    for change in changes {
        outln!(c, "   {} {}", change.op.name(), change.path.bold());
    }
    outln!(c);
    let result = edit_core(file, changes, dry_run)?;
    if result.files.is_empty() {
        outln!(
            c,
            "{}",
            "✅ Nothing to change: the files already say this".green()
        );
    }
    let dir = file.parent().unwrap_or_else(|| Path::new(""));
    for (name, text) in &result.files {
        if dry_run {
            outln!(c, "{}", format!("📋 Dry run: {name} would read").yellow());
            outln!(c, "{text}");
        } else {
            let backup = dir.join(name).with_extension("yaml.bak");
            outln!(
                c,
                "{} {name} (as it was: {})",
                "✅ Wrote".green(),
                backup.display()
            );
        }
    }
    Ok(())
}

/// The arguments of `forge edit`.
#[derive(clap::Args)]
#[command(
    about = "Change a model's text: values, formulas, names added, deleted and renamed, inputs made uncertain, a game's strategies and players, comments kept",
    long_about = "Change a model's text: set an input's value or a formula, add or delete an
input, a formula or a table's column, or rename a scalar, a group, a table, a
column, a game, a player or a strategy and every name that reads it (formulas,
scenarios, expectations, a simulation's outputs, a tornado's inputs, a game's
results, in every file of the model), or make
an input uncertain (a distribution, its value kept as the plan) and certain
again, or add or delete a game's strategy or player (a new cell is 0; a new
player comes last, with the last player's strategies), in the model or in a
file it includes (@alias.name). Every byte outside
the change stays as it was, comments and layout included. A change is made
only when Forge reads the model back valid, with that change and nothing else;
when one cannot be made, no file is written and the message says why. Each
file that changes is kept as it was next to it (model.yaml.bak). Results are
not written: forge calculate does.

A name is written as formulas read it: price, year.price, a table's column
months.sales, or @costs.rent in the file included as costs. The changes are
made in this order: --changes, --set, --formula, --add, --delete, --rename,
--uncertain, --certain, --add-strategy, --delete-strategy, --add-player,
--delete-player.

EXAMPLES:
  forge edit model.yaml --set year.price=17
  forge edit model.yaml --formula 'year.margin==year.profit / year.revenue'
  forge edit model.yaml --add year.tax=0.15 --add 'year.net==year.profit * 0.85'
  forge edit model.yaml --add fee=25 --after rate          # At the top level
  forge edit model.yaml --delete year.old_fee --dry-run
  forge edit model.yaml --rename year.price=unit_price   # And every name that reads it
  forge edit model.yaml --rename price_war.strategies.hold=keep   # A game's strategy
  forge edit model.yaml --uncertain year.price           # A triangle, 80% to 120%
  forge edit model.yaml --uncertain 'year.price==MC.PERT(12, 16, 22)'
  forge edit model.yaml --certain year.price             # Back to its value
  forge edit deal_game.yaml --add-strategy sealed_bids.strategies.bid_50   # Its cells 0
  forge edit inspection.yaml --add-strategy inspection.strategies.human.ignore
  forge edit price_war.yaml --add-player rivalry.players.copycat=tit_for_tat
  forge edit public-goods.yaml --delete-player pg.players.a5
  forge edit model.yaml --changes '[{\"op\": \"add\", \"path\": \"year.fee\", \"value\": 25, \"unit\": \"CAD\"}]'"
)]
pub struct EditArgs {
    /// The model file
    pub file: PathBuf,

    /// Set an input: PATH=VALUE (a number, or a date 2026-01-31)
    #[arg(long = "set", value_name = "PATH=VALUE")]
    pub set: Vec<String>,

    /// Set a formula: PATH==FORMULA (the formula's = after the name's)
    #[arg(long = "formula", value_name = "PATH==FORMULA")]
    pub formula: Vec<String>,

    /// Add an input, PATH=VALUE, or a formula or table column, PATH==FORMULA
    #[arg(long = "add", value_name = "PATH=VALUE")]
    pub add: Vec<String>,

    /// The top-level name an added top-level name follows
    #[arg(long, value_name = "NAME")]
    pub after: Option<String>,

    /// Delete an input, a formula or a column, with the lines under it
    #[arg(long = "delete", value_name = "PATH")]
    pub delete: Vec<String>,

    /// Rename a scalar, a group, a table, a column, a game, a player or a
    /// strategy, and every name that reads it: PATH=NAME, the new name one
    /// word (year.price=cost, rivalry.players.steady=calm)
    #[arg(long = "rename", value_name = "PATH=NAME")]
    pub rename: Vec<String>,

    /// Make an input uncertain: PATH, or PATH==DISTRIBUTION (=MC.PERT(1, 2, 3))
    #[arg(long = "uncertain", value_name = "PATH")]
    pub uncertain: Vec<String>,

    /// Make a distribution an input again: PATH, or PATH=VALUE to keep
    #[arg(long = "certain", value_name = "PATH")]
    pub certain: Vec<String>,

    /// Add a strategy to a game, its cells 0: PATH, every player's
    /// (inspection.strategies.wait) or one player's
    /// (inspection.strategies.human.ignore)
    #[arg(long = "add-strategy", value_name = "PATH")]
    pub add_strategy: Vec<String>,

    /// Delete a game's strategy, every player's of that name or one
    /// player's, with its cells
    #[arg(long = "delete-strategy", value_name = "PATH")]
    pub delete_strategy: Vec<String>,

    /// Add a player to a game, last, with the last player's strategies and
    /// payoffs of 0 (inspection.players.board); a tournament's player with
    /// its habit, PATH=HABIT (rivalry.players.copycat=random)
    #[arg(long = "add-player", value_name = "PATH")]
    pub add_player: Vec<String>,

    /// Delete a game's player, the payoffs of its first strategy kept
    #[arg(long = "delete-player", value_name = "PATH")]
    pub delete_player: Vec<String>,

    /// Changes as JSON, as Forge's MCP edit tool takes them
    #[arg(long, value_name = "JSON")]
    pub changes: Option<String>,

    /// Show the new text without writing the files
    #[arg(short = 'n', long)]
    pub dry_run: bool,
}

impl EditArgs {
    /// The changes the flags name, in the order of the help
    ///
    /// # Errors
    ///
    /// Returns why when a flag is not written PATH=..., `--changes` is not a
    /// list of changes, or no change is named.
    pub fn changes(&self) -> ForgeResult<Vec<Change>> {
        let mut changes: Vec<Change> = match &self.changes {
            Some(json) => serde_json::from_str(json)
                .map_err(|e| ForgeError::Validation(format!("--changes: {e}")))?,
            None => Vec::new(),
        };
        let pair = |flag: &str, arg: &str| -> ForgeResult<(String, String)> {
            arg.split_once('=')
                .filter(|(path, _)| !path.is_empty())
                .map(|(path, rest)| (path.to_string(), rest.to_string()))
                .ok_or_else(|| ForgeError::Validation(format!("--{flag} {arg}: write it PATH=...")))
        };
        let change = |op, path| Change {
            op,
            path,
            value: None,
            formula: None,
            unit: None,
            after: None,
            new_name: None,
            plays: None,
        };
        for arg in &self.set {
            let (path, v) = pair("set", arg)?;
            changes.push(Change {
                value: Some(Value::Text(v)),
                ..change(Op::SetValue, path)
            });
        }
        for arg in &self.formula {
            let (path, f) = pair("formula", arg)?;
            changes.push(Change {
                formula: Some(f),
                ..change(Op::SetFormula, path)
            });
        }
        for arg in &self.add {
            let (path, rest) = pair("add", arg)?;
            let (value, formula) = if rest.starts_with('=') {
                (None, Some(rest))
            } else {
                (Some(Value::Text(rest)), None)
            };
            changes.push(Change {
                value,
                formula,
                after: self.after.clone(),
                ..change(Op::Add, path)
            });
        }
        changes.extend(
            self.delete
                .iter()
                .map(|path| change(Op::Delete, path.clone())),
        );
        for arg in &self.rename {
            let (path, new_name) = pair("rename", arg)?;
            changes.push(Change {
                new_name: Some(new_name),
                ..change(Op::Rename, path)
            });
        }
        for (flag, args, op) in [
            ("uncertain", &self.uncertain, Op::MakeUncertain),
            ("certain", &self.certain, Op::MakeCertain),
        ] {
            for arg in args {
                let (path, rest) = arg
                    .split_once('=')
                    .map_or((arg.as_str(), None), |(p, r)| (p, Some(r)));
                if path.is_empty() {
                    return Err(ForgeError::Validation(format!(
                        "--{flag} {arg}: write it PATH or PATH=..."
                    )));
                }
                let given = rest.map(str::to_string);
                changes.push(if op == Op::MakeUncertain {
                    Change {
                        formula: given,
                        ..change(op, path.to_string())
                    }
                } else {
                    Change {
                        value: given.map(Value::Text),
                        ..change(op, path.to_string())
                    }
                });
            }
        }
        changes.extend(self.game_changes(change));
        if changes.is_empty() {
            return Err(ForgeError::Validation(
                "name a change: --set, --formula, --add, --delete, --rename, --uncertain, --certain, \
                 --add-strategy, --delete-strategy, --add-player, --delete-player or --changes"
                    .into(),
            ));
        }
        Ok(changes)
    }

    /// The changes of a game's strategies and players the flags name, each
    /// made by `change`
    fn game_changes(&self, change: impl Fn(Op, String) -> Change) -> Vec<Change> {
        let mut changes = Vec::new();
        for (paths, op) in [
            (&self.add_strategy, Op::AddStrategy),
            (&self.delete_strategy, Op::DeleteStrategy),
        ] {
            changes.extend(paths.iter().map(|path| change(op, path.clone())));
        }
        for arg in &self.add_player {
            let (path, plays) = arg
                .split_once('=')
                .map_or((arg.as_str(), None), |(path, habit)| {
                    (path, Some(habit.to_string()))
                });
            changes.push(Change {
                plays,
                ..change(Op::AddPlayer, path.to_string())
            });
        }
        changes.extend(
            self.delete_player
                .iter()
                .map(|path| change(Op::DeletePlayer, path.clone())),
        );
        changes
    }

    /// Run the command on `c`.
    ///
    /// # Errors
    ///
    /// As [`EditArgs::changes`] and [`edit`].
    pub fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        let changes = self.changes()?;
        edit(c, &self.file, &changes, self.dry_run)
    }
}

#[cfg(test)]
mod tests {
    use crate::cli::forge;

    const MODEL: &str = "_forge_version: \"5.1.0\"
_includes:
  - file: data/costs.yaml
    as: costs
rate: 0.05   # a rate
year:
  price: 16   # dollars
  units: 100
  revenue:
    formula: \"=year.price * year.units - @costs.rent\"
";
    const COSTS: &str =
        "_forge_version: \"5.1.0\"\n_includes:\n  - file: ../more.yaml\n    as: more\nrent: 900\n";
    const MORE: &str = "_forge_version: \"5.1.0\"\nspare: 1\n";

    /// A folder with the model, a file it includes and one that includes
    fn folder() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("data")).unwrap();
        std::fs::write(dir.path().join("model.yaml"), MODEL).unwrap();
        std::fs::write(dir.path().join("data/costs.yaml"), COSTS).unwrap();
        std::fs::write(dir.path().join("more.yaml"), MORE).unwrap();
        dir
    }

    fn read(dir: &tempfile::TempDir, name: &str) -> String {
        std::fs::read_to_string(dir.path().join(name)).unwrap()
    }

    #[test]
    fn each_flag_makes_its_change_and_the_files_are_written_with_a_backup() {
        let dir = folder();
        let model = dir.path().join("model.yaml");
        let ran = forge(&[
            "edit",
            model.to_str().unwrap(),
            "--set",
            "year.price=17",
            "--formula",
            "year.revenue==year.price * year.units",
            "--add",
            "year.tax=0.15",
            "--add",
            "year.net==year.revenue * 0.85",
            "--set",
            "@costs.rent=950",
            "--delete",
            "@costs.@more.nothing",
        ]);
        assert_eq!(ran.code, 1, "{}", ran.both());
        assert!(
            ran.err
                .contains("delete @costs.@more.nothing: it is not in the model (more.yaml)"),
            "{}",
            ran.err
        );
        // Nothing was written
        assert_eq!(read(&dir, "model.yaml"), MODEL);
        assert!(!dir.path().join("model.yaml.bak").exists());

        let ran = forge(&[
            "edit",
            model.to_str().unwrap(),
            "--set",
            "year.price=17",
            "--formula",
            "year.revenue==year.price * year.units",
            "--add",
            "year.tax=0.15",
            "--add",
            "year.net==year.revenue * 0.85",
            "--set",
            "@costs.rent=950",
        ]);
        assert_eq!(ran.code, 0, "{}", ran.both());
        assert!(ran.out.contains("   set_value year.price\n"), "{}", ran.out);
        assert!(
            ran.out.contains("✅ Wrote data/costs.yaml (as it was: "),
            "{}",
            ran.out
        );
        let want = MODEL
            .replace("price: 16", "price: 17")
            .replace(" - @costs.rent", "")
            + "  tax: 0.15\n  net:\n    formula: \"=year.revenue * 0.85\"\n";
        assert_eq!(read(&dir, "model.yaml"), want);
        assert_eq!(read(&dir, "model.yaml.bak"), MODEL);
        assert_eq!(read(&dir, "data/costs.yaml"), COSTS.replace("900", "950"));
        assert_eq!(read(&dir, "data/costs.yaml.bak"), COSTS);
        assert_eq!(read(&dir, "more.yaml"), MORE);
    }

    #[test]
    fn a_dry_run_shows_the_new_text_and_writes_nothing() {
        let dir = folder();
        let model = dir.path().join("model.yaml");
        let ran = forge(&[
            "edit",
            model.to_str().unwrap(),
            "-n",
            "--add",
            "fee=25",
            "--after",
            "rate",
        ]);
        assert_eq!(ran.code, 0, "{}", ran.both());
        assert!(
            ran.out.contains("📋 Dry run: model.yaml would read\n"),
            "{}",
            ran.out
        );
        assert!(
            ran.out.contains("rate: 0.05   # a rate\nfee: 25\nyear:"),
            "{}",
            ran.out
        );
        assert_eq!(read(&dir, "model.yaml"), MODEL);
        // A value as it is already: nothing to change
        let same = forge(&["edit", model.to_str().unwrap(), "--set", "year.units=100"]);
        assert!(
            same.out
                .contains("✅ Nothing to change: the files already say this"),
            "{}",
            same.out
        );
        assert!(!dir.path().join("model.yaml.bak").exists());
    }

    #[test]
    fn a_rename_renames_every_name_that_reads_it_in_every_file() {
        let dir = folder();
        let model = dir.path().join("model.yaml");
        let ran = forge(&[
            "edit",
            model.to_str().unwrap(),
            "--rename",
            "year.price=unit_price",
            "--rename",
            "@costs.rent=lease",
        ]);
        assert_eq!(ran.code, 0, "{}", ran.both());
        assert!(ran.out.contains("   rename year.price\n"), "{}", ran.out);
        assert_eq!(
            read(&dir, "model.yaml"),
            MODEL
                .replace("price: 16", "unit_price: 16")
                .replace("year.price", "year.unit_price")
                .replace("@costs.rent", "@costs.lease")
        );
        assert_eq!(
            read(&dir, "data/costs.yaml"),
            COSTS.replace("rent:", "lease:")
        );
    }

    #[test]
    fn an_input_is_made_uncertain_and_certain_again() {
        let dir = folder();
        let model = dir.path().join("model.yaml");
        let ran = forge(&[
            "edit",
            model.to_str().unwrap(),
            "--uncertain",
            "year.price",
            "--uncertain",
            "year.units==MC.PERT(80, 100, 130)",
        ]);
        assert_eq!(ran.code, 0, "{}", ran.both());
        assert!(
            ran.out.contains("   make_uncertain year.price\n"),
            "{}",
            ran.out
        );
        let made = read(&dir, "model.yaml");
        assert!(
            made.contains("  price:\n    value: 16   # dollars\n    formula: \"=MC.Triangular(12.8, 16, 19.2)\"\n"),
            "{made}"
        );
        assert!(
            made.contains("formula: \"=MC.PERT(80, 100, 130)\"\n"),
            "{made}"
        );
        assert!(made.ends_with("    - variable: year.revenue\n"), "{made}");
        let ran = forge(&[
            "edit",
            model.to_str().unwrap(),
            "--certain",
            "year.price",
            "--certain",
            "year.units=90",
        ]);
        assert_eq!(ran.code, 0, "{}", ran.both());
        let back = read(&dir, "model.yaml");
        assert!(
            back.contains("  price: 16   # dollars\n  units: 90\n"),
            "{back}"
        );
    }

    #[test]
    fn a_games_strategy_and_players_are_added_and_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("game.yaml");
        let text = "_forge_version: \"5.6.0\"
games:
  pd:
    players: [me, you]
    strategies: [cooperate, defect]
    symmetric: true
    payoffs:
      cooperate: {cooperate: 3, defect: 0}
      defect: {cooperate: 5, defect: 1}
  duel:
    tournament: {round: pd, players: {a: tit_for_tat, b: always_defect}, rounds: 10}
  pair:
    players: [x, y]
    strategies: [up, down]
    payoffs: {up: {up: [1, 1], down: [0, 0]}, down: {up: [0, 0], down: [2, 2]}}
";
        std::fs::write(&model, text).unwrap();
        let path = model.to_str().unwrap();
        let ran = forge(&[
            "edit",
            path,
            "--add-strategy",
            "pair.strategies.wait",
            "--add-player",
            "duel.players.c=random",
            "--add-player",
            "pair.players.z",
            "--delete-player",
            "duel.players.a",
        ]);
        let both = ran.both();
        assert_eq!(ran.code, 0, "{both}");
        assert!(
            ran.out.contains("   add_player duel.players.c\n"),
            "{}",
            ran.out
        );
        let made = read(&dir, "game.yaml");
        assert!(
            made.contains("players: {b: always_defect, c: random}"),
            "{made}"
        );
        assert!(
            made.contains("    players: [x, y, z]\n    strategies: [up, down, wait]\n"),
            "{made}"
        );
        assert!(
            made.contains("payoffs: {up: {up: {up: [1, 1, 0], down: [1, 1, 0], wait: [1, 1, 0]}"),
            "{made}"
        );
        let ran = forge(&[
            "edit",
            path,
            "--delete-strategy",
            "pair.strategies.wait",
            "--delete-player",
            "pair.players.z",
        ]);
        let both = ran.both();
        assert_eq!(ran.code, 0, "{both}");
        assert_eq!(
            read(&dir, "game.yaml"),
            text.replace(
                "{a: tit_for_tat, b: always_defect}",
                "{b: always_defect, c: random}"
            )
        );
    }

    #[test]
    fn changes_can_be_given_as_json() {
        let dir = folder();
        let model = dir.path().join("model.yaml");
        let json = r#"[{"op": "add", "path": "year.fee", "value": 25, "unit": "CAD"}]"#;
        let ran = forge(&["edit", model.to_str().unwrap(), "--changes", json]);
        assert_eq!(ran.code, 0, "{}", ran.both());
        assert_eq!(
            read(&dir, "model.yaml"),
            format!("{MODEL}  fee:\n    value: 25\n    unit: CAD\n")
        );
    }

    #[test]
    fn a_change_written_wrong_is_refused_before_any_file_is_read() {
        let dir = folder();
        let model = dir.path().join("model.yaml");
        let model = model.to_str().unwrap();
        let refused = |args: &[&str], why: &str| {
            let ran = forge(&[&["edit", model][..], args].concat());
            assert_eq!(ran.code, 1, "{args:?}");
            assert!(ran.err.contains(why), "{args:?}: {}", ran.err);
        };
        refused(
            &[],
            "name a change: --set, --formula, --add, --delete, --rename, --uncertain, --certain, \
             --add-strategy, --delete-strategy, --add-player, --delete-player or --changes",
        );
        refused(
            &["--uncertain", "=x"],
            "--uncertain =x: write it PATH or PATH=...",
        );
        refused(&["--rename", "rate"], "--rename rate: write it PATH=...");
        refused(&["--set", "price"], "--set price: write it PATH=...");
        refused(&["--set", "=17"], "--set =17: write it PATH=...");
        refused(&["--formula", "x"], "--formula x: write it PATH=...");
        refused(&["--add", "x"], "--add x: write it PATH=...");
        refused(&["--changes", "{"], "--changes: ");
        refused(
            &["--formula", "year.revenue=year.price"],
            "year.price is not a formula",
        );
        assert_eq!(read(&dir, "model.yaml"), MODEL);
    }

    #[test]
    fn a_file_that_cannot_be_written_is_an_error_and_a_missing_model_too() {
        let dir = folder();
        let model = dir.path().join("model.yaml");
        // The backup's place is taken by a folder
        std::fs::create_dir(dir.path().join("model.yaml.bak")).unwrap();
        let ran = forge(&["edit", model.to_str().unwrap(), "--set", "rate=0.06"]);
        assert_eq!(ran.code, 1, "{}", ran.both());
        assert!(ran.err.starts_with("Error: IO error: "), "{}", ran.err);
        let missing = dir.path().join("none.yaml");
        let ran = forge(&["edit", missing.to_str().unwrap(), "--set", "rate=0.06"]);
        assert_eq!(ran.code, 1, "{}", ran.both());
    }

    #[test]
    fn an_included_file_that_is_gone_says_the_model_cannot_be_read() {
        let dir = folder();
        let mut files = std::collections::HashMap::new();
        let model = crate::parser::parse_model(&dir.path().join("model.yaml")).unwrap();
        std::fs::remove_file(dir.path().join("more.yaml")).unwrap();
        super::included_files(&model, "model.yaml", dir.path(), &mut files);
        assert_eq!(files["data/costs.yaml"], COSTS);
        assert_eq!(files["more.yaml"], "");
    }
}
