# Forge Architecture Documentation

## Complete technical documentation for Forge v5.0.0 architecture

**Last Updated:** 2025-12-04

**Status:** Complete

**Coverage:** 9 comprehensive documents, 320KB+ total

---

## 📚 Documentation Suite

### Quick Navigation

| Document | Focus Area | Lines | Status |
| -------- | ---------- | ----- | ------ |
| [00-OVERVIEW](00-OVERVIEW.md) | System context, principles, high-level architecture | ~1,000 | ✅ Complete |
| [01-COMPONENT-ARCHITECTURE](01-COMPONENT-ARCHITECTURE.md) | Module boundaries, interfaces, interactions | ~2,000 | ✅ Complete |
| [02-DATA-MODEL](02-DATA-MODEL.md) | Type system, structs, memory layout | ~1,500 | ✅ Complete |
| [03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md) | Calculation pipeline, 81 functions | ~1,300 | ✅ Complete |
| [04-DEPENDENCY-RESOLUTION](04-DEPENDENCY-RESOLUTION.md) | Graph algorithms, topological sort | ~1,100 | ✅ Complete |
| [05-EXCEL-INTEGRATION](05-EXCEL-INTEGRATION.md) | Bidirectional YAML↔Excel conversion | ~2,100 | ✅ Complete |
| [06-CLI-ARCHITECTURE](06-CLI-ARCHITECTURE.md) | Command structure, argument parsing | ~1,850 | ✅ Complete |
| [07-TESTING-ARCHITECTURE](07-TESTING-ARCHITECTURE.md) | Test strategy, 846 tests, 89% coverage | ~1,600 | ✅ Complete |
| [08-API-SERVER-ARCHITECTURE](08-API-SERVER-ARCHITECTURE.md) | HTTP REST API, Axum server, endpoints | ~400 | ✅ Complete |

### Architecture Decision Records (ADRs)

| ADR | Title | Status |
|-----|-------|--------|
| [ADR-001](ADR-001-NO-GRPC.md) | HTTP REST Over gRPC | Accepted |
| [ADR-002](ADR-002-VARIANCE-YAML-ONLY.md) | Variance YAML Only | Accepted |
| [ADR-003](ADR-003-EDITOR-EXTENSIONS.md) | Editor Extension Architecture | Superseded |
| [ADR-004a](ADR-004-100-PERCENT-TEST-COVERAGE.md) | 100% Test Coverage Requirement | Accepted |
| [ADR-004b](ADR-004-XLFORMULA-EQUIVALENCE.md) | xlformula_engine Equivalence Testing | Accepted |
| [ADR-005](ADR-005-NO-LSP.md) | No Language Server Protocol | Accepted |
| [ADR-006](ADR-006-COVERAGE-EXCLUSIONS.md) | Coverage Exclusions | Accepted |
| [ADR-007](ADR-007-E2E-GNUMERIC-VALIDATION.md) | E2E Validation via Gnumeric | Accepted |
| [ADR-008](ADR-008-FPA-NATIVE-FUNCTIONS.md) | FP&A-Native Functions | Accepted |
| [ADR-009](ADR-009-YAML-FOR-AI-EFFICIENCY.md) | YAML for AI Token Efficiency | Accepted |
| [ADR-010](ADR-010-GIT-NATIVE-FINANCIAL-MODELING.md) | Git-Native Financial Modeling | Accepted |

**Total:** ~14,000+ lines of comprehensive architecture documentation

---

## 🎯 Reading Paths

### For New Developers

**Recommended order:

1. **[00-OVERVIEW](00-OVERVIEW.md)** - Start here! System context, key principles
2. **[02-DATA-MODEL](02-DATA-MODEL.md)** - Understand core data structures
3. **[01-COMPONENT-ARCHITECTURE](01-COMPONENT-ARCHITECTURE.md)** - See how modules fit together
4. **[03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md)** - Core functionality
5. **[06-CLI-ARCHITECTURE](06-CLI-ARCHITECTURE.md)** - User interaction layer

