# Contributing to Forge

## How to Contribute

| Method | Description |
|--------|-------------|
| **Issues** | Report bugs, request features |
| **Discussions** | Ask questions, share ideas |
| **Pull Requests** | Bug fixes, new functions, documentation |
| **Forks** | Create your own version |

## Development

```bash
make check          # Format + lint + unit tests
make test           # Run all tests
make lint           # Clippy with pedantic lints
```

## What a change needs

- **An entry in CHANGELOG.md**, under `[Unreleased]`, saying in plain words what a user saw and what changed.
- **A test that fails without the change** for a fix to a wrong answer.
- **An example with outside numbers** for a new or changed command: a model in `examples/` whose results come from a calculation made outside Forge (by hand, a spreadsheet, R, Python), asserted in a test. `tests/bagel_example.rs` shows how. Using the bagel models found more bugs in one evening than the unit tests had.
- **Green CI**, including the Validation workflow, which runs forge-e2e-gnumeric and forge-e2e-r. A change that needs new cases there uses a branch of the same name in that repository.
- **Docs that stay true**: update what the change makes wrong; a page about a design that no longer exists gets a dated note at the top.

The pull request template has these as a checklist. `.asimov/project.yaml` lists the conventions; `tests/conventions.rs` checks the ones a test can.

## License

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as MIT OR Apache-2.0,
without any additional terms or conditions.
