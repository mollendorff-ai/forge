# ADR-024: Self-Update Command

## Status

**Restored** - v10.0.0-alpha.5

## History

- v9.2.0: Removed (see "Removal Context" below)
- v10.0.0-alpha.5: Restored (forge is now public on GitHub)

## Removal Context (v9.2.0)

The `forge update` command was originally removed because:

1. **Enterprise binary (`forge`)** - Was self-hosted, not published to GitHub
2. **Demo binary (`forge-demo`)** - Public on `mollendorff-ai/forge-demo`, but `forge update` checked `mollendorff-ai/forge/releases` which didn't exist
3. **No valid target** - Neither binary had a valid GitHub release endpoint

## Restoration Context (v10.0.0-alpha.5)

With forge now public on GitHub:
- Repository: `https://github.com/mollendorff-ai/forge`
- Releases published to GitHub with multi-platform binaries
- Self-update now has a valid target

## Current Implementation

### Features

```bash
forge update              # Check and install update (with confirmation)
forge update --check      # Check only, don't install
forge update --verbose    # Show detailed progress
```

### Platform Support

| Platform | Asset Pattern |
|----------|---------------|
| Linux x86_64 | `forge-linux-x86_64.tar.gz` |
| Linux ARM64 | `forge-linux-arm64.tar.gz` |
| macOS Intel | `forge-macos-x86_64.tar.gz` |
| macOS Apple Silicon | `forge-macos-arm64.tar.gz` |
| Windows | `forge-windows.exe` |

### Implementation Details

- Uses `curl` for HTTP requests (available on all platforms)
- Fetches from GitHub API: `api.github.com/repos/mollendorff-ai/forge/releases/latest`
- Semver comparison with pre-release support (e.g., `alpha.5` > `alpha.4`)
- Backs up existing binary to `.bak` before replacement
- Preserves Unix permissions (0755)

### Files

- `src/cli/commands/update.rs` - Main implementation (~300 lines)
- Unit tests for version comparison, platform detection, asset matching

## Consequences

### Positive
- Users can easily update to latest version
- No need to manually download from GitHub releases
- Platform detection is automatic
- Backup created before update (safe rollback)

### Negative
- Requires `curl` to be installed (standard on all supported platforms)
- Cannot update if GitHub is unreachable
- Interactive confirmation required (no `--yes` flag yet)

## References

- ADR-030: GTM Licensing Strategy
- ADR-031: License (MIT OR Apache-2.0)
- Version: v10.0.0-alpha.5
