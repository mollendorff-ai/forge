//! Setting a model's inputs: a scenario's override of a scalar, a date, one
//! row of an uncertain column, and the refusal to vary a date.

use super::{parse_row_input_name, row_input_name, Column, ColumnValue, ParsedModel, Variable};

impl ParsedModel {
    /// An override of `@namespace.name`, made in the included file the
    /// namespace names; `None` when `name` is not an include's.
    fn override_in_include(
        &mut self,
        name: &str,
        apply: impl FnOnce(&mut Self, &str) -> Result<(), String>,
    ) -> Option<Result<(), String>> {
        let rest = name.strip_prefix('@')?;
        Some(rest.split_once('.').map_or_else(
            || {
                Err(format!(
                    "Scenario override '{name}' needs a name after the include, as in @costs.flour"
                ))
            },
            |(namespace, inner)| {
                let resolved = self.resolved_includes.get_mut(namespace).ok_or_else(|| {
                    format!(
                        "Scenario override '{name}': the model has no `_includes` entry with `as: {namespace}`"
                    )
                })?;
                apply(&mut resolved.model, inner)
                    .map_err(|e| format!("{e} (in the file included as '{namespace}')"))
            },
        ))
    }

    /// The scalar a name stands for, with its full name
    fn scalar_to_override(&mut self, name: &str) -> Result<(String, &mut Variable), String> {
        let resolved = self.resolve_scalar_name(name)?;
        // `resolve_scalar_name` only gives the name of a scalar there is
        let missing = format!("Scenario override '{name}' does not name a scalar in the model.");
        self.scalars
            .get_mut(&resolved)
            .map(|scalar| (resolved, scalar))
            .ok_or(missing)
    }

    /// Give a scalar a scenario's value or formula.
    ///
    /// `name` is a full dotted path, a bare name that ends exactly one grouped
    /// scalar, or `@namespace.name` for a scalar of an included file (which is
    /// then calculated with the new value).
    ///
    /// # Errors
    ///
    /// Returns an error if the name matches no scalar, matches several, or
    /// names an include the model does not have.
    pub fn override_scalar(
        &mut self,
        name: &str,
        value: Option<f64>,
        formula: Option<String>,
    ) -> Result<(), String> {
        if let Some(done) = self.override_in_include(name, |model, inner| {
            model.override_scalar(inner, value, formula.clone())
        }) {
            return done;
        }
        let (resolved, scalar) = self.scalar_to_override(name)?;
        if scalar.is_date && scalar.formula.is_none() && value.is_some() {
            return Err(format!(
                "'{resolved}' is a date ({}), so it is set to a date, not a number (ADR-060)",
                scalar.date_text().unwrap_or_default()
            ));
        }
        scalar.value = value;
        scalar.formula = formula;
        Ok(())
    }

    /// Give an uncertain input its value for one simulated future: a scalar
    /// (as [`Self::override_scalar`]), or one row of a table column whose
    /// formula is an `MC.*` distribution, named as [`row_input_name`] writes
    /// it (`years.shock (row 2)`). The first row set turns that column from
    /// its formula into a column of numbers; every row of it is then set by
    /// the caller, which draws one value per row.
    ///
    /// # Errors
    ///
    /// Returns an error if the name matches no scalar, no table, no uncertain
    /// column of it or no row of it.
    pub fn set_input(&mut self, name: &str, value: f64) -> Result<(), String> {
        let Some((table, column, row)) = parse_row_input_name(name) else {
            return self.override_scalar(name, Some(value), None);
        };
        if let Some(rest) = table.strip_prefix('@') {
            let (namespace, table) = rest.split_once('.').unwrap_or((rest, ""));
            let resolved = self.resolved_includes.get_mut(namespace).ok_or_else(|| {
                format!("'{name}': the model has no `_includes` entry with `as: {namespace}`")
            })?;
            return resolved
                .model
                .set_input(&row_input_name(table, column, row), value)
                .map_err(|e| format!("{e} (in the file included as '{namespace}')"));
        }
        let t = self
            .tables
            .get_mut(table)
            .ok_or_else(|| format!("'{name}': the model has no table '{table}'"))?;
        let rows = t.row_count();
        if row >= rows {
            return Err(format!("'{name}': table '{table}' has {rows} rows"));
        }
        if t.row_formulas.shift_remove(column).is_some() {
            let metadata = t.formula_metadata.shift_remove(column).unwrap_or_default();
            t.add_column(Column::with_metadata(
                column.to_string(),
                ColumnValue::Number(vec![f64::NAN; rows]),
                metadata,
            ));
        }
        match t.columns.get_mut(column).map(|c| &mut c.values) {
            Some(ColumnValue::Number(values)) => {
                values[row] = value;
                Ok(())
            },
            _ => Err(format!(
                "'{name}': '{table}.{column}' is not an uncertain column of the model"
            )),
        }
    }

