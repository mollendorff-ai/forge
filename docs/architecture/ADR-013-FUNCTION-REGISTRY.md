# ADR-013: Function Registry - Single Source of Truth

**Status:** Proposed
**Date:** 2025-12-09
**Author:** Rex (CEO) + Claude Opus 4.5 (Principal Autonomous AI)
**Type:** Architecture Decision Record (ADR)

---

## Context

### The Problem

Function definitions are currently scattered across multiple locations:

| Location | Purpose | Example |
|----------|---------|---------|
| `evaluator/*.rs` | Implementation | `"SUM" => { ... }` |
| `cli/commands/functions.rs` | `forge functions` output | Hardcoded `FunctionCategory` structs |
| `README.md` | User-facing docs | "36 demo functions" |
| `docs/FUNCTIONS.md` | Detailed reference | Function tables |

### Consequences of Current Design

1. **Counts drift** - README said 134, actual implementation has 158
2. **Missing functions** - Info (13) and Trig (11) modules not in `forge functions` output
3. **Manual sync required** - Every new function requires 4+ file updates
4. **No validation** - No way to verify docs match implementation

### Discovery

During v5.16.0 release, we discovered:
- `forge functions` reported 134 enterprise functions
- Actual evaluator implements 158 functions
- Info and Trig modules completely missing from CLI output
- Demo binary displayed "Enterprise: 154 functions" (stale hardcoded string)

## Decision

**Implement a centralized Function Registry as the single source of truth.**

### Design

```rust
// src/functions/registry.rs

/// Function category for grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Math,
    Aggregation,
    Logical,
    Text,
    Date,
    Lookup,
    Financial,
    Statistical,
    Trigonometric,
    Information,
    Conditional,
    Array,
    Advanced,
    ForgeNative,
}

/// Function definition with all metadata
#[derive(Debug, Clone)]
pub struct FunctionDef {
    /// Function name (e.g., "SUM")
    pub name: &'static str,

    /// Category for grouping
    pub category: Category,

    /// Short description
    pub description: &'static str,

    /// Usage syntax (e.g., "=SUM(value1, value2, ...)")
    pub syntax: &'static str,

    /// Available in demo build (false = enterprise only)
    pub demo: bool,
}

/// All supported functions - THE SINGLE SOURCE OF TRUTH
pub static FUNCTIONS: &[FunctionDef] = &[
    // ══════════════════════════════════════════════════════════════
    // MATH (9 demo + 10 enterprise = 19 total)
    // ══════════════════════════════════════════════════════════════
    FunctionDef {
        name: "ABS",
        category: Category::Math,
        description: "Absolute value",
        syntax: "=ABS(value)",
        demo: true,
    },
    FunctionDef {
        name: "SQRT",
        category: Category::Math,
        description: "Square root",
        syntax: "=SQRT(value)",
        demo: true,
    },
    // ... all 158 functions
];

/// Get demo functions only
pub fn demo_functions() -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter().filter(|f| f.demo)
}

/// Get enterprise functions (all)
pub fn enterprise_functions() -> impl Iterator<Item = &'static FunctionDef> {
    FUNCTIONS.iter()
}

/// Count functions by build type
pub fn count_demo() -> usize {
    FUNCTIONS.iter().filter(|f| f.demo).count()
}

pub fn count_enterprise() -> usize {
    FUNCTIONS.len()
}
```

### Integration Points

#### 1. CLI `forge functions` Command

```rust
// cli/commands/functions.rs
use crate::functions::registry::{self, Category};

pub fn functions(json_output: bool) -> ForgeResult<()> {
    let funcs: Vec<_> = if cfg!(feature = "full") {
        registry::enterprise_functions().collect()
    } else {
        registry::demo_functions().collect()
    };

    // Group by category and display
    // Count is always accurate: funcs.len()
}
```

#### 2. Build Script for Documentation

```rust
// build.rs or src/bin/generate-docs.rs
use forge::functions::registry;

fn main() {
    let markdown = generate_functions_md();
    std::fs::write("docs/FUNCTIONS.md", markdown).unwrap();
}

fn generate_functions_md() -> String {
    let mut md = String::from("# Forge Functions\n\n");
    md.push_str(&format!("**Demo:** {} functions\n", registry::count_demo()));
    md.push_str(&format!("**Enterprise:** {} functions\n\n", registry::count_enterprise()));

    // Generate tables by category...
    md
}
```

#### 3. CI Validation

```yaml
# .github/workflows/ci.yml
- name: Validate function registry
  run: |
    # Count match arms in evaluator
    IMPL_COUNT=$(grep -rh '"[A-Z][A-Z0-9_.]*" =>' src/core/array_calculator/evaluator/*.rs | wc -l)
    # Count registry entries
    REG_COUNT=$(cargo run --bin count-functions)
    # They should match (within tolerance for aliases)
    if [ $IMPL_COUNT -lt $REG_COUNT ]; then
      echo "Registry has more functions than implementation!"
      exit 1
    fi
```

## Consequences

### Positive

1. **Single source of truth** - One place to add/modify functions
2. **Always accurate counts** - `FUNCTIONS.len()` can't be wrong
3. **Auto-generated docs** - No manual markdown updates
4. **CI validation** - Catch drift before release
5. **Rich metadata** - Category, syntax, demo flag all in one place

### Negative

1. **Migration effort** - Must define all 158 functions in registry
2. **Redundancy** - Function name appears in registry AND evaluator match
3. **Build complexity** - Doc generation adds build step

### Mitigations

1. **Script-assisted migration** - Parse existing evaluator code to bootstrap registry
2. **Macro potential** - Future: derive evaluator match from registry
3. **Optional doc gen** - Only run on release builds

## Alternatives Considered

### 1. Macro-Based Definition

```rust
define_function! {
    name: "SUM",
    category: Aggregation,
    demo: true,
    impl: |args, ctx| { ... }
}
```

**Rejected:** Too complex, harder to debug, compile time impact.

### 2. External Config File (YAML/TOML)

```yaml
functions:
  - name: SUM
    category: aggregation
    demo: true
```

**Rejected:** Requires code generation, adds build dependency.

### 3. Keep Current Design + Better Process

Just be more careful about updating all files.

**Rejected:** Human error inevitable, already failed multiple times.

## Implementation Plan

1. Create `src/functions/registry.rs` with struct definitions
2. Define all 158 functions (script-assisted from evaluator grep)
3. Refactor `cli/commands/functions.rs` to use registry
4. Add `cargo run --bin generate-docs` for FUNCTIONS.md
5. Add CI check comparing registry to evaluator
6. Update forge-gh with generated docs

## References

- v5.16.0 release notes (function count discrepancy discovery)
- `src/cli/commands/functions.rs` (current hardcoded approach)
- `src/core/array_calculator/evaluator/` (implementation)
