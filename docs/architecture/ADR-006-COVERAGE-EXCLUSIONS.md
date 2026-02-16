# ADR-006: Coverage Exclusions

**Status:** Accepted (Updated 2025-12-29)
**Date:** 2025-12-04 (Updated 2025-12-29)
**Author:** Claude Opus 4.5 (Principal Autonomous AI)

---

## Context

ADR-004 requires 100% test coverage. However, some code is **inherently untestable** in a unit test environment:

1. **Server startup/shutdown** - Binds to ports, runs forever
2. **Signal handlers** - Platform-specific, requires OS signals
3. **Binary entry points** - Thin wrappers around library code
4. **Network operations** - External dependencies, rate limits
5. **Interactive I/O** - stdin prompts, terminal detection
6. **File system edge cases** - Permission errors, disk full, symlinks
7. **Platform-specific code** - Terminal colors, OS-specific paths
8. **Process spawning** - Opening editors, browsers, external tools

## Decision

**Functional code must be 100% tested. Untestable code is excluded via `#[cfg(not(coverage))]` attributes.**

This follows the **Rust nightly compiler pattern**: code that cannot be tested in CI is explicitly marked and documented.

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
| Function | Reason | Category |
|----------|--------|----------|
| `run_api_server()` | Binds to TCP port, runs forever | Server |
| `shutdown_signal()` | Waits for OS signals (Ctrl+C, SIGTERM) | Signal |

#### 2. `src/mcp/server.rs`
| Function | Reason | Category |
|----------|--------|----------|
| `run_mcp_server_sync()` | Reads from stdin forever until EOF | Interactive I/O |

#### 3. `src/main.rs` (`forge mcp` and `forge serve` subcommands)

The MCP server and API server are now subcommands of the single `forge` binary (`forge mcp` and `forge serve`),
handled in `src/main.rs`. The old `src/bin/forge_mcp.rs` and `src/bin/forge_server.rs` files no longer exist.

#### 4. `src/main.rs`
| Function | Reason | Category |
|----------|--------|----------|
| `main()` | Reads from `std::env::args()` | Entry Point |

#### 6. `src/cli/commands/mod.rs`
| Function | Reason | Category |
|----------|--------|----------|
| `prompt_confirmation()` | Reads from stdin interactively | Interactive I/O |
| `open_in_editor()` | Spawns external editor process | Process Spawn |
| `detect_terminal_width()` | Platform-specific terminal query | Platform |
| `is_terminal()` | Platform-specific tty detection | Platform |

#### 7. `src/cli/commands/export.rs`
| Function/Path | Reason | Category |
|---------------|--------|----------|
| File permission error paths | Requires privileged filesystem state | File System |
| Disk full error paths | Cannot simulate in unit tests | File System |

#### 8. `src/cli/commands/import.rs`
| Function/Path | Reason | Category |
|---------------|--------|----------|
| File not found error display | Tested via E2E | File System |
| Permission denied paths | Requires privileged filesystem state | File System |

#### 9. `src/cli/commands/serve.rs`
| Function | Reason | Category |
|----------|--------|----------|
| `run_server()` | Binds to port, blocks forever | Server |

#### 10. `src/cli/commands/watch.rs`
| Function | Reason | Category |
|----------|--------|----------|
| `watch_file()` | Blocks on filesystem notify events | Blocking I/O |

#### 11. `src/update.rs` (if present)
| Function | Reason | Category |
|----------|--------|----------|
| `check_for_update()` | HTTP request to GitHub API | Network |
| `perform_update()` | Downloads files, replaces binary | Network |
| `verify_checksum()` | Downloads checksums.txt | Network |

### Coverage Categories

| Category | Target | Current | Justification |
|----------|--------|---------|---------------|
| Core Calculator | 100% | 95%+ | All Excel functions, formulas |
| Parser | 100% | 88% | YAML parsing, validation |
| Excel Import/Export | 100% | 85%+ | Data conversion |
| Writer | 100% | 96% | File output |
| CLI Commands (logic) | 100% | 70% | Business logic |
| CLI Commands (I/O) | Excluded | N/A | Interactive prompts, file errors |
| Types | 100% | 99.8% | Data structures |
| API Handlers | 100% | 76% | Request handling logic |
| API Server | Excluded | N/A | Port binding, forever-running |
| MCP Server (handlers) | 100% | 95%+ | Tool handlers tested |
| MCP Server (stdin) | Excluded | N/A | Stdin loop exempt |
| Binary Entry Points | Excluded | N/A | Thin wrappers |
| Analytics (core) | 100% | 85-95% | Monte Carlo, Bootstrap, Bayesian |
| Analytics (I/O) | Excluded | N/A | File export paths |

