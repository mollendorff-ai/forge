//! The pages for a contributor, a maintainer and a developer who embeds Forge.
//!
//! ADR-078, roadmap A9: `CONTRIBUTING.md`, `docs/MAINTAINING.md` and
//! `docs/EMBEDDING.md`. Each states versions, jobs, targets and code that
//! live in other files, so each claim is held by the file that has it, and the
//! REST example of the last runs against a real `forge serve`.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::Value;

use crate::architecture_docs::prose;
use crate::guide::fenced_blocks;
use crate::{forge_bin, root};

/// The ids of the jobs of a workflow file, in order.
#[must_use]
pub fn job_ids(workflow: &str) -> Vec<String> {
    workflow
        .lines()
        .skip_while(|line| *line != "jobs:")
        .skip(1)
        .filter_map(|line| line.strip_prefix("  ")?.strip_suffix(':'))
        .filter(|id| {
            id.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
        })
        .map(str::to_string)
        .collect()
}

/// The targets of a Makefile (`check:`, `test-repo: build`), not its
/// variables.
#[must_use]
pub fn make_targets(makefile: &str) -> Vec<String> {
    makefile
        .lines()
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, rest)| {
            !rest.starts_with('=')
                && !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
        .map(|(name, _)| name.to_string())
        .collect()
}

/// What `doc` shows as code: its fenced blocks and its code spans.
#[must_use]
pub fn code_of(doc: &str) -> String {
    let mut code: String = prose(doc)
        .split('`')
        .skip(1)
        .step_by(2)
        .collect::<Vec<_>>()
        .join("\n");
    for (_, lines) in fenced_blocks(doc) {
        code.push('\n');
        code.push_str(&lines.join("\n"));
    }
    code
}

/// The words after `make ` in `code`: the targets a page tells a reader to run.
#[must_use]
pub fn make_commands(code: &str) -> Vec<String> {
    let mut words = Vec::new();
    for (at, _) in code.match_indices("make ") {
        let starts_a_word = code[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric());
        let word: String = code[at + 5..]
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == '-')
            .collect();
        if starts_a_word && !word.is_empty() && !words.contains(&word) {
            words.push(word);
        }
    }
    words
}

/// Each claim `(file, what the file says, what the page says)` that is not so.
///
/// The file lacks its words, or the page lacks its own: a page that says
/// `Node 22` and a workflow that says another number disagree.
#[must_use]
pub fn unheld(
    doc: &str,
    claims: &[(&str, &str, &str)],
    file: &dyn Fn(&str) -> String,
) -> Vec<String> {
    let mut wrong = Vec::new();
    for (path, in_file, in_page) in claims {
        if !file(path).contains(in_file) {
            wrong.push(format!("{path} does not have `{in_file}`"));
        }
        if !doc.contains(in_page) {
            wrong.push(format!("the page does not say `{in_page}`"));
        }
    }
    wrong
}

/// The text after `key = "` up to the next quote, on the first line that
/// starts with `key`.
#[must_use]
pub fn quoted_after(text: &str, key: &str) -> Option<String> {
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("{key} = \"")))?;
    line.split('"').nth(1).map(ToString::to_string)
}

/// The first fenced block of the crate's own documentation (the `//!` lines
/// of `lib`), which `cargo test --doc` runs.
#[must_use]
pub fn crate_doc_example(lib: &str) -> Vec<String> {
    let doc: Vec<&str> = lib
        .lines()
        .filter_map(|l| l.strip_prefix("//!"))
        .map(|l| l.strip_prefix(' ').unwrap_or(l))
        .collect();
    fenced_blocks(&doc.join("\n")).remove(0).1
}

/// Whether `have` holds everything `want` says: the same scalars, the keys of
/// each object of `want` in the one of `have`, and arrays of the same length.
#[must_use]
pub fn holds(have: &Value, want: &Value) -> bool {
    match (have, want) {
        (Value::Object(h), Value::Object(w)) => {
            w.iter().all(|(k, v)| h.get(k).is_some_and(|x| holds(x, v)))
        },
        (Value::Array(h), Value::Array(w)) => {
            h.len() == w.len() && h.iter().zip(w).all(|(x, v)| holds(x, v))
        },
        _ => have == want,
    }
}

