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
/// row formulas as `=...` strings.
fn table_to_value(table: &crate::types::Table) -> yaml_serde::Value {
    use crate::types::ColumnValue;
    use yaml_serde::Value;
    let mut map = yaml_serde::Mapping::new();
    let mut names: Vec<&String> = table.columns.keys().collect();
    names.sort();
    for name in names {
        let items: Vec<Value> = match &table.columns[name].values {
            ColumnValue::Number(v) => v.iter().map(|n| Value::Number((*n).into())).collect(),
            ColumnValue::Text(v) | ColumnValue::Date(v) => {
                v.iter().map(|t| Value::String(t.clone())).collect()
            },
            ColumnValue::Boolean(v) => v.iter().map(|b| Value::Bool(*b)).collect(),
        };
        map.insert(Value::String(name.clone()), Value::Sequence(items));
    }
    let mut formulas: Vec<(&String, &String)> = table.row_formulas.iter().collect();
    formulas.sort();
    for (name, formula) in formulas {
        map.insert(Value::String(name.clone()), Value::String(formula.clone()));
    }
    Value::Mapping(map)
}

/// All tables as `{table_name: table}`.
fn tables_to_value(
    tables: &std::collections::HashMap<String, crate::types::Table>,
) -> yaml_serde::Mapping {
    let mut names: Vec<&String> = tables.keys().collect();
    names.sort();
    let mut map = yaml_serde::Mapping::new();
    for name in names {
        map.insert(
            yaml_serde::Value::String(name.clone()),
            table_to_value(&tables[name]),
        );
    }
    map
}

/// Scalars as the parser reads them: `group: {name: {value, formula}}` for
/// `group.name`, and `name: {value, formula}` at the top for ungrouped ones.
fn scalars_to_value(
    scalars: &std::collections::HashMap<String, crate::types::Variable>,
) -> yaml_serde::Mapping {
    use yaml_serde::{Mapping, Value};
    let mut names: Vec<&String> = scalars.keys().collect();
    names.sort();
    let mut map = Mapping::new();
    for path in names {
        let variable = &scalars[path];
        let mut entry = Mapping::new();
        if let Some(value) = variable.value {
            entry.insert(Value::String("value".into()), Value::Number(value.into()));
        }
        if let Some(formula) = &variable.formula {
            entry.insert(
                Value::String("formula".into()),
                Value::String(formula.clone()),
            );
        }
        if entry.is_empty() {
            entry.insert(Value::String("value".into()), Value::Null);
        }
        match path.split_once('.') {
            Some((group, name)) => {
                let group_entry = map
                    .entry(Value::String(group.to_string()))
                    .or_insert_with(|| Value::Mapping(Mapping::new()));
                if let Value::Mapping(group_map) = group_entry {
                    group_map.insert(Value::String(name.to_string()), Value::Mapping(entry));
                }
            },
            None => {
                map.insert(Value::String(path.clone()), Value::Mapping(entry));
            },
        }
    }
    map
}

