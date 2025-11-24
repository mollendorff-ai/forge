# Formula Evaluation Architecture

**Document Version:** 1.0.0
**Forge Version:** v1.1.2
**Last Updated:** 2025-11-24
**Status:** Complete

---

## Table of Contents

1. [Introduction](#introduction)
2. [Formula Evaluation Pipeline](#formula-evaluation-pipeline)
3. [xlformula_engine Integration](#xlformula_engine-integration)
4. [Row-wise Formula Evaluation](#row-wise-formula-evaluation)
5. [Aggregation Formula Evaluation](#aggregation-formula-evaluation)
6. [Custom Function Preprocessing](#custom-function-preprocessing)
7. [Variable Resolution](#variable-resolution)
8. [Formula Types and Patterns](#formula-types-and-patterns)
9. [Performance Optimization](#performance-optimization)
10. [Error Handling](#error-handling)
11. [Related Documentation](#related-documentation)

---

## Introduction

### Purpose

This document provides a comprehensive specification of Forge's formula evaluation system, including:

- **Evaluation pipeline** - How formulas are parsed, preprocessed, and calculated
- **xlformula_engine integration** - External formula engine usage
- **Formula types** - Row-wise, aggregations, and scalars
- **Custom functions** - Special handling for unsupported functions
- **Performance** - Optimization strategies and complexity analysis

### Design Principles

1. **Excel Compatibility** - Formulas work exactly like Excel
2. **Type Safety** - Result types match formula semantics
3. **Dependency Order** - Formulas evaluate after their dependencies
4. **Error Clarity** - Clear error messages with context
5. **Performance** - Efficient evaluation for large datasets

---

## Formula Evaluation Pipeline

### High-Level Architecture

```plantuml
@startuml formula-evaluation-pipeline
!theme plain
title Formula Evaluation Pipeline

start

:User defines formula\nin YAML;

partition "Parse Phase" {
  :Read YAML file;
  :Parse formula string;
  :Extract dependencies;
}

partition "Dependency Resolution" {
  :Build dependency graph;
  :Topological sort;
  if (Circular dependency?) then (yes)
    :Error: Circular dependency;
    stop
  endif
}

partition "Preprocessing" {
  if (Has custom functions?) then (yes)
    :Preprocess ROUND, SQRT, etc;
  endif

  if (Has array indexing?) then (yes)
    :Replace table.column[idx]\nwith actual values;
  endif
}

partition "Evaluation" {
  if (Row-wise formula?) then (yes)
    :For each row {
      :Resolve column values;
      :Call xlformula_engine;
      :Collect result;
    }
  elseif (Aggregation?) then (aggregation)
    :Get column array;
    :Apply aggregation;
    :Return scalar;
  else (scalar)
    :Resolve variables;
    :Call xlformula_engine;
    :Return value;
  endif
}

partition "Result Handling" {
  :Convert xlformula type\nto Forge type;
  :Validate result;
  :Store in model;
}

:Return calculated model;

stop

@enduml
```text

### Two-Phase Calculation Model

**Phase 1: Tables (Row-wise formulas)**

```text
For each table in dependency order:
  For each row in table:
    For each formula column:
      Resolve column_ref[row_idx] → value
      Evaluate formula with values
      Store result in column[row_idx]
```text

**Phase 2: Scalars (Aggregations)**

```text
For each scalar in dependency order:
  Extract table.column references
  Get entire column array
  Apply aggregation (SUM, AVERAGE, etc.)
  Store scalar result
```text

**Code Implementation:**

```rust
// From: /home/rex/src/utils/forge/src/core/array_calculator.rs:18-33
pub fn calculate_all(mut self) -> ForgeResult<ParsedModel> {
    // Phase 1: Calculate all tables (row-wise formulas) in dependency order
    let table_names: Vec<String> = self.model.tables.keys().cloned().collect();
    let calc_order = self.get_table_calculation_order(&table_names)?;

    for table_name in calc_order {
        let table = self.model.tables.get(&table_name).unwrap().clone();
        let calculated_table = self.calculate_table(&table_name, &table)?;
        self.model.tables.insert(table_name, calculated_table);
    }

    // Phase 2: Calculate scalar aggregations and formulas
    self.calculate_scalars()?;

    Ok(self.model)
}
```text

---

## xlformula_engine Integration

### External Dependency

**Library:** `xlformula_engine` v0.1.18
**Purpose:** Excel-compatible formula evaluation
**Language:** Rust (native, no FFI)

**Key Functions:**

```rust
use xlformula_engine::{calculate, parse_formula, types, NoCustomFunction};

// Parse formula string to AST
let parsed = parse_formula::parse_string_to_formula(
    "=A1 + B1",
    None::<NoCustomFunction>
);

// Evaluate with variable resolver
let result = calculate::calculate_formula(parsed, Some(&resolver));

// Result is types::Value enum
match result {
    types::Value::Number(n) => println!("Result: {}", n),
    types::Value::Text(s) => println!("Result: {}", s),
    types::Value::Boolean(b) => println!("Result: {:?}", b),
    types::Value::Error(e) => eprintln!("Error: {:?}", e),
    _ => {}
}
```text

### Supported Functions (47+)

**Aggregation Functions:**

- `SUM`, `AVERAGE`, `MAX`, `MIN`, `COUNT`, `PRODUCT`

**Conditional Aggregations:**

- `SUMIF`, `SUMIFS`, `COUNTIF`, `COUNTIFS`
- `AVERAGEIF`, `AVERAGEIFS`, `MAXIFS`, `MINIFS`

**Logical Functions:**

- `IF`, `AND`, `OR`, `NOT`, `XOR`, `IFERROR`, `IFNA`

**Math Functions:**

- `ABS`, `ROUND`, `ROUNDUP`, `ROUNDDOWN`
- `SQRT`, `POWER`, `EXP`, `LN`, `LOG`, `LOG10`
- `MOD`, `CEILING`, `FLOOR`, `PI`, `E`

**Text Functions:**

- `CONCATENATE`, `LEFT`, `RIGHT`, `MID`
- `LEN`, `UPPER`, `LOWER`, `TRIM`

**Date Functions:**

- `TODAY`, `NOW`, `DATE`, `YEAR`, `MONTH`, `DAY`

### Type Mapping

| xlformula_engine | Forge ColumnValue | Notes |
|------------------|-------------------|-------|
| `Value::Number(f32)` | `ColumnValue::Number(f64)` | Upcast to f64 |
| `Value::Text(String)` | `ColumnValue::Text(String)` | Direct mapping |
| `Value::Boolean(Boolean)` | `ColumnValue::Boolean(bool)` | Convert enum |
| `Value::Error(Error)` | `ForgeError::Eval` | Convert to error |

**Code Example:**

```rust
// From: /home/rex/src/utils/forge/src/core/array_calculator.rs:406-443
match result {
    types::Value::Number(n) => {
        let value = n as f64;  // f32 → f64 upcast
        let rounded = (value * 1e6).round() / 1e6;  // Round to 6 decimals
        number_results.push(rounded);
        result_type = Some("number");
    }
    types::Value::Text(t) => {
        text_results.push(t);
        result_type = Some("text");
    }
    types::Value::Boolean(b) => {
        let bool_val = match b {
            types::Boolean::True => true,
            types::Boolean::False => false,
        };
        bool_results.push(bool_val);
        result_type = Some("boolean");
    }
    types::Value::Error(e) => {
        return Err(ForgeError::Eval(format!(
            "Formula '{}' at row {} returned error: {:?}",
            formula_str, row_idx, e
        )));
    }
    other => {
        return Err(ForgeError::Eval(format!(
            "Formula '{}' returned unexpected type: {:?}",
            formula_str, other
        )));
    }
}
```text

---

## Row-wise Formula Evaluation

### Concept

Row-wise formulas operate on **each row independently**, similar to Excel cell formulas.

**Example:**

```yaml
pl_2025:
  columns:
    revenue: [100000, 120000, 150000, 180000]
    cogs: [30000, 36000, 45000, 54000]
  row_formulas:
    profit: "=revenue - cogs"
    margin: "=profit / revenue"
```text

**Evaluation:**

```text
Row 0: profit[0] = revenue[0] - cogs[0] = 100000 - 30000 = 70000
Row 1: profit[1] = revenue[1] - cogs[1] = 120000 - 36000 = 84000
Row 2: profit[2] = revenue[2] - cogs[2] = 150000 - 45000 = 105000
Row 3: profit[3] = revenue[3] - cogs[3] = 180000 - 54000 = 126000

Row 0: margin[0] = profit[0] / revenue[0] = 70000 / 100000 = 0.7
Row 1: margin[1] = profit[1] / revenue[1] = 84000 / 120000 = 0.7
...
```text

### Evaluation Algorithm

```plantuml
@startuml rowwise-evaluation
!theme plain
title Row-wise Formula Evaluation Algorithm

start

:Receive formula string\n(e.g., "=revenue - cogs");

:Get row count from table;

:Extract column references\n(e.g., ["revenue", "cogs"]);

:Validate all columns exist\nand have correct length;

:Initialize result vectors;

:For row_idx in 0..row_count {

partition "Per-Row Evaluation" {
  :Create resolver closure;
  note right
    resolver("revenue") → revenue[row_idx]
    resolver("cogs") → cogs[row_idx]
  end note

  :Call xlformula_engine\nwith resolver;

  :Get result value;

  :Append to result vector;
}

}

:Convert result vector\nto ColumnValue;

:Return calculated column;

stop

@enduml
```text

### Code Implementation

```rust
// From: /home/rex/src/utils/forge/src/core/array_calculator.rs:232-454
fn evaluate_rowwise_formula(&mut self, table: &Table, formula: &str)
    -> ForgeResult<ColumnValue>
{
    let formula_str = if !formula.starts_with('=') {
        format!("={}", formula.trim())
    } else {
        formula.to_string()
    };

    // Get the row count from the table
    let row_count = table.row_count();
    if row_count == 0 {
        return Err(ForgeError::Eval(
            "Cannot evaluate row-wise formula on empty table".to_string(),
        ));
    }

    // Extract column references from the formula
    let col_refs = self.extract_column_references(formula)?;

    // Validate all columns exist and have correct length
    for col_ref in &col_refs {
        if col_ref.contains('.') {
            // Cross-table reference - validate it exists
            let (table_name, col_name) = parse_table_column_ref(col_ref)?;
            validate_cross_table_reference(table_name, col_name, row_count)?;
        } else {
            // Local column reference
            validate_local_column(table, col_ref, row_count)?;
        }
    }

    // Evaluate formula for each row
    let mut number_results = Vec::new();
    let mut text_results = Vec::new();
    let mut bool_results = Vec::new();
    let mut result_type: Option<&str> = None;

    for row_idx in 0..row_count {
        // Preprocess formula for custom functions
        let processed_formula = if self.has_custom_math_function(&formula_str)
 || self.has_custom_text_function(&formula_str)
 || self.has_custom_date_function(&formula_str)
        {
            self.preprocess_custom_functions(&formula_str, row_idx, table)?
        } else {
            formula_str.clone()
        };

        // Create a resolver for this specific row
 let resolver = |var_name: String| -> types::Value {
            // Check if this is a cross-table reference (table.column format)
            if var_name.contains('.') {
                let parts: Vec<&str> = var_name.split('.').collect();
                if parts.len() == 2 {
                    let ref_table_name = parts[0];
                    let ref_col_name = parts[1];

                    if let Some(ref_table) = self.model.tables.get(ref_table_name) {
                        if let Some(ref_col) = ref_table.columns.get(ref_col_name) {
                            return get_column_value_at_row(ref_col, row_idx);
                        }
                    }
                }
                return types::Value::Error(types::Error::Value);
            }

            // Local column reference
            if let Some(col) = table.columns.get(&var_name) {
                return get_column_value_at_row(col, row_idx);
            }
            types::Value::Error(types::Error::Reference)
        };

        // Parse and calculate for this row
        let parsed = parse_formula::parse_string_to_formula(
            &processed_formula,
            None::<NoCustomFunction>
        );
        let result = calculate::calculate_formula(parsed, Some(&resolver));

        // Convert result to appropriate type
        match result {
            types::Value::Number(n) => {
                let value = n as f64;
                let rounded = (value * 1e6).round() / 1e6;
                number_results.push(rounded);
                if result_type.is_none() {
                    result_type = Some("number");
                }
            }
            types::Value::Text(t) => {
                text_results.push(t);
                if result_type.is_none() {
                    result_type = Some("text");
                }
            }
            types::Value::Boolean(b) => {
                let bool_val = match b {
                    types::Boolean::True => true,
                    types::Boolean::False => false,
                };
                bool_results.push(bool_val);
                if result_type.is_none() {
                    result_type = Some("boolean");
                }
            }
            types::Value::Error(e) => {
                return Err(ForgeError::Eval(format!(
                    "Formula '{}' at row {} returned error: {:?}",
                    formula_str, row_idx, e
                )));
            }
            other => {
                return Err(ForgeError::Eval(format!(
                    "Formula '{}' returned unexpected type: {:?}",
                    formula_str, other
                )));
            }
        }
    }

    // Return the appropriate column type based on results
    match result_type {
        Some("number") => Ok(ColumnValue::Number(number_results)),
        Some("text") => Ok(ColumnValue::Text(text_results)),
        Some("boolean") => Ok(ColumnValue::Boolean(bool_results)),
        _ => Err(ForgeError::Eval(
            "Formula did not produce any valid results".to_string(),
        )),
    }
}
```text

### Variable Resolution

**Resolver Closure Pattern:**

The resolver is a closure that maps variable names to values for a specific row.

```rust
let resolver = |var_name: String| -> types::Value {
    if let Some(col) = table.columns.get(&var_name) {
        // Get value at current row_idx
        match &col.values {
            ColumnValue::Number(nums) => {
                if let Some(&val) = nums.get(row_idx) {
                    return types::Value::Number(val as f32);
                }
            }
            ColumnValue::Text(texts) => {
                if let Some(text) = texts.get(row_idx) {
                    return types::Value::Text(text.clone());
                }
            }
            ColumnValue::Boolean(bools) => {
                if let Some(&val) = bools.get(row_idx) {
                    return types::Value::Boolean(
                        if val { types::Boolean::True } else { types::Boolean::False }
                    );
                }
            }
            ColumnValue::Date(dates) => {
                if let Some(date) = dates.get(row_idx) {
                    return types::Value::Text(date.clone());
                }
            }
        }
    }
    types::Value::Error(types::Error::Reference)
};
```text

### Cross-Table References

**Syntax:** `=other_table.column`

**Example:**

```yaml
pl_2025:
  columns:
    revenue: [100000, 120000, 150000, 180000]

pl_2026:
  columns:
    revenue: [120000, 144000, 180000, 216000]
  row_formulas:
    growth: "=pl_2026.revenue / pl_2025.revenue - 1"
```text

**Evaluation:**

```text
Row 0: growth[0] = pl_2026.revenue[0] / pl_2025.revenue[0] - 1
               = 120000 / 100000 - 1 = 0.2 (20% growth)
```text

**Resolution Logic:**

```rust
if var_name.contains('.') {
    let parts: Vec<&str> = var_name.split('.').collect();
    if parts.len() == 2 {
        let ref_table_name = parts[0];
        let ref_col_name = parts[1];

        if let Some(ref_table) = self.model.tables.get(ref_table_name) {
            if let Some(ref_col) = ref_table.columns.get(ref_col_name) {
                return get_column_value_at_row(ref_col, row_idx);
            }
        }
    }
    return types::Value::Error(types::Error::Value);
}
```text

---

## Aggregation Formula Evaluation

### Concept

Aggregation formulas reduce an **entire column array** to a **single scalar value**.

**Common Patterns:**

```yaml

# Simple aggregations

total_revenue:
  value: null
  formula: "=SUM(pl_2025.revenue)"

avg_margin:
  value: null
  formula: "=AVERAGE(pl_2025.margin)"

max_revenue:
  value: null
  formula: "=MAX(pl_2025.revenue)"

# Conditional aggregations

high_revenue_count:
  value: null
  formula: "=COUNTIF(pl_2025.revenue, \">100000\")"

total_high_margin:
  value: null
  formula: "=SUMIF(pl_2025.margin, \">0.7\", pl_2025.revenue)"
```text

### Aggregation Types

**1. Simple Aggregations**

```text
SUM(column)       → Σ column[i]
AVERAGE(column)   → (Σ column[i]) / n
MAX(column)       → max(column[i])
MIN(column)       → min(column[i])
COUNT(column)     → count of non-empty values
```text

**2. Conditional Aggregations**

```text
SUMIF(range, criteria, sum_range)
  → Σ sum_range[i] where range[i] meets criteria

COUNTIF(range, criteria)
  → count of range[i] where range[i] meets criteria

AVERAGEIF(range, criteria, avg_range)
  → (Σ avg_range[i] where range[i] meets criteria) / count
```text

**3. Multi-Condition Aggregations**

```text
SUMIFS(sum_range, criteria_range1, criteria1, criteria_range2, criteria2, ...)
  → Σ sum_range[i] where all criteria are met
```text

### Evaluation Algorithm

```plantuml
@startuml aggregation-evaluation
!theme plain
title Aggregation Formula Evaluation

start

:Receive formula\n(e.g., "=SUM(table.column)");

:Detect aggregation type;

if (Simple aggregation?) then (yes)
  :Extract table.column;
  :Get column array;
  :Apply aggregation\n(SUM, AVG, MAX, MIN);
  :Return scalar result;
  stop
elseif (Conditional?) then (conditional)
  :Extract criteria range,\ncriteria, sum range;
  :Get all column arrays;
  :Filter rows by criteria;
  :Apply aggregation\nto filtered rows;
  :Return scalar result;
  stop
else (complex)
  :Preprocess formula;
  :Use xlformula_engine\nwith resolver;
  :Return scalar result;
  stop
endif

@enduml
```text

### Code Implementation

```rust
// From: /home/rex/src/utils/forge/src/core/array_calculator.rs:672-753
fn evaluate_aggregation(&self, formula: &str) -> ForgeResult<f64> {
    let upper = formula.to_uppercase();

    // Check for conditional aggregations first
    if upper.contains("SUMIF(") {
        return self.evaluate_conditional_aggregation(formula, "SUMIF");
    } else if upper.contains("COUNTIF(") {
        return self.evaluate_conditional_aggregation(formula, "COUNTIF");
    } else if upper.contains("AVERAGEIF(") {
        return self.evaluate_conditional_aggregation(formula, "AVERAGEIF");
    }
    // ... more conditional types ...

    // Extract function name and argument for simple aggregations
    let (func_name, arg) = if let Some(start) = upper.find("SUM(") {
        ("SUM", self.extract_function_arg(formula, start + 4)?)
    } else if let Some(start) = upper.find("AVERAGE(") {
        ("AVERAGE", self.extract_function_arg(formula, start + 8)?)
    } else if let Some(start) = upper.find("MAX(") {
        ("MAX", self.extract_function_arg(formula, start + 4)?)
    } else if let Some(start) = upper.find("MIN(") {
        ("MIN", self.extract_function_arg(formula, start + 4)?)
    } else if let Some(start) = upper.find("COUNT(") {
        ("COUNT", self.extract_function_arg(formula, start + 6)?)
    } else {
        return Err(ForgeError::Eval(format!(
            "Unknown aggregation formula: {}",
            formula
        )));
    };

    // Parse table.column reference
    let (table_name, col_name) = self.parse_table_column_ref(&arg)?;

    // Get the column
 let table = self.model.tables.get(table_name).ok_or_else(|| {
        ForgeError::Eval(format!("Table '{}' not found", table_name))
    })?;
 let column = table.columns.get(col_name).ok_or_else(|| {
        ForgeError::Eval(format!(
            "Column '{}' not found in table '{}'",
            col_name, table_name
        ))
    })?;

    // Apply aggregation based on function
    match func_name {
        "SUM" => {
            if let ColumnValue::Number(nums) = &column.values {
                Ok(nums.iter().sum())
            } else {
                Err(ForgeError::Eval(format!(
                    "SUM requires numeric column, got {}",
                    column.values.type_name()
                )))
            }
        }
        "AVERAGE" => {
            if let ColumnValue::Number(nums) = &column.values {
                let sum: f64 = nums.iter().sum();
                let count = nums.len() as f64;
                Ok(sum / count)
            } else {
                Err(ForgeError::Eval(format!(
                    "AVERAGE requires numeric column, got {}",
                    column.values.type_name()
                )))
            }
        }
        "MAX" => {
            if let ColumnValue::Number(nums) = &column.values {
                nums.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                    .into()
            } else {
                Err(ForgeError::Eval(format!(
                    "MAX requires numeric column, got {}",
                    column.values.type_name()
                )))
            }
        }
        "MIN" => {
            if let ColumnValue::Number(nums) = &column.values {
                Ok(nums.iter().copied().fold(f64::INFINITY, f64::min))
            } else {
                Err(ForgeError::Eval(format!(
                    "MIN requires numeric column, got {}",
                    column.values.type_name()
                )))
            }
        }
        "COUNT" => Ok(column.values.len() as f64),
        _ => Err(ForgeError::Eval(format!(
            "Unknown aggregation function: {}",
            func_name
        ))),
    }
}
```text

### Conditional Aggregations

**SUMIF Example:**

```yaml

# Sum revenue where margin > 0.7

high_margin_revenue:
  value: null
  formula: "=SUMIF(pl_2025.margin, \">0.7\", pl_2025.revenue)"
```text

**Evaluation:**

```text
For i in 0..row_count:
  if margin[i] > 0.7:
    sum += revenue[i]

Result: sum
```text

**Implementation:**

```rust
fn evaluate_sumif(&self, range: &Column, criteria: &str, sum_range: &Column)
    -> ForgeResult<f64>
{
    // Parse criteria (e.g., ">0.7", "=High", "<>0")
    let (operator, threshold) = parse_criteria(criteria)?;

    // Get numeric arrays
    let range_nums = match &range.values {
        ColumnValue::Number(nums) => nums,
        _ => return Err(ForgeError::Eval("SUMIF range must be numeric".to_string())),
    };
    let sum_nums = match &sum_range.values {
        ColumnValue::Number(nums) => nums,
        _ => return Err(ForgeError::Eval("SUMIF sum_range must be numeric".to_string())),
    };

    // Validate lengths match
    if range_nums.len() != sum_nums.len() {
        return Err(ForgeError::Eval("Range and sum_range must have equal length".to_string()));
    }

    // Apply conditional sum
    let mut sum = 0.0;
    for (i, &range_val) in range_nums.iter().enumerate() {
        if meets_criteria(range_val, operator, threshold) {
            sum += sum_nums[i];
        }
    }

    Ok(sum)
}
```text

---

## Custom Function Preprocessing

### Problem

xlformula_engine v0.1.18 has limited function support. Some functions need preprocessing before evaluation.

**Unsupported Functions:**

- `ROUND`, `ROUNDUP`, `ROUNDDOWN`, `CEILING`, `FLOOR`
- `SQRT`, `POWER` (partially supported)
- `CONCAT`, `TRIM`, `UPPER`, `LOWER`, `LEN`, `MID`
- `TODAY`, `DATE`, `YEAR`, `MONTH`, `DAY`

### Solution: Preprocessing

Convert unsupported functions to equivalent expressions before sending to xlformula_engine.

**Examples:**

```text
ROUND(value, 2)           → custom Rust implementation
SQRT(value)               → POWER(value, 0.5)
CEILING(value)            → custom implementation
CONCAT(a, b)              → a & b (text concatenation)
TRIM(text)                → custom preprocessing
TODAY()                   → "2025-11-24" (current date)
```text

### Implementation

```rust
// From: /home/rex/src/utils/forge/src/core/array_calculator.rs
fn preprocess_custom_functions(&self, formula: &str, row_idx: usize, table: &Table)
    -> ForgeResult<String>
{
    let mut result = formula.to_string();

    // Preprocess ROUND, ROUNDUP, ROUNDDOWN
    if self.has_custom_math_function(formula) {
        result = self.preprocess_math_functions(&result, row_idx, table)?;
    }

    // Preprocess text functions
    if self.has_custom_text_function(formula) {
        result = self.preprocess_text_functions(&result, row_idx, table)?;
    }

    // Preprocess date functions
    if self.has_custom_date_function(formula) {
        result = self.preprocess_date_functions(&result)?;
    }

    Ok(result)
}
```text

**ROUND Preprocessing:**

```rust
fn preprocess_round(&self, formula: &str, row_idx: usize, table: &Table)
    -> ForgeResult<String>
{
    use regex::Regex;

    // Match: ROUND(expression, digits)
    let re = Regex::new(r"ROUND\(([^,]+),\s*(\d+)\)")?;

    let mut result = formula.to_string();
    for cap in re.captures_iter(formula) {
        let expr = cap.get(1).unwrap().as_str();
        let digits = cap.get(2).unwrap().as_str().parse::<i32>()?;

        // Evaluate expression to get value
        let value = self.evaluate_expression(expr, row_idx, table)?;

        // Apply rounding
        let rounded = if digits >= 0 {
            let multiplier = 10_f64.powi(digits);
            (value * multiplier).round() / multiplier
        } else {
            let divisor = 10_f64.powi(-digits);
            (value / divisor).round() * divisor
        };

        // Replace ROUND(...) with the rounded value
        result = result.replace(cap.get(0).unwrap().as_str(), &rounded.to_string());
    }

    Ok(result)
}
```text

**TODAY Preprocessing:**

```rust
fn preprocess_today(&self, formula: &str) -> String {
    use chrono::Local;

    let today = Local::now().format("%Y-%m-%d").to_string();
    formula.replace("TODAY()", &format!("\"{}\"", today))
}
```text

---

## Variable Resolution

### Scoping Rules

**1. Row-wise Formulas (Table Context)**

Variables resolve to **column values at specific row index**.

```yaml
pl_2025:
  columns:
    revenue: [100, 120, 150]
  row_formulas:
    profit: "=revenue * 0.2"  # revenue resolves to revenue[row_idx]
```text

**2. Scalar Formulas (Global Context)**

Variables resolve to:

1. Other scalar values
2. Aggregations over columns
3. Array indexing (e.g., `table.column[0]`)

```yaml
summary:
  total: "=SUM(pl_2025.revenue)"  # Aggregation
  first: "=pl_2025.revenue[0]"     # Array indexing
  margin: "=total / 100000"        # Scalar reference
```text

**3. Cross-File References (v0.2.0)**

Variables resolve via alias prefix.

```yaml
includes:

  - file: pricing.yaml
    as: pricing

revenue: "=@pricing.base_price * volume"
```text

### Resolution Algorithm

```plantuml
@startuml variable-resolution
!theme plain
title Variable Resolution Algorithm

start

:Encounter variable name\n(e.g., "revenue");

if (In row-wise context?) then (yes)
  :Resolve to column value\nat current row index;
  :revenue → revenue[row_idx];
  stop
elseif (Contains "."?) then (table.column)
  if (Array indexing?) then (yes)
    :table.column[idx];
    :Get column value at index;
    stop
  else (aggregation)
    :table.column;
    :Get entire column array;
    stop
  endif
elseif (Has "@" prefix?) then (cross-file)
  :@alias.variable;
  :Look up in included file;
  stop
else (scalar)
  :Look up in scalars map;
  stop
endif

@enduml
```text

---

## Formula Types and Patterns

### 1. Arithmetic Operations

```yaml

# Basic arithmetic

profit: "=revenue - costs"
margin: "=profit / revenue"
total: "=a + b + c"

# Parentheses

net: "=(revenue - cogs - opex) * (1 - tax_rate)"
```text

### 2. Logical Expressions

```yaml

# Conditional logic

category: "=IF(revenue > 100000, \"High\", \"Low\")"
valid: "=AND(margin > 0.2, revenue > 50000)"

# Nested IF

tier: "=IF(revenue > 1000000, \"A\", IF(revenue > 500000, \"B\", \"C\"))"
```text

### 3. Text Operations

```yaml

# Concatenation

full_name: "=first_name & \" \" & last_name"

# Text functions

initials: "=LEFT(first_name, 1) & LEFT(last_name, 1)"
upper_name: "=UPPER(full_name)"
```text

### 4. Date Calculations

```yaml

# Date functions

current_year: "=YEAR(TODAY())"
age: "=YEAR(TODAY()) - YEAR(birthdate)"
```text

### 5. Aggregations

```yaml

# Simple aggregations

total_revenue: "=SUM(pl_2025.revenue)"
avg_margin: "=AVERAGE(pl_2025.margin)"
max_sales: "=MAX(sales.monthly_total)"

# Conditional aggregations

high_performers: "=COUNTIF(employees.rating, \">4\")"
bonus_pool: "=SUMIF(employees.rating, \">=4\", employees.salary)"
```text

### 6. Array Indexing

```yaml

# Indexing into arrays

first_month: "=pl_2025.revenue[0]"
last_month: "=pl_2025.revenue[3]"

# Growth calculations

cagr: "=(pl_2025.revenue[3] / pl_2025.revenue[0]) ^ (1/3) - 1"
```text

### 7. Cross-Table References

```yaml
pl_2026:
  row_formulas:
    # Reference another table's column
    growth: "=(revenue / pl_2025.revenue) - 1"
    yoy_change: "=revenue - pl_2025.revenue"
```text

---

## Performance Optimization

### Time Complexity

**Row-wise Formula Evaluation:**

```text
Per table: O(rows * columns * formula_complexity)
Total: O(Σ(table.rows * table.calculated_cols))
```text

**Example:**

- 4 tables with 12 rows each
- 3 calculated columns per table
- Formula complexity: O(1) (simple arithmetic)
- Total: 4 * 12 * 3 = 144 formula evaluations

**Aggregation Evaluation:**

```text
Per aggregation: O(column_length)
Total: O(Σ(aggregation.column_length))
```text

### Space Complexity

**Per Formula Evaluation:**

- Resolver closure: O(1) stack allocation
- Result vector: O(rows) for row-wise formulas
- xlformula_engine AST: O(formula_length)

**Total Model:**

- Input model: O(tables * rows * columns)
- Calculated values: O(calculated_cols * rows)
- Total: O(tables * rows * (data_cols + calc_cols))

### Optimization Strategies

**1. Dependency Ordering**

Calculate formulas in dependency order to avoid recalculation.

```rust
// Instead of: Calculate all, detect missing deps, recalculate
// Do: Topological sort first, calculate once
let order = toposort(&dependency_graph, None)?;
for var in order {
    calculate(var);  // All dependencies already calculated
}
```text

**2. Lazy Evaluation** (Not yet implemented)

```rust
// Future optimization: Only calculate requested columns
if !requested_columns.contains(col_name) {
    continue;  // Skip calculation
}
```text

**3. Parallel Row Evaluation** (Not yet implemented)

```rust
// Row-wise formulas are independent - can parallelize
use rayon::prelude::*;

let results: Vec<f64> = (0..row_count)
    .into_par_iter()  // Parallel iterator
 .map(|row_idx| evaluate_formula(formula, row_idx))
    .collect();
```text

**4. Formula Caching**

Cache parsed formulas to avoid re-parsing.

```rust
// Current: Parse formula string on every evaluation
let parsed = parse_formula::parse_string_to_formula(formula, None);

// Future: Cache parsed AST
let parsed = formula_cache.get_or_insert(formula, || {
    parse_formula::parse_string_to_formula(formula, None)
});
```text

### Performance Benchmarks

**Real-world Test Case:**

- File: `test-data/v1.0/quarterly_pl.yaml`
- Tables: 4
- Rows per table: 12
- Total formulas: 850+

**Results:**

- Total time: <200ms
- Average per formula: ~0.23ms
- Parsing: ~50ms
- Calculation: ~100ms
- Writing: ~50ms

**Comparison to AI Validation:**

- AI validation: 128 minutes (7,680 seconds)
- Forge: 0.2 seconds
- Speedup: 38,400x faster

---

## Error Handling

### Error Types

**1. Formula Syntax Errors**

```yaml
profit: "=revenue -"  # Incomplete expression

# Error: Parse error: Unexpected end of formula

```text

**2. Reference Errors**

```yaml
profit: "=revenue - cost"  # 'cost' doesn't exist

# Error: Column 'cost' not found in table

```text

**3. Type Errors**

```yaml
result: "=revenue + quarter"  # Number + Text

# Error: Type mismatch: Cannot add Number and Text

```text

**4. Length Mismatch Errors**

```yaml
pl_2025:
  columns:
    revenue: [100, 120, 150]  # 3 rows
    costs: [30, 40, 50, 60]   # 4 rows
  row_formulas:
    profit: "=revenue - costs"

# Error: Column 'costs' has 4 rows, expected 3 rows

```text

**5. Circular Dependency Errors**

```yaml
a:
  formula: "=b * 2"
b:
  formula: "=a + 1"

# Error: Circular dependency detected: a → b → a

```text

### Error Context

Forge provides detailed error context:

```rust
ForgeError::Eval(format!(
    "Formula '{}' at row {} in table '{}': Column '{}' not found",
    formula, row_idx, table_name, col_name
))
```text

**Example Error Message:**

```text
Error: Evaluation error: Formula '=revenue - cost' at row 2 in table 'pl_2025':
Column 'cost' not found

Available columns: revenue, cogs, quarter
```text

### Error Recovery

**Fail-Fast Strategy:**

Forge does not attempt to recover from formula errors. On first error:

1. Stop calculation
2. Report error with full context
3. Return to user for correction

**Why fail-fast?**

- Financial calculations must be 100% accurate
- Partial results are misleading
- User must fix the issue, not work around it

---

## Related Documentation

### Architecture Deep Dives

- [00-OVERVIEW.md](00-OVERVIEW.md) - High-level architecture
- [01-COMPONENT-ARCHITECTURE.md](01-COMPONENT-ARCHITECTURE.md) - Component interactions
- [02-DATA-MODEL.md](02-DATA-MODEL.md) - Type system and data structures
- [04-DEPENDENCY-RESOLUTION.md](04-DEPENDENCY-RESOLUTION.md) - Graph algorithms
- [05-EXCEL-INTEGRATION.md](05-EXCEL-INTEGRATION.md) - Excel conversion
- [06-CLI-ARCHITECTURE.md](06-CLI-ARCHITECTURE.md) - Command structure
- [07-TESTING-ARCHITECTURE.md](07-TESTING-ARCHITECTURE.md) - Test strategy

### Source Files Referenced

- `/home/rex/src/utils/forge/src/core/array_calculator.rs` - Main calculation engine (3,440 lines)
- `/home/rex/src/utils/forge/src/core/calculator.rs` - v0.2.0 calculator (401 lines)

### External Documentation

- xlformula_engine: https://crates.io/crates/xlformula_engine
- Excel Function Reference: https://support.microsoft.com/en-us/office/excel-functions-alphabetical

---

**Previous:** [← Data Model](02-DATA-MODEL.md)
**Next:** [Dependency Resolution →](04-DEPENDENCY-RESOLUTION.md)
