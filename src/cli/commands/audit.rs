//! `forge audit`: what a value is calculated from, all the way down.
//!
//! Each name a formula reads is resolved the way the calculator resolves it:
//! in a row formula, a bare name is a column of the same table first; in a
//! scalar formula, a bare name is a scalar of the formula's own group first,
//! then a top-level scalar, then the only scalar with that last part;
//! `table.column` is a column; `@costs.labour.wage` is a value of the file
//! included as `costs`, whose own formulas are followed inside that file.
//! Values are the calculated ones. A node that appears again further down is
//! shown once, then marked as repeated.

use crate::cli::console::{out, outln};
use crate::cli::Console;
use crate::core::array_calculator::tokenizer::{tokenize, Token};
use crate::core::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use crate::types::{ColumnValue, ParsedModel};
use colored::Colorize;
use std::collections::HashSet;
use std::path::Path;

use super::format_number;
use super::results::{AuditDep, AuditResult};

/// One model to resolve names in: the main file or an included one.
struct Scope<'a> {
    /// As parsed (formulas)
    model: &'a ParsedModel,
    /// As calculated (values), when it calculates
    calculated: Option<ParsedModel>,
    /// The included file, for an include (`None` for the main file)
    file: Option<String>,
    /// `@namespace.` for an include: its names as the including file writes them
    prefix: String,
    /// The files it includes, by namespace, in name order
    includes: Vec<(String, Self)>,
}

impl<'a> Scope<'a> {
    fn new(model: &'a ParsedModel, file: Option<String>, prefix: String) -> Self {
        let calculated = ArrayCalculator::new(model.clone()).calculate_all().ok();
        let mut includes: Vec<(String, Self)> = model
            .resolved_includes
            .iter()
            .map(|(ns, r)| {
                (
                    ns.clone(),
                    Self::new(&r.model, Some(r.include.file.clone()), format!("@{ns}.")),
                )
            })
            .collect();
        includes.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            model,
            calculated,
            file,
            prefix,
            includes,
        }
    }

    fn scalar_value(&self, name: &str) -> Option<f64> {
        self.calculated
            .as_ref()
            .unwrap_or(self.model)
            .scalars
            .get(name)
            .and_then(|s| s.value)
    }

    fn scalar_date(&self, name: &str) -> Option<String> {
        self.calculated
            .as_ref()
            .unwrap_or(self.model)
            .scalars
            .get(name)
            .and_then(crate::types::Variable::date_text)
    }

    fn column_values(&self, table: &str, column: &str) -> Option<Vec<Option<f64>>> {
        let model = self.calculated.as_ref().unwrap_or(self.model);
        match &model.tables[table].columns.get(column)?.values {
            ColumnValue::Number(v) => Some(v.iter().copied().map(Some).collect()),
            ColumnValue::Blanks(v) => Some(v.clone()),
            _ => None,
        }
    }

    /// A scalar the way the calculator resolves a name in group `section`.
    fn resolve_scalar(&self, word: &str, section: Option<&str>) -> Option<String> {
        let scalars = &self.model.scalars;
        if let Some(section) = section {
            if !word.contains('.') {
                let scoped = format!("{section}.{word}");
                if scalars.contains_key(&scoped) {
                    return Some(scoped);
                }
            }
        }
        if scalars.contains_key(word) {
            return Some(word.to_string());
        }
        if word.contains('.') {
            return None;
        }
        let mut matches = scalars
            .keys()
            .filter(|name| name.rsplit_once('.').is_some_and(|(_, last)| last == word));
        match (matches.next(), matches.next()) {
            (Some(only), None) => Some(only.clone()),
            _ => None,
        }
    }
}

/// What a formula is written in: a scalar (its group) or a table's row formula.
#[derive(Clone, Copy)]
enum Place<'a> {
    Scalar { section: Option<&'a str> },
    Row { table: &'a str },
}

/// The names a formula reads: every identifier that is not a function name.
fn formula_references(formula: &str) -> Vec<String> {
    let body = formula.trim().trim_start_matches('=');
    let Ok(tokens) = tokenize(body) else {
        return Vec::new();
    };
    let mut refs: Vec<String> = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        if let Token::Identifier(word) = token {
            let is_call = matches!(tokens.get(i + 1), Some(Token::OpenParen));
            if !is_call && !refs.contains(word) {
                refs.push(word.clone());
            }
        }
    }
    refs
}

