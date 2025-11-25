# Forge Extension for Zed

Language support for Forge YAML formula files in Zed.

## Features

- **Syntax Highlighting** - Highlighting for Forge YAML files
- **LSP Integration** - Connect to forge-lsp for validation, completion, hover
- **Rust-Native** - Fast, memory-efficient (Forge is built in Rust!)

## Installation

### From Zed Extensions

1. Open Zed
2. Press `Cmd+Shift+P` (macOS) or `Ctrl+Shift+P` (Linux)
3. Search for "Extensions"
4. Search for "Forge" and install

### Manual Installation

1. Clone this repo to `~/.config/zed/extensions/forge`
2. Restart Zed

## Requirements

- Install the Forge CLI: `cargo install royalbit-forge`
- The `forge-lsp` binary must be in your PATH

## Why Zed?

Zed is the #2 most popular AI IDE (State of AI 2025) with:
- 150K active developers per month
- 9% of Rust developers use it
- Used by Anthropic, Linear, Ramp, Shopify
- Opens 100K line repos in 0.8s (VSCode: 6s)

Forge is Rust-native, making it a perfect fit for the Zed community!

## Configuration

Add to your Zed settings:

```json
{
  "lsp": {
    "forge-lsp": {
      "binary": {
        "path": "forge-lsp"
      }
    }
  }
}
```

## License

MIT - RoyalBit Inc.
