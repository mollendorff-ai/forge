### Changed

- **docs/SESSIONS.md says how to measure coverage, check an advisory or a signature, and find a duplicate crate from a fresh cloud container.** `cargo-llvm-cov` and the wasm32 target have to be installed first; osv.dev, scorecard.dev's API and Sigstore's TUF CDN are blocked, with what works instead; and `cargo tree -d --target all` shows the duplicates clippy counts. It also says how to measure the coverage of `validation/repo`, and how to wait for the pre-commit hook without the wait finding itself. No behaviour changes.
