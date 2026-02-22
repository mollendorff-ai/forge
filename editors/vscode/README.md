# Forge YAML Extension for VSCode

Language support for Forge YAML formula files.

## Features

- **Syntax Highlighting** - Beautiful highlighting for Forge YAML files
- **Real-time Validation** - See formula errors as you type
- **Autocomplete** - Complete variable names and 50+ Excel functions
- **Hover Information** - See calculated values and formula documentation
- **Go to Definition** - Jump to variable definitions
- **Commands** - Validate, calculate, export, and audit

## Requirements

- Install the Forge CLI: `cargo install mollendorff-forge`
- The `forge-lsp` binary must be in your PATH

## Commands

- `Forge: Validate Current File` - Validate formulas
- `Forge: Calculate All Formulas` - Calculate with dry-run
- `Forge: Export to Excel` - Export to .xlsx
- `Forge: Show Audit Trail` - Show dependency chain

## Supported Functions (50+)

### Aggregation
`SUM`, `AVERAGE`, `COUNT`, `MAX`, `MIN`, `PRODUCT`

### Conditional
`SUMIF`, `COUNTIF`, `AVERAGEIF`, `SUMIFS`, `COUNTIFS`, `AVERAGEIFS`, `MAXIFS`, `MINIFS`

### Logical
`IF`, `AND`, `OR`, `NOT`, `IFERROR`

### Math
`ROUND`, `ROUNDUP`, `ROUNDDOWN`, `CEILING`, `FLOOR`, `SQRT`, `POWER`, `MOD`, `ABS`

### Text
`CONCAT`, `UPPER`, `LOWER`, `TRIM`, `LEN`, `MID`

### Date
`TODAY`, `DATE`, `YEAR`, `MONTH`, `DAY`

### Lookup
`MATCH`, `INDEX`, `XLOOKUP`, `VLOOKUP`

## Configuration

- `forge.lspPath` - Path to forge-lsp binary (default: "forge-lsp")
- `forge.validateOnSave` - Validate on save (default: true)
- `forge.validateOnType` - Validate while typing (default: true)
- `forge.showCalculatedValues` - Show values on hover (default: true)

## Installation

1. Install from the VS Code Marketplace (search for "Forge YAML")
2. Or install the VSIX file directly

## Building from Source

```bash
cd editors/vscode
npm install
npm run compile
```

## License

MIT OR Apache-2.0 - See [LICENSE-MIT](../../LICENSE-MIT) or [LICENSE-APACHE](../../LICENSE-APACHE)

Copyright (c) 2025-2026 Möllendorff AI
