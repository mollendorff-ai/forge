# Glossary

**Purpose:** Single source of truth for canonical terminology used across all Forge documentation, code, and communications.

**Usage:** When writing or reviewing documentation, verify terminology matches this glossary. Inconsistent terms confuse readers and break trust.

---

## Version Notation

### v1.0.0 (version format)

**Correct:** v1.0.0 (lowercase v, dots)

**Wrong:** V1.0.0, version 1.0.0, v1_0_0, 1.0.0, v1-0-0

**Definition:** Semantic version notation for Forge releases. Always use lowercase 'v' prefix with dot-separated numbers.

**Usage:**

- ✅ "Forge v1.0.0 introduced array support"
- ✅ "v1.1.0 added 27 Excel functions"
- ❌ "Version 1.0.0" (too verbose)
- ❌ "v1_0_0" (underscores are for code enums)

**Code representation:** `ForgeVersion::V1_0_0` (enum uses underscores, docs use dots)

---

## Data Model Terms

### Column Array

**Correct:** column array (lowercase, two words)

**Wrong:** column-array, columnArray, array (ambiguous), column (too generic)

**Definition:** Homogeneous array of values that maps 1:1 to an Excel column. All elements must be the same type (Number, Text, Date, or Boolean).

**Usage:**

- ✅ "Each column array maps to one Excel column"
- ✅ "Column arrays must be homogeneous"
- ❌ "Arrays in Forge" (too generic - specify column array)

**Related terms:** table, row-wise formula, aggregation

---

### Table

**Correct:** table (lowercase when general, capitalized when referring to Rust `Table` struct)

**Wrong:** sheet, worksheet (those are Excel terms), data table, array table

**Definition:** YAML object containing multiple column arrays. Maps to one Excel sheet during export.

**Usage:**

- ✅ "The `quarterly_revenue` table contains four columns"
- ✅ "Tables can have row-wise and aggregation formulas"
- ❌ "The quarterly_revenue sheet" (sheet is Excel terminology)

**File representation:**

```yaml
table_name:
  column1: [1, 2, 3]
  column2: [4, 5, 6]
```

---

### Row-wise Formula

**Correct:** row-wise (hyphenated, lowercase)

**Wrong:** rowwise, row wise, element-wise, per-row

**Definition:** Formula applied independently to each row index, producing an array result of the same length as input columns.

**Usage:**

- ✅ "Row-wise formulas operate on corresponding elements"
- ✅ "profit: =revenue - expenses" (row-wise operation)
- ❌ "Element-wise operation" (correct but not our canonical term)

**Contrast with:** Aggregation formula (produces scalar from array)

---

### Aggregation Formula

**Correct:** aggregation formula (lowercase, two words)

**Wrong:** aggregate formula, reduction formula, summary formula

**Definition:** Formula that reduces a column array to a single scalar value (e.g., SUM, AVERAGE, MAX).

**Usage:**

- ✅ "Aggregation formulas like SUM reduce arrays to scalars"
- ✅ "total: =SUM(revenue)" (aggregation)
- ❌ "Summary formula" (not our canonical term)

**Functions:** SUM, AVERAGE, MAX, MIN, COUNT, PRODUCT, SUMIF, COUNTIF, AVERAGEIF, etc.

---

### Scalar

**Correct:** scalar (lowercase)

**Wrong:** discrete value, single value, constant, scalar value (redundant)

**Definition:** Single value (not an array). Used in v0.2.0 model and as results of aggregation formulas in v1.0.0.

**Usage:**

- ✅ "Scalars can be referenced in row-wise formulas (broadcasting)"
- ✅ "tax_rate: 0.25" (scalar)
- ❌ "Discrete value" (technically correct but not our term)

**v0.2.0 format:**

```yaml
tax_rate:
  value: 0.25
  formula: null
```

**v1.0.0 aggregation result:** `total: =SUM(revenue)` (produces scalar)

---

## Formula Terms

### Formula

**Correct:** formula (lowercase when general)

**Wrong:** expression, equation, calculation

**Definition:** String starting with `=` that defines a calculation using Excel-compatible syntax.

**Usage:**

- ✅ "Formulas must start with ="
- ✅ "=revenue * 0.9" (formula)
- ❌ "revenue * 0.9" (expression, not formula - missing =)

