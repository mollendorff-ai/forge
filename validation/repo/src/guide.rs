//! The guide's pages say what Forge does (roadmap A2-GUIDE-TESTS, ADR-078).
//!
//! A page in `docs/guide/` is a lesson, and a lesson is true only if what it
//! shows happens. Every page is checked, from its first commit, in four ways:
//!
//! - **Commands.** Each `text` block that starts with `$ forge` is run, in
//!   order, and every line the page shows as its output is a line the command
//!   prints, as `readme_commands` checks the README. The commands run in a
//!   copy of `examples/` in a temporary folder, so a lesson can run
//!   `forge calculate` and `forge edit` on the models it teaches without
//!   changing the repository, and a later command sees what an earlier one
//!   wrote.
//! - **Files.** A `yaml` block whose first line is `# examples/….yaml` is that
//!   file, the rest of the block line for line, so the model a lesson shows is
//!   one the tests run and Studio opens.
//! - **Links.** Every relative link and anchor resolves.
//! - **Answers.** Every number in a code span inside `<details>` (the answer
//!   to a lesson's challenge) is a number one of the page's commands printed.
//!
//! And `docs/cli/README.md`, which `make docs-cli` writes from `--help`, is
//! equal to what the binary prints: the page was equal when this check was
//! written, and nothing kept it so.

use crate::architecture_docs::{broken_link, local_links};
use crate::readme_commands::{rejoin_quotes, squeeze, text_blocks};
use crate::Ran;
use std::path::{Path, PathBuf};

/// What a page's checks looked at.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Checked {
    /// Commands run
    pub commands: usize,
    /// Lines of output compared
    pub lines: usize,
    /// YAML blocks compared with their files
    pub files: usize,
    /// Links followed
    pub links: usize,
    /// Numbers of answers found among the printed ones
    pub answers: usize,
}

/// Whether `name` ends in `.ext`, as a file extension.
fn has_extension(name: &str, ext: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|found| found.eq_ignore_ascii_case(ext))
}

/// The Markdown files in `dir`, each named `prefix/name.md`, in order. None if
/// `dir` is not there.
#[must_use]
pub fn pages_in(dir: &Path, prefix: &str) -> Vec<String> {
    let mut pages: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| has_extension(name, "md"))
        .map(|name| format!("{prefix}/{name}"))
        .collect();
    pages.sort();
    pages
}

/// The pages of the guide, as paths from the top (`docs/guide/….md`), in
/// order. None while the guide has no pages.
#[must_use]
pub fn guide_pages() -> Vec<String> {
    pages_in(&crate::root().join("docs/guide"), "docs/guide")
}

/// A copy of `examples/` in a new temporary folder, where a lesson's commands
/// run as they would in the repository's top folder.
///
/// # Panics
///
/// Panics if the folder cannot be made or `examples/` cannot be copied.
#[must_use]
pub fn sandbox() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary folder");
    copy_folder(
        &crate::root().join("examples"),
        &dir.path().join("examples"),
    );
    dir
}

/// `from` and everything in it, copied to `to`.
fn copy_folder(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a folder to copy into");
    for entry in std::fs::read_dir(from).expect("a folder to copy") {
        let path = entry.expect("a folder entry").path();
        let target = to.join(path.file_name().expect("a name"));
        if path.is_dir() {
            copy_folder(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("a file to copy");
        }
    }
}

/// The numbers written in `text`, without thousands separators: `386,827.45`
/// gives `386827.45`, `-56,463.56` gives `-56463.56`, and `P(x > 0) = 0.8`
/// gives `0` and `0.8`. A date gives itself, as a token.
#[must_use]
pub fn numbers_in(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut run = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() || matches!(c, '.' | ',' | '-') {
            run.push(c);
            continue;
        }
        let digits = run.trim_matches(|c: char| matches!(c, '.' | ',' | '-'));
        if digits.contains(|c: char| c.is_ascii_digit()) {
            let sign = if run.starts_with('-') { "-" } else { "" };
            found.push(format!("{sign}{}", digits.replace(',', "")));
        }
        run.clear();
    }
    found
}

