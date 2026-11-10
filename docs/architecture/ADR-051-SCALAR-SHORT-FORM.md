# ADR-051: Scalar Short Form, No Nulls, Results Written Back in Place

**Status:** Accepted
**Date:** 2026-11-08
**Decided by:** the maintainer (format and write-back); details by Claude Opus 5.5

---

## Context

A Forge scalar was always a mapping with two slots:

```yaml
assumptions:
  price_per_dozen:
    value: 16
    formula: null
year:
  profit:
    value: null
    formula: "=revenue - costs"
```

So a model was full of `null`: `formula: null` on every input, `value: null` on every formula until it was calculated. The nulls are Forge's internal state showing through. They double the length of a model, they are what LLMs most often get wrong, and the maintainer found them confusing to read.

`forge calculate` then wrote the results into the file by serializing the whole YAML document again. That deleted every comment, removed quotes, and turned `16` into `16.0`, so each run damaged what the author had written.

What the maintainer wants to keep: **one file that holds formulas and their results**, the way an Excel cell holds a formula and its last value.

## Decision

1. **Short form.** In format 5.1.0, an input scalar may be a plain number, and a formula scalar may be a mapping with only `formula:`:

   ```yaml
   _forge_version: "5.1.0"
   assumptions:
     price_per_dozen: 16
   year:
     profit:
       formula: "=revenue - costs"
       unit: CAD
   ```

   Metadata (`unit`, `notes`, `source`, `better`, ...) still uses the mapping, with `value:` for an input or `formula:` for a formula. The long form, nulls included, stays valid in 5.1.0 and in 5.0.0.

2. **One file, results in place.** `forge calculate` still writes the results into the model by default, keeping a `.bak`. It writes only the `value:` of scalars that have a formula, adding it if missing, and changes nothing else: comments, quotes, key order, blank lines and inputs stay byte for byte as typed. After editing, Forge reads the new text back and checks that the only differences are those values; if anything else would change, it does not write and says so.

3. **Format version 5.1.0, field name unchanged.** `_forge_version` is the version of the model format, not of the tool; the tool is 10.x and the format moves only when files change. The short form is a backward-compatible addition, so it is a minor version under Semantic Versioning: 5.0.0 files stay valid, and an older Forge reading a 5.1.0 file fails with "Unsupported _forge_version" instead of misreading it. The field keeps its name, because renaming it would touch every model and the existing habits of every reader, and the old name would have to be accepted anyway; the schema and FEATURES.md say what it means instead.

4. **Conversion is explicit.** `forge upgrade --to 5.1.0` rewrites a 5.0.0 model in the short form, editing in place like calculate (comments kept). It checks that the converted file parses to the same model as the original before writing. Nothing changes a file's format on its own.

## How a group is read

A mapping under a top-level key is:

- **one scalar** if it has a `value` that is not a list, or a `formula` and no child mapping (and is not a table); its keys are then `value`, `formula` and the metadata, and any other key is an error;
- a **table** if any child is a list (a column) or a mapping whose `value` is a list (since 2026-11-09 and format 5.2.0, also if a child is a period column, a mapping with `start` and `count`: ADR-053);
- otherwise a **scalar group** if any child is a number or a mapping with `value` or `formula`;
- otherwise a table of formula columns.

The 5.1.0 schema applies these rules in this order (`if`/`then`/`else`, since 2026-11-09), so a mistake is reported under the reading Forge will use. It used to accept a mapping under any reading that fitted, and Forge then read some of them another way and dropped part without a word (`single: {value: 3, doubled: {formula: ...}}` lost `doubled`).

In a scalar group, a bare formula string (`profit: "=revenue - costs"`) is an error that says to write `profit: {formula: "=revenue - costs"}`: a bare string has no place to hold its result, and writing one back would change what the author typed. In a table, a bare formula string is a row formula, as before.

## Alternatives considered

- **Only short form, no write-back** (results shown, never stored). Cleanest file, but loses "open the file and see the numbers", which is the point.
- **Formula and result on one line** (`profit: "=revenue - costs" # 30000`). Compact, but data in a comment is fragile and invisible to YAML tools.
- **Two files** (`model.yaml` and `model.calculated.yaml`). Best for git diffs, but two files where the maintainer wants one. Can be added later as a flag without changing this decision.
- **A `_results:` section in the same file.** One file, but every formula name appears twice.
- **Tie the format version to the tool's major version.** Every tool major would then stamp a new format on unchanged files. Rejected.
- **Version 6.0.0.** A major bump says old files break; they do not. Rejected.

## Consequences

- Models in `examples/` are converted to 5.1.0 with `forge upgrade`, and each one must calculate to the same results as before (checked byte for byte in the PR). `test-data/` and the two validation repositories stay at 5.0.0, so the long form keeps being tested.
- A formula scalar's stored `value` can be stale after an input changes, as with any stored result; `forge validate` reports values that do not match their formulas, and `forge calculate` says how many it updated.
- Supersedes the edition framing of ADR-014 for `_forge_version`: the versions are formats, not editions.