**Components:** Functions (SUM, IF), operators (+, -, *, /), references (column names, table.column)

---

### Cross-file Reference

**Correct:** cross-file reference (hyphenated)

**Wrong:** external reference, file reference, inter-file reference, @reference

**Definition:** Reference to a variable or column in another YAML file using the `@alias.variable` or `@alias.table.column` syntax.

**Usage:**

- ✅ "Cross-file references use the @ prefix"
- ✅ "=@pricing.table.revenue" (cross-file reference)
- ❌ "External variable reference" (too verbose)

**Syntax:** `@alias.variable` (v0.2.0), `@alias.table.column` (v1.0.0)

---

### Cross-table Reference

**Correct:** cross-table reference (hyphenated)

**Wrong:** inter-table reference, table reference

**Definition:** Reference to a column in a different table within the same file using `table.column` syntax.

**Usage:**

- ✅ "Use cross-table references to access other tables"
- ✅ "=pricing.revenue" (cross-table reference)
- ❌ "Other table reference" (not canonical)

**Syntax:** `table.column` or `table.column[index]`

---

## Type System

### Type-safe

**Correct:** type-safe (hyphenated)

**Wrong:** typesafe, type safe, strongly-typed

**Definition:** Property where type mismatches are caught at parse time, preventing runtime errors.

**Usage:**

- ✅ "Forge uses type-safe column arrays"
- ✅ "Type-safe validation prevents mixed-type arrays"
- ❌ "Strongly typed arrays" (correct concept, not our term)

---

### Homogeneous Array

**Correct:** homogeneous array (two words)

**Wrong:** homogenous array (common misspelling), uniform array, same-type array

**Definition:** Array where all elements have the same type. Required for all Forge column arrays.

**Usage:**

- ✅ "Column arrays must be homogeneous"
- ✅ "[100, 120, 150]" (homogeneous - all numbers)
- ❌ "[100, 'Q2', true]" (heterogeneous - INVALID)

**Validation:** Parser rejects heterogeneous arrays immediately.

---

## File Naming Conventions

### test-data/

**Correct:** test-data/ (hyphenated, lowercase)

**Wrong:** testdata/, test_data/, TestData/, tests/data/

**Definition:** Directory containing example YAML and Excel files for testing.

**Usage:**

- ✅ "test-data/v1.0/saas_unit_economics.yaml"
- ❌ "tests/saas_economics.yaml" (wrong directory)

**Subdirectories:** test-data/v0.2/, test-data/v1.0/

---

### v1.0/ (directory naming)

**Correct:** v1.0/ or v1.0.0/ (lowercase v, dots)

**Wrong:** v1_0/, v1-0/, V1.0/, version-1.0/

**Definition:** Directory naming for version-specific examples and schemas.

**Usage:**

- ✅ "test-data/v1.0/"
- ✅ "schema/forge-v1.0.schema.json"
- ❌ "schema/forge_v1_0_schema.json" (underscores)

---

## Command Names

### forge calculate

**Correct:** forge calculate (two words, lowercase)

**Wrong:** forge-calculate, calculate, recalculate

**Definition:** CLI command that evaluates formulas and updates calculated values in YAML files.

**Usage:**

- ✅ "Run `forge calculate model.yaml`"
- ✅ "The calculate command updates stale values"
- ❌ "`forge recalculate`" (not a real command)

---

### forge validate

**Correct:** forge validate (two words, lowercase)

**Wrong:** forge-validate, validate, check

**Definition:** CLI command that verifies formulas are correct and values match calculations, without modifying files.

**Usage:**

- ✅ "Use `forge validate` to check for stale values"
- ✅ "Validation detects inconsistencies"
- ❌ "`forge check`" (not a real command)

---

### forge export

**Correct:** forge export (two words, lowercase)

**Wrong:** forge-export, export-to-excel, to-excel

**Definition:** CLI command that converts YAML tables to Excel .xlsx files with formula translation.

**Usage:**

- ✅ "`forge export model.yaml output.xlsx`"
- ✅ "Export translates 60+ formula functions"
- ❌ "`forge to-excel`" (not a real command)

---

### forge import

**Correct:** forge import (two words, lowercase)

**Wrong:** forge-import, import-from-excel, from-excel

**Definition:** CLI command that converts Excel .xlsx files to YAML tables with reverse formula translation.