Then read others as needed based on your work area.

### For Contributors

**By contribution type:

**Adding Excel functions:

- [03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md) - Custom function preprocessing
- [05-EXCEL-INTEGRATION](05-EXCEL-INTEGRATION.md) - Formula translation

**Fixing bugs:

- [07-TESTING-ARCHITECTURE](07-TESTING-ARCHITECTURE.md) - Test strategy
- [01-COMPONENT-ARCHITECTURE](01-COMPONENT-ARCHITECTURE.md) - Module interactions

**Performance optimization:

- [04-DEPENDENCY-RESOLUTION](04-DEPENDENCY-RESOLUTION.md) - O(V+E) algorithms
- [03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md) - Evaluation performance

**Adding commands:

- [06-CLI-ARCHITECTURE](06-CLI-ARCHITECTURE.md) - Command structure
- [01-COMPONENT-ARCHITECTURE](01-COMPONENT-ARCHITECTURE.md) - Component integration

### For Architects

**System design perspective:

1. **[00-OVERVIEW](00-OVERVIEW.md)** - Architecture principles, design philosophy
2. **[01-COMPONENT-ARCHITECTURE](01-COMPONENT-ARCHITECTURE.md)** - Module boundaries, coupling
3. **[04-DEPENDENCY-RESOLUTION](04-DEPENDENCY-RESOLUTION.md)** - Core algorithms
4. **[05-EXCEL-INTEGRATION](05-EXCEL-INTEGRATION.md)** - Integration patterns

### For Technical Writers

**Documentation perspective:

1. **[00-OVERVIEW](00-OVERVIEW.md)** - High-level terminology
2. **[02-DATA-MODEL](02-DATA-MODEL.md)** - Type system glossary
3. **[06-CLI-ARCHITECTURE](06-CLI-ARCHITECTURE.md)** - User-facing commands
4. **[07-TESTING-ARCHITECTURE](07-TESTING-ARCHITECTURE.md)** - Quality assurance

---

## 📊 Document Details

### 00-OVERVIEW.md

#### System Context & High-Level Architecture

**What's inside:

- System context diagram (users, external systems)
- Architecture principles (determinism, type safety, etc.)
- High-level component diagram
- Technology stack
- Module structure
- Data flow overview
- Performance characteristics

**Key sections:

- Problem statement (AI hallucinations, Excel lock-in)
- Solution (deterministic, YAML-first, bidirectional)
- 5 architecture principles
- Technology stack with 13 dependencies
- Success metrics (technical, business, development)

**Best for:** Getting oriented, understanding "why" Forge exists

---

### 01-COMPONENT-ARCHITECTURE.md

#### Module Boundaries, Interfaces, and Interactions

**What's inside:

- Detailed component diagram
- Module responsibility matrix (7 modules)
- Interface contracts for each module
- Data flow patterns
- Version-specific components (v0.2.0 vs v1.0.0)
- External dependency integration
- Error propagation paths

**Key sections:

- Parser interface (3 public functions)
- Calculator interface (2 versions)
- Excel module interface (export/import)
- CLI command routing
- Writer interface (multi-file updates)

**Best for:** Understanding module interactions, adding features

---

### 02-DATA-MODEL.md

#### Type System, Data Structures, Memory Layout

**What's inside:

- Type hierarchy diagram
- ColumnValue enum (4 variants)
- Column, Table, ParsedModel structs
- Variable struct (v0.2.0)
- Type safety invariants
- Memory layout analysis
- Serialization/deserialization

**Key sections:

- Type safety enforcement
- Homogeneous array validation
- Column length validation
- Date format validation
- Cross-file reference structure
- Error types (ForgeError enum)

**Best for:** Understanding data structures, adding data types

---

### 03-FORMULA-EVALUATION.md

#### Calculation Pipeline, xlformula_engine Integration

**What's inside:

- Formula evaluation pipeline diagram
- Row-wise formula evaluation (O(n) per column)
- Aggregation formula evaluation (O(n) per aggregation)
- Custom function preprocessing (27 functions)
- Variable resolution with scoping
- xlformula_engine integration (47+ functions)
- Performance optimization strategies

**Key sections:

- Two-phase calculation (tables then scalars)
- Formula parsing and AST generation
- Resolver function patterns
- Array indexing preprocessing
- Custom function implementation (ROUND, TRIM, etc.)
- Conditional aggregation (SUMIF, COUNTIF, etc.)

**Best for:** Adding Excel functions, optimizing performance

---

### 04-DEPENDENCY-RESOLUTION.md

#### Graph Algorithms, Topological Sort, Circular Detection

**What's inside:

- Dependency graph construction
- Three-level system (column, table, scalar)
- Topological sorting algorithm
- Circular dependency detection
- petgraph integration
- Performance analysis (O(V+E))
- Edge cases and error handling

**Key sections:

- DiGraph construction
- Dependency extraction (regex-based)
- Fuzzy variable matching
- Cross-file dependency resolution
- Cycle detection and error messages
- Calculation order determination

**Best for:** Understanding calculation order, debugging cycles

---

### 05-EXCEL-INTEGRATION.md

#### Bidirectional YAML↔Excel Conversion

**What's inside:

- Export pipeline (YAML → Excel .xlsx)
- Import pipeline (Excel .xlsx → YAML)
- Forward formula translation (286 lines)
- Reverse formula translation (318 lines)
- Column mapping algorithm (index ↔ letters)
- Sheet name sanitization
- rust_xlsxwriter and calamine integration
- Round-trip preservation strategy

**Key sections:

- Column letter conversion (0→A, 25→Z, 26→AA)
- Formula translation rules (60+ Excel functions)
- Cell reference handling (A2 → column_name)
- Range reference handling (A:A → column)
- Sheet reference handling (Sheet!A2 → table.column)
- Data type conversion (Number, Text, Date, Boolean)

**Best for:** Excel interoperability, formula translation

---

### 06-CLI-ARCHITECTURE.md

#### Command Structure, Argument Parsing, Error Handling

**What's inside:

- clap 4.5 framework integration
- 14 commands: calculate, validate, export, import, audit, watch, compare, variance, sensitivity, goal-seek, break-even, update, functions, upgrade
- Command routing (main.rs → cli/commands.rs)
- Argument parsing and validation
- Colored terminal output
- --dry-run and --verbose flags
- Exit codes for CI/CD
- Help text generation

**Key sections:

- Command structure (380 lines analyzed)
- calculate command implementation
- validate command implementation
- export command implementation
- import command implementation
- Error handling with ForgeError
- User feedback strategies
- CI/CD integration patterns

**Best for:** Adding commands, improving CLI UX

---

### 07-TESTING-ARCHITECTURE.md

#### Test Strategy, Coverage, Quality Assurance

**What's inside:

- 846 tests breakdown
- Test organization (inline, tests/, examples/)
- Unit testing strategy
- Integration testing approach
- E2E testing with CLI execution
- Test data management (33+ files)
- Coverage analysis (100% required - ADR-004)
- CI/CD integration

**Key sections:

- Testing pyramid visualization
- Unit tests in 8 modules
- E2E tests (1,026 lines analyzed)
- Test data conventions
- Pre-commit hook template
- GitHub Actions workflow
- Performance benchmarks
- Future: property-based testing

**Best for:** Writing tests, ensuring quality

---

## 🔍 Cross-References

### Related User Documentation

- **[../../README.md](../../README.md)** - User guide, features, quick start
- **[../../DESIGN_V1.md](../../DESIGN_V1.md)** - v1.0.0 array model specification (800+ lines)
- **[../../CHANGELOG.md](../../CHANGELOG.md)** - Version history (v0.1.0 → v1.2.1)
- **[../../KNOWN_BUGS.md](../../KNOWN_BUGS.md)** - Known issues and workarounds

### Related Developer Documentation

