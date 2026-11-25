//! The examples in `forge <command> --help` are ones Forge accepts.
//!
//! A help text once showed a Bayesian network whose nodes were not the ones its
//! example queried, and no `_forge_version`: the model was refused as shown.
//! Every command `forge --help` lists is read here, and:
//!
//! - each example line names a command Forge has and flags that command's
//!   help lists;
//! - for the commands whose YAML CONFIGURATION is a whole model (a Bayesian
//!   network, a bootstrap, a decision tree, real options), the block is
//!   written to a file and every example that does not need another
//!   scenario runs on it, and must succeed.
//!
//! The other commands' blocks are sections of a model that needs scalars of
//! its own, so only their example lines are checked.

use crate::readme_commands::rejoin_quotes;
use std::path::Path;

/// The commands `forge --help` lists under COMMANDS.
#[must_use]
pub fn commands(top: &str) -> Vec<String> {
    top.lines()
        .skip_while(|l| *l != "COMMANDS:")
        .skip(1)
        .take_while(|l| !l.is_empty())
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect()
}

/// The lines under a header (`EXAMPLES:`), up to the next header.
#[must_use]
pub fn section<'a>(help: &'a str, headers: &[&str]) -> Vec<&'a str> {
    help.lines()
        .skip_while(|l| !headers.contains(l))
        .skip(1)
        .take_while(|l| l.is_empty() || l.starts_with(' '))
        .collect()
}

/// The example lines of a help text: those that start with `forge`, without
/// their `# comment`.
#[must_use]
pub fn examples(help: &str) -> Vec<String> {
    section(help, &["EXAMPLES:", "EXAMPLE:"])
        .into_iter()
        .map(str::trim)
        .filter(|l| l.starts_with("forge ") || l.contains(" forge "))
        .map(|l| l.split(" #").next().unwrap_or(l).trim_end().to_string())
        .collect()
}

/// The YAML CONFIGURATION block, without its indent, up to the first blank
/// line; empty if the help has none.
#[must_use]
pub fn yaml_block(help: &str) -> String {
    let mut out = String::new();
    for line in section(help, &["YAML CONFIGURATION:"])
        .into_iter()
        .take_while(|l| !l.is_empty())
    {
        out.push_str(line.strip_prefix("  ").unwrap_or(line));
        out.push('\n');
    }
    out
}