**Usage:**

- ✅ "`forge import input.xlsx output.yaml`"
- ✅ "Import preserves formulas and data"
- ❌ "`forge from-excel`" (not a real command)

---

## Technical Terms

### Zero hallucinations

**Correct:** zero hallucinations (lowercase, not hyphenated in prose)

**Wrong:** 0 hallucinations, no hallucinations, zero-hallucinations

**Definition:** Property where calculations are mathematically deterministic (not AI-predicted), producing no false or invented results.

**Usage:**

- ✅ "Forge guarantees zero hallucinations"
- ✅ "Deterministic calculations, not pattern matching"
- ❌ "No AI errors" (too vague)

**Context:** Contrasts with AI (ChatGPT, Claude, Copilot) which hallucinates numbers.

---

### Excel-compatible

**Correct:** Excel-compatible (hyphenated, capitalized Excel)

**Wrong:** excel-compatible, Excel compatible, Excel-like

**Definition:** Functions and formulas that match Microsoft Excel syntax and semantics exactly.

**Usage:**

- ✅ "47+ Excel-compatible functions"
- ✅ "Excel-compatible formula evaluation"
- ❌ "Excel-like functions" (implies approximate, not exact)

---

## Repository Terms

### forge (repository/project)

**Correct:** forge (lowercase), Forge (capitalized when starting sentence or as product name)

**Wrong:** FORGE, Forge-Calculator, forge-tool

**Definition:** The project and repository name. Also the CLI command name.

**Usage:**

- ✅ "The forge repository contains the calculator"
- ✅ "Forge is a YAML formula calculator"
- ✅ "Run `forge calculate`" (command)
- ❌ "The FORGE tool" (all caps only for emphasis)

**Crates.io name:** royalbit-forge (hyphenated)

---

### royalbit-forge

**Correct:** royalbit-forge (lowercase, hyphenated)

**Wrong:** royalbit_forge, RoyalBitForge, royalbit/forge

**Definition:** Cargo package name for Forge.

**Note:** Forge is proprietary R&D software. Not published to crates.io.

**Usage:**

- ✅ "Clone from github.com/royalbit/forge"
- ❌ "`cargo install royalbit-forge`" (not on crates.io)

**GitHub:** github.com/royalbit/forge (slash, not hyphen)

---

## Linting Terms

### markdownlint-cli2

**Correct:** markdownlint-cli2 (lowercase, hyphenated, includes version)

**Wrong:** markdownlint, markdown-lint, MarkdownLint

**Definition:** Tool for linting markdown files. Version 2 CLI (cli2) is the current recommended version.

**Usage:**

- ✅ "`npm install -g markdownlint-cli2`"
- ✅ "Run markdownlint-cli2 on all markdown files"
- ❌ "`markdownlint`" (older version, not recommended)

**Config file:** .markdownlint.json

---

### yamllint

**Correct:** yamllint (lowercase, no hyphen)

**Wrong:** yaml-lint, YAMLLint, yml-lint

**Definition:** Tool for linting YAML files, checking syntax and style.

**Usage:**

- ✅ "`pip install yamllint`"
- ✅ "yamllint validates YAML structure"
- ❌ "`yaml-lint`" (wrong tool name)

**Config file:** .yamllint

---

## Testing Terms

### E2E tests

**Correct:** E2E tests (uppercase, no periods)

**Wrong:** e2e tests, E2E-tests, end-to-end tests (spell out in prose, use E2E in headings/code)

**Definition:** End-to-end tests that validate full user workflows from CLI invocation to file output.

**Usage:**

- ✅ "33 E2E tests passing"
- ✅ "E2E test for export/import roundtrip"
- ✅ "End-to-end testing ensures commands work" (spelled out in prose is OK)

**Location:** tests/e2e_tests.rs

---

### Roundtrip test

**Correct:** roundtrip test (lowercase, one word)

**Wrong:** round-trip test, round trip test, RT test

**Definition:** Test that verifies YAML → Excel → YAML produces identical results.

**Usage:**

- ✅ "Roundtrip tests ensure lossless conversion"
- ✅ "YAML → Excel → YAML roundtrip"
- ❌ "Round-trip validation" (hyphen not canonical)

---

## Diagram Terms

### PlantUML

