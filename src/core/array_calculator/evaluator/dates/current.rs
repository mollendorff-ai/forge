//! Current date/time functions: TODAY, NOW

use super::Value;

/// Evaluate a current date/time function, or `None` for another name.
/// Neither can fail.
pub fn try_evaluate(name: &str) -> Option<Value> {
    let result = match name {
        "TODAY" => {
            use chrono::Local;
            let today = Local::now().date_naive();
            Value::Text(today.format("%Y-%m-%d").to_string())
        },

        "NOW" => {
            use chrono::Local;
            let now = Local::now();
            Value::Text(now.format("%Y-%m-%d %H:%M:%S").to_string())
        },

        _ => return None,
    };

    Some(result)
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    #[test]
    fn test_today_function_basic() {
        let mut model = ParsedModel::new();

        let mut data = Table::new("dates".to_string());
        data.add_column(Column::new(
            "dummy".to_string(),
            ColumnValue::Number(vec![1.0]),
        ));
        data.row_formulas
            .insert("current".to_string(), "=TODAY()".to_string());
        model.add_table(data);

        let today = || ColumnValue::Text(vec![chrono::Local::now().format("%Y-%m-%d").to_string()]);
        let before = today();
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");

        let col = result
            .tables
            .get("dates")
            .unwrap()
            .columns
            .get("current")
            .unwrap();
        // The day it is, also if the calculation ran across midnight
        let after = today();
        let got = &col.values;
        assert!([before, after].contains(got), "{got:?}");
    }

    #[test]
    fn test_today_minus_today_equals_zero() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "result".to_string(),
            Variable::new(
                "result".to_string(),
                None,
                Some("=TODAY() - TODAY()".to_string()),
            ),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        assert_eq!(result.scalars.get("result").unwrap().value, Some(0.0));
    }

    #[test]
    fn test_now_function() {
        let mut model = ParsedModel::new();
        model.add_scalar(
            "now_len".to_string(),
            Variable::new("now_len".to_string(), None, Some("=LEN(NOW())".to_string())),
        );
        let calculator = ArrayCalculator::new(model);
        let result = calculator.calculate_all().expect("Should calculate");
        // NOW() returns "YYYY-MM-DD HH:MM:SS" format = 19 characters
        let val = result.scalars.get("now_len").unwrap().value.unwrap();
        assert_exact!(val, 19.0);
    }
}