/// The whole imported model as YAML that `forge validate` and `forge calculate` accept.
fn model_to_yaml(model: &crate::types::ParsedModel) -> ForgeResult<String> {
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
        return Ok("_forge_version: \"5.0.0\"\n".to_string());
    }
    let body = yaml_serde::to_string(&map).map_err(ForgeError::Yaml)?;
    Ok(format!("_forge_version: \"5.0.0\"\n\n{body}"))
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
        write_split_files_quiet(output, model)?;
    } else if multi_doc {
        write_multi_doc(output, model)?;
    } else {
        let yaml_string = model_to_yaml(&model)?;
        fs::write(output, yaml_string).map_err(ForgeError::Io)?;
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

/// Write split files without printing (for core function)
fn write_split_files_quiet(output: &Path, model: crate::types::ParsedModel) -> ForgeResult<()> {
    fs::create_dir_all(output).map_err(ForgeError::Io)?;
    for (table_name, table) in &model.tables {
        let mut table_model = crate::types::ParsedModel::new();
        table_model.tables.insert(table_name.clone(), table.clone());
        let file_path = output.join(format!("{table_name}.yaml"));
        let yaml_string = format!(
            "_forge_version: \"1.0.0\"\n_name: \"{}\"\n\n{}",
            table_name,
            yaml_serde::to_string(&tables_to_value(&table_model.tables))
                .map_err(ForgeError::Yaml)?
        );
        fs::write(&file_path, yaml_string).map_err(ForgeError::Io)?;
    }
    if !model.scalars.is_empty() {
        let file_path = output.join("scalars.yaml");
        let mut scalar_model = crate::types::ParsedModel::new();
        scalar_model.scalars = model.scalars;
        let yaml_string = format!(
            "_forge_version: \"1.0.0\"\n_name: \"scalars\"\n\n{}",
            yaml_serde::to_string(&scalars_to_value(&scalar_model.scalars))
                .map_err(ForgeError::Yaml)?
        );
        fs::write(&file_path, yaml_string).map_err(ForgeError::Io)?;
    }
    Ok(())
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
        println!(
            "{}",
            "⚠️  Written as calculated values, because Excel cannot express them:".yellow()
        );
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
        write_split_files(output, model, verbose)?;
    } else if multi_doc {
        write_multi_doc(output, model)?;
    } else {
        let yaml_string = model_to_yaml(&model)?;
        fs::write(output, yaml_string).map_err(ForgeError::Io)?;
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
    model: crate::types::ParsedModel,
    verbose: bool,
) -> ForgeResult<()> {
    fs::create_dir_all(output).map_err(ForgeError::Io)?;
    for (table_name, table) in &model.tables {
        let mut table_model = crate::types::ParsedModel::new();
        table_model.tables.insert(table_name.clone(), table.clone());
        let file_path = output.join(format!("{table_name}.yaml"));
        let yaml_string = format!(
            "_forge_version: \"1.0.0\"\n_name: \"{}\"\n\n{}",
            table_name,
            yaml_serde::to_string(&tables_to_value(&table_model.tables))
                .map_err(ForgeError::Yaml)?
        );
        fs::write(&file_path, yaml_string).map_err(ForgeError::Io)?;
        if verbose {
            println!("   Created: {}", file_path.display());
        }
    }
    if !model.scalars.is_empty() {
        let file_path = output.join("scalars.yaml");
        let mut scalar_model = crate::types::ParsedModel::new();
        scalar_model.scalars = model.scalars;
        let yaml_string = format!(
            "_forge_version: \"1.0.0\"\n_name: \"scalars\"\n\n{}",
            yaml_serde::to_string(&scalars_to_value(&scalar_model.scalars))
                .map_err(ForgeError::Yaml)?
        );
        fs::write(&file_path, yaml_string).map_err(ForgeError::Io)?;
        if verbose {
            println!("   Created: {}", file_path.display());
        }
    }
    println!("{}", "✅ Import Complete!".bold().green());
    println!("   Output directory: {}\n", output.display());
    Ok(())
}

fn write_multi_doc(output: &Path, model: crate::types::ParsedModel) -> ForgeResult<()> {
    let mut yaml_output = String::new();
    for (table_name, table) in &model.tables {
        let mut table_model = crate::types::ParsedModel::new();
        table_model.tables.insert(table_name.clone(), table.clone());
        yaml_output.push_str("---\n");
        yaml_output.push_str("_forge_version: \"1.0.0\"\n");
        let _ = write!(yaml_output, "_name: \"{table_name}\"\n\n");
        yaml_output.push_str(
            &yaml_serde::to_string(&tables_to_value(&table_model.tables))
                .map_err(ForgeError::Yaml)?,
        );
        yaml_output.push('\n');
    }
    if !model.scalars.is_empty() {
        let mut scalar_model = crate::types::ParsedModel::new();
        scalar_model.scalars = model.scalars;
        yaml_output.push_str("---\n");
        yaml_output.push_str("_forge_version: \"1.0.0\"\n");
        yaml_output.push_str("_name: \"scalars\"\n\n");
        yaml_output.push_str(
            &yaml_serde::to_string(&scalars_to_value(&scalar_model.scalars))
                .map_err(ForgeError::Yaml)?,
        );
    }
    fs::write(output, yaml_output).map_err(ForgeError::Io)?;
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
        assert!(yaml.starts_with("_forge_version: \"5.0.0\""));
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

    /// The shipped examples that calculate on their own come back from Excel
    /// with the same numbers.
    #[test]
    fn examples_round_trip_through_excel() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for example in ["scenarios", "variance", "breakeven", "quota_forecast"] {
            let file = root.join(format!("examples/{example}.yaml"));
            let diffs = round_trip(&file).unwrap_or_else(|e| panic!("{example}: {e}"));
            assert!(diffs.is_empty(), "{example}: {diffs:#?}");
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