**Correct:** PlantUML (capitalized P, capitalized UML, no space)

**Wrong:** plantuml, Plant UML, PlantUml, PLANTUML

**Definition:** Open-source tool for creating UML and architecture diagrams from plain text descriptions.

**Usage:**

- ✅ "Create diagrams with PlantUML"
- ✅ "PlantUML server validation"
- ❌ "plantuml diagrams" (wrong capitalization)

**File extensions:** `.puml` or `.plantuml`

---

### .puml (file extension)

**Correct:** .puml (lowercase, dot prefix)

**Wrong:** .PUML, .Puml, .plantuml (use .puml as primary)

**Definition:** File extension for PlantUML diagram source files.

**Usage:**

- ✅ `diagrams/architecture-overview.puml`
- ✅ "Validate all .puml files"
- ⚠️ `.plantuml` (acceptable but .puml preferred)

**Naming:** Use kebab-case: `data-model-v1.0.puml`

---

### Sequence diagram

**Correct:** sequence diagram (lowercase, two words)

**Wrong:** sequence-diagram, sequenceDiagram, Sequence Diagram (unless title case)

**Definition:** Diagram showing interactions between components over time, typically for workflows or protocols.

**Usage:**

- ✅ "Create a sequence diagram for the calculate workflow"
- ✅ "Sequence diagrams show time-ordered interactions"
- ❌ "Sequence-diagram validation" (no hyphen)

**When to use:** User workflows, API call sequences, multi-step processes

---

### Component diagram

**Correct:** component diagram (lowercase, two words)

**Wrong:** component-diagram, componentDiagram

**Definition:** Diagram showing system architecture, modules, and their relationships.

**Usage:**

- ✅ "Component diagram of Forge architecture"
- ✅ "Use component diagrams for high-level structure"

**When to use:** System architecture, module dependencies, integration points

---

### Class diagram

**Correct:** class diagram (lowercase, two words)

**Wrong:** class-diagram, classDiagram, UML class diagram (redundant - all class diagrams are UML)

**Definition:** Diagram showing data structures, types, and their relationships.

**Usage:**

- ✅ "Class diagram of the v1.0.0 data model"
- ✅ "Class diagrams document type hierarchies"

**When to use:** Data models, type systems, inheritance relationships

---

### Flowchart

**Correct:** flowchart (lowercase, one word)

**Wrong:** flow-chart, flow chart, FlowChart

**Definition:** Diagram showing decision logic, algorithms, or process flows.

**Usage:**

- ✅ "Flowchart for formula evaluation logic"
- ✅ "Use flowcharts for complex decision trees"
- ❌ "Flow-chart diagram" (flowchart is one word)

**When to use:** Decision logic, algorithms, conditional flows

---

### diagrams/ (directory)

**Correct:** diagrams/ (lowercase, no hyphen, trailing slash when referring to directory)

**Wrong:** Diagrams/, diagram/, DIAGRAMS/

**Definition:** Directory containing PlantUML diagram source files (.puml).

**Usage:**

- ✅ `diagrams/architecture-overview.puml`
- ✅ "Store all diagrams in diagrams/"
- ❌ `Diagrams/overview.puml` (wrong case)

**Structure:** Flat directory (no subdirectories yet)

---

## Style Consistency

### Zero tolerance

**Correct:** zero tolerance (lowercase, two words)

**Wrong:** zero-tolerance, 0 tolerance, ZERO TOLERANCE

**Definition:** Policy of allowing no errors or warnings (100% pass rate required).

**Usage:**

- ✅ "Zero tolerance for clippy warnings"
- ✅ "ZERO warnings" (emphasis is OK)
- ❌ "Zero-tolerance policy" (hyphen only as adjective: "zero-tolerance approach")

---

## When in Doubt

**Check order:**

1. Search this GLOSSARY.md
2. Search existing documentation (README, DESIGN_V1, roadmap)
3. Check code comments and struct names (src/)
4. Ask user or add new entry to GLOSSARY.md

**Adding new terms:**

- Follow alphabetical order within sections
- Include: Correct, Wrong, Definition, Usage, Examples
- Use same format as existing entries
- Commit with message: "docs: Add [term] to GLOSSARY.md"

---

**Last updated:** 2025-11-24
**Maintained by:** RoyalBit Inc.
**Version:** 1.0.0
