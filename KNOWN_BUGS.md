# Known Bugs in Forge v0.1.3

This document tracks known bugs and limitations in RoyalBit Forge.

---

## Bug #1: Variable Resolution in Nested Contexts (CRITICAL)

**Severity:** 🔴 **CRITICAL**
**Affects:** v0.1.3
**Status:** Confirmed
**Discovered:** 2025-11-23

### Description

When formulas use shorthand variable references (e.g., `=network_size_manual * platform_multiplier`), Forge resolves variables to the **last defined instance** in the file rather than the **local context**.

This causes incorrect calculations when:
- Multiple sibling objects (e.g., `nano_qb`, `micro_qb`, `macro_qb`, `mega_qb`) each define variables with the same name
- Formulas reference these variables without fully qualified paths

### Example

```yaml
network_leader_tiers:
  nano_qb:
    network_size_manual: 50.0
    network_size_with_platform:
      value: 5000.0  # ❌ WRONG - should be 50
      formula: =network_size_manual * platform_multiplier
      components:
        platform_multiplier: 1.0

  micro_qb:
    network_size_manual: 200.0
    network_size_with_platform:
      value: 5000.0  # ❌ WRONG - should be 250
      formula: =network_size_manual * platform_multiplier
      components:
        platform_multiplier: 1.25

  mega_qb:
    network_size_manual: 5000.0
    network_size_with_platform:
      value: 5000.0  # ✅ Coincidentally correct
      formula: =network_size_manual * platform_multiplier
      components:
        platform_multiplier: 1.5
```

