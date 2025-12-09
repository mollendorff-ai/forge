# ADR-012: Feature Flags for Demo/Enterprise Binary Split

**Status:** Accepted
**Date:** 2025-12-08
**Author:** Rex (CEO) + Claude Opus 4.5 (Principal Autonomous AI)
**Type:** Architecture Decision Record (ADR)

---

## Context

Forge v5.13.0 achieved full function parity with 149 evaluator functions. The next step is creating a demo binary that potential customers can evaluate without access to the full enterprise feature set.

### Business Requirements

1. **Demo Binary**: Free download, limited functionality (~40 functions)
2. **Enterprise Binary**: Licensed customers get full functionality (149 functions)
3. **Single Codebase**: Maintain one codebase, not two forks
4. **Clear Separation**: Demo should be genuinely useful but missing "enterprise magic"

### Options Considered

| Option | Approach | Pros | Cons |
|--------|----------|------|------|
| A | Rust feature flags | Single codebase, compile-time, zero runtime overhead | Requires careful annotation |
| B | Runtime license check | Single binary | Runtime overhead, can be cracked |
| C | Separate codebases | Complete control | Maintenance nightmare, divergence |
| D | Plugin architecture | Flexible | Over-engineered for this use case |

## Decision

**Option A: Rust Feature Flags**

```toml
# Cargo.toml
[features]
default = []   # Demo build (minimal)
full = []      # Enterprise build (everything)
```

### Build Commands

```bash
# Demo binary (~40 functions, v1.0.0 schema only)
cargo build --release

# Enterprise binary (149 functions, all schemas, API server)
cargo build --release --features full
```

### Gating Strategy

#### 1. Enterprise Functions (Gated)

```rust
#[cfg(feature = "full")]
"VARIANCE" | "VARIANCE_PCT" | "VARIANCE_STATUS" => { ... }

#[cfg(feature = "full")]
"BREAKEVEN" | "BREAKEVEN_UNITS" | "BREAKEVEN_REVENUE" => { ... }

#[cfg(feature = "full")]
"LET" | "LAMBDA" | "SWITCH" => { ... }

#[cfg(feature = "full")]
"VAR" | "STDEV" | "CORREL" | "PERCENTILE" | "QUARTILE" => { ... }
```

#### 2. Schema Versions (Gated)

```rust
#[cfg(feature = "full")]
mod schema_v4;

#[cfg(feature = "full")]
mod schema_v5;
```

#### 3. API Server (Gated)

```rust
#[cfg(feature = "full")]
pub mod api;
```

### Demo Functions (~40)

The demo includes enough to be genuinely useful:

| Category | Functions |
|----------|-----------|
| **Math** | SUM, AVERAGE, MIN, MAX, COUNT, COUNTA, ROUND, ABS, SQRT, POWER, MOD, EXP, LN |
| **Financial** | PMT, PV, FV, NPV, IRR, NPER, RATE |
| **Logical** | IF, AND, OR, NOT, IFERROR |
| **Text** | CONCAT, LEFT, RIGHT, MID, LEN, UPPER, LOWER, TRIM |
| **Date** | TODAY, DATE, YEAR, MONTH, DAY, DATEDIF, EDATE, EOMONTH |
| **Lookup** | INDEX, MATCH, CHOOSE |

### Enterprise-Only Functions (~109)

| Category | Functions |
|----------|-----------|
| **Forge-Specific** | VARIANCE, VARIANCE_PCT, VARIANCE_STATUS, BREAKEVEN, BREAKEVEN_UNITS, BREAKEVEN_REVENUE, SCENARIO |
| **Advanced** | LET, LAMBDA, SWITCH |
| **Statistical** | VAR, VARP, STDEV, STDEVP, CORREL, PERCENTILE, QUARTILE, LARGE, SMALL, RANK |
| **Financial (Adv)** | MIRR, DB, DDB, SLN, XNPV, XIRR |
| **Array** | UNIQUE, SORT, FILTER, SEQUENCE, RANDARRAY |
| **Lookup (Adv)** | VLOOKUP, HLOOKUP, OFFSET, ADDRESS |
| **Trig** | SIN, COS, TAN, ASIN, ACOS, ATAN, etc. |
| **Info** | ISBLANK, ISERROR, ISNUMBER, ISTEXT, TYPE, etc. |
| **Conditional** | SUMIF, COUNTIF, AVERAGEIF |

## Consequences

### Positive

1. **Zero Runtime Overhead**: Feature flags are compile-time, no performance impact
2. **Single Codebase**: No fork divergence, one CI/CD pipeline
3. **Clear Value Proposition**: Demo is useful, enterprise is powerful
4. **Rust Idiomatic**: Standard Cargo feature pattern

### Negative

1. **Annotation Burden**: Must mark every enterprise function with `#[cfg(feature = "full")]`
2. **Test Complexity**: Need to test both demo and full builds
3. **Documentation Split**: Must maintain two function lists

### Mitigations

1. **Annotation**: Group enterprise functions in dedicated modules where possible
2. **Testing**: CI runs `cargo test` AND `cargo test --features full`
3. **Documentation**: Auto-generate function lists from code annotations

## Implementation

1. Add `full` feature to `Cargo.toml`
2. Annotate enterprise functions in evaluator modules
3. Gate `api` module behind `full` feature
4. Gate v4/v5 schema support behind `full` feature
5. Update README with build instructions
6. Cross-compile demo binaries for distribution

## References

- [Rust Features Documentation](https://doc.rust-lang.org/cargo/reference/features.html)
- [Conditional Compilation](https://doc.rust-lang.org/reference/conditional-compilation.html)
- ADR-011: Source Code Closure (business context)
