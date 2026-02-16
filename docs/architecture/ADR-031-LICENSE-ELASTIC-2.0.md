# ADR-031: License Change - Elastic-2.0 to MIT OR Apache-2.0

**Status**: SUPERSEDED
**Date**: 2025-12-29 (original), 2026-02-16 (updated)
**Supersedes**: ADR-030 (license section only)

---

## Context

Forge was originally licensed under Elastic License 2.0 (source-available, not FOSS).

In February 2026, the decision was made to open-source Forge under the standard
Rust dual license (MIT OR Apache-2.0) for portfolio visibility and community adoption.

---

## Original Decision (December 2025)

Elastic-2.0 was chosen to protect commercial interests:
- No automatic conversion to open source
- Blocks hosted/managed service offerings
- Enterprise-recognized license

## Updated Decision (February 2026)

**License: MIT OR Apache-2.0**

Reasons for the change:
1. **Portfolio visibility** — Forge is being open-sourced as a CV/portfolio project
2. **Rust ecosystem standard** — rustc, Tokio, Serde, Axum all use this dual license
3. **Maximum adoption** — No friction for users, contributors, or evaluators
4. **Future flexibility** — Commercial services (SaaS, support) can be built on top of MIT-licensed code
5. **Patent grant** — Apache-2.0 provides explicit patent protection

---

## Implementation

| File | Change |
|------|--------|
| `LICENSE-MIT` | MIT license text |
| `LICENSE-APACHE` | Apache 2.0 license text |
| `Cargo.toml` | `license = "MIT OR Apache-2.0"` |
| `README.md` | Updated badge and license section |
| `LICENSE` | Deleted (was Elastic-2.0) |
| `COMMERCIAL_LICENSE.md` | Deleted |

---

## References

- [SPDX MIT](https://spdx.org/licenses/MIT.html)
- [SPDX Apache-2.0](https://spdx.org/licenses/Apache-2.0.html)
- [Rust API Guidelines - C-PERMISSIVE](https://rust-lang.github.io/api-guidelines/necessities.html#crate-and-its-dependencies-have-a-permissive-license-c-permissive)
