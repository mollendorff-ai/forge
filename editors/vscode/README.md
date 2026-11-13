# Forge YAML for VS Code

Highlighting for the formulas in [Forge](https://github.com/mollendorff-ai/forge) model files, and the Forge schema for the files themselves.

- **Formulas**: in any YAML file, a value that starts with `=` (`"=SUM(months.revenue)"`, `'=MC.Normal(1, 2)'`, `price: =cost * 1.3`) is highlighted as a formula: functions, names, values of included files (`@costs.flour_per_kg`), numbers, operators and text. The file stays a YAML file, with everything VS Code's YAML support does for it.
- **Schema**: files named `*.forge.yaml` or `*.forge.yml` are checked against the schema of the model format they name in `_forge_version` (5.1.0, 5.2.0 or 5.3.0), with completion of keys, by the [YAML extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml), which this extension depends on.

There is no language server: the extension does not calculate. Run `forge validate` and `forge calculate` in a terminal.

## The schema for any file name

Forge models are usually plain `.yaml` files. To check one against the schema, put this first line in it:

```yaml
# yaml-language-server: $schema=https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge.schema.json
```

or map a folder to it in your settings:

```json
"yaml.schemas": {
  "https://raw.githubusercontent.com/mollendorff-ai/forge/main/schema/forge.schema.json": "models/**/*.yaml"
}
```

`forge schema v5.2 > forge.schema.json` (or `v5.1`) writes the schema of one format from the Forge you have installed, to use as a local path instead.

## Install

The extension is not on the Marketplace. Build it and install the file:

```bash
cd editors/vscode
npm install
npx vsce package                          # writes forge-yaml-4.0.0.vsix
code --install-extension forge-yaml-4.0.0.vsix
```

## Tests

`npm test` tokenizes every YAML file of the repository with VS Code's own YAML grammar, with and without the formula highlighting, and checks that nothing outside a formula changes and that every formula is recognised. It then validates every example model, of either format, with yaml-language-server against the bundled schemas, and checks that a model with mistakes in each format gets its diagnostics on the right lines. CI runs both.
