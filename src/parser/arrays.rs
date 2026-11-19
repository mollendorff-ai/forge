//! Array parsing utilities for Forge YAML models
//!
//! Handles parsing of typed column arrays (Number, Text, Date, Boolean).

use crate::error::{ForgeError, ForgeResult};
use crate::types::ColumnValue;
use yaml_serde::Value;

/// Parse a YAML array into a typed `ColumnValue`
///
/// # Errors
///
/// Returns an error if the array is empty, contains mixed types, or has invalid values.
pub fn parse_array_value(col_name: &str, seq: &[Value]) -> ForgeResult<ColumnValue> {
    if seq.is_empty() {
        return Err(ForgeError::Parse(format!(
            "Column '{col_name}' cannot be empty"
        )));
    }

    // The type comes from the first value that is not empty (null, ADR-059);
    // a column of only empty cells is a column of numbers
    let array_type = seq
        .iter()
        .find(|v| !v.is_null())
        .map_or(Ok("Number"), detect_array_type)?;

    match array_type {
        "Number" => parse_numbers(col_name, seq),
        "Text" => {
            let mut texts = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::String(s) => texts.push(s.clone()),
                    Value::Null => return Err(empty_outside_numbers(col_name, i, "text")),
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Text, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    },
                }
            }
            Ok(ColumnValue::Text(texts))
        },
        "Date" => {
            let mut dates = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::String(s) => {
                        // Validate date format (YYYY-MM or YYYY-MM-DD)
                        if !is_valid_date_format(s) {
                            return Err(ForgeError::Parse(format!(
                                "Column '{col_name}' row {i}: Invalid date format '{s}' (expected YYYY-MM or YYYY-MM-DD)"
                            )));
                        }
                        dates.push(s.clone());
                    },
                    Value::Null => return Err(empty_outside_numbers(col_name, i, "dates")),
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Date, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    },
                }
            }
            Ok(ColumnValue::Date(dates))
        },
        "Boolean" => {
            let mut bools = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::Bool(b) => bools.push(*b),
                    Value::Null => return Err(empty_outside_numbers(col_name, i, "booleans")),
                    _ => {
                        return Err(ForgeError::Parse(format!(
                            "Column '{}' row {}: Expected Boolean, found {}",
                            col_name,
                            i,
                            type_name(val)
                        )));
                    },
                }
            }
            Ok(ColumnValue::Boolean(bools))
        },
        _ => Err(ForgeError::Parse(format!(
            "Column '{col_name}': Unsupported array type '{array_type}'"
        ))),
    }
}

/// A column of numbers; with a null, a column with empty cells (ADR-059)
fn parse_numbers(col_name: &str, seq: &[Value]) -> ForgeResult<ColumnValue> {
    let mut numbers = Vec::new();
    for (i, val) in seq.iter().enumerate() {
        match val {
            Value::Number(n) => {
                if let Some(f) = n.as_f64() {
                    numbers.push(Some(f));
                } else {
                    return Err(ForgeError::Parse(format!(
                        "Column '{col_name}' row {i}: Invalid number format"
                    )));
                }
            },
            // An empty cell; the schema check allows it only in 5.3.0
            Value::Null => numbers.push(None),
            _ => {
                return Err(ForgeError::Parse(format!(
                    "Column '{}' row {}: Expected Number, found {}",
                    col_name,
                    i,
                    type_name(val)
                )));
            },
        }
    }
    if numbers.iter().any(Option::is_none) {
        Ok(ColumnValue::Blanks(numbers))
    } else {
        Ok(ColumnValue::Number(numbers.into_iter().flatten().collect()))
    }
}

/// An empty cell is allowed only in a column of numbers (ADR-059)
fn empty_outside_numbers(col_name: &str, row: usize, kind: &str) -> ForgeError {
    ForgeError::Parse(format!(
        "Column '{col_name}' row {row}: an empty cell (null) is allowed only in a column of \
         numbers, and this is a column of {kind}"
    ))
}

/// Detect the type of a YAML value
///
/// # Errors
///
/// Returns an error if the value is null or an unsupported type (e.g., nested mapping).
pub fn detect_array_type(val: &Value) -> ForgeResult<&'static str> {
    match val {
        Value::Number(_) => Ok("Number"),
        Value::String(s) => {
            // Check if it's a date string
            if is_valid_date_format(s) {
                Ok("Date")
            } else {
                Ok("Text")
            }
        },
        Value::Bool(_) => Ok("Boolean"),
        Value::Null => Err(ForgeError::Parse(
            "Array cannot start with null. First element must be a valid value to determine column type.".to_string()
        )),
        _ => Err(ForgeError::Parse(format!(
            "Unsupported array element type: {}",
            type_name(val)
        ))),
    }
}