struct Builder<'a> {
    main: Scope<'a>,
    /// Nodes already expanded, by scope and name
    seen: HashSet<String>,
}

/// Where a name is resolved: the indexes of the includes followed from the
/// main file (empty for the main file itself).
type At = Vec<usize>;

impl<'a> Builder<'a> {
    fn new(model: &'a ParsedModel) -> Self {
        Self {
            main: Scope::new(model, None, String::new()),
            seen: HashSet::new(),
        }
    }

    fn scope(&self, at: &[usize]) -> &Scope<'a> {
        at.iter().fold(&self.main, |scope, &i| &scope.includes[i].1)
    }

    /// Whether `label` in `at` was expanded already (and mark it if not).
    fn seen_before(&mut self, at: &[usize], label: &str) -> bool {
        !self.seen.insert(format!("{at:?} {label}"))
    }

    /// The dependencies of `formula`, written at `place` in `scope`.
    fn children(
        &mut self,
        at: &[usize],
        formula: &str,
        place: Place<'_>,
        depth: usize,
    ) -> Vec<AuditDep> {
        formula_references(formula)
            .into_iter()
            .map(|word| self.node(at, &word, place, depth))
            .collect()
    }

    fn node(&mut self, at: &[usize], word: &str, place: Place<'_>, depth: usize) -> AuditDep {
        let scope = self.scope(at);
        let dep = AuditDep {
            name: if word.starts_with('@') {
                word.to_string()
            } else {
                format!("{}{word}", scope.prefix)
            },
            dep_type: "Unknown".to_string(),
            formula: None,
            value: None,
            children: Vec::new(),
            values: None,
            source: scope.file.clone(),
            repeated: false,
            date: None,
        };

        // A value of an included file: `@costs.labour.wage`
        if let Some(rest) = word.strip_prefix('@') {
            let (ns, path) = rest.split_once('.').unwrap_or((rest, ""));
            // An included file's own includes are followed too
            let Some(index) = scope.includes.iter().position(|(n, _)| n == ns) else {
                return dep;
            };
            let mut inner: At = at.to_vec();
            inner.push(index);
            // A column of a table of the included file: `@costs.prices.flour`
            let included = self.scope(&inner);
            if !included.model.scalars.contains_key(path) {
                if let Some((table, column)) = path.rsplit_once('.') {
                    let found = included.model.tables.get(table).is_some_and(|t| {
                        t.columns.contains_key(column) || t.row_formulas.contains_key(column)
                    });
                    if found {
                        return self.column_node(&inner, table, column, depth);
                    }
                }
            }
            return self.scalar_node(&inner, path, depth);
        }

        let column = match place {
            Place::Row { table }
                if !word.contains('.')
                    && scope.model.tables.get(table).is_some_and(|t| {
                        t.columns.contains_key(word) || t.row_formulas.contains_key(word)
                    }) =>
            {
                Some((table.to_string(), word.to_string()))
            },
            _ => None,
        };
        if let Some((table, column)) = column {
            return self.column_node(at, &table, &column, depth);
        }
        let section = match place {
            Place::Scalar { section } => section,
            Place::Row { .. } => None,
        };
        if let Some(full) = scope.resolve_scalar(word, section) {
            return self.scalar_node(at, &full, depth);
        }
        if let Some((table, column)) = word.split_once('.') {
            let found = scope.model.tables.get(table).is_some_and(|t| {
                t.columns.contains_key(column) || t.row_formulas.contains_key(column)
            });
            if found {
                return self.column_node(at, table, column, depth);
            }
        }
        dep
    }

    fn scalar_node(&mut self, at: &[usize], name: &str, depth: usize) -> AuditDep {
        let scope = self.scope(at);
        let label = format!("{}{name}", scope.prefix);
        let Some(var) = scope.model.scalars.get(name) else {
            return AuditDep::unknown(label, scope.file.clone());
        };
        let formula = var.formula.clone();
        let mut dep = AuditDep {
            name: label.clone(),
            dep_type: if at.is_empty() {
                "Scalar".to_string()
            } else {
                "Included scalar".to_string()
            },
            formula: formula.clone(),
            value: scope.scalar_value(name),
            children: Vec::new(),
            values: None,
            source: scope.file.clone(),
            repeated: false,
            date: scope.scalar_date(name),
        };
        if let Some(formula) = formula {
            if depth >= 30 || self.seen_before(at, &label) {
                dep.repeated = true;
                return dep;
            }
            let section = name.rsplit_once('.').map(|(s, _)| s.to_string());
            dep.children = self.children(
                at,
                &formula,
                Place::Scalar {
                    section: section.as_deref(),
                },
                depth + 1,
            );
        }
        dep
    }

    fn column_node(&mut self, at: &[usize], table: &str, column: &str, depth: usize) -> AuditDep {
        let scope = self.scope(at);
        let label = format!("{}{table}.{column}", scope.prefix);
        let formula = scope.model.tables[table].row_formulas.get(column).cloned();
        let mut dep = AuditDep {
            name: label.clone(),
            dep_type: format!("Column[{table}]"),
            formula: formula.clone(),
            value: None,
            children: Vec::new(),
            values: scope.column_values(table, column),
            source: scope.file.clone(),
            repeated: false,
            date: None,
        };
        if let Some(formula) = formula {
            if depth >= 30 || self.seen_before(at, &label) {
                dep.repeated = true;
                return dep;
            }
            dep.children = self.children(at, &formula, Place::Row { table }, depth + 1);
        }
        dep
    }
}

