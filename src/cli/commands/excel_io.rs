//! Excel export and import commands

use crate::error::{ForgeError, ForgeResult};
use crate::excel::{ExcelExporter, ExcelImporter};
use crate::parser;
use colored::Colorize;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// Parse a model for export and calculate it, so a formula that Excel cannot
/// express can be written as its value. If calculation fails, the formulas are
/// still exported and those cells are left empty.
fn load_for_export(input: &Path) -> ForgeResult<crate::types::ParsedModel> {
    let model = parser::parse_model(input)?;
    Ok(crate::core::ArrayCalculator::new(model.clone())
        .calculate_all()
        .unwrap_or(model))
}

/// Export YAML to Excel and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed or the Excel export fails.
pub fn export_core(input: &Path, output: &Path) -> ForgeResult<super::results::ExportResult> {
    let model = load_for_export(input)?;
    let table_count = model.tables.len();
    let scalar_count = model.scalars.len();

    let exporter = ExcelExporter::new(model);
    let warnings = exporter.export_with_warnings(output)?;

    Ok(super::results::ExportResult {
        input_path: input.display().to_string(),
        output_path: output.display().to_string(),
        table_count,
        scalar_count,
        warnings,
    })
}

/// Export YAML to an in-memory Excel buffer and return base64-encoded content.
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed or the Excel buffer export fails.
pub fn export_buffer_core(input: &Path) -> ForgeResult<super::results::ExportBufferResult> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let model = load_for_export(input)?;
    let table_count = model.tables.len();
    let scalar_count = model.scalars.len();

    let exporter = ExcelExporter::new(model);
    let buffer = exporter.export_to_buffer()?;
    let base64_content = STANDARD.encode(&buffer);

    Ok(super::results::ExportBufferResult {
        table_count,
        scalar_count,
        excel_base64: base64_content,
        byte_count: buffer.len(),
    })
}

/// One table in the YAML format the parser reads: data columns as arrays,
/// row formulas as `=...` strings, in the table's column order. A name with
/// a row formula is written as the formula.
fn table_to_value(table: &crate::types::Table) -> yaml_serde::Value {
    use crate::types::ColumnValue;
    use yaml_serde::Value;
    let mut map = yaml_serde::Mapping::new();
    for name in table.column_names() {
        if let Some(formula) = table.row_formulas.get(name) {
            map.insert(Value::String(name.clone()), Value::String(formula.clone()));
            continue;
        }
        let items: Vec<Value> = match &table.columns[name].values {
            ColumnValue::Number(v) => v.iter().map(|n| number(*n)).collect(),
            ColumnValue::Text(v) | ColumnValue::Date(v) => {
                v.iter().map(|t| Value::String(t.clone())).collect()
            },
            ColumnValue::Boolean(v) => v.iter().map(|b| Value::Bool(*b)).collect(),
        };
        map.insert(Value::String(name.clone()), Value::Sequence(items));
    }
    Value::Mapping(map)
}

/// All tables as `{table_name: table}`, in model order.
fn tables_to_value(
    tables: &indexmap::IndexMap<String, crate::types::Table>,
) -> yaml_serde::Mapping {
    let mut map = yaml_serde::Mapping::new();
    for (name, table) in tables {
        map.insert(
            yaml_serde::Value::String(name.clone()),
            table_to_value(table),
        );
    }
    map
}

/// A number as YAML: `16`, not `16.0`, when it is a whole number.
fn number(n: f64) -> yaml_serde::Value {
    #[allow(clippy::cast_possible_truncation, clippy::float_cmp)] // whole and in range
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return yaml_serde::Value::Number((n as i64).into());
    }
    yaml_serde::Value::Number(n.into())
}

/// Scalars in model format 5.1.0 (ADR-051): an input as a plain number, a
/// formula as `{formula, value}` with its last result. Grouped as
/// `group: {name: ...}` for `group.name`, and at the top for ungrouped ones.
/// Groups appear in the order of their first scalar, and scalars in model order.
fn scalars_to_value(
    scalars: &indexmap::IndexMap<String, crate::types::Variable>,
) -> yaml_serde::Mapping {
    use yaml_serde::{Mapping, Value};
    let mut map = Mapping::new();
    for (path, variable) in scalars {
        let entry = match (&variable.formula, variable.value) {
            (None, Some(value)) => number(value),
            (Some(formula), value) => {
                let mut entry = Mapping::new();
                entry.insert(Value::from("formula"), Value::from(formula.clone()));
                if let Some(value) = value {
                    entry.insert(Value::from("value"), number(value));
                }
                Value::Mapping(entry)
            },
            // An empty cell: no number and no formula
            (None, None) => {
                let mut entry = Mapping::new();
                entry.insert(Value::from("value"), Value::Null);
                Value::Mapping(entry)
            },
        };
        match path.split_once('.') {
            Some((group, name)) => {
                let group_entry = map
                    .entry(Value::String(group.to_string()))
                    .or_insert_with(|| Value::Mapping(Mapping::new()));
                if let Value::Mapping(group_map) = group_entry {
                    group_map.insert(Value::String(name.to_string()), entry);
                }
            },
            None => {
                map.insert(Value::String(path.clone()), entry);
            },
        }
    }
    map
}