/// Run each `$ forge` block of `doc` with `run` and compare the lines the page
/// shows with the lines printed. Returns the counts, and everything the
/// commands printed, for [`check_answers`].
///
/// A block may show some of the lines a command prints, not all, and a block
/// without a command shows more lines of the command above it.
///
/// # Errors
///
/// Names the page and the command, or the line, that is wrong.
pub fn check_commands(
    page: &str,
    doc: &str,
    run: &mut dyn FnMut(&[String]) -> Ran,
) -> Result<(Checked, Vec<String>), String> {
    let mut checked = Checked::default();
    let mut printed: Vec<String> = Vec::new();
    let mut everything: Vec<String> = Vec::new();
    let mut label = String::new();
    for block in text_blocks(doc) {
        let shown = if let Some(args) = block.first().and_then(|l| l.strip_prefix("$ forge ")) {
            let args = rejoin_quotes(&args.split_whitespace().collect::<Vec<_>>());
            label = format!("forge {}", args.join(" "));
            let out = run(&args);
            if !out.success() {
                return Err(format!("{page}: `{label}` fails:\n{}{}", out.out, out.err));
            }
            printed = out
                .out
                .lines()
                .chain(out.err.lines())
                .map(squeeze)
                .collect();
            everything.extend(printed.iter().cloned());
            checked.commands += 1;
            &block[1..]
        } else if checked.commands > 0 {
            &block[..]
        } else {
            continue;
        };
        for line in shown {
            let wanted = squeeze(line);
            if wanted.is_empty() {
                continue;
            }
            if !printed.contains(&wanted) {
                return Err(format!(
                    "{page} shows a line that `{label}` does not print:\n  {wanted}\nIt prints:\n{}",
                    printed.join("\n")
                ));
            }
            checked.lines += 1;
        }
    }
    Ok((checked, everything))
}

/// The `yaml` blocks of `doc` whose first line is `# examples/….yaml`: the
/// path, and the rest of the block.
#[must_use]
pub fn file_blocks(doc: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut lines = doc.lines();
    while let Some(line) = lines.next() {
        if line.trim() != "```yaml" {
            continue;
        }
        let mut block: Vec<&str> = Vec::new();
        for inner in lines.by_ref() {
            if inner.starts_with("```") {
                break;
            }
            block.push(inner);
        }
        let named = block
            .first()
            .and_then(|first| first.strip_prefix("# "))
            .map(str::trim)
            .filter(|path| path.starts_with("examples/") && has_extension(path, "yaml"));
        if let Some(path) = named {
            blocks.push((path.to_string(), block[1..].join("\n")));
        }
    }
    blocks
}

/// Compare each file block of `doc` with the file it names, as `read` gives it
/// (`None` for no such file), ignoring blank lines at the end.
///
/// # Errors
///
/// Names the page and the file, and the first line that differs.
pub fn check_file_blocks(
    page: &str,
    doc: &str,
    read: &dyn Fn(&str) -> Option<String>,
) -> Result<usize, String> {
    let blocks = file_blocks(doc);
    for (path, shown) in &blocks {
        let Some(file) = read(path) else {
            return Err(format!("{page} shows {path}, which is not a file"));
        };
        let (shown, file) = (shown.trim_end(), file.trim_end());
        if shown != file {
            let at = shown
                .lines()
                .zip(file.lines())
                .position(|(a, b)| a != b)
                .unwrap_or_else(|| shown.lines().count().min(file.lines().count()));
            return Err(format!(
                "{page} shows {path} differently from the file, from line {}:\n  page: {}\n  file: {}",
                at + 1,
                shown.lines().nth(at).unwrap_or("(ends)"),
                file.lines().nth(at).unwrap_or("(ends)")
            ));
        }
    }
    Ok(blocks.len())
}

/// Every relative link of `doc`, a page at `page`, resolves, with its anchor.
///
/// # Errors
///
/// Names the page and the first link that does not.
pub fn check_links(page: &Path, doc: &str) -> Result<usize, String> {
    let links = local_links(doc);
    for link in &links {
        if let Some(why) = broken_link(page, link) {
            return Err(format!("{}: {why}", page.display()));
        }
    }
    Ok(links.len())
}

/// The numbers in the code spans inside `<details>` of `doc`: a lesson's
/// answers to its challenge.
#[must_use]
pub fn answers(doc: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut inside = false;
    for line in doc.lines() {
        if line.trim_start().starts_with("<details") {
            inside = true;
        }
        if inside {
            for span in line.split('`').skip(1).step_by(2) {
                found.extend(numbers_in(span));
            }
        }
        if line.contains("</details>") {
            inside = false;
        }
    }
    found
}

