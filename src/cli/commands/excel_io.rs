//! Excel export and import commands

use crate::error::{ForgeError, ForgeResult};
use crate::excel::{ExcelExporter, ExcelImporter};
use crate::parser;
use colored::Colorize;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// Export YAML to Excel and return structured results (no printing).
///
/// # Errors
///
/// Returns an error if the YAML file cannot be parsed or the Excel export fails.
pub fn export_core(input: &Path, output: &Path) -> ForgeResult<super::results::ExportResult> {
    let model = parser::parse_model(input)?;
    let table_count = model.tables.len();
    let scalar_count = model.scalars.len();

    let exporter = ExcelExporter::new(model);
    exporter.export(output)?;

    Ok(super::results::ExportResult {
        input_path: input.display().to_string(),
        output_path: output.display().to_string(),
        table_count,
        scalar_count,
    })
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
    let model = importer.import()?;
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
        let yaml_string = serde_yaml_ng::to_string(&model).map_err(ForgeError::Yaml)?;
        fs::write(output, yaml_string).map_err(ForgeError::Io)?;
    }

    Ok(super::results::ImportResult {
        input_path: input.display().to_string(),
        output_path: output.display().to_string(),
        table_count,
        scalar_count,
        mode: mode.to_string(),
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
            serde_yaml_ng::to_string(&table_model.tables).map_err(ForgeError::Yaml)?
        );
        fs::write(&file_path, yaml_string).map_err(ForgeError::Io)?;
    }
    if !model.scalars.is_empty() {
        let file_path = output.join("scalars.yaml");
        let mut scalar_model = crate::types::ParsedModel::new();
        scalar_model.scalars = model.scalars;
        let yaml_string = format!(
            "_forge_version: \"1.0.0\"\n_name: \"scalars\"\n\n{}",
            serde_yaml_ng::to_string(&scalar_model.scalars).map_err(ForgeError::Yaml)?
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

    let model = parser::parse_model(input)?;

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
    exporter.export(output)?;

    println!("{}", "✅ Export Complete!".bold().green());
    println!("   Excel file: {}\n", output.display());
    println!("{}", "✅ Phase 3: Excel Export Complete!".bold().green());
    println!("   ✅ Table columns → Excel columns");
    println!("   ✅ Data values exported");
    println!("   ✅ Multiple worksheets");
    println!("   ✅ Scalars worksheet");
    println!("   ✅ Row formulas → Excel cell formulas (=A2-B2)");
    println!("   ✅ Cross-table references (=Sheet!Column)");
    println!("   ✅ Supports 60+ Excel functions (IFERROR, SUMIF, VLOOKUP, etc.)\n");

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
    let model = importer.import()?;

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
        let yaml_string = serde_yaml_ng::to_string(&model).map_err(ForgeError::Yaml)?;
        fs::write(output, yaml_string).map_err(ForgeError::Io)?;
        println!("{}", "✅ Import Complete!".bold().green());
        println!("   YAML file: {}\n", output.display());
    }

    print_import_summary(split_files, multi_doc);
    Ok(())
}

fn print_import_summary(split_files: bool, multi_doc: bool) {
    println!("{}", "✅ Phase 4: Excel Import Complete!".bold().green());
    println!("   ✅ Excel worksheets → YAML tables");
    println!("   ✅ Data values imported");
    if split_files {
        println!("   ✅ Multiple worksheets → Separate YAML files (v4.4.2)");
    } else if multi_doc {
        println!("   ✅ Multiple worksheets → Multi-document YAML (v4.4.2)");
    } else {
        println!("   ✅ Multiple worksheets → One YAML file");
    }
    println!("   ✅ Scalars sheet detected");
    println!("   ✅ Formula translation (Excel → YAML syntax)");
    println!("   ✅ Supports 60+ Excel functions (IFERROR, SUMIF, VLOOKUP, etc.)\n");
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
            serde_yaml_ng::to_string(&table_model.tables).map_err(ForgeError::Yaml)?
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
            serde_yaml_ng::to_string(&scalar_model.scalars).map_err(ForgeError::Yaml)?
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
        yaml_output
            .push_str(&serde_yaml_ng::to_string(&table_model.tables).map_err(ForgeError::Yaml)?);
        yaml_output.push('\n');
    }
    if !model.scalars.is_empty() {
        let mut scalar_model = crate::types::ParsedModel::new();
        scalar_model.scalars = model.scalars;
        yaml_output.push_str("---\n");
        yaml_output.push_str("_forge_version: \"1.0.0\"\n");
        yaml_output.push_str("_name: \"scalars\"\n\n");
        yaml_output
            .push_str(&serde_yaml_ng::to_string(&scalar_model.scalars).map_err(ForgeError::Yaml)?);
    }
    fs::write(output, yaml_output).map_err(ForgeError::Io)?;
    println!("{}", "✅ Import Complete!".bold().green());
    println!("   YAML file: {}\n", output.display());
    Ok(())
}