/// `_includes:` for the included files a workbook held (one `{ns}.Scalars`
/// sheet each), as YAML text; empty when there are none, or when `uses` is
/// given and names none of them (`@ns.`).
fn includes_block(model: &crate::types::ParsedModel, uses: Option<&str>) -> String {
    let mut block = String::new();
    for include in &model.includes {
        if uses.is_some_and(|text| !text.contains(&format!("@{}.", include.namespace))) {
            continue;
        }
        if block.is_empty() {
            block.push_str("_includes:\n");
        }
        let _ = writeln!(
            block,
            "  - file: {}\n    as: {}",
            include.file, include.namespace
        );
    }
    block
}

/// The included files to write into `dir`, as (path, YAML).
///
/// A file already there is usually the original include, exported and
/// imported unchanged: it is kept as written (comments and all) when it
/// calculates to the values on the workbook's sheet. When it holds other
/// values, nothing is written: replacing it would lose what it says.
fn include_files(
    dir: &Path,
    model: &crate::types::ParsedModel,
) -> ForgeResult<Vec<(std::path::PathBuf, String)>> {
    let mut files = Vec::new();
    for include in &model.includes {
        let Some(resolved) = model.resolved_includes.get(&include.namespace) else {
            continue;
        };
        let path = dir.join(&include.file);
        if path.exists() {
            if holds_the_same_values(&path, &resolved.model) {
                continue;
            }
            return Err(ForgeError::Import(format!(
                "the workbook's sheets for {} hold other values than {}. Nothing was \
                 written. Import into another folder, then compare the two files",
                include.namespace,
                path.display()
            )));
        }
        files.push((path, model_to_yaml(&resolved.model)?));
    }
    Ok(files)
}

/// Whether the model file at `path` calculates to what the workbook's sheets
/// for it hold: the same scalars and tables, the same values, and a formula
/// where the sheets have one.
fn holds_the_same_values(path: &Path, sheet: &crate::types::ParsedModel) -> bool {
    let Ok(existing) = parser::parse_model(path) else {
        return false;
    };
    let calculated = |m: &crate::types::ParsedModel| {
        crate::core::ArrayCalculator::new(m.clone()).calculate_all()
    };
    let Ok(existing) = calculated(&existing) else {
        return false;
    };
    let Ok(sheet) = calculated(sheet) else {
        return false;
    };
    let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0);
    let same_scalars = existing.scalars.len() == sheet.scalars.len()
        && sheet.scalars.iter().all(|(name, var)| {
            existing.scalars.get(name).is_some_and(|old| {
                old.formula.is_some() == var.formula.is_some()
                    && match (old.value, var.value) {
                        (Some(a), Some(b)) => close(a, b),
                        (a, b) => a.is_none() && b.is_none(),
                    }
            })
        });
    let same_column = |a: &crate::types::ColumnValue, b: &crate::types::ColumnValue| {
        use crate::types::ColumnValue::Number;
        match (a, b) {
            (Number(x), Number(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(p, q)| close(*p, *q))
            },
            _ => a == b,
        }
    };
    let same_tables = existing.tables.len() == sheet.tables.len()
        && sheet.tables.iter().all(|(name, table)| {
            existing.tables.get(name).is_some_and(|old| {
                old.columns.len() == table.columns.len()
                    && old
                        .row_formulas
                        .keys()
                        .collect::<std::collections::BTreeSet<_>>()
                        == table.row_formulas.keys().collect()
                    && table.columns.iter().all(|(col, c)| {
                        old.columns
                            .get(col)
                            .is_some_and(|o| same_column(&o.values, &c.values))
                    })
            })
        });
    same_scalars && same_tables
}

/// Write the imported model as one file, with its included files beside it.
fn write_single(output: &Path, model: &crate::types::ParsedModel) -> ForgeResult<()> {
    let yaml = model_to_yaml(model)?;
    let dir = output.parent().unwrap_or_else(|| Path::new("."));
    let files = include_files(dir, model)?;
    for (path, text) in files {
        fs::write(path, text).map_err(ForgeError::Io)?;
    }
    fs::write(output, yaml).map_err(ForgeError::Io)
}

/// The whole imported model as YAML that `forge validate` and `forge calculate` accept.
fn model_to_yaml(model: &crate::types::ParsedModel) -> ForgeResult<String> {
    let includes = includes_block(model, None);
    let mut map = tables_to_value(&model.tables);
    for (key, value) in scalars_to_value(&model.scalars) {
        if map.contains_key(&key) {
            return Err(ForgeError::Import(format!(
                "'{}' is both a table and a scalar group; rename one sheet",
                key.as_str().unwrap_or_default()
            )));
        }
        map.insert(key, value);
    }
    if map.is_empty() {
        return Ok(format!("_forge_version: \"5.1.0\"\n{includes}"));
    }
    let body = yaml_serde::to_string(&map).map_err(ForgeError::Yaml)?;
    Ok(format!("_forge_version: \"5.1.0\"\n{includes}\n{body}"))
}

