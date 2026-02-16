//! Array parsing utilities for Forge YAML models
//!
//! Handles parsing of typed column arrays (Number, Text, Date, Boolean).

use crate::error::{ForgeError, ForgeResult};
use crate::types::ColumnValue;
use serde_yaml_ng::Value;

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

    // Detect the type from the first element
    let array_type = detect_array_type(&seq[0])?;

    match array_type {
        "Number" => {
            let mut numbers = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::Number(n) => {
                        if let Some(f) = n.as_f64() {
                            numbers.push(f);
                        } else {
                            return Err(ForgeError::Parse(format!(
                                "Column '{col_name}' row {i}: Invalid number format"
                            )));
                        }
                    },
                    Value::Null => {
                        // Provide clear error for null values in numeric arrays
                        return Err(ForgeError::Parse(format!(
                            "Column '{col_name}' row {i}: null values not allowed in numeric arrays. \
                            Use 0 or remove the row if the value is missing."
                        )));
                    },
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
            Ok(ColumnValue::Number(numbers))
        },
        "Text" => {
            let mut texts = Vec::new();
            for (i, val) in seq.iter().enumerate() {
                match val {
                    Value::String(s) => texts.push(s.clone()),
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
            type_name(&Value::Mapping(serde_yaml_ng::Mapping::new())),
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
}
