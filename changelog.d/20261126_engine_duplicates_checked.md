### Changed

- **The duplicate crate versions Forge allows are checked against the newest releases, and the reasons written next to them are corrected.** `getrandom` 0.3 is held on `wasm32` by `jsonschema` itself, not by `ahash` (0.8.14 no longer brings it), and `strum_macros` also holds `syn` 2. Nothing could be removed: the count stays at 4 (`syn`, `getrandom`, `r-efi`, `hashbrown`). No behaviour changes.
