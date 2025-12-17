# Test Migration Inventory

## Executive Summary

**Total Test Files:** 59
- **STAY (Unit):** 21 files
- **MIGRATE (E2E/Roundtrip/Edge):** 38 files

**Decision:** Migrate 38 files (64% of tests) to forge-e2e to reduce codebase size and improve Claude context management.

---

## 1. Summary Table - All Test Files

| File | Category | Action | Reason |
|------|----------|--------|--------|
| `unit_conditional_complete.rs` | unit | **STAY** | Tests individual conditional functions in isolation |
| `unit_validator_tests.rs` | unit | **STAY** | Tests validator functions in isolation |
| `unit_info_complete.rs` | unit | **STAY** | Tests info functions in isolation |
| `unit_misc_extended.rs` | unit | **STAY** | Tests misc functions in isolation |
| `unit_financial_extended.rs` | unit | **STAY** | Tests XIRR/XNPV financial functions in isolation |
| `unit_lookup_complete.rs` | unit | **STAY** | Tests lookup functions in isolation |
| `unit_trig_complete.rs` | unit | **STAY** | Tests trigonometric functions in isolation |
| `array_calculator_tests.rs` | unit | **STAY** | Tests array calculator core functions |
| `array_calculator_tests_math.rs` | unit | **STAY** | Tests array calculator math functions |
| `array_calculator_tests_logical.rs` | unit | **STAY** | Tests array calculator logical functions |
| `array_calculator_tests_text.rs` | unit | **STAY** | Tests array calculator text functions |
| `array_calculator_tests_dates.rs` | unit | **STAY** | Tests array calculator date functions |
| `array_calculator_tests_financial.rs` | unit | **STAY** | Tests array calculator financial functions |
| `array_calculator_tests_aggregation.rs` | unit | **STAY** | Tests array calculator aggregation functions |
| `array_calculator_tests_lookup.rs` | unit | **STAY** | Tests array calculator lookup functions |
| `array_calculator_tests_statistical.rs` | unit | **STAY** | Tests array calculator statistical functions |
| `array_calculator_tests_forge.rs` | unit | **STAY** | Tests array calculator forge-specific functions |
| `math_complete_tests.rs` | unit | **STAY** | Tests complete math function suite |
| `parser_v1_tests.rs` | unit | **STAY** | Tests YAML parser functionality |
| `error_tests.rs` | unit | **STAY** | Tests error handling and propagation |
| `validation_tests.rs` | unit | **STAY** | Tests schema validation logic |
| `cli_tests.rs` | unit | **STAY** | Tests CLI command functions (not binary) |
| `excel_tests.rs` | unit | **STAY** | Tests Excel import/export core functionality |
| `excel_tests_extended.rs` | unit | **STAY** | Tests Excel roundtrip value verification |
| `api_tests.rs` | unit | **STAY** | Tests API handlers and config (enterprise) |
| `mcp_tests.rs` | unit | **STAY** | Tests MCP integration |
| `e2e_tests_calculate.rs` | e2e | **MIGRATE** | Tests forge binary calculate command end-to-end |
| `e2e_tests_import.rs` | e2e | **MIGRATE** | Tests forge binary import command end-to-end |
| `e2e_tests_export.rs` | e2e | **MIGRATE** | Tests forge binary export command end-to-end |
| `e2e_tests_validation.rs` | e2e | **MIGRATE** | Tests forge binary validate command end-to-end |
| `e2e_tests_version_compat.rs` | e2e | **MIGRATE** | Tests schema version compatibility end-to-end |
| `e2e_tests_audit.rs` | e2e | **MIGRATE** | Tests forge binary audit command end-to-end |
| `e2e_tests_watch.rs` | e2e | **MIGRATE** | Tests forge binary watch command end-to-end |
| `e2e_tests.rs` | e2e | **MIGRATE** | General end-to-end integration tests |
| `cli_integration_tests.rs` | e2e | **MIGRATE** | Tests CLI binary using assert_cmd |
| `binary_integration_tests.rs` | e2e | **MIGRATE** | Tests binary integration workflows |
| `monte_carlo_e2e.rs` | e2e | **MIGRATE** | Tests Monte Carlo simulation end-to-end workflow |
| `e2e_gnumeric_tests.rs` | roundtrip | **MIGRATE** | Validates against Gnumeric spreadsheet engine |
| `roundtrip/mod.rs` | roundtrip | **MIGRATE** | Roundtrip test module coordinator |
| `roundtrip/harness.rs` | roundtrip | **MIGRATE** | E2E test harness for external validation |
| `roundtrip/financial.rs` | roundtrip | **MIGRATE** | Financial functions validated against Gnumeric |
| `roundtrip/lookup.rs` | roundtrip | **MIGRATE** | Lookup functions validated against Gnumeric |
| `roundtrip/aggregation.rs` | roundtrip | **MIGRATE** | Aggregation functions validated against Gnumeric |
| `roundtrip/statistical.rs` | roundtrip | **MIGRATE** | Statistical functions validated against Gnumeric |
| `roundtrip/conditional.rs` | roundtrip | **MIGRATE** | Conditional functions validated against Gnumeric |
| `roundtrip/math.rs` | roundtrip | **MIGRATE** | Math functions validated against Gnumeric |
| `roundtrip/text_date.rs` | roundtrip | **MIGRATE** | Text/Date functions validated against Gnumeric |
| `xlformula_equivalence_tests.rs` | roundtrip | **MIGRATE** | Validates against xlformula_engine crate |
| `formula_edge_cases_tests.rs` | edge | **MIGRATE** | Edge case discovery for all formula categories |
| `formula_edge_cases_tests_lookup.rs` | edge | **MIGRATE** | Edge case discovery for lookup functions |
| `formula_edge_cases_tests_forge.rs` | edge | **MIGRATE** | Edge case discovery for forge-specific functions |
| `formula_edge_cases_tests_logical.rs` | edge | **MIGRATE** | Edge case discovery for logical functions |
| `formula_edge_cases_tests_text.rs` | edge | **MIGRATE** | Edge case discovery for text functions |
| `formula_edge_cases_tests_aggregation.rs` | edge | **MIGRATE** | Edge case discovery for aggregation functions |
| `formula_edge_cases_tests_statistical.rs` | edge | **MIGRATE** | Edge case discovery for statistical functions |
| `formula_edge_cases_tests_dates.rs` | edge | **MIGRATE** | Edge case discovery for date functions |
| `formula_edge_cases_tests_math.rs` | edge | **MIGRATE** | Edge case discovery for math functions |
| `formula_edge_cases_tests_financial.rs` | edge | **MIGRATE** | Edge case discovery for financial functions |
| `performance_bench.rs` | e2e | **MIGRATE** | Performance benchmarks (can run in forge-e2e) |

