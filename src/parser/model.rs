//! Core model parsing for Forge YAML files
//!
//! Handles parsing of the main model structure including tables, scalars, and scenarios.

use crate::error::{ForgeError, ForgeResult};
use crate::types::{ParsedModel, Scenario};
use yaml_serde::Value;

use super::includes::parse_includes;
use super::schema::validate_against_schema;
use super::variables::{is_nested_scalar_section, parse_scalar_variable, parse_table};

/// Parse v1.0.0 array model
///
/// # Errors
///
/// Returns an error if the YAML fails schema validation or contains invalid table/scalar definitions.
pub fn parse_v1_model(yaml: &Value) -> ForgeResult<ParsedModel> {
    // No NaN or infinity anywhere, before anything reads a number (the
    // schema's message for `.nan` would call it empty)
    super::finite::check_finite(yaml)?;
    // Validate against JSON Schema - this is mandatory
    validate_against_schema(yaml)?;
    read_model(yaml)
}

/// The top-level sections that are analyses, read by their own commands
const ANALYSES: [&str; 6] = [
    "monte_carlo",
    "tornado",
    "decision_tree",
    "real_options",
    "bootstrap",
    "bayesian_network",
];

/// The tables, scalars and scenarios of a model the schema has accepted.
fn read_model(yaml: &Value) -> ForgeResult<ParsedModel> {
    let mut model = ParsedModel::new();
    let version = yaml
        .get("_forge_version")
        .and_then(Value::as_str)
        .unwrap_or("");

    // Parse each top-level key as either a table or scalar
    if let Value::Mapping(map) = yaml {
        for (key, value) in map {
            let key_str = key
                .as_str()
                .ok_or_else(|| ForgeError::Parse("Table name must be a string".to_string()))?;

            // Skip special keys (handled by specific commands)
            // Note: scenarios is NOT skipped here - it has special handling below
            // to distinguish scenario overrides from tables named "scenarios"
            if key_str == "_forge_version" || key_str == "_name" {
                continue;
            }
            // An analysis section is read by its command; the model notes
            // that the file has one (ADR-077)
            if ANALYSES.contains(&key_str) {
                model.analyses.push(key_str.to_string());
                continue;
            }

            // Games (5.5.0, ADR-068); before 5.5.0 a group named `games`
            // reads as a group (the schema check refused a game section)
            if key_str == "games" && super::schema::format_at_least(version, "5.5.0") {
                for game in crate::games::parse_games(value)? {
                    model.games.insert(game.name().to_string(), game);
                }
                continue;
            }

            // What the file as written should give (5.6.0, ADR-074); before
            // 5.6.0 a group named `expect` reads as a group
            if key_str == "expect" && super::schema::format_at_least(version, "5.6.0") {
                model.expect = super::expect::parse_expect(value, "expect")?;
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
                        && scenarios_map
                            .values()
                            .filter_map(Value::as_mapping)
                            .any(|m| {
                                // A number (a flat override, or `probability`) or
                                // a `scalars` mapping, which needs no probability
                                m.iter().any(|(_, vv)| match vv {
                                    Value::Number(_) => true,
                                    Value::String(text) => super::variables::looks_like_date(text),
                                    _ => false,
                                }) || matches!(m.get("scalars"), Some(Value::Mapping(_)))
                            });

                    if is_scenarios_section {
                        parse_scenarios(scenarios_map, &mut model)?;
                        continue;
                    }
                    // Otherwise fall through to parse as table
                }
            }

            // A top-level number is an input scalar (5.1.0 short form)
            if let Value::Number(n) = value {
                let variable = crate::types::Variable::new(key_str.to_string(), n.as_f64(), None);
                model.add_scalar(key_str.to_string(), variable);
                continue;
            }
            // So is a date (5.4.0, ADR-060)
            if let Value::String(text) = value {
                if let Some(serial) = super::variables::scalar_date(text, key_str)? {
                    model.add_scalar(key_str.to_string(), date_input(key_str, serial));
                    continue;
                }
            }

            // A table, a group of scalars, or one scalar (ADR-051 "How a group is read")
            if let Value::Mapping(inner_map) = value {
                let is_one_scalar = (inner_map.contains_key("value")
                    && !matches!(inner_map.get("value"), Some(Value::Sequence(_))))
                    || (inner_map.contains_key("formula")
                        && !inner_map.values().any(|v| matches!(v, Value::Mapping(_))));
                if is_one_scalar && !is_table(inner_map) {
                    let variable = parse_scalar_variable(value, key_str)?;
                    model.add_scalar(key_str.to_string(), variable);
                } else if is_table(inner_map) {
                    let table = parse_table(key_str, inner_map)?;
                    model.add_table(table);
                } else if is_nested_scalar_section(inner_map) {
                    // A group of scalars (e.g., summary.total)
                    parse_nested_scalars(key_str, inner_map, &mut model)?;
                } else {
                    // Only formula columns: a table
                    let table = parse_table(key_str, inner_map)?;
                    model.add_table(table);
                }
            }
        }
    }

    check_game_names(&model)?;

    // Note: Table column length validation is deferred to calculation time
    // This allows test files to have columns of different lengths when used independently
    // Row-wise operations will still validate at runtime in array_calculator

    Ok(model)
}

