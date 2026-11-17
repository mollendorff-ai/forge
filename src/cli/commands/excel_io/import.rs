//! `forge import`: a workbook back as a model, in one file, split files or
//! one file of several documents.

use crate::error::{ForgeError, ForgeResult};
use crate::excel::ExcelImporter;
use crate::parser;
// Printing the commands belongs to the command line (feature `cli`)
use std::fmt::Write as _;
use std::fs;
use std::path::Path;
#[cfg(feature = "cli")]
use {crate::cli::console::outln, crate::cli::Console, colored::Colorize};

/// A workbook imported into YAML text, for a caller with no files.
pub struct ImportText {
    /// The main model, as `forge import` writes it to a single file
    pub main: String,
    /// Each included file's text, by the name `_includes` gives it
    pub includes: Vec<(String, String)>,
    pub table_count: usize,
    pub scalar_count: usize,
    /// As `forge import`: formulas imported as values, text in a column of numbers
    pub warnings: Vec<String>,
}

/// [`import_core`] of a single file, from the workbook's bytes to YAML text.
///
/// # Errors
///
/// Returns an error if the bytes are not a workbook Forge can read, or the
/// model cannot be written as YAML.
pub fn import_text_core(bytes: Vec<u8>) -> ForgeResult<ImportText> {
    let (model, warnings) = ExcelImporter::from_bytes(bytes).import_with_warnings()?;
    let mut includes = Vec::new();
    for include in &model.includes {
        if let Some(resolved) = model.resolved_includes.get(&include.namespace) {
            includes.push((include.file.clone(), model_to_yaml(&resolved.model)?));
        }
    }
    Ok(ImportText {
        main: model_to_yaml(&model)?,
        includes,
        table_count: model.tables.len(),
        scalar_count: model.scalars.len(),
        warnings,
    })
}