/// Import Excel to YAML and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the Excel file cannot be read, imported, or written as YAML.
pub fn import_core(
    input: &Path,
    output: &Path,
    split_files: bool,
    multi_doc: bool,
) -> ForgeResult<super::results::ImportResult> {
    let importer = ExcelImporter::new(input);
    let (model, warnings) = importer.import_with_warnings()?;
    let table_count = model.tables.len();
    let scalar_count = model.scalars.len();

    let mode = if split_files {
        "split"
    } else if multi_doc {
        "multi-doc"
    } else {
        "single"
    };

    if split_files {
        write_split_files_quiet(output, &model)?;
    } else if multi_doc {
        write_multi_doc_quiet(output, &model)?;
    } else {
        write_single(output, &model)?;
    }

    Ok(super::results::ImportResult {
        input_path: input.display().to_string(),
        output_path: output.display().to_string(),
        table_count,
        scalar_count,
        mode: mode.to_string(),
        warnings,
    })
}

/// `--split-files` writes each table, and the scalars, as files of their own,
/// which cannot refer to each other. Refuse, before writing anything, when a
/// formula in one part names a table or scalar group in another (ADR-052).
fn check_split_is_self_contained(model: &crate::types::ParsedModel) -> ForgeResult<()> {
    // The part each table or scalar group goes to: a table's own file, or scalars.yaml
    let mut part_of: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for name in model.tables.keys() {
        part_of.insert(name.clone(), format!("{name}.yaml"));
    }
    for path in model.scalars.keys() {
        let group = path.split_once('.').map_or(path.as_str(), |(g, _)| g);
        part_of
            .entry(group.to_string())
            .or_insert_with(|| "scalars.yaml".to_string());
    }
    let formulas = model
        .scalars
        .iter()
        .filter_map(|(n, v)| {
            v.formula
                .as_deref()
                .map(|f| (n.clone(), "scalars.yaml".to_string(), f))
        })
        .chain(model.tables.iter().flat_map(|(t, table)| {
            table
                .row_formulas
                .iter()
                .map(move |(c, f)| (format!("{t}.{c}"), format!("{t}.yaml"), f.as_str()))
        }));
    let mut crossing = Vec::new();
    for (owner, own_part, formula) in formulas {
        for word in formula.split(|c: char| !(c.is_alphanumeric() || matches!(c, '_' | '.'))) {
            let Some((first, _)) = word.split_once('.') else {
                continue;
            };
            if let Some(part) = part_of.get(first) {
                if *part != own_part {
                    crossing.push(format!("{owner} ({own_part}) uses {word} ({part})"));
                    break;
                }
            }
        }
    }
    if crossing.is_empty() {
        return Ok(());
    }
    Err(ForgeError::Import(format!(
        "--split-files would break {} formula{}, because separate files cannot refer to each \
         other: {}{}. Nothing was written. Import without --split-files (one file), \
         or with --multi-doc (one file, one document per sheet)",
        crossing.len(),
        if crossing.len() == 1 { "" } else { "s" },
        crossing
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("; "),
        if crossing.len() > 3 { "; ..." } else { "" }
    )))
}

/// Write each table, and the scalars, as files of their own, with the
/// included files beside them; each part lists the includes it reads.
/// Returns the files written.
fn write_split_files_quiet(
    output: &Path,
    model: &crate::types::ParsedModel,
) -> ForgeResult<Vec<std::path::PathBuf>> {
    check_split_is_self_contained(model)?;
    let mut parts = Vec::new();
    for (table_name, table) in &model.tables {
        let mut one = indexmap::IndexMap::new();
        one.insert(table_name.clone(), table.clone());
        let body = yaml_serde::to_string(&tables_to_value(&one)).map_err(ForgeError::Yaml)?;
        parts.push((format!("{table_name}.yaml"), table_name.clone(), body));
    }
    if !model.scalars.is_empty() {
        let body =
            yaml_serde::to_string(&scalars_to_value(&model.scalars)).map_err(ForgeError::Yaml)?;
        parts.push(("scalars.yaml".to_string(), "scalars".to_string(), body));
    }
    fs::create_dir_all(output).map_err(ForgeError::Io)?;
    let includes = include_files(output, model)?;
    let mut written = Vec::new();
    for (file, name, body) in parts {
        let path = output.join(file);
        let includes = includes_block(model, Some(&body));
        let text = format!("_forge_version: \"5.1.0\"\n_name: \"{name}\"\n{includes}\n{body}");
        fs::write(&path, text).map_err(ForgeError::Io)?;
        written.push(path);
    }
    for (path, text) in includes {
        fs::write(&path, text).map_err(ForgeError::Io)?;
        written.push(path);
    }
    Ok(written)
}

/// Execute the export command.
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed or the Excel export fails.
pub fn export(input: &Path, output: &Path, verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Excel Export".bold().green());
    println!("   Input:  {}", input.display());
    println!("   Output: {}\n", output.display());

    if verbose {
        println!("{}", "📖 Parsing YAML file...".cyan());
    }

    let model = load_for_export(input)?;

    if verbose {
        println!(
            "   Found {} tables, {} scalars\n",
            model.tables.len(),
            model.scalars.len()
        );
    }

    if verbose {
        println!("{}", "📊 Exporting to Excel...".cyan());
    }

    let exporter = ExcelExporter::new(model);
    let warnings = exporter.export_with_warnings(output)?;

    println!("{}", "✅ Export Complete!".bold().green());
    println!("   Excel file: {}\n", output.display());
    if !warnings.is_empty() {
        println!("{}", "⚠️  Not exported as Excel formulas:".yellow());
        for warning in &warnings {
            println!("   - {warning}");
        }
        println!();
    }

    Ok(())
}

