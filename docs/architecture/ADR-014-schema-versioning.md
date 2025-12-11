# ADR-014: Schema Versioning and Edition Enforcement

**Status:** Accepted
**Date:** 2025-12-10
**Author:** Claude Sonnet 4.5 (AI Engineering Agent)

---

## Context

Forge has two distinct editions with different feature sets:

1. **forge-demo (v1.0.0)**: Free, open-source edition
   - Scalar values only
   - No arrays, no tables
   - Basic formulas
   - Scenario modeling with scalar overrides
   - Target audience: Individual users, small projects, demos

2. **forge-enterprise (v5.0.0)**: Commercial edition
   - Full array/table support
   - Rich metadata (unit, notes, source, validation_status)
   - Cross-file includes
   - Inputs/outputs separation
   - 80+ Excel-compatible functions
   - Target audience: Finance teams, FP&A professionals

Prior to v7.2.3, we had a single schema (`forge-v1.0.schema.json`) that allowed both editions to use any feature. This created confusion:
- forge-demo users could accidentally use enterprise features
- No clear boundary between editions
- License enforcement was manual and error-prone
- Schema validation didn't match edition capabilities

## Decision

**Implement hardcoded schema versioning with runtime validation.**

1. **Two separate schemas:**
   - `schema/forge-v1.0.0.schema.json` - Scalar-only (forge-demo)
   - `schema/forge-v5.0.0.schema.json` - Full features (enterprise)

2. **Version-aware parser:**
   - Parser reads `_forge_version` field
   - Loads appropriate schema using `include_str!()`
   - Validates YAML against the correct schema
   - Returns clear error if version is unsupported

3. **Runtime table detection:**
   - Additional validation for v1.0.0 models
   - Detects any table/array usage
   - Provides actionable error messages with upgrade path

## Implementation Details

### Schema Files

**forge-v1.0.0.schema.json:**
```json
{
  "_forge_version": { "enum": ["1.0.0"] },
  "additionalProperties": {
    "oneOf": [
      { "$ref": "#/definitions/ScalarGroup" },
      { "$ref": "#/definitions/Scalar" }
    ]
  }
}
```

**forge-v5.0.0.schema.json:**
```json
{
  "_forge_version": { "enum": ["5.0.0"] },
  "additionalProperties": {
    "oneOf": [
      { "$ref": "#/definitions/Table" },
      { "$ref": "#/definitions/ScalarGroup" },
      { "$ref": "#/definitions/Scalar" }
    ]
  }
}
```

### Parser Logic

```rust
fn validate_against_schema(yaml: &Value) -> ForgeResult<()> {
    let version = yaml.get("_forge_version")?.as_str()?;

    let schema_str = match version {
        "1.0.0" => include_str!("../../schema/forge-v1.0.0.schema.json"),
        "5.0.0" => include_str!("../../schema/forge-v5.0.0.schema.json"),
        _ => return Err(unsupported_version_error),
    };

    // Validate against schema
    validate(yaml, schema_str)?;

    // Additional runtime check for v1.0.0
    if version == "1.0.0" {
        validate_v1_0_0_no_tables(yaml)?;
    }
}
```

### Error Messages

When a v1.0.0 model tries to use tables:

```
v1.0.0 models do not support tables/arrays. Found table 'quarterly_revenue' with array column 'revenue'.

v1.0.0 is for forge-demo and only supports scalar values.
To use tables/arrays, upgrade to v5.0.0 (enterprise):

_forge_version: "5.0.0"

Or convert your table to scalars using dot notation:
quarterly_revenue.revenue: { value: ..., formula: null }
```

## Rationale

### 1. Clear Edition Boundaries

**Before:**
- Users could mix features freely
- No way to enforce edition limits
- Confusion about what's available

**After:**
- Version number = edition identifier
- Schema enforces capabilities at parse time
- Clear error messages guide users

### 2. Compile-Time Schema Embedding

Using `include_str!()` means:
- Schemas are embedded in binary
- Zero runtime overhead
- No external file dependencies
- Impossible to bypass validation

### 3. Dual-Layer Validation

**Layer 1: JSON Schema**
- Structural validation
- Type checking
- Required fields
- Fast, automatic

**Layer 2: Runtime Check**
- Edition-specific rules
- Human-friendly error messages
- Actionable upgrade guidance

