# ADR-003: Editor Extension Architecture

**Status:** ~~Accepted~~ **SUPERSEDED by [ADR-005](ADR-005-NO-LSP.md)**
**Date:** 2025-11-25
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

> **SUPERSEDED:** This ADR is obsolete. LSP support was removed in v5.0.0.
> See [ADR-005: No LSP](ADR-005-NO-LSP.md) for the rationale.

---

## Context (Historical)

Forge v3.1.0 adds editor extensions for VSCode and Zed. The question arose: how should these extensions integrate with the Forge LSP server?

### Options Considered

| Option | Description | Pros | Cons |
|--------|-------------|------|------|
| **A: Native WASM (Zed)** | Compile Rust extension to WASM | Fast startup, no IPC | Zed-specific |
| **B: Node.js bridge (VSCode)** | TypeScript extension spawns forge-lsp | Standard VSCode pattern | Extra process |
| **C: Embedded LSP** | Bundle LSP into extension | Single binary | Larger package |
| **D: Remote LSP** | Connect to running forge-lsp | Shared server | Complex setup |

## Decision

**Hybrid approach: Native WASM for Zed, Node.js bridge for VSCode.**

### Zed Extension (Native WASM)

```
editors/zed/
├── extension.toml      # Manifest with grammar reference
├── Cargo.toml          # Rust project targeting wasm32-wasip1
├── src/lib.rs          # Extension entry point
└── languages/forge/
    ├── config.toml     # Language configuration
    ├── highlights.scm  # 60+ function highlighting
    ├── brackets.scm    # Bracket matching
    ├── indents.scm     # Auto-indentation
    └── outline.scm     # Code outline
```

The Zed extension:
1. Compiles to WASM (~2.1MB UPX compressed)
2. Uses Zed's built-in tree-sitter YAML grammar
3. Spawns `forge-lsp` as external process via `worktree.which()`
4. Requires `forge-lsp` in PATH

### VSCode Extension (Node.js Bridge)

```
editors/vscode/
├── package.json        # Extension manifest
├── src/extension.ts    # TypeScript entry point
├── syntaxes/           # TextMate grammar
└── language-configuration.json
```

The VSCode extension:
1. Uses vscode-languageclient to spawn forge-lsp
2. Provides syntax highlighting via TextMate grammar
3. Adds commands: validate, calculate, export, audit
4. Configurable LSP path via settings

## Rationale

### Why Native WASM for Zed?

1. **Zed is Rust-native** - Extensions compile to WASM, perfect fit for Forge
2. **Fast startup** - No Node.js runtime, no IPC serialization
3. **Type safety** - Rust extension uses Zed's typed API
4. **Future-proof** - Zed's extension model is stabilizing

### Why Node.js Bridge for VSCode?

1. **Standard pattern** - All major LSP extensions use this approach
2. **Ecosystem** - vscode-languageclient handles protocol details
3. **Debugging** - VSCode's extension host provides excellent tooling
4. **User base** - 51% market share, can't ignore

### Why Not Embedded LSP?

1. **Duplication** - forge-lsp already exists as binary
2. **Updates** - Users can update forge-lsp independently
3. **Size** - Would bloat extension packages
4. **Maintenance** - Two codebases to maintain

### Why Not Remote LSP?

1. **Complexity** - Users don't want to manage server processes
2. **Latency** - Local subprocess is faster than TCP/WebSocket
3. **Security** - No network exposure needed for local files

## Consequences

### Positive

- **Consistent behavior** - Both extensions use same forge-lsp
- **Minimal maintenance** - Extensions are thin wrappers
- **User choice** - Works with existing forge installation
- **Performance** - Zed gets native speed, VSCode gets standard behavior

### Negative

- **Dependency** - Users must install forge-lsp separately
- **Two codebases** - Rust for Zed, TypeScript for VSCode
- **Version sync** - Extension version should match forge-lsp version

## Implementation Notes

### Zed Extension Installation

```bash
# forge-lsp must be in PATH
cargo install royalbit-forge

# Extension published to zed-industries/extensions
# Users install via Zed's extension browser
```

### VSCode Extension Installation

```bash
# Option 1: From marketplace (future)
code --install-extension royalbit.forge-yaml

# Option 2: From .vsix file
code --install-extension forge-yaml-3.1.0.vsix
```

### LSP Features (Both Editors)

| Feature | Status |
|---------|--------|
| Diagnostics | Real-time formula validation |
| Completion | 60+ functions + variables |
| Hover | Calculated values, types |
| Go to Definition | Variable references |
| Code Actions | Quick fixes (planned) |

---

## Related

- [ADR-001: HTTP REST Over gRPC](ADR-001-NO-GRPC.md)
- [ADR-002: Variance YAML Only](ADR-002-VARIANCE-YAML-ONLY.md)
- [06-CLI-ARCHITECTURE](06-CLI-ARCHITECTURE.md)

---

**Previous:** [ADR-002](ADR-002-VARIANCE-YAML-ONLY.md)