/// The models of `WRITING A FORMULA:` in `forge calculate --help`, without
/// their indent: the blocks of its section that start with a group's name.
#[must_use]
pub fn formula_models(help: &str) -> String {
    let text = section(help, &["WRITING A FORMULA:"]).join("\n");
    text.split("\n\n")
        .filter(|block| {
            block
                .trim_start()
                .lines()
                .next()
                .is_some_and(|l| l.ends_with(':') && !l.contains(' '))
        })
        .map(|block| {
            block
                .lines()
                .map(|l| l.strip_prefix("  ").unwrap_or(l))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The arguments of an example after `forge`, the quoted ones joined.
#[must_use]
pub fn arguments(example: &str) -> Vec<String> {
    let words: Vec<&str> = example.split_whitespace().collect();
    let at = words.iter().position(|w| *w == "forge").unwrap_or(0);
    rejoin_quotes(&words[at + 1..])
}

/// Whether a help text lists the flag (`-n` or `--iterations`).
#[must_use]
pub fn lists_flag(help: &str, flag: &str) -> bool {
    help.split(|c: char| c.is_whitespace() || c == ',' || c == '=' || c == '<')
        .any(|w| w == flag)
}

/// The arguments with `model.yaml` replaced by `model` and the file an
/// option writes by a file in `dir`, so a check writes nothing in the
/// repository.
#[must_use]
pub fn in_dir(args: &[String], model: &Path, dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        let writes = i > 0 && (args[i - 1] == "-o" || args[i - 1] == "--output");
        out.push(if arg == "model.yaml" {
            model.display().to_string()
        } else if writes {
            dir.join(arg).display().to_string()
        } else {
            arg.clone()
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{forge, forge_in};

    /// The commands whose YAML CONFIGURATION is a whole model.
    const WHOLE_MODELS: [&str; 4] = ["bayesian", "bootstrap", "decision-tree", "real-options"];

    fn help(command: &str) -> String {
        forge(&[command, "--help"]).printed()
    }

    #[test]
    fn the_parts_of_a_help_text_are_found() {
        let top =
            "Forge\n\nCOMMANDS:\n  calculate     - Execute\n  edit - Change\n\nEXAMPLES:\n  x\n";
        assert_eq!(commands(top), ["calculate", "edit"]);
        assert_eq!(commands("nothing"), Vec::<String>::new());
        let text = "About\n\nYAML CONFIGURATION:\n  a:\n    b: 1\n\n  prose\n\nEXAMPLES:\n  forge x model.yaml -n 5   # note\n  -> a comment\n  FORGE_API_TOKEN=t forge serve\n\nUsage: forge x\n";
        assert_eq!(yaml_block(text), "a:\n  b: 1\n");
        assert_eq!(yaml_block("About\n"), "");
        assert_eq!(
            examples(text),
            ["forge x model.yaml -n 5", "FORGE_API_TOKEN=t forge serve"]
        );
        assert_eq!(
            examples("EXAMPLE:\n  forge y\n\nOUTPUT:\n  forge z\n"),
            ["forge y"]
        );
        assert_eq!(examples("nothing"), Vec::<String>::new());
    }

    #[test]
    fn arguments_are_what_follows_forge_with_quotes_joined() {
        assert_eq!(
            arguments(r#"forge simulate model.yaml --seek "P(a > 0) = 0.8" -n 5"#),
            [
                "simulate",
                "model.yaml",
                "--seek",
                "P(a > 0) = 0.8",
                "-n",
                "5"
            ]
        );
        assert_eq!(
            arguments("T=1 forge serve --host h"),
            ["serve", "--host", "h"]
        );
    }

    #[test]
    fn a_flag_is_listed_when_the_help_names_it() {
        let text = "  -n, --iterations <N>\n  --seed=<S>\n";
        assert!(lists_flag(text, "-n"));
        assert!(lists_flag(text, "--iterations"));
        assert!(lists_flag(text, "--seed"));
        assert!(!lists_flag(text, "--iteration"));
        assert!(!lists_flag(text, "-m"));
    }

    #[test]
    fn a_check_writes_only_in_its_directory() {
        let args: Vec<String> = [
            "x",
            "model.yaml",
            "-o",
            "out.json",
            "--output",
            "o.yaml",
            "-n",
        ]
        .map(String::from)
        .into();
        let out = in_dir(&args, Path::new("/m/model.yaml"), Path::new("/d"));
        assert_eq!(
            out,
            [
                "x",
                "/m/model.yaml",
                "-o",
                "/d/out.json",
                "--output",
                "/d/o.yaml",
                "-n"
            ]
        );
    }

    #[test]
    fn every_example_names_a_command_and_flags_forge_has() {
        let top = forge(&["--help"]).printed();
        let all = commands(&top);
        assert!(all.len() >= 25, "{all:?}");
        let mut checked = 0;
        for command in &all {
            let text = help(command);
            for example in examples(&text) {
                let args = arguments(&example);
                assert!(
                    all.contains(&args[0]),
                    "`{example}` in `forge {command} --help` names no command"
                );
                let name = &args[0];
                let own = help(name);
                for flag in args.iter().filter(|a| {
                    a.starts_with("--")
                        || (a.starts_with('-') && a[1..].starts_with(|c: char| c.is_alphabetic()))
                }) {
                    let flag = flag.split('=').next().unwrap_or(flag);
                    assert!(
                        lists_flag(&own, flag),
                        "`{example}` in `forge {command} --help` uses {flag}, which `forge {name} --help` does not list"
                    );
                }
                checked += 1;
            }
        }
        assert!(checked >= 60, "only {checked} examples checked");
    }

    #[test]
    fn the_models_the_help_shows_run_with_their_examples() {
        let dir = tempfile::tempdir().unwrap();
        let mut ran = 0;
        for command in WHOLE_MODELS {
            let text = help(command);
            let mut model = yaml_block(&text);
            assert!(model.contains(':'), "`forge {command} --help` has no model");
            if !model.contains("_forge_version") {
                model = format!("_forge_version: \"5.6.0\"\n{model}");
            }
            let path = dir.path().join(format!("{command}.yaml"));
            std::fs::write(&path, model).unwrap();
            for example in examples(&text) {
                let args = arguments(&example);
                // A scenario the block does not have
                if args.contains(&"--scenario".to_string()) {
                    continue;
                }
                let args = in_dir(&args, &path, dir.path());
                let out = forge(&args);
                assert!(
                    out.success(),
                    "`{example}` fails on the model `forge {command} --help` shows:\n{}{}",
                    out.out,
                    out.err
                );
                ran += 1;
            }
        }
        assert!(ran >= 8, "only {ran} examples ran");
    }

    #[test]
    fn the_formula_forms_calculate_help_shows_calculate_as_it_says() {
        let text = "About\n\nWRITING A FORMULA:\n  A scalar:\n\n  a:\n    x: 1\n\n  Then it reads:\n\n  b:\n    y: 2\n\nOTHER:\n  c:\n";
        assert_eq!(formula_models(text), "a:\n  x: 1\nb:\n  y: 2");
        assert_eq!(formula_models("About\n"), "");
        let dir = tempfile::tempdir().unwrap();
        let model = format!(
            "_forge_version: \"5.1.0\"\n{}\n",
            formula_models(&help("calculate"))
        );
        let path = dir.path().join("m.yaml");
        std::fs::write(&path, model).unwrap();
        let out = forge_in(dir.path(), &["calculate", "m.yaml"]).printed();
        assert!(out.contains("plan.revenue = 480"), "{out}");
        assert!(out.contains("total = [2, 4]"), "{out}");
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains(r#"revenue: {formula: "=plan.price * 12", value: 480}"#));
    }
}
