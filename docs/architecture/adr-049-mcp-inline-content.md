# ADR-049: MCP Inline Content for Sandboxed Clients

**Status:** Accepted
**Date:** 2026-03-07
**Author:** Claude Opus 4.6 (AI Engineering Agent)

---

## Context

Forge MCP tools require a `file_path` parameter pointing to a YAML model on the host filesystem. This works when the MCP client (Claude Desktop, Claude Code) runs on the same machine as the Forge server and has direct filesystem access.

However, sandboxed MCP clients cannot use `file_path`:

- **Claude.ai** generates YAML in a cloud sandbox -- the path does not exist on the Forge host.
- **Cursor** and similar IDE agents may operate in a virtual workspace with no shared filesystem.
- **Container-based clients** have an isolated filesystem that does not overlap with the host.

In all these cases, the client constructs valid Forge YAML but has no way to write it to a path the server can read. Calls fail with `"IO error: No such file or directory"`.

A secondary concern is `_includes` support. Forge models can reference external YAML files via `_includes` entries. Even if the main model content is passed inline, include files would still require host filesystem paths -- defeating the purpose of inline content.

**Related:** ADR-047 (Core Extraction Pattern), ADR-048 (rmcp Migration)

## Decision

**Add `content` and `includes` parameters to all 16 file-path MCP tools.** When `content` is provided, write it to a temporary directory and pass the temp path to existing `*_core()` functions. No changes to the parser, calculation engine, or CLI.

### Parameters

Each of the 16 tools that currently take a file path gains two optional parameters:

| Parameter | Type | Description |
|-----------|------|-------------|
| `content` | `Option<String>` | Raw YAML model content (alternative to `file_path`) |
| `includes` | `Option<HashMap<String, String>>` | Inline include content as namespace-to-YAML map |

- **`file_path` becomes optional** -- clients provide either `file_path` or `content`, not both.
- **`includes`** maps namespace names (matching the `as` field in `_includes` entries) to YAML content strings. Each entry is written to the temp directory using the original `file` field as the filename.
- **`import` is unchanged** -- Excel I/O is inherently file-based.

### Resolution Logic

A helper function `resolve_model_input(file_path, content, includes)` returns `(PathBuf, Option<TempDir>)`:

1. Both `file_path` and `content` provided -- error: "Provide either file_path or content, not both"
2. Neither provided -- error: "Either file_path or content is required"
3. `file_path` only -- return the path directly (existing behavior)
4. `content` only -- create a `TempDir`, write `model.yaml`, resolve includes, return temp path

The `TempDir` guard is kept alive in the handler via a `_tmpdir` binding. When the handler returns, the guard drops and cleans up the temporary directory.

### Special Cases

- **`calculate` with `content` + `dry_run=false`**: The core function writes results to the temp file. The handler reads back the temp file and returns a `calculated_content` field in the JSON response, since there is no persistent file to update.
- **`variance`**: Has two file paths (`budget_path`/`actual_path`), each gaining its own content alternative (`budget_content`/`actual_content`). A single shared `includes` map serves both.
- **`export`**: The YAML source gains `content`/`includes` alternatives. The `excel_path` output path remains required (the client needs a host path for the Excel file).

## Consequences

### Positive

1. **Sandboxed clients work**: Claude.ai, Cursor, and container-based MCP clients can pass YAML content directly without filesystem access.
2. **No core changes**: The temp file approach reuses the entire existing `*_core()` function layer unchanged.
3. **Include support preserved**: The `includes` map lets sandboxed clients provide multi-file models without host filesystem access.
4. **Backward compatible**: Existing clients using `file_path` are unaffected -- the parameter is now optional but behaves identically when provided.

### Negative

1. **`tempfile` becomes a runtime dependency**: Previously a dev-dependency only. Adds ~15 KB to the binary.
2. **Disk I/O overhead**: Inline content is written to a temp file and read back by core functions. For typical model sizes (< 1 MB), this is negligible.

### Neutral

1. **`import` unchanged**: Excel I/O requires physical files on both sides. No inline alternative is practical.
2. **Wire protocol unchanged**: The MCP tool definitions gain optional parameters, which is backward-compatible per JSON-RPC semantics.

## Alternatives Considered

### 1. Add `parse_model_from_str()` to the parser

**Rejected.** Would require changing the signature of 15+ `*_core()` functions from `&Path` to an enum of path-or-string, cascading through the parser, engine, and all analysis modules. The temp file approach achieves the same result with changes isolated to the MCP layer.

### 2. Add a `base_dir` parameter for path resolution

**Rejected.** Only works when the client and server share a filesystem (e.g., different working directories on the same host). Does not solve the sandbox problem where no shared filesystem exists.

### 3. Reject `_includes` in inline mode

**Rejected.** Too limiting. Multi-file models using `_includes` are common in enterprise use cases. The `includes` map provides a clean solution without complicating the core parser.

---

*This decision enables sandboxed MCP clients to use all Forge tools by passing YAML content inline, without requiring host filesystem access.*

-- Claude Opus 4.6, AI Engineering Agent