/// Execute the import command.
///
/// # Errors
///
/// Returns an error if the Excel file cannot be read, imported, or written as YAML.
pub fn import(
    input: &Path,
    output: &Path,
    verbose: bool,
    split_files: bool,
    multi_doc: bool,
) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Excel Import".bold().green());
    println!("   Input:  {}", input.display());
    println!("   Output: {}", output.display());
    if split_files {
        println!("   Mode:   Split files (one YAML per sheet)");
    } else if multi_doc {
        println!("   Mode:   Multi-document YAML");
    }
    println!();

    if verbose {
        println!("{}", "📖 Reading Excel file...".cyan());
    }

    let importer = ExcelImporter::new(input);
    let (model, warnings) = importer.import_with_warnings()?;

    if verbose {
        println!("   Found {} tables", model.tables.len());
        println!("   Found {} scalars\n", model.scalars.len());
        for (table_name, table) in &model.tables {
            println!("   📊 Table: {}", table_name.bright_blue());
            println!(
                "      {} columns, {} rows",
                table.columns.len(),
                table.row_count()
            );
        }
        println!();
        println!("{}", "💾 Writing YAML file(s)...".cyan());
    }

    if split_files {
        write_split_files(output, &model, verbose)?;
    } else if multi_doc {
        write_multi_doc(output, &model)?;
    } else {
        write_single(output, &model)?;
        println!("{}", "✅ Import Complete!".bold().green());
        println!("   YAML file: {}\n", output.display());
    }

    if !warnings.is_empty() {
        println!(
            "{}",
            "⚠️  Imported as values, because Forge cannot express these formulas:".yellow()
        );
        for warning in &warnings {
            println!("   - {warning}");
        }
        println!();
    }
    Ok(())
}

fn write_split_files(
    output: &Path,
    model: &crate::types::ParsedModel,
    verbose: bool,
) -> ForgeResult<()> {
    let written = write_split_files_quiet(output, model)?;
    if verbose {
        for path in &written {
            println!("   Created: {}", path.display());
        }
    }
    println!("{}", "✅ Import Complete!".bold().green());
    println!("   Output directory: {}\n", output.display());
    Ok(())
}

/// One file, one document per table and one for the scalars (ADR-052: one
/// namespace), the first listing the included files, written beside it.
fn write_multi_doc_quiet(output: &Path, model: &crate::types::ParsedModel) -> ForgeResult<()> {
    let mut yaml_output = String::new();
    let mut includes = includes_block(model, None);
    let mut document = |name: &str, body: String| {
        yaml_output.push_str("---\n_forge_version: \"5.1.0\"\n");
        let _ = writeln!(yaml_output, "_name: \"{name}\"");
        yaml_output.push_str(&std::mem::take(&mut includes));
        yaml_output.push('\n');
        yaml_output.push_str(&body);
    };
    for (table_name, table) in &model.tables {
        let mut one = indexmap::IndexMap::new();
        one.insert(table_name.clone(), table.clone());
        let body = yaml_serde::to_string(&tables_to_value(&one)).map_err(ForgeError::Yaml)?;
        document(table_name, format!("{body}\n"));
    }
    if !model.scalars.is_empty() {
        let body =
            yaml_serde::to_string(&scalars_to_value(&model.scalars)).map_err(ForgeError::Yaml)?;
        document("scalars", body);
    }
    let dir = output.parent().unwrap_or_else(|| Path::new("."));
    for (path, text) in include_files(dir, model)? {
        fs::write(path, text).map_err(ForgeError::Io)?;
    }
    fs::write(output, yaml_output).map_err(ForgeError::Io)
}

fn write_multi_doc(output: &Path, model: &crate::types::ParsedModel) -> ForgeResult<()> {
    write_multi_doc_quiet(output, model)?;
    println!("{}", "✅ Import Complete!".bold().green());
    println!("   YAML file: {}\n", output.display());
    Ok(())
}

#[cfg(test)]
mod import_output_tests {
    use super::*;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    /// What `forge import` writes must load and calculate like a hand-written model.
    #[test]
    fn imported_yaml_is_valid_and_calculates() {
        let mut model = ParsedModel::new();
        let mut table = Table::new("sales".to_string());
        table.add_column(Column::new(
            "revenue".to_string(),
            ColumnValue::Number(vec![100.0, 200.0]),
        ));
        table.add_column(Column::new(
            "region".to_string(),
            ColumnValue::Text(vec!["N".to_string(), "S".to_string()]),
        ));
        table.add_row_formula("double".to_string(), "=revenue * 2".to_string());
        model.add_table(table);
        model.add_scalar(
            "rate".to_string(),
            Variable::new("rate".to_string(), Some(0.1), None),
        );

        let yaml = model_to_yaml(&model).unwrap();
        assert!(yaml.starts_with("_forge_version: \"5.1.0\""), "{yaml}");
        // 5.1.0 short form: an input is a plain number, whole numbers stay whole
        assert!(yaml.contains("rate: 0.1\n"), "{yaml}");
        assert!(yaml.contains("- 100\n"), "{yaml}");
        assert!(!yaml.contains("null"), "{yaml}");
        assert!(
            !yaml.contains("!Number") && !yaml.contains("!Text"),
            "{yaml}"
        );

        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("imported.yaml");
        fs::write(&path, yaml).unwrap();
        let parsed = parser::parse_model(&path).unwrap();
        assert_eq!(parsed.tables["sales"].columns.len(), 2);
        assert_eq!(
            parsed.tables["sales"].row_formulas["double"],
            "=revenue * 2"
        );
        crate::core::ArrayCalculator::new(parsed)
            .calculate_all()
            .unwrap();
    }
}

