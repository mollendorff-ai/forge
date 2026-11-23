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
| `posts` | The numbers in docs/posts/ are what the commands print |
| `workflows_doc` | Every command docs/WORKFLOWS.md names exists |
| `install_script` | `install.sh`, the README and the prompts install what the release publishes |
| `architecture_docs` | Every link and anchor in docs/architecture resolves, every source path its pages name exists, every ADR is in its index, and every module of `src/lib.rs` is in its module map |

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
