# ADR-024: Self-Update Command Removal

## Status

**Implemented** - v9.2.0

## Context

The `forge update` command was designed to check for updates and self-update the binary by downloading releases from GitHub. However, this functionality has become dead code:

1. **Enterprise binary (`forge`)** - Self-hosted, proprietary, NEVER pushed to GitHub
2. **Demo binary (`forge-demo`)** - Public on `royalbit/forge-demo`, but `forge update` checks `royalbit/forge/releases` which doesn't exist
3. **No valid target** - Neither binary has a valid GitHub release endpoint for self-update

The update module was ~28KB of code that could never successfully complete its intended function.

## Decision

**Remove the `forge update` command entirely.**

### Options Considered

| Option | Description | Chosen |
|--------|-------------|--------|
| A. Fix it | Point demo at correct repo, disable for enterprise | No - complex, still useless for enterprise |
| B. Remove it | Delete entire update module and command | **Yes** |
| C. Keep disabled | Gate behind feature flag, document as broken | No - dead code accumulates |

### Rationale

- **Zero value**: Command cannot work for either binary distribution
- **Code hygiene**: ~28KB of unused code removed
- **Simpler maintenance**: One less command to document and test
- **Clear installation path**: README directs users to GitHub releases or local build

## Changes

### Removed Files
- `src/update.rs` (28KB) - Update checking and binary replacement logic
- `tests/update_tests.rs` (4KB) - Unit tests for update module

### Modified Files
- `src/main.rs` - Removed `Update` command variant and match arm
- `src/lib.rs` - Removed `pub mod update;`
- `tests/cli_integration_tests.rs` - Removed 3 update-related tests
- `README.md` - Removed `update` from enterprise commands list
- `docs/cli/README.md` - Removed update command documentation

## Installation Guidance

Users should install via:

### Demo (forge-demo)
```bash
# Download from GitHub releases
curl -L https://github.com/royalbit/forge-demo/releases/latest/download/forge-demo-$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m) -o forge-demo
chmod +x forge-demo
```

### Enterprise (forge)
```bash
# Build from source (requires access to private repo)
cargo build --release --features full
cp target/release/forge ~/bin/
```

## Consequences

### Positive
- Cleaner codebase (~32KB code removed)
- Fewer tests to maintain (3 tests removed)
- No confusion about non-functional command
- Simpler --help output

### Negative
- No self-update mechanism (acceptable - users can re-download or rebuild)
- Breaking change for any scripts using `forge update` (unlikely - command never worked)

## References

- Issue: Dead code in update module
- Version: v9.2.0