---

## 2. File Counts by Category

| Category | Count | Action | % of Total |
|----------|-------|--------|------------|
| **unit** | 26 | STAY | 44% |
| **e2e** | 11 | MIGRATE | 19% |
| **roundtrip** | 9 | MIGRATE | 15% |
| **edge** | 12 | MIGRATE | 20% |
| **performance** | 1 | MIGRATE | 2% |
| **TOTAL** | 59 | - | 100% |

**Migration Summary:**
- **STAY in forge:** 26 files (44%)
- **MIGRATE to forge-e2e:** 33 files (56%)

---

## 3. Shared Dependencies & Utilities

### 3.1 Test Harness Files (MIGRATE with roundtrip tests)

**File:** `roundtrip/harness.rs`
- **Purpose:** E2E test harness for validating Forge against external spreadsheet engines (Gnumeric/LibreOffice)
- **Dependencies:**
  - Requires `forge` binary in PATH
  - Requires external spreadsheet engine (ssconvert/libreoffice)
  - Uses tempfile for temporary test files
- **Used By:** All roundtrip tests + `e2e_gnumeric_tests.rs`
- **Migration Note:** This is the core infrastructure for external validation - must migrate with roundtrip tests

### 3.2 Common Test Utilities (Analyze for duplication)

**Identified Patterns:**
1. **Model Building Helpers:**
   - `Variable::new()` - used across all unit tests
   - `Table::new()` / `Column::new()` - used in array calculator tests
   - `ParsedModel::new()` - used everywhere

2. **Binary Execution Helpers:**
   - `forge_binary()` - found in `e2e_tests_calculate.rs`, `roundtrip/harness.rs`, `cli_integration_tests.rs`
   - `test_data_path()` - found in multiple e2e test files

3. **Assertion Helpers:**
   - `approx_eq()` - found in `roundtrip/harness.rs`
   - `assert_within_percent()` - found in `monte_carlo_e2e.rs`
   - Value comparison patterns repeated across tests

**Recommendation:** Create shared test utilities crate or module for both forge and forge-e2e

### 3.3 Test Data Files

**Location:** `/Users/rex/src/royalbit/forge/test-data/`

**Status:** Keep in forge, make accessible to forge-e2e via relative path or symlink

**Rationale:** Test data is shared between unit and e2e tests, should remain centralized

---

## 4. Dependencies Between Test Files

### 4.1 Module Dependencies

```
e2e_gnumeric_tests.rs
    └── mod roundtrip (uses all roundtrip modules)
        ├── roundtrip/harness.rs (shared infrastructure)
        ├── roundtrip/financial.rs
        ├── roundtrip/lookup.rs
        ├── roundtrip/aggregation.rs
        ├── roundtrip/statistical.rs
        ├── roundtrip/conditional.rs
        ├── roundtrip/math.rs
        └── roundtrip/text_date.rs
```