impl AuditDep {
    fn unknown(name: String, source: Option<String>) -> Self {
        Self {
            name,
            dep_type: "Unknown".to_string(),
            formula: None,
            value: None,
            children: Vec::new(),
            values: None,
            source,
            repeated: false,
            date: None,
        }
    }
}

/// Audit a variable and return structured results (no printing).
///
/// `variable` is a scalar (`year.profit`, or a short name that is unique),
/// or a table column (`months.profit`).
///
/// # Errors
///
/// Returns an error if the file cannot be parsed or the variable is not found.
pub fn audit_core(file: &Path, variable: &str, plan: bool) -> ForgeResult<AuditResult> {
    let mut model = parser::parse_model(file)?;
    // At plan, each uncertain input is its plan figure before anything is
    // calculated; a distribution that does not parse is said as any other
    // calculation error is, below
    let refused = crate::cli::commands::plan::at_plan_or_refused(&mut model, plan, "audit");
    let mut builder = Builder::new(&model);
    let target = builder
        .main
        .resolve_scalar(variable, None)
        .map(|name| builder.scalar_node(&[], &name, 0))
        .or_else(|| {
            let (table, column) = variable.split_once('.')?;
            let t = model.tables.get(table)?;
            (t.columns.contains_key(column) || t.row_formulas.contains_key(column))
                .then(|| builder.column_node(&[], table, column, 0))
        })
        .ok_or_else(|| not_found(&model, variable))?;

    let stored = model.scalars.get(&target.name).and_then(|s| s.value);
    let is_scalar = target.dep_type == "Scalar";
    let is_valid = match (stored, target.value) {
        (Some(stored), Some(calculated)) => {
            (stored - calculated).abs() <= 1e-9 * stored.abs().max(calculated.abs()).max(1.0)
        },
        _ => true,
    };
    let mut warnings = Vec::new();
    empty_cell_warnings(
        &target.name,
        target.formula.as_deref(),
        &target.children,
        &mut warnings,
    );
    warnings.dedup();
    Ok(AuditResult {
        variable: target.name.clone(),
        var_type: if is_scalar {
            "Scalar".to_string()
        } else {
            target.dep_type.clone()
        },
        current_value: stored,
        calculated_value: target.value,
        calculated_date: target.date.clone(),
        calculated_values: target.values,
        formula: target.formula,
        dependencies: target.children,
        is_valid: is_valid && builder.main.calculated.is_some(),
        calculation_error: refused
            .and_then(|()| {
                ArrayCalculator::new(model.clone())
                    .calculate_all()
                    .map(drop)
            })
            .err()
            .map(|e| e.to_string()),
        warnings,
    })
}

/// Whether `formula` calls `function`
fn calls(formula: &str, function: &str) -> bool {
    use crate::core::array_calculator::tokenizer::{tokenize, Token};
    tokenize(formula.trim().trim_start_matches('=')).is_ok_and(|tokens| {
        tokens.windows(2).any(|w| {
            matches!(&w[0], Token::Identifier(name) if name.eq_ignore_ascii_case(function))
                && matches!(w[1], Token::OpenParen)
        })
    })
}

