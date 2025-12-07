//! Excel export and import commands

use crate::error::{ForgeError, ForgeResult};
use crate::excel::{ExcelExporter, ExcelImporter};
use crate::parser;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

/// Execute the export command
pub fn export(input: PathBuf, output: PathBuf, verbose: bool) -> ForgeResult<()> {
    println!("{}", "🔥 Forge - Excel Export".bold().green());
    println!("   Input:  {}", input.display());
    println!("   Output: {}\n", output.display());

    // Parse the YAML file
    if verbose {
        println!("{}", "📖 Parsing YAML file...".cyan());
    }

    let model = parser::parse_model(&input)?;

    if verbose {
        println!(
            "   Found {} tables, {} scalars\n",
            model.tables.len(),
            model.scalars.len()
        );
    }

    // Export to Excel
    if verbose {
        println!("{}", "📊 Exporting to Excel...".cyan());
    }

    let exporter = ExcelExporter::new(model);
    exporter.export(&output)?;

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

/// Execute the import command
pub fn import(
    input: PathBuf,
    output: PathBuf,
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

    // Import Excel file
    if verbose {
        println!("{}", "📖 Reading Excel file...".cyan());
    }

    let importer = ExcelImporter::new(&input);
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
    }

    // Write YAML file(s) based on mode
    if verbose {
        println!("{}", "💾 Writing YAML file(s)...".cyan());
    }

    if split_files {
        // Create output directory if it doesn't exist
        fs::create_dir_all(&output).map_err(ForgeError::Io)?;

        // Write each table to a separate file
        for (table_name, table) in &model.tables {
            let mut table_model = crate::types::ParsedModel::new();
            table_model.tables.insert(table_name.clone(), table.clone());

            let file_path = output.join(format!("{}.yaml", table_name));
            let yaml_string = format!(
                "_forge_version: \"1.0.0\"\n_name: \"{}\"\n\n{}",
                table_name,
                serde_yaml::to_string(&table_model.tables).map_err(ForgeError::Yaml)?
            );
            fs::write(&file_path, yaml_string).map_err(ForgeError::Io)?;

            if verbose {
                println!("   Created: {}", file_path.display());
            }
        }

        // Write scalars to separate file if present
        if !model.scalars.is_empty() {
            let file_path = output.join("scalars.yaml");
            let mut scalar_model = crate::types::ParsedModel::new();
            scalar_model.scalars = model.scalars.clone();

            let yaml_string = format!(
                "_forge_version: \"1.0.0\"\n_name: \"scalars\"\n\n{}",
                serde_yaml::to_string(&scalar_model.scalars).map_err(ForgeError::Yaml)?
            );
            fs::write(&file_path, yaml_string).map_err(ForgeError::Io)?;

            if verbose {
                println!("   Created: {}", file_path.display());
            }
        }

        println!("{}", "✅ Import Complete!".bold().green());
        println!("   Output directory: {}\n", output.display());
    } else if multi_doc {
        // Write as multi-document YAML
        let mut yaml_output = String::new();

        for (table_name, table) in &model.tables {
            let mut table_model = crate::types::ParsedModel::new();
            table_model.tables.insert(table_name.clone(), table.clone());

            yaml_output.push_str("---\n");
            yaml_output.push_str("_forge_version: \"1.0.0\"\n");
            yaml_output.push_str(&format!("_name: \"{}\"\n\n", table_name));
            yaml_output
                .push_str(&serde_yaml::to_string(&table_model.tables).map_err(ForgeError::Yaml)?);
            yaml_output.push('\n');
        }

        // Add scalars as separate document if present
        if !model.scalars.is_empty() {
            let mut scalar_model = crate::types::ParsedModel::new();
            scalar_model.scalars = model.scalars.clone();

            yaml_output.push_str("---\n");
            yaml_output.push_str("_forge_version: \"1.0.0\"\n");
            yaml_output.push_str("_name: \"scalars\"\n\n");
            yaml_output
                .push_str(&serde_yaml::to_string(&scalar_model.scalars).map_err(ForgeError::Yaml)?);
        }

        fs::write(&output, yaml_output).map_err(ForgeError::Io)?;

        println!("{}", "✅ Import Complete!".bold().green());
        println!("   YAML file: {}\n", output.display());
    } else {
        // Default: single file with all tables
        let yaml_string = serde_yaml::to_string(&model).map_err(ForgeError::Yaml)?;
        fs::write(&output, yaml_string).map_err(ForgeError::Io)?;

        println!("{}", "✅ Import Complete!".bold().green());
        println!("   YAML file: {}\n", output.display());
    }

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

    Ok(())
}