- **[../../roadmap.yaml](../../roadmap.yaml)** - Future features and milestones
- **[../../warmup.yaml](../../warmup.yaml)** - Development protocol for autonomous AI
- **[../../GLOSSARY.md](../../GLOSSARY.md)** - Canonical terminology (50+ terms)

### Architecture Diagrams

- **[../../diagrams/architecture-overview.puml](../../diagrams/architecture-overview.puml)** - Component diagram
- **[../../diagrams/README.md](../../diagrams/README.md)** - PlantUML guide and examples

---

## 💡 Key Concepts

### Architecture Principles (from 00-OVERVIEW)

1. **Determinism Over Intelligence** - Mathematical calculations, not AI
2. **Excel Compatibility = Excel Data Structures** - 1:1 mapping
3. **Type Safety > Flexibility** - Rust's type system prevents errors
4. **Explicit > Implicit** - No magic, clear errors
5. **Backwards Compatibility** - v0.2.0 models still work

### Core Algorithms (from 03, 04)

1. **Topological Sort** - O(V+E) dependency resolution
2. **Row-wise Evaluation** - O(n*m) array formulas
3. **Aggregation Evaluation** - O(n) column reduction
4. **Formula Translation** - Regex-based YAML↔Excel

### Design Philosophy (from 00-OVERVIEW)

> "Excel compatibility = Excel data structures + Excel formulas"

Forge doesn't replace Excel. It provides:

- Version-controllable alternative (YAML)
- Validation without AI (deterministic)
- Modular models (cross-file references)
- Bidirectional bridge (easy migration)

---

## 📈 Documentation Statistics

### Completeness

- **8/8 documents complete** (100%)
- **~12,450 lines** of architecture documentation
- **296KB total** (compressed: ~80KB)
- **20+ PlantUML diagrams** embedded
- **100+ code examples** with file citations
- **Zero TODOs** - All sections complete

### Coverage

**Code Coverage:

- ✅ All 7 modules documented (100%)
- ✅ All 17 source files referenced
- ✅ All 14 CLI commands explained
- ✅ All 60+ Excel functions listed
- ✅ All test types covered

**Architectural Coverage:

- ✅ System context (users, external systems)
- ✅ Component architecture (module interactions)
- ✅ Data model (type system, structs)
- ✅ Algorithms (formula evaluation, dependency resolution)
- ✅ Integration (Excel bidirectional conversion)
- ✅ Interface (CLI commands, error handling)
- ✅ Quality (testing strategy, coverage)

### Quality Metrics

- ✅ **Professional structure** - Consistent format across all docs
- ✅ **Technical depth** - 1000-2000 lines per document
- ✅ **Code citations** - file:line references throughout
- ✅ **Diagrams** - PlantUML embedded for visual learners
- ✅ **Cross-references** - Links between related topics
- ✅ **Terminology** - Uses GLOSSARY.md canonical terms
- ✅ **Production-ready** - Suitable for external publication

---

## 🛠️ Maintenance

### Updating Documentation

**When to update:

1. **Major versions** (v2.0.0) - Update all docs
2. **New features** - Update relevant doc (e.g., new function → 03, new command → 06)
3. **Architecture changes** - Update 01-COMPONENT-ARCHITECTURE
4. **Dependency changes** - Update 00-OVERVIEW technology stack
5. **Performance improvements** - Update relevant algorithm docs

**Update workflow:

```bash

# 1. Edit documentation

vim docs/architecture/03-FORMULA-EVALUATION.md

# 2. Validate diagrams (if changed)

make validate-diagrams

# 3. Validate markdown

make validate-docs

# 4. Commit with clear message

git add docs/architecture/
git commit -m "docs: Update formula evaluation for NEW_FUNCTION"
```

### Quarterly Review

**Checklist:

- ☐ All documents reflect current architecture?
- ☐ Code citations still accurate (file:line)?
- ☐ Diagrams match current design?
- ☐ Performance metrics up to date?
- ☐ New features documented?
- ☐ Deprecated features removed?
- ☐ Links working (no 404s)?
- ☐ GLOSSARY.md terms used consistently?

