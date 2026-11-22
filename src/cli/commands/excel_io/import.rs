//! `forge import`: a workbook back as a model, in one file, split files or
//! one file of several documents.

use crate::error::{ForgeError, ForgeResult};
use crate::excel::ExcelImporter;
use crate::parser;
use num_traits::ToPrimitive;
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
    check_names(&model)?;
    let includes = model
        .includes
        .iter()
        .map(|include| {
            // The importer adds an include and its model together
            let resolved = &model.resolved_includes[&include.namespace];
            (include.file.clone(), model_to_yaml(&resolved.model))
        })
        .collect();
    Ok(ImportText {
        main: model_to_yaml(&model),
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
    match n.to_i64() {
        Some(whole) if n.fract() == 0.0 && n.abs() < 1e15 => {
            yaml_serde::Value::Number(whole.into())
        },
        _ => yaml_serde::Value::Number(n.into()),
    }
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
        let described = variable.metadata.unit.is_some() || variable.metadata.notes.is_some();
        let entry = match (&variable.formula, variable.value) {
            // An input with a format:, unit: or notes: is a mapping (ADR-061)
            (None, Some(value)) if format.is_some() || described => {
                let mut entry = Mapping::new();
                let value = variable
                    .date_text()
                    .map_or_else(|| number(value), Value::String);
                entry.insert(Value::from("value"), value);
                if let Some(format) = format {
                    entry.insert(Value::from("format"), Value::from(format.clone()));
                }
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
        // A unit and notes from a sheet of labels (`Unit`, `Notes` columns)
        let entry = match entry {
            Value::Mapping(mut long) => {
                let metadata = &variable.metadata;
                for (key, text) in [("unit", &metadata.unit), ("notes", &metadata.notes)] {
                    if let Some(text) = text {
                        long.insert(Value::from(key), Value::from(text.clone()));
                    }
                }
                Value::Mapping(long)
            },
            plain => plain,
        };
        match path.split_once('.') {
            Some((group, name)) => {
                let group_entry = map
                    .entry(Value::String(group.to_string()))
                    .or_insert_with(|| Value::Mapping(Mapping::new()));
                // `check_names` has refused a group that is also a scalar
                let _ = group_entry
                    .as_mapping_mut()
                    .map(|group_map| group_map.insert(Value::String(name.to_string()), entry));
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
        files.push((path, model_to_yaml(&resolved.model)));
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
    let yaml = model_to_yaml(model);
    let written = write_include_files(&output.with_file_name(""), model)?;
    fs::write(output, yaml).map_err(ForgeError::Io)?;
    Ok(written)
}

/// Write each file, and give the paths written.
fn write_files(files: Vec<(std::path::PathBuf, String)>) -> ForgeResult<Vec<std::path::PathBuf>> {
    files
        .into_iter()
        .map(|(path, text)| {
            fs::write(&path, text)
                .map(|()| path)
                .map_err(ForgeError::Io)
        })
        .collect()
}

/// Write the included files the workbook holds, beside the model, and give
/// their names. A file that is already there with the same values is not
/// written again, and is not listed.
fn write_include_files(dir: &Path, model: &crate::types::ParsedModel) -> ForgeResult<Vec<String>> {
    Ok(write_files(include_files(dir, model)?)?
        .iter()
        .map(|path| path.strip_prefix(dir).unwrap_or(path).display().to_string())
        .collect())
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

/// A model's YAML text: its values, rounded to 15 digits. Values read from a
/// workbook always serialize.
fn yaml_of<T: serde::Serialize>(value: &T) -> String {
    yaml_serde::to_string(&crate::core::precision::Rounded(value)).expect("model values serialize")
}

/// Refuse a model, or an included file's model, in which a name is both a table
/// and a scalar group, or both a scalar and a scalar group: one would overwrite
/// the other.
fn check_names(model: &crate::types::ParsedModel) -> ForgeResult<()> {
    let included = model.resolved_includes.values().map(|r| &r.model);
    for each in std::iter::once(model).chain(included) {
        let tables = tables_to_value(&each.tables);
        let groups = scalars_to_value(&each.scalars);
        if let Some(key) = groups.keys().find(|key| tables.contains_key(*key)) {
            return Err(ForgeError::Import(format!(
                "'{}' is both a table and a scalar group; rename one sheet",
                key.as_str().unwrap_or_default()
            )));
        }
        if let Some(name) = each
            .scalars
            .keys()
            .filter_map(|path| path.split_once('.'))
            .map(|(group, _)| group)
            .find(|group| each.scalars.contains_key(*group))
        {
            return Err(ForgeError::Import(format!(
                "'{name}' is both a scalar and a scalar group; rename one"
            )));
        }
    }
    Ok(())
}

/// The whole imported model as YAML that `forge validate` and `forge calculate` accept.
fn model_to_yaml(model: &crate::types::ParsedModel) -> String {
    let includes = includes_block(model, None);
    let mut map = tables_to_value(&model.tables);
    map.extend(scalars_to_value(&model.scalars));
    if map.is_empty() {
        return format!("_forge_version: \"5.1.0\"\n{includes}");
    }
    let body = yaml_of(&map);
    let version = import_format(model);
    format!("_forge_version: \"{version}\"\n{includes}\n{body}")
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
    check_names(&model)?;
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
        let body = yaml_of(&tables_to_value(&one));
        parts.push((format!("{table_name}.yaml"), table_name.clone(), body));
    }
    if !model.scalars.is_empty() {
        let body = yaml_of(&scalars_to_value(&model.scalars));
        parts.push(("scalars.yaml".to_string(), "scalars".to_string(), body));
    }
    fs::create_dir_all(output).map_err(ForgeError::Io)?;
    let includes = include_files(output, model)?;
    let version = import_format(model);
    let mut files = Vec::new();
    for (file, name, body) in parts {
        let includes = includes_block(model, Some(&body));
        let text = format!("_forge_version: \"{version}\"\n_name: \"{name}\"\n{includes}\n{body}");
        files.push((output.join(file), text));
    }
    files.extend(includes);
    write_files(files)
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
    check_names(&model)?;

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
        let body = yaml_of(&tables_to_value(&one));
        document(table_name, format!("{body}\n"));
    }
    if !model.scalars.is_empty() {
        document("scalars", yaml_of(&scalars_to_value(&model.scalars)));
    }
    let written = write_include_files(&output.with_file_name(""), model)?;
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

        let yaml = model_to_yaml(&model);
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
    use super::number;

    #[test]
    fn a_whole_number_below_1e15_is_written_as_an_integer() {
        assert_eq!(number(16.0), yaml_serde::Value::Number(16.into()));
        assert_eq!(number(-3.0), yaml_serde::Value::Number((-3).into()));
        assert_eq!(
            number(999_999_999_999_999.0),
            yaml_serde::Value::Number(999_999_999_999_999_i64.into())
        );
        assert_eq!(number(1e15), yaml_serde::Value::Number(1e15.into()));
        assert_eq!(number(2.5), yaml_serde::Value::Number(2.5.into()));
    }

    /// A unit beside a value, with no number format, comes back as the
    /// value's mapping (ADR-061): the unit is not lost.
    #[test]
    fn an_input_with_a_unit_and_no_format_is_a_mapping() {
        let mut book = rust_xlsxwriter::Workbook::new();
        let sheet = book.add_worksheet().set_name("Scalars").unwrap();
        for (col, header) in (0..).zip(["Name", "Value", "Unit"]) {
            sheet.write_string(0, col, header).unwrap();
        }
        sheet.write_string(1, 0, "price").unwrap();
        sheet.write_number(1, 1, 10.0).unwrap();
        sheet.write_string(1, 2, "$").unwrap();
        let imported = super::import_text_core(book.save_to_buffer().unwrap()).unwrap();
        assert_eq!(
            imported.main,
            "_forge_version: \"5.1.0\"\n\nprice:\n  value: 10\n  unit: $\n"
        );
        assert_eq!(imported.warnings, Vec::<String>::new());
    }

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

    mod commands {
        //! What `forge import` prints and writes in each of its modes, from a
        //! workbook `forge export` made of the bagel (a table, scalars and an
        //! included file).

        use crate::cli::forge;
        use std::fs;
        use std::path::{Path, PathBuf};

        fn bagel_workbook(dir: &Path) -> PathBuf {
            let xlsx = dir.join("bagel.xlsx");
            let exported = forge(&[
                "export",
                "examples/bagel/bagel.yaml",
                xlsx.to_str().unwrap(),
            ]);
            assert!(exported.success(), "{}", exported.err);
            xlsx
        }

        #[test]
        fn a_verbose_import_lists_the_sheets_it_read() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = bagel_workbook(dir.path());
            let out = dir.path().join("m.yaml");
            let text = forge(&[
                "import",
                xlsx.to_str().unwrap(),
                out.to_str().unwrap(),
                "--verbose",
            ])
            .printed();
            for line in [
                "Reading Excel file",
                "Found",
                "tables",
                "scalars",
                "Table:",
                "columns,",
                "Writing YAML",
            ] {
                assert!(text.contains(line), "{line} in {text}");
            }
            assert!(out.exists());
        }

        #[test]
        fn split_files_writes_one_yaml_per_sheet() {
            let dir = tempfile::tempdir().unwrap();
            let model = dir.path().join("two.yaml");
            fs::write(
                &model,
                "_forge_version: \"5.1.0\"\na:\n  n: [1, 2]\n  d: \"=n * 2\"\nb:\n  m: [3, 4]\n",
            )
            .unwrap();
            let xlsx = dir.path().join("two.xlsx");
            let exported = forge(&["export", model.to_str().unwrap(), xlsx.to_str().unwrap()]);
            assert!(exported.success(), "{}", exported.err);
            let out = dir.path().join("split");
            fs::create_dir(&out).unwrap();
            let args = [
                "import",
                xlsx.to_str().unwrap(),
                out.to_str().unwrap(),
                "--split-files",
            ];
            let text = forge(&args).printed();
            assert!(text.contains("Split files"), "{text}");
            assert!(text.contains("Import Complete"), "{text}");
            assert!(!text.contains("Created:"), "{text}");
            let verbose = forge(&[args[0], args[1], args[2], args[3], "--verbose"]).printed();
            assert!(verbose.contains("Created:"), "{verbose}");
            assert!(out.join("a.yaml").exists() && out.join("b.yaml").exists());
        }

        #[test]
        fn multi_doc_writes_one_file_of_several_documents() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = bagel_workbook(dir.path());
            let out = dir.path().join("all.yaml");
            let text = forge(&[
                "import",
                xlsx.to_str().unwrap(),
                out.to_str().unwrap(),
                "--multi-doc",
            ])
            .printed();
            assert!(text.contains("Multi-document YAML"), "{text}");
            assert!(text.contains("YAML file:"), "{text}");
            assert!(fs::read_to_string(&out).unwrap().contains("---"));
        }

        #[test]
        fn an_import_that_cannot_read_or_write_is_an_error() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = bagel_workbook(dir.path());
            let junk = dir.path().join("junk.xlsx");
            fs::write(&junk, "not a workbook").unwrap();
            let out = dir.path().join("m.yaml");
            let ran = forge(&["import", junk.to_str().unwrap(), out.to_str().unwrap()]);
            assert!(!ran.success());
            let elsewhere = dir.path().join("absent").join("m.yaml");
            for flags in [&[][..], &["--multi-doc"][..], &["--split-files"][..]] {
                let mut args = vec![
                    "import",
                    xlsx.to_str().unwrap(),
                    elsewhere.to_str().unwrap(),
                ];
                args.extend_from_slice(flags);
                let ran = forge(&args);
                assert!(!ran.success(), "{flags:?}");
            }
        }
    }

    mod failures {
        //! What `forge import` does with a workbook it cannot carry, and with
        //! places it cannot write.

        use super::super::{import_core, import_text_core};
        use crate::cli::forge;
        use rust_xlsxwriter::Workbook;
        use std::fs;
        use std::path::{Path, PathBuf};

        /// A workbook with a table sheet `a` and a scalar of the group `a`
        fn clashing_workbook(dir: &Path) -> PathBuf {
            let mut book = Workbook::new();
            let table = book.add_worksheet().set_name("a").unwrap();
            table.write_string(0, 0, "n").unwrap();
            table.write_number(1, 0, 1.0).unwrap();
            let scalars = book.add_worksheet().set_name("Scalars").unwrap();
            scalars.write_string(0, 0, "Name").unwrap();
            scalars.write_string(0, 1, "Value").unwrap();
            scalars.write_string(1, 0, "a.x").unwrap();
            scalars.write_number(1, 1, 5.0).unwrap();
            let path = dir.join("clash.xlsx");
            book.save(&path).unwrap();
            path
        }

        #[test]
        fn a_name_that_is_a_table_and_a_scalar_group_is_refused_in_every_form() {
            let dir = tempfile::tempdir().unwrap();
            let xlsx = clashing_workbook(dir.path());
            let out = dir.path().join("m.yaml");
            let message = import_core(&xlsx, &out, false, false)
                .err()
                .unwrap()
                .to_string();
            assert!(
                message.contains("'a' is both a table and a scalar group"),
                "{message}"
            );
            assert!(import_text_core(fs::read(&xlsx).unwrap()).is_err());
            let ran = forge(&["import", xlsx.to_str().unwrap(), out.to_str().unwrap()]);
            assert!(!ran.success());
            assert!(
                ran.err.contains("both a table and a scalar group"),
                "{}",
                ran.err
            );
            assert!(!out.exists());
        }

        #[test]
        fn a_scalar_that_is_also_a_scalar_group_is_refused() {
            let dir = tempfile::tempdir().unwrap();
            let mut book = Workbook::new();
            let scalars = book.add_worksheet().set_name("Scalars").unwrap();
            scalars.write_string(0, 0, "Name").unwrap();
            scalars.write_string(0, 1, "Value").unwrap();
            for (row, (name, value)) in [("a", 1.0), ("a.x", 2.0)].into_iter().enumerate() {
                let row = u32::try_from(row).unwrap() + 1;
                scalars.write_string(row, 0, name).unwrap();
                scalars.write_number(row, 1, value).unwrap();
            }
            let xlsx = dir.path().join("scalar.xlsx");
            book.save(&xlsx).unwrap();
            let message = import_core(&xlsx, &dir.path().join("m.yaml"), false, false)
                .err()
                .unwrap()
                .to_string();
            assert!(
                message.contains("'a' is both a scalar and a scalar group"),
                "{message}"
            );
        }

        #[test]
        fn a_workbook_that_is_not_one_is_an_error_for_the_library_too() {
            let dir = tempfile::tempdir().unwrap();
            let junk = dir.path().join("junk.xlsx");
            fs::write(&junk, "not a workbook").unwrap();
            let out = dir.path().join("m.yaml");
            for (split, multi) in [(false, false), (true, false), (false, true)] {
                assert!(import_core(&junk, &out, split, multi).is_err());
            }
        }

        #[test]
        fn a_place_that_cannot_be_written_is_an_error_in_every_mode() {
            let dir = tempfile::tempdir().unwrap();
            let model = dir.path().join("two.yaml");
            fs::write(
                &model,
                "_forge_version: \"5.1.0\"\na:\n  n: [1, 2]\nb:\n  m: [3, 4]\n",
            )
            .unwrap();
            let xlsx = dir.path().join("two.xlsx");
            assert!(forge(&["export", model.to_str().unwrap(), xlsx.to_str().unwrap()]).success());

            // The output is a folder where a file should go
            let folder = dir.path().join("folder.yaml");
            fs::create_dir(&folder).unwrap();
            assert!(import_core(&xlsx, &folder, false, false).is_err());
            assert!(import_core(&xlsx, &folder, false, true).is_err());
            // The output is a file where a folder should go
            let file = dir.path().join("file");
            fs::write(&file, "x").unwrap();
            assert!(import_core(&xlsx, &file, true, false).is_err());
            // A part of a split import cannot be written
            let split = dir.path().join("split");
            fs::create_dir_all(split.join("a.yaml")).unwrap();
            assert!(import_core(&xlsx, &split, true, false).is_err());
        }

        #[test]
        fn a_split_that_would_break_one_formula_says_so_in_the_singular() {
            let dir = tempfile::tempdir().unwrap();
            let model = dir.path().join("m.yaml");
            fs::write(
                &model,
                "_forge_version: \"5.1.0\"\na:\n  n: [1, 2]\ntotal: {formula: \"=SUM(a.n)\"}\n",
            )
            .unwrap();
            let xlsx = dir.path().join("m.xlsx");
            assert!(forge(&["export", model.to_str().unwrap(), xlsx.to_str().unwrap()]).success());
            let err = import_core(&xlsx, &dir.path().join("out"), true, false)
                .err()
                .unwrap()
                .to_string();
            assert!(err.contains("would break 1 formula,"), "{err}");
            assert!(!err.contains("; ..."), "{err}");
        }
    }

    mod yaml_forms {
        //! How each kind of value is written, and which format version a model needs.

        use super::super::{
            holds_the_same_values, import_format, include_files, includes_block, scalars_to_value,
            table_to_value, write_split_files_quiet,
        };
        use crate::types::{
            Column, ColumnValue, Include, ParsedModel, ResolvedInclude, Table, Variable,
        };
        use std::fs;
        use std::path::Path;

        fn parsed(dir: &Path, name: &str, text: &str) -> ParsedModel {
            let path = dir.join(name);
            fs::write(&path, text).unwrap();
            crate::parser::parse_model(&path).unwrap()
        }

        fn yaml(value: &impl serde::Serialize) -> String {
            yaml_serde::to_string(value).unwrap()
        }

        #[test]
        fn a_boolean_column_and_a_column_with_empty_cells_are_written_as_they_are() {
            let mut table = Table::new("t".to_string());
            table.add_column(Column::new(
                "flag".to_string(),
                ColumnValue::Boolean(vec![true, false]),
            ));
            table.add_column(Column::new(
                "gappy".to_string(),
                ColumnValue::Blanks(vec![Some(1.5), None]),
            ));
            let text = yaml(&table_to_value(&table));
            assert!(text.contains("flag:\n- true\n- false"), "{text}");
            assert!(text.contains("gappy:\n- 1.5\n- null"), "{text}");
            let mut model = ParsedModel::new();
            model.add_table(table);
            assert_eq!(import_format(&model), "5.3.0");
        }

        #[test]
        fn scalars_are_written_by_what_they_hold() {
            let mut model = ParsedModel::new();
            let mut dated = Variable::new("when".to_string(), Some(46_023.0), None);
            dated.is_date = true;
            let mut formatted = Variable::new("share".to_string(), Some(0.5), None);
            formatted.metadata.format = Some("0.0%".to_string());
            let mut formula = Variable::new("g.f".to_string(), None, Some("=1 + 1".to_string()));
            formula.metadata.format = Some("0.00".to_string());
            for variable in [
                dated,
                formatted,
                formula,
                Variable::new("empty".to_string(), None, None),
            ] {
                model.add_scalar(variable.path.clone(), variable);
            }
            let text = yaml(&scalars_to_value(&model.scalars));
            assert!(text.contains("when: 2026-01-01"), "{text}");
            assert!(
                text.contains("share:\n  value: 0.5\n  format: 0.0%"),
                "{text}"
            );
            assert!(
                text.contains("g:\n  f:\n    formula: =1 + 1\n    format: '0.00'"),
                "{text}"
            );
            assert!(text.contains("empty:\n  value: null"), "{text}");
            assert_eq!(import_format(&model), "5.4.0");
        }

        #[test]
        fn only_the_includes_a_part_reads_are_listed() {
            let mut model = ParsedModel::new();
            model.includes = vec![
                Include::new("a.yaml".to_string(), "a".to_string()),
                Include::new("b.yaml".to_string(), "b".to_string()),
            ];
            let all = includes_block(&model, None);
            assert!(
                all.contains("file: a.yaml") && all.contains("file: b.yaml"),
                "{all}"
            );
            let some = includes_block(&model, Some("=@b.total * 2"));
            assert!(
                !some.contains("a.yaml") && some.contains("file: b.yaml"),
                "{some}"
            );
            assert_eq!(includes_block(&model, Some("=1")), "");
        }

        #[test]
        fn an_include_with_no_sheets_is_skipped_and_one_with_other_values_is_refused() {
            let dir = tempfile::tempdir().unwrap();
            let mut model = ParsedModel::new();
            model.includes = vec![
                Include::new("gone.yaml".to_string(), "gone".to_string()),
                Include::new("kept.yaml".to_string(), "kept".to_string()),
            ];
            let kept = parsed(dir.path(), "kept.yaml", "_forge_version: \"5.1.0\"\nx: 2\n");
            let resolved = ResolvedInclude {
                include: model.includes[1].clone(),
                resolved_path: dir.path().join("kept.yaml"),
                model: parsed(
                    dir.path(),
                    "sheet.yaml",
                    "_forge_version: \"5.1.0\"\nx: 3\n",
                ),
            };
            model.resolved_includes.insert("kept".to_string(), resolved);
            assert!(kept.scalars.contains_key("x"));
            let err = include_files(dir.path(), &model).err().unwrap().to_string();
            assert!(err.contains("hold other values than"), "{err}");
            // The same error stops a split import before it writes anything
            let split = write_split_files_quiet(dir.path(), &model).err().unwrap();
            assert!(split.to_string().contains("hold other values"), "{split}");
        }

        #[test]
        fn a_file_holds_the_same_values_only_when_everything_agrees() {
            let dir = tempfile::tempdir().unwrap();
            let sheet = parsed(
                dir.path(),
                "sheet.yaml",
                "_forge_version: \"5.1.0\"\nx: 2\nempty: {value: null, formula: null}\nt:\n  n: [1, 2]\n  label: [\"a\", \"b\"]\n  d: \"=n * 2\"\n",
            );
            let same = |text: &str| {
                let path = dir.path().join("existing.yaml");
                fs::write(&path, text).unwrap();
                holds_the_same_values(&path, &sheet)
            };
            let base = "_forge_version: \"5.1.0\"\nx: 2\nempty: {value: null, formula: null}\nt:\n  n: [1, 2]\n  label: [\"a\", \"b\"]\n  d: \"=n * 2\"\n";
            assert!(same(base));
            assert!(!same(&base.replace("x: 2", "x: 3")));
            assert!(!same(
                &base.replace("empty: {value: null, formula: null}", "empty: 1")
            ));
            assert!(!same(&base.replace("[\"a\", \"b\"]", "[\"a\", \"c\"]")));
            assert!(!same(&base.replace("[1, 2]", "[1, 3]")));
            assert!(!same(&base.replace("d: \"=n * 2\"", "d: [2, 4]")));
            assert!(!same("_forge_version: \"5.1.0\"\nx: 2\n"));
            // A file that does not parse, or does not calculate
            assert!(!same("a: [1, 2\n"));
            assert!(!same(
                "_forge_version: \"5.1.0\"\ny: {formula: \"=nosuch + 1\"}\n"
            ));
            // A sheet that does not calculate
            let broken = parsed(
                dir.path(),
                "broken.yaml",
                "_forge_version: \"5.1.0\"\ny: {formula: \"=nosuch + 1\"}\n",
            );
            assert!(!holds_the_same_values(
                &dir.path().join("sheet.yaml"),
                &broken
            ));
        }
    }

    mod text {
        //! `import_text_core`: the same import with no files.

        use super::super::import_text_core;
        use crate::excel::ExcelExporter;

        #[test]
        fn a_workbook_comes_back_as_text_with_its_included_files() {
            let model =
                crate::parser::parse_model(std::path::Path::new("examples/bagel/bagel.yaml"))
                    .unwrap();
            let model = crate::core::ArrayCalculator::new(model)
                .calculate_all()
                .unwrap();
            let dir = tempfile::tempdir().unwrap();
            let xlsx = dir.path().join("b.xlsx");
            ExcelExporter::new(model).export(&xlsx).unwrap();
            let imported = import_text_core(std::fs::read(&xlsx).unwrap()).unwrap();
            assert!(
                imported.main.starts_with("_forge_version:"),
                "{}",
                imported.main
            );
            assert_eq!(imported.includes.len(), 1);
            assert_eq!(imported.includes[0].0, "costs.yaml");
            assert!(imported.table_count > 0 && imported.scalar_count > 0);
            assert!(import_text_core(b"not a workbook".to_vec()).is_err());
        }
    }
}
