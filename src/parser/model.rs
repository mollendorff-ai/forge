//! Core model parsing for Forge YAML files
//!
//! Handles parsing of the main model structure including tables, scalars, and scenarios.

use crate::error::{ForgeError, ForgeResult};
use crate::types::{ParsedModel, Scenario};
use serde_yaml_ng::Value;

use super::includes::parse_includes;
use super::schema::validate_against_schema;
use super::variables::{is_nested_scalar_section, parse_scalar_variable, parse_table};

/// Parse v1.0.0 array model
///
/// # Errors
///
/// Returns an error if the YAML fails schema validation or contains invalid table/scalar definitions.
pub fn parse_v1_model(yaml: &Value) -> ForgeResult<ParsedModel> {
    // Validate against JSON Schema - this is mandatory
    validate_against_schema(yaml)?;

    let mut model = ParsedModel::new();

    // Parse each top-level key as either a table or scalar
    if let Value::Mapping(map) = yaml {
        for (key, value) in map {
            let key_str = key
                .as_str()
                .ok_or_else(|| ForgeError::Parse("Table name must be a string".to_string()))?;

            // Skip special keys (handled by specific commands)
            // Note: scenarios is NOT skipped here - it has special handling below
            // to distinguish scenario overrides from tables named "scenarios"
            if key_str == "_forge_version"
                || key_str == "_name"
                || key_str == "monte_carlo"
                || key_str == "tornado"
                || key_str == "decision_tree"
            {
                continue;
            }

            // Parse _includes section (v4.0 cross-file references)
            if key_str == "_includes" {
                if let Value::Sequence(includes_seq) = value {
                    parse_includes(includes_seq, &mut model)?;
                }
                continue;
            }

            // Parse scenarios section - but only if it looks like scenario overrides
            // (mapping of scenario_name -> {variable: value}), not a table (mapping of column_name -> array)
            if key_str == "scenarios" {
                if let Value::Mapping(scenarios_map) = value {
                    // Check if this is actually a scenarios section or a table named "scenarios"
                    // Scenarios section has nested mappings with numeric values
                    // Tables have arrays (sequences) as column values
                    let is_scenarios_section = scenarios_map
                        .iter()
                        .all(|(_, v)| matches!(v, Value::Mapping(_)))
                        && scenarios_map.iter().any(|(_, v)| {
                            if let Value::Mapping(m) = v {
                                m.iter().any(|(_, vv)| matches!(vv, Value::Number(_)))
                            } else {
                                false
                            }
                        });

                    if is_scenarios_section {
                        parse_scenarios(scenarios_map, &mut model)?;
                        continue;
                    }
                    // Otherwise fall through to parse as table
                }
            }

            // Check if this is a table (mapping with arrays) or scalar (mapping with value/formula)
            if let Value::Mapping(inner_map) = value {
                // Check if it has {value, formula} pattern (scalar)
                if inner_map.contains_key("value") || inner_map.contains_key("formula") {
                    // This is a scalar variable
                    let variable = parse_scalar_variable(value, key_str)?;
                    model.add_scalar(key_str.to_string(), variable);
                } else if is_nested_scalar_section(inner_map) {
                    // This is a section containing nested scalars (e.g., summary.total)
                    parse_nested_scalars(key_str, inner_map, &mut model)?;
                } else {
                    // This is a table - parse it
                    let table = parse_table(key_str, inner_map)?;
                    model.add_table(table);
                }
            }
        }
    }

    // Note: Table column length validation is deferred to calculation time
    // This allows test files to have columns of different lengths when used independently
    // Row-wise operations will still validate at runtime in array_calculator

    Ok(model)
}

/// Parse nested scalar variables (e.g., summary.total, summary.average)
///
/// # Errors
///
/// Returns an error if a nested scalar name is not a string or has an invalid format.
pub fn parse_nested_scalars(
    parent_key: &str,
    map: &serde_yaml_ng::Mapping,
    model: &mut ParsedModel,
) -> ForgeResult<()> {
    for (key, value) in map {
        let key_str = key
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Scalar name must be a string".to_string()))?;

        if let Value::Mapping(child_map) = value {
            if child_map.contains_key("value") || child_map.contains_key("formula") {
                // This is a scalar variable
                let full_path = format!("{parent_key}.{key_str}");
                let variable = parse_scalar_variable(value, &full_path)?;
                model.add_scalar(full_path.clone(), variable);
            }
        }
    }
    Ok(())
}