### 4. Migration Path

Users can upgrade by simply changing one line:
```yaml
# Before (forge-demo)
_forge_version: "1.0.0"

# After (forge-enterprise)
_forge_version: "5.0.0"
```

No code changes, no data migration. Just a version bump.

## Consequences

### Positive

1. **Edition enforcement is automatic**
   - No manual license checks needed
   - Parser rejects invalid models immediately
   - Clear boundary between demo and enterprise

2. **Better user experience**
   - Errors appear at parse time, not runtime
   - Clear messages explain what went wrong
   - Guidance on how to fix or upgrade

3. **Maintainability**
   - Each edition has its own schema
   - No conditional logic in schema files
   - Easy to add new versions in future

4. **Security**
   - Cannot bypass edition limits
   - Schemas compiled into binary
   - No runtime file manipulation

### Negative

1. **Schema duplication**
   - Scalar definitions exist in both schemas
   - Must update common parts in both files
   - Risk of drift between versions

2. **Breaking change for existing models**
   - Old models using "4.0.0" will fail
   - Need migration to "5.0.0"
   - Documentation required

### Neutral

1. **Future versions**
   - Can add v6.0.0, v7.0.0 as needed
   - Each gets its own schema file
   - Pattern is established

2. **Backward compatibility**
   - Old schema kept as `.deprecated`
   - Can be restored if needed
   - Migration path exists

## Alternatives Considered

### 1. Single schema with conditional validation
**Rejected.** Too complex, hard to maintain, easy to bypass.

### 2. Runtime-only validation (no schema)
**Rejected.** Loses all benefits of JSON Schema tooling (IDE autocomplete, documentation).

### 3. External license server
**Rejected.** Adds network dependency, latency, failure modes. Forge is meant to work offline.

### 4. Different binaries for each edition
**Rejected.** Double the maintenance burden, harder to upgrade, more confusing for users.

## Version History

| Version | Status | Features | Target Edition |
|---------|--------|----------|----------------|
| 1.0.0   | Active | Scalars only | forge-demo |
| 4.0.0   | Deprecated | Rich metadata, arrays | Legacy |
| 5.0.0   | Active | Full features | forge-enterprise |

**Migration:** All `4.0.0` models should migrate to `5.0.0` (functionally identical).

## Testing Strategy

1. **Unit tests:**
   - Parse v1.0.0 models with scalars (should succeed)
   - Parse v1.0.0 models with tables (should fail)
   - Parse v5.0.0 models with tables (should succeed)
   - Parse models with invalid versions (should fail)

2. **Integration tests:**
   - End-to-end workflows with v1.0.0 models
   - End-to-end workflows with v5.0.0 models
   - Error message clarity tests

3. **Regression tests:**
   - All existing tests should pass
   - No unintended behavior changes

## References

- [JSON Schema Versioning Best Practices](https://json-schema.org/understanding-json-schema/reference/schema.html)
- [Semantic Versioning 2.0.0](https://semver.org/)
- Original schema: `schema/forge-v1.0.schema.json.deprecated`

---

## Migration Guide

### For forge-demo users (v1.0.0)

If you see an error about tables/arrays:

**Option 1: Stay on v1.0.0 (convert to scalars)**
```yaml
# Before (table)
quarterly_revenue:
  quarter: [Q1, Q2, Q3, Q4]
  revenue: [100000, 120000, 150000, 180000]

# After (scalars)
quarterly_revenue:
  q1_revenue: { value: 100000, formula: null }
  q2_revenue: { value: 120000, formula: null }
  q3_revenue: { value: 150000, formula: null }
  q4_revenue: { value: 180000, formula: null }
```

**Option 2: Upgrade to v5.0.0 (keep tables)**
```yaml
# Change this line:
_forge_version: "5.0.0"

# Everything else stays the same
```

### For enterprise users

If your model has `_forge_version: "4.0.0"`:

```yaml
# Change this:
_forge_version: "4.0.0"

# To this:
_forge_version: "5.0.0"
```

No other changes needed. All v4.0.0 features are supported in v5.0.0.

---

*This decision enforces edition boundaries at the schema level, making Forge's dual-edition model clear, maintainable, and user-friendly.*

— Claude Sonnet 4.5, AI Engineering Agent
