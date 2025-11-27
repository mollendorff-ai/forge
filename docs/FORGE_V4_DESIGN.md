# Forge v4.0 Design: Rich Metadata Schema

> Unifying Forge calculation engine with Mouvify's business model format

## Motivation

Mouvify's business YAML models have 850+ formulas with rich metadata that Forge can't currently parse. The current Forge v1.0 schema is a subset of what's needed for enterprise financial modeling.

**Current Forge v1.0:**
```yaml
revenue: [100, 200, 300]
profit: "=revenue * 0.3"
```

**Mouvify's richer format:**
```yaml
revenue:
  value: [100, 200, 300]
  unit: "CAD"
  notes: "Monthly revenue projection"
  source: "market_research.yaml"
  validation_status: "VALIDATED"
```

## Design Goals

1. **Backward compatible** - v1.0 models still work unchanged
2. **Rich metadata** - Support unit, notes, source, validation_status
3. **Cross-file references** - `@sources.platform.take_rate` syntax
4. **Excel export** - Metadata preserved as comments/formatting
5. **Validation** - Check sources exist, units consistent

## Schema v4.0

### Scalar Values

```yaml
# Simple (v1.0 compatible)
price: 100
margin: 0.3

# Rich (v4.0)
price:
  value: 100
  unit: "CAD"
  notes: "Base price per unit"

margin:
  value: 0.3
  unit: "%"
  source: "industry_benchmarks.yaml"
```

### Formulas

```yaml
# Simple (v1.0 compatible)
profit: "=revenue * margin"

# Rich (v4.0)
profit:
  formula: "=revenue * margin"
  unit: "CAD"
  notes: "Gross profit calculation"
  source: "assumptions.yaml:margin"
  validation_status: "VALIDATED"
```

### Tables

```yaml
# Simple (v1.0 compatible)
sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
  profit: "=revenue * 0.3"

# Rich (v4.0)
sales:
  _metadata:
    description: "Monthly sales projections"
    source: "market_research.yaml"
  month:
    value: ["Jan", "Feb", "Mar"]
    unit: "month"
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
    notes: "Based on 20% MoM growth"
  profit:
    formula: "=revenue * margin"
    unit: "CAD"
```

### Cross-File References

```yaml
# Include external files
_includes:
  - file: "data_sources.yaml"
    as: "sources"

# Reference included data
revenue:
  formula: "=base_units * @sources.pricing.unit_price"
  notes: "Using centralized pricing from data_sources.yaml"
```

## Metadata Fields

| Field | Type | Description |
|-------|------|-------------|
| `value` | any | The data value (number, string, array) |
| `formula` | string | Calculation formula (starts with =) |
| `unit` | string | Unit of measurement (CAD, %, count, days, ratio) |
| `notes` | string | Human-readable explanation |
| `source` | string | Where data came from (file:field or URL) |
| `validation_status` | string | VALIDATED, PROJECTED, ESTIMATED |
| `last_updated` | string | ISO date of last update |

## Implementation Plan

### Phase 1: Parser Enhancement
- Detect rich vs simple format per field
- Extract value/formula from rich objects
- Store metadata in ParsedModel

### Phase 2: Validation Enhancement
- Validate units are consistent in formulas
- Check source references exist
- Warn on stale validation_status

### Phase 3: Excel Export Enhancement
- Add cell comments from notes field
- Format cells based on unit (currency, percentage)
- Create "Sources" sheet with traceability

### Phase 4: Cross-File References
- Parse `_includes` directive
- Resolve `@namespace.field` references
- Circular dependency detection across files

## Migration Path

1. **v3.x users**: No changes required, models work as-is
2. **Mouvify models**: Run `forge migrate --to-v4` to validate compatibility
3. **New models**: Use rich format for better documentation

## Example: Mouvify Model Validation

After v4.0, this Mouvify model will validate:

```yaml
_forge_version: "4.0.0"

_includes:
  - file: "data_sources.yaml"
    as: "sources"

market_sizing:
  montreal_nightlife:
    value: 300000000
    unit: "CAD"
    notes: "Montreal nightlife market annual revenue"
    source: "https://mtl.org/tourism-stats"
    validation_status: "VALIDATED"

  target_capture:
    value: 0.05
    unit: "%"
    notes: "Year 1 market capture rate (conservative)"

  year1_revenue:
    formula: "=montreal_nightlife * target_capture * @sources.platform.take_rate"
    unit: "CAD"
    notes: "Projected Year 1 revenue from Montreal market"
```

## Success Criteria

- [ ] All 850+ Mouvify formulas validate without errors
- [ ] Excel export includes metadata as comments
- [ ] Backward compatible with all v1.0-v3.x models
- [ ] Cross-file references resolve correctly
- [ ] Unit consistency warnings catch errors

## Timeline

- **v4.0-alpha**: Parser recognizes rich format (2 sessions)
- **v4.0-beta**: Full metadata support + Excel comments (2 sessions)
- **v4.0**: Cross-file references + validation (2 sessions)

---

*Designed by Claude (Opus 4.5) - Principal Engineer*
*November 2025*