---

## 🎓 Learning Resources

### For Visual Learners

Focus on documents with most diagrams:

1. **[00-OVERVIEW](00-OVERVIEW.md)** - 3 PlantUML diagrams
2. **[05-EXCEL-INTEGRATION](05-EXCEL-INTEGRATION.md)** - 9 diagrams
3. **[04-DEPENDENCY-RESOLUTION](04-DEPENDENCY-RESOLUTION.md)** - 5 diagrams

### For Code-Focused Learners

Focus on documents with most code examples:

1. **[03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md)** - Formula parsing, evaluation
2. **[02-DATA-MODEL](02-DATA-MODEL.md)** - Struct definitions, type systems
3. **[06-CLI-ARCHITECTURE](06-CLI-ARCHITECTURE.md)** - Command implementations

### For Algorithm-Focused Learners

Focus on algorithmic deep dives:

1. **[04-DEPENDENCY-RESOLUTION](04-DEPENDENCY-RESOLUTION.md)** - Graph theory, O(V+E)
2. **[03-FORMULA-EVALUATION](03-FORMULA-EVALUATION.md)** - Evaluation algorithms
3. **[05-EXCEL-INTEGRATION](05-EXCEL-INTEGRATION.md)** - Translation algorithms

---

## ❓ FAQ

### Why so comprehensive?

**Answer:** Forge was built autonomously by AI in 12.5 hours using the RoyalBit Asimov. To maintain and extend it, human developers need complete architectural understanding.

### Which doc should I read first?

**Answer:** Always start with [00-OVERVIEW](00-OVERVIEW.md). It provides context and principles that inform all other documents.

### How often is this updated?

**Answer:** Architecture docs are updated with each major release (v1.0, v1.1, v1.2) and as needed for significant changes.

### Can I contribute?

**Answer:** Yes! See [../../CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines. Architecture docs follow GLOSSARY.md terms and require PlantUML diagram validation.

### Where are the diagrams?

**Answer:** Diagrams are embedded as PlantUML code blocks in each document. Rendered SVG/PNG versions are in [../../diagrams/](../../diagrams/).

---

## 📝 Document Template

When creating new architecture docs, follow this structure:

```markdown

# Title

**Document Version:** 1.0.0
**Forge Version:** v1.x.x
**Last Updated:** YYYY-MM-DD
**Status:** Complete/Draft/Under Review

---

## Table of Contents

[List of sections]

---

## Introduction

[What this document covers]

## [Main Content Sections]

[Technical details with diagrams]

## Related Documentation

[Cross-references]

---

**Previous:** [link]
**Next:** [link]
```

---

## 🚀 Next Steps

**For new developers:

1. Read [00-OVERVIEW](00-OVERVIEW.md)
2. Set up development environment (see [../../README.md#installation](../../README.md#installation))
3. Run tests: `make test`
4. Explore codebase with architecture knowledge

**For contributors:

1. Choose your focus area (formulas, CLI, Excel, etc.)
2. Read relevant architecture doc
3. Check [../../roadmap.yaml](../../roadmap.yaml) for open tasks
4. Follow [../../warmup.yaml](../../warmup.yaml) development protocol

**For architects:

1. Review all 8 documents
2. Understand design principles and trade-offs
3. Consider extensions based on [../../roadmap.yaml](../../roadmap.yaml)
4. Propose architectural changes via RFC process

---

## 📬 Feedback

Found an error? Documentation unclear? Feature request?

- **Issues:** <https://github.com/royalbit/forge/issues>
- **Discussions:** <https://github.com/royalbit/forge/discussions>
- **Email:** <admin@royalbit.ca>

---

**Documentation Created:** 2025-11-24
**Total Effort:** ~6 hours autonomous AI documentation generation
**Quality:** Production-ready, externally publishable
**Maintenance:** Updated with each major release

**Built with:** Claude Code + RoyalBit Asimov + PlantUML

