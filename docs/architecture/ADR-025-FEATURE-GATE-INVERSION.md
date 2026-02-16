# ADR-025: Feature Gate Inversion

## Status

**Implemented** - v9.3.0

> **Note:** This ADR is historical. The demo/enterprise split was removed in v10.0.0-alpha.0. Forge is now a single MIT/Apache-2.0 licensed binary.

## Context

Forge previously had two binaries:
- `forge` - Full functionality
- `forge-demo` - Limited functionality

Previously, enterprise features were gated with `#[cfg(feature = "full")]`:

```rust
#[cfg(feature = "full")]
pub mod monte_carlo;
```

This meant:
- `cargo build` → demo (minimal)
- `cargo build --features full` → enterprise
- `cargo test` → tests demo subset only

## Problem

1. **Dev friction** - Most development is on enterprise features, requiring `--features full` everywhere
2. **Incomplete testing** - `cargo test` only tests demo, missing enterprise code
3. **Forgotten gates** - New code requires explicit `#[cfg(feature = "full")]` or it's excluded

## Decision

**Invert the feature gate logic.**

Change from:
```rust
#[cfg(feature = "full")]      // Include if full
#[cfg(not(feature = "full"))] // Include if not full (demo)
```

To:
```rust
#[cfg(not(feature = "demo"))] // Include unless demo
#[cfg(feature = "demo")]       // Include only for demo
```

### New Behavior

| Command | Result |
|---------|--------|
| `cargo build` | Enterprise (full) |
| `cargo test` | Tests everything |
| `cargo build --features demo` | Demo (restricted) |

### Makefile Updates

```makefile
install-forge:      cargo build --release
install-forge-demo: cargo build --release --features demo
```

## Rationale

1. **Enterprise is primary** - Demo is a marketing artifact, not the product
2. **Test everything by default** - `cargo test` should cover all code
3. **New code works immediately** - No need to remember feature gates
4. **Explicit restriction** - Demo restrictions are intentional, not default

## Implementation

### Cargo.toml

```toml
[features]
default = []
demo = []  # Restricts to demo functionality
full = []  # Backward compatibility alias (deprecated)
```

### Code Changes

All instances of:
- `#[cfg(feature = "full")]` → `#[cfg(not(feature = "demo"))]`
- `#[cfg(not(feature = "full"))]` → `#[cfg(feature = "demo")]`
- `#[cfg_attr(feature = "full", ...)]` → `#[cfg_attr(not(feature = "demo"), ...)]`
- `#[cfg_attr(not(feature = "full"), ...)]` → `#[cfg_attr(feature = "demo"), ...)]`

### Files Affected

- `src/lib.rs` - Module declarations
- `src/main.rs` - Command definitions and match arms
- `tests/*.rs` - Test gating
- `Makefile` - Build targets
- `Cargo.toml` - Feature definitions

## Consequences

### Positive
- Better developer experience (enterprise by default)
- Complete test coverage with `cargo test`
- New enterprise features work immediately
- Clearer mental model (demo = restricted, not full = unlocked)

### Negative
- Breaking change for anyone using `--features full` (use default now)
- Must remember `--features demo` when building public release
- Makefile protects against accidental demo misconfiguration

### Migration

Users of `--features full`:
```bash
# Before
cargo build --features full

# After
cargo build
```

## References

- Discussion: Feature gating strategy for dual-binary projects
- Version: v9.3.0