#[cfg(test)]
mod round_trip_tests {
    use super::*;
    use crate::core::ArrayCalculator;
    use crate::types::{ColumnValue, ParsedModel};

    fn calculate(model: ParsedModel) -> ForgeResult<ParsedModel> {
        ArrayCalculator::new(model).calculate_all()
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    /// Export `path`, import the workbook, and compare every calculated number
    /// with the original. Returns the differences found.
    fn round_trip(path: &Path) -> Result<Vec<String>, String> {
        let original = calculate(parser::parse_model(path).map_err(|e| e.to_string())?)
            .map_err(|e| format!("original does not calculate: {e}"))?;
        let dir = tempfile::TempDir::new().unwrap();
        let xlsx = dir.path().join("model.xlsx");
        let yaml = dir.path().join("model.yaml");
        ExcelExporter::new(original.clone())
            .export(&xlsx)
            .map_err(|e| format!("export: {e}"))?;
        import_core(&xlsx, &yaml, false, false).map_err(|e| format!("import: {e}"))?;
        let back = parser::parse_model(&yaml).map_err(|e| format!("imported YAML: {e}"))?;
        let back = calculate(back).map_err(|e| format!("imported model: {e}"))?;

        let mut diffs = Vec::new();
        for (path, var) in &original.scalars {
            if let Some(a) = var.value {
                let lower = path.to_lowercase();
                match back.scalars.get(path).or_else(|| back.scalars.get(&lower)) {
                    Some(v) if v.value.is_some_and(|b| close(a, b)) => {},
                    Some(v) => diffs.push(format!("{path}: {a} vs {:?}", v.value)),
                    None => diffs.push(format!("{path}: missing")),
                }
            }
        }
        for (name, table) in &original.tables {
            for (col, column) in &table.columns {
                let ColumnValue::Number(a) = &column.values else {
                    continue;
                };
                let b = back
                    .tables
                    .get(&name.to_lowercase())
                    .and_then(|t| t.columns.get(col));
                match b.map(|c| &c.values) {
                    Some(ColumnValue::Number(b))
                        if a.len() == b.len() && a.iter().zip(b).all(|(x, y)| close(*x, *y)) => {},
                    other => diffs.push(format!("{name}.{col}: {a:?} vs {other:?}")),
                }
            }
        }
        Ok(diffs)
    }

    /// Columns and scalars keep the order of the model file through calculate,
    /// export and import, and the numbers survive. The file order here is
    /// deliberately not alphabetical, and mixes data and formula columns.
    #[test]
    fn model_order_survives_calculate_export_and_import() {
        let dir = tempfile::TempDir::new().unwrap();
        let model = dir.path().join("order.yaml");
        fs::write(
            &model,
            r#"_forge_version: "5.0.0"
t:
  zeta: [1, 2, 3]
  alpha: "=zeta * 2"
  mid: [10, 20, 30]
  beta: "=alpha + mid"
g:
  z_last:
    value: 5
  a_first:
    value: null
    formula: "=g.z_last * 3"
"#,
        )
        .unwrap();

        let calculated = calculate(parser::parse_model(&model).unwrap()).unwrap();
        let order = |m: &ParsedModel| -> (Vec<String>, Vec<String>) {
            let columns = m.tables["t"].column_names().into_iter().cloned().collect();
            let scalars = m.scalars.keys().cloned().collect();
            (columns, scalars)
        };
        let expected_columns = ["zeta", "alpha", "mid", "beta"].map(String::from).to_vec();
        let expected_scalars = ["g.z_last", "g.a_first"].map(String::from).to_vec();
        assert_eq!(
            order(&calculated),
            (expected_columns.clone(), expected_scalars.clone())
        );

        let xlsx = dir.path().join("order.xlsx");
        let back_yaml = dir.path().join("back.yaml");
        ExcelExporter::new(calculated).export(&xlsx).unwrap();
        import_core(&xlsx, &back_yaml, false, false).unwrap();
        let back = calculate(parser::parse_model(&back_yaml).unwrap()).unwrap();
        assert_eq!(order(&back), (expected_columns, expected_scalars));
        assert_eq!(
            back.tables["t"].columns["beta"].values,
            ColumnValue::Number(vec![12.0, 24.0, 36.0])
        );
        assert_eq!(back.scalars["g.a_first"].value, Some(15.0));

        let diffs = round_trip(&model).unwrap();
        assert!(diffs.is_empty(), "{diffs:#?}");
    }

    /// The shipped examples that calculate on their own come back from Excel
    /// with the same numbers.
    #[test]
    fn examples_round_trip_through_excel() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for example in [
            "scenarios",
            "variance",
            "breakeven",
            "quota_forecast",
            "bagel/bagel",
            "bagel/actual",
            "bagel/costs",
        ] {
            let file = root.join(format!("examples/{example}.yaml"));
            let diffs = round_trip(&file).unwrap_or_else(|e| panic!("{example}: {e}"));
            assert!(diffs.is_empty(), "{example}: {diffs:#?}");
        }
    }