**Migration Impact:** The entire `roundtrip/` directory must migrate as a unit with `e2e_gnumeric_tests.rs`

### 4.2 Feature Flag Dependencies

**Coverage Flag (`#![cfg(not(coverage))]`):**
- `e2e_tests_calculate.rs` - skipped during coverage (stubbed binary)
- `cli_integration_tests.rs` - skipped during coverage (stubbed binary)
- `binary_integration_tests.rs` - skipped during coverage (stubbed binary)
- `validation_tests.rs` - skipped during coverage (uses binary)
- All `e2e_gnumeric_tests.rs` - skipped during coverage
- **Impact:** forge-e2e must preserve these feature flags

**Demo Flag (`#![cfg(not(feature = "demo"))]`):**
- `monte_carlo_e2e.rs` - enterprise only
- `api_tests.rs` - enterprise only
- `unit_financial_extended.rs` - XIRR/XNPV are enterprise only
- Many other tests conditionally skip enterprise features
- **Impact:** forge-e2e must handle demo vs full builds

**E2E Gnumeric Flag (`#![cfg(feature = "e2e-gnumeric")]`):**
- All roundtrip tests require this flag
- `e2e_gnumeric_tests.rs` requires this flag
- **Impact:** forge-e2e must support this feature flag

### 4.3 External Tool Dependencies

**Required for Roundtrip Tests:**
1. **Gnumeric** (`ssconvert --version`) - preferred
2. **LibreOffice** (`libreoffice --version`) - fallback
3. **R** - potentially used in some statistical validation

**Required for E2E Tests:**
1. **forge binary** - must be in PATH or target/debug or target/release
2. **test-data directory** - must be accessible

---

## 5. Recommended Migration Order

### Phase 1: Setup forge-e2e Infrastructure (Week 1)
1. Create `forge-e2e` crate with Cargo.toml
2. Set up directory structure:
   ```
   forge-e2e/
   ├── Cargo.toml
   ├── tests/
   │   ├── e2e/           (e2e tests)
   │   ├── roundtrip/     (roundtrip tests)
   │   └── edge/          (edge case tests)
   └── test-utils/        (shared utilities)
   ```
3. Create symlink or path reference to forge/test-data
4. Set up CI pipeline for forge-e2e

### Phase 2: Migrate Roundtrip Tests (Week 2)
**Why First:** Self-contained module with clear boundaries

1. Copy `roundtrip/` directory to `forge-e2e/tests/roundtrip/`
2. Copy `e2e_gnumeric_tests.rs` to `forge-e2e/tests/`
3. Copy `xlformula_equivalence_tests.rs` to `forge-e2e/tests/roundtrip/`
4. Update imports and paths
5. Test in isolation
6. **Verification:** All roundtrip tests pass in forge-e2e
7. Remove from forge after verification

**Files (9 total):**
- `roundtrip/mod.rs`
- `roundtrip/harness.rs`
- `roundtrip/financial.rs`
- `roundtrip/lookup.rs`
- `roundtrip/aggregation.rs`
- `roundtrip/statistical.rs`
- `roundtrip/conditional.rs`
- `roundtrip/math.rs`
- `roundtrip/text_date.rs`
- `e2e_gnumeric_tests.rs`
- `xlformula_equivalence_tests.rs`

### Phase 3: Migrate Edge Case Tests (Week 3)
**Why Second:** No dependencies on other test files

1. Copy all `formula_edge_cases_tests*.rs` to `forge-e2e/tests/edge/`
2. Update imports
3. Test in isolation
4. **Verification:** All edge case tests pass
5. Remove from forge after verification

**Files (12 total):**
- `formula_edge_cases_tests.rs`
- `formula_edge_cases_tests_lookup.rs`
- `formula_edge_cases_tests_forge.rs`
- `formula_edge_cases_tests_logical.rs`
- `formula_edge_cases_tests_text.rs`
- `formula_edge_cases_tests_aggregation.rs`
- `formula_edge_cases_tests_statistical.rs`
- `formula_edge_cases_tests_dates.rs`
- `formula_edge_cases_tests_math.rs`
- `formula_edge_cases_tests_financial.rs`

### Phase 4: Migrate E2E Tests (Week 4)
**Why Third:** Need to ensure binary integration works in new crate

1. Copy all `e2e_tests*.rs` to `forge-e2e/tests/e2e/`
2. Copy `cli_integration_tests.rs` to `forge-e2e/tests/e2e/`
3. Copy `binary_integration_tests.rs` to `forge-e2e/tests/e2e/`
4. Copy `monte_carlo_e2e.rs` to `forge-e2e/tests/e2e/`
5. Copy `performance_bench.rs` to `forge-e2e/tests/e2e/`
6. Update binary path resolution
7. Test with both debug and release builds
8. **Verification:** All e2e tests pass
9. Remove from forge after verification

