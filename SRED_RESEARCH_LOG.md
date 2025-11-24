# SR&ED Research and Development Log

**Project:** RoyalBit Forge - YAML Formula Calculator with Excel Compatibility

**SR&ED Program:** Scientific Research & Experimental Development (Canada)
**Purpose:** Document technical challenges, hypotheses, experiments, and advancements for Canadian tax credit eligibility

---

## About SR&ED Eligibility

This project qualifies for SR&ED tax credits because it involves:
- **Technological Advancement:** Creating a novel approach to YAML-based formula calculation with Excel-compatible array model
- **Technical Uncertainty:** Resolving complex problems in formula parsing, dependency resolution, and type-safe array operations
- **Systematic Investigation:** Iterative design, prototyping, testing of various approaches

**Key SR&ED Activities:**
- Algorithm development (dependency resolution, topological sorting)
- Data structure design (type-safe column arrays, unified model)
- Performance optimization (zero-copy operations, efficient formula evaluation)
- Experimental testing (property-based testing, mutation testing, fuzzing)

---

## Research Entries

### Entry 1: Formula Engine Selection (v0.2.0)
**Date:** 2025-11-23
**Challenge:** Excel-compatible formula evaluation in Rust

**Technical Uncertainty:**
- Initial approach used `meval` crate for simple math expressions
- Could not support Excel functions (SUM, AVERAGE, IF, etc.)
- Needed Excel compatibility for professional use

**Hypothesis:**
Use `xlformula_engine` crate to gain Excel function support

**Experiment:**
1. Evaluated alternative formula engines:
   - `meval` - simple but no Excel functions ❌
   - `xlformula_engine` - Excel-compatible, actively maintained ✅
   - Custom parser - too complex, high maintenance burden ❌
2. Implemented prototype with `xlformula_engine`
3. Tested with financial model (850+ formulas)

**Results:**
- ✅ Successfully replaced meval with xlformula_engine
- ✅ Gained support for SUM, AVERAGE, PRODUCT, IF, ABS, MAX, MIN
- ✅ Performance acceptable (<250ms for 850 formulas)
- ✅ Full backwards compatibility maintained

**Technological Advancement:**
Created bridge between YAML data model and Excel formula engine, enabling business users to use familiar Excel syntax in version-controlled YAML files.

---

### Entry 2: Array Model Design (v1.0.0)
**Date:** 2025-11-23
**Challenge:** Excel export requires 1:1 mapping between YAML and Excel columns

**Technical Uncertainty:**
- v0.2.0 scalar model stores individual values: `revenue: {value: 100}`
- Excel uses column arrays: `revenue = [100, 200, 300, 400]`
- No clear path to convert scalar formulas to Excel cell formulas
- Risk of incorrect conversion or data loss

**Hypothesis:**
Design native array model where YAML column arrays map directly to Excel columns

**Design Alternatives Considered:**

1. **Auto-convert scalars to arrays** ❌
   - Unclear semantics (how to infer row count?)
   - Risk of data loss
   - Difficult to detect errors

2. **Native array model (chosen)** ✅
   - Explicit column arrays: `revenue: [1000, 1200, 1500, 1800]`
   - Row-wise formulas: `profit = revenue - expenses`
   - Direct 1:1 Excel mapping
   - Type-safe validation

3. **Dual representation** ❌
   - Maintain both scalar and array representations
   - Complex synchronization logic
   - High maintenance burden

**Experimental Approach:**
1. Designed type-safe column value enum (Number, Text, Date, Boolean)
2. Implemented homogeneous type validation
3. Created JSON Schema for validation
4. Built parser with automatic version detection (v0.2.0 vs v1.0.0)
5. Tested backwards compatibility with existing models

**Results:**
- ✅ Type-safe array parsing with validation
- ✅ 100% backwards compatibility with v0.2.0 models
- ✅ JSON Schema validation (zero invalid models pass)
- ✅ Test coverage: 29/29 unit tests + 3/3 integration tests

**Technological Advancement:**
Created unified parser that supports both legacy scalar model (v0.2.0) and new array model (v1.0.0), enabling gradual migration without breaking existing users.

---

### Entry 3: Row-wise Formula Evaluation (Phase 2 Part 1)
**Date:** 2025-11-23
**Challenge:** Evaluate formulas element-wise across array columns

**Technical Uncertainty:**
- Standard formula engines expect scalar values
- Array formulas require evaluating formula once per row
- Must handle dependencies between calculated columns
- Risk of incorrect evaluation order (circular dependencies)

**Hypothesis:**
Use topological sorting to determine calculation order, then evaluate formula per row with row-specific variable resolver

**Technical Approach:**

1. **Dependency Graph Construction:**
   ```rust
   // Build directed graph of formula dependencies
   for (col_name, formula) in table.row_formulas {
       let deps = extract_column_references(formula);
       for dep in deps {
           graph.add_edge(dep, col_name);  // dep → col_name
       }
   }
   ```

2. **Topological Sort:**
   - Detects circular dependencies (compile-time guarantee)
   - Returns calculation order
   - Example: revenue → expenses → profit → margin

3. **Row-wise Evaluation:**
   ```rust
   for row_idx in 0..row_count {
       let resolver = |var_name: String| -> Value {
           let column = table.get(var_name);
           column.values[row_idx]  // Get value at THIS row
       };
       result[row_idx] = evaluate_formula(formula, resolver);
   }
   ```

**Challenges Encountered:**

**Challenge 3.1: Borrow Checker Issues**
- Cannot mutate `self.model.tables` while iterating
- Solution: Clone table names first, then iterate
  ```rust
  let table_names: Vec<String> = self.model.tables.keys().cloned().collect();
  for table_name in table_names { ... }
  ```