    /// Set the date input `name` to the date with day number `serial`, as a
    /// scenario does (ADR-060).
    ///
    /// # Errors
    ///
    /// Returns an error if the name matches no scalar, or a scalar that is not
    /// a date.
    pub fn override_date(&mut self, name: &str, serial: f64) -> Result<(), String> {
        if let Some(done) =
            self.override_in_include(name, |model, inner| model.override_date(inner, serial))
        {
            return done;
        }
        let (resolved, scalar) = self.scalar_to_override(name)?;
        if !scalar.is_date || scalar.formula.is_some() {
            return Err(format!(
                "'{resolved}' is not a date input, so a date cannot be given to it (ADR-060)"
            ));
        }
        scalar.value = Some(serial);
        Ok(())
    }

    /// An error if `name` is a date scalar: the commands that set an input to
    /// numbers of their own (sensitivity, goal-seek, break-even, tornado,
    /// simulate) do not vary a date (ADR-060).
    ///
    /// # Errors
    ///
    /// Returns the message, naming the scalar and the command.
    pub fn refuse_date_input(&self, name: &str, command: &str) -> Result<(), String> {
        if let Some((alias, inner)) = name.strip_prefix('@').and_then(|r| r.split_once('.')) {
            return self.resolved_includes.get(alias).map_or(Ok(()), |file| {
                file.model
                    .refuse_date_input(inner, command)
                    .map_err(|e| format!("{e} (in the file included as '{alias}')"))
            });
        }
        let found = self.scalars.get(name).or_else(|| {
            self.resolve_scalar_name(name)
                .ok()
                .and_then(|n| self.scalars.get(&n))
        });
        match found {
            Some(scalar) if scalar.is_date => Err(format!(
                "'{name}' is a date ({}), and {command} varies numbers: varying a date is not \
                 supported (ADR-060)",
                scalar.date_text().unwrap_or_default()
            )),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Column, ColumnValue, Include, Metadata, ResolvedInclude, Table, Variable};

    #[test]
    fn an_override_of_an_include_is_made_there_or_refused_with_its_name() {
        let mut inner = ParsedModel::new();
        inner
            .scalars
            .insert("a.n".into(), Variable::new("a.n".into(), Some(1.0), None));
        let mut day = Variable::new("a.d".into(), Some(46_000.0), None);
        day.is_date = true;
        inner.scalars.insert("a.d".into(), day);
        let mut model = ParsedModel::new();
        let include = Include::new("i.yaml".to_string(), "i".to_string());
        let resolved_path = std::path::PathBuf::from("i.yaml");
        model.resolved_includes.insert(
            "i".into(),
            ResolvedInclude {
                include,
                resolved_path,
                model: inner,
            },
        );
        model.override_scalar("@i.a.n", Some(5.0), None).unwrap();
        model.override_date("@i.a.d", 46_100.0).unwrap();
        let held = &model.resolved_includes["i"].model.scalars;
        assert_eq!(held["a.n"].value, Some(5.0));
        assert_eq!(held["a.d"].value, Some(46_100.0));
        let suffix = "(in the file included as 'i')";
        assert!(model
            .override_date("@i.a.n", 1.0)
            .unwrap_err()
            .ends_with(suffix));
        assert!(model
            .override_scalar("@i.a.d", Some(1.0), None)
            .unwrap_err()
            .ends_with(suffix));
        for err in [
            model.override_scalar("@i", Some(1.0), None).unwrap_err(),
            model.override_date("@i", 1.0).unwrap_err(),
        ] {
            assert!(
                err.contains("needs a name after the include, as in @costs.flour"),
                "{err}"
            );
        }
        let err = model.override_date("@x.a.d", 1.0).unwrap_err();
        assert!(err.contains("no `_includes` entry with `as: x`"), "{err}");
        assert!(model
            .override_date("ghost", 1.0)
            .unwrap_err()
            .contains("ghost"));
        // A bare name finds the date it ends
        let mut when = Variable::new("g.when".into(), Some(1.0), None);
        when.is_date = true;
        model.scalars.insert("g.when".into(), when);
        let err = model.refuse_date_input("when", "tornado").unwrap_err();
        assert!(err.contains("'when' is a date"), "{err}");
    }

    mod uncertain_rows {
        //! One row of a table column whose formula is an `MC.*` distribution is
        //! an input of its own, set for each simulated future by `set_input`.

        use super::*;

        fn model() -> ParsedModel {
            let mut table = Table::new("t".to_string());
            table.add_column(Column::new(
                "n".to_string(),
                ColumnValue::Number(vec![1.0, 2.0, 3.0]),
            ));
            table.add_row_formula("shock".to_string(), "=MC.Normal(0, 1)".to_string());
            table.formula_metadata.insert(
                "shock".to_string(),
                Metadata {
                    unit: Some("%".to_string()),
                    ..Metadata::default()
                },
            );
            table.add_column(Column::new(
                "label".to_string(),
                ColumnValue::Text(vec!["a".into(), "b".into(), "c".into()]),
            ));
            let mut model = ParsedModel::new();
            model.add_table(table);
            model.add_scalar(
                "a.x".to_string(),
                Variable::new("a.x".to_string(), Some(1.0), None),
            );
            model
        }

        #[test]
        fn a_name_is_written_and_read_back() {
            assert_eq!(row_input_name("t", "shock", 1), "t.shock (row 2)");
            assert_eq!(
                parse_row_input_name("t.shock (row 2)"),
                Some(("t", "shock", 1))
            );
            assert_eq!(
                parse_row_input_name("@ns.t.shock (row 11)"),
                Some(("@ns.t", "shock", 10))
            );
            for not_a_row in [
                "a.x",
                "f(x)",
                "t.shock (row 0)",
                "t.shock (row x)",
                "shock (row 1)",
            ] {
                assert_eq!(parse_row_input_name(not_a_row), None, "{not_a_row}");
            }
        }

        #[test]
        fn each_row_is_set_and_the_column_becomes_numbers() {
            let mut model = model();
            for (row, value) in [0.5, -0.25, 2.0].into_iter().enumerate() {
                model
                    .set_input(&row_input_name("t", "shock", row), value)
                    .unwrap();
            }
            let table = &model.tables["t"];
            assert!(!table.row_formulas.contains_key("shock"));
            let values = table.columns["shock"].values.numbers().unwrap();
            assert_eq!(values.as_ref(), [0.5, -0.25, 2.0]);
            // The column keeps its place and its unit
            let names: Vec<&str> = table.column_names().iter().map(|n| n.as_str()).collect();
            assert_eq!(names, ["n", "shock", "label"]);
            assert_eq!(table.columns["shock"].metadata.unit.as_deref(), Some("%"));
        }

        #[test]
        fn a_scalar_is_set_as_an_override() {
            let mut model = model();
            model.set_input("a.x", 7.0).unwrap();
            assert_eq!(model.scalars["a.x"].value, Some(7.0));
        }

        #[test]
        fn a_name_that_matches_nothing_is_refused() {
            let mut model = model();
            let cases = [
                ("u.shock (row 1)", "has no table 'u'"),
                ("t.shock (row 4)", "table 't' has 3 rows"),
                ("t.label (row 1)", "'t.label' is not an uncertain column"),
                (
                    "t.nothing (row 1)",
                    "'t.nothing' is not an uncertain column",
                ),
                (
                    "@rates.t.shock (row 1)",
                    "no `_includes` entry with `as: rates`",
                ),
            ];
            for (name, want) in cases {
                let err = model.set_input(name, 1.0).unwrap_err();
                assert!(err.contains(want), "{name}: {err}");
            }
        }

        #[test]
        fn a_row_of_an_included_file_is_set_there() {
            let mut model = ParsedModel::new();
            model.resolved_includes.insert(
                "rates".to_string(),
                ResolvedInclude {
                    include: Include::new("rates.yaml".to_string(), "rates".to_string()),
                    resolved_path: std::path::PathBuf::from("rates.yaml"),
                    model: self::model(),
                },
            );
            model.set_input("@rates.t.shock (row 3)", 4.0).unwrap();
            let included = &model.resolved_includes["rates"].model.tables["t"];
            let values = included.columns["shock"].values.numbers().unwrap();
            assert!(values[0].is_nan());
            assert!((values[2] - 4.0).abs() <= 0.0);
            let err = model.set_input("@rates.u.shock (row 1)", 1.0).unwrap_err();
            assert!(err.ends_with("(in the file included as 'rates')"), "{err}");
            // A table must follow the include's name
            let err = model.set_input("@rates.shock (row 1)", 1.0).unwrap_err();
            assert!(err.contains("has no table ''"), "{err}");
        }

        /// A date in an included file is refused as one in the model is
        /// (roadmap T1-VARY-INCLUDED); a number there, or an include that is
        /// not there, is not this check's to refuse.
        #[test]
        fn a_date_in_an_included_file_is_refused() {
            let mut included = self::model();
            let mut day = Variable::new("a.closing".to_string(), Some(46_000.0), None);
            day.is_date = true;
            included.scalars.insert("a.closing".to_string(), day);
            let mut model = ParsedModel::new();
            model.resolved_includes.insert(
                "deal".to_string(),
                ResolvedInclude {
                    include: Include::new("deal.yaml".to_string(), "deal".to_string()),
                    resolved_path: std::path::PathBuf::from("deal.yaml"),
                    model: included,
                },
            );
            let err = model
                .refuse_date_input("@deal.a.closing", "tornado")
                .unwrap_err();
            assert!(err.contains("'a.closing' is a date"), "{err}");
            assert!(err.ends_with("(in the file included as 'deal')"), "{err}");
            assert!(model.refuse_date_input("@deal.a.x", "tornado").is_ok());
            assert!(model
                .refuse_date_input("@other.a.closing", "tornado")
                .is_ok());
        }
    }
}