/// NPV and IRR skip an empty cell, as in Excel, so every cash flow after it
/// moves a period earlier (ADR-059, decision 7): one warning for each
/// formula that gives one of them a column with an empty cell.
fn empty_cell_warnings(
    name: &str,
    formula: Option<&str>,
    deps: &[AuditDep],
    out: &mut Vec<String>,
) {
    if let Some(formula) = formula {
        for function in ["NPV", "IRR"] {
            if !calls(formula, function) {
                continue;
            }
            for dep in deps {
                let empty = dep
                    .values
                    .as_ref()
                    .map_or(0, |v| v.iter().filter(|n| n.is_none()).count());
                if empty > 0 {
                    out.push(format!(
                        "{name}: {function} skips the {empty} empty cell{} of {}, as Excel does, \
                         so each cash flow after one is discounted a period earlier; write 0 \
                         where a period has no cash flow",
                        if empty == 1 { "" } else { "s" },
                        dep.name
                    ));
                }
            }
        }
    }
    for dep in deps {
        empty_cell_warnings(&dep.name, dep.formula.as_deref(), &dep.children, out);
    }
}

fn not_found(model: &ParsedModel, variable: &str) -> ForgeError {
    let tables: Vec<String> = model
        .tables
        .iter()
        .flat_map(|(t, table)| {
            table
                .column_names()
                .into_iter()
                .map(move |c| format!("{t}.{c}"))
        })
        .collect();
    ForgeError::Validation(format!(
        "'{variable}' is not a scalar or a table column of the model.\n  Scalars: {}\n  Columns: {}",
        model.scalars.keys().cloned().collect::<Vec<_>>().join(", "),
        tables.join(", ")
    ))
}

/// Execute the audit command - show calculation dependency chain.
///
/// # Errors
///
/// Returns an error if the file cannot be parsed or the variable is not found.
pub fn audit(c: &mut Console<'_>, file: &Path, variable: &str, plan: bool) -> ForgeResult<()> {
    outln!(c, "{}", "🔍 Forge - Audit Trail".bold().green());
    outln!(c, "   File: {}", file.display());
    outln!(c, "   Variable: {}\n", variable.bright_blue().bold());
    crate::cli::commands::plan::print_what_if_plan(c, plan);

    let result = audit_core(file, variable, plan)?;
    outln!(c, "{}", "📋 Variable Information:".bold().cyan());
    outln!(c, "   Name: {}", result.variable.bright_blue());
    outln!(c, "   Type: {}", result.var_type.cyan());
    if let Some(ref f) = result.formula {
        outln!(c, "   Formula: {}", f.bright_yellow());
    }
    if let Some(value) = result.calculated_value {
        let shown = result
            .calculated_date
            .clone()
            .unwrap_or_else(|| format_number(value));
        outln!(c, "   Calculated: {}", shown.bold().green());
    }
    if let Some(values) = &result.calculated_values {
        outln!(c, "   Calculated: {}", summarize(values).green());
    }
    if let (Some(stored), Some(calculated)) = (result.current_value, result.calculated_value) {
        if result.is_valid {
            outln!(c, "   {} The value in the file matches", "✅".green());
        } else if (stored - calculated).abs() > 0.0 {
            outln!(
                c,
                "   {} The value in the file is {}: run forge calculate to update it",
                "⚠️".yellow(),
                format_number(stored).red()
            );
        }
    }
    outln!(c);

    for warning in &result.warnings {
        outln!(c, "   {} {warning}", "⚠️".yellow());
    }
    if !result.warnings.is_empty() {
        outln!(c);
    }
    if let Some(error) = &result.calculation_error {
        outln!(
            c,
            "   {} The model does not calculate, so values are the file's: {error}",
            "⚠️".yellow()
        );
        outln!(c);
    }
    if result.formula.is_some() {
        outln!(c, "{}", "🌳 Dependency Tree:".bold().cyan());
        if result.dependencies.is_empty() {
            outln!(c, "   No dependencies (constant formula)");
        }
        for dep in &result.dependencies {
            print_dependency(c, dep, 1);
        }
        outln!(c);
    }
    outln!(c, "{}", "✅ Audit complete".bold().green());
    Ok(())
}

/// `12 rows: 4,632.01 … 7,346.41` for a column's values.
fn summarize(values: &[Option<f64>]) -> String {
    let shown = |v: &Option<f64>| v.map_or_else(|| "empty".to_string(), format_number);
    let empty = values.iter().filter(|v| v.is_none()).count();
    let empty = match empty {
        0 => String::new(),
        1 => " (1 empty cell)".to_string(),
        n => format!(" ({n} empty cells)"),
    };
    match values {
        [] => "no rows".to_string(),
        [only] => format!("1 row: {}", shown(only)),
        [first, .., last] => format!(
            "{} rows: {} … {}{empty}",
            values.len(),
            shown(first),
            shown(last)
        ),
    }
}

