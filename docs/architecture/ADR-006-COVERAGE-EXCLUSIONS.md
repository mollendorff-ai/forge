# ADR-006: Coverage Exclusions

**Status:** Accepted
**Date:** 2025-12-04
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

ADR-004 requires 100% test coverage. However, some code is inherently untestable:

1. **Server startup/shutdown** - Binds to ports, runs forever
2. **Signal handlers** - Platform-specific, requires OS signals
3. **Binary entry points** - Thin wrappers around library code
4. **Network operations** - External dependencies, rate limits

## Decision

**Functional code must be 100% tested. Server/network code is excluded via `#[cfg(not(coverage))]` attributes.**

### Implementation

Excluded functions use conditional compilation:

```rust
/// Real implementation - excluded from coverage
#[cfg(not(coverage))]
pub fn run_api_server(config: ApiConfig) -> Result<()> {
    // Binds to port, runs forever...
}

/// Stub for coverage builds
#[cfg(coverage)]
pub fn run_api_server(_config: ApiConfig) -> Result<()> {
    Ok(())
}
```

To run coverage with exclusions active:
```bash
cargo llvm-cov --cfg coverage
```

### Excluded Functions

#### 1. `src/api/server.rs`
| Function | Reason |
|----------|--------|
| `run_api_server()` | Binds to TCP port, runs forever until terminated |
| `shutdown_signal()` | Waits for OS signals (Ctrl+C, SIGTERM) |

#### 2. `src/mcp/server.rs`
| Function | Reason |
|----------|--------|
| `run_mcp_server_sync()` | Reads from stdin forever until EOF |

#### 3. `src/bin/forge_mcp.rs`
| Function | Reason |
|----------|--------|
| `main()` | Binary entry point, calls `run_mcp_server_sync()` |

#### 4. `src/main.rs`
| Function | Reason |
|----------|--------|
| `main()` | Reads from `std::env::args()`, dispatches to library functions |

#### 5. `src/update.rs`
| Function | Reason |
|----------|--------|
| `check_for_update()` | HTTP request to GitHub API |
| `perform_update()` | Downloads files from internet, replaces binary |
| `verify_checksum()` | Downloads checksums.txt from internet |

### Coverage Categories

| Category | Target | Justification |
|----------|--------|---------------|
| Core Calculator | 100% | All Excel functions, formulas |
| Parser | 100% | YAML parsing, validation |
| Excel Import/Export | 100% | Data conversion |
| Writer | 100% | File output |
| CLI Commands | 100% | Business logic |
| Types | 100% | Data structures |
| API Handlers | 100% | Request handling logic |
| API Server | Excluded | Port binding, forever-running |
| MCP Server | 95%+ | Tool handlers tested, stdin loop exempt |
| Update (network) | Excluded | Network calls to GitHub API |
| Update (helpers) | 100% | Version parsing, comparison |
| Binary Entry Points | Excluded | Thin wrappers |

## Consequences

### Positive
- Exclusions are in source code, not external config
- Clear documentation via doc comments on each function
- Functional code maintains 100% requirement
- Stubs ensure code compiles during coverage builds

### Negative
- Requires `--cfg coverage` flag when running llvm-cov
- Duplicate function signatures for stubs

### Excluded Test Files

These integration test files are skipped during coverage builds because they test stubbed binaries:

| Test File | Reason |
|-----------|--------|
| `tests/binary_integration_tests.rs` | Tests binary entry points (stubbed during coverage) |
| `tests/cli_integration_tests.rs` | Tests CLI commands (relies on stubbed main()) |
| `tests/e2e_tests.rs` | End-to-end tests (run forge binary directly) |
| `tests/validation_tests.rs` | Some tests run forge binary directly |

### Verification

The excluded functions are verified via:
1. `tests/binary_integration_tests.rs` - Tests binary entry points as subprocesses (run without `--cfg coverage`)
2. `tests/cli_integration_tests.rs` - Tests CLI commands end-to-end (run without `--cfg coverage`)
3. Manual testing - Server startup, update functionality

---

*Exclusions are explicit, documented, and minimal.*

— Claude Opus 4.5, Principal Autonomous AI
