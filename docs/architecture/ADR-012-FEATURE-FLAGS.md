# ADR-012: Feature Flags for Demo/Enterprise Binary Split

**Status:** Implemented (v5.15.0)
**Date:** 2025-12-08
**Updated:** 2025-12-08 (Binary split complete: forge-demo vs forge)
**Author:** Rex (CEO) + Claude Opus 4.5 (Principal Autonomous AI)
**Type:** Architecture Decision Record (ADR)

---

## Context

Forge v5.14.0 achieved full function parity. The next step is creating a demo binary that potential customers can evaluate without access to the full enterprise feature set.

### Business Requirements

1. **Demo Binary**: Free download, THE HOOK for big financial consultants
2. **Enterprise Binary**: Licensed customers get full functionality
3. **Single Codebase**: Maintain one codebase, not two forks
4. **Clear Separation**: Demo has NO financial functions - that's the enterprise value prop

## Decision

**Rust Feature Flags with Function-Level Gating and Binary Split**

```toml
# Cargo.toml
[features]
default = []   # Demo build (minimal)
full = []      # Enterprise build (everything)

# Demo binary (47 functions)
[[bin]]
name = "forge-demo"
path = "src/main.rs"

# Enterprise binary (160 functions)
[[bin]]
name = "forge"
path = "src/main.rs"
required-features = ["full"]

# Enterprise MCP server
[[bin]]
name = "forge-mcp"
required-features = ["full"]

# Enterprise API server
[[bin]]
name = "forge-server"
required-features = ["full"]
```

### Build Commands

```bash
# Demo binary only (47 functions) - installs to ~/bin
make install-forge-demo

# Enterprise binary (160 functions) - installs to ~/bin
make install-forge

# Cross-platform builds (5 platforms → dist/)
make cross-forge-demo

# Publish to GitHub releases (auto-detects version from Cargo.toml)
make publish-demo
```

**Raw cargo commands (if needed):**
```bash
cargo build --release --bin forge-demo          # Demo
cargo build --release --features full --bin forge  # Enterprise
```

### Distribution

- **Demo**: Published to [royalbit.ca/forge](https://royalbit.ca/forge) for download
- **Enterprise**: Licensed distribution only

## Implementation Summary

### Demo Build (36 Functions)

| Category | Count | Functions |
|----------|-------|-----------|
| **Math** | 9 | ABS, SQRT, ROUND, ROUNDUP, ROUNDDOWN, FLOOR, CEILING, POWER, MOD |
| **Aggregation** | 5 | SUM, AVERAGE, MIN, MAX, COUNT |
| **Logical** | 5 | IF, AND, OR, NOT, IFERROR |
| **Text** | 8 | CONCAT, UPPER, LOWER, TRIM, LEN, LEFT, RIGHT, MID |
| **Date** | 6 | TODAY, DATE, YEAR, MONTH, DAY, DATEDIF |
| **Lookup** | 3 | INDEX, MATCH, CHOOSE |

**Demo includes:**
- `forge` CLI binary only
- Core formula evaluation
- Excel import/export
- YAML validation

**Demo excludes:**
- ALL financial functions (NPV, IRR, PMT, etc.)
- ALL statistical functions (MEDIAN, STDEV, VAR, etc.)
- API server (forge-server)
- MCP server (forge-mcp)
- Forge-native functions (VARIANCE_PCT, BREAKEVEN, etc.)

### Enterprise Build (134 Functions + API + MCP)

| Category | Count | Notes |
|----------|-------|-------|
| **Financial** | 13 | NPV, IRR, MIRR, XNPV, XIRR, PMT, PV, FV, NPER, RATE, DB, DDB, SLN |
| **Statistical** | 16 | MEDIAN, STDEV, VAR, PERCENTILE, QUARTILE, CORREL, etc. |
| **Math** | 19 | Full suite including EXP, LN, LOG, RAND, PRODUCT, etc. |
| **Aggregation** | 9 | Full suite including COUNTA, MEDIAN, COUNTBLANK, etc. |
| **Logical** | 10 | Full suite including IFS, SWITCH, XOR |
| **Text** | 15 | Full suite including SUBSTITUTE, FIND, TEXT, VALUE |
| **Date** | 18 | Full suite including NETWORKDAYS, WORKDAY, YEARFRAC |
| **Lookup** | 13 | Full suite including VLOOKUP, XLOOKUP, INDIRECT, OFFSET |
| **Conditional** | 8 | SUMIF, COUNTIF, AVERAGEIF, SUMIFS, COUNTIFS, etc. |
| **Array** | 4 | UNIQUE, SORT, FILTER, SEQUENCE |
| **Advanced** | 3 | LET, LAMBDA, SWITCH |
| **Forge-Native** | 6 | VARIANCE_PCT, BREAKEVEN_UNITS, BREAKEVEN_REVENUE, SCENARIO, etc. |

**Enterprise includes:**
- `forge` CLI binary
- `forge-mcp` - MCP server for AI integration
- `forge-server` - REST API server
- All 159 functions
- Full financial modeling power

### Gating Pattern

```rust
// lib.rs - Module-level gating
#[cfg(feature = "full")]
pub mod api;
#[cfg(feature = "full")]
pub mod mcp;

// evaluator/mod.rs - Function-level gating
#[cfg(feature = "full")]
{
    if let Some(result) = financial::try_evaluate(&name, args, ctx)? {
        return Ok(result);
    }
    // ... other enterprise modules
}

// Individual function gating
#[cfg(feature = "full")]
"NPV" | "IRR" | "MIRR" => { ... }
```

## Consequences

### Positive

1. **Zero Runtime Overhead**: Compile-time gating, no performance impact
2. **Single Codebase**: No fork divergence, one CI/CD pipeline
3. **Clear Value Proposition**: Demo hooks them, enterprise closes the deal
4. **Rust Idiomatic**: Standard Cargo feature pattern

### Negative

1. **Annotation Burden**: Must mark enterprise code with `#[cfg(feature = "full")]`
2. **Test Complexity**: CI must test both builds

### Mitigations

1. **Module Grouping**: Enterprise functions in dedicated modules (financial.rs, statistical.rs)
2. **Test Gating**: Tests use same `#[cfg(feature = "full")]` pattern

## Verification

```bash
# Demo: 36 functions, 1 binary
cargo build && ./target/debug/forge functions | grep -E "^  [A-Z]" | wc -l
# Output: 36

# Enterprise: 159 functions, 3 binaries
cargo build --features full && ls target/debug/forge*
# Output: forge, forge-mcp, forge-server
```

## References

- [Rust Features Documentation](https://doc.rust-lang.org/cargo/reference/features.html)
- ADR-011: Source Code Closure (business context)
