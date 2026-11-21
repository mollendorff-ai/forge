# ADR-071: jsonschema Checks Every Model Again

**Status:** Accepted; built
**Date:** 2026-11-21
**Decided by:** the maintainer ("is jsonschema closer to the industry standard than our code, and more battle-proven? If so, let's bring it back"; and the rule against duplicate crates "must not compete with replacing a core stable library"); the measurements are Claude's
**Supersedes:** ADR-069
**Closes:** roadmap T1-REVISIT-VALIDATOR

---

## Context

ADR-069 (2026-11-20) replaced the `jsonschema` crate with a reader of Forge's own (`schema_reader.rs`, `schema_check.rs`, `schema_pattern.rs`, about 2,000 lines), weighed mostly by its size in the browser against a budget that had no stated purpose. ADR-070 (2026-11-21) gave the budget one, a 3,000,000-byte ceiling from a load-time target, and ruled that size alone is no reason to replace a stable library with code of our own. The maintainer was uncomfortable with the removal and asked the question on its merits.

**The standard.** `jsonschema` (jsonschema-rs) implements JSON Schema drafts 4 to 2020-12 and is listed on Bowtie, the JSON Schema organisation's report that runs the official JSON Schema Test Suite against each implementation; its own CI runs that suite. It has years of releases and users outside Forge. Forge's reader read only the twenty or so keywords Forge's seven schema files use, had no test against the official suite, and was one day old: its correctness was borrowed from agreeing with `jsonschema` on Forge's models.

**What changes for a user.** Nothing in what is accepted or said. While the reader was in place, every model any test read went through both, and they had to refuse the same models with the same message lines, as did 78,838 models generated from the examples. The messages are Forge's either way: `schema_errors.rs` writes them from the errors, whoever found them.

**The cost, measured 2026-11-21:**

| | Forge's reader | `jsonschema` 0.60 |
| --- | --- | --- |
| Browser module, gzipped, before wasm-bindgen | 1,355,065 bytes | 2,037,051 bytes (+682 KB) |
| At 10 Mbps (ADR-070) | about 1.1 s | about 1.6 s, under the 3,000,000-byte ceiling |
| Crates in two versions | `syn` | `syn`, and `getrandom` 0.3 beside 0.4 with its `r-efi` 5 beside 6 |
| Code of our own | about 2,000 lines | the conversion of its errors (`schema_reader.rs`), and the messages both need |

`jsonschema`, `jsonschema-value` and `referencing` (0.60 and 0.62, the latest) take `ahash` with its default features, whose `runtime-rng` brings `getrandom` 0.3; Forge, `rand` and `zip` are on 0.4. `ahash` uses it only to seed its hash maps; a model's numbers never pass through it.

## Decision

1. **`jsonschema` checks every model** on the command line, in the MCP server, the REST API and the browser, against the schema file of the model's format, read as draft 07 (the draft every file declares, in which `format: date` is a rule). It is a dependency again, with no default features: nothing is fetched or read from a file, and the `idna` stack is not needed for `format: date`.
2. **Its errors are said in Forge's words**, as before: `schema_reader.rs` turns each into one of the kinds `schema_errors.rs` has words for. A kind no schema of Forge's gives (`const`, say) is reported as a bug in Forge, and a test makes sure every kind the schemas give has words, on every model made from the examples.
3. **The reader is removed:** `schema_check.rs`, `schema_pattern.rs` and its keyword engine, and the `regex` dev-dependency that only checked its pattern matcher.
4. **`getrandom` may be in two versions, and `r-efi` with it**, beside `syn` in `clippy.toml`'s `allowed-duplicate-crates`; the budget in `validation/repo` rises from 1 to 3. `getrandom` 0.3.4, the last 0.3, takes `r-efi` 5 for UEFI firmware where 0.4 takes `r-efi` 6, so the one cause brings two duplicates. The maintainer decided that the rule against duplicate crates does not compete with keeping a core, stable library. The exception goes when `jsonschema` no longer brings `getrandom` 0.3.
5. **The browser module grows by 682 KB**, said in the CHANGELOG as ADR-070 asks; it stays under the ceiling.

## Consequences

- A change to a schema file can use any draft-07 keyword `jsonschema` supports. If it gives an error Forge has no words for, the message says it is a bug in Forge, and the test of every generated model fails first.
- Upgrading `jsonschema` can change its error types; the conversion in `schema_reader.rs` and the message tests show where.
- ADR-069 is superseded. Its other points stand where they did not depend on the reader: one validator on every surface, the same messages everywhere, and forge-wasm's refused models checked against the command line's message.

## When to decide again

- `jsonschema` stops bringing `getrandom` 0.3: remove the two exceptions.
- The browser module nears ADR-070's ceiling: first build `jsonschema`'s crates smaller (`opt-level = "z"` for them saved about 70 KB, PR #296), before anything else.
