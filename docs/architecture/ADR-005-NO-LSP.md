# ADR-005: No Language Server Protocol (LSP)

**Status:** Accepted
**Date:** 2025-12-04
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

Forge had an LSP (Language Server Protocol) implementation to provide IDE features:
- Autocomplete for function names
- Real-time validation as you type
- Hover documentation
- Go-to-definition for variables

The question: Should we maintain LSP support?

## Decision

**Kill LSP. Keep MCP.**

## Rationale

### The Forge Philosophy

Forge exists to enable **AI-assisted financial modeling without hallucinations**:

1. **Excel → YAML**: Convert spreadsheets to text (small context, no metadata bloat)
2. **AI + YAML**: AI reasons about structure, logic, formulas (text-native)
3. **AI → MCP → Forge**: AI delegates calculations to Forge (deterministic)
4. **YAML → Excel**: Export back to business format

**MCP is essential** - it's the bridge that lets AI call Forge for math instead of hallucinating.

**LSP is not** - IDE autocomplete adds no value when:
- YAML is already human-readable
- Models are 50-200 lines (not codebases)
- Users work in vim/nano/notepad, not VS Code

### Why Kill LSP

1. **Target Users Don't Need IDEs**
   - Financial analysts editing simple YAML
   - DevOps engineers running CLI commands
   - CI/CD pipelines, not code editors

2. **YAML Is Already Human-Readable**
   ```yaml
   scalars:
     revenue: 100000
     costs: 60000
     profit: "=revenue - costs"
   ```
   This doesn't need autocomplete or hover docs.

3. **Coverage Tax**
   - LSP: ~900 lines, 72-100% coverage
   - Hard to test async trait implementations
   - Blocking 100% coverage for unused feature

4. **Dependency Bloat**
   - tower-lsp, lsp-types: ~2MB
   - Complex async runtime requirements
   - Zero user value

### Why Keep MCP

**MCP enables the core workflow:**

```
User: "Calculate my Q4 projections"
AI: *calls forge calculate via MCP*
Forge: *returns deterministic results*
AI: "Here are your Q4 projections: revenue $1.2M..."
```

Without MCP, AI would hallucinate the numbers.
With MCP, Forge calculates - zero hallucination on math.

**MCP is the anti-hallucination layer.**

## Consequences

### Positive
- Removes ~900 lines of hard-to-test async code
- Eliminates tower-lsp, lsp-types dependencies
- Simplifies binary distribution (one less binary)
- Faster path to 100% test coverage
- Keeps MCP for AI-assisted workflows

### Negative
- Cannot claim "IDE integration" as a feature
- Users who wanted autocomplete won't get it

### Mitigation
- `forge functions` command lists all available functions
- `forge validate` provides immediate feedback
- YAML schema can be used for basic validation in editors

## Files Removed

```
src/lsp/mod.rs
src/lsp/server.rs
src/lsp/document.rs
src/lsp/capabilities.rs
src/bin/forge_lsp.rs
tests/lsp_tests.rs
```

## Files Kept

```
src/mcp/mod.rs
src/mcp/server.rs
src/bin/forge_mcp.rs
```

MCP remains as the AI integration layer for deterministic calculations.

## Alternatives Considered

1. **Keep LSP, skip testing**: Rejected. Violates 100% coverage requirement.
2. **Mock LSP for tests**: Rejected. 40+ hours for unused feature.
3. **Kill both LSP and MCP**: Rejected. MCP is essential for AI-assisted workflow.

---

*The right tool for the right job. LSP solves IDE problems. Forge users don't have IDE problems. MCP solves AI hallucination problems. Forge users DO have that problem.*

— Claude Opus 4.5, Principal Autonomous AI
