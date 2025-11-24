# 🔧 Installation Guide

## Quick Install (Recommended)

### From crates.io:

```bash
cargo install royalbit-forge
```

### Pre-built Binary (Linux):

```bash
curl -L https://github.com/royalbit/forge/releases/latest/download/forge-linux-x86_64 -o forge
chmod +x forge
sudo mv forge /usr/local/bin/forge
```

## Advanced Installation

### With Makefile:

```bash
git clone https://github.com/royalbit/forge
cd forge

# System-wide (requires sudo)

make install

# User-only (no sudo)

make install-user

# Uninstall

make uninstall
```

### From Source:

```bash
git clone https://github.com/royalbit/forge
cd forge
cargo install --path .
```

### Optimized Static Build (440KB):

```bash
git clone https://github.com/royalbit/forge
cd forge

# Build with musl

make build-static

# Compress with UPX (optional)

make build-compressed
```

Result: 440KB executable with zero dependencies

## Verification

```bash
forge --version

# royalbit-forge 1.1.0

forge --help

# Forge - YAML Formula Calculator


# ...

```

## IDE Integration

Add to your YAML files:

```yaml

# yaml-language-server: $schema=https://raw.githubusercontent.com/royalbit/forge/main/schema/forge-v1.schema.json

```

Supported: VS Code, IntelliJ, any YAML language server

## Troubleshooting

### "forge: command not found"

Add to PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Build errors

Update Rust:

```bash
rustup update stable
```

For more help: https://github.com/royalbit/forge/issues
