//! A formula's `format:` against its value: the check that runs after a
//! calculation, when the value is known.

use super::evaluator;
use crate::error::{ForgeError, ForgeResult};
use crate::types::{ColumnValue, ParsedModel};

/// A formula's `format:` must be a date code where its value is a date and a
/// number code where it is a number (ADR-061); its value is known only once
/// calculated.
pub(super) fn check_formula_formats(model: &ParsedModel) -> ForgeResult<()> {
    use evaluator::text::number_format::{check_format, FormatKind};
    let mismatch = |path: &str, format: &str, is_date: bool| -> ForgeResult<()> {
        let kind = check_format(format).map_err(ForgeError::Eval)?;
        let (wrong, value) = match (kind, is_date) {
            (FormatKind::Date, false) => ("a date format", "a number"),
            (FormatKind::Number, true) => ("a number format", "a date"),
            _ => return Ok(()),
        };
        Err(ForgeError::Eval(format!(
            "'{path}.format': \"{format}\" is {wrong}, and the value of '{path}' is {value}"
        )))
    };
    for (path, var) in &model.scalars {
        if let (Some(format), Some(_), Some(_)) = (&var.metadata.format, &var.formula, var.value) {
            mismatch(path, format, var.is_date)?;
        }
    }
    for (name, table) in &model.tables {
        for (column, metadata) in &table.formula_metadata {
            let (Some(format), Some(col)) = (&metadata.format, table.columns.get(column)) else {
                continue;
            };
            let is_date = match &col.values {
                ColumnValue::Date(_) => true,
                ColumnValue::Text(v) => {
                    !v.is_empty() && v.iter().all(|t| evaluator::excel_date_serial(t).is_some())
                },
                _ => false,
            };
            mismatch(&format!("{name}.{column}"), format, is_date)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, Metadata, ParsedModel, Table, Variable};

    fn model(body: &str) -> ParsedModel {
        let text = format!("_forge_version: \"5.4.0\"\n{body}");
        crate::parser::parse_model_text("m.yaml", &text, &std::collections::HashMap::new()).unwrap()
    }

    fn calculate_error(model: ParsedModel) -> String {
        ArrayCalculator::new(model)
            .calculate_all()
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_format_of_the_wrong_kind_for_its_value_is_refused_after_the_calculation() {
        for (body, want) in [
            (
                "s:\n  a: {formula: \"=1\", format: \"yyyy-mm-dd\"}\n",
                "'s.a.format': \"yyyy-mm-dd\" is a date format, and the value of 's.a' is a number",
            ),
            (
                "s:\n  a: {formula: \"=DATE(2026, 1, 1)\", format: \"0.00\"}\n",
                "'s.a.format': \"0.00\" is a number format, and the value of 's.a' is a date",
            ),
            (
                "t:\n  x: [1, 2]\n  r: {formula: \"=x * 2\", format: \"yyyy-mm-dd\"}\n",
                "'t.r.format': \"yyyy-mm-dd\" is a date format, and the value of 't.r' is a number",
            ),
            (
                "t:\n  x: [1, 2]\n  r: {formula: \"=DATE(2026, 1, x)\", format: \"0.00\"}\n",
                "'t.r.format': \"0.00\" is a number format, and the value of 't.r' is a date",
            ),
        ] {
            let want = format!("Formula evaluation error: {want}");
            assert_eq!(calculate_error(model(body)), want, "{body}");
            // As the outputs of a simulation, which asks for none of them
            let err = ArrayCalculator::new(model(body))
                .calculate_outputs(&[], &[])
                .unwrap_err();
            assert_eq!(err.to_string(), want, "{body}");
        }
    }

    #[test]
    fn a_format_in_a_model_built_in_code_that_the_parser_would_refuse_is_refused() {
        let mut m = ParsedModel::new();
        let mut scalar = Variable::new("s.a".to_string(), None, Some("=1".to_string()));
        scalar.metadata.format = Some("nope".to_string());
        m.scalars.insert("s.a".to_string(), scalar);
        assert_eq!(
            calculate_error(m),
            "Formula evaluation error: TEXT: the format \"nope\" is not one Forge reads (write the text n in quotes)"
        );
    }

    #[test]
    fn a_column_of_dates_with_a_number_format_is_refused_and_a_format_with_no_column_is_not() {
        let mut m = ParsedModel::new();
        let mut table = Table::new("t".to_string());
        table.add_column(Column::new(
            "d".to_string(),
            ColumnValue::Date(vec!["2026-01-01".to_string()]),
        ));
        let metadata = Metadata {
            format: Some("0.00".to_string()),
            ..Metadata::default()
        };
        // A formula column that has no values, and one whose values are dates
        table
            .formula_metadata
            .insert("missing".to_string(), metadata.clone());
        table.formula_metadata.insert("d".to_string(), metadata);
        m.add_table(table);
        assert_eq!(
            calculate_error(m),
            "Formula evaluation error: 't.d.format': \"0.00\" is a number format, and the value of 't.d' is a date"
        );
    }
}