/// A game's results are scalars named under it (`stag_hunt.equilibria`), so
/// its name cannot also be a table, a scalar or a group of scalars.
///
/// # Errors
///
/// Returns an error naming the first game whose name is taken.
pub fn check_game_names(model: &ParsedModel) -> ForgeResult<()> {
    for name in model.games.keys() {
        let prefix = format!("{name}.");
        let taken = if model.tables.contains_key(name) {
            Some("a table".to_string())
        } else if model.scalars.contains_key(name) {
            Some("a scalar".to_string())
        } else {
            model
                .scalars
                .keys()
                .find(|s| s.starts_with(&prefix))
                .map(|s| format!("the group of scalar '{s}'"))
        };
        if let Some(what) = taken {
            return Err(ForgeError::Parse(format!(
                "game '{name}' has the name of {what}; its results are named under it \
                 ({name}.equilibria), so rename one"
            )));
        }
    }
    Ok(())
}

/// Parse nested scalar variables (e.g., summary.total, summary.average)
///
/// # Errors
///
/// Returns an error if a nested scalar name is not a string or has an invalid format.
pub fn parse_nested_scalars(
    parent_key: &str,
    map: &yaml_serde::Mapping,
    model: &mut ParsedModel,
) -> ForgeResult<()> {
    for (key, value) in map {
        let key_str = key
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Scalar name must be a string".to_string()))?;

        let full_path = format!("{parent_key}.{key_str}");
        match value {
            // 5.1.0 short form: a plain number is an input
            Value::Number(n) => {
                let variable = crate::types::Variable::new(full_path.clone(), n.as_f64(), None);
                model.add_scalar(full_path, variable);
            },
            Value::Mapping(child_map)
                if child_map.contains_key("value") || child_map.contains_key("formula") =>
            {
                let variable = parse_scalar_variable(value, &full_path)?;
                model.add_scalar(full_path, variable);
            },
            Value::String(text) if super::variables::looks_like_date(text) => {
                // Text that looks like a date is a date or an error
                let serial = super::variables::scalar_date(text, &full_path)?.unwrap_or_default();
                model.add_scalar(full_path.clone(), date_input(&full_path, serial));
            },
            Value::String(s) if s.starts_with('=') => {
                return Err(ForgeError::Parse(format!(
                    "'{full_path}' is a formula in a group of scalars; write it as \
                     `{key_str}: {{formula: \"{s}\"}}` so its result has a place in the file"
                )));
            },
            _ => {
                return Err(ForgeError::Parse(format!(
                    "'{full_path}' must be a number, a date (2026-10-31) or a mapping with `value:` or `formula:`"
                )));
            },
        }
    }
    Ok(())
}

