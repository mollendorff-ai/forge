# Changelog fragments

Every change adds its changelog entry here, as a new file, and never in
`CHANGELOG.md`. Four lanes merge into main at once (`AGENTS.md`); one file
per change means no two pull requests edit the same lines.

Name the file `<date>_<lane>_<what>.md`, for example
`20261125_engine_lanes.md`. It holds one or more of the headings
`### Added`, `### Changed`, `### Fixed` and `### Removed`, each followed by
its entries in plain words that say what a user saw and what changed:

```markdown
### Fixed

- **`forge validate` fails a formula with no stored result.** It used to pass.
```

When the engine lane makes a release, `make changelog VERSION=10.0.0-rc.N`
runs [scriv](https://scriv.readthedocs.io) to move every fragment into
`CHANGELOG.md` under a dated heading and delete it from here
(`docs/MAINTAINING.md`). `validation/repo/src/lanes.rs` checks each
fragment's headings and that `[Unreleased]` holds only scriv's marker.
