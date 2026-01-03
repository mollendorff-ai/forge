# ADR-032: CLI Schema and Examples Commands

**Status:** Accepted
**Date:** 2026-01-02
**Author:** Claude Opus 4.5 (AI Engineering Agent)

---

## Context

Users need better discoverability for:

1. **JSON Schemas** - For IDE integration, validation, and understanding model structure
2. **Runnable Examples** - For learning Forge-specific capabilities beyond Excel functions

Currently:
- Schemas exist in `schema/` but require manual file access
- Examples are scattered in tests and documentation
- `--help` is the only CLI documentation mechanism
- No way to quickly see what Monte Carlo, Decision Trees, etc. look like in practice

## Decision

**Add two new CLI commands: `forge schema` and `forge examples`**

### `forge schema`

```bash
forge schema              # List available versions
forge schema v1           # Show v1.0.0 schema (scalar-only)
forge schema v5           # Show v5.0.0 schema (full features)
forge schema v5 > schema.json  # Pipe to file for IDE use
```

### `forge examples`

```bash
forge examples                    # List all 9 examples
forge examples monte-carlo        # Show Monte Carlo YAML
forge examples monte-carlo --run  # Show and execute
forge examples --json             # Machine-readable list
```

## Implementation

### Compile-Time Embedding

Use `include_str!()` to embed schemas and examples directly in the binary:

```rust
const SCHEMA_V1: &str = include_str!("../../../schema/forge-v1.0.0.schema.json");
const SCHEMA_V5: &str = include_str!("../../../schema/forge-v5.0.0.schema.json");

const EXAMPLE_MONTE_CARLO: &str = include_str!("../../../examples/monte-carlo.yaml");
// ... etc
```

Benefits:
- Zero network dependency
- Works offline
- Examples guaranteed to match current version
- No external file dependencies

### File Structure

```
src/cli/commands/
├── schema.rs      # forge schema command
└── examples.rs    # forge examples command

examples/
├── monte-carlo.yaml
├── scenarios.yaml
├── decision-tree.yaml
├── real-options.yaml
├── tornado.yaml
├── bootstrap.yaml
├── bayesian.yaml
├── variance.yaml
└── breakeven.yaml
```

### Examples Covered

| Example | Capability | Command |
|---------|------------|---------|
| `monte-carlo` | Probabilistic simulation | `forge simulate` |
| `scenarios` | Probability-weighted scenarios | `forge scenarios` |
| `decision-tree` | Sequential decisions | `forge decision-tree` |
| `real-options` | Option pricing | `forge real-options` |
| `tornado` | Sensitivity analysis | `forge tornado` |
| `bootstrap` | Confidence intervals | `forge bootstrap` |
| `bayesian` | Probabilistic graphs | `forge bayesian` |
| `variance` | Budget vs actual | `forge calculate` |
| `breakeven` | Break-even analysis | `forge calculate` |

## Rationale

### Why Separate Commands (Not `--help` Extensions)?

1. **Keeps `--help` concise** - Users scanning help don't want 500-line schemas
2. **Pipeable output** - `forge schema v5 > schema.json` for IDE integration
3. **Executable examples** - `--run` flag actually runs the example
4. **Machine-readable** - `--json` for tooling integration

### Why Compile-Time Embedding?

1. **Offline-first** - Forge is meant to work without network
2. **Version consistency** - Examples always match binary version
3. **No file resolution** - Works regardless of installation path
4. **Fast** - No I/O at runtime

### Why These 9 Examples?

These are Forge's unique capabilities - features Excel cannot replicate:

- Monte Carlo, Bootstrap, Bayesian (probabilistic)
- Decision Trees, Real Options (sequential decisions)
- Tornado (sensitivity visualization)
- Scenarios (probability-weighted outcomes)
- Variance, Break-even (FP&A-native functions)

Excel functions (SUM, NPV, etc.) don't need examples - they work as expected.

## Consequences

### Positive

1. **Better discoverability** - Users can explore capabilities from CLI
2. **Copy-paste ready** - Examples are runnable without modification
3. **IDE integration** - Schemas can be extracted for editor support
4. **Self-documenting** - Binary contains its own documentation

### Negative

1. **Binary size increase** - ~50KB for embedded examples
2. **Maintenance burden** - Examples must stay in sync with features
3. **No partial updates** - Changing examples requires recompilation

### Neutral

1. **Future examples** - Easy to add more by adding files to `examples/`
2. **Versioned schemas** - Can add v6.0.0, etc. as needed

## Alternatives Considered

### 1. Web-hosted documentation
**Rejected.** Forge is offline-first. Network dependency is unacceptable.

### 2. Man pages
**Rejected.** Platform-specific, harder to maintain, less accessible.

### 3. Extended `--help` with examples
**Rejected.** Makes help output overwhelming. Hard to pipe/parse.

### 4. Separate `forge-docs` binary
**Rejected.** Extra binary to distribute. Friction for users.

## Testing

1. `forge schema v1` - Outputs valid JSON
2. `forge schema v5` - Outputs valid JSON
3. `forge examples` - Lists all 9 examples
4. `forge examples <name>` - Shows YAML for each
5. `forge examples <name> --run` - Executes successfully
6. All examples validate against appropriate schema

---

*This decision improves CLI discoverability while maintaining Forge's offline-first philosophy.*

-- Claude Opus 4.5, AI Engineering Agent
