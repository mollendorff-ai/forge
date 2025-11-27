# Forge - YAML Formula Calculator

## CRITICAL: Self-Healing Protocol (Survives Auto-Compact)

After ANY context compaction, confusion, or uncertainty, RE-READ:
1. `warmup.yaml` - Full protocol and rules
2. `.claude_checkpoint.yaml` - Session state (if exists)

## Mandatory Checkpoints

- **Every 2 hours**: Write progress to `.claude_checkpoint.yaml`, re-read `warmup.yaml`
- **Before any commit**: Re-read quality gates from `warmup.yaml`
- **After task completion**: Update `.claude_checkpoint.yaml`
- **When confused**: STOP → re-read `warmup.yaml` → re-read `.claude_checkpoint.yaml`

## Core Rules (Memorize - These Must Survive)

- 4hr MAX session, 1 milestone, NO scope creep
- Tests pass + ZERO warnings → then commit
- NO "let me also...", NO "while I'm here..."
- Done > Perfect. Ship it.

## Commands

```bash
cargo test                           # Must pass
cargo clippy -- -D warnings          # Zero warnings
cargo build --release                # Before release
```

## Quality Gates

1. `cargo test` passes
2. `cargo clippy -- -D warnings` clean
3. CHANGELOG.md updated
4. Version bumped if releasing

## Key Files

- `warmup.yaml` - Full protocol (RE-READ after compact)
- `roadmap.yaml` - Current milestone
- `Cargo.toml` - Version
- `src/` - Source code
