# ADR-027: E2E Test Migration to forge-e2e

## Status

**Accepted** - Planned for v10.0.0

## Context

Forge has grown to 636,000 lines of code, which exceeds Claude's effective context window for reliable analysis (~200,000 tokens). This has created challenges for AI-assisted development and code maintenance.

### Current Test Architecture

| Test Type | Location | Size | Purpose |
|-----------|----------|------|---------|
| Unit tests | `src/*/tests/*.rs` | 22,620 LOC | Test functions in isolation |
| Integration tests | `tests/*.rs` | 59 files, 25,089 LOC | Test module interactions |
| **Total** | | **47,709 LOC** | **~8% of codebase** |

### The Context Management Problem

1. **Claude token limit**: 200K tokens ≈ 150K LOC with reasonable context
2. **Forge codebase**: 636K LOC is 4x the effective limit
3. **Result**: Incomplete analysis, missed edge cases, degraded code quality

### The Wake-Up Call: forge-demo E2E Bugs

The separate `forge-demo` e2e test suite caught **BUG-010 through BUG-013** that were missed in Forge's internal tests:

- **BUG-010**: Boolean-number comparison in IF (`=IF(TRUE = 1, 1, 0)`)
- **BUG-011**: String literal comparison in IF (`=IF("ABC" = "abc", 1, 0)`)
- **BUG-012**: TRIM internal spaces inconsistency (`=LEN(TRIM("  a  b  "))`)
- **BUG-013**: `0^0` edge case (Excel convention: returns 1)

**These bugs were in production until forge-demo caught them.**

### Unit vs Integration vs E2E

| Test Type | Validation | Location | Trust Model |
|-----------|------------|----------|-------------|
| Unit | Function works as written | `src/*/tests/` | Self-validation |
| Integration | Modules interact correctly | `tests/` | Internal validation |
| **E2E** | **Output matches external engines** | **forge-e2e repo** | **Third-party validation** |

**Key insight**: Unit and integration tests prove code works as written. E2E tests prove code is *correct* by comparing against Gnumeric, R, Python, and Excel.

## Decision

**Migrate all integration and e2e tests from `tests/` to a separate `forge-e2e` repository.**

### What Stays in Forge

- **Unit tests** (`src/*/tests/*.rs`) - Test individual functions in isolation
- **Test count**: 2,703 unit tests (displayed in `--help`)
- **Purpose**: Fast feedback during development, prove functions work as written

### What Moves to forge-e2e

- **Integration tests** (`tests/*.rs`) - 59 files, 25,089 LOC
- **E2E validation** - Tests that compare Forge output to external engines:
  - Gnumeric (see ADR-007) - Primary validation for Excel functions
  - R - Gold standard for statistical validation (boot, stats, bnlearn)
  - Excel (via COM automation on Windows CI)
- **Roundtrip tests** - YAML → XLSX → recalculate → compare
- **Performance benchmarks** - Large model stress tests

### New Architecture

```
forge/
├── src/
│   ├── calculate/tests/   ← Unit tests (KEEP)
│   ├── parser/tests/       ← Unit tests (KEEP)
│   └── */tests/            ← Unit tests (KEEP)
└── tests/                  ← REMOVE (move to forge-e2e)

forge-e2e/ (NEW REPO)
├── tests/
│   ├── integration/        ← Moved from forge/tests/
│   ├── gnumeric/           ← E2E validation (ADR-007)
│   ├── r_validation/       ← R comparison tests
│   ├── python_validation/  ← Python comparison tests
│   └── benchmarks/         ← Performance stress tests
└── Cargo.toml              ← Depends on forge as library
```

## Rationale

### 1. Context Management

**Problem**: 636K LOC exceeds Claude's context window by 4x.

**Solution**: Remove 25K LOC of integration tests → ~611K LOC (still large, but 4% smaller).

**Impact**: Every percentage point matters when working at scale. Removing integration tests makes room for core logic analysis.

### 2. Separation of Concerns

| Concern | forge (core) | forge-e2e (validation) |
|---------|--------------|------------------------|
| What? | Financial modeling engine | Validation against external engines |
| Tests what? | Functions work as written | Output is mathematically correct |
| Runs when? | Every commit (`cargo test`) | Release candidates, nightly CI |
| Speed | Fast (~2-5s for 2,703 tests) | Slow (~30-60s, spawns external processes) |

**Unit tests belong with code. E2E validation belongs separate.**

### 3. Independent Evolution

- **forge**: Can release engine updates without waiting for full e2e suite
- **forge-e2e**: Can add validation engines (Julia, Mathematica) without bloating forge
- **CI/CD**: Can run unit tests on every commit, e2e on release candidates

### 4. Clearer Trust Model

