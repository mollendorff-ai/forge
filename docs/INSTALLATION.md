# Forge Installation Guide

Complete guide for installing and configuring all Forge components (v3.1.0).

---

## Table of Contents

- [Quick Start](#quick-start)
- [CLI Installation](#cli-installation)
- [HTTP API Server](#http-api-server-forge-server)
- [MCP Server (AI Agents)](#mcp-server-forge-mcp)
- [LSP Server (Editors)](#lsp-server-forge-lsp)
- [VSCode Extension](#vscode-extension)
- [Zed Extension](#zed-extension)
- [GitHub Action](#github-action)
- [Troubleshooting](#troubleshooting)

---

## Quick Start

```bash
# Install from crates.io (installs ALL binaries)
cargo install royalbit-forge

# Verify installation
forge --version          # CLI
forge-server --version   # HTTP API
forge-mcp --version      # MCP for AI agents
forge-lsp --version      # LSP for editors

# Basic usage
forge validate model.yaml
forge calculate model.yaml
```

---

## CLI Installation

### From crates.io (Recommended)

```bash
cargo install royalbit-forge
```

This installs **all binaries**:
| Binary | Purpose |
|--------|---------|
| `forge` | Main CLI tool |
| `forge-server` | HTTP REST API server |
| `forge-mcp` | MCP server for Claude/ChatGPT |
| `forge-lsp` | Language server for editors |

### From Source

```bash
git clone https://github.com/royalbit/forge.git
cd forge
cargo build --release

# Binaries are in target/release/
ls target/release/forge*
```

### Pre-built Binary (Linux)

```bash
curl -L https://github.com/royalbit/forge/releases/latest/download/forge-linux-x86_64.tar.gz | tar xz
sudo mv forge* /usr/local/bin/
```

### Verify PATH

```bash
# Add to ~/.bashrc or ~/.zshrc
export PATH="$HOME/.cargo/bin:$PATH"

# Reload
source ~/.bashrc
```

---

## HTTP API Server (`forge-server`)

Enterprise-grade REST API for integrations with web apps, CI/CD, and automation.

### Starting the Server

```bash
# Default: http://127.0.0.1:8080
forge-server

# Custom host/port
forge-server --host 0.0.0.0 --port 3000

# Using environment variables
FORGE_HOST=0.0.0.0 FORGE_PORT=3000 forge-server
```

### Configuration

| Option | Env Variable | Default | Description |
|--------|--------------|---------|-------------|
| `-H, --host` | `FORGE_HOST` | `127.0.0.1` | Bind address |
| `-p, --port` | `FORGE_PORT` | `8080` | Listen port |

### API Endpoints

```
GET  /              API documentation
GET  /health        Health check
GET  /version       Server version

POST /api/v1/validate    Validate YAML model
POST /api/v1/calculate   Calculate formulas
POST /api/v1/audit       Audit variable dependencies
POST /api/v1/export      Export YAML to Excel
POST /api/v1/import      Import Excel to YAML
```

### Example Usage

```bash
# Health check
curl http://localhost:8080/health

# Validate a model
curl -X POST http://localhost:8080/api/v1/validate \
  -H "Content-Type: application/json" \
  -d '{"file_path": "/path/to/model.yaml"}'

# Calculate with dry-run
curl -X POST http://localhost:8080/api/v1/calculate \
  -H "Content-Type: application/json" \
  -d '{"file_path": "/path/to/model.yaml", "dry_run": true}'

# Export to Excel
curl -X POST http://localhost:8080/api/v1/export \
  -H "Content-Type: application/json" \
  -d '{"yaml_path": "model.yaml", "excel_path": "output.xlsx"}'
```

### Response Format

All responses use a consistent JSON format:

```json
{
  "success": true,
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "data": {
    "valid": true,
    "file_path": "model.yaml",
    "message": "Validation successful"
  }
}
```

### Running as a Service (systemd)

Create `/etc/systemd/system/forge-server.service`:

```ini
[Unit]
Description=Forge API Server
After=network.target

[Service]
Type=simple
User=forge
Environment=FORGE_HOST=0.0.0.0
Environment=FORGE_PORT=8080
ExecStart=/usr/local/bin/forge-server
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now forge-server
sudo systemctl status forge-server
```

### Docker

```dockerfile
FROM rust:1.75-slim AS builder
RUN cargo install royalbit-forge

FROM debian:bookworm-slim
COPY --from=builder /usr/local/cargo/bin/forge-server /usr/local/bin/
EXPOSE 8080
CMD ["forge-server", "--host", "0.0.0.0"]
```

```bash
docker build -t forge-server .
docker run -p 8080:8080 forge-server
```

---

## MCP Server (`forge-mcp`)

Model Context Protocol server for AI agents (Claude, ChatGPT, etc.).

### What is MCP?

MCP (Model Context Protocol) is the standard for AI tool integration, adopted by Anthropic, OpenAI, and Google DeepMind. It allows AI assistants to use external tools.

### Setup for Claude Desktop

**Step 1: Install Forge**

```bash
cargo install royalbit-forge
```

**Step 2: Locate Claude Desktop Config**

| OS | Config Path |
|----|-------------|
| macOS | `~/Library/Application Support/Claude/claude_desktop_config.json` |
| Windows | `%APPDATA%\Claude\claude_desktop_config.json` |
| Linux | `~/.config/Claude/claude_desktop_config.json` |

**Step 3: Add Forge to Config**

Edit the config file:

```json
{
  "mcpServers": {
    "forge": {
      "command": "forge-mcp"
    }
  }
}
```

If `forge-mcp` is not in PATH, use full path:

```json
{
  "mcpServers": {
    "forge": {
      "command": "/Users/yourname/.cargo/bin/forge-mcp"
    }
  }
}
```

**Step 4: Restart Claude Desktop**

Completely quit and reopen Claude Desktop.

**Step 5: Verify**

Ask Claude: *"What tools do you have available?"*

Claude should list:
- `forge_validate`
- `forge_calculate`
- `forge_audit`
- `forge_export`
- `forge_import`

### Available Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `forge_validate` | Validate YAML model | `file_path` |
| `forge_calculate` | Calculate formulas | `file_path`, `dry_run` (optional) |
| `forge_audit` | Audit variable dependencies | `file_path`, `variable` |
| `forge_export` | Export YAML to Excel | `yaml_path`, `excel_path` |
| `forge_import` | Import Excel to YAML | `excel_path`, `yaml_path` |

### Example Conversations

> **You:** Validate my budget at ~/Documents/budget.yaml
>
> **Claude:** *[uses forge_validate]* Your budget model is valid. 12 formulas checked, no errors found.

> **You:** What does total_revenue depend on?
>
> **Claude:** *[uses forge_audit]* The total_revenue variable depends on: product_sales, service_revenue, and other_income.

---

## LSP Server (`forge-lsp`)

Language Server Protocol for real-time editor integration.

### Features

- Real-time validation as you type
- Error highlighting with descriptive messages
- Hover information for variables
- Cross-reference detection

### How It Works

The LSP server communicates over stdin/stdout using the Language Server Protocol:

```bash
forge-lsp  # Starts LSP server (stdin/stdout)
```

### Generic Editor Setup

Configure your editor's LSP client:

| Setting | Value |
|---------|-------|
| Command | `forge-lsp` |
| File types | `yaml`, `yml` |
| Root pattern | `*.yaml` |

---

## VSCode Extension

### Install from Marketplace

1. Open VSCode
2. Press `Ctrl+Shift+X` (Extensions)
3. Search: **"Forge YAML"**
4. Click **Install**

### Manual Installation

```bash
# Clone and build
git clone https://github.com/royalbit/forge-vscode.git
cd forge-vscode
npm install
npm run package

# Install
code --install-extension forge-yaml-*.vsix
```

### Configuration

Add to VSCode `settings.json`:

```json
{
  "forge.enable": true,
  "forge.validateOnSave": true,
  "forge.lspPath": "forge-lsp"
}
```

### Features

- Syntax highlighting for formulas (`=SUM(...)`)
- Real-time error diagnostics
- Hover tooltips with variable info
- Error squiggles with fix suggestions

---

## Zed Extension

### Install from Extensions

1. Open Zed
2. Press `Cmd+Shift+X` (Extensions)
3. Search: **"forge"**
4. Click **Install**

### Manual Installation

```bash
git clone https://github.com/royalbit/forge-zed.git
cp -r forge-zed ~/.config/zed/extensions/forge
```

### Configuration

Add to Zed `settings.json`:

```json
{
  "lsp": {
    "forge-lsp": {
      "binary": {
        "path": "forge-lsp"
      }
    }
  },
  "languages": {
    "YAML": {
      "language_servers": ["forge-lsp"]
    }
  }
}
```

---

## GitHub Action

### Simple Workflow

```yaml
name: Validate Forge Models
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Forge
        run: cargo install royalbit-forge

      - name: Validate Models
        run: forge validate models/*.yaml
```

### With Caching

```yaml
name: Validate Forge Models
on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Cache Forge
        uses: actions/cache@v4
        with:
          path: ~/.cargo/bin/forge*
          key: forge-${{ runner.os }}

      - name: Install Forge
        run: command -v forge || cargo install royalbit-forge

      - name: Validate
        run: |
          for f in models/*.yaml; do
            echo "Validating $f..."
            forge validate "$f"
          done
```

### Reusable Workflow

```yaml
name: Validate
on: [push, pull_request]

jobs:
  validate:
    uses: royalbit/forge/.github/workflows/forge-validate.yml@main
    with:
      files: "models/**/*.yaml"
```

---

## Troubleshooting

### "command not found: forge"

```bash
# Check if installed
ls ~/.cargo/bin/forge*

# Add to PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify
which forge
```

### MCP Server Not Working

1. **Check binary exists:**
   ```bash
   which forge-mcp
   forge-mcp --version
   ```

2. **Test manually:**
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | forge-mcp
   ```

3. **Check Claude Desktop logs:**
   - macOS: `~/Library/Logs/Claude/`

4. **Use full path in config:**
   ```json
   {"mcpServers": {"forge": {"command": "/full/path/to/forge-mcp"}}}
   ```

### API Server Connection Refused

```bash
# Check if running
curl http://localhost:8080/health

# Check port in use
lsof -i :8080

# Try different port
forge-server --port 3000
```

### LSP Not Connecting

```bash
# Test LSP binary
forge-lsp --version

# Check editor's LSP logs
# VSCode: Output > Forge Language Server
# Zed: View > Developer Tools
```

### Build Errors

```bash
# Update Rust
rustup update stable

# Clean rebuild
cargo clean
cargo build --release
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Forge Ecosystem                          │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   forge     │  │forge-server │  │     forge-mcp       │ │
│  │   (CLI)     │  │ (HTTP API)  │  │  (AI Integration)   │ │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         │                │                    │             │
│         └────────────────┼────────────────────┘             │
│                          │                                  │
│                    ┌─────▼─────┐                            │
│                    │  Forge    │                            │
│                    │   Core    │                            │
│                    └─────┬─────┘                            │
│                          │                                  │
│  ┌───────────────────────┼───────────────────────────────┐ │
│  │                       │                               │ │
│  │  ┌─────────┐  ┌───────▼───────┐  ┌─────────────────┐ │ │
│  │  │ Parser  │  │  Calculator   │  │ Excel Bridge    │ │ │
│  │  │ (YAML)  │  │  (57+ funcs)  │  │ (Import/Export) │ │ │
│  │  └─────────┘  └───────────────┘  └─────────────────┘ │ │
│  └───────────────────────────────────────────────────────┘ │
│                                                             │
│  ┌─────────────┐  ┌─────────────────────────────────────┐  │
│  │  forge-lsp  │  │        Editor Extensions            │  │
│  │(Lang Server)│──│  VSCode  │  Zed  │  Others...      │  │
│  └─────────────┘  └─────────────────────────────────────┘  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Getting Help

- **Documentation:** [README.md](../README.md)
- **Features:** [FEATURES.md](FEATURES.md)
- **Issues:** [GitHub Issues](https://github.com/royalbit/forge/issues)
- **Architecture:** [docs/architecture/](architecture/)

---

*Built by Claude Opus 4.5 (Principal Autonomous AI) using the Asimov Protocol.*
