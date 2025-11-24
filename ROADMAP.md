# Forge Roadmap

## v1.0.0 - Current Release (2025-11-24)

### Core Features
- ✅ Column-based array model (v1.0.0)
- ✅ Formula calculation engine
- ✅ Cross-file references with `@alias.var` syntax
- ✅ Excel export with formula translation
- ✅ Excel import with reverse formula translation
- ✅ 60+ Excel function support (IFERROR, SUMIF, VLOOKUP, etc.)
- ✅ Multi-table support (multiple Excel worksheets)
- ✅ Comprehensive testing (100 tests: unit + e2e)

### Commands
- `forge calculate` - Calculate formulas and update YAML files
- `forge validate` - Verify formulas and detect stale values
- `forge audit` - Audit formula dependencies and detect circular references
- `forge export` - Export YAML to Excel with formula translation
- `forge import` - Import Excel to YAML with reverse formula translation

---

## Known Limitations

### Import Format Issue
**Issue**: `forge import` produces verbose internal YAML format instead of user-friendly v1.0.0 array syntax.

**Current Behavior**:
```yaml
# Import produces this (verbose internal format):
version: V1_0_0
tables:
  financial_summary:
    name: financial_summary
    columns:
      revenue:
        name: revenue
        values: !Number
        - 100000
        - 120000
```

**Expected Behavior**:
```yaml
# Should produce this (user-friendly v1.0.0 format):
financial_summary:
  revenue: [100000, 120000]
```

**Impact**:
- Round-trip (YAML → Excel → YAML) doesn't preserve format
- Imported YAML is valid but harder to read/edit manually
- Parser accepts both formats, so functionality works

**Workaround**: Run `forge calculate` on imported file to normalize format (though this doesn't fully solve the issue yet).

**Priority**: Medium - functionality works, but UX could be improved

---

## v1.1.0 - Planned

### Import Format Fix
- [ ] Create user-friendly YAML formatter
- [ ] Use formatter in `forge import` command
- [ ] Ensure round-trip preserves original format
- [ ] Add tests for exact round-trip preservation

### Excel Enhancements
- [ ] Support for Excel charts/graphs metadata
- [ ] Support for cell formatting (colors, borders)
- [ ] Support for named ranges
- [ ] Better error messages for unsupported Excel features

### Testing Enhancements
- [ ] Create test .xlsx files in test-data/ for import testing
- [ ] Add calamine-based verification of Excel formulas in tests
- [ ] More edge case tests (empty sheets, large files, malformed data)
- [ ] Performance benchmarks

---

## v1.2.0 - Future

### Cross-Table Formula Support
- [ ] Enable formulas that reference other tables: `=@other_table.revenue * 0.1`
- [ ] Proper dependency ordering across tables
- [ ] Excel translation for cross-table references

### Advanced Excel Features
- [ ] Pivot table support
- [ ] Conditional formatting
- [ ] Data validation rules
- [ ] Excel macros/VBA (read-only metadata)

### Performance
- [ ] Parallel formula evaluation
- [ ] Incremental calculation (only recalc changed formulas)
- [ ] Lazy loading for large files

---

## v2.0.0 - Major Features

### Database Integration
- [ ] PostgreSQL connector
- [ ] MySQL connector
- [ ] SQLite connector
- [ ] SQL query as data source for tables

### Web Service
- [ ] REST API for formula calculation
- [ ] Web UI for editing tables
- [ ] Real-time collaboration

### Advanced DSL
- [ ] Custom functions (user-defined)
- [ ] Conditional logic (IF/ELSE blocks)
- [ ] Loops and iterations
- [ ] Type system improvements

---

## Contributing

Found a bug or have a feature request? Please open an issue on GitHub!

Limitations documented here help prioritize future development and set clear expectations for users.
