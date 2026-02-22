# ADR-047: Dual CLI/MCP Output Architecture (Core Extraction Pattern)

**Status:** Accepted
**Date:** 2026-02-22
**Author:** Claude Opus 4.6 (AI Engineering Agent)

---

## Context

Forge's MCP server communicates with AI hosts (Claude Desktop, Claude Code) via JSON-RPC over stdin/stdout. Two problems existed:

1. **Stdout pollution**: CLI command functions used `println!()` to display human-readable output. When these same functions were called from the MCP server, the printed text corrupted the JSON-RPC transport. AI hosts received interleaved plain text and JSON, causing protocol parse failures.

2. **Missing MCP tools**: Of Forge's 20 command capabilities, only 10 were exposed via MCP. The 7 analysis engines (simulate, scenarios, decision-tree, real-options, tornado, bootstrap, bayesian) and 3 discovery tools (schema, functions, examples) had no MCP tool definitions. AI agents could not access probabilistic modeling, sensitivity analysis, or self-documentation features.

## Decision

**Extract `*_core()` functions that return structured result types. CLI wrappers call core + print. MCP handlers call core + serialize to JSON.**

### Pattern

Each command is split into two layers:

```
cli::calculate()      -> calls calculate_core() -> prints result
mcp::call_tool()      -> calls calculate_core() -> serializes to JSON-RPC
```

The `*_core()` functions:
- Accept typed parameters (not CLI strings)
- Return `Result<T, ForgeError>` where `T: Serialize`
- Never call `println!()`, `eprintln!()`, or any I/O
- Are the single source of truth for command logic

### New MCP Tools (10 added)

**7 Analysis Engine Tools:**
- `forge_simulate` - Monte Carlo simulation with distributions
- `forge_scenarios` - Probability-weighted scenario analysis
- `forge_decision_tree` - Sequential decisions with backward induction
- `forge_real_options` - Option pricing for managerial flexibility
- `forge_tornado` - One-at-a-time sensitivity analysis
- `forge_bootstrap` - Non-parametric confidence intervals
- `forge_bayesian` - Bayesian network inference

**3 Discovery Tools:**
- `forge_schema` - Display JSON schemas for model validation
- `forge_functions` - List all 173 supported functions
- `forge_examples` - Show runnable YAML examples

## Consequences

### Positive

1. **Clean protocol**: MCP JSON-RPC transport is never corrupted by stray stdout output
2. **Full tool coverage**: All 20 tools now available to AI agents (10 existing + 7 analysis + 3 discovery)
3. **Structured results**: MCP tools return typed JSON instead of formatted text, enabling programmatic use by AI agents
4. **Separation of concerns**: Presentation logic (CLI formatting) is fully decoupled from business logic (core computation)
5. **Testability**: Core functions are independently testable without capturing stdout

### Negative

1. **Code duplication**: Each command now has two entry points (CLI wrapper + MCP handler) that must stay in sync
2. **Migration effort**: All 20 commands required refactoring to extract core logic

### Neutral

1. **Engine result types already derive `Serialize`**: Monte Carlo, Scenarios, Decision Trees, Real Options, Tornado, Bootstrap, and Bayesian result structs already had `#[derive(Serialize)]`, so MCP serialization required no new derive annotations
2. **API server benefits**: The REST API server (`forge serve`) can also call `*_core()` functions directly

## Alternatives Considered

### 1. Redirect stdout during MCP calls
**Rejected.** Thread-unsafe, fragile, and would hide legitimate errors. Does not solve the missing tools problem.

### 2. Add `--json` flag to CLI commands
**Rejected.** Conflates CLI concerns with MCP transport. Would still require calling CLI dispatch from MCP, which introduces unnecessary complexity.

### 3. Separate MCP binary with duplicated logic
**Rejected.** Would create maintenance burden and divergence risk. The core extraction pattern keeps logic in one place.

---

*This decision enables AI agents to access Forge's full analytical capabilities through MCP while maintaining clean JSON-RPC protocol compliance.*

-- Claude Opus 4.6, AI Engineering Agent
