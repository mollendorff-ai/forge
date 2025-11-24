# Component Architecture

**Document Version:** 1.0.0
**Forge Version:** v1.1.2
**Last Updated:** 2025-11-24
**Status:** Complete

---

## Table of Contents

1. [Introduction](#introduction)
2. [Component Overview](#component-overview)
3. [Core Components](#core-components)
4. [Component Interactions](#component-interactions)
5. [Module Boundaries](#module-boundaries)
6. [Interface Contracts](#interface-contracts)
7. [Data Flow Patterns](#data-flow-patterns)
8. [Version-Specific Components](#version-specific-components)
9. [External Dependencies](#external-dependencies)
10. [Related Documentation](#related-documentation)

---

## Introduction

### Purpose

This document provides a comprehensive view of Forge's component architecture, detailing how the system's major components interact, their responsibilities, and the boundaries between them. Understanding this architecture is essential for:

- **Developers** extending Forge with new features
- **Maintainers** debugging issues or refactoring code
- **Architects** evaluating Forge's design decisions
- **Contributors** understanding where to make changes

### Architectural Philosophy

Forge follows a **layered architecture** with clear separation of concerns:

1. **CLI Layer** - User interaction and command routing
2. **Parser Layer** - YAML deserialization and validation
3. **Core Layer** - Formula evaluation and calculation
4. **Writer Layer** - YAML serialization and file updates
5. **Excel Layer** - Bidirectional Excel integration
6. **Types Layer** - Shared data structures and enums

This separation enables:

- Independent testing of each layer
- Easy addition of new formats (JSON, CSV, etc.)
- Version-specific implementations without breaking changes
- Clear error boundaries and recovery strategies

---

## Component Overview

### High-Level Component Diagram

```plantuml
@startuml component-architecture
!theme plain
title Forge Component Architecture - Detailed View

package "CLI Layer" {
  [main.rs] as main
  [cli/commands.rs] as commands
}

package "Parser Layer" {
  [parser/mod.rs] as parser
  [Version Detector] as version_detector
  [Schema Validator] as schema_validator
}

package "Core Layer" {
  [array_calculator.rs] as array_calc
  [calculator.rs] as scalar_calc
  [Dependency Resolver] as dep_resolver
  [Formula Evaluator] as formula_eval
}

package "Writer Layer" {
  [writer/mod.rs] as writer
  [YAML Serializer] as yaml_serializer
}

package "Excel Layer" {
  [excel/exporter.rs] as exporter
  [excel/importer.rs] as importer
  [formula_translator.rs] as translator
  [reverse_formula_translator.rs] as reverse_translator
}

package "Types Layer" {
  [types.rs] as types
  [error.rs] as errors
}

package "External Libraries" {
  [xlformula_engine] as xle
  [petgraph] as petgraph_lib
  [serde_yaml] as serde
  [rust_xlsxwriter] as xlsx_writer
  [calamine] as xlsx_reader
}

' CLI Layer connections
main --> commands : routes commands
commands --> parser : parse YAML
commands --> array_calc : calculate v1.0
commands --> scalar_calc : calculate v0.2
commands --> exporter : export to Excel
commands --> importer : import from Excel
commands --> writer : write YAML

' Parser Layer connections
parser --> version_detector : detect version
parser --> schema_validator : validate structure
parser --> types : create Model
parser --> serde : deserialize YAML

' Core Layer connections
array_calc --> dep_resolver : resolve dependencies
array_calc --> formula_eval : evaluate formulas
array_calc --> types : read/write data
scalar_calc --> dep_resolver : resolve dependencies
scalar_calc --> formula_eval : evaluate formulas

' Writer Layer connections
writer --> yaml_serializer : serialize data
writer --> serde : write YAML

' Excel Layer connections
exporter --> translator : translate formulas
exporter --> xlsx_writer : create workbook
importer --> reverse_translator : reverse translate
importer --> xlsx_reader : read workbook
translator --> types : read model data
importer --> types : create model

' External dependencies
dep_resolver --> petgraph_lib : topological sort
formula_eval --> xle : evaluate Excel functions

' Shared dependencies
array_calc --> errors
scalar_calc --> errors
parser --> errors
exporter --> errors
importer --> errors

note right of array_calc

#### v1.0.0 Calculator

  - 3,440 lines
  - Array operations
  - Row-wise formulas
  - Two-phase calculation

end note

note right of scalar_calc

#### v0.2.0 Calculator

  - 401 lines
  - Scalar operations
  - Cross-file references
  - Backwards compatible

end note

note bottom of parser

#### Dual-version support

  Auto-detects v0.2.0 vs v1.0.0
  Parses both formats
end note

@enduml
```text

### Component Responsibilities Matrix

| Component | Primary Responsibility | Lines of Code | Key Types |
|-----------|----------------------|---------------|-----------|
| **main.rs** | CLI entry point, argument parsing | 220 | `Cli`, `Commands` |
| **cli/commands.rs** | Command execution logic | 380 | - |
| **parser/mod.rs** | YAML parsing, version detection | 1,011 | `ParsedModel`, `ParsedYaml` |
| **types.rs** | Data structure definitions | 290 | `Table`, `Column`, `ColumnValue` |
| **array_calculator.rs** | v1.0.0 array calculations | 3,440 | `ArrayCalculator` |
| **calculator.rs** | v0.2.0 scalar calculations | 401 | `Calculator` |
| **writer/mod.rs** | YAML file updates | 300+ | - |
| **excel/exporter.rs** | YAML → Excel conversion | 218 | `ExcelExporter` |
| **excel/importer.rs** | Excel → YAML conversion | 400+ | `ExcelImporter` |
| **formula_translator.rs** | YAML formulas → Excel | 286 | `FormulaTranslator` |
| **reverse_formula_translator.rs** | Excel formulas → YAML | 300+ | `ReverseFormulaTranslator` |
| **error.rs** | Error types | 33 | `ForgeError`, `ForgeResult<T>` |

**Total:** ~7,436 lines of Rust code

---

## Core Components

### 1. CLI Layer (/home/rex/src/utils/forge/src/main.rs)

**Responsibility:** Entry point for all user interactions. Parses command-line arguments and routes to appropriate handlers.

**Key Technologies:**

- `clap` v4.5 - Declarative argument parsing
- Derive macros for automatic help generation

**Structure:**

```rust
// From: /home/rex/src/utils/forge/src/main.rs:1-220

#[derive(Parser)]


#[command(name = "forge")]

struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]

enum Commands {
    Calculate { file: PathBuf, dry_run: bool, verbose: bool },
    Audit { file: PathBuf, variable: String },
    Validate { file: PathBuf },
    Export { input: PathBuf, output: PathBuf, verbose: bool },
    Import { input: PathBuf, output: PathBuf, verbose: bool },
}
```text

**Interface:**

```text
Input:  Command-line arguments (String[])
Output: ForgeResult<()>
Errors: Propagated from command handlers
```text

**Design Decisions:**

1. **Why clap derive macros?**
   - Automatic help generation
   - Type-safe argument parsing
   - Self-documenting code

2. **Why separate Commands enum?**
   - Clear separation of concerns
   - Easy to add new commands
   - Enables command-specific validation

3. **Why ForgeResult return type?**
   - Consistent error handling across all commands
   - Automatic error propagation with `?` operator
   - User-friendly error messages

### 2. Command Handler Layer (/home/rex/src/utils/forge/src/cli/commands.rs)

**Responsibility:** Execute business logic for each command. Orchestrate interactions between parser, calculator, writer, and Excel components.

**Key Functions:**

```rust
// From: /home/rex/src/utils/forge/src/cli/commands.rs:24-181
pub fn calculate(file: PathBuf, dry_run: bool, verbose: bool) -> ForgeResult<()>
pub fn validate(file: PathBuf) -> ForgeResult<()>
pub fn export(input: PathBuf, output: PathBuf, verbose: bool) -> ForgeResult<()>
pub fn import(input: PathBuf, output: PathBuf, verbose: bool) -> ForgeResult<()>
pub fn audit(file: PathBuf, variable: String) -> ForgeResult<()>
```text

**Calculate Command Flow:**

```plantuml
@startuml calculate-command-flow
!theme plain
title Calculate Command - Internal Flow

start

:User executes\n**forge calculate model.yaml**;

:Parse command-line args\n(clap);

:Read YAML file;

:Detect version\n(v0.2.0 or v1.0.0);

if (Version?) then (v1.0.0)
  :Parse as array model;
  :Create ArrayCalculator;
  :Calculate tables\n(two-phase);
  :Calculate scalars;
  :Display results;
  note right
    v1.0.0 writer
    not yet implemented
  end note
else (v0.2.0)
  :Parse with includes;
  :Create Calculator;
  :Build dependency graph;
  :Topological sort;
  :Evaluate formulas;
  :Write ALL files\n(main + includes);
  :Display success;
endif

stop

@enduml
```text

**Design Patterns:**

1. **Version Dispatch Pattern**

   ```rust
   match model.version {
       ForgeVersion::V1_0_0 => /* ArrayCalculator */,
       ForgeVersion::V0_2_0 => /* Calculator */,
   }
```text

2. **Early Validation Pattern**

   ```rust
   if dry_run {
       println!("DRY RUN MODE");
   }
   // ... perform calculations ...
   if !dry_run {
       writer::update_all_yaml_files(...)?;
   }
```text

3. **Progressive Disclosure**
   - Verbose mode shows detailed steps
   - Non-verbose shows only results

### 3. Parser Layer (/home/rex/src/utils/forge/src/parser/mod.rs)

**Responsibility:** Parse YAML files into Forge's internal data structures. Detect version, validate schema, and handle includes.

**Key Functions:**

```rust
// From: /home/rex/src/utils/forge/src/parser/mod.rs:39-104
pub fn parse_model(path: &Path) -> ForgeResult<ParsedModel>
fn parse_v1_model(path: &Path, yaml: &Value) -> ForgeResult<ParsedModel>
fn parse_v0_model(path: &Path, yaml: &Value) -> ForgeResult<ParsedModel>
pub fn parse_yaml_with_includes(path: &Path) -> ForgeResult<ParsedYaml>
fn validate_against_schema(yaml: &Value) -> ForgeResult<()>
```text

**Version Detection Algorithm:**

```plantuml
@startuml version-detection
!theme plain
title Version Detection Algorithm

start

:Read YAML file;

if (Has "_forge_version" field?) then (yes)
  if (Starts with "1.0"?) then (yes)
    :Return **V1_0_0**;
    stop
  else (no)
    :Continue checking;
  endif
endif

if (Has "includes" field?) then (yes)
  :Return **V0_2_0**;
  note right
    includes: indicates
    cross-file references
    (v0.2.0 feature)
  end note
  stop
endif

if (Has "tables" with "columns"?) then (yes)
  :Return **V1_0_0**;
  stop
endif

if (Contains array values?) then (yes)
  :Return **V1_0_0**;
  stop
endif

:Return **V0_2_0**\n(default for backwards compatibility);

stop

@enduml
```text

**Schema Validation:**

```rust
// From: /home/rex/src/utils/forge/src/parser/mod.rs:106-131
fn validate_against_schema(yaml: &Value) -> ForgeResult<()> {
    // Load embedded JSON Schema
    let schema_str = include_str!("../../schema/forge-v1.0.schema.json");
    let schema_value: serde_json::Value = serde_json::from_str(schema_str)?;

    // Compile schema
    let compiled_schema = JSONSchema::compile(&schema_value)?;

    // Convert YAML → JSON and validate
    let json_value: serde_json::Value = serde_json::to_value(yaml)?;
    compiled_schema.validate(&json_value)?;

    Ok(())
}
```text

**Design Decisions:**

1. **Why auto-detect version instead of requiring explicit marker?**
   - Backwards compatibility with v0.2.0 files
   - Better user experience (no boilerplate)
   - Fallback to v0.2.0 ensures old files never break

2. **Why validate against JSON Schema?**
   - Machine-readable format specification
   - Tooling support (IDEs, validators)
   - Clear error messages with field paths

3. **Why separate parse_v1_model and parse_v0_model?**
   - Different data structures (arrays vs scalars)
   - Independent evolution of each version
   - Clear error boundaries

### 4. Type System (/home/rex/src/utils/forge/src/types.rs)

**Responsibility:** Define core data structures used throughout Forge. Provide type-safe abstractions for tables, columns, and values.

**Key Types:**

```rust
// From: /home/rex/src/utils/forge/src/types.rs:1-290

// Version enumeration
pub enum ForgeVersion {
    V0_2_0,  // Scalar model
    V1_0_0,  // Array model
}

// Homogeneous column arrays (v1.0.0)
pub enum ColumnValue {
    Number(Vec<f64>),
    Text(Vec<String>),
    Date(Vec<String>),  // ISO format: YYYY-MM-DD
    Boolean(Vec<bool>),
}

// Column with name and typed values
pub struct Column {
    pub name: String,
    pub values: ColumnValue,
}

// Table with columns and formulas
pub struct Table {
    pub name: String,
    pub columns: HashMap<String, Column>,
    pub row_formulas: HashMap<String, String>,  // Calculated columns
}

// Unified model for both versions
pub struct ParsedModel {
    pub version: ForgeVersion,
    pub tables: HashMap<String, Table>,           // v1.0.0
    pub scalars: HashMap<String, Variable>,       // v0.2.0 + v1.0.0
    pub includes: Vec<Include>,                   // v0.2.0
    pub aggregations: HashMap<String, String>,    // v1.0.0
}

// v0.2.0 scalar variable
pub struct Variable {
    pub path: String,
    pub value: Option<f64>,
    pub formula: Option<String>,
    pub alias: Option<String>,  // For cross-file references
}
```text

**Type System Design:**

```plantuml
@startuml type-hierarchy
!theme plain
title Forge Type Hierarchy

class ForgeVersion {
  V0_2_0
  V1_0_0
  --
  +detect(yaml: &Value): Self
}

class ColumnValue {
  Number(Vec<f64>)
  Text(Vec<String>)
  Date(Vec<String>)
  Boolean(Vec<bool>)
  --
  +len(): usize
  +is_empty(): bool
  +type_name(): &str
}

class Column {
  +name: String
  +values: ColumnValue
  --
  +new(name, values): Self
  +len(): usize
}

class Table {
  +name: String
  +columns: HashMap<String, Column>
  +row_formulas: HashMap<String, String>
  --
  +new(name): Self
  +add_column(column: Column)
  +add_row_formula(name, formula)
  +row_count(): usize
  +validate_lengths(): Result<()>
}

class ParsedModel {
  +version: ForgeVersion
  +tables: HashMap<String, Table>
  +scalars: HashMap<String, Variable>
  +includes: Vec<Include>
  +aggregations: HashMap<String, String>
  --
  +new(version): Self
  +add_table(table: Table)
  +add_scalar(name, variable)
  +add_aggregation(name, formula)
}

class Variable {
  +path: String
  +value: Option<f64>
  +formula: Option<String>
  +alias: Option<String>
}

ParsedModel --> ForgeVersion : uses
ParsedModel --> Table : contains
ParsedModel --> Variable : contains
Table --> Column : contains
Column --> ColumnValue : contains

note right of ColumnValue

#### Type Safety

  Homogeneous arrays only
  No mixed types allowed
  Enforced at parse time
end note

note bottom of ParsedModel

#### Unified Model

  Supports both v0.2.0 and v1.0.0
  Version-specific fields clearly marked
end note

@enduml
```text

**Key Invariants:**

1. **Homogeneous Arrays**: All elements in `ColumnValue` must be the same type
2. **Length Consistency**: All columns in a table must have the same length
3. **Formula Validity**: Row formulas reference only existing columns
4. **Version Consistency**: v1.0.0 uses tables, v0.2.0 uses scalars

**Validation Logic:**

```rust
// From: /home/rex/src/utils/forge/src/types.rs:176-189
impl Table {
    pub fn validate_lengths(&self) -> Result<(), String> {
        let row_count = self.row_count();
        for (name, column) in &self.columns {
            if column.len() != row_count {
                return Err(format!(
                    "Column '{}' has {} rows, expected {} rows",
                    name, column.len(), row_count
                ));
            }
        }
        Ok(())
    }
}
```text

### 5. Array Calculator (/home/rex/src/utils/forge/src/core/array_calculator.rs)

**Responsibility:** Calculate all formulas in v1.0.0 array models. Handle row-wise formulas, aggregations, and cross-table references.

**Size:** 3,440 lines (largest module in Forge)

**Key Algorithm - Two-Phase Calculation:**

```plantuml
@startuml two-phase-calculation
!theme plain
title ArrayCalculator - Two-Phase Calculation Algorithm

start

:Receive ParsedModel;

partition "Phase 1: Calculate Tables" {
  :Build table dependency graph\n(cross-table references);

  :Topological sort\n(petgraph);

  :For each table in order {

  partition "Table Calculation" {
    :Build column dependency graph\n(within table);

    :Topological sort columns;

    :For each formula column {

    if (Is aggregation?) then (yes)
      :Error: Aggregations\nbelonging in scalars;
      stop
    else (no)
      :Evaluate row-wise formula\n(element-wise);

      :Create result column;

      :Add to table;
    endif
    }
  }
  }

  :Update model with calculated tables;
}

partition "Phase 2: Calculate Scalars" {
  :Build scalar dependency graph\n(references to tables);

  :Topological sort scalars;

  :For each scalar formula {

  :Evaluate aggregation\n(SUM, AVERAGE, etc.);

  :Store result;
  }
}

:Return calculated model;

stop

@enduml
```text

**Core Functions:**

```rust
// From: /home/rex/src/utils/forge/src/core/array_calculator.rs:1-200

pub struct ArrayCalculator {
    model: ParsedModel,
}

impl ArrayCalculator {
    pub fn new(model: ParsedModel) -> Self

    pub fn calculate_all(mut self) -> ForgeResult<ParsedModel>

    fn get_table_calculation_order(&self, table_names: &[String])
        -> ForgeResult<Vec<String>>

    fn calculate_table(&mut self, table_name: &str, table: &Table)
        -> ForgeResult<Table>

    fn get_formula_calculation_order(&self, table: &Table)
        -> ForgeResult<Vec<String>>

    fn evaluate_rowwise_formula(&mut self, table: &Table, formula: &str)
        -> ForgeResult<ColumnValue>

    fn calculate_scalars(&mut self) -> ForgeResult<()>
}
```text

**Dependency Resolution Example:**

```yaml
Given tables:
  pl_2025: { revenue, cogs, gross_profit: =revenue - cogs }
  pl_2026: { revenue: =pl_2025.revenue * 1.2, cogs, profit }

Dependency graph:
  pl_2025 → pl_2026 (pl_2026.revenue references pl_2025)

Calculation order:

  1. pl_2025 (no dependencies)
  2. pl_2026 (depends on pl_2025)

```text

**Row-wise Formula Evaluation:**

```rust
// From: /home/rex/src/utils/forge/src/core/array_calculator.rs:232-399

fn evaluate_rowwise_formula(&mut self, table: &Table, formula: &str)
    -> ForgeResult<ColumnValue>
{
    // Get row count from table
    let row_count = table.row_count();

    // Extract column references
    let col_refs = self.extract_column_references(formula)?;

    // Validate all columns exist and have correct length
    for col_ref in &col_refs {
        // Check cross-table references (table.column)
        if col_ref.contains('.') {
            let (table_name, col_name) = parse_table_column_ref(col_ref)?;
            validate_cross_table_reference(table_name, col_name, row_count)?;
        } else {
            // Local column reference
            validate_local_column(table, col_ref, row_count)?;
        }
    }

    // Evaluate formula for EACH row
    let mut results = Vec::new();
    for row_idx in 0..row_count {
        // Create resolver for this specific row
 let resolver = |var_name: String| -> types::Value {
            // Resolve column value at row_idx
            get_column_value_at_row(table, &var_name, row_idx)
        };

        // Evaluate formula with xlformula_engine
        let result = calculate(&formula, &resolver, &NoCustomFunction)?;
        results.push(result);
    }

    // Convert to appropriate ColumnValue type
    Ok(convert_to_column_value(results))
}
```text

**Design Decisions:**

1. **Why two-phase calculation?**
   - Tables can reference other tables (cross-table dependencies)
   - Scalars aggregate table columns (must calculate tables first)
   - Clear separation of concerns

2. **Why petgraph for dependency resolution?**
   - Robust topological sort implementation
   - Detects circular dependencies automatically
   - Standard library in Rust ecosystem

3. **Why 3,440 lines in one file?**
   - Co-location of related functionality
   - Easier to understand the full calculation pipeline
   - Private helper functions remain encapsulated
   - Future refactoring can split if needed

### 6. Scalar Calculator (/home/rex/src/utils/forge/src/core/calculator.rs)

**Responsibility:** Calculate formulas in v0.2.0 scalar models. Handle cross-file references via includes.

**Size:** 401 lines

**Key Feature:** Cross-file references with `@alias.variable` syntax

**Architecture:**

```rust
// From: /home/rex/src/utils/forge/src/core/calculator.rs (structure)

pub struct Calculator {
    variables: HashMap<String, Variable>,
}

impl Calculator {
    pub fn new(variables: HashMap<String, Variable>) -> Self

    pub fn calculate_all(&mut self) -> ForgeResult<HashMap<String, f64>>

    fn build_dependency_graph(&self) -> ForgeResult<Graph>

    fn evaluate_formula(&self, formula: &str, context: &EvalContext)
        -> ForgeResult<f64>
}
```text

**Cross-File Reference Resolution:**

```plantuml
@startuml cross-file-references
!theme plain
title Cross-File Reference Resolution (v0.2.0)

participant "main.yaml" as main
participant "Calculator" as calc
participant "pricing.yaml" as pricing
participant "costs.yaml" as costs

main -> calc : calculate()
activate calc

calc -> calc : parse_yaml_with_includes()
note right
  Reads:
  - main.yaml
  - pricing.yaml (as: pricing)
  - costs.yaml (as: costs)

end note

calc -> calc : build_dependency_graph()
note right
  Variables:
  - revenue (main)
  - base_price (pricing)
  - unit_cost (costs)
  - profit = @pricing.base_price * qty - @costs.unit_cost

  Dependencies:
  base_price → profit
  unit_cost → profit
end note

calc -> calc : topological_sort()
note right
  Order:
  1. base_price
  2. unit_cost
  3. profit

end note

calc -> calc : evaluate_formula("=@pricing.base_price * 10")
calc -> pricing : resolve("@pricing.base_price")
pricing --> calc : 99.0
calc -> calc : evaluate("=99.0 * 10")
calc --> main : 990.0

deactivate calc

@enduml
```text

**Variable Resolution Logic:**

```rust
// Simplified from calculator.rs

fn resolve_variable(name: &str, variables: &HashMap<String, Variable>)
    -> ForgeResult<f64>
{
    // Check for @alias.variable syntax
    if name.starts_with('@') {
        let parts: Vec<&str> = name[1..].split('.').collect();
        if parts.len() == 2 {
            let alias = parts[0];
            let var_name = parts[1];

            // Find variable with matching alias
            for (_, var) in variables {
                if var.alias == Some(alias.to_string())
                    && var.path.ends_with(var_name)
                {
                    return Ok(var.value.unwrap_or(0.0));
                }
            }
        }
    }

    // Regular variable lookup
    if let Some(var) = variables.get(name) {
        return Ok(var.value.unwrap_or(0.0));
    }

    Err(ForgeError::Eval(format!("Variable not found: {}", name)))
}
```text

### 7. Writer Layer (/home/rex/src/utils/forge/src/writer/mod.rs)

**Responsibility:** Write calculated values back to YAML files. Preserve formatting, comments, and structure.

**Key Challenge:** Update values in-place without destroying YAML structure.

**Key Function:**

```rust
pub fn update_all_yaml_files(
    main_file: &Path,
    parsed: &ParsedYaml,
    results: &HashMap<String, f64>,
    variables: &HashMap<String, Variable>,
) -> ForgeResult<()>
```text

**Update Algorithm:**

```plantuml
@startuml yaml-update-algorithm
!theme plain
title YAML Update Algorithm - Preserving Structure

start

:Receive calculated values;

:For each file (main + includes) {

partition "File Update" {
  :Read original YAML as string;

  :Parse YAML to Value tree;

  :Traverse tree depth-first;

  :For each node {

  if (Has "formula" field?) then (yes)
    :Extract variable path;

    if (Value in results?) then (yes)
      :Update "value" field;
      note right
        Preserve original formatting:
        - Indentation
        - Comments
        - Key ordering
      end note
    endif
  endif
  }

  :Serialize back to YAML;

  :Write to file;
}
}

stop

@enduml
```text

**Design Decision: Why not use serde_yaml directly?**

Serde_yaml doesn't preserve:

- Comments (`# This is a comment`)
- Key ordering
- Whitespace formatting
- Custom formatting choices

Solution: Parse → Update specific paths → Serialize

---

## Component Interactions

### Calculate Command - Component Collaboration

```plantuml
@startuml calculate-collaboration
!theme plain
title Calculate Command - Component Collaboration Diagram

actor User
participant "main.rs" as main
participant "commands.rs" as cmd
participant "parser/mod.rs" as parser
participant "array_calculator.rs" as calc
participant "xlformula_engine" as xle
participant "petgraph" as pg
participant "writer/mod.rs" as writer

User -> main : forge calculate model.yaml
activate main

main -> cmd : calculate(file, dry_run, verbose)
activate cmd

cmd -> parser : parse_model(&file)
activate parser

parser -> parser : detect_version()
parser -> parser : parse_v1_model()
parser --> cmd : ParsedModel
deactivate parser

cmd -> calc : new(model)
activate calc

cmd -> calc : calculate_all()

calc -> pg : build_graph()
pg --> calc : graph

calc -> pg : toposort(graph)
pg --> calc : calculation_order

loop for each table
  calc -> calc : calculate_table()

  loop for each formula
    calc -> xle : calculate(formula, resolver)
    xle --> calc : result
  end
end

loop for each scalar
  calc -> xle : calculate(formula, resolver)
  xle --> calc : result
end

calc --> cmd : calculated_model
deactivate calc

alt if not dry_run
  cmd -> writer : update_all_yaml_files()
  activate writer
  writer -> writer : update_values()
  writer --> cmd : success
  deactivate writer
end

cmd --> main : Ok(())
deactivate cmd

main --> User : ✨ Files updated successfully!
deactivate main

@enduml
```text

### Export Command - Component Collaboration

```plantuml
@startuml export-collaboration
!theme plain
title Export Command - Component Collaboration Diagram

actor User
participant "main.rs" as main
participant "commands.rs" as cmd
participant "parser/mod.rs" as parser
participant "exporter.rs" as exporter
participant "formula_translator.rs" as translator
participant "rust_xlsxwriter" as xlsx

User -> main : forge export model.yaml output.xlsx
activate main

main -> cmd : export(input, output, verbose)
activate cmd

cmd -> parser : parse_model(&input)
activate parser
parser --> cmd : ParsedModel
deactivate parser

cmd -> exporter : new(model)
activate exporter

cmd -> exporter : export(&output_path)

loop for each table
  exporter -> exporter : export_table(table)

  exporter -> translator : new(column_map)
  activate translator

  loop for each row
    alt if has formula
      exporter -> translator : translate_row_formula(formula, row)
      translator --> exporter : excel_formula
      exporter -> xlsx : write_formula(row, col, formula)
    else has data
      exporter -> xlsx : write_number/text/boolean(row, col, value)
    end
  end

  deactivate translator
end

exporter -> exporter : export_scalars()

exporter -> xlsx : save(output_path)
xlsx --> exporter : success

exporter --> cmd : Ok(())
deactivate exporter

cmd --> main : Ok(())
deactivate cmd

main --> User : ✅ Exported to output.xlsx
deactivate main

@enduml
```text

### Version Detection - Component Interaction

```plantuml
@startuml version-detection-interaction
!theme plain
title Version Detection - Component Interaction

participant "commands.rs" as cmd
participant "parser/mod.rs" as parser
participant "types.rs" as types
participant "array_calculator.rs" as array_calc
participant "calculator.rs" as scalar_calc

cmd -> parser : parse_model(&file)
activate parser

parser -> types : ForgeVersion::detect(&yaml)
activate types

alt Explicit version marker
  types --> parser : V1_0_0 or V0_2_0
else Has "includes" field
  types --> parser : V0_2_0
else Has "tables" with "columns"
  types --> parser : V1_0_0
else Has array values
  types --> parser : V1_0_0
else Default
  types --> parser : V0_2_0 (backwards compatible)
end

deactivate types

alt if V1_0_0
  parser -> parser : parse_v1_model()
  parser --> cmd : ParsedModel(V1_0_0)
  cmd -> array_calc : new(model)
else if V0_2_0
  parser -> parser : parse_v0_model()
  parser --> cmd : ParsedModel(V0_2_0)
  cmd -> scalar_calc : new(variables)
end

deactivate parser

@enduml
```text

---

## Module Boundaries

### Boundary Definitions

**1. CLI ↔ Core Boundary**

```text
Interface: ParsedModel, ForgeResult<T>
Data Flow: CLI → Core (input), Core → CLI (output)
Coupling: Loose (via types.rs)
```text

**2. Core ↔ Parser Boundary**

```text
Interface: parse_model() returns ParsedModel
Data Flow: Parser → Core (one-way)
Coupling: Medium (shared types)
```text

**3. Core ↔ Writer Boundary**

```text
Interface: update_all_yaml_files()
Data Flow: Core → Writer (calculated values)
Coupling: Medium (shared types)
```text

**4. Core ↔ Excel Boundary**

```text
Interface: ExcelExporter, ExcelImporter
Data Flow: Bidirectional (import/export)
Coupling: Loose (via ParsedModel)
```text

**5. All ↔ Types Boundary**

```text
Interface: Public types (Column, Table, etc.)
Data Flow: Shared read access
Coupling: Strong (central data structures)
```text

### Dependency Graph

```plantuml
@startuml module-dependencies
!theme plain
title Module Dependency Graph

package "Public API" {
  [main.rs]
  [lib.rs]
}

package "CLI" {
  [commands.rs]
}

package "Core" {
  [array_calculator.rs]
  [calculator.rs]
}

package "Parser" {
  [parser/mod.rs]
}

package "Writer" {
  [writer/mod.rs]
}

package "Excel" {
  [exporter.rs]
  [importer.rs]
  [formula_translator.rs]
}

package "Foundation" {
  [types.rs]
  [error.rs]
}

[main.rs] --> [commands.rs]
[lib.rs] --> [parser/mod.rs]
[lib.rs] --> [array_calculator.rs]
[lib.rs] --> [calculator.rs]
[lib.rs] --> [types.rs]

[commands.rs] --> [parser/mod.rs]
[commands.rs] --> [array_calculator.rs]
[commands.rs] --> [calculator.rs]
[commands.rs] --> [exporter.rs]
[commands.rs] --> [importer.rs]
[commands.rs] --> [writer/mod.rs]

[array_calculator.rs] --> [types.rs]
[array_calculator.rs] --> [error.rs]
[calculator.rs] --> [types.rs]
[calculator.rs] --> [error.rs]

[parser/mod.rs] --> [types.rs]
[parser/mod.rs] --> [error.rs]

[writer/mod.rs] --> [types.rs]
[writer/mod.rs] --> [error.rs]

[exporter.rs] --> [types.rs]
[exporter.rs] --> [formula_translator.rs]
[importer.rs] --> [types.rs]

note right of [types.rs]

#### Foundation Module

  No dependencies except stdlib
  Shared by all modules
end note

note right of [error.rs]

#### Error Types

  Used by all modules
  Unified error handling
end note

@enduml
```text

### Boundary Enforcement

**1. Private Module Functions**

Rust's module system enforces boundaries via visibility:

```rust
// Public API
pub fn calculate_all(self) -> ForgeResult<ParsedModel>

// Private implementation
fn get_table_calculation_order(&self, ...) -> ForgeResult<Vec<String>>
fn calculate_table(&mut self, ...) -> ForgeResult<Table>
fn evaluate_rowwise_formula(&mut self, ...) -> ForgeResult<ColumnValue>
```text

**2. Type Boundaries**

```rust
// types.rs - Public interface
pub struct ParsedModel { ... }
pub enum ColumnValue { ... }

// array_calculator.rs - Private state
struct ArrayCalculator {
    model: ParsedModel,  // Owns the model
}
```text

**3. Error Boundaries**

Each layer can only produce errors from its domain:

```rust
// Parser errors
ForgeError::Parse("Invalid YAML structure")

// Calculator errors
ForgeError::Eval("Column not found: revenue")
ForgeError::CircularDependency("Circular dependency detected")

// Excel errors
ForgeError::Export("Failed to write formula")
ForgeError::Import("Failed to read worksheet")
```text

---

## Interface Contracts

### 1. Parser Interface

**Contract:** Parse YAML files into ParsedModel, detect version, validate structure.

```rust
/// Parse a Forge model file (v0.2.0 or v1.0.0)
///
/// # Arguments
/// * `path` - Path to the Forge YAML file
///
/// # Returns
/// * `Ok(ParsedModel)` - Successfully parsed model
/// * `Err(ForgeError)` - Parse error with context
///
/// # Guarantees
/// - Version is correctly detected
/// - All columns in tables have equal length
/// - Schema is valid (v1.0.0)
/// - Cross-file references are resolved (v0.2.0)
pub fn parse_model(path: &Path) -> ForgeResult<ParsedModel>
```text

**Preconditions:**

- File exists and is readable
- File contains valid YAML

**Postconditions:**

- Model version is V0_2_0 or V1_0_0
- All table columns have same length
- All formulas reference existing columns
- No duplicate column names within a table

**Error Conditions:**

- `ForgeError::IO` - File not found or permission denied
- `ForgeError::Parse` - Invalid YAML syntax
- `ForgeError::Validation` - Schema validation failed

### 2. Calculator Interface

**Contract:** Calculate all formulas in dependency order, return updated model.

```rust
/// Calculate all formulas in the model
///
/// # Returns
/// * `Ok(ParsedModel)` - Model with calculated values
/// * `Err(ForgeError)` - Calculation error
///
/// # Guarantees
/// - All formulas evaluated in dependency order
/// - No formula evaluated before its dependencies
/// - Circular dependencies detected and rejected
pub fn calculate_all(self) -> ForgeResult<ParsedModel>
```text

**Preconditions:**

- Model is valid (all columns exist)
- No circular dependencies

**Postconditions:**

- All formula columns have calculated values
- Result values match formula definitions
- Original data columns unchanged

**Error Conditions:**

- `ForgeError::CircularDependency` - Circular reference detected
- `ForgeError::Eval` - Formula evaluation failed
- `ForgeError::Eval` - Column not found

### 3. Exporter Interface

**Contract:** Export ParsedModel to Excel .xlsx format with formula translation.

```rust
/// Export the model to an Excel .xlsx file
///
/// # Arguments
/// * `output_path` - Path to output .xlsx file
///
/// # Returns
/// * `Ok(())` - Export succeeded
/// * `Err(ForgeError)` - Export failed
///
/// # Guarantees
/// - Each table becomes a separate worksheet
/// - Row formulas translated to Excel syntax
/// - Data values written with correct types
pub fn export(&self, output_path: &Path) -> ForgeResult<()>
```text

**Preconditions:**

- Model is v1.0.0 (has tables)
- Output path is writable

**Postconditions:**

- Excel file created at output_path
- All tables exported as worksheets
- Formulas translated to Excel syntax
- Data types preserved (Number, Text, Boolean, Date)

**Error Conditions:**

- `ForgeError::Export` - Failed to write file
- `ForgeError::Export` - Formula translation failed

### 4. Importer Interface

**Contract:** Import Excel .xlsx file to ParsedModel, preserve formulas.

```rust
/// Import Excel .xlsx file to YAML v1.0.0 format
///
/// # Arguments
/// * `input_path` - Path to input .xlsx file
///
/// # Returns
/// * `Ok(ParsedModel)` - Imported model
/// * `Err(ForgeError)` - Import failed
///
/// # Guarantees
/// - Each worksheet becomes a table
/// - Formulas preserved as Excel syntax
/// - Data types inferred correctly
pub fn import(&self, input_path: &Path) -> ForgeResult<ParsedModel>
```text

**Preconditions:**

- Input file exists and is valid Excel format
- File is readable

**Postconditions:**

- Model version is V1_0_0
- All worksheets imported as tables
- Formulas preserved (as Excel syntax initially)
- Data types correctly inferred

**Error Conditions:**

- `ForgeError::Import` - Failed to read file
- `ForgeError::Import` - Invalid Excel format

---

## Data Flow Patterns

### Pattern 1: Linear Pipeline (Calculate v0.2.0)

```text
YAML File → Parser → Calculator → Writer → Updated YAML Files
```text

**Characteristics:**

- One-way data flow
- Each stage transforms data
- No cycles or feedback loops

**Implementation:**

```rust
// From commands.rs
pub fn calculate(file: PathBuf, dry_run: bool, verbose: bool) -> ForgeResult<()> {
    let parsed = parser::parse_yaml_with_includes(&file)?;  // Stage 1
    let mut calculator = Calculator::new(parsed.variables.clone());
    let results = calculator.calculate_all()?;  // Stage 2
    if !dry_run {
        writer::update_all_yaml_files(&file, &parsed, &results, &parsed.variables)?;  // Stage 3
    }
    Ok(())
}
```text

### Pattern 2: Two-Phase Pipeline (Calculate v1.0.0)

```text
YAML File → Parser → ArrayCalculator
                         ↓
                    Phase 1: Tables → Phase 2: Scalars → Result
```text

**Characteristics:**

- Sequential phases with dependencies
- Phase 2 depends on Phase 1 output
- No feedback between phases

**Implementation:**

```rust
// From array_calculator.rs:18-33
pub fn calculate_all(mut self) -> ForgeResult<ParsedModel> {
    // Phase 1: Calculate all tables
    let table_names: Vec<String> = self.model.tables.keys().cloned().collect();
    let calc_order = self.get_table_calculation_order(&table_names)?;

    for table_name in calc_order {
        let table = self.model.tables.get(&table_name).unwrap().clone();
        let calculated_table = self.calculate_table(&table_name, &table)?;
        self.model.tables.insert(table_name, calculated_table);
    }

    // Phase 2: Calculate scalar aggregations
    self.calculate_scalars()?;

    Ok(self.model)
}
```text

### Pattern 3: Bidirectional Bridge (Excel Integration)

```text
YAML ←→ ParsedModel ←→ Excel
     (Parser/Writer)   (Exporter/Importer)
```text

**Characteristics:**

- Two-way conversion
- Lossless round-trip (ideally)
- Formula translation in both directions

**Implementation:**

```rust
// Export: YAML → Excel
let model = parser::parse_model(&input)?;
let exporter = ExcelExporter::new(model);
exporter.export(&output)?;

// Import: Excel → YAML
let importer = ExcelImporter::new();
let model = importer.import(&input)?;
writer::write_model(&output, &model)?;
```text

### Pattern 4: Dependency-Driven Execution

```text
Variables → Dependency Graph → Topological Sort → Ordered Execution
```text

**Characteristics:**

- Order determined by dependencies
- Parallel execution possible (not implemented yet)
- Circular dependencies rejected

**Implementation:**

```rust
// From array_calculator.rs:36-81
fn get_table_calculation_order(&self, table_names: &[String])
    -> ForgeResult<Vec<String>>
{
    use petgraph::algo::toposort;
    use petgraph::graph::DiGraph;
    use std::collections::HashMap;

    let mut graph = DiGraph::new();
    let mut node_indices = HashMap::new();

    // Create nodes for all tables
    for name in table_names {
        let idx = graph.add_node(name.clone());
        node_indices.insert(name.clone(), idx);
    }

    // Add edges for cross-table dependencies
    for name in table_names {
        if let Some(table) = self.model.tables.get(name) {
            for formula in table.row_formulas.values() {
                let deps = self.extract_table_dependencies_from_formula(formula)?;
                for dep_table in deps {
                    if let Some(&dep_idx) = node_indices.get(&dep_table) {
                        if let Some(&name_idx) = node_indices.get(name) {
                            graph.add_edge(dep_idx, name_idx, ());
                        }
                    }
                }
            }
        }
    }

    // Topological sort
 let order = toposort(&graph, None).map_err(|_| {
        ForgeError::CircularDependency(
            "Circular dependency detected between tables".to_string(),
        )
    })?;

    let ordered_names: Vec<String> = order
        .iter()
 .filter_map(|idx| graph.node_weight(*idx).cloned())
        .collect();

    Ok(ordered_names)
}
```text

---

## Version-Specific Components

### v0.2.0 Components (Backwards Compatible)

**Active Components:**

- `calculator.rs` - Scalar calculator
- `parse_v0_model()` - v0.2.0 parser
- `parse_yaml_with_includes()` - Include handling

**Data Structures:**

```rust
pub struct Variable {
    pub path: String,
    pub value: Option<f64>,
    pub formula: Option<String>,
    pub alias: Option<String>,
}

pub struct Include {
    pub file: String,
    pub r#as: String,
}
```text

**Key Features:**

- Cross-file references (`@alias.variable`)
- Scalar values only
- Discrete formulas (`{value, formula}`)

### v1.0.0 Components (Current)

**Active Components:**

- `array_calculator.rs` - Array calculator
- `parse_v1_model()` - v1.0.0 parser
- Excel integration (exporter, importer)

**Data Structures:**

```rust
pub struct Table {
    pub name: String,
    pub columns: HashMap<String, Column>,
    pub row_formulas: HashMap<String, String>,
}

pub enum ColumnValue {
    Number(Vec<f64>),
    Text(Vec<String>),
    Date(Vec<String>),
    Boolean(Vec<bool>),
}
```text

**Key Features:**

- Column arrays
- Row-wise formulas
- Aggregation formulas
- Excel export/import
- 47+ Excel functions

### Version Coexistence Strategy

**Design:** Both versions coexist in the same codebase.

```rust
// From parser/mod.rs:39-50
pub fn parse_model(path: &Path) -> ForgeResult<ParsedModel> {
    let content = std::fs::read_to_string(path)?;
    let yaml: Value = serde_yaml::from_str(&content)?;

    let version = ForgeVersion::detect(&yaml);

    match version {
        ForgeVersion::V1_0_0 => parse_v1_model(path, &yaml),
        ForgeVersion::V0_2_0 => parse_v0_model(path, &yaml),
    }
}
```text

**Benefits:**

- No breaking changes for existing users
- Easy migration path (v0.2.0 → v1.0.0)
- Both versions tested and maintained

**Trade-offs:**

- Increased code complexity
- Larger binary size
- More test coverage needed

---

## External Dependencies

### 1. xlformula_engine (v0.1.18)

**Purpose:** Excel-compatible formula evaluation

**Interface:**

```rust
use xlformula_engine::{calculate, parse_formula, types, NoCustomFunction};

// Evaluate a formula
let result = calculate(
    "=A1 + B1",
    &resolver,  // Variable resolver closure
    &NoCustomFunction,  // No custom functions
)?;
```text

**Supported Functions:**

- Math: SUM, AVERAGE, MAX, MIN, ABS, ROUND, SQRT, POWER
- Logic: IF, AND, OR, NOT
- Text: LEFT, RIGHT, CONCATENATE
- Conditional: SUMIF, COUNTIF, AVERAGEIF

**Integration Points:**

- `array_calculator.rs:400+` - Row-wise formula evaluation
- `calculator.rs` - Scalar formula evaluation

**Design Decision:** Why xlformula_engine?

- Excel-compatible syntax
- No need to implement 47+ functions manually
- Active maintenance and updates
- Rust-native (no FFI)

### 2. petgraph (v0.6)

**Purpose:** Dependency graph construction and topological sorting

**Interface:**

```rust
use petgraph::algo::toposort;
use petgraph::graph::DiGraph;

let mut graph = DiGraph::new();
let node_a = graph.add_node("A");
let node_b = graph.add_node("B");
graph.add_edge(node_a, node_b, ());  // A → B

let order = toposort(&graph, None)?;  // Returns [A, B]
```text

**Integration Points:**

- `array_calculator.rs:36-81` - Table dependency resolution
- `array_calculator.rs:136-176` - Column dependency resolution
- `calculator.rs` - Variable dependency resolution

**Design Decision:** Why petgraph?

- Robust topological sort
- Automatic cycle detection
- Standard library in Rust ecosystem
- Well-tested and documented

### 3. serde_yaml (v0.9)

**Purpose:** YAML serialization and deserialization

**Interface:**

```rust
use serde_yaml::Value;

let yaml: Value = serde_yaml::from_str(&content)?;
let tables = yaml.get("tables")?;
```text

**Integration Points:**

- `parser/mod.rs` - YAML parsing
- `writer/mod.rs` - YAML serialization

**Design Decision:** Why serde_yaml?

- De facto standard for YAML in Rust
- Serde integration (derive macros)
- Good error messages
- Supports YAML 1.2

### 4. rust_xlsxwriter (v0.90)

**Purpose:** Create Excel .xlsx files

**Interface:**

```rust
use rust_xlsxwriter::{Workbook, Worksheet, Formula};

let mut workbook = Workbook::new();
let worksheet = workbook.add_worksheet();
worksheet.write_string(0, 0, "Header")?;
worksheet.write_formula(1, 0, Formula::new("=A1+B1"))?;
workbook.save("output.xlsx")?;
```text

**Integration Points:**

- `excel/exporter.rs` - Excel file creation

**Design Decision:** Why rust_xlsxwriter?

- Pure Rust (no external dependencies)
- Fast and efficient
- Supports formulas
- Active development

### 5. calamine (v0.31)

**Purpose:** Read Excel .xlsx files

**Interface:**

```rust
use calamine::{Reader, Xlsx};

let mut workbook: Xlsx<_> = open_workbook(&path)?;
let worksheet = workbook.worksheet_range("Sheet1")?;
for row in worksheet.rows() {
    // Process row
}
```text

**Integration Points:**

- `excel/importer.rs` - Excel file reading

**Design Decision:** Why calamine?

- Fast Excel reading
- Supports .xlsx, .xls, .ods
- Formula preservation
- Widely used in Rust ecosystem

### 6. clap (v4.5)

**Purpose:** CLI argument parsing

**Interface:**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]

struct Cli {
    #[command(subcommand)]
    command: Commands,
}
```text

**Integration Points:**

- `main.rs` - CLI definition

**Design Decision:** Why clap?

- Derive macros for clean code
- Automatic help generation
- Subcommand support
- Industry standard

### Dependency Graph

```plantuml
@startuml external-dependencies
!theme plain
title External Dependencies

package "Forge" {
  [array_calculator.rs]
  [calculator.rs]
  [parser/mod.rs]
  [writer/mod.rs]
  [excel/exporter.rs]
  [excel/importer.rs]
  [main.rs]
}

package "External Libraries" {
  [xlformula_engine\nv0.1.18]
  [petgraph\nv0.6]
  [serde_yaml\nv0.9]
  [rust_xlsxwriter\nv0.90]
  [calamine\nv0.31]
  [clap\nv4.5]
}

[array_calculator.rs] --> [xlformula_engine\nv0.1.18] : formula evaluation
[array_calculator.rs] --> [petgraph\nv0.6] : dependency graph
[calculator.rs] --> [xlformula_engine\nv0.1.18] : formula evaluation
[calculator.rs] --> [petgraph\nv0.6] : dependency graph

[parser/mod.rs] --> [serde_yaml\nv0.9] : deserialize
[writer/mod.rs] --> [serde_yaml\nv0.9] : serialize

[excel/exporter.rs] --> [rust_xlsxwriter\nv0.90] : create Excel
[excel/importer.rs] --> [calamine\nv0.31] : read Excel

[main.rs] --> [clap\nv4.5] : CLI parsing

note right of [xlformula_engine\nv0.1.18]

#### Formula Engine

  47+ Excel functions
  Excel-compatible syntax
end note

note right of [petgraph\nv0.6]

#### Graph Algorithms

  Topological sort
  Cycle detection
end note

@enduml
```text

---

## Related Documentation

### Architecture Deep Dives

- [00-OVERVIEW.md](00-OVERVIEW.md) - High-level architecture overview
- [02-DATA-MODEL.md](02-DATA-MODEL.md) - Type system and data structures
- [03-FORMULA-EVALUATION.md](03-FORMULA-EVALUATION.md) - Calculation pipeline details
- [04-DEPENDENCY-RESOLUTION.md](04-DEPENDENCY-RESOLUTION.md) - Graph algorithms
- [05-EXCEL-INTEGRATION.md](05-EXCEL-INTEGRATION.md) - Bidirectional conversion
- [06-CLI-ARCHITECTURE.md](06-CLI-ARCHITECTURE.md) - Command structure
- [07-TESTING-ARCHITECTURE.md](07-TESTING-ARCHITECTURE.md) - Test strategy

### User Documentation

- [README.md](../../README.md) - User guide, features, installation
- [DESIGN_V1.md](../../DESIGN_V1.md) - v1.0.0 specification
- [GLOSSARY.md](../../GLOSSARY.md) - Canonical terminology

### Source Files Referenced

- `/home/rex/src/utils/forge/src/main.rs` - CLI entry point
- `/home/rex/src/utils/forge/src/cli/commands.rs` - Command handlers
- `/home/rex/src/utils/forge/src/parser/mod.rs` - YAML parser
- `/home/rex/src/utils/forge/src/types.rs` - Core data structures
- `/home/rex/src/utils/forge/src/core/array_calculator.rs` - Array calculator
- `/home/rex/src/utils/forge/src/core/calculator.rs` - Scalar calculator
- `/home/rex/src/utils/forge/src/writer/mod.rs` - YAML writer
- `/home/rex/src/utils/forge/src/excel/exporter.rs` - Excel exporter
- `/home/rex/src/utils/forge/src/excel/importer.rs` - Excel importer
- `/home/rex/src/utils/forge/src/excel/formula_translator.rs` - Formula translation

---

**Previous:** [← Architecture Overview](00-OVERVIEW.md)
**Next:** [Data Model →](02-DATA-MODEL.md)
