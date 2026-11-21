//! A YAML merge key (`<<`) is refused, the same way wherever it is written.
//!
//! YAML 1.1 read `<<: *base` as "the keys of `base`, then these". YAML 1.2
//! dropped it, and Forge does not read it (ADR-071): a model means what its
//! keys say, in every reader. Left to the parts of a model, a merge key was
//! refused differently in each (a tournament's players not "any of the forms
//! allowed here", a payoff row with a strategy named `<<`, a scenario with no
//! key `<<`), and a section a command deserialises itself could drop it
//! without a word. So every model's text becomes YAML through [`read_yaml`],
//! which refuses one with one message, naming its place.
//!
//! An alias (`players: *roster`) reuses a whole block and is read as before.

use serde::de::Error as _;
use yaml_serde::Value;

/// The key YAML 1.1 read as a merge.
const MERGE_KEY: &str = "<<";

/// Read `text` as YAML, refusing a merge key (`<<`) anywhere in it.
///
/// Every model text Forge reads becomes YAML here, so the refusal is the same
/// in the model, an included file, each document of a multi-document file and
/// the analysis sections (`monte_carlo:`, `decision_tree:` and the others).
///
/// # Errors
///
/// Returns the YAML error if the text is not YAML, or an error naming the first
/// merge key and where it is.
pub fn read_yaml(text: &str) -> Result<Value, yaml_serde::Error> {
    let yaml: Value = yaml_serde::from_str(text)?;
    merge_key_in(&yaml, "").map_or(Ok(yaml), |place| {
        Err(yaml_serde::Error::custom(refused(&place)))
    })
}

/// The place of the first merge key in `value`, under `path`.
fn merge_key_in(value: &Value, path: &str) -> Option<String> {
    match value {
        Value::Mapping(map) => map.iter().find_map(|(key, item)| {
            let key = key.as_str().unwrap_or_default();
            let inner = if path.is_empty() {
                key.to_string()
            } else {
                format!("{path}.{key}")
            };
            if key == MERGE_KEY {
                Some(inner)
            } else {
                merge_key_in(item, &inner)
            }
        }),
        Value::Sequence(items) => (1..)
            .zip(items)
            .find_map(|(row, item)| merge_key_in(item, &format!("{path} row {row}"))),
        Value::Tagged(tagged) => merge_key_in(&tagged.value, path),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => None,
    }
}

fn refused(place: &str) -> String {
    format!(
        "'{place}' is a YAML merge key (<<), which Forge does not read: YAML 1.2 dropped \
         merge keys. Write the keys out (an alias such as `players: *roster` still reuses a \
         whole block)"
    )
}

#[cfg(test)]
mod tests {
    use super::read_yaml;

    fn refused(yaml: &str) -> String {
        read_yaml(yaml).unwrap_err().to_string()
    }

    /// A merge key is refused wherever it is, by its place: at the top (also
    /// quoted, as YAML does not keep the quotes), in a mapping, in a list's
    /// item, a list of merges, and under a tag
    #[test]
    fn a_merge_key_is_refused_by_its_place() {
        for (yaml, place) in [
            ("base: &b {x: 1}\n<<: *b\n", "<<"),
            ("\"<<\": 1\n", "<<"),
            (
                "base: &b {x: 1}\nscenarios:\n  more:\n    <<: *b\n    y: 2\n",
                "scenarios.more.<<",
            ),
            ("a: &a {x: 1}\nt:\n  - <<: *a\n", "t row 1.<<"),
            ("a: &a {x: 1}\nb: &b {y: 2}\nc:\n  <<: [*a, *b]\n", "c.<<"),
            ("a: &a {x: 1}\nt: !k\n  <<: *a\n", "t.<<"),
        ] {
            let error = refused(yaml);
            assert!(
                error.contains(&format!(
                    "'{place}' is a YAML merge key (<<), which Forge does not read"
                )),
                "{yaml}: {error}"
            );
        }
    }