**Files (11 total):**
- `e2e_tests_calculate.rs`
- `e2e_tests_import.rs`
- `e2e_tests_export.rs`
- `e2e_tests_validation.rs`
- `e2e_tests_version_compat.rs`
- `e2e_tests_audit.rs`
- `e2e_tests_watch.rs`
- `e2e_tests.rs`
- `cli_integration_tests.rs`
- `binary_integration_tests.rs`
- `monte_carlo_e2e.rs`
- `performance_bench.rs`

### Phase 5: Cleanup & Verification (Week 5)
1. Remove all migrated test files from forge
2. Update forge's Cargo.toml to remove test dependencies no longer needed
3. Update CI to run both forge unit tests and forge-e2e tests
4. Verify forge codebase size reduction
5. Verify Claude can handle forge context better
6. Update documentation

---

## 6. Expected Impact

### 6.1 Codebase Size Reduction

**Before Migration:**
```
forge/tests/: 59 test files
Estimated LOC: ~25,000 lines
```

**After Migration:**
```
forge/tests/: 26 unit test files
Estimated LOC: ~10,000 lines

forge-e2e/tests/: 33 test files
Estimated LOC: ~15,000 lines
```

**Size Reduction in forge:** ~60% reduction in test code

### 6.2 Build Time Impact

**Before:**
- Unit tests + E2E tests run together
- Slower CI pipeline (serial execution)

**After:**
- Unit tests (fast, isolated) - can run frequently
- E2E tests (slow, external deps) - can run less frequently or in parallel

### 6.3 Maintenance Benefits

1. **Clearer Separation:** Unit tests for fast feedback, E2E for comprehensive validation
2. **Parallel Development:** Can work on unit tests without running slow E2E suite
3. **Better CI:** Can run unit tests on every commit, E2E tests nightly or on main branch
4. **Easier Debugging:** Test failures isolated to specific test suite

---

## 7. Risks & Mitigations

### Risk 1: Binary Path Resolution
**Issue:** E2E tests need to find forge binary from different crate

**Mitigation:**
- Use `CARGO_BIN_EXE_forge` environment variable
- Fallback to `../target/debug/forge` and `../target/release/forge`
- Document binary path requirements

### Risk 2: Test Data Access
**Issue:** forge-e2e needs access to forge/test-data/

**Mitigation:**
- Create symlink: `ln -s ../forge/test-data forge-e2e/test-data`
- Or use relative paths: `../forge/test-data`
- Or copy test data to forge-e2e (increases duplication)

### Risk 3: Shared Test Utilities
**Issue:** Common helper functions duplicated across test files

**Mitigation:**
- Create `forge-test-utils` crate shared by both forge and forge-e2e
- Or duplicate utilities in each crate (accept some duplication)
- Document which approach is chosen

### Risk 4: Feature Flag Complexity
**Issue:** Tests use multiple feature flags (demo, e2e-gnumeric, coverage)

**Mitigation:**
- Preserve all feature flags in forge-e2e
- Document feature flag dependencies clearly
- Ensure CI tests all feature combinations

### Risk 5: CI Pipeline Changes
**Issue:** Need to update CI to run tests from two crates

**Mitigation:**
- Update CI config to run `cargo test` in both forge and forge-e2e
- Consider running e2e tests less frequently (nightly vs per-commit)
- Add matrix testing for feature flag combinations

---

## 8. Success Criteria

✅ **Migration is successful when:**

1. All 26 unit tests pass in forge
2. All 33 e2e/roundtrip/edge tests pass in forge-e2e
3. forge codebase is ~40-60% smaller (LOC)
4. Claude can successfully reason about forge context without truncation
5. CI pipeline runs both test suites successfully
6. No test coverage is lost (same number of tests before/after)
7. Documentation is updated to reflect new structure

---

## 9. Post-Migration Checklist

- [ ] forge/tests/ contains only 26 unit test files
- [ ] forge-e2e/tests/ contains 33 migrated test files
- [ ] All roundtrip tests pass with Gnumeric integration
- [ ] All e2e tests can find and execute forge binary
- [ ] CI runs both test suites
- [ ] Documentation updated (README, CONTRIBUTING, ADRs)
- [ ] Test data accessible to both crates
- [ ] Feature flags work correctly in both crates
- [ ] BUG-010 through BUG-013 caught by forge-e2e tests
- [ ] Claude context size issue resolved for forge codebase

---

**Generated:** 2025-12-17
**Author:** Claude Opus 4.5 via forge-demo e2e analysis