**Challenge 3.2: Formula Parser Expects `f32`, We Use `f64`**
- xlformula_engine uses `f32` for calculations
- Forge uses `f64` for precision
- Solution: Cast at boundaries, round results to 6 decimals
  ```rust
  let value = result as f64;
  let rounded = (value * 1e6).round() / 1e6;
  ```

**Results:**
- ✅ Row-wise formulas working correctly
- ✅ Dependency resolution with circular detection
- ✅ Test: revenue=[1000,1200,1500,1800], cogs=[300,360,450,540]
  - gross_profit = [700, 840, 1050, 1260] ✅
  - gross_margin = [0.7, 0.7, 0.7, 0.7] ✅

**Technological Advancement:**
Created array-aware formula calculator that bridges row-wise array operations with Excel-style formula syntax, maintaining Excel semantics while operating on entire columns.

---

### Entry 4: Aggregation Formulas and Scalar Calculation (Phase 2 Part 2)
**Date:** 2025-11-23
**Status:** IN PROGRESS
**Challenge:** Support aggregation functions (SUM, AVERAGE, MAX, MIN) on table columns

**Technical Uncertainty:**
- Aggregations reduce arrays to scalars: `SUM([100,200,300]) → 600`
- Must support cross-table references: `SUM(pl_2025.revenue)`
- Must support array indexing: `revenue[3]` (get specific element)
- Must handle scalar dependencies: `avg_margin = total_profit / total_revenue`
- Risk of incorrect reference resolution (Bug #1: variable scoping issue documented)

**Hypothesis 4.1: Table.Column Reference Parsing**
Parse `table.column` syntax to extract table and column names, then look up column in model

**Approach:**
```rust
fn parse_table_column_ref(ref_str: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = ref_str.split('.').collect();
    if parts.len() == 2 {
        Some((parts[0].to_string(), parts[1].to_string()))
    } else {
        None  // Not a table.column reference
    }
}
```

**Hypothesis 4.2: Array Indexing**
Parse `table.column[index]` syntax, extract array element at index

**Approach:**
```rust
// Pattern: table.column[3]
if ref_str.contains('[') && ref_str.ends_with(']') {
    let parts: Vec<&str> = ref_str.split('[').collect();
    let table_col = parts[0];  // "table.column"
    let index_str = parts[1].trim_end_matches(']');  // "3"
    let index = index_str.parse::<usize>()?;

    let (table, col) = parse_table_column_ref(table_col)?;
    let column = model.tables.get(table)?.columns.get(col)?;

    match &column.values {
        ColumnValue::Number(nums) => nums.get(index).copied(),
        _ => None
    }
}
```

**Hypothesis 4.3: Aggregation Function Evaluation**
Detect SUM/AVERAGE/MAX/MIN, extract table.column reference, apply aggregation

**Approach:**
```rust
fn evaluate_aggregation(formula: &str) -> Result<f64> {
    if formula.contains("SUM(") {
        let ref_str = extract_arg(formula, "SUM")?;  // "pl_2025.revenue"
        let (table, col) = parse_table_column_ref(ref_str)?;
        let column = get_column(table, col)?;

        match &column.values {
            ColumnValue::Number(nums) => Ok(nums.iter().sum()),
            _ => Err("SUM requires numeric column")
        }
    }
    // Similar for AVERAGE, MAX, MIN
}
```

**Hypothesis 4.4: Scalar Dependency Resolution**
Use topological sort (like row-wise formulas) to calculate scalars in dependency order

**Experiment Plan:**
1. ✅ Implement table.column reference parsing
2. ⏸️ Implement array indexing parsing
3. ⏸️ Implement aggregation function evaluation (SUM, AVERAGE, MAX, MIN)
4. ⏸️ Implement scalar dependency graph and topological sort
5. ⏸️ Test with quarterly_pl.yaml (has all patterns)

**Expected Results:**
- annual_2025.total_revenue = SUM(pl_2025.revenue) = 5,500,000
- annual_2025.avg_gross_margin = 0.7 (calculated from other scalars)
- growth.revenue_q4_vs_q1 = 0.8 (1800/1000 - 1)

**Current Status:** Implementing aggregation formula support...

---

## SR&ED Documentation Guidelines

When adding entries to this log, document:

1. **Technical Challenge:**
   - What problem are we trying to solve?
   - Why is it uncertain/difficult?
   - What makes it non-obvious?

2. **Hypothesis:**
   - What approach do we think will work?
   - What alternatives did we consider?
   - Why did we choose this approach?

3. **Experiment:**
   - What did we implement?
   - What tests did we run?
   - What measurements did we take?

4. **Results:**
   - Did it work? Why or why not?
   - What did we learn?
   - Were there unexpected findings?

5. **Technological Advancement:**
   - What new capability did we create?
   - How does it advance the state of the art?
   - What can users now do that they couldn't before?

---

## Tips for SR&ED Eligibility

**Qualifying Activities:**
- ✅ Algorithm design and optimization
- ✅ Performance analysis and improvements
- ✅ Experimental testing approaches (property-based, mutation, fuzzing)
- ✅ Resolving technical uncertainties (formula parsing, dependency resolution, type safety)
- ✅ Creating novel data structures and abstractions

**Non-Qualifying Activities:**
- ❌ Routine coding (following established patterns)
- ❌ UI design and styling
- ❌ Documentation writing (unless documenting research)
- ❌ Bug fixes for simple typos or logic errors
- ❌ Standard library integration

**Documentation Best Practices:**
- Write entries DURING development (not after)
- Be specific about technical challenges
- Document dead ends and failed approaches
- Quantify results (performance, test coverage, etc.)
- Link to commits, test results, benchmarks

---

**Last Updated:** 2025-11-23
**Total Research Entries:** 4 (1 complete + 3 in progress)