/// Print a dependency with indentation
fn print_dependency(c: &mut Console<'_>, dep: &AuditDep, indent: usize) {
    let prefix = "   ".repeat(indent);
    out!(
        c,
        "{prefix}└─ {} ({})",
        dep.name.bright_blue(),
        dep.dep_type.cyan()
    );
    if let Some(val) = dep.value {
        let shown = dep.date.clone().unwrap_or_else(|| format_number(val));
        out!(c, " = {}", shown.green());
    }
    if let Some(values) = &dep.values {
        out!(c, " = {}", summarize(values).green());
    }
    if let Some(ref f) = dep.formula {
        out!(c, " {}", f.yellow());
    }
    if let Some(source) = &dep.source {
        out!(c, " [{source}]");
    }
    if dep.repeated && dep.formula.is_some() {
        out!(c, " {}", "(see above)".dimmed());
    }
    outln!(c);
    for child in &dep.children {
        print_dependency(c, child, indent + 1);
    }
}

#[cfg(test)]
mod tests {
    /// A model with an `MC.*` input is refused with what to run instead
    /// (roadmap E-MC-MESSAGE)
    mod distributions {
        #[test]
        fn audit_names_the_distribution_and_what_to_run() {
            crate::cli::commands::assert_distributions_refused(
                &["audit", "FILE", "outputs.profit"],
                "audit",
            );
        }
    }

    use super::*;

    #[test]
    fn references_skip_function_names_and_text() {
        let refs = formula_references(
            "=NPV(rate / 12, months.profit) - SUM(sales.tax) * @rates.fx.usd + LEN(\"total\")",
        );
        assert_eq!(
            refs,
            ["rate", "months.profit", "sales.tax", "@rates.fx.usd"]
        );
    }

