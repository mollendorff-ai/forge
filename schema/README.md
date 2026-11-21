# Forge JSON Schema

JSON Schemas for the Forge model formats. The version is the model format (`_forge_version` in a file), not the version of Forge.

| File | Format |
| --- | --- |
| `forge.schema.json` | Any of them, for editors: it checks a file against the schema of the format its `_forge_version` names (5.5.0, 5.4.0, 5.3.0, 5.2.0 or 5.1.0; for 5.0.0 and 1.0.0 it checks only the version, and `forge validate` does the rest). |
| `forge-v5.5.0.schema.json` | 5.5.0 (ADR-068): 5.4.0 plus `games:`, each game with players, their strategies and a payoff table whose cells are numbers or formulas over the model (`hare: {stag: =3 - c, hare: =3 - c}`). The schema checks a game's shape; Forge checks it in full and names the cell that is wrong. Needed only by a file that has a game. |
| `forge-v5.4.0.schema.json` | 5.4.0 (ADR-060, ADR-061): 5.3.0 plus a date as a scalar (`last_actual: 2026-10-31`) and `format:` beside `unit:` (`{value: 16, format: "$#,##0.00"}`). Needed only by a file that has one. |
| `forge-v5.3.0.schema.json` | 5.3.0 (ADR-059): 5.2.0 plus the empty cell, `null` in a column of numbers (`actual: [985, 1032, null]`). Needed only by a file that has one. |
| `forge-v5.2.0.schema.json` | 5.2.0 (ADR-053): 5.1.0 plus the period column, a table column written as `{start: 2026-12, count: 300, unit: month}`. Needed only by a file that has one. |
| `forge-v5.1.0.schema.json` | 5.1.0 (ADR-051): 5.0.0 plus plain-number inputs (`rate: 0.05`) and formula scalars written as `{formula: "=..."}` without `value: null`. `examples/` use it. |
| `forge-v5.0.0.schema.json` | 5.0.0: tables, row formulas, includes and the analysis sections (`monte_carlo`, `scenarios`, `decision_tree`, `tornado` and the others). `test-data/` uses it. |
| `forge-v1.0.0.schema.json` | 1.0.0: scalars only. |

Forge embeds these files: `forge schema v5.5` (or `v5.4`, `v5.3`, `v5.2`, `v5.1`, `v5`, `v1`) prints the one of the Forge you have installed, and every model is checked against its schema before it is calculated. So a mixed array (`[100, "Q2", 150]`) or a formula without its `=` (`total: "SUM(revenue)"`) is an error from `forge validate` and `forge calculate`. The 5.1.0 to 5.5.0 schemas read each top-level mapping the way Forge does (one scalar, a table or a group of scalars, ADR-051), so an error is reported under the one reading that applies, with its key path: `'loan.payment.formla': there is no key 'formla' here. Did you mean 'formula'?` The schema checks the shape of a file; whether a formula's names exist and its stored results are current is what `forge validate` checks after that.

Forge checks every model against these files with the `jsonschema` crate (0.60, draft 07, `src/parser/schema_reader.rs`, ADR-071), the same on the command line, in the MCP server, in the REST API and in the browser, and says what does not fit in its own words (`src/parser/schema_errors.rs`). `format: date` is checked, as draft 07 lets a validator check it: a real day, `YYYY-MM-DD`; an editor whose JSON Schema engine treats `format` as a note can disagree with Forge where it decides the form a value is read in. From 2026-11-20 to 2026-11-21 a reader of Forge's own did the checking (ADR-069); `jsonschema` replaced it again because it is the implementation the JSON Schema test suite checks, and Forge's was not.

## In an editor

Any editor that runs [yaml-language-server](https://github.com/redhat-developer/yaml-language-server) (VS Code with Red Hat's YAML extension, Zed, Neovim, Helix) can use the schema to complete keys and to mark mistakes as you type. The schema for every format, on the main branch (it reads the schemas of the formats from beside it):

```text
https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge.schema.json
```

**In the file**, as its first line (works in every such editor):

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge.schema.json
_forge_version: "5.2.0"
```

**VS Code**: the extension in [`editors/vscode`](../editors/vscode/README.md) applies the schema to `*.forge.yaml` files and highlights formulas. For other file names, map them in `settings.json`:

```json
"yaml.schemas": {
  "https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge.schema.json": "models/**/*.yaml"
}
```

**Zed** has YAML and yaml-language-server built in (there is no Zed extension: one could highlight formulas only in a language of its own, which would lose Zed's YAML language server). In Zed's `settings.json`:

```json
"lsp": {
  "yaml-language-server": {
    "settings": {
      "yaml": {
        "schemas": {
          "https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge.schema.json": "models/**/*.yaml"
        }
      }
    }
  }
}
```

**JetBrains IDEs**: Settings > Languages & Frameworks > Schemas and DTDs > JSON Schema Mappings, with the URL above and a file pattern.

In CI, `editors/vscode/test/schema.js` validates every example with yaml-language-server against this schema.

## Changing a schema

A change here changes what `forge validate` accepts. After one:

```bash
cargo test                                                    # every example still passes (src/cli/commands/examples.rs (its demo_examples tests))
cp schema/forge.schema.json schema/forge-v5.[1-5].0.schema.json editors/vscode/schemas/    # the extension's copies; its test checks they match
```
