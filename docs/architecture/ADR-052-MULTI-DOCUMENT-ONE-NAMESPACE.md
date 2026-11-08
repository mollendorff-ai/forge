# ADR-052: The Documents of a File Share One Namespace

**Status:** Accepted
**Date:** 2026-11-08
**Decided by:** the maintainer asked for multi-document models to work properly; the design is Claude Opus 5.5's

---

## Context

A Forge file may hold several YAML documents separated by `---`, each with an optional `_name`. `forge import --multi-doc` writes one document per sheet. The parser stored every table, scalar and scenario as `document.name` (`summary.totals.tax`), but left each document's formulas as written (`=SUM(sales.tax)`, `=totals.tax * 2`). So a formula could not find any table or scalar group, in another document or even in its own: only a bare column inside its own table resolved. The test file for multi-document models used `=1000+1200+1500` where it meant `=SUM(sales.revenue)`.

`forge calculate` did not write results back into multi-document files at all, and `forge import --split-files` and `--multi-doc` declared format 1.0.0, which allows no tables, so their output failed validation (fixed earlier the same day).

## Decision

1. **One namespace.** The documents of a file are one model laid out in parts. Their tables, scalars and scenarios are merged under their own names, so a formula refers to a table or scalar in any document exactly as it would in a single file. `_name` labels a document; it is not part of any name.
2. **A name defined twice is an error** that names both documents ("Scalar 'revenue.total' is defined in document 'budget' and again in 'actual'"). Two versions of the same model belong in two files, which is what `forge variance` compares.
3. **A formula that starts with a document label** (`=SUM(data.sales.amount)`, the old stored name) is an error that says what to write instead (`sales.amount`).
4. **`forge calculate` writes results into multi-document files**, document by document, with the same in-place editing and checks as a single file (ADR-051).
5. **`forge import --split-files` refuses**, before writing anything, a model whose parts refer to each other, and says which formulas would break. Separate files cannot reach each other's tables until includes support table columns (roadmap T2-INCLUDE-COLUMNS). A self-contained model is still split.

## Alternatives considered

- **Keep the `document.` prefix and rewrite each document's formulas to it**, resolving a short name to its own document first and then to a unique match elsewhere. This allows the same group name in two documents, but rewrites formulas, and every result and export name keeps an extra segment that the user never wrote. Nothing relied on the prefixes, because no formula could use them.
- **Merge tables of the same name across documents** (columns from each). Quietly combining half-tables hides mistakes; rejected.
- **Make `--split-files` write `_includes`.** Includes read scalars only, so tables would still be unreachable; revisit with T2-INCLUDE-COLUMNS.

## Consequences

- Result names in multi-document models lose the document segment (`totals.tax`, not `summary.totals.tax`). Formulas that did not work before now do. No file format version change: files are read the same way; only names that could not be referred to change.
- Dotted table names, which came only from the prefixes and did not survive `forge import`, no longer arise.
- `forge import --multi-doc` of the bagel example calculates to exactly the numbers of the single-file import (tested).