    /// T1-EXPORT-INCLUDES: a value from an included file is a cell reference
    /// in Excel, the included file's own formula is a formula on its sheet,
    /// and import gives back the included file and `@costs` references.
    #[test]
    fn included_values_stay_live_and_come_back_as_an_include() {
        use calamine::{open_workbook, Reader, Xlsx};
        let dir = tempfile::TempDir::new().unwrap();
        let costs = "_forge_version: \"5.1.0\"\n# kept as written\nlabour:\n  wage: 18.5\n  load: 0.15\n  loaded:\n    formula: \"=wage * (1 + load)\"\n";
        fs::write(dir.path().join("costs.yaml"), costs).unwrap();
        let main = dir.path().join("main.yaml");
        fs::write(
            &main,
            "_forge_version: \"5.1.0\"\n_includes:\n  - file: costs.yaml\n    as: costs\nshop:\n  hours: 100\n  pay:\n    formula: \"=hours * @costs.labour.loaded\"\n",
        )
        .unwrap();
        let original = calculate(parser::parse_model(&main).unwrap()).unwrap();
        let xlsx = dir.path().join("main.xlsx");
        let warnings = ExcelExporter::new(original.clone())
            .export_with_warnings(&xlsx)
            .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        let mut wb: Xlsx<_> = open_workbook(&xlsx).unwrap();
        let shop = wb.worksheet_formula("shop").unwrap();
        assert_eq!(
            shop.get_value((2, 1)).map(String::as_str),
            Some("$B$2 * 'costs.Scalars'!$B$4"),
            "pay reads the include's cell"
        );
        let inc = wb.worksheet_formula("costs.Scalars").unwrap();
        assert_eq!(
            inc.get_value((3, 1)).map(String::as_str),
            Some("'costs.Scalars'!$B$2 * (1 + 'costs.Scalars'!$B$3)"),
            "the include's own formula is a formula"
        );

        // Into another folder: the included file is written, and read by `@costs`
        let other = tempfile::TempDir::new().unwrap();
        let back = other.path().join("main.yaml");
        import_core(&xlsx, &back, false, false).unwrap();
        let text = fs::read_to_string(&back).unwrap();
        assert!(
            text.contains("_includes:\n  - file: costs.yaml\n    as: costs\n"),
            "{text}"
        );
        assert!(
            text.contains("=shop.hours * @costs.labour.loaded"),
            "{text}"
        );
        let included = fs::read_to_string(other.path().join("costs.yaml")).unwrap();
        assert!(
            included.contains("labour:\n  wage: 18.5\n  load: 0.15\n  loaded:\n    formula: =labour.wage * (1 + labour.load)"),
            "{included}"
        );
        let calculated = calculate(parser::parse_model(&back).unwrap()).unwrap();
        assert_eq!(
            calculated.scalars["shop.pay"].value,
            original.scalars["shop.pay"].value
        );

        // Beside the original: it means the same, so it stays as written
        import_core(&xlsx, &dir.path().join("back.yaml"), false, false).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("costs.yaml")).unwrap(),
            costs
        );

        // Beside a file with other values: refused, nothing written
        let changed = tempfile::TempDir::new().unwrap();
        fs::write(
            changed.path().join("costs.yaml"),
            costs.replace("18.5", "19"),
        )
        .unwrap();
        let err = import_core(&xlsx, &changed.path().join("main.yaml"), false, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("hold other values"), "{err}");
        assert!(
            !changed.path().join("main.yaml").exists(),
            "nothing may be written"
        );

        // Split files: the part that reads `@costs` lists the include
        let split = other.path().join("split");
        import_core(&xlsx, &split, true, false).unwrap();
        let scalars = fs::read_to_string(split.join("scalars.yaml")).unwrap();
        assert!(scalars.contains("_includes:"), "{scalars}");
        let part = calculate(parser::parse_model(&split.join("scalars.yaml")).unwrap()).unwrap();
        assert_eq!(
            part.scalars["shop.pay"].value,
            original.scalars["shop.pay"].value
        );
    }

    /// A column shorter than its table is its own range: with the table's
    /// rows, `ROWS(d.short)` was 5 in Excel and 2 in Forge, and `INDEX` past
    /// the column's end read an empty cell.
    #[test]
    fn a_short_column_is_exported_as_its_own_range() {
        use calamine::{open_workbook, Reader, Xlsx};
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("m.yaml");
        fs::write(
            &path,
            "_forge_version: \"5.1.0\"\nd:\n  long: [1, 2, 3, 4, 5]\n  short: [7, 8]\ns:\n  rows:\n    formula: \"=ROWS(d.short)\"\n  last:\n    formula: \"=INDEX(d.short, ROWS(d.short))\"\n",
        )
        .unwrap();
        let model = calculate(parser::parse_model(&path).unwrap()).unwrap();
        assert_eq!(model.scalars["s.rows"].value, Some(2.0));
        let xlsx = dir.path().join("m.xlsx");
        ExcelExporter::new(model)
            .export_with_warnings(&xlsx)
            .unwrap();
        let mut wb: Xlsx<_> = open_workbook(&xlsx).unwrap();
        let s = wb.worksheet_formula("s").unwrap();
        assert_eq!(
            s.get_value((1, 1)).map(String::as_str),
            Some("ROWS('d'!$B$2:$B$3)")
        );
        assert_eq!(
            s.get_value((2, 1)).map(String::as_str),
            Some("INDEX('d'!$B$2:$B$3, ROWS('d'!$B$2:$B$3))")
        );
    }

    /// T2-EXPORT-LET: LET, a LAMBDA called at once, COUNTUNIQUE of a column
    /// and TODAY are Excel formulas, and import reads them back to formulas
    /// that calculate the same. (forge-e2e-gnumeric recalculates each kind in
    /// `LibreOffice`.)
    #[test]
    fn let_lambda_countunique_and_today_are_formulas_both_ways() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("m.yaml");
        fs::write(
            &path,
            concat!(
                "_forge_version: \"5.1.0\"\n",
                "t:\n  name: [\"a\", \"A\", \"b\", \"a\"]\n  n: [1, 2, 2, 3]\n  year: \"=YEAR(TODAY()) + n\"\n",
                "x: 100\n",
                "s:\n",
                "  let:\n    formula: \"=LET(a, 2, b, a * 3, a + b)\"\n",
                "  lambda:\n    formula: \"=LAMBDA(x, r, x * r + 1)(4, x)\"\n",
                "  nested:\n    formula: \"=LET(v, LET(a, 5, a * 2), v + x)\"\n",
                "  text:\n    formula: \"=COUNTUNIQUE(t.name)\"\n",
                "  numbers:\n    formula: \"=COUNTUNIQUE(t.n)\"\n",
                "  today:\n    formula: \"=TODAY() - TODAY()\"\n",
            ),
        )
        .unwrap();
        let model = calculate(parser::parse_model(&path).unwrap()).unwrap();
        let xlsx = dir.path().join("m.xlsx");
        let warnings = ExcelExporter::new(model.clone())
            .export_with_warnings(&xlsx)
            .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        let back = dir.path().join("back.yaml");
        import_core(&xlsx, &back, false, false).unwrap();
        let text = fs::read_to_string(&back).unwrap();
        for formula in [
            "=LET(a, 2, b, a * 3, a + b)",
            "=LET(x_, 4, r_, x, x_ * r_ + 1)",
            "=LET(a, 5, v, a * 2, v + x)",
            "=COUNTUNIQUE(t.name)",
            "=COUNTUNIQUE(t.n)",
            "=YEAR(TODAY()) + n",
        ] {
            assert!(text.contains(formula), "{formula} in {text}");
        }
        let again = calculate(parser::parse_model(&back).unwrap()).unwrap();
        for (name, want) in [
            ("s.let", 8.0),
            ("s.lambda", 401.0),
            ("s.nested", 110.0),
            ("s.text", 3.0),
            ("s.numbers", 3.0),
            ("s.today", 0.0),
        ] {
            assert_eq!(model.scalars[name].value, Some(want), "{name}");
            assert_eq!(again.scalars[name].value, Some(want), "{name} after import");
        }
    }

    /// T2-INCLUDE-COLUMNS: a table of an included file is a sheet with its
    /// formulas, a formula that reads one of its columns reads that sheet
    /// (the same row in a row formula, the whole column in an aggregate), and
    /// import puts the table back in the included file.
    #[test]
    fn included_tables_stay_live_and_come_back_in_the_include() {
        use calamine::{open_workbook, Reader, Xlsx};
        let dir = tempfile::TempDir::new().unwrap();
        let prices = "_forge_version: \"5.1.0\"\ntax:\n  rate: 0.15\nlist:\n  per_kg: [1.15, 9.40, 0.80]\n  with_tax: \"=per_kg * (1 + tax.rate)\"\n";
        fs::write(dir.path().join("prices.yaml"), prices).unwrap();
        let main = dir.path().join("main.yaml");
        fs::write(
            &main,
            "_forge_version: \"5.1.0\"\n_includes:\n  - file: prices.yaml\n    as: prices\norder:\n  kg: [120, 6, 3]\n  cost: \"=kg * @prices.list.with_tax\"\ntotal:\n  dearest:\n    formula: \"=MAX(@prices.list.per_kg)\"\n",
        )
        .unwrap();
        let original = calculate(parser::parse_model(&main).unwrap()).unwrap();
        let xlsx = dir.path().join("main.xlsx");
        let warnings = ExcelExporter::new(original.clone())
            .export_with_warnings(&xlsx)
            .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");

        let mut wb: Xlsx<_> = open_workbook(&xlsx).unwrap();
        let order = wb.worksheet_formula("order").unwrap();
        assert_eq!(
            order.get_value((1, 1)).map(String::as_str),
            Some("A2 * 'prices.list'!B2"),
            "the same row of the included table"
        );
        let list = wb.worksheet_formula("prices.list").unwrap();
        assert_eq!(
            list.get_value((1, 1)).map(String::as_str),
            Some("A2 * (1 + 'prices.Scalars'!$B$2)"),
            "the included table's own formula"
        );
        let total = wb.worksheet_formula("total").unwrap();
        assert_eq!(
            total.get_value((1, 1)).map(String::as_str),
            Some("MAX('prices.list'!$A$2:$A$4)")
        );

        let other = tempfile::TempDir::new().unwrap();
        let back = other.path().join("main.yaml");
        import_core(&xlsx, &back, false, false).unwrap();
        let text = fs::read_to_string(&back).unwrap();
        assert!(text.contains("cost: =kg * @prices.list.with_tax"), "{text}");
        assert!(text.contains("=MAX(@prices.list.per_kg)"), "{text}");
        assert!(
            !text.contains("per_kg:"),
            "the table stays in the include: {text}"
        );
        let included = fs::read_to_string(other.path().join("prices.yaml")).unwrap();
        assert!(
            included.contains("with_tax: =per_kg * (1 + tax.rate)"),
            "{included}"
        );
        let calculated = calculate(parser::parse_model(&back).unwrap()).unwrap();
        assert_eq!(
            calculated.tables["order"].columns["cost"].values,
            original.tables["order"].columns["cost"].values
        );
        assert_eq!(
            calculated.scalars["total.dearest"].value,
            original.scalars["total.dearest"].value
        );

        // Beside the original, which means the same: kept as written
        import_core(&xlsx, &dir.path().join("back.yaml"), false, false).unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join("prices.yaml")).unwrap(),
            prices
        );
        // Beside one whose table holds another price: refused
        let changed = tempfile::TempDir::new().unwrap();
        fs::write(
            changed.path().join("prices.yaml"),
            prices.replace("9.40", "9.50"),
        )
        .unwrap();
        let err = import_core(&xlsx, &changed.path().join("main.yaml"), false, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("hold other values"), "{err}");
    }

    /// ADR-052: `--multi-doc` gives the same numbers as one file, and
    /// `--split-files` refuses a model whose parts refer to each other.
    #[test]
    fn multi_doc_import_calculates_like_one_file_and_split_refuses() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let model =
            calculate(parser::parse_model(&root.join("examples/bagel/bagel.yaml")).unwrap())
                .unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let xlsx = dir.path().join("m.xlsx");
        ExcelExporter::new(model).export(&xlsx).unwrap();

        let single = dir.path().join("single.yaml");
        let multi = dir.path().join("multi.yaml");
        import_core(&xlsx, &single, false, false).unwrap();
        import_core(&xlsx, &multi, false, true).unwrap();
        let a = calculate(parser::parse_model(&single).unwrap()).unwrap();
        let b = calculate(parser::parse_model(&multi).unwrap()).unwrap();
        assert_eq!(a.scalars.len(), b.scalars.len());
        for (name, var) in &a.scalars {
            assert_eq!(var.value, b.scalars[name].value, "{name}");
        }
        for (name, table) in &a.tables {
            assert_eq!(
                format!("{:?}", table.columns),
                format!("{:?}", b.tables[name].columns),
                "{name}"
            );
        }

        let split = dir.path().join("split");
        let err = import_core(&xlsx, &split, true, false)
            .unwrap_err()
            .to_string();
        assert!(err.contains("--split-files would break"), "{err}");
        assert!(!split.exists(), "nothing may be written");
    }

    /// `forge import` writes model format 5.1.0, without nulls (ADR-051).
    #[test]
    fn imported_examples_are_short_form() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for example in ["variance", "breakeven", "bagel/bagel"] {
            let model = calculate(
                parser::parse_model(&root.join(format!("examples/{example}.yaml"))).unwrap(),
            )
            .unwrap();
            let dir = tempfile::TempDir::new().unwrap();
            let xlsx = dir.path().join("m.xlsx");
            let yaml = dir.path().join("m.yaml");
            ExcelExporter::new(model).export(&xlsx).unwrap();
            import_core(&xlsx, &yaml, false, false).unwrap();
            let text = fs::read_to_string(&yaml).unwrap();
            assert!(text.starts_with("_forge_version: \"5.1.0\""), "{example}");
            assert!(!text.contains("null"), "{example}:\n{text}");
            assert!(
                !text.contains(".0\n"),
                "{example}: whole numbers written as floats\n{text}"
            );
        }
    }

    /// `FORGE_ROUNDTRIP_DIR=<dir> cargo test round_trip_corpus -- --ignored --nocapture`
    /// round-trips every YAML model in a directory and prints a summary.
    #[test]
    #[ignore = "needs FORGE_ROUNDTRIP_DIR"]
    fn round_trip_corpus() {
        let dir = std::env::var("FORGE_ROUNDTRIP_DIR").expect("set FORGE_ROUNDTRIP_DIR");
        let mut files: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "yaml"))
            .collect();
        files.sort();
        let (mut same, mut different, mut skipped) = (0, 0, 0);
        for file in &files {
            match round_trip(file) {
                Ok(d) if d.is_empty() => same += 1,
                Ok(d) => {
                    different += 1;
                    println!("DIFF {}: {} values, e.g. {}", file.display(), d.len(), d[0]);
                },
                Err(e) => {
                    skipped += 1;
                    println!("SKIP {}: {e}", file.display());
                },
            }
        }
        println!(
            "models {}: same {same}, different {different}, not round-tripped {skipped}",
            files.len()
        );
    }
}