### Exclusion Categories Summary

| Category | Description | How to Mark |
|----------|-------------|-------------|
| **Entry Point** | Binary main() functions | `#[cfg(not(coverage))]` on function |
| **Server** | Binds ports, runs forever | `#[cfg(not(coverage))]` on function |
| **Signal** | OS signal handlers | `#[cfg(not(coverage))]` on function |
| **Interactive I/O** | Reads stdin, prompts user | `#[cfg(not(coverage))]` on function |
| **Process Spawn** | Launches external processes | `#[cfg(not(coverage))]` on function |
| **Platform** | OS-specific behavior | `#[cfg(not(coverage))]` on function |
| **Network** | HTTP/TCP to external services | `#[cfg(not(coverage))]` on function |
| **File System** | Permission/disk errors only | `#[cfg(not(coverage))]` on error path |
| **Blocking I/O** | Waits on external events | `#[cfg(not(coverage))]` on function |

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
1. `forge-e2e/` - E2E test suite runs actual binary (836 tests)
2. `tests/binary_integration_tests.rs` - Tests binary entry points as subprocesses
3. Manual testing - Server startup, interactive prompts

---

## Implementation Plan (v9.9.6)

### Current Status (2025-12-29)
- **Overall coverage:** 86.99% (target: 100% after exclusions)
- **Unit tests:** 1,295 passing
- **E2E tests:** 836 (in forge-e2e)

### Phase 1: Mark Untestable Code
Add `#[cfg(not(coverage))]` to all functions in the exclusion list above:

```bash
# Files requiring exclusion markers:
src/api/server.rs          # run_api_server, shutdown_signal
src/mcp/server.rs          # run_mcp_server_sync
src/main.rs                # main (also handles `forge mcp` and `forge serve` subcommands)
src/cli/commands/mod.rs    # prompt_*, open_in_editor, is_terminal
src/cli/commands/serve.rs  # run_server
src/cli/commands/watch.rs  # watch_file
```

### Phase 2: Add Missing Tests for Testable Code
These modules have testable logic that needs more tests:

| Module | Current | Gap | Action |
|--------|---------|-----|--------|
| `cli/commands/export.rs` | 63% | 37% | Test all export formats, error handling |
| `cli/commands/import.rs` | 72% | 28% | Test import validation, format detection |
| `cli/commands/audit.rs` | 85% | 15% | Test audit report generation |
| `api/handlers.rs` | 76% | 24% | Test all HTTP status codes, edge cases |
| `parser/mod.rs` | 88% | 12% | Test malformed YAML edge cases |
| `monte_carlo/*.rs` | 85-96% | 4-15% | Test edge cases in sampling |

### Phase 3: Verify 100% Coverage (After Exclusions)
```bash
# Run coverage with exclusions active
cargo llvm-cov --cfg coverage --fail-under-lines 100

# This should pass after Phase 1 + Phase 2
```

### Ideas to Address Coverage Gaps

1. **Extract Pure Functions**
   - Move all business logic OUT of I/O functions
   - I/O functions become thin wrappers around tested logic
   - Example: `export_to_excel()` calls `prepare_excel_data()` (tested) + `write_file()` (excluded)

2. **Dependency Injection for File Operations**
   - Create trait `FileSystem` with `read()`, `write()`, `exists()`
   - Production: `RealFileSystem`
   - Tests: `MockFileSystem`
   - All file error paths become testable

3. **Builder Pattern for CLI Commands**
   - `CommandBuilder::new().input(file).output(path).dry_run(true).build()`
   - All configuration is testable
   - Only final `execute()` touches I/O

4. **Response Objects for All Operations**
   - Every command returns `CommandResult { success, messages, data }`
   - Test the result generation
   - I/O is just serializing the result

5. **Separate Validation from Execution**
   - `validate_export_request()` - 100% testable, returns errors
   - `execute_export()` - thin I/O wrapper, excluded

---

*Exclusions are explicit, documented, and minimal.*

— Claude Opus 4.5, Principal Autonomous AI (Updated 2025-12-29)