/// Every number of an answer in `doc` is one of the numbers in `printed`, what
/// the page's commands printed.
///
/// # Errors
///
/// Names the page and the first answer no command printed.
pub fn check_answers(page: &str, doc: &str, printed: &[String]) -> Result<usize, String> {
    let known: Vec<String> = printed.iter().flat_map(|line| numbers_in(line)).collect();
    let wanted = answers(doc);
    for answer in &wanted {
        if !known.contains(answer) {
            return Err(format!(
                "{page}: the answer {answer} is not a number any command on the page prints"
            ));
        }
    }
    Ok(wanted.len())
}

/// All four checks of one page, `page` being its path from the top.
///
/// # Errors
///
/// The first thing wrong, naming the page.
pub fn check_page(
    page: &str,
    doc: &str,
    run: &mut dyn FnMut(&[String]) -> Ran,
    read: &dyn Fn(&str) -> Option<String>,
) -> Result<Checked, String> {
    let (mut checked, printed) = check_commands(page, doc, run)?;
    checked.files = check_file_blocks(page, doc, read)?;
    checked.links = check_links(&crate::root().join(page), doc)?;
    checked.answers = check_answers(page, doc, &printed)?;
    Ok(checked)
}

/// The sections of `docs/cli/README.md` (`## calculate`, then a code block),
/// each with the arguments that print it and the text it shows.
#[must_use]
pub fn cli_sections(doc: &str) -> Vec<(Vec<String>, String)> {
    let mut sections = Vec::new();
    let mut lines = doc.lines();
    while let Some(line) = lines.next() {
        let Some(name) = line.strip_prefix("## ") else {
            continue;
        };
        if !lines.by_ref().any(|l| l.starts_with("```")) {
            break;
        }
        let shown: Vec<&str> = lines
            .by_ref()
            .take_while(|l| !l.starts_with("```"))
            .collect();
        let args = if name == "Main Help" {
            vec!["--help".to_string()]
        } else {
            vec![name.to_string(), "--help".to_string()]
        };
        sections.push((args, shown.join("\n")));
    }
    sections
}

/// The sections of the CLI reference `doc` that differ from what `run` prints.
#[must_use]
pub fn stale_cli_sections(doc: &str, run: &mut dyn FnMut(&[String]) -> Ran) -> Vec<String> {
    cli_sections(doc)
        .into_iter()
        .filter(|(args, shown)| run(args).out.trim_end() != shown.trim_end())
        .map(|(args, _)| args.join(" "))
        .collect()
}

/// A command run in `dir`, as [`crate::forge`] runs one in the top folder.
fn in_folder(dir: &Path) -> impl FnMut(&[String]) -> Ran + '_ {
    move |args| crate::forge_in(dir, args)
}

/// The text of a file of the repository, by its path from the top.
fn repository_file(path: &str) -> Option<String> {
    std::fs::read_to_string(crate::root().join(path)).ok()
}

/// What is wrong with `pages` (paths from the top), whose text `doc_of` gives:
/// one message for each page that fails a check, each page run in a copy of
/// `examples/` of its own.
#[must_use]
pub fn wrong_pages_of(pages: &[String], doc_of: &dyn Fn(&str) -> String) -> Vec<String> {
    pages
        .iter()
        .filter_map(|page| {
            let (_dir, path) = page_folder();
            let mut run = in_folder(&path);
            check_page(page, &doc_of(page), &mut run, &repository_file).err()
        })
        .collect()
}

/// What is wrong with the pages of the guide.
#[must_use]
pub fn wrong_pages() -> Vec<String> {
    wrong_pages_of(&guide_pages(), &crate::read)
}

