# Forge JSON Schema

This directory contains JSON Schema definitions for Forge models.

## Schema Files

- `forge-v1.0.schema.json` - Schema for v1.0.0 array model

## What is JSON Schema?

JSON Schema validates the **structure** of your YAML files before Forge processes them.

**Benefits:**

- ✅ **Catch errors early** - Before formulas are evaluated
- ✅ **IDE autocomplete** - VSCode/IntelliJ suggest valid keys
- ✅ **Type safety** - Ensure arrays are homogeneous
- ✅ **Self-documenting** - Schema describes the model format
- ✅ **Financial grade** - Zero tolerance for structural errors

## IDE Integration

### VSCode (Recommended)

Install the **YAML extension** by Red Hat:

1. Install extension: `ext install redhat.vscode-yaml`

2. Add to your workspace settings (`.vscode/settings.json`):

```json
{
  "yaml.schemas": {
    "https://raw.githubusercontent.com/royalbit/forge/main/schema/forge-v1.0.schema.json": [
      "*.forge.yaml",
      "**/forge-models/**/*.yaml"
    ]
  }
}
```

3. Rename your files to `*.forge.yaml` or place them in a `forge-models/` directory

**You now get:**

- ✅ Autocomplete for column names
- ✅ Real-time validation errors
- ✅ Documentation on hover
- ✅ Formula syntax highlighting

### IntelliJ IDEA / PyCharm

1. Settings → Languages & Frameworks → Schemas and DTDs → JSON Schema Mappings

2. Add new mapping:
   - **Schema URL:** `https://raw.githubusercontent.com/royalbit/forge/main/schema/forge-v1.0.schema.json`
   - **File pattern:** `*.forge.yaml` or `forge-models/**/*.yaml`

### Local Schema File

Alternatively, reference the schema locally in your YAML files:

```yaml
# yaml-language-server: $schema=./schema/forge-v1.0.schema.json

_forge_version: "1.0.0"
quarterly_revenue:
  quarter: [Q1, Q2, Q3, Q4]
  revenue: [100000, 120000, 150000, 180000]
```

## Command-Line Validation

Forge automatically validates against the schema:

```bash
# Validate structure before calculating
forge validate model.yaml

# Force schema validation
forge validate-schema model.yaml

# Skip schema validation (not recommended!)
forge calculate model.yaml --skip-schema-validation
```

## Schema Validation Rules

### v1.0.0 Model

**Required:**

- `_forge_version: "1.0.0"` at top level

**Tables (Column Arrays):**

- All arrays in a table must be homogeneous
- Number arrays: `[100, 120, 150]`
- Text arrays: `["Q1", "Q2", "Q3"]`
- Date arrays: `["2025-01", "2025-02"]` (ISO format)
- Boolean arrays: `[true, false, true]`

**Formulas:**

- Row-wise: `=revenue - expenses` (applied to each element)
- Aggregation: `=SUM(revenue)` (single result)

**Scalars (v0.2.0 compatible):**

- Must have `value` and `formula` keys
- Value: number or null
- Formula: string starting with `=` or null

### Common Errors

**❌ Mixed-type array:**

```yaml
revenue: [100, "Q2", 150]  # ERROR: Number and String mixed
```

**❌ Missing required keys:**

```yaml
assumptions:
  tax_rate:
    value: 0.25
    # ERROR: Missing 'formula' key
```

**❌ Invalid formula pattern:**

```yaml
total: "SUM(revenue)"  # ERROR: Missing '=' prefix
```

**✅ Correct:**

```yaml
revenue: [100, 120, 150]  # Homogeneous number array
total: "=SUM(revenue)"    # Valid aggregation formula
```

## Schema Development

### Updating the Schema

After modifying `forge-v1.0.schema.json`:

1. Validate the schema itself:

```bash
jsonschema --check-schema schema/forge-v1.0.schema.json
```

2. Test against example files:

```bash
jsonschema -i test-data/v1.0/saas_unit_economics.yaml schema/forge-v1.0.schema.json
```

3. Commit and push to update the public URL

### Schema Versioning

Each Forge version has its own schema:

- `forge-v0.2.schema.json` - v0.2.0 scalar model
- `forge-v1.0.schema.json` - v1.0.0 array model
- `forge-v2.0.schema.json` - Future versions

## Resources

- [JSON Schema Official Site](https://json-schema.org/)
- [Understanding JSON Schema](https://json-schema.org/understanding-json-schema/)
- [VSCode YAML Extension](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml)
- [JSON Schema Validator (Online)](https://www.jsonschemavalidator.net/)

## Examples

See `test-data/v1.0/` for complete examples:

- `saas_unit_economics.yaml` - Cohort analysis
- `quarterly_pl.yaml` - P&L statement
- `budget_vs_actual.yaml` - Variance analysis

All examples include schema references and pass validation.
