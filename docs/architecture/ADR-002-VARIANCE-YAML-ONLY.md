# ADR-002: Variance Analysis Accepts YAML Only

## Status

**Accepted** - 2025-11-25

## Context

Forge v2.3.0 will add variance analysis for budget vs actual comparisons:

```bash
forge variance budget.yaml actual.yaml
```

The question arose: should variance accept Excel files directly as inputs?

### The Problem with Excel Inputs

Excel files are freeform:
- Any sheet names
- Any column layouts
- Any cell positions
- No enforced schema

Unlike YAML files which follow Forge's JSON Schema, Excel has no predictable structure. Comparing two arbitrary Excel files would require:

1. **Guessing structure** - Which sheet? Which columns? Which rows?
2. **Mapping configuration** - User-defined mappings for each file
3. **Fragile assumptions** - Assuming both files have identical structure

### Options Considered

| Option | Description | Complexity | Reliability |
|--------|-------------|------------|-------------|
| **A: YAML only** | Require YAML inputs | Low | High |
| **B: Auto-import** | Accept Excel, import on-the-fly | Medium | Low |
| **C: Mapping file** | User defines Excel→variable mappings | High | Medium |

## Decision

**Option A: YAML only.**

Variance analysis will only accept YAML files as inputs. Users who have Excel files must first convert them using `forge import`:

```bash
# Convert Excel to YAML first
forge import budget.xlsx -o budget.yaml
forge import actual.xlsx -o actual.yaml

# Then compare
forge variance budget.yaml actual.yaml
```

## Rationale

1. **Deterministic behavior** - YAML structure is known via schema; comparison is predictable
2. **Separation of concerns** - Import handles Excel→YAML conversion; variance handles comparison
3. **Explicit over implicit** - User sees and controls the YAML structure before comparison
4. **Existing tooling** - `forge import` already handles Excel conversion with clear assumptions
5. **Simpler implementation** - No Excel parsing in variance command
6. **Better error messages** - Mismatched variables are clear in YAML; Excel cell references are opaque

## Consequences

### Positive

- Variance command stays simple and focused
- No ambiguity about what's being compared
- Users can inspect/edit YAML before comparison
- Consistent with Forge's "YAML as source of truth" philosophy

### Negative

- Extra step for users with Excel files (must import first)
- Can't do quick one-liner comparison of two Excel files

### Mitigation

Document the workflow clearly:

```bash
# Quick workflow for Excel users:
forge import budget.xlsx -o /tmp/budget.yaml && \
forge import actual.xlsx -o /tmp/actual.yaml && \
forge variance /tmp/budget.yaml /tmp/actual.yaml
```

Could add a convenience script or alias in future if frequently requested.

## Output Formats

While inputs are YAML-only, variance **output** can support multiple formats:

```bash
forge variance budget.yaml actual.yaml              # Terminal table (default)
forge variance budget.yaml actual.yaml -o out.yaml  # YAML output
forge variance budget.yaml actual.yaml -o out.xlsx  # Excel report
```

Excel output makes sense because we control the structure - it's a generated report, not arbitrary input.

---

**Decision made by:** Claude Opus 4.5, Principal Autonomous AI
**Date:** 2025-11-25
