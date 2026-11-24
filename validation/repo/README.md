# mollendorff-forge-repo

What Forge says about itself is what it does. These checks compare the
repository's documents, workflows and sources with its code and with what the
`forge` binary prints, so a doc, a count or a rule cannot drift from the engine:

| Module | Checks |
|---|---|
| `conventions` | The conventions in `.asimov/project.yaml` a test can check: no Rust file over 1,500 lines, no `partial_cmp().unwrap()` |
| `no_skipping` | Every way round the coverage, lint and formatting rules, counted; the counts only fall (roadmap T1-NO-SKIPPING) |
| `doc_counts` | The function and test counts written in the docs |
| `doc_examples` | Every model in docs/EXAMPLES.md runs and prints the numbers the doc shows |
| `readme_commands` | Every `$ forge` command in the README prints the lines the README shows |
| `help_examples` | Every example in `forge <command> --help` names a command and flags Forge has, and the models whose YAML CONFIGURATION is a whole one (bayesian, bootstrap, decision-tree, real-options) run with their examples |
| `agent_rules` | The rules `docs/guide/ai.md` gives an agent for a model repository: each rule is numbered, has a row under the template naming the tools it rests on, and a case that starts `forge mcp --root` on a copy of `examples/`, makes those calls one at a time and asserts the answers |
| `guide` | Every page of docs/guide/ (ADR-078): each `$ forge` command runs in a copy of `examples/` and prints the lines the page shows, each `yaml` block headed by `# examples/….yaml` is that file, a block headed by another file (`# bagel.yaml`) is written for the commands after it and one headed `# bagel.yaml, after ...` or `# examples/….yaml, part of it` is lines of it, in a row, as they left it, a command fails only where the page shows its `Error:` line, every link and anchor resolves, and every number in a `<details>` answer is one a command printed; and docs/cli/README.md is what `forge --help` prints |
| `posts` | The numbers in docs/posts/ are what the commands print |
| `studio_guide` | The Studio guide (docs/guide/studio.md): every picture has alternative text, is a file in `docs/guide/img/` and is one `studio/screenshots.mjs` makes, the script makes no picture the page does not show, every `studio/tests/*.mjs` the page names exists, and the numbers it quotes are the first lesson's; its commands and links are `guide`'s |
| `roles` | The pages for a contributor, a maintainer and a developer who embeds Forge (ADR-078): CONTRIBUTING.md says the Rust, Node, LibreOffice and wasm-bindgen versions and commands that Cargo.toml, clippy.toml, the workflows and `scripts/coverage.sh` hold, and every `make` target it names is in the Makefile; docs/MAINTAINING.md names every job of `release.yml`, the secrets, scripts and files a release uses, and each place the version is written has it; docs/EMBEDDING.md shows the crate's own documentation example, the crate's version, and a REST request that is sent to a running `forge serve` whose answer holds what the page shows; every link and path of the three resolves |
| `workflows_doc` | Every command docs/WORKFLOWS.md names exists |
| `install_script` | `install.sh`, the README and the prompts install what the release publishes |
| `architecture_docs` | Every link and anchor in docs/architecture resolves, every source path its pages name exists, every ADR is in its index with the status the ADR states, and every module of `src/lib.rs` is in its module map |

Each module's code finds what its check reads, and its tests, inline, assert
that the repository passes. The checks that run commands run the built binary,
as a reader types them, like the [Gnumeric](../gnumeric) and [R](../r) suites
(ADR-057):

```bash
cargo build --bin forge && cargo test -p mollendorff-forge-repo
FORGE_BIN=target/release/forge cargo test -p mollendorff-forge-repo   # another build
```

Every test of the main crate is inline, in the file it tests; there is no
`tests/` folder (CLAUDE.md). A test of the command line calls it in-process
(`cli::forge`, `mcp::Session`); what only a real process shows (a closed pipe)
is a step of CI's MCP Protocol Test job.
