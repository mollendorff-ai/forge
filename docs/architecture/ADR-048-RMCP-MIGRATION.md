# ADR-048: Migrate MCP Server to rmcp SDK

**Status:** Accepted
**Date:** 2026-02-22
**Author:** Claude Opus 4.6 (AI Engineering Agent)

---

## Context

The MCP server (`src/mcp/server.rs`, 1,842 lines) uses hand-rolled JSON-RPC parsing:

- Manual `JsonRpcRequest`, `JsonRpcResponse`, and `JsonRpcError` structs with custom serialization
- A synchronous `BufReader` stdin loop processing one line at a time
- 423 lines of manual `get_tools()` constructing JSON schemas via `json!({})` macro calls for all 20 tools
- 527 lines of manual `call_tool()` extracting parameters from untyped `serde_json::Value` maps
- No transport abstraction (hardcoded to stdin/stdout, no path to SSE or WebSocket)

This approach is fragile (parameter mismatches are runtime errors), verbose (schema definitions duplicated between code and docs), and blocks transport abstraction needed for future web-based MCP hosts.

**Supersedes:** Hand-rolled JSON-RPC implementation from beta.1

**Related:** ADR-047 (MCP Core Extraction Pattern) -- the `*_core()` function layer is unchanged by this migration. Only the protocol glue above it changes.

## Decision

**Migrate to rmcp (Anthropic's official Rust MCP SDK, v0.16).** Use `#[tool]` derive macros for tool registration, `ServerHandler` trait implementation for request routing, and `schemars`-based automatic JSON schema generation from typed request structs.

### Architecture

```
Before (beta.2):
  stdin -> BufReader -> manual JSON parse -> match method string -> manual param extraction -> *_core()

After (beta.3):
  stdin -> rmcp transport -> rmcp router -> #[tool] handler -> typed struct deserialize -> *_core()
```

### Key Changes

1. **Typed request structs** (`src/mcp/types.rs`): Each of the 20 tools gets a dedicated request struct with `#[derive(JsonSchema, Deserialize)]`. Field-level `#[schemars(description = "...")]` annotations replace manual `json!({})` schema construction.

2. **`#[tool]` derive macros**: Tool metadata (name, description) is declared inline with the handler function. rmcp generates the `tools/list` response automatically.

3. **`ServerHandler` trait**: A single `ForgeServer` struct implements rmcp's `ServerHandler`, replacing the manual method dispatch in `handle_request()`.

4. **Async entry point**: The MCP server entry point uses `tokio::main` for rmcp's async transport layer. The `*_core()` functions remain synchronous.

## Consequences

### Positive

1. **~722 net lines eliminated**: Manual JSON-RPC parsing, schema construction, and parameter extraction replaced by derive macros and typed deserialization
2. **Automatic spec compliance**: rmcp handles JSON-RPC framing, error codes, and protocol negotiation per the MCP specification
3. **Type-safe tool definitions**: Parameter mismatches caught at compile time via typed request structs, not at runtime via `Value::as_str().unwrap()`
4. **Automatic JSON schema generation**: `schemars` derives schemas from struct definitions, eliminating manual `json!({})` construction and the risk of schema/code drift
5. **Transport-ready**: rmcp's transport abstraction enables SSE and WebSocket support in beta.4 without changing tool definitions
6. **`*_core()` layer unchanged**: ADR-047's core extraction pattern is fully preserved. Only the protocol glue above it changes.

### Negative

1. **New dependencies**: `rmcp` (+ transitive deps) and `schemars` added to the dependency tree
2. **Async runtime required**: The MCP entry point now requires tokio, though `*_core()` functions remain synchronous

### Neutral

1. **Wire protocol unchanged**: rmcp speaks the same JSON-RPC 2.0 / MCP protocol. Existing clients (Claude Desktop, Claude Code) require no changes.
2. **CI smoke test may need minor adaptation**: If rmcp changes any wire-level details (e.g., error message formatting), the CI MCP smoke test assertions may need updating.

## Alternatives Considered

### 1. Keep hand-rolled JSON-RPC, add transport abstraction manually

**Rejected.** Would require reimplementing transport negotiation, keep-alive, and framing for each transport (SSE, WebSocket). rmcp provides this out of the box.

### 2. Use a generic JSON-RPC crate (jsonrpc-core, etc.) without MCP awareness

**Rejected.** Would handle JSON-RPC framing but not MCP-specific concerns (tool schemas, `tools/list`, `initialize` handshake). Still requires manual MCP protocol implementation.

### 3. Wait for rmcp 1.0 stable release

**Rejected.** rmcp v0.16 is actively maintained by Anthropic, used in production by multiple MCP servers, and the API surface needed (tool macros, stdio transport) is stable. Waiting provides no concrete benefit.

---

*This decision replaces ~1,100 lines of hand-rolled JSON-RPC with Anthropic's official rmcp SDK, gaining type safety, automatic schema generation, and a path to multi-transport support.*

-- Claude Opus 4.6, AI Engineering Agent