    /// Anchors and aliases, a `<<` that is a value and not a key, a key that
    /// is not text, and everything else YAML holds are read as before
    #[test]
    fn what_is_not_a_merge_key_is_read() {
        for yaml in [
            "players: &roster [a, b]\nagain: *roster\n",
            "op: <<\nlist: [\"<<\", 1, 2.5, true, ~]\n",
            "1: 2\nt: !k [1]\n",
        ] {
            assert!(read_yaml(yaml).is_ok(), "{yaml}");
        }
        let yaml = read_yaml("players: &roster [a, b]\nagain: *roster\n").unwrap();
        assert_eq!(yaml["again"], yaml["players"]);
    }

    /// The three places the roadmap item names (a tournament's players, a
    /// payoff row, a scenario) give the same refusal through the commands
    /// that read them, as do an included file, a document of a
    /// multi-document file and an analysis section a command reads itself;
    /// each was refused in its own words, or not at all
    #[test]
    fn every_place_gives_the_same_refusal() {
        let game = "_forge_version: \"5.5.0\"\ngames:\n  dilemma:\n    players: [me, you]\n    \
                    strategies: [cooperate, defect]\n    symmetric: true\n    payoffs:\n";
        let rows =
            "      cooperate: {cooperate: 3, defect: 0}\n      defect: {cooperate: 5, defect: 1}\n";
        let tournament =
            "  wide:\n    tournament:\n      round: dilemma\n      rounds: 10\n      players:\n";
        let scalars = "_forge_version: \"5.1.0\"\nprice: 10\nunits: 5\n";
        let analysis =
            "\ndecision_tree:\n  name: d\n  root:\n    type: decision\n    name: go\n    \
                        branches:\n      yes: &yes {value: 10}\n      no:\n        <<: *yes\n";
        let dir = tempfile::tempdir().unwrap();
        let path = |name: &str, text: &str| {
            let file = dir.path().join(name);
            std::fs::write(&file, text).unwrap();
            file.display().to_string()
        };
        let included = path(
            "base.yaml",
            &format!("{scalars}low: &low {{price: 8}}\nlower:\n  <<: *low\n"),
        );
        for (command, text, place) in [
            (
                "game",
                format!("{game}{rows}{tournament}        base: &base {{tft: tit_for_tat}}\n        <<: *base\n        ad: always_defect\n"),
                "games.wide.tournament.players.<<",
            ),
            (
                "game",
                format!("{game}      cooperate: &row {{cooperate: 3, defect: 0}}\n      defect:\n        <<: *row\n        cooperate: 5\n"),
                "games.dilemma.payoffs.defect.<<",
            ),
            (
                "scenarios",
                format!("{scalars}scenarios:\n  low: &low\n    price: 8\n  more:\n    <<: *low\n    units: 6\n"),
                "scenarios.more.<<",
            ),
            (
                "calculate",
                format!("{scalars}_includes:\n  - file: base.yaml\n    as: base\n"),
                "lower.<<",
            ),
            (
                "calculate",
                format!("---\n{scalars}_name: one\n---\n_forge_version: \"5.1.0\"\n_name: two\nx: &x {{value: 1}}\ny:\n  <<: *x\n"),
                "y.<<",
            ),
            (
                "decision-tree",
                format!("{scalars}{analysis}"),
                "decision_tree.root.branches.no.<<",
            ),
        ] {
            let file = path("m.yaml", &text);
            let ran = crate::cli::forge(&[command, &file]);
            assert_eq!(ran.code, 1, "{command} {text}: {}", ran.err);
            assert!(
                ran.err.contains(&format!(
                    "'{place}' is a YAML merge key (<<), which Forge does not read: YAML 1.2 \
                     dropped merge keys. Write the keys out"
                )),
                "{command} {text}: {}",
                ran.err
            );
        }
        assert!(included.ends_with("base.yaml"));
    }

    /// Text that is not YAML gives the YAML error, as before
    #[test]
    fn text_that_is_not_yaml_gives_the_yaml_error() {
        let error = refused("a: [1, 2\n");
        assert!(!error.contains("merge key"), "{error}");
    }
}