/// What a `curl -X POST URL ... -d '{...}'` command sends: the path of the
/// URL and the body.
#[must_use]
pub fn curl_request(command: &str) -> Option<(String, String)> {
    let url = command
        .split_whitespace()
        .find_map(|w| w.strip_prefix("http://"))?;
    let path = url.split_once('/')?.1;
    let body = command.split_once("-d '")?.1.rsplit_once('\'')?.0;
    Some((format!("/{path}"), body.to_string()))
}

/// Post `body` to `path` of the server on `port`, and give the answer's body.
///
/// # Panics
///
/// Panics if the server cannot be reached.
#[must_use]
pub fn post(port: u16, path: &str, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the server answers");
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .expect("the request is sent");
    let mut answer = String::new();
    stream
        .read_to_string(&mut answer)
        .expect("the answer is read");
    answer
        .split_once("\r\n\r\n")
        .map_or_else(|| answer.clone(), |(_, b)| b.to_string())
}

/// Wait until something listens on `port`.
///
/// # Panics
///
/// Panics if nothing does within ten seconds.
pub fn wait_for(port: u16) {
    let mut tries = 0;
    while TcpStream::connect(("127.0.0.1", port)).is_err() {
        tries += 1;
        assert!(tries < 100, "nothing listens on port {port}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// A `forge serve` that ends with the test.
pub struct Server(Child);

impl Server {
    /// Start `forge serve` on a free port; give the server and the port.
    ///
    /// # Panics
    ///
    /// Panics if no port is free or the binary does not start.
    #[must_use]
    pub fn start() -> (Self, u16) {
        let port = std::net::TcpListener::bind(("127.0.0.1", 0))
            .expect("a free port")
            .local_addr()
            .expect("its address")
            .port();
        let child = Command::new(forge_bin())
            .args(["serve", "--port", &port.to_string()])
            .current_dir(root())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("forge serve starts");
        (Self(child), port)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture_docs::named_paths;
    use crate::guide::check_links;
    use crate::read;
    use serde_json::json;
    use std::net::TcpListener;

    /// What a client sent, read until it ends with `end`.
    fn request_ending(stream: &mut TcpStream, end: &str) -> String {
        let mut sent = String::new();
        let mut chunk = [0_u8; 512];
        while !sent.ends_with(end) {
            let n = stream.read(&mut chunk).unwrap();
            sent.push_str(&String::from_utf8_lossy(&chunk[..n]));
        }
        sent
    }

    fn page(name: &str) -> (std::path::PathBuf, String) {
        (root().join(name), read(name))
    }

    #[test]
    fn the_jobs_of_a_workflow_are_found_by_their_ids() {
        let workflow = "name: X\njobs:\n  # a comment:\n  test:\n    name: T\n    steps:\n  build-all:\n    uses: y\n";
        assert_eq!(job_ids(workflow), ["test", "build-all"]);
        assert_eq!(job_ids("on: push\n"), Vec::<String>::new());
    }

    #[test]
    fn the_targets_of_a_makefile_are_not_its_variables() {
        let makefile =
            "CARGO := cargo\nx:= y\n.PHONY: a\ncheck: lint test\n\tcargo fmt\ntest-repo:\n";
        assert_eq!(make_targets(makefile), ["check", "test-repo"]);
    }

    #[test]
    fn the_make_commands_a_page_shows_are_the_words_after_make() {
        let doc = "Make it. `make check` and `RUSTUP_TOOLCHAIN=1 make coverage`, not `cmake build`.\n\n```bash\nmake test   # all\nmake check\n```\n";
        assert_eq!(make_commands(&code_of(doc)), ["check", "coverage", "test"]);
        assert_eq!(make_commands("make "), Vec::<String>::new());
    }

    #[test]
    fn a_claim_is_held_when_the_file_and_the_page_both_say_it() {
        let file = |path: &str| {
            if path == "a.yml" {
                "node-version: 22".to_string()
            } else {
                String::new()
            }
        };
        let claim = [("a.yml", "node-version: 22", "Node 22")];
        assert_eq!(unheld("Node 22", &claim, &file), Vec::<String>::new());
        assert_eq!(
            unheld("Node 20", &claim, &file),
            ["the page does not say `Node 22`"]
        );
        assert_eq!(
            unheld(
                "Node 22",
                &[("b.yml", "node-version: 22", "Node 22")],
                &file
            ),
            ["b.yml does not have `node-version: 22`"]
        );
    }

    #[test]
    fn a_value_is_read_from_the_line_that_starts_with_its_key() {
        let toml = "[package]\nname = \"x\"\nrust-version = \"1.97\"\n";
        assert_eq!(quoted_after(toml, "rust-version").as_deref(), Some("1.97"));
        assert_eq!(quoted_after(toml, "version"), None);
    }

    #[test]
    fn the_crates_example_is_its_first_block() {
        let lib = "//! A crate.\n//!\n//! ```\n//! let x = 1;\n//!\n//! assert_eq!(x, 1);\n//! ```\nfn f() {}\n";
        assert_eq!(
            crate_doc_example(lib),
            ["let x = 1;", "", "assert_eq!(x, 1);"]
        );
    }

    #[test]
    fn an_answer_holds_what_a_page_shows_of_it() {
        let have = json!({"a": 1.0, "b": {"c": [1, 2], "d": "x"}, "e": true});
        assert!(holds(&have, &json!({"a": 1.0, "b": {"c": [1, 2]}})));
        assert!(!holds(&have, &json!({"a": 2.0})));
        assert!(!holds(&have, &json!({"z": 1})));
        assert!(!holds(&have, &json!({"b": {"c": [1]}})));
        assert!(!holds(&have, &json!({"b": {"c": [1, 3]}})));
        assert!(!holds(&json!(1), &json!({"a": 1})));
    }

    #[test]
    fn a_curl_command_gives_its_path_and_its_body() {
        let command = "curl -X POST http://127.0.0.1:8080/api/v1/calculate \\\n  -H \"Content-Type: application/json\" \\\n  -d '{\"dry_run\": true}'";
        assert_eq!(
            curl_request(command),
            Some((
                "/api/v1/calculate".to_string(),
                "{\"dry_run\": true}".to_string()
            ))
        );
        assert_eq!(curl_request("curl -d '{}'"), None);
        assert_eq!(curl_request("curl http://x:1"), None);
        assert_eq!(curl_request("curl http://x:1/p"), None);
        assert_eq!(curl_request("curl http://x:1/p -d '{"), None);
    }

    #[test]
    fn a_post_gives_the_body_of_the_answer_and_waiting_ends_when_a_server_listens() {
        let port = TcpListener::bind(("127.0.0.1", 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let server = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            let listener = TcpListener::bind(("127.0.0.1", port)).unwrap();
            // the probe of wait_for, then the request
            drop(listener.accept().unwrap());
            let (mut stream, _) = listener.accept().unwrap();
            let sent = request_ending(&mut stream, "{\"a\":1}");
            assert!(sent.starts_with("POST /p HTTP/1.1\r\n"), "{sent}");
            assert!(sent.ends_with("\r\n\r\n{\"a\":1}"), "{sent}");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\n\r\n{\"ok\":true}")
                .unwrap();
        });
        wait_for(port);
        assert_eq!(post(port, "/p", "{\"a\":1}"), "{\"ok\":true}");
        server.join().unwrap();
    }

    #[test]
    fn an_answer_without_headers_is_given_whole() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            request_ending(&mut stream, "\r\n\r\n");
            stream.write_all(b"plain").unwrap();
        });
        assert_eq!(post(port, "/p", ""), "plain");
        server.join().unwrap();
    }

    #[test]
    fn a_link_to_a_file_that_is_not_there_is_an_error() {
        let page = root().join("CONTRIBUTING.md");
        assert!(check_links(&page, "[x](nowhere.md)").is_err());
        assert_eq!(
            check_links(&page, "[x](README.md) and [y](LICENSE-MIT)"),
            Ok(2)
        );
    }

    /// CONTRIBUTING.md's set-up is what the files that set the project up say.
    #[test]
    fn contributing_says_what_cargo_clippy_ci_and_the_makefile_say() {
        let (path, doc) = page("CONTRIBUTING.md");
        assert!(check_links(&path, &doc).is_ok_and(|n| n > 5));
        let rust = quoted_after(&read("Cargo.toml"), "rust-version").unwrap();
        assert_eq!(
            quoted_after(&read("clippy.toml"), "msrv"),
            Some(format!("{rust}.0"))
        );
        assert!(
            doc.contains(&format!("is {rust} (`rust-version`")),
            "the oldest Rust is {rust}"
        );
        let ci = ".github/workflows/ci.yml";
        let validation = ".github/workflows/validation.yml";
        let coverage = "scripts/coverage.sh";
        let claims = [
            (ci, "node-version: 22", "Node 22"),
            (ci, "wasm32-unknown-unknown", "wasm32-unknown-unknown"),
            (
                ci,
                "cargo install wasm-bindgen-cli --version",
                "cargo install wasm-bindgen-cli --version",
            ),
            (validation, "24.8", "24.8"),
            (
                validation,
                "-t noble-backports libreoffice-calc-nogui",
                "-t noble-backports libreoffice-calc-nogui",
            ),
            (
                coverage,
                "rustup toolchain install 1.100.0 --component llvm-tools-preview",
                "rustup toolchain install 1.100.0 --component llvm-tools-preview",
            ),
            (
                coverage,
                "RUSTUP_TOOLCHAIN=1.100.0",
                "RUSTUP_TOOLCHAIN=1.100.0",
            ),
            (
                "validation/gnumeric/README.md",
                "LibreOffice 24.8 or later",
                "LibreOffice 24.8 or later",
            ),
            (
                "validation/games/requirements.txt",
                "pygambit",
                "pip install -r validation/games/requirements.txt",
            ),
            (
                "Makefile",
                "markdownlint-cli2",
                "npm install -g markdownlint-cli2",
            ),
        ];
        assert_eq!(unheld(&doc, &claims, &read), Vec::<String>::new());
        let targets = make_targets(&read("Makefile"));
        for command in make_commands(&code_of(&doc)) {
            assert!(
                targets.contains(&command),
                "make {command} is not a target of the Makefile"
            );
        }
        for file in named_paths(&doc) {
            assert!(root().join(&file).exists(), "{file}");
        }
        assert!(read("Cargo.lock").contains("name = \"wasm-bindgen\""));
    }

    /// MAINTAINING.md names each job of the Release workflow and what a
    /// release changes.
    #[test]
    fn maintaining_names_every_job_secret_script_and_file_a_release_has() {
        let (path, doc) = page("docs/MAINTAINING.md");
        assert!(check_links(&path, &doc).is_ok_and(|n| n >= 3));
        let release = read(".github/workflows/release.yml");
        let jobs = job_ids(&release);
        assert_eq!(
            jobs,
            ["test", "build", "release", "studio", "publish", "homebrew", "install"]
        );
        for job in jobs.iter().chain(&["auto-release".to_string()]) {
            assert!(
                doc.contains(&format!("`{job}`")),
                "the page does not name the job `{job}`"
            );
        }
        assert!(job_ids(&read(".github/workflows/ci.yml")).contains(&"auto-release".to_string()));
        assert_eq!(
            release.matches("- target: ").count(),
            5,
            "the page says five targets"
        );
        assert!(doc.contains("five targets") && doc.contains("five archives"));
        let (rel, ci) = (".github/workflows/release.yml", ".github/workflows/ci.yml");
        let claims = [
            (rel, "CRATES_IO_TOKEN", "CRATES_IO_TOKEN"),
            (
                rel,
                "cargo fmt --all -- --check",
                "cargo fmt --all -- --check",
            ),
            (
                rel,
                "cargo clippy --all-targets -- -D warnings",
                "cargo clippy --all-targets -- -D warnings",
            ),
            (rel, "FORGE_RELEASE", "FORGE_RELEASE=1"),
            (rel, "sha256sum -c", "sha256sum -c"),
            (
                rel,
                "actions/attest-build-provenance",
                "actions/attest-build-provenance",
            ),
            (rel, "continue-on-error", "continue-on-error"),
            (rel, "cosign sign-blob", "cosign sign-blob"),
            (rel, "cargo cyclonedx", "cargo-cyclonedx"),
            (rel, "forge.cdx.json", "forge.cdx.json"),
            (rel, "gh attestation verify", "gh attestation verify"),
            (
                rel,
                "scripts/release-checksums.sh",
                "scripts/release-checksums.sh",
            ),
            (
                rel,
                "scripts/homebrew-formula.sh",
                "scripts/homebrew-formula.sh",
            ),
            (rel, "pages.yml", "pages.yml"),
            (rel, "install.yml", "install.yml"),
            (rel, "for attempt in 1 2 3", "retries three times"),
            (
                ci,
                "gh workflow run release.yml --ref",
                "gh workflow run release.yml --ref",
            ),
            (
                ci,
                "needs: [test, lint, build, repo-checks]",
                "`repo-checks`",
            ),
            (
                ".github/workflows/weekly.yml",
                "cargo audit",
                "`cargo audit`",
            ),
            (".github/workflows/weekly.yml", "Mondays", "Mondays"),
            (".github/workflows/validation.yml", "Mondays", "Mondays"),
            (
                ".github/workflows/validation.yml",
                "workflow_dispatch",
                "`workflow_dispatch`",
            ),
            (
                ".github/dependabot.yml",
                "package-ecosystem",
                ".github/dependabot.yml",
            ),
        ];
        assert_eq!(unheld(&doc, &claims, &read), Vec::<String>::new());
        for file in named_paths(&doc) {
            assert!(root().join(&file).exists(), "{file}");
        }
    }

    /// A release changes the version where `docs/MAINTAINING.md` says it is
    /// written, and no other place says another.
    #[test]
    fn maintaining_lists_the_places_that_write_the_version() {
        let (_, doc) = page("docs/MAINTAINING.md");
        let version = quoted_after(&read("Cargo.toml"), "version").unwrap();
        for file in ["README.md", "docs/STABILITY.md"] {
            assert!(
                read(file).contains(&format!("(v{version})")),
                "{file} has no (v{version})"
            );
            assert!(
                doc.contains(&format!("`{file}`")),
                "the page does not name {file}"
            );
        }
        assert!(read("Cargo.lock").contains(&format!(
            "name = \"mollendorff-forge\"\nversion = \"{version}\""
        )));
        // metadata.rs holds these two to the version, so a release changes them
        for file in ["CITATION.cff", "server.json"] {
            assert!(read(file).contains(&version), "{file} has no {version}");
            assert!(
                doc.contains(&format!("`{file}`")),
                "the page does not name {file}"
            );
        }
        assert!(read("CHANGELOG.md").contains("## [Unreleased]"));
        // the example dependency in EMBEDDING.md is held to the version by
        // the embedding check, so a release changes it too
        assert!(
            doc.contains("`docs/EMBEDDING.md`"),
            "the page does not name EMBEDDING.md"
        );
    }

    /// EMBEDDING.md's Rust example is the crate's own, which `cargo test --doc`
    /// runs; its version is the crate's; its REST example is sent to a real
    /// `forge serve` and the answer holds what the page shows.
    #[test]
    fn embedding_shows_the_crates_example_its_version_and_a_rest_answer_that_is_true() {
        let (path, doc) = page("docs/EMBEDDING.md");
        assert!(check_links(&path, &doc).is_ok_and(|n| n > 5));
        let blocks = fenced_blocks(&doc);
        let block = |lang: &str, starts: &str| {
            blocks
                .iter()
                .find(|(l, lines)| l == lang && lines.join("\n").starts_with(starts))
                .expect("the page has the block")
                .1
                .clone()
        };
        assert_eq!(
            block("rust", "use std"),
            crate_doc_example(&read("src/lib.rs"))
        );
        let version = quoted_after(&read("Cargo.toml"), "version").unwrap();
        assert!(block("toml", "[dependencies]")
            .join("\n")
            .contains(&format!("version = \"{version}\"")));
        assert!(read(".github/workflows/ci.yml")
            .contains(&block("bash", "cargo build -p forge-wasm").join("\n")));
        let (server, port) = Server::start();
        wait_for(port);
        let (url_path, body) = curl_request(&block("bash", "curl").join("\n")).unwrap();
        let answer: Value = serde_json::from_str(&post(port, &url_path, &body)).unwrap();
        let shown: Value = serde_json::from_str(&block("json", "{").join("\n")).unwrap();
        assert!(holds(&answer, &shown), "the server answered {answer}");
        drop(server);
    }
}
