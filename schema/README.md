# Forge JSON Schema

JSON Schemas for the Forge model formats. The version is the model format (`_forge_version` in a file), not the version of Forge.

| File | Format |
| --- | --- |
| `forge-v5.1.0.schema.json` | 5.1.0, the current format (ADR-051): 5.0.0 plus plain-number inputs (`rate: 0.05`) and formula scalars written as `{formula: "=..."}` without `value: null`. `examples/` use it. |
| `forge-v5.0.0.schema.json` | 5.0.0: tables, row formulas, includes and the analysis sections (`monte_carlo`, `scenarios`, `decision_tree`, `tornado` and the others). `test-data/` uses it. |
| `forge-v1.0.0.schema.json` | 1.0.0: scalars only. |

Forge embeds these files: `forge schema v5.1` prints the one of the Forge you have installed, and every model is checked against its schema before it is calculated. So a mixed array (`[100, "Q2", 150]`) or a formula without its `=` (`total: "SUM(revenue)"`) is an error from `forge validate` and `forge calculate`. The 5.1.0 schema reads each top-level mapping the way Forge does (one scalar, a table or a group of scalars, ADR-051), so an error is reported under the one reading that applies, with its key path: `'loan.payment.formla': there is no key 'formla' here. Did you mean 'formula'?` The schema checks the shape of a file; whether a formula's names exist and its stored results are current is what `forge validate` checks after that.

## In an editor

Any editor that runs [yaml-language-server](https://github.com/redhat-developer/yaml-language-server) (VS Code with Red Hat's YAML extension, Zed, Neovim, Helix) can use the schema to complete keys and to mark mistakes as you type. The schema on the main branch:

```text
https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge-v5.1.0.schema.json
```

**In the file**, as its first line (works in every such editor):

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge-v5.1.0.schema.json
_forge_version: "5.1.0"
```

**VS Code**: the extension in [`editors/vscode`](../editors/vscode/README.md) applies the schema to `*.forge.yaml` files and highlights formulas. For other file names, map them in `settings.json`:

```json
"yaml.schemas": {
  "https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge-v5.1.0.schema.json": "models/**/*.yaml"
}
```

**Zed** has YAML and yaml-language-server built in (there is no Zed extension: one could highlight formulas only in a language of its own, which would lose Zed's YAML language server). In Zed's `settings.json`:

```json
"lsp": {
  "yaml-language-server": {
    "settings": {
      "yaml": {
        "schemas": {
          "https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge-v5.1.0.schema.json": "models/**/*.yaml"
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
cargo test                                                    # every example still passes (tests/demo_examples.rs)
cp schema/forge-v5.1.0.schema.json editors/vscode/schemas/    # the extension's copy; its test checks they match
```