When enterprise customers ask "how do you know your calculations are correct?", the answer is:

> "We have 2,703 unit tests in the main repo, plus a separate validation suite that compares every formula against Gnumeric, R, Python, and Excel. Here's the forge-e2e repo."

**Separate repos = separate trust layers.**

## Implementation

### Phase 1: Create forge-e2e Repository

```bash
# New repository structure
mkdir -p forge-e2e/{tests,benches,scripts}
cd forge-e2e
cargo init --lib
```

**Cargo.toml**:
```toml
[dependencies]
forge = { path = "../forge" }  # Use forge as library

[dev-dependencies]
# E2E testing dependencies
tempfile = "3.8"
assert_approx_eq = "1.1"
```

### Phase 2: Migrate Integration Tests

```bash
# Move integration tests
mv forge/tests/* forge-e2e/tests/integration/

# Remove tests/ directory from forge
rm -rf forge/tests/
```

### Phase 3: Add External Validation

```bash
# Add Gnumeric validation (see ADR-007)
forge-e2e/tests/gnumeric/

# Add R validation
forge-e2e/tests/r_validation/

# Add Python validation
forge-e2e/tests/python_validation/
```

### Phase 4: Update CI/CD

**.github/workflows/forge.yml** (fast unit tests):
```yaml
test:
  runs-on: ubuntu-latest
  steps:
    - run: cargo test
    # Only unit tests (src/*/tests/)
```

**.github/workflows/forge-e2e.yml** (comprehensive validation):
```yaml
e2e:
  runs-on: ubuntu-latest
  needs: test  # Only run after unit tests pass
  steps:
    - run: cargo test --manifest-path forge-e2e/Cargo.toml
    # Runs integration + e2e validation
```

## Consequences

### Positive

1. **Leaner codebase**: forge drops from 636K to ~611K LOC (4% reduction)
2. **Faster unit tests**: `cargo test` in forge only runs unit tests (~2-5s)
3. **Dedicated validation**: forge-e2e becomes authoritative validation suite
4. **Independent release cycles**: Can update engine without re-running hour-long e2e suite
5. **Clearer trust model**: "We test ourselves (unit) AND validate externally (e2e)"
6. **Better context management**: More room for Claude to analyze core logic
7. **Caught real bugs**: forge-demo proved external validation works (BUG-010 to BUG-013)

### Negative

1. **Two repos to maintain**: forge + forge-e2e
2. **Two test suites**: Must run both for full confidence
3. **CI/CD complexity**: Need to coordinate unit → e2e pipeline
4. **Dependency management**: forge-e2e depends on forge, version synchronization
5. **Developer workflow**: Must clone both repos for comprehensive testing

### Mitigation Strategies

| Problem | Solution |
|---------|----------|
| Two repos | Makefile targets: `make test-all` runs both |
| CI coordination | GitHub Actions: e2e only runs after unit tests pass |
| Version sync | forge-e2e pins to forge git hash, not version |
| Developer UX | README: "Run `cargo test` for fast feedback, forge-e2e for release" |

## Migration Checklist

- [ ] Create `forge-e2e` repository on GitHub
- [ ] Set up Cargo.toml with forge dependency
- [ ] Migrate 59 integration test files from `tests/` to `forge-e2e/tests/integration/`
- [ ] Add Gnumeric validation tests (ADR-007 implementation)
- [ ] Add R validation framework
- [ ] Add Python validation framework
- [ ] Update forge CI to skip `tests/` directory
- [ ] Create forge-e2e CI workflow
- [ ] Update forge README to reference forge-e2e
- [ ] Remove `tests/` directory from forge
- [ ] Update documentation: testing strategy now two-tier

## References

### Related ADRs
- **ADR-007**: E2E Validation via Gnumeric - Technical foundation for external validation
- **ADR-004**: 100% Test Coverage - Unit test strategy (unchanged)
- **ADR-006**: Coverage Exclusions - What not to test (unchanged)

### External Resources
- **forge-e2e repository**: https://github.com/royalbit/forge-e2e (to be created)
- **forge-demo repository**: https://github.com/royalbit/forge-demo (proved need for external validation)

### Validation Engines
- **Gnumeric**: https://www.gnumeric.org/ (primary validation, see ADR-007)
- **R Project**: https://www.r-project.org/ (gold standard for statistical validation)
- **Excel**: Via COM automation on Windows CI runners

> **Note**: Python (scipy/numpy) was considered but removed - R is the gold standard. See forge-e2e ADR-002.

---

**Decision rationale**: forge-demo e2e tests caught bugs (BUG-010 to BUG-013) that internal tests missed. External validation is not optional—it's the only way to prove correctness. Separate repo keeps forge lean while maintaining validation rigor.
