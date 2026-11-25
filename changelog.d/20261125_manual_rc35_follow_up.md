### Changed

- **The pipeline example and the modelling skill assume Forge 10.0.0-rc.35.** `examples/pipeline/forge.yml` installs rc.35 and no longer checks `uncalculated` with `jq`, since `forge validate` now fails a formula with no stored result; the skill drops its notes for older releases.
