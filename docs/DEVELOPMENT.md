# Development Guide

## Library & Dependency Management

**Philosophy:** Don't reinvent the wheel - leverage FOSS ecosystem.

### Mandatory FOSS Research

**BEFORE writing ANY complex code (>50 lines), you MUST:**

1. Search crates.io AND web for existing solutions
2. Document search results and decision
3. Justify if choosing to write from scratch

**FAILURE TO RESEARCH = WASTED EFFORT**

### Definition of Complex

- Any new parser or formatter
- Any file format handling (Excel, CSV, JSON Schema, etc.)
- Any mathematical/formula processing
- Any graph/tree algorithms
- Any serialization/deserialization
- Any HTTP/API client or server code
- Any authentication/authorization
- Anything that would take >2 hours to write

### Web Search Protocol

- Always include year: Search `rust [topic] [current year]` to get current results
- Search queries:
  - `rust [functionality] crate [current year]`
  - `best rust [topic] library`
  - `rust [problem] solution`
  - crates.io direct search
- ALWAYS compare at least 3 options before deciding
- Add comment in code explaining why this library was chosen

### Before Implementing Complex Code

1. Web search: `rust [functionality] crate [current year]`
2. Search crates.io for relevant functionality
3. Check license compatibility (we use Proprietary, dependencies must be permissive)
4. Verify library is maintained (recent commits, active issues)
5. Check download counts and GitHub stars
6. Review documentation quality
7. Test with small example before integrating
8. Document your search and decision

### License Compatibility

Forge is proprietary. Dependencies must use permissive licenses that allow commercial use.

| Status | Licenses |
|--------|----------|
| **Allowed** | MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unlicense, CC0-1.0 |
| **Not Allowed** | GPL (any), AGPL, LGPL, CC-BY-NC, any copyleft |

### Evaluation Checklist

- [ ] License allows commercial use (MIT, Apache, BSD)
- [ ] Last commit within 6 months
- [ ] Has documentation
- [ ] No critical security issues
- [ ] Reasonable download count (or new but promising)
- [ ] Solves our problem completely (or mostly)

### Example Workflow

**Scenario:** Need to export to Excel

1. Search: `rust excel xlsx 2025` (include current year!)
2. Check top results: rust_xlsxwriter, calamine, umya-spreadsheet
3. Review licenses: rust_xlsxwriter (MIT), calamine (MIT)
4. Check maintenance: rust_xlsxwriter (active)
5. Read docs: rust_xlsxwriter has good examples
6. Decision: Use rust_xlsxwriter

**DON'T:** Write Excel export from scratch (thousands of lines!)

---

## Keeping Dependencies Updated

**Importance:** Security, bug fixes, performance, new features.

### When to Update

- Start of new implementation phase
- Before release
- When security advisory published
- When new feature needed from dependency

### How to Update

| Action | Command | Notes |
|--------|---------|-------|
| Check outdated | `cargo outdated` | Install: `cargo install cargo-outdated` |
| Update minor | `cargo update` | Safe - follows semver |
| Update all | `cargo upgrade` | Install: `cargo install cargo-edit`. Test thoroughly! |

### Update Major Versions

1. Review CHANGELOG for breaking changes
2. Update Cargo.toml version requirement
3. Run `cargo update`
4. Fix any breaking changes
5. Run full test suite

### Testing After Update

```bash
cargo test --release        # All tests must pass
cargo clippy --release -- -D warnings  # ZERO warnings
cargo build --release       # Verify compilation
```

### Critical Dependencies

- `xlformula_engine` - Excel formula engine
- `serde` / `serde_yaml` - YAML parsing
- `clap` - CLI parsing
- `petgraph` - Dependency graph
- `jsonschema` - JSON Schema validation

Check `Cargo.toml` for current versions.