/// Check if a string is a valid date format (YYYY-MM or YYYY-MM-DD)
#[must_use]
pub fn is_valid_date_format(s: &str) -> bool {
    // YYYY-MM format
    if s.len() == 7 {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 2 {
            return parts[0].len() == 4
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].len() == 2
                && parts[1].chars().all(|c| c.is_ascii_digit());
        }
    }
    // YYYY-MM-DD format
    if s.len() == 10 {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() == 3 {
            return parts[0].len() == 4
                && parts[0].chars().all(|c| c.is_ascii_digit())
                && parts[1].len() == 2
                && parts[1].chars().all(|c| c.is_ascii_digit())
                && parts[2].len() == 2
                && parts[2].chars().all(|c| c.is_ascii_digit());
        }
    }
    false
}

/// Get the type name of a YAML value for error messages
#[must_use]
pub const fn type_name(val: &Value) -> &'static str {
    match val {
        Value::Null => "Null",
        Value::Bool(_) => "Boolean",
        Value::Number(_) => "Number",
        Value::String(_) => "String",
        Value::Sequence(_) => "Array",
        Value::Mapping(_) => "Mapping",
        Value::Tagged(_) => "Tagged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_number_array() {
        let yaml_seq: Vec<Value> = vec![
            Value::Number(1.into()),
            Value::Number(2.into()),
            Value::Number(3.into()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Number(nums) => {
                assert_eq!(nums, vec![1.0, 2.0, 3.0]);
            },
            _ => panic!("Expected Number array"),
        }
    }

    #[test]
    fn test_parse_text_array() {
        let yaml_seq: Vec<Value> = vec![
            Value::String("A".to_string()),
            Value::String("B".to_string()),
            Value::String("C".to_string()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Text(texts) => {
                assert_eq!(texts, vec!["A", "B", "C"]);
            },
            _ => panic!("Expected Text array"),
        }
    }

    #[test]
    fn test_parse_date_array() {
        let yaml_seq: Vec<Value> = vec![
            Value::String("2025-01".to_string()),
            Value::String("2025-02".to_string()),
            Value::String("2025-03".to_string()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Date(dates) => {
                assert_eq!(dates, vec!["2025-01", "2025-02", "2025-03"]);
            },
            _ => panic!("Expected Date array"),
        }
    }

    #[test]
    fn test_parse_boolean_array() {
        let yaml_seq: Vec<Value> = vec![Value::Bool(true), Value::Bool(false), Value::Bool(true)];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Boolean(bools) => {
                assert_eq!(bools, vec![true, false, true]);
            },
            _ => panic!("Expected Boolean array"),
        }
    }

    #[test]
    fn test_mixed_type_array_error() {
        let yaml_seq: Vec<Value> = vec![Value::Number(1.into()), Value::String("text".to_string())];
        let result = parse_array_value("test_col", &yaml_seq);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Expected Number, found String"));
    }

    #[test]
    fn test_empty_array_error() {
        let yaml_seq: Vec<Value> = vec![];
        let result = parse_array_value("test_col", &yaml_seq);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("cannot be empty"));
    }

    #[test]
    fn test_invalid_date_format_error() {
        let yaml_seq: Vec<Value> = vec![
            Value::String("2025-01".to_string()),
            Value::String("2025-1".to_string()),
        ];
        let result = parse_array_value("test_col", &yaml_seq);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid date format"));
    }

    #[test]
    fn test_valid_date_formats() {
        assert!(is_valid_date_format("2025-01"));
        assert!(is_valid_date_format("2025-12"));
        assert!(is_valid_date_format("2025-01-15"));
        assert!(is_valid_date_format("2025-12-31"));
        assert!(!is_valid_date_format("2025-1"));
        assert!(!is_valid_date_format("2025-1-1"));
        assert!(!is_valid_date_format("25-01-01"));
        assert!(!is_valid_date_format("not-a-date"));
    }

    #[test]
    fn test_parse_date_format_yyyy_mm_dd() {
        let yaml_seq: Vec<Value> = vec![
            Value::String("2025-01-15".to_string()),
            Value::String("2025-02-20".to_string()),
        ];
        let result = parse_array_value("test_col", &yaml_seq).unwrap();

        match result {
            ColumnValue::Date(dates) => {
                assert_eq!(dates, vec!["2025-01-15", "2025-02-20"]);
            },
            _ => panic!("Expected Date array"),
        }
    }

    #[test]
    fn test_type_name_function() {
        assert_eq!(type_name(&Value::Null), "Null");
        assert_eq!(type_name(&Value::Bool(true)), "Boolean");
        assert_eq!(type_name(&Value::Number(1.into())), "Number");
        assert_eq!(type_name(&Value::String("test".to_string())), "String");
        assert_eq!(type_name(&Value::Sequence(vec![])), "Array");
        assert_eq!(
            type_name(&Value::Mapping(yaml_serde::Mapping::new())),
            "Mapping"
        );
    }

    #[test]
    fn test_detect_array_type_unsupported() {
        let val = Value::Sequence(vec![]);
        let result = detect_array_type(&val);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unsupported"));
    }

    #[test]
    fn test_boolean_array_wrong_type() {
        let yaml_seq: Vec<Value> = vec![Value::Bool(true), Value::String("not bool".to_string())];
        let result = parse_array_value("test_col", &yaml_seq);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Expected Boolean"));
    }

    #[test]
    fn test_date_array_wrong_type() {
        let yaml_seq: Vec<Value> = vec![
            Value::String("2025-01".to_string()),
            Value::Number(123.into()),
        ];
        let result = parse_array_value("test_col", &yaml_seq);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Expected Date"));
    }

    #[test]
    fn test_text_array_wrong_type() {
        let yaml_seq: Vec<Value> = vec![Value::String("text".to_string()), Value::Bool(true)];
        let result = parse_array_value("test_col", &yaml_seq);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Expected Text"));
    }

    #[test]
    fn test_null_first_element_error() {
        let result = detect_array_type(&Value::Null);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("cannot start with null"));
    }

    mod empty_cells {
        //! An empty cell in a column of numbers (ADR-059): written `null` in model
        //! format 5.3.0, kept by `forge calculate`, shown as null in JSON, exported as
        //! an empty cell and imported back as one. Formats before 5.3.0 refuse it.

        use crate::cli::{calculate_core, import_core};
        use std::path::Path;

        const MODEL: &str = r#"_forge_version: "5.3.0"
months:
  actual: [985, 1032, null, null]
  is_actual: "=ISNUMBER(actual)"
  shown: "=actual"
totals:
  average:
    formula: "=AVERAGE(months.actual)"
  months_closed:
    formula: "=COUNT(months.actual)"
"#;

        fn write(dir: &Path, name: &str, text: &str) -> std::path::PathBuf {
            let path = dir.join(name);
            std::fs::write(&path, text).unwrap();
            path
        }

        #[test]
        fn an_empty_cell_is_skipped_by_aggregates_and_read_as_0_by_formulas() {
            let dir = tempfile::tempdir().unwrap();
            let result = calculate_core(&write(dir.path(), "m.yaml", MODEL), true, None).unwrap();
            // (985 + 1032) / 2, of the two months that have one
            assert_eq!(result.scalars["totals.average"], Some(1008.5));
            assert_eq!(result.scalars["totals.months_closed"], Some(2.0));
            let column = |name: &str| -> Vec<serde_json::Value> {
                result.tables["months"].values[name]
                    .iter()
                    .map(|v| serde_json::to_value(v).unwrap())
                    .collect()
            };
            // JSON shows the empty cell as null; a formula that reads it gives 0
            assert_eq!(column("actual")[2], serde_json::Value::Null);
            assert_eq!(column("shown")[2], serde_json::json!(0.0));
            assert_eq!(column("is_actual")[2], serde_json::json!(false));
        }

        #[test]
        fn forge_calculate_keeps_the_empty_cells_it_writes_back() {
            let dir = tempfile::tempdir().unwrap();
            let path = write(dir.path(), "m.yaml", MODEL);
            calculate_core(&path, false, None).unwrap();
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(text.contains("actual: [985, 1032, null, null]"), "{text}");
            assert!(text.contains("value: 1008.5"), "{text}");
        }

        #[test]
        fn an_earlier_format_says_which_one_has_the_empty_cell() {
            let dir = tempfile::tempdir().unwrap();
            for version in ["5.1.0", "5.2.0"] {
                let text = MODEL.replace("5.3.0", version);
                let err = calculate_core(&write(dir.path(), "m.yaml", &text), true, None)
                    .unwrap_err()
                    .to_string();
                assert!(err.contains("'months.actual' row 3 is empty"), "{err}");
                assert!(err.contains("5.3.0"), "{err}");
            }
        }

        #[test]
        fn only_a_column_of_numbers_has_empty_cells() {
            let dir = tempfile::tempdir().unwrap();
            let text = "_forge_version: \"5.3.0\"\nt:\n  name: [\"a\", null]\n";
            let err = calculate_core(&write(dir.path(), "m.yaml", text), true, None)
                .unwrap_err()
                .to_string();
            // The schema says so before the parser does
            assert!(
                err.contains("'t.name' mixes text and empty entries"),
                "{err}"
            );
        }

        #[test]
        fn export_leaves_the_cell_empty_and_import_reads_it_back() {
            use calamine::{open_workbook, Data, Reader, Xlsx};
            let dir = tempfile::tempdir().unwrap();
            let model = write(dir.path(), "m.yaml", MODEL);
            let xlsx = dir.path().join("m.xlsx");
            crate::cli::console::capture(|c| crate::cli::export(c, &model, &xlsx, false))
                .0
                .unwrap();
            let mut book: Xlsx<_> = open_workbook(&xlsx).unwrap();
            let sheet = book.worksheet_range("months").unwrap();
            assert_eq!(sheet.get((1, 0)), Some(&Data::Float(985.0)));
            assert!(
                matches!(sheet.get((3, 0)), None | Some(Data::Empty)),
                "{:?}",
                sheet.get((3, 0))
            );

            let back = dir.path().join("back.yaml");
            import_core(&xlsx, &back, false, false).unwrap();
            let result = calculate_core(&back, true, None).unwrap();
            assert_eq!(result.scalars["totals.average"], Some(1008.5));
            let actual = &result.tables["months"].values["actual"];
            assert_eq!(
                serde_json::to_value(&actual[3]).unwrap(),
                serde_json::Value::Null
            );
        }
    }
}