/// An input scalar written as a date, held as its day number (ADR-060).
fn date_input(path: &str, serial: f64) -> crate::types::Variable {
    let mut variable = crate::types::Variable::new(path.to_string(), Some(serial), None);
    variable.is_date = true;
    variable
}

/// A mapping is a table if any child is a list, a mapping whose `value` is
/// a list (a column with metadata), or a period column (ADR-053).
#[must_use]
pub fn is_table(map: &yaml_serde::Mapping) -> bool {
    map.values().any(|v| match v {
        Value::Sequence(_) => true,
        Value::Mapping(m) => {
            matches!(m.get("value"), Some(Value::Sequence(_)))
                || super::periods::is_period_column(m)
        },
        _ => false,
    })
}

/// Parse scenarios section from YAML
///
/// Supports two formats:
///
/// **Flat format** (variable overrides only):
/// ```yaml
/// scenarios:
///   base:
///     growth_rate: 0.05
///     churn_rate: 0.02
/// ```
///
/// **Structured format** (`ScenarioDefinition` with probability, description, scalars):
/// ```yaml
/// scenarios:
///   base:
///     probability: 0.50
///     description: "Base case"
///     scalars:
///       growth_rate: 0.05
///       churn_rate: 0.02
/// ```
///
/// # Errors
///
/// Returns an error if a scenario name or variable is not valid, if a variable
/// value is not a number, or if the `scalars` field is not a mapping.
pub fn parse_scenarios(
    scenarios_map: &yaml_serde::Mapping,
    model: &mut ParsedModel,
) -> ForgeResult<()> {
    for (scenario_name, scenario_value) in scenarios_map {
        let name = scenario_name
            .as_str()
            .ok_or_else(|| ForgeError::Parse("Scenario name must be a string".to_string()))?;

        if let Value::Mapping(overrides_map) = scenario_value {
            let mut scenario = Scenario::new();

            // Detect structured format: has reserved keys "probability",
            // "scalars" or "expect" (ADR-074)
            let is_structured = ["probability", "scalars", "expect"]
                .iter()
                .any(|key| overrides_map.contains_key(*key));

            if is_structured {
                scenario.description = overrides_map
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                if let Some(expect) = overrides_map.get("expect") {
                    scenario.expect =
                        super::expect::parse_expect(expect, &format!("scenarios.{name}.expect"))?;
                }
                // Structured ScenarioDefinition format: extract overrides from "scalars" only.
                // "probability" and "description" are intentionally skipped here;
                // they are consumed by ScenarioEngine via serde deserialization.
                if let Some(scalars_value) = overrides_map.get("scalars") {
                    if let Value::Mapping(scalars_map) = scalars_value {
                        for (var_name, var_value) in scalars_map {
                            let var_name_str = var_name.as_str().ok_or_else(|| {
                                ForgeError::Parse("Variable name must be a string".to_string())
                            })?;

                            add_override(&mut scenario, name, var_name_str, var_value)?;
                        }
                    } else {
                        return Err(ForgeError::Parse(format!(
                            "Scenario '{name}': 'scalars' must be a mapping of variable overrides"
                        )));
                    }
                }
            } else {
                // Flat format: each key is a variable name with a numeric override value
                for (var_name, var_value) in overrides_map {
                    let var_name_str = var_name.as_str().ok_or_else(|| {
                        ForgeError::Parse("Variable name must be a string".to_string())
                    })?;

                    add_override(&mut scenario, name, var_name_str, var_value)?;
                }
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

/// One override of scenario `name`: a number, or a date for a date scalar
/// (ADR-060).
fn add_override(
    scenario: &mut Scenario,
    name: &str,
    var_name: &str,
    var_value: &Value,
) -> ForgeResult<()> {
    let must = || {
        ForgeError::Parse(format!(
            "Scenario '{name}': Variable '{var_name}' must be a number or a date (2026-10-31)"
        ))
    };
    match var_value {
        Value::Number(n) => {
            scenario.add_override(var_name.to_string(), n.as_f64().unwrap_or_default());
        },
        Value::String(text) => {
            let path = format!("scenarios.{name}.{var_name}");
            let serial = super::variables::scalar_date(text, &path)?.ok_or_else(must)?;
            scenario.add_override(var_name.to_string(), serial);
            scenario.dates.insert(var_name.to_string());
        },
        _ => return Err(must()),
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
        let yaml: Value = yaml_serde::from_str(yaml_str).unwrap();
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
    fn test_parse_scenarios_structured_format() {
        let yaml_content = r#"
_forge_version: "5.0.0"

price:
  value: 100
  formula: null
units:
  value: 50
  formula: null

scenarios:
  high:
    probability: 0.5
    description: "High price"
    scalars:
      price: 200
      units: 60
  low:
    probability: 0.5
    description: "Low price"
    scalars:
      price: 50
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.scenarios.len(), 2);

        let high = result.scenarios.get("high").unwrap();
        assert_eq!(high.overrides.get("price"), Some(&200.0));
        assert_eq!(high.overrides.get("units"), Some(&60.0));
        // probability/description should NOT appear as overrides
        assert!(!high.overrides.contains_key("probability"));
        assert!(!high.overrides.contains_key("description"));

        let low = result.scenarios.get("low").unwrap();
        assert_eq!(low.overrides.get("price"), Some(&50.0));
    }

    #[test]
    fn test_parse_scenarios_structured_no_scalars() {
        // Structured format with empty/missing scalars section is valid (no overrides)
        let yaml_content = r#"
_forge_version: "5.0.0"

rate:
  value: 0.05
  formula: null

scenarios:
  base:
    probability: 1.0
    description: "Base only"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path()).unwrap();

        assert_eq!(result.scenarios.len(), 1);
        let base = result.scenarios.get("base").unwrap();
        assert!(base.overrides.is_empty());
    }

    #[test]
    fn test_parse_scenarios_structured_invalid_scalars_type() {
        let yaml_content = r#"
_forge_version: "5.0.0"

scenarios:
  bad:
    probability: 0.5
    scalars: "not a mapping"
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(yaml_content.as_bytes()).unwrap();

        let result = parse_model(temp_file.path());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("scalars"));
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
        let yaml: Value = yaml_serde::from_str(yaml_str).unwrap();
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
        let yaml: Value = yaml_serde::from_str(yaml_str).unwrap();
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
        let yaml: Value = yaml_serde::from_str(yaml_str).unwrap();
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

        // An empty cell needs model format 5.3.0 (ADR-059)
        let err = result.unwrap_err().to_string();
        assert!(err.contains("'data.values' row 2 is empty"), "{err}");
        assert!(err.contains("Set `_forge_version: \"5.3.0\"`"), "{err}");
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

    fn mapping(text: &str) -> yaml_serde::Mapping {
        yaml_serde::from_str(text).unwrap()
    }

    fn read(text: &str) -> ForgeResult<ParsedModel> {
        read_model(&yaml_serde::from_str(text).unwrap())
    }

    fn reason<T>(result: ForgeResult<T>) -> String {
        result.err().unwrap().to_string()
    }

    /// The analysis sections are noted in the file's order, and read as
    /// neither tables nor scalars (ADR-077)
    #[test]
    fn a_files_analysis_sections_are_noted_in_its_order() {
        let model =
            read("bootstrap:\n  data: [1, 2]\nprice: 16\ntornado:\n  output: price\n_name: x\n")
                .unwrap();
        assert_eq!(model.analyses, ["bootstrap", "tornado"]);
        assert_eq!(model.scalars.keys().collect::<Vec<_>>(), ["price"]);
        assert_eq!(model.tables.len(), 0);
        assert_eq!(read("price: 16\n").unwrap().analyses, Vec::<String>::new());
    }

    #[test]
    fn what_the_schema_lets_through_is_still_read_with_its_reasons() {
        assert_eq!(
            reason(read("_includes:\n  - file: x.yaml\n")),
            "Parse error: Include 'x.yaml' must have an 'as' field for the namespace"
        );
        assert_eq!(
            reason(read("1:\n  value: 5\n")),
            "Parse error: Table name must be a string"
        );
    }

    #[test]
    fn a_top_level_date_is_an_input_scalar_held_as_its_day_number() {
        let model = read("start: \"2026-10-31\"\n").unwrap();
        let start = &model.scalars["start"];
        assert_eq!(start.value, Some(46326.0));
        assert!(start.is_date);
        assert_eq!(
            reason(read("start: \"2026-02-30\"\n")),
            "Parse error: 'start' is not a date: 2026-02-30 is no day of the calendar (a date is written 2026-10-31)"
        );
    }

    #[test]
    fn a_mapping_of_formula_columns_alone_is_a_table() {
        let model = read("t:\n  x: \"=1 + 2\"\n").unwrap();
        assert_eq!(model.tables.len(), 1);
        assert!(model.scalars.is_empty());
    }

    #[test]
    fn a_group_of_scalars_holds_numbers_dates_and_mappings_only() {
        let mut model = ParsedModel::new();
        parse_nested_scalars(
            "g",
            &mapping("a: 1\nb: \"2026-10-31\"\nc:\n  value: 3\n"),
            &mut model,
        )
        .unwrap();
        assert_eq!(model.scalars["g.a"].value, Some(1.0));
        assert_eq!(model.scalars["g.b"].value, Some(46326.0));
        assert!(model.scalars["g.b"].is_date);
        assert_eq!(model.scalars["g.c"].value, Some(3.0));
        let refused = |text: &str| {
            reason(parse_nested_scalars(
                "g",
                &mapping(text),
                &mut ParsedModel::new(),
            ))
        };
        assert_eq!(
            refused("1: 5\n"),
            "Parse error: Scalar name must be a string"
        );
        assert_eq!(
            refused("a: \"=1 + 2\"\n"),
            "Parse error: 'g.a' is a formula in a group of scalars; write it as \
             `a: {formula: \"=1 + 2\"}` so its result has a place in the file"
        );
        assert_eq!(
            refused("a: hello\n"),
            "Parse error: 'g.a' must be a number, a date (2026-10-31) or a mapping with `value:` or `formula:`"
        );
    }

    #[test]
    fn a_scenario_is_read_in_either_form_or_refused_with_its_reason() {
        let mut model = ParsedModel::new();
        parse_scenarios(
            &mapping(
                "flat:\n  rate: 0.05\n  start: \"2026-10-31\"\nfull:\n  probability: 0.5\n  description: Base\n  scalars:\n    rate: 0.07\n",
            ),
            &mut model,
        )
        .unwrap();
        assert_eq!(
            model.scenarios["flat"].overrides["rate"].to_bits(),
            0.05_f64.to_bits()
        );
        assert_eq!(
            model.scenarios["flat"].overrides["start"].to_bits(),
            46326.0_f64.to_bits()
        );
        assert!(model.scenarios["flat"].dates.contains("start"));
        assert_eq!(
            model.scenarios["full"].overrides["rate"].to_bits(),
            0.07_f64.to_bits()
        );
        assert_eq!(model.scenarios["full"].description.as_deref(), Some("Base"));
        let refused = |text: &str| reason(parse_scenarios(&mapping(text), &mut ParsedModel::new()));
        assert_eq!(
            refused("1:\n  rate: 1\n"),
            "Parse error: Scenario name must be a string"
        );
        assert_eq!(
            refused("up: 5\n"),
            "Parse error: Scenario 'up' must be a mapping of variable overrides"
        );
        assert_eq!(
            refused("up:\n  probability: 1\n  scalars: 5\n"),
            "Parse error: Scenario 'up': 'scalars' must be a mapping of variable overrides"
        );
        assert_eq!(
            refused("up:\n  probability: 1\n  scalars:\n    1: 5\n"),
            "Parse error: Variable name must be a string"
        );
        assert_eq!(
            refused("up:\n  1: 5\n"),
            "Parse error: Variable name must be a string"
        );
        for value in ["hello", "[1]"] {
            assert_eq!(
                refused(&format!("up:\n  rate: {value}\n")),
                "Parse error: Scenario 'up': Variable 'rate' must be a number or a date (2026-10-31)"
            );
        }
    }

    #[test]
    fn what_is_neither_a_scenario_nor_a_scalar_nor_a_table_is_read_as_it_is() {
        // A scenario with a description and a `scalars` mapping, and no number
        let model =
            read("scenarios:\n  s:\n    description: Base\n    scalars:\n      rate: 1\n").unwrap();
        assert_eq!(
            model.scenarios["s"].overrides["rate"].to_bits(),
            1.0_f64.to_bits()
        );
        // Mappings of lists are a table that is called scenarios
        let model = read("scenarios:\n  rate: [1, 2]\n").unwrap();
        assert_eq!(model.tables.len(), 1);
        assert!(model.scenarios.is_empty());
        // Text that is not a date, as a value, is skipped
        let model = read("note: hello\n").unwrap();
        assert!(model.scalars.is_empty() && model.tables.is_empty());
        // Scenarios are refused with the reason of the one that fails
        assert_eq!(
            reason(read("scenarios:\n  up:\n    n: 1\n    rate: hello\n")),
            "Parse error: Scenario 'up': Variable 'rate' must be a number or a date (2026-10-31)"
        );
    }

    #[test]
    fn a_scalar_or_table_that_cannot_be_read_gives_its_reason() {
        assert_eq!(
            reason(read("t:\n  1: [1, 2]\n")),
            "Parse error: Column name must be a string"
        );
        assert_eq!(reason(read("t:\n  value: 1\n  bogus: 2\n")), "Parse error: 't.bogus': a scalar has no key 'bogus'. A scalar's keys are value, formula, unit, notes, source, validation_status, last_updated, better, format");
        assert_eq!(
            reason(read("t:\n  x: \"=1\"\n  1: \"=2\"\n")),
            "Parse error: Column name must be a string"
        );
    }

    #[test]
    fn a_value_that_is_neither_text_nor_a_mapping_is_skipped() {
        let model = read("scenarios: hello\nflag: true\nlist_of: null\n_includes: none\n").unwrap();
        assert!(model.scalars.is_empty() && model.tables.is_empty() && model.scenarios.is_empty());
    }

    #[test]
    fn a_date_in_a_group_or_a_scenario_that_is_no_day_is_refused() {
        assert_eq!(
            reason(parse_nested_scalars(
                "g",
                &mapping("a: \"2026-02-30\"\n"),
                &mut ParsedModel::new()
            )),
            "Parse error: 'g.a' is not a date: 2026-02-30 is no day of the calendar (a date is written 2026-10-31)"
        );
        assert_eq!(
            reason(parse_scenarios(
                &mapping("up:\n  probability: 1\n  scalars:\n    rate: hello\n"),
                &mut ParsedModel::new()
            )),
            "Parse error: Scenario 'up': Variable 'rate' must be a number or a date (2026-10-31)"
        );
    }

    #[test]
    fn a_scenario_override_that_is_no_day_is_refused() {
        assert_eq!(
            reason(parse_scenarios(
                &mapping("up:\n  rate: \"2026-02-30\"\n"),
                &mut ParsedModel::new()
            )),
            "Parse error: 'scenarios.up.rate' is not a date: 2026-02-30 is no day of the calendar (a date is written 2026-10-31)"
        );
    }

    #[test]
    fn a_mapping_of_mappings_called_scenarios_without_numbers_is_not_a_scenario_section() {
        // It is read as a table, and its column is neither a list nor a formula
        assert_eq!(
            reason(read("scenarios:\n  a:\n    x: hello\n")),
            "Parse error: Column 'a' in table 'scenarios' must be an array or formula"
        );
    }

    #[test]
    fn a_document_that_is_not_a_mapping_has_nothing_in_it() {
        let model = read("- 1\n").unwrap();
        assert!(model.scalars.is_empty() && model.tables.is_empty());
    }
}