/// The folder a page's commands run in: its own copy of `examples/`.
#[must_use]
pub fn page_folder() -> (tempfile::TempDir, PathBuf) {
    let dir = sandbox();
    let path = dir.path().to_path_buf();
    (dir, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What a command prints, for a test that needs no binary.
    fn ran(code: i32, out: &str) -> Ran {
        Ran {
            code: Some(code),
            out: out.to_string(),
            err: String::new(),
        }
    }

    /// A page with one command that prints `out`.
    fn page_with(shown: &str) -> String {
        format!("A lesson.\n\n```text\n$ forge calculate a.yaml\n{shown}\n```\n")
    }

    #[test]
    fn numbers_are_read_without_separators_and_with_their_sign() {
        assert_eq!(
            numbers_in("year.revenue = 386,827.45 (at -56,463.56), P(x > 0) = 0.8."),
            ["386827.45", "-56463.56", "0", "0.8"]
        );
        assert_eq!(numbers_in("on 2026-11-23, 5%"), ["2026-11-23", "5"]);
        assert_eq!(
            numbers_in("no number here, nor - or ..."),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_command_is_run_and_the_lines_a_page_shows_must_be_printed() {
        let doc = page_with("  year.profit  =  786.32\n\n```\n\n```text\nmore: 1");
        let mut seen: Vec<Vec<String>> = Vec::new();
        let (checked, printed) = check_commands("p.md", &doc, &mut |args| {
            seen.push(args.to_vec());
            ran(0, "year.profit = 786.32\nmore: 1\nunshown: 2\n")
        })
        .unwrap();
        assert_eq!(seen, [["calculate", "a.yaml"]]);
        assert_eq!(
            checked,
            Checked {
                commands: 1,
                lines: 2,
                ..Checked::default()
            }
        );
        assert_eq!(printed, ["year.profit = 786.32", "more: 1", "unshown: 2"]);
    }

    #[test]
    fn a_quoted_argument_is_one_argument() {
        let doc = "```text\n$ forge simulate m.yaml --seek \"P(x > 0) = 0.8\"\n```\n";
        let mut seen = Vec::new();
        check_commands("p.md", doc, &mut |args| {
            seen = args.to_vec();
            ran(0, "")
        })
        .unwrap();
        assert_eq!(seen, ["simulate", "m.yaml", "--seek", "P(x > 0) = 0.8"]);
    }

    #[test]
    fn a_line_the_command_does_not_print_is_named() {
        let err = check_commands("p.md", &page_with("year.profit = 1"), &mut |_| {
            ran(0, "year.profit = 2\n")
        })
        .unwrap_err();
        assert!(err.contains("p.md shows a line that `forge calculate a.yaml` does not print"));
        assert!(err.contains("year.profit = 1") && err.contains("It prints:\nyear.profit = 2"));
    }

    #[test]
    fn a_command_that_fails_is_named_with_what_it_wrote() {
        let mut out = ran(1, "partial\n");
        out.err = "Error: no such file\n".to_string();
        let err = check_commands("p.md", &page_with("x"), &mut |_| Ran {
            code: out.code,
            out: out.out.clone(),
            err: out.err.clone(),
        })
        .unwrap_err();
        assert!(err.contains("`forge calculate a.yaml` fails:\npartial\nError: no such file"));
    }

    #[test]
    fn what_a_command_writes_to_standard_error_counts_as_printed() {
        let mut out = ran(0, "");
        out.err = "warning: stale\n".to_string();
        let (checked, _) = check_commands("p.md", &page_with("warning: stale"), &mut |_| Ran {
            code: out.code,
            out: out.out.clone(),
            err: out.err.clone(),
        })
        .unwrap();
        assert_eq!(checked.lines, 1);
    }

    #[test]
    fn a_block_before_any_command_is_not_output() {
        let doc = "```text\nsomething else\n```\n\n```text\n$ forge validate a.yaml\nok\n```\n";
        let (checked, _) = check_commands("p.md", doc, &mut |_| ran(0, "ok\n")).unwrap();
        assert_eq!((checked.commands, checked.lines), (1, 1));
    }

    #[test]
    fn a_block_that_is_not_text_is_not_run() {
        let doc = "```sh\n$ forge validate a.yaml\n```\n\n```text\n$ forge calculate a.yaml\n```\n";
        let mut run_count = 0;
        let (checked, _) = check_commands("p.md", doc, &mut |args| {
            run_count += 1;
            assert_eq!(args[0], "calculate");
            ran(0, "")
        })
        .unwrap();
        assert_eq!((checked.commands, run_count), (1, 1));
    }

    #[test]
    fn a_yaml_block_headed_by_a_file_is_found() {
        let doc = "```yaml\n# examples/guide/a.yaml\nx: 1\ny: 2\n```\n\n\
                   ```yaml\n# a comment, not a file\nz: 3\n```\n\n\
                   ```yaml\n# examples/not-yaml.txt\nz: 3\n```\n\n\
                   ```yaml\n```\n\n```text\n# examples/b.yaml\n```\n";
        assert_eq!(
            file_blocks(doc),
            [(
                "examples/guide/a.yaml".to_string(),
                "x: 1\ny: 2".to_string()
            )]
        );
    }

    #[test]
    fn a_yaml_block_must_be_its_file() {
        let doc = "```yaml\n# examples/guide/a.yaml\nx: 1\ny: 2\n```\n";
        let same = |_: &str| Some("x: 1\ny: 2\n\n".to_string());
        assert_eq!(check_file_blocks("p.md", doc, &same), Ok(1));
        let changed = |_: &str| Some("x: 1\ny: 3\n".to_string());
        let err = check_file_blocks("p.md", doc, &changed).unwrap_err();
        assert!(
            err.contains("from line 2:\n  page: y: 2\n  file: y: 3"),
            "{err}"
        );
        let longer = |_: &str| Some("x: 1\ny: 2\nz: 3\n".to_string());
        let err = check_file_blocks("p.md", doc, &longer).unwrap_err();
        assert!(
            err.contains("from line 3:\n  page: (ends)\n  file: z: 3"),
            "{err}"
        );
        let err = check_file_blocks("p.md", doc, &|_: &str| None).unwrap_err();
        assert_eq!(err, "p.md shows examples/guide/a.yaml, which is not a file");
        assert_eq!(check_file_blocks("p.md", "no blocks", &same), Ok(0));
    }

    #[test]
    fn links_and_anchors_must_resolve() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("other.md");
        std::fs::write(&other, "# A heading\n").unwrap();
        let page = dir.path().join("page.md");
        std::fs::write(&page, "# Own\n").unwrap();
        let good = "[a](other.md) [b](other.md#a-heading) [c](#own) [d](https://x.org)\n# Own\n";
        assert_eq!(check_links(&page, good), Ok(3));
        let err = check_links(&page, "[a](other.md#nowhere)").unwrap_err();
        assert!(
            err.ends_with("other.md#nowhere: no heading with that anchor"),
            "{err}"
        );
        let err = check_links(&page, "[a](gone.md)").unwrap_err();
        assert!(err.ends_with("gone.md: no such file"), "{err}");
    }

    #[test]
    fn answers_are_the_numbers_in_the_code_of_a_details_block() {
        let doc = "Before `1`.\n\n<details>\n<summary>Answer</summary>\n\n\
                   The price is `18.32`, with a `34.76%` chance, in `year.profit`.\n\
                   </details>\n\nAfter `2`.\n\n<details><summary>x</summary>`3` </details>\n`4`\n";
        assert_eq!(answers(doc), ["18.32", "34.76", "3"]);
    }

    #[test]
    fn an_answer_must_be_a_number_a_command_printed() {
        let doc = "<details>\n`18.32` and `1,000`\n</details>\n";
        let printed = vec![
            "price = 18.32".to_string(),
            "revenue 1,000.00 and 1,000".to_string(),
        ];
        assert_eq!(check_answers("p.md", doc, &printed), Ok(2));
        let err = check_answers("p.md", doc, &printed[..1]).unwrap_err();
        assert_eq!(
            err,
            "p.md: the answer 1000 is not a number any command on the page prints"
        );
        assert_eq!(check_answers("p.md", "no answers", &[]), Ok(0));
    }

    #[test]
    fn a_page_is_checked_four_ways() {
        let doc = format!(
            "{}\n```yaml\n# examples/guide/a.yaml\nx: 1\n```\n\n[glossary](GLOSSARY.md)\n\n\
             <details>\n`786.32`\n</details>\n",
            page_with("year.profit = 786.32")
        );
        let read = |_: &str| Some("x: 1\n".to_string());
        let mut run = |_: &[String]| ran(0, "year.profit = 786.32\n");
        // Links resolve from the page's place in the repository
        let checked = check_page("docs/p.md", &doc, &mut run, &read).unwrap();
        assert_eq!(
            checked,
            Checked {
                commands: 1,
                lines: 1,
                files: 1,
                links: 1,
                answers: 1
            }
        );
        let bad_file = |_: &str| Some("x: 2\n".to_string());
        assert!(check_page("docs/p.md", &doc, &mut run, &bad_file).is_err());
        let doc_bad_link = format!("{doc}\n[x](gone.md)\n");
        assert!(check_page("docs/p.md", &doc_bad_link, &mut run, &read).is_err());
        let doc_bad_answer = doc.replace("`786.32`", "`1`");
        assert!(check_page("docs/p.md", &doc_bad_answer, &mut run, &read).is_err());
        let mut fails = |_: &[String]| ran(1, "");
        assert!(check_page("docs/p.md", &doc, &mut fails, &read).is_err());
    }

    #[test]
    fn the_cli_reference_is_read_by_section() {
        let doc = "# Forge CLI Reference\n\n> Auto-generated\n\n## Main Help\n\n```\nTop\n```\n\n\
                   ## calculate\n\n```\nUsage: forge calculate\n\n  --dry-run\n```\n\n## trailing\n";
        assert_eq!(
            cli_sections(doc),
            [
                (vec!["--help".to_string()], "Top".to_string()),
                (
                    vec!["calculate".to_string(), "--help".to_string()],
                    "Usage: forge calculate\n\n  --dry-run".to_string()
                ),
            ]
        );
    }

    #[test]
    fn a_cli_section_that_differs_from_the_help_is_named() {
        let doc = "## Main Help\n\n```\nTop\n```\n\n## calculate\n\n```\nold text\n```\n";
        let mut run = |args: &[String]| match args[0].as_str() {
            "--help" => ran(0, "Top\n"),
            _ => ran(0, "new text\n"),
        };
        assert_eq!(stale_cli_sections(doc, &mut run), ["calculate --help"]);
        let mut same = |args: &[String]| match args[0].as_str() {
            "--help" => ran(0, "Top\n\n"),
            _ => ran(0, "old text"),
        };
        assert_eq!(stale_cli_sections(doc, &mut same), Vec::<String>::new());
    }

    #[test]
    fn the_cli_reference_is_what_the_binary_prints() {
        let doc = crate::read("docs/cli/README.md");
        assert!(cli_sections(&doc).len() >= 28);
        let mut run = |args: &[String]| crate::forge(args);
        let stale = stale_cli_sections(&doc, &mut run);
        assert!(
            stale.is_empty(),
            "docs/cli/README.md differs from `forge {stale:?}`: run `make docs-cli` and commit it"
        );
    }

    #[test]
    fn the_sandbox_is_a_copy_a_command_can_change() {
        let (dir, path) = page_folder();
        assert_eq!(dir.path(), path);
        let starter = path.join("examples/starter.yaml");
        assert!(path.join("examples/bagel/bagel.yaml").is_file());
        assert!(!std::fs::read_to_string(&starter)
            .unwrap()
            .contains("value: 4800"));
        let mut run = in_folder(&path);
        let args = ["calculate", "examples/starter.yaml"].map(String::from);
        assert!(run(&args).success());
        assert!(std::fs::read_to_string(&starter)
            .unwrap()
            .contains("value: 4800"));
        // The repository's copy is as it was
        assert!(!crate::read("examples/starter.yaml").contains("value: 4800"));
    }

    #[test]
    fn the_markdown_files_of_a_folder_are_its_pages_in_order() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["b.md", "a.md", "notes.txt", "c.MD"] {
            std::fs::write(dir.path().join(name), "").unwrap();
        }
        assert_eq!(
            pages_in(dir.path(), "docs/guide"),
            ["docs/guide/a.md", "docs/guide/b.md", "docs/guide/c.MD"]
        );
        assert_eq!(
            pages_in(&dir.path().join("none"), "x"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn the_guides_pages_are_in_the_guide_folder_and_each_is_true() {
        let folder = crate::root().join("docs/guide");
        assert_eq!(guide_pages(), pages_in(&folder, "docs/guide"));
        assert_eq!(wrong_pages(), Vec::<String>::new());
    }

    #[test]
    fn each_page_runs_in_a_copy_of_the_examples_and_the_wrong_ones_are_named() {
        let good = "```text\n$ forge calculate examples/starter.yaml\n```\n\n\
                    ```text\n$ forge calculate examples/starter.yaml --dry-run\n```\n\n\
                    [glossary](GLOSSARY.md)\n";
        let bad = "```text\n$ forge validate examples/starter.yaml\nnot printed\n```\n";
        let pages = ["docs/good.md", "docs/bad.md"].map(String::from);
        let doc_of = |page: &str| if page == "docs/good.md" { good } else { bad }.to_string();
        let wrong = wrong_pages_of(&pages, &doc_of);
        assert_eq!(wrong.len(), 1, "{wrong:?}");
        assert!(
            wrong[0].starts_with("docs/bad.md shows a line that"),
            "{wrong:?}"
        );
        // Neither page changed the repository's examples
        assert!(!crate::read("examples/starter.yaml").contains("value: 4800"));
    }

    #[test]
    fn a_file_of_the_repository_is_read_by_its_path() {
        assert!(repository_file("examples/starter.yaml").is_some_and(|s| s.contains("plan:")));
        assert_eq!(repository_file("examples/no-such-file.yaml"), None);
    }
}