/// Parse scenarios section from YAML
///
/// Expected format:
/// ```yaml
/// scenarios:
///   base:
///     growth_rate: 0.05
///     churn_rate: 0.02
///   optimistic:
///     growth_rate: 0.12
///     churn_rate: 0.01
/// ```
///
/// # Errors
///
/// Returns an error if a scenario name or variable is not valid, or if a variable
/// value is not a number.
pub fn parse_scenarios(
    scenarios_map: &serde_yaml_ng::Mapping,
    model: &mut ParsedModel,
) -> ForgeResult<()> {
    for (scenario_name, scenario_value) in scenarios_map {
        let name = scenario_name
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Scenario name must be a string".to_string()))?;

        if let Value::Mapping(overrides_map) = scenario_value {
            let mut scenario = Scenario::new();

            for (var_name, var_value) in overrides_map {
                let var_name_str = var_name.as_str().ok_or_else(|| {
                    ForgeError::Parse("Variable name must be a string".to_string())
                })?;

                let value = match var_value {
                    Value::Number(n) => n.as_f64().ok_or_else(|| {
                        ForgeError::Parse(format!(
                            "Scenario '{name}': Variable '{var_name_str}' must be a number"
                        ))
                    })?,
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Scenario '{name}': Variable '{var_name_str}' must be a number"
                        )));
                    },
                };

                scenario.add_override(var_name_str.to_string(), value);
            }

            model.add_scenario(name.to_string(), scenario);
        } else {
            return Err(ForgeError::Parse(format!(
                "Scenario '{name}' must be a mapping of variable overrides"
            )));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_model;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_v1_model_simple() {
        let yaml_content = r#"
_forge_version: "5.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
  profit: "=revenue * 0.2"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert!(result.tables.contains_key("sales"));

        let sales_table = result.tables.get("sales").unwrap();
        assert_eq!(sales_table.columns.len(), 2);
        assert_eq!(sales_table.row_formulas.len(), 1);
    }

    #[test]
    fn test_parse_v1_model_with_scalars() {
        let yaml_content = r#"
_forge_version: "5.0.0"

data:
  values: [1, 2, 3]

summary:
  total:
    value: null
    formula: "=SUM(data.values)"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert_eq!(result.scalars.len(), 1);
        assert!(result.scalars.contains_key("summary.total"));
    }

    #[test]
    fn test_parse_scenarios() {
        let yaml_content = r#"
_forge_version: "1.0.0"

growth_rate:
  value: 0.05
  formula: null

scenarios:
  base:
    growth_rate: 0.05
  optimistic:
    growth_rate: 0.12
  pessimistic:
    growth_rate: 0.02
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.scenarios.len(), 3);
        assert!(result.scenarios.contains_key("base"));
        assert!(result.scenarios.contains_key("optimistic"));
        assert!(result.scenarios.contains_key("pessimistic"));

        let base = result.scenarios.get("base").unwrap();
        assert_eq!(base.overrides.get("growth_rate"), Some(&0.05));

        let optimistic = result.scenarios.get("optimistic").unwrap();
        assert_eq!(optimistic.overrides.get("growth_rate"), Some(&0.12));

        let pessimistic = result.scenarios.get("pessimistic").unwrap();
        assert_eq!(pessimistic.overrides.get("growth_rate"), Some(&0.02));
    }

    #[test]
    fn test_table_named_scenarios() {
        let yaml_content = r#"
_forge_version: "5.0.0"

scenarios:
  name: ["Base", "Optimistic", "Pessimistic"]
  probability: [0.3, 0.5, 0.2]
  revenue: [100000, 150000, 80000]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.scenarios.len(), 0);
        assert_eq!(result.tables.len(), 1);

        let scenarios_table = result.tables.get("scenarios").unwrap();
        assert_eq!(scenarios_table.columns.len(), 3);
        assert!(scenarios_table.columns.contains_key("name"));
        assert!(scenarios_table.columns.contains_key("probability"));
        assert!(scenarios_table.columns.contains_key("revenue"));
        assert_eq!(scenarios_table.row_count(), 3);
    }

    #[test]
    fn test_parse_table_named_scenarios_as_table() {
        let yaml_str = r#"
_forge_version: "5.0.0"
scenarios:
  year: [2023, 2024, 2025]
  revenue: [1000, 2000, 3000]
"#;
        let yaml: Value = serde_yaml_ng::from_str(yaml_str).unwrap();
        let result = parse_v1_model(&yaml).unwrap();
        assert!(result.tables.contains_key("scenarios"));
        assert!(result.scenarios.is_empty());
    }

    #[test]
    fn test_parse_scenario_invalid_value_type() {
        let yaml_content = r#"
_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
scenarios:
  base:
    rate: "not a number"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_scenario_not_mapping() {
        let yaml_content = r#"
_forge_version: "1.0.0"
rate:
  value: 0.05
  formula: null
scenarios:
  base: "not a mapping"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_parser_skips_tornado_section() {
        let yaml_str = r#"
_forge_version: "5.0.0"

price:
  value: 100

quantity:
  value: 50

profit:
  formula: "=price * quantity"

tornado:
  output: profit
  inputs:
    - name: price
      low: 80
      high: 120
    - name: quantity
      low: 40
      high: 60
"#;
        let yaml: Value = serde_yaml_ng::from_str(yaml_str).unwrap();
        let result = parse_v1_model(&yaml).unwrap();

        // Tornado should NOT be parsed as a table
        assert!(!result.tables.contains_key("tornado"));
        // Scalars should be parsed correctly
        assert!(result.scalars.contains_key("price"));
        assert!(result.scalars.contains_key("quantity"));
        assert!(result.scalars.contains_key("profit"));
    }

    #[test]
    fn test_parser_skips_decision_tree_section() {
        let yaml_str = r#"
_forge_version: "5.0.0"

investment:
  value: 50000

decision_tree:
  name: "Investment Decision"
  root:
    type: decision
    name: "Invest?"
    branches:
      invest:
        cost: 50000
        next: market_outcome
      dont_invest:
        value: 0
  nodes:
    market_outcome:
      type: chance
      name: "Market Outcome"
      branches:
        success:
          probability: 0.6
          value: 150000
        failure:
          probability: 0.4
          value: 20000
"#;
        let yaml: Value = serde_yaml_ng::from_str(yaml_str).unwrap();
        let result = parse_v1_model(&yaml).unwrap();

        // Decision tree should NOT be parsed as a table
        assert!(!result.tables.contains_key("decision_tree"));
        // Scalars should be parsed correctly
        assert!(result.scalars.contains_key("investment"));
    }

    #[test]
    fn test_parser_skips_monte_carlo_section() {
        let yaml_str = r#"
_forge_version: "5.0.0"

revenue:
  value: 100000

monte_carlo:
  iterations: 10000
  variables:
    revenue:
      distribution: normal
      mean: 100000
      std: 10000
"#;
        let yaml: Value = serde_yaml_ng::from_str(yaml_str).unwrap();
        let result = parse_v1_model(&yaml).unwrap();

        // Monte Carlo should NOT be parsed as a table
        assert!(!result.tables.contains_key("monte_carlo"));
        // Scalars should be parsed correctly
        assert!(result.scalars.contains_key("revenue"));
    }

    #[test]
    fn test_parse_v4_backward_compatible_with_v1() {
        let yaml_content = r#"
_forge_version: "5.0.0"

sales:
  month: ["Jan", "Feb", "Mar"]
  revenue: [100, 200, 300]
  profit: "=revenue * 0.3"

summary:
  total:
    value: null
    formula: "=SUM(sales.revenue)"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        let sales = result.tables.get("sales").unwrap();
        assert_eq!(sales.columns.len(), 2);
        assert_eq!(sales.row_formulas.len(), 1);

        assert_eq!(result.scalars.len(), 1);
        let total = result.scalars.get("summary.total").unwrap();
        assert_eq!(total.formula, Some("=SUM(sales.revenue)".to_string()));

        assert!(sales.columns.get("revenue").unwrap().metadata.is_empty());
        assert!(total.metadata.is_empty());
    }

    #[test]
    fn test_parse_v4_mixed_formats() {
        let yaml_content = r#"
_forge_version: "5.0.0"
sales:
  month: ["Jan", "Feb", "Mar"]
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
    notes: "Rich format column"
  expenses: [50, 100, 150]
  profit: "=revenue - expenses"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        let sales = result.tables.get("sales").unwrap();

        assert!(sales.columns.get("month").unwrap().metadata.is_empty());
        assert!(sales.columns.get("expenses").unwrap().metadata.is_empty());

        let revenue = sales.columns.get("revenue").unwrap();
        assert_eq!(revenue.metadata.unit, Some("CAD".to_string()));
        assert_eq!(
            revenue.metadata.notes,
            Some("Rich format column".to_string())
        );
    }

    #[test]
    fn test_parse_v5_inputs_outputs_sections() {
        let yaml_content = r#"
_forge_version: "5.0.0"

inputs:
  tax_rate:
    value: 0.25
    formula: null
    unit: "%"
    notes: "Corporate tax rate"
  discount_rate:
    value: 0.10
    formula: null
    unit: "%"

outputs:
  net_profit:
    value: 75000
    formula: "=revenue * (1 - tax_rate)"
    unit: "CAD"
  npv:
    value: null
    formula: "=NPV(discount_rate, cashflows)"

data:
  quarter: ["Q1", "Q2", "Q3", "Q4"]
  revenue: [100000, 120000, 150000, 180000]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        assert!(result.tables.contains_key("data"));

        assert_eq!(result.scalars.len(), 4);

        let tax_rate = result.scalars.get("inputs.tax_rate").unwrap();
        assert_eq!(tax_rate.value, Some(0.25));
        assert!(tax_rate.formula.is_none());
        assert_eq!(tax_rate.metadata.unit, Some("%".to_string()));
        assert_eq!(
            tax_rate.metadata.notes,
            Some("Corporate tax rate".to_string())
        );

        let discount_rate = result.scalars.get("inputs.discount_rate").unwrap();
        assert_eq!(discount_rate.value, Some(0.10));

        let net_profit = result.scalars.get("outputs.net_profit").unwrap();
        assert_eq!(net_profit.value, Some(75000.0));
        assert_eq!(
            net_profit.formula,
            Some("=revenue * (1 - tax_rate)".to_string())
        );
        assert_eq!(net_profit.metadata.unit, Some("CAD".to_string()));

        let npv = result.scalars.get("outputs.npv").unwrap();
        assert!(npv.value.is_none());
        assert_eq!(
            npv.formula,
            Some("=NPV(discount_rate, cashflows)".to_string())
        );
    }

    #[test]
    fn test_null_in_numeric_array_error() {
        let yaml_content = r#"
_forge_version: "5.0.0"
data:
  values: [1000, null, 2000]
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path());
        assert!(result.is_err());

        let err = result.unwrap_err().to_string();
        assert!(err.contains("null values not allowed"));
        assert!(err.contains("Use 0 or remove the row"));
    }

    #[test]
    fn test_parse_v4_table_column_with_metadata() {
        let yaml_content = r#"
_forge_version: "5.0.0"

sales:
  month:
    value: ["Jan", "Feb", "Mar"]
    unit: "month"
  revenue:
    value: [100, 200, 300]
    unit: "CAD"
    notes: "Monthly revenue projection"
    validation_status: "PROJECTED"
  profit:
    formula: "=revenue * 0.3"
    unit: "CAD"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.tables.len(), 1);
        let sales = result.tables.get("sales").unwrap();

        let month = sales.columns.get("month").unwrap();
        assert_eq!(month.metadata.unit, Some("month".to_string()));

        let revenue = sales.columns.get("revenue").unwrap();
        assert_eq!(revenue.metadata.unit, Some("CAD".to_string()));
        assert_eq!(
            revenue.metadata.notes,
            Some("Monthly revenue projection".to_string())
        );
        assert_eq!(
            revenue.metadata.validation_status,
            Some("PROJECTED".to_string())
        );

        assert!(sales.row_formulas.contains_key("profit"));
        assert_eq!(sales.row_formulas.get("profit").unwrap(), "=revenue * 0.3");
    }
}