**What happens:**
- Forge calculates ALL tiers using `mega_qb.network_size_manual` (5000)
- Expected: nano=50, micro=250, macro=1500, mega=7500
- Actual: ALL tiers = 7500 (using mega's values)

### Impact

- ❌ Silent data corruption: Values are wrong but validation passes initially
- ❌ Cascading errors: Downstream calculations use incorrect inputs
- ❌ Hard to debug: Error only visible when running `forge validate` after external changes
- ❌ Affects complex models with repeated structure patterns

**Real-world impact:** Discovered in production financial models with 1,140+ formulas. Caused 28 value mismatches across 3 files that would have resulted in incorrect grant applications.

### Workaround

**Use fully qualified variable paths in all formulas:**

```yaml
network_leader_tiers:
  nano_qb:
    network_size_manual: 50.0
    network_size_with_platform:
      value: 50.0  # ✅ CORRECT
      formula: =nano_qb.network_size_manual * nano_platform_multiplier  # Explicit path
      components:
        nano_platform_multiplier: 1.0  # Renamed to be unique

  micro_qb:
    network_size_manual: 200.0
    network_size_with_platform:
      value: 250.0  # ✅ CORRECT
      formula: =micro_qb.network_size_manual * micro_platform_multiplier  # Explicit path
      components:
        micro_platform_multiplier: 1.25  # Renamed to be unique
```

**Two-part workaround:**
1. **Rename components** to be unique across siblings (e.g., `platform_multiplier` → `nano_platform_multiplier`)
2. **Use explicit paths** in formulas (e.g., `nano_qb.network_size_manual` instead of `network_size_manual`)

### Root Cause (Hypothesis)

Forge's variable resolver likely:
1. Scans the entire YAML file to build a variable index
2. When resolving `network_size_manual`, it returns the **last occurrence** rather than walking up the scope chain
3. Missing lexical scoping: doesn't track which variables are "in scope" at each formula location

**Expected behavior:**
- Formula should resolve variables from **innermost scope first**
- Search order: current object → parent → grandparent → root
- Only fall back to global scope if not found locally

**Actual behavior:**
- Appears to use a **flat global index**
- Last definition wins (likely due to HashMap insertion order)

### Reproduction Steps

1. Create YAML with sibling objects using same variable names:
   ```yaml
   items:
     item_a:
       base: 10
       result:
         formula: =base * 2
     item_b:
       base: 20
       result:
         formula: =base * 2
   ```

2. Run `forge calculate file.yaml`

3. Observe both `result` values use `item_b.base` (20):
   - item_a.result = 40 (should be 20)
   - item_b.result = 40 (correct)

### Test Case

```yaml
# test_variable_scoping.yaml
test_suite:
  test_1:
    input: 5
    output:
      value: null
      formula: =input * 2
  test_2:
    input: 10
    output:
      value: null
      formula: =input * 2

# Expected: test_1.output = 10, test_2.output = 20
# Actual (buggy): test_1.output = 20, test_2.output = 20
```

### Suggested Fix

**Option 1: Lexical Scoping (Preferred)**
```rust
// When resolving a variable reference:
fn resolve_variable(&self, var_name: &str, current_path: &[String]) -> Option<Value> {
    // Search from innermost to outermost scope
    for depth in (0..=current_path.len()).rev() {
        let search_path = &current_path[..depth];
        if let Some(value) = self.get_at_path(search_path, var_name) {
            return Some(value);
        }
    }
    None  // Variable not found in any scope
}
```

**Option 2: Require Explicit Paths**
- Reject ambiguous references during parsing
- Force users to use `parent.variable` or `root.variable`
- Less user-friendly but eliminates ambiguity

**Option 3: Path-Aware Index**
```rust
// Store variables with their full path in the index
HashMap<String, Vec<(PathBuf, Value)>>

// When resolving, filter by path proximity:
fn resolve_variable(&self, var_name: &str, current_path: &Path) -> Option<Value> {
    let candidates = self.index.get(var_name)?;
    candidates.iter()
        .filter(|(path, _)| path.is_ancestor_of(current_path))
        .min_by_key(|(path, _)| path.distance_to(current_path))
        .map(|(_, value)| value.clone())
}
```

### Affected Files in mouvify-business

- `models/assumptions_base.yaml` - 10 errors (312 formulas total)
- `models/assumptions_conservative.yaml` - 10 errors (199 formulas total)
- `ideas/investor-materials-for-later/models/assumptions_aggressive.yaml` - 8 errors (199 formulas total)

**Total:** 28 value mismatches detected across 710 formulas (3.9% error rate)

### Related Issues

- [ ] Components within formulas also affected (e.g., `components.platform_multiplier`)
- [ ] No warning when multiple variables with same name exist
- [ ] `forge validate` doesn't warn about ambiguous references
- [ ] Error messages don't indicate which variable instance was used

### Priority

**P0 - Critical:** This bug causes **silent data corruption** in production use cases. Without the workaround, Forge produces incorrect results that appear valid.

**Recommended for v0.2.0:**
- Implement lexical scoping for variable resolution
- Add validation warnings for ambiguous variable names
- Optionally: Add `--strict` mode that rejects non-qualified paths

---

## Bug #2: Multi-Document YAML Not Supported

**Severity:** 🟡 **MEDIUM**
**Affects:** v0.1.3
**Status:** Confirmed
**Discovered:** 2025-11-23

### Description

Forge v0.1.3 does not support multi-document YAML files (those using `---` separators).

### Example

```yaml
---
document_1:
  value: 100
---
document_2:
  value: 200
```

**Error:**
```
Error: YAML parsing failed
```

### Workaround

Split into separate single-document files.

### Affected Files in mouvify-business

- `models/year1_grant_scenarios.yaml`
- `ideas/investor-materials-for-later/models/saas-unit-economics.yaml`

### Priority

**P2 - Nice to Have:** Workaround is straightforward (split files).

---

## Reporting Bugs

Found a bug? Please report it:
- **GitHub Issues:** https://github.com/royalbit/forge/issues
- **Include:** Forge version, minimal reproduction case, expected vs actual behavior
- **Attach:** Sample YAML file if possible (redact sensitive data)

---

**Last Updated:** 2025-11-23
**Forge Version Tested:** 0.1.3