/// One table in the YAML format the parser reads: data columns as arrays,
/// row formulas as `=...` strings, in the table's column order. A name with
/// a row formula is written as the formula.
fn table_to_value(table: &crate::types::Table) -> yaml_serde::Value {
    use crate::types::ColumnValue;
    use yaml_serde::Value;
    let mut map = yaml_serde::Mapping::new();
    // A column with a format: in its long form (ADR-061)
    let with_format = |value: Value, key: &str, name: &str| -> Value {
        match table.format_of(name) {
            Some(format) => {
                let mut long = yaml_serde::Mapping::new();
                long.insert(Value::from(key), value);
                long.insert(Value::from("format"), Value::from(format));
                Value::Mapping(long)
            },
            None => value,
        }
    };
    for name in table.column_names() {
        if let Some(formula) = table.row_formulas.get(name) {
            let formula = with_format(Value::String(formula.clone()), "formula", name);
            map.insert(Value::String(name.clone()), formula);
            continue;
        }
        let items: Vec<Value> = match &table.columns[name].values {
            ColumnValue::Number(v) => v.iter().map(|n| number(*n)).collect(),
            ColumnValue::Text(v) | ColumnValue::Date(v) => {
                v.iter().map(|t| Value::String(t.clone())).collect()
            },
            ColumnValue::Boolean(v) => v.iter().map(|b| Value::Bool(*b)).collect(),
            // An empty cell is null (ADR-059)
            ColumnValue::Blanks(v) => v.iter().map(|n| n.map_or(Value::Null, number)).collect(),
        };
        let column = with_format(Value::Sequence(items), "value", name);
        map.insert(Value::String(name.clone()), column);
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
    #[allow(clippy::cast_possible_truncation)] // whole and in range
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
        let format = variable.metadata.format.as_ref();
        let entry = match (&variable.formula, variable.value) {
            // An input with a format: is a mapping (ADR-061)
            (None, Some(value)) if format.is_some() => {
                let mut entry = Mapping::new();
                let value = variable
                    .date_text()
                    .map_or_else(|| number(value), Value::String);
                entry.insert(Value::from("value"), value);
                entry.insert(
                    Value::from("format"),
                    Value::from(format.cloned().unwrap_or_default()),
                );
                Value::Mapping(entry)
            },
            // A date input is its text (ADR-060)
            (None, Some(_)) if variable.is_date => {
                Value::String(variable.date_text().unwrap_or_default())
            },
            (None, Some(value)) => number(value),
            (Some(formula), value) => {
                let mut entry = Mapping::new();
                entry.insert(Value::from("formula"), Value::from(formula.clone()));
                if let Some(value) = value {
                    entry.insert(Value::from("value"), number(value));
                }
                if let Some(format) = format {
                    entry.insert(Value::from("format"), Value::from(format.clone()));
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
fn write_single(output: &Path, model: &crate::types::ParsedModel) -> ForgeResult<Vec<String>> {
    let yaml = model_to_yaml(model)?;
    let dir = output.parent().unwrap_or_else(|| Path::new("."));
    let written = write_include_files(dir, model)?;
    fs::write(output, yaml).map_err(ForgeError::Io)?;
    Ok(written)
}

/// Write the included files the workbook holds, beside the model, and give
/// their names. A file that is already there with the same values is not
/// written again, and is not listed.
fn write_include_files(dir: &Path, model: &crate::types::ParsedModel) -> ForgeResult<Vec<String>> {
    let mut written = Vec::new();
    for (path, text) in include_files(dir, model)? {
        fs::write(&path, text).map_err(ForgeError::Io)?;
        written.push(
            path.strip_prefix(dir)
                .unwrap_or(&path)
                .display()
                .to_string(),
        );
    }
    Ok(written)
}

/// The model format an imported model needs: 5.4.0 when a scalar is a date
/// (ADR-060), 5.3.0 when a column has an empty cell (ADR-059), 5.1.0
/// otherwise.
fn import_format(model: &crate::types::ParsedModel) -> &'static str {
    // A date input (ADR-060) or a format: (ADR-061) is 5.4.0
    let dates = model
        .scalars
        .values()
        .any(|v| v.metadata.format.is_some() || (v.is_date && v.formula.is_none()))
        || model.tables.values().any(|t| {
            t.column_names()
                .into_iter()
                .any(|name| t.format_of(name).is_some())
        });
    let blanks = model.tables.values().any(|table| {
        table
            .columns
            .values()
            .any(|c| matches!(c.values, crate::types::ColumnValue::Blanks(_)))
    });
    if dates {
        "5.4.0"
    } else if blanks {
        "5.3.0"
    } else {
        "5.1.0"
    }
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
    let body =
        yaml_serde::to_string(&crate::core::precision::Rounded(&map)).map_err(ForgeError::Yaml)?;
    let version = import_format(model);
    Ok(format!("_forge_version: \"{version}\"\n{includes}\n{body}"))
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
) -> ForgeResult<crate::cli::commands::results::ImportResult> {
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

    let mut included_files = Vec::new();
    if split_files {
        write_split_files_quiet(output, &model)?;
    } else if multi_doc {
        included_files = write_multi_doc_quiet(output, &model)?;
    } else {
        included_files = write_single(output, &model)?;
    }

    Ok(crate::cli::commands::results::ImportResult {
        input_path: input.display().to_string(),
        output_path: output.display().to_string(),
        table_count,
        scalar_count,
        mode: mode.to_string(),
        warnings,
        included_files,
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
        let body = yaml_serde::to_string(&crate::core::precision::Rounded(&tables_to_value(&one)))
            .map_err(ForgeError::Yaml)?;
        parts.push((format!("{table_name}.yaml"), table_name.clone(), body));
    }
    if !model.scalars.is_empty() {
        let body = yaml_serde::to_string(&crate::core::precision::Rounded(&scalars_to_value(
            &model.scalars,
        )))
        .map_err(ForgeError::Yaml)?;
        parts.push(("scalars.yaml".to_string(), "scalars".to_string(), body));
    }
    fs::create_dir_all(output).map_err(ForgeError::Io)?;
    let includes = include_files(output, model)?;
    let mut written = Vec::new();
    for (file, name, body) in parts {
        let path = output.join(file);
        let includes = includes_block(model, Some(&body));
        let version = import_format(model);
        let text = format!("_forge_version: \"{version}\"\n_name: \"{name}\"\n{includes}\n{body}");
        fs::write(&path, text).map_err(ForgeError::Io)?;
        written.push(path);
    }
    for (path, text) in includes {
        fs::write(&path, text).map_err(ForgeError::Io)?;
        written.push(path);
    }
    Ok(written)
}

/// Execute the import command.
///
/// # Errors
///
/// Returns an error if the Excel file cannot be read, imported, or written as YAML.
#[cfg(feature = "cli")]
pub fn import(
    c: &mut Console<'_>,
    input: &Path,
    output: &Path,
    verbose: bool,
    split_files: bool,
    multi_doc: bool,
) -> ForgeResult<()> {
    outln!(c, "{}", "🔥 Forge - Excel Import".bold().green());
    outln!(c, "   Input:  {}", input.display());
    outln!(c, "   Output: {}", output.display());
    if split_files {
        outln!(c, "   Mode:   Split files (one YAML per sheet)");
    } else if multi_doc {
        outln!(c, "   Mode:   Multi-document YAML");
    }
    outln!(c);

    if verbose {
        outln!(c, "{}", "📖 Reading Excel file...".cyan());
    }

    let importer = ExcelImporter::new(input);
    let (model, warnings) = importer.import_with_warnings()?;

    if verbose {
        outln!(c, "   Found {} tables", model.tables.len());
        outln!(c, "   Found {} scalars\n", model.scalars.len());
        for (table_name, table) in &model.tables {
            outln!(c, "   📊 Table: {}", table_name.bright_blue());
            outln!(
                c,
                "      {} columns, {} rows",
                table.columns.len(),
                table.row_count()
            );
        }
        outln!(c);
        outln!(c, "{}", "💾 Writing YAML file(s)...".cyan());
    }

    if split_files {
        write_split_files(c, output, &model, verbose)?;
    } else if multi_doc {
        write_multi_doc(c, output, &model)?;
    } else {
        let included = write_single(output, &model)?;
        outln!(c, "{}", "✅ Import Complete!".bold().green());
        outln!(c, "   YAML file: {}", output.display());
        print_included_files(c, &included);
        outln!(c);
    }

    crate::cli::commands::carried::print_import_formulas(c, &model, &warnings);
    Ok(())
}

#[cfg(feature = "cli")]
fn write_split_files(
    c: &mut Console<'_>,
    output: &Path,
    model: &crate::types::ParsedModel,
    verbose: bool,
) -> ForgeResult<()> {
    let written = write_split_files_quiet(output, model)?;
    if verbose {
        for path in &written {
            outln!(c, "   Created: {}", path.display());
        }
    }
    outln!(c, "{}", "✅ Import Complete!".bold().green());
    outln!(c, "   Output directory: {}\n", output.display());
    Ok(())
}

/// One file, one document per table and one for the scalars (ADR-052: one
/// namespace), the first listing the included files, written beside it.
fn write_multi_doc_quiet(
    output: &Path,
    model: &crate::types::ParsedModel,
) -> ForgeResult<Vec<String>> {
    let mut yaml_output = String::new();
    let mut includes = includes_block(model, None);
    let version = import_format(model);
    let mut document = |name: &str, body: String| {
        let _ = writeln!(yaml_output, "---\n_forge_version: \"{version}\"");
        let _ = writeln!(yaml_output, "_name: \"{name}\"");
        yaml_output.push_str(&std::mem::take(&mut includes));
        yaml_output.push('\n');
        yaml_output.push_str(&body);
    };
    for (table_name, table) in &model.tables {
        let mut one = indexmap::IndexMap::new();
        one.insert(table_name.clone(), table.clone());
        let body = yaml_serde::to_string(&crate::core::precision::Rounded(&tables_to_value(&one)))
            .map_err(ForgeError::Yaml)?;
        document(table_name, format!("{body}\n"));
    }
    if !model.scalars.is_empty() {
        let body = yaml_serde::to_string(&crate::core::precision::Rounded(&scalars_to_value(
            &model.scalars,
        )))
        .map_err(ForgeError::Yaml)?;
        document("scalars", body);
    }
    let dir = output.parent().unwrap_or_else(|| Path::new("."));
    let written = write_include_files(dir, model)?;
    fs::write(output, yaml_output).map_err(ForgeError::Io)?;
    Ok(written)
}

#[cfg(feature = "cli")]
fn write_multi_doc(
    c: &mut Console<'_>,
    output: &Path,
    model: &crate::types::ParsedModel,
) -> ForgeResult<()> {
    let included = write_multi_doc_quiet(output, model)?;
    outln!(c, "{}", "✅ Import Complete!".bold().green());
    outln!(c, "   YAML file: {}", output.display());
    print_included_files(c, &included);
    outln!(c);
    Ok(())
}

/// Say which included files an import wrote beside the model: they hold the
/// values of the sheets that belong to an included file.
#[cfg(feature = "cli")]
fn print_included_files(c: &mut Console<'_>, written: &[String]) {
    if !written.is_empty() {
        outln!(c, "   Included files: {}", written.join(", "));
    }
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
mod tests {

    mod included_files {
        //! `forge import` says which included file it wrote (the end-to-end pass of
        //! 2026-11-12).

        use crate::cli::forge;

        #[test]
        fn an_import_says_which_included_file_it_wrote() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = dir.path().join("bagel.xlsx");
            let model = std::path::Path::new("examples/bagel/bagel.yaml");
            // The model reads examples/bagel/costs.yaml, so export from there
            let exported =
                forge(&["export", model.to_str().unwrap(), xlsx.to_str().unwrap()]).printed();
            assert!(exported.contains("Export Complete"), "{exported}");
            let back = dir.path().join("back");
            std::fs::create_dir(&back).unwrap();
            let text = forge(&[
                "import",
                xlsx.to_str().unwrap(),
                back.join("model.yaml").to_str().unwrap(),
            ])
            .printed();
            assert!(text.contains("YAML file:"), "{text}");
            assert!(text.contains("Included files: costs.yaml"), "{text}");
            assert!(back.join("costs.yaml").exists());
            // Importing again finds the same values there and writes nothing more
            let again = forge(&[
                "import",
                xlsx.to_str().unwrap(),
                back.join("model.yaml").to_str().unwrap(),
            ])
            .printed();
            assert!(!again.contains("Included files"), "{again}");
        }
    }
}
