//! Templates command - the blocks `forge edit --insert` adds to a model
//! (ADR-075 step 6; ADR-076, decision 6).

use crate::cli::console::{out, outln};
use crate::cli::Console;
use crate::edit::templates;
use crate::error::{ForgeError, ForgeResult};

/// The templates as JSON (no printing): all of them with their blocks, or
/// the one called `name`.
///
/// # Errors
///
/// Returns why, with the names there are, when `name` is not a template.
pub fn templates_core(name: Option<&str>) -> ForgeResult<serde_json::Value> {
    let all = templates::templates();
    name.map_or_else(
        || Ok(serde_json::json!(all)),
        |name| {
            all.iter()
                .find(|t| t.name == name)
                .map(|t| serde_json::json!(t))
                .ok_or_else(|| {
                    let names: Vec<&str> = all.iter().map(|t| t.name.as_str()).collect();
                    ForgeError::Validation(format!(
                        "Unknown template '{name}'. Available: {}",
                        names.join(", ")
                    ))
                })
        },
    )
}

/// Execute the templates command: list the templates, or print one's block.
///
/// # Errors
///
/// As [`templates_core`].
///
/// # Panics
///
/// Never: the JSON of a template is text and numbers.
pub fn templates(c: &mut Console<'_>, name: Option<&str>, json_output: bool) -> ForgeResult<()> {
    let found = templates_core(name)?;
    if json_output {
        outln!(
            c,
            "{}",
            serde_json::to_string_pretty(&found).expect("a template is JSON")
        );
        return Ok(());
    }
    match found.get("yaml").and_then(serde_json::Value::as_str) {
        Some(yaml) => out!(c, "{yaml}"),
        None => {
            for t in templates::templates() {
                outln!(c, "{:<14} {}", t.name, t.line);
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::templates_core;
    use crate::cli::forge;

    #[test]
    fn the_list_names_each_template_with_its_line() {
        let ran = forge(&["templates"]);
        let both = ran.both();
        assert_eq!(ran.code, 0, "{both}");
        let lines: Vec<&str> = ran.out.lines().collect();
        assert_eq!(lines.len(), 4, "{}", ran.out);
        assert_eq!(
            lines[0],
            "game_of_two    A game of two players, played once"
        );
        assert!(lines[3].starts_with("dynamics       A game whose strategies spread"));
    }

    #[test]
    fn one_template_is_its_block_with_its_default_name() {
        let ran = forge(&["templates", "dynamics"]);
        let both = ran.both();
        assert_eq!(ran.code, 0, "{both}");
        assert!(ran
            .out
            .starts_with("  new_population:\n    description: A population of hunters"));
        assert!(ran.out.ends_with("      generations: 100\n"));
    }

    #[test]
    fn json_is_the_core_answer_and_an_unknown_name_says_what_there_is() {
        let ran = forge(&["templates", "--json"]);
        let all: serde_json::Value = serde_json::from_str(&ran.out).unwrap();
        assert_eq!(all, templates_core(None).unwrap());
        assert_eq!(all[2]["name"], "tournament");
        assert_eq!(all[2]["default"], "new_tournament");
        let one = forge(&["templates", "game_of_many", "--json"]);
        let one: serde_json::Value = serde_json::from_str(&one.out).unwrap();
        assert_eq!(one, templates_core(Some("game_of_many")).unwrap());
        let bad = forge(&["templates", "nothing"]);
        assert_eq!(bad.code, 1);
        assert!(
            bad.err.contains(
                "Unknown template 'nothing'. Available: game_of_two, game_of_many, tournament, dynamics"
            ),
            "{}",
            bad.err
        );
    }
}
