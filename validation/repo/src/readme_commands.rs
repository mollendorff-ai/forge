//! Every command the README shows is run, and prints what the README shows.
//!
//! A command is a `text` block that starts with `$ forge`, and every line the
//! README shows as its output must be a line the command prints (roadmap
//! T1-CLAIMS). The README once showed `397.04 → 80,000.00`
//! for a command that printed something else.
//!
//! A block may show only some of the lines ("A few of its lines"), so a line
//! of the README must appear in the output, not the other way round. A block
//! without a command shows more lines of the command above it.
//!
//! Every command runs, `forge simulate --seek` too: 22 simulations of 5,000
//! iterations, seconds with a release binary and over a minute with a debug
//! one. CI runs these checks on the release binary of the Build job.

/// The `text` code blocks of a markdown file, as lines.
#[must_use]
pub fn text_blocks(doc: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut lines = doc.lines();
    while let Some(line) = lines.next() {
        if line.trim() == "```text" {
            let mut block = Vec::new();
            for inner in lines.by_ref() {
                if inner.starts_with("```") {
                    break;
                }
                block.push(inner.to_string());
            }
            out.push(block);
        }
    }
    out
}

/// Words separated by single spaces, so a changed column width is not a change.
#[must_use]
pub fn squeeze(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Join the words of a `"..."` argument back into one.
#[must_use]
pub fn rejoin_quotes(words: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let mut open: Option<String> = None;
    for word in words {
        match open.take() {
            Some(mut text) => {
                text.push(' ');
                text.push_str(word.trim_end_matches('"'));
                if word.ends_with('"') {
                    out.push(text);
                } else {
                    open = Some(text);
                }
            },
            None if word.starts_with('"') => {
                let inner = word.trim_start_matches('"');
                if inner.ends_with('"') {
                    out.push(inner.trim_end_matches('"').to_string());
                } else {
                    open = Some(inner.to_string());
                }
            },
            None => out.push((*word).to_string()),
        }
    }
    out
}

/// Whether `text` holds a date written `2026-11-17`.
#[must_use]
pub fn has_date(text: &str) -> bool {
    text.as_bytes().windows(10).any(|w| {
        w.iter().enumerate().all(|(i, b)| {
            if i == 4 || i == 7 {
                *b == b'-'
            } else {
                b.is_ascii_digit()
            }
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forge;

    /// Runs a command: whether it succeeded, and the lines it printed
    type Run = dyn Fn(&[String]) -> (bool, Vec<String>);

    /// What the `blocks` of the README show that its commands do not print:
    /// how many commands ran, how many lines were checked, and what is wrong.
    /// `run` runs a command and gives whether it succeeded and the lines it
    /// printed.
    fn check_blocks(blocks: &[Vec<String>], run: &Run) -> (usize, usize, Vec<String>) {
        let mut commands = 0;
        let mut lines_checked = 0;
        let mut wrong = Vec::new();
        let mut printed: Vec<String> = Vec::new();
        for block in blocks {
            let (shown, label) = match block.first().and_then(|l| l.strip_prefix("$ forge ")) {
                Some(args) => {
                    let args: Vec<&str> = args.split_whitespace().collect();
                    // The shown arguments are quoted as a shell would: "P(year.profit > 0) = 0.8"
                    let args = rejoin_quotes(&args);
                    let (success, lines) = run(&args);
                    if !success {
                        wrong.push(format!(
                            "`forge {}` fails:\n{}",
                            args.join(" "),
                            lines.join("\n")
                        ));
                        continue;
                    }
                    printed = lines.iter().map(|l| squeeze(l)).collect();
                    commands += 1;
                    (&block[1..], format!("forge {}", args.join(" ")))
                },
                None if commands > 0 => (&block[..], "the command above".to_string()),
                None => continue,
            };
            for line in shown {
                let wanted = squeeze(line);
                if wanted.is_empty() {
                    continue;
                }
                if printed.contains(&wanted) {
                    lines_checked += 1;
                } else {
                    wrong.push(format!(
                        "README shows a line that `{label}` does not print:\n  {wanted}\nIt prints:\n{}",
                        printed.join("\n")
                    ));
                }
            }
        }
        (commands, lines_checked, wrong)
    }

    #[test]
    fn every_command_in_the_readme_prints_what_the_readme_shows() {
        let doc = crate::read("README.md");
        let (commands, lines_checked, wrong) = check_blocks(&text_blocks(&doc), &|args| {
            let out = forge(args);
            let lines = out.out.lines().chain(out.err.lines()).map(str::to_string);
            (out.success(), lines.collect())
        });
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        assert!(
            commands >= 7,
            "only {commands} commands found in the README"
        );
        assert!(lines_checked >= 20, "only {lines_checked} lines checked");
    }

    #[test]
    fn a_readme_block_that_a_command_does_not_print_is_named() {
        let block = |lines: &[&str]| lines.iter().map(|l| (*l).to_string()).collect::<Vec<_>>();
        let blocks = [
            // Before any command: nothing to check it against
            block(&["not a command"]),
            block(&["$ forge ok", "  one   line", "", "gone"]),
            // A block with no command shows more of the command above
            block(&["one line", "another"]),
            block(&["$ forge bad --flag"]),
        ];
        let run = |args: &[String]| {
            if args[0] == "ok" {
                (true, vec!["one line".to_string()])
            } else {
                (false, vec!["boom".to_string()])
            }
        };
        let (commands, checked, wrong) = check_blocks(&blocks, &run);
        assert_eq!((commands, checked), (1, 2));
        assert_eq!(wrong.len(), 3, "{wrong:#?}");
        assert!(
            wrong[0].contains("`forge ok` does not print:\n  gone"),
            "{wrong:#?}"
        );
        assert!(
            wrong[1].contains("`the command above` does not print:\n  another"),
            "{wrong:#?}"
        );
        assert_eq!(wrong[2], "`forge bad --flag` fails:\nboom");
    }

    #[test]
    fn a_word_in_quotes_is_one_argument_whole_or_in_pieces() {
        assert_eq!(
            rejoin_quotes(&["\"one\"", "\"two", "words\"", "plain"]),
            ["one", "two words", "plain"]
        );
    }

    /// A development command the README says runs "as CI runs it" is a command
    /// CI runs, word for word (roadmap A0-DOC-ERRORS): the README once showed a
    /// lint weaker than CI's, so a change could pass it and fail CI.
    #[test]
    fn the_readmes_development_commands_are_cis() {
        let readme = crate::read("README.md");
        let ci = crate::read(".github/workflows/ci.yml");
        let ran: Vec<&str> = ci
            .lines()
            .map(|l| l.trim().trim_start_matches("run: "))
            .collect();
        let claimed: Vec<&str> = readme
            .lines()
            .filter(|l| l.contains("# ") && l.contains("as CI runs it"))
            .map(|l| l.split(" # ").next().unwrap_or_default().trim())
            .collect();
        assert!(
            claimed.contains(&"cargo clippy --locked --all-targets -- -D warnings"),
            "the README shows no lint as CI runs it: {claimed:?}"
        );
        assert!(
            claimed.contains(&"cargo fmt --all -- --check"),
            "the README shows no format check as CI runs it: {claimed:?}"
        );
        for command in claimed {
            assert!(
                ran.contains(&command),
                "the README says `{command}` is run as CI runs it; ci.yml does not run it"
            );
        }
    }

    /// Every release archive the README names is one the release workflow builds
    /// (T2-LINUX-INSTALL), so the install lines cannot point at a file that no
    /// release has.
    #[test]
    fn the_archives_the_readme_names_are_built_by_the_release() {
        let root = crate::root();
        let readme = std::fs::read_to_string(root.join("README.md")).unwrap();
        let workflow = std::fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
        let targets: Vec<&str> = workflow
            .lines()
            .filter_map(|l| l.trim().strip_prefix("- target:"))
            .map(str::trim)
            .collect();
        assert!(targets.len() >= 4, "{targets:?}");
        let named: Vec<&str> = readme
            .split(|c: char| c.is_whitespace() || c == '"' || c == '/' || c == ';' || c == ')')
            .filter_map(|w| w.strip_prefix("forge-"))
            .map(|w| w.trim_end_matches(".tar.gz").trim_end_matches(".zip"))
            .filter(|w| w.contains("-unknown-") || w.contains("-apple-") || w.contains("-pc-"))
            .collect();
        assert!(
            named.contains(&"x86_64-unknown-linux-musl"),
            "the README names no Linux archive: {named:?}"
        );
        for target in named {
            assert!(
                targets.contains(&target),
                "the README names forge-{target}, which release.yml does not build ({targets:?})"
            );
        }
    }

    /// The README's section with its name, up to the next `## ` heading.
    fn section(readme: &str, heading: &str) -> String {
        readme
            .split(&format!("\n## {heading}\n"))
            .nth(1)
            .and_then(|rest| rest.split("\n## ").next())
            .unwrap_or_default()
            .to_string()
    }

    /// Every link and anchor of the front page resolves (A10), as the
    /// guide's do: it is the page every reader comes in by.
    #[test]
    fn every_link_in_the_readme_resolves() {
        let readme = crate::read("README.md");
        let links = crate::guide::check_links(&crate::root().join("README.md"), &readme);
        assert!(links.as_ref().is_ok_and(|n| *n > 40), "{links:?}");
    }

    /// The first section is a door for each of ADR-078's five readers, to the
    /// page the documentation's index starts that reader on (A10).
    #[test]
    fn the_readme_has_a_door_for_each_reader() {
        let readme = crate::read("README.md");
        let doors = section(&readme, "Where to start");
        assert_eq!(
            doors.lines().filter(|l| l.starts_with("| **")).count(),
            5,
            "{doors}"
        );
        for start in [
            "docs/guide/README.md",
            "docs/guide/ai.md",
            "docs/guide/pipeline.md",
            "docs/EMBEDDING.md",
            "CONTRIBUTING.md",
        ] {
            assert!(doors.contains(&format!("]({start})")), "no door to {start}");
        }
        assert!(doors.contains("](docs/README.md)"));
        assert_eq!(section(&readme, "Nowhere"), "");
    }

    #[test]
    fn a_date_is_four_two_and_two_digits() {
        assert!(has_date("2,137 pass (2026-11-17); CI runs them"));
        assert!(!has_date(
            "2,182 cases, 2026 to 11-17, 2026-1-17, 2026/11/17"
        ));
        assert!(!has_date("short"));
    }

    /// The table of checks says what runs each one on a pull request, not a
    /// dated result that the next run outdates (A10): the results, with their
    /// dates, are VALIDATION.md's.
    #[test]
    fn the_checks_table_names_what_runs_each_check_and_no_date() {
        let readme = crate::read("README.md");
        let checks = section(&readme, "How we know the numbers are right");
        let rows: Vec<&str> = checks.lines().filter(|l| l.starts_with("| ")).collect();
        assert_eq!(
            rows.first().copied(),
            Some("| Check | What it covers | Checked on every pull request by |")
        );
        assert!(rows.len() >= 6, "{rows:?}");
        let dated: Vec<&&str> = rows.iter().filter(|row| has_date(row)).collect();
        assert!(dated.is_empty(), "a row gives a dated result: {dated:?}");
        assert!(checks.contains("](VALIDATION.md)"));
    }

    /// The MCP Registry verifies a crate by this line in the README as
    /// crates.io shows it, and the name is `server.json`'s (M-MCP-NAME).
    #[test]
    fn the_readme_carries_the_registry_name_of_server_json() {
        let server: serde_json::Value =
            serde_json::from_str(&crate::read("server.json")).unwrap_or_default();
        let name = server["name"].as_str().unwrap_or_default();
        assert!(name.starts_with("io.github.mollendorff-ai/"), "{name}");
        let readme = crate::read("README.md");
        let line = format!("mcp-name: {name}");
        assert_eq!(
            readme.lines().filter(|l| l.ends_with(&line)).count(),
            1,
            "the README has no visible `{line}` line"
        );
    }
}