    fn find<'a>(deps: &'a [AuditDep], name: &str) -> Option<&'a AuditDep> {
        deps.iter().find_map(|d| {
            if d.name == name {
                Some(d)
            } else {
                find(&d.children, name)
            }
        })
    }

    /// Every name in a tree that has no formula: what was typed in
    fn leaves(deps: &[AuditDep], out: &mut std::collections::BTreeSet<String>) {
        for d in deps {
            if d.formula.is_none() {
                out.insert(d.name.clone());
            }
            leaves(&d.children, out);
        }
    }

    fn walk(deps: &[AuditDep], unknown: &mut Vec<String>, dozens: &mut usize) {
        for d in deps {
            if d.dep_type == "Unknown" {
                unknown.push(d.name.clone());
            }
            if d.name == "months.dozens" && !d.repeated {
                *dozens += 1;
            }
            walk(&d.children, unknown, dozens);
        }
    }

    /// The bagel model, from the year's profit down to the included costs.
    #[test]
    fn bagel_profit_down_to_the_included_costs() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let result = audit_core(
            &root.join("examples/bagel/bagel.yaml"),
            "year.profit",
            false,
        )
        .unwrap();
        assert!((result.calculated_value.unwrap() - 786.320_361_7).abs() < 1e-6);
        assert!(result.is_valid);
        let deps = &result.dependencies;
        assert_eq!(deps[0].name, "months.profit");
        assert_eq!(deps[0].values.as_ref().map(Vec::len), Some(12));
        // Bare names in a row formula are columns of the same table
        let revenue = find(deps, "months.revenue").unwrap();
        assert_eq!(revenue.dep_type, "Column[months]");
        assert!(find(&revenue.children, "assumptions.price_per_dozen").is_some());
        // Included values: their value, formula and file, followed inside it
        let wage = find(deps, "@costs.labour.loaded_wage").unwrap();
        assert_eq!(wage.dep_type, "Included scalar");
        assert_eq!(wage.source.as_deref(), Some("costs.yaml"));
        assert!((wage.value.unwrap() - 21.275).abs() < 1e-9);
        let rate = find(&wage.children, "@costs.labour.wage_per_hour").unwrap();
        assert!((rate.value.unwrap() - 18.5).abs() < 1e-12);
        // Nothing unresolved, and `dozens` is expanded once
        let (mut unknown, mut dozens) = (Vec::new(), 0);
        walk(deps, &mut unknown, &mut dozens);
        assert!(unknown.is_empty(), "{unknown:?}");
        assert_eq!(dozens, 1);
        // The README's count: what has no formula, 9 inputs in bagel.yaml, 6
        // in costs.yaml, and the month and season columns
        let mut inputs = std::collections::BTreeSet::new();
        leaves(deps, &mut inputs);
        let count = |p: &str| inputs.iter().filter(|n| n.starts_with(p)).count();
        assert_eq!(count("assumptions."), 9, "{inputs:?}");
        assert_eq!(count("@costs."), 6, "{inputs:?}");
        assert_eq!(count("months."), 2, "{inputs:?}");
        assert_eq!(inputs.len(), 17, "{inputs:?}");
    }

    /// A column of an included table is followed into its file.
    #[test]
    fn an_included_column_is_followed_into_its_file() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("prices.yaml"),
            "_forge_version: \"5.1.0\"\ntax:\n  rate: 0.15\nlist:\n  per_kg: [1.15, 9.40, 0.80]\n  with_tax: \"=per_kg * (1 + tax.rate)\"\n",
        )
        .unwrap();
        let main = dir.path().join("main.yaml");
        std::fs::write(
            &main,
            "_forge_version: \"5.1.0\"\n_includes:\n  - file: prices.yaml\n    as: prices\norder:\n  kg: [120, 6, 3]\n  cost: \"=kg * @prices.list.with_tax\"\ntotal:\n  cost:\n    formula: \"=SUM(order.cost)\"\n",
        )
        .unwrap();
        let result = audit_core(&main, "total.cost", false).unwrap();
        let deps = &result.dependencies;
        let with_tax = find(deps, "@prices.list.with_tax").unwrap();
        assert_eq!(with_tax.dep_type, "Column[list]");
        assert_eq!(with_tax.source.as_deref(), Some("prices.yaml"));
        assert_eq!(with_tax.values.as_ref().map(Vec::len), Some(3));
        assert!(find(&with_tax.children, "@prices.list.per_kg").is_some());
        assert!(find(&with_tax.children, "@prices.tax.rate").is_some());
        let (mut unknown, mut dozens) = (Vec::new(), 0);
        walk(deps, &mut unknown, &mut dozens);
        assert!(unknown.is_empty(), "{unknown:?}");
    }

    #[test]
    fn a_column_and_a_scalar_from_its_own_group() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let column = audit_core(
            &root.join("examples/bagel/bagel.yaml"),
            "months.costs",
            false,
        )
        .unwrap();
        assert_eq!(column.var_type, "Column[months]");
        assert_eq!(column.calculated_values.as_ref().map(Vec::len), Some(12));
        // `margin = profit / revenue`: both are scalars of `year`
        let margin = audit_core(
            &root.join("examples/bagel/bagel.yaml"),
            "year.margin",
            false,
        )
        .unwrap();
        let names: Vec<&str> = margin
            .dependencies
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(names, ["year.profit", "year.revenue"]);
        let err = audit_core(&root.join("examples/bagel/bagel.yaml"), "nope", false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("not a scalar or a table column"), "{err}");
    }

    mod printed {
        //! What `forge audit` prints and finds, on small models, one case each.

        use super::*;
        use crate::cli::console::capture;
        use std::fmt::Write as _;
        use std::fs;

        const SCALARS: &str = "_forge_version: \"5.4.0\"\na: 10\nb: {formula: \"=a * 2\"}\nc: {formula: \"=b + a\"}\nf: {formula: \"=b + c\"}\nk: {formula: \"=1 + 1\"}\nstale: {formula: \"=a + 1\", value: 99}\nok: {formula: \"=a + 1\", value: 11}\nwhen: {formula: \"=DATE(2026, 1, 1)\"}\nuntil: {formula: \"=when + 1\"}\ngrp:\n  x: 3\n  y: {formula: \"=x * 2\"}\nflows:\n  cf: [-100, null, 60, null, 70]\n  label: [\"a\", \"b\", \"c\", \"d\", \"e\"]\ngrp2:\n  z: {formula: \"=a + 1\"}\nclean:\n  cf: [1, 2]\nnpv_clean: {formula: \"=NPV(0.1, clean.cf)\"}\nnpv_total: {formula: \"=NPV(0.1, flows.cf)\"}\nirr_total: {formula: \"=IRR(flows.cf)\"}\n";

        fn audit_of(text: &str, variable: &str) -> (ForgeResult<()>, String) {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("m.yaml");
            fs::write(&file, text).unwrap();
            let (result, captured) = capture(|c| audit(c, &file, variable, false));
            (result, captured.out)
        }

        fn shown(variable: &str) -> String {
            let (result, out) = audit_of(SCALARS, variable);
            result.unwrap();
            out
        }

        #[test]
        fn a_scalar_is_shown_with_its_formula_value_and_dependencies() {
            let out = shown("c");
            for line in [
                "Variable: c",
                "Type: Scalar",
                "Formula: =b + a",
                "Calculated: 30",
                "Dependency Tree:",
                "└─ b (Scalar) = 20 =a * 2",
                "Audit complete",
            ] {
                assert!(out.contains(line), "{line} in {out}");
            }
            // `b` is expanded once; its second appearance says so
            assert!(shown("f").contains("(see above)"));
        }

        #[test]
        fn the_value_in_the_file_is_checked_against_the_calculated_one() {
            assert!(shown("ok").contains("The value in the file matches"));
            let stale = shown("stale");
            assert!(
                stale.contains("The value in the file is 99: run forge calculate"),
                "{stale}"
            );
            // Nothing stored, nothing to compare
            assert!(!shown("c").contains("The value in the file"));
        }

        #[test]
        fn a_constant_formula_and_a_date_say_so() {
            assert!(shown("k").contains("No dependencies (constant formula)"));
            assert!(shown("when").contains("Calculated: 2026-01-01"));
            // Plus a day is a day number again
            let out = shown("until");
            assert!(out.contains("Calculated: 46024"), "{out}");
            assert!(out.contains("when (Scalar) = 2026-01-01"), "{out}");
            assert!(!shown("a").contains("Dependency Tree"));
        }

        #[test]
        fn a_name_without_its_group_finds_the_scalar_of_the_group_it_is_written_in() {
            let found = audit_core_of(SCALARS, "grp.y");
            assert_eq!(found.dependencies[0].name, "grp.x");
            assert_eq!(found.calculated_value, Some(6.0));
        }

        fn audit_core_of(text: &str, variable: &str) -> super::super::AuditResult {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("m.yaml");
            fs::write(&file, text).unwrap();
            audit_core(&file, variable, false).unwrap()
        }

        #[test]
        fn a_name_in_a_group_falls_back_to_the_plain_scalar() {
            let found = audit_core_of(SCALARS, "grp2.z");
            assert_eq!(found.dependencies[0].name, "a");
            assert_eq!(found.calculated_value, Some(11.0));
            // No empty cell in what NPV reads, no warning
            assert_eq!(
                audit_core_of(SCALARS, "npv_clean").warnings,
                Vec::<String>::new()
            );
        }

        #[test]
        fn a_column_of_a_model_that_does_not_calculate_has_no_values() {
            let found = audit_core_of(
                "_forge_version: \"5.1.0\"\nt:\n  n: [1, 2]\n  d: \"=n * nosuch\"\n",
                "t.d",
            );
            assert!(found.calculated_values.is_none());
            assert!(found.calculation_error.is_some());
        }

        #[test]
        fn a_stored_value_of_a_model_that_does_not_calculate_is_not_called_stale() {
            let (result, out) = audit_of(
                "_forge_version: \"5.1.0\"\nu: {formula: \"=nosuch + 1\", value: 1}\n",
                "u",
            );
            result.unwrap();
            assert!(!out.contains("The value in the file"), "{out}");
        }

        #[test]
        fn no_rows_are_summarized_as_none() {
            assert_eq!(summarize(&[]), "no rows");
        }

        #[test]
        fn a_column_is_summarized_with_its_empty_cells() {
            let out = shown("flows.cf");
            assert!(
                out.contains("Calculated: 5 rows: -100 … 70 (2 empty cells)"),
                "{out}"
            );
            assert!(shown("flows.label").contains("Type: Column[flows]"));
            let (result, out) = audit_of(
                "_forge_version: \"5.3.0\"\nflows:\n  cf: [-100, null]\n  one: [5]\n",
                "flows.cf",
            );
            result.unwrap();
            assert!(out.contains("2 rows: -100 … empty (1 empty cell)"), "{out}");
            let (result, out) = audit_of(
                "_forge_version: \"5.3.0\"\nflows:\n  one: [5]\n",
                "flows.one",
            );
            result.unwrap();
            assert!(out.contains("1 row: 5"), "{out}");
        }

        #[test]
        fn npv_and_irr_warn_about_the_empty_cells_they_skip() {
            let out = shown("npv_total");
            assert!(
                out.contains("npv_total: NPV skips the 2 empty cells of flows.cf, as Excel does"),
                "{out}"
            );
            let out = shown("irr_total");
            assert!(
                out.contains("IRR skips the 2 empty cells of flows.cf"),
                "{out}"
            );
            let (result, out) = audit_of(
                "_forge_version: \"5.3.0\"\nflows:\n  cf: [-100, null, 60]\ntotal: {formula: \"=NPV(0.1, flows.cf)\"}\nouter: {formula: \"=total + 1\"}\n",
                "outer",
            );
            result.unwrap();
            assert!(
                out.contains("total: NPV skips the 1 empty cell of flows.cf"),
                "{out}"
            );
        }

        #[test]
        fn a_model_that_does_not_calculate_is_audited_from_the_file() {
            let (result, out) = audit_of(
                "_forge_version: \"5.1.0\"\nu: {formula: \"=nosuch + 1\"}\n",
                "u",
            );
            result.unwrap();
            assert!(
                out.contains("The model does not calculate, so values are the file's"),
                "{out}"
            );
            assert!(out.contains("nosuch (Unknown)"), "{out}");
        }

        #[test]
        fn a_name_that_is_not_in_the_model_lists_what_is() {
            let (result, _) = audit_of(SCALARS, "nosuch");
            let message = result.unwrap_err().to_string();
            assert!(
                message.contains("'nosuch' is not a scalar or a table column"),
                "{message}"
            );
            assert!(message.contains("flows.cf"), "{message}");
            let (result, _) = audit_of(SCALARS, "nosuch.col");
            assert!(result.is_err());
            let (result, _) = audit_of(SCALARS, "flows.nosuch");
            assert!(result.is_err());
        }

        #[test]
        fn a_long_chain_stops_at_depth_30() {
            let mut text = String::from("_forge_version: \"5.1.0\"\ns0: 1\n");
            for n in 1..=32 {
                writeln!(text, "s{n}: {{formula: \"=s{} + 1\"}}", n - 1).unwrap();
            }
            let found = audit_core_of(&text, "s32");
            let mut node = &found.dependencies[0];
            let mut depth = 1;
            while let Some(next) = node.children.first() {
                node = next;
                depth += 1;
            }
            assert!(node.repeated && node.formula.is_some(), "{}", node.name);
            assert_eq!(depth, 30);
        }

        #[test]
        fn names_that_resolve_nowhere_stay_unknown() {
            let dir = tempfile::tempdir().unwrap();
            fs::write(
                dir.path().join("prices.yaml"),
                "_forge_version: \"5.1.0\"\ntax:\n  rate: 0.15\nlist:\n  per_kg: [1, 2]\n",
            )
            .unwrap();
            fs::write(
                dir.path().join("other.yaml"),
                "_forge_version: \"5.1.0\"\nbonus: 1\n",
            )
            .unwrap();
            let main = dir.path().join("main.yaml");
            fs::write(
                &main,
                "_forge_version: \"5.1.0\"\n_includes:\n  - file: prices.yaml\n    as: prices\n  - file: other.yaml\n    as: other\nx: {formula: \"=@nope.y + @prices.list.missing + @prices.absent + @other.bonus + nosuch.col\"}\n",
            )
            .unwrap();
            let found = audit_core(&main, "x", false).unwrap();
            let kinds: Vec<(&str, &str)> = found
                .dependencies
                .iter()
                .map(|d| (d.name.as_str(), d.dep_type.as_str()))
                .collect();
            let (mut unknown, mut dozens) = (Vec::new(), 0);
            walk(&found.dependencies, &mut unknown, &mut dozens);
            assert_eq!(unknown.len(), 4, "{unknown:?}");
            let (result, captured) = capture(|c| audit(c, &main, "x", false));
            result.unwrap();
            assert!(captured.out.contains("[other.yaml]"), "{}", captured.out);
            assert_eq!(
                kinds,
                [
                    ("@nope.y", "Unknown"),
                    ("@prices.list.missing", "Unknown"),
                    ("@prices.absent", "Unknown"),
                    ("@other.bonus", "Included scalar"),
                    ("nosuch.col", "Unknown"),
                ]
            );
        }

        #[test]
        fn a_formula_that_cannot_be_read_has_no_references() {
            assert_eq!(formula_references("=\"unterminated"), Vec::<String>::new());
            assert!(calls("=NPV(0.1, x)", "npv"));
            assert!(!calls("=npv + 1", "NPV"));
            assert!(!calls("=\"unterminated", "NPV"));
        }
    }
}
