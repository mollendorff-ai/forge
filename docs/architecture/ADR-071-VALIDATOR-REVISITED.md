# ADR-071: Forge's Schema Reader or jsonschema, Decided Without Size

**Status:** Proposed
**Date:** 2026-11-21
**Decided by:** the maintainer (to decide); the measurements and the recommendation are Claude's
**Amends:** ADR-069
**Closes:** roadmap T1-REVISIT-VALIDATOR

---

## Context

ADR-069 (2026-11-20) replaced the `jsonschema` crate with Forge's own reader of its schema files (`schema_reader.rs`, `schema_check.rs`, `schema_pattern.rs`). Its context weighed `jsonschema` mostly by its size in the browser module, against a budget that had no stated purpose. ADR-070 (2026-11-21) gave the budget one, a 3,000,000-byte ceiling from a load-time target, and ruled that size alone is no reason to replace a stable library with code of our own. The maintainer was uncomfortable with the removal. This ADR asks the question again on the other merits.

Measured on 2026-11-21, on main (`ac9b919`) with `jsonschema` linked back into every build as it would be used, `default-features = false`, without `idna`, its crates built at `opt-level = "z"`:

| | Forge's reader (main) | `jsonschema` 0.60 |
| --- | --- | --- |
| Browser module, gzipped, before wasm-bindgen | 1,355,065 bytes | 1,964,618 bytes (+609 KB, +45%) |
| At 10 Mbps (ADR-070) | about 1.1 s | about 1.6 s |
| Under the 3,000,000-byte ceiling | yes | yes |
| Crates in two versions, besides those main already has (`memchr`, `serde_core`, `serde_json`, `syn`) | none | `getrandom` 0.3 beside 0.4, in every build: the command line, the servers and the browser |
| Messages | the same lines (asserted) | the same lines |
| Unicode names (`hypothèses.coût_unitaire`, `假设.单价`) | accepted | accepted (and by Python's `jsonschema` 4.26, a third implementation) |
| Code to keep | 2,879 lines with the message layer, which both need (893 of them `schema_errors.rs`) | the message layer, and keeping up with the crate's error types |

What the numbers rest on:

- **Same answers.** In every test run, every model any test reads goes through both, and they must refuse the same models with the same message lines (`schema_reader::tests::agrees_with_jsonschema`, called from `validate_against_schema` under `cfg(test)`), and so must 78,838 models generated from the examples, over 30,000 of them refused. Messages are Forge's either way: `schema_errors.rs` writes them from the errors, whichever validator found them.
- **What the schemas need.** About twenty keywords (`type`, `$ref` to `#/definitions/`, `properties`, `additionalProperties`, `required`, `anyOf`, `oneOf`, `allOf`, `not`, `if`/`then`/`else`, `enum`, `items`, `minItems`, `maxItems`, `minimum`, `maximum`, `minProperties`, `pattern`, `format: date`), and six patterns, all dates and `^=`. The reader refuses every model when a schema uses a keyword it does not read, so a new keyword cannot be passed over in silence.
- **The second `getrandom`.** `jsonschema`, `jsonschema-value` and `referencing` (0.60 and 0.62, the latest) take `ahash` with its default features, whose `runtime-rng` brings `getrandom` 0.3; Forge uses 0.4. Forge cannot turn the feature off from its side. CLAUDE.md allows no second version of any crate but `syn`, and no lint allowance for it.

## Decision (proposed)

**Keep Forge's reader; keep `jsonschema` as the test it is checked against.** ADR-069 stands, for these reasons, none of them size:

1. **`jsonschema` breaks a rule that is not negotiable.** It brings `getrandom` 0.3 into every build, and CLAUDE.md allows no second version of a crate. Allowing it would raise a budget that may only fall (`validation/repo`).
2. **Nothing is gained in what Forge accepts or says.** The two agree on every model the tests read and on 78,838 generated ones, with the same lines, and the schemas use a small, fixed part of JSON Schema.
3. **The risk of code of our own is bounded and watched.** The reader refuses what it does not know, and `jsonschema` stays in `[dev-dependencies]` to check it on every test run. A test in `validation/repo` makes that permanent: it fails if `jsonschema` leaves the dev-dependencies or the comparison stops being called from `validate_against_schema`.

The +609 KB would fit under ADR-070's ceiling, and ADR-070 says size alone would not decide; it does not decide here.

## When to decide again

- `jsonschema` (or `ahash`) stops bringing a second `getrandom`, for example by taking `ahash` without default features or moving to a hasher on `getrandom` 0.4. Then the rule no longer decides, and the choice is the reader's 2,000 lines against the crate's upkeep.
- A schema needs what is large to write well: references outside the file, `unevaluatedProperties`, other formats, a newer draft.
- The comparison finds a difference the reader cannot fix simply.

## Alternatives considered

- **Return to `jsonschema` everywhere, the reader removed.** Same answers and messages, +609 KB, and a second `getrandom` against CLAUDE.md. Possible only with an allowance the rules forbid, or after an upstream change.
- **`jsonschema` natively and the reader in the browser.** Two validators, which ADR-069 rules out: the same answer everywhere is the promise.
- **Ask upstream to take `ahash` without default features.** Worth an issue: it would remove the reason above. `ahash` without `runtime-rng` uses fixed keys, which `jsonschema`'s maintainers may not want for a crate that hashes untrusted input, so it is not something to wait on.
