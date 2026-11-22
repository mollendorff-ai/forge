//! How a name in a formula resolves to a scalar: its own section first,
//! then a short name unique in the model; an ambiguous one is an error.

use super::order::{is_name_char, strip_string_literals};
use super::{evaluator, plan, ArrayCalculator};
use crate::error::{ForgeError, ForgeResult};
use crate::types::Table;
use std::collections::HashMap;
use std::rc::Rc;

/// What a name used in a formula refers to
pub(super) enum ScalarName {
    /// Exactly one scalar, by full name
    Found(String),
    /// Several scalars in different sections, sorted by full name
    Ambiguous(Vec<String>),
    /// No scalar
    Missing,
}

/// See [`ArrayCalculator::sections`]
pub(super) struct Sections {
    own: HashMap<String, Rc<evaluator::NameMap<String>>>,
    unique: Rc<evaluator::NameMap<String>>,
}

impl ArrayCalculator {
    /// Resolve a name used in a formula to a full scalar name.
    ///
    /// A bare name such as `revenue` in a formula of section `budget` means
    /// `budget.revenue` if that exists, then a top-level scalar `revenue`, then
    /// the only scalar in any section with that last part. Dotted names must
    /// match exactly. Dependency ordering and evaluation both use this, so they
    /// agree, and the result never depends on hash map order.
    pub(super) fn resolve_scalar_name(&self, word: &str, section: Option<&str>) -> ScalarName {
        if let Some(section) = section {
            if !word.contains('.') {
                let scoped = format!("{section}.{word}");
                if self.model.scalars.contains_key(&scoped) {
                    return ScalarName::Found(scoped);
                }
            }
        }
        if self.model.scalars.contains_key(word) {
            return ScalarName::Found(word.to_string());
        }
        if word.contains('.') {
            return ScalarName::Missing;
        }
        let by_last_part = self.by_last_part.get_or_init(|| {
            let mut map: HashMap<String, Vec<String>> = HashMap::new();
            for name in self.model.scalars.keys() {
                if let Some((_, last)) = name.rsplit_once('.') {
                    map.entry(last.to_string()).or_default().push(name.clone());
                }
            }
            for names in map.values_mut() {
                names.sort();
            }
            map
        });
        match by_last_part.get(word).map(Vec::as_slice) {
            None | Some([]) => ScalarName::Missing,
            Some([only]) => ScalarName::Found(only.clone()),
            Some(several) => ScalarName::Ambiguous(several.to_vec()),
        }
    }

    /// Return an error if a bare name in `formula` could mean several scalars.
    pub(super) fn check_ambiguous_names(
        &self,
        formula: &str,
        section: Option<&str>,
        table: &Table,
    ) -> ForgeResult<()> {
        self.step_cached(
            |step| step.unambiguous.then_some(()),
            |step, ()| step.unambiguous = true,
            || self.check_ambiguous_names_by_text(formula, section, table),
        )
    }

    pub(super) fn check_ambiguous_names_by_text(
        &self,
        formula: &str,
        section: Option<&str>,
        table: &Table,
    ) -> ForgeResult<()> {
        let key = (
            formula.to_string(),
            section.map(str::to_string),
            Some(table.name.clone()),
        );
        if self
            .plan
            .structure(self.fingerprint, |s| s.unambiguous.contains(&key))
        {
            return Ok(());
        }
        self.find_ambiguous_names(formula, section, table)?;
        self.plan
            .structure(self.fingerprint, |s| s.unambiguous.insert(key));
        Ok(())
    }

    pub(super) fn find_ambiguous_names(
        &self,
        formula: &str,
        section: Option<&str>,
        table: &Table,
    ) -> ForgeResult<()> {
        let stripped = strip_string_literals(formula);
        for word in stripped.split(|c: char| !is_name_char(c)) {
            // A bare name is a column of the table first: a data column, or a
            // formula column (which may not be calculated yet, when the two
            // are calculated together row by row)
            if word.is_empty()
                || word.contains('.')
                || table.columns.contains_key(word)
                || table.row_formulas.contains_key(word)
            {
                continue;
            }
            if let ScalarName::Ambiguous(matches) = self.resolve_scalar_name(word, section) {
                return Err(ForgeError::Eval(format!(
                    "Name '{word}' in formula '{formula}' is ambiguous: it matches {}. \
                     Use the full name.",
                    matches.join(", ")
                )));
            }
        }
        Ok(())
    }

    /// The model's scalars by section, and the last parts that name one
    /// scalar only and no top-level scalar.
    pub(super) fn sections(&self) -> &Sections {
        self.sections.get_or_init(|| {
            let mut own: HashMap<String, evaluator::NameMap<String>> = HashMap::new();
            for name in self.model.scalars.keys() {
                if let Some((section, last)) = name.rsplit_once('.') {
                    own.entry(section.to_string())
                        .or_default()
                        .insert(last.to_string(), name.clone());
                }
            }
            let own = own.into_iter().map(|(k, v)| (k, Rc::new(v))).collect();
            let mut unique = evaluator::NameMap::default();
            for name in self.model.scalars.keys() {
                if let Some((_, last)) = name.rsplit_once('.') {
                    if let ScalarName::Found(full) = self.resolve_scalar_name(last, None) {
                        if full != last {
                            unique.insert(last.to_string(), full);
                        }
                    }
                }
            }
            Sections {
                own,
                unique: Rc::new(unique),
            }
        })
    }

    /// The short names (`price` for `summary.price`) a formula of `section`
    /// may use, each with the scalar it resolves to there, as
    /// [`ArrayCalculator::resolve_scalar_name`] resolves it: the section's
    /// own scalars, then the last parts that name one scalar only. And those
    /// of them that are also a name in the context of their own, which are
    /// laid over it.
    pub(super) fn short_names(&self, section: Option<&str>) -> Rc<plan::ShortNames> {
        let key = section.map(str::to_string);
        if let Some(found) = self
            .plan
            .structure(self.fingerprint, |s| s.short_names.get(&key).cloned())
        {
            return found;
        }
        let sections = self.sections();
        let own = section
            .and_then(|section| sections.own.get(section))
            .cloned()
            .unwrap_or_default();
        // A short name of the section hides a top-level scalar of that name;
        // one from elsewhere never does (it resolves to the top-level one). A
        // short name, the last part of a full one, has no `.`, so it is never
        // the name of an included value (`@namespace.path`)
        let mut shadowing: Vec<(String, String)> = own
            .iter()
            .filter(|(short, _)| self.model.scalars.contains_key(*short))
            .map(|(short, full)| (short.clone(), full.clone()))
            .collect();
        shadowing.sort();
        let resolved = Rc::new(plan::ShortNames {
            aliases: Rc::new(evaluator::Aliases {
                own,
                unique: Rc::clone(&sections.unique),
            }),
            shadowing,
        });
        self.plan.structure(self.fingerprint, |s| {
            s.short_names.insert(key, Rc::clone(&resolved));
        });
        resolved
    }
}

#[cfg(test)]
mod tests {
    //! Tests for how bare scalar names in formulas resolve

    use crate::core::array_calculator::ArrayCalculator;
    use crate::types::{Column, ColumnValue, ParsedModel, Table, Variable};

    fn model(scalars: &[(&str, Option<f64>, Option<&str>)]) -> ParsedModel {
        let mut model = ParsedModel::new();
        for (name, value, formula) in scalars {
            model.add_scalar(
                (*name).to_string(),
                Variable::new((*name).to_string(), *value, formula.map(str::to_string)),
            );
        }
        model
    }

    fn calculate(scalars: &[(&str, Option<f64>, Option<&str>)]) -> Result<ParsedModel, String> {
        ArrayCalculator::new(model(scalars))
            .calculate_all()
            .map_err(|e| e.to_string())
    }

    fn value(model: &ParsedModel, name: &str) -> f64 {
        model.scalars[name].value.unwrap()
    }

    #[test]
    fn test_bare_name_uses_own_section_every_time() {
        // Same shape as examples/variance.yaml: two sections with the same names
        let scalars = [
            ("budget.revenue", Some(1000.0), None),
            ("budget.cogs", Some(400.0), None),
            ("budget.gross_profit", None, Some("=revenue - cogs")),
            ("actual.revenue", Some(950.0), None),
            ("actual.cogs", Some(420.0), None),
            ("actual.gross_profit", None, Some("=revenue - cogs")),
        ];
        // Hash map order changes between runs, so repeat
        for _ in 0..25 {
            let result = calculate(&scalars).unwrap();
            assert!((value(&result, "budget.gross_profit") - 600.0).abs() < 1e-9);
            assert!((value(&result, "actual.gross_profit") - 530.0).abs() < 1e-9);
        }
    }

    #[test]
    fn test_ambiguous_bare_name_is_an_error() {
        let err = calculate(&[
            ("budget.revenue", Some(1000.0), None),
            ("actual.revenue", Some(950.0), None),
            ("summary.total", None, Some("=revenue * 2")),
        ])
        .unwrap_err();
        assert!(err.contains("ambiguous"), "{err}");
        assert!(err.contains("actual.revenue, budget.revenue"), "{err}");
    }

    #[test]
    fn test_unique_bare_name_from_another_section_is_ordered() {
        for _ in 0..25 {
            let result = calculate(&[
                ("inputs.base", Some(10.0), None),
                ("calc.derived", None, Some("=base + 1")),
                ("summary.result", None, Some("=derived * 2")),
            ])
            .unwrap();
            assert!((value(&result, "summary.result") - 22.0).abs() < 1e-9);
        }
    }

    #[test]
    fn test_own_section_beats_top_level_name() {
        let result = calculate(&[
            ("rate", Some(0.1), None),
            ("loan.rate", Some(0.2), None),
            ("loan.cost", None, Some("=rate * 100")),
            ("plain", None, Some("=rate * 100")),
        ])
        .unwrap();
        assert!((value(&result, "loan.cost") - 20.0).abs() < 1e-9);
        assert!((value(&result, "plain") - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_ambiguous_bare_name_in_table_formula_is_an_error() {
        let mut model = model(&[("a.rate", Some(0.1), None), ("b.rate", Some(0.2), None)]);
        let mut table = Table::new("t".to_string());
        table.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        table.add_row_formula("y".to_string(), "=x * rate".to_string());
        model.add_table(table);
        let err = ArrayCalculator::new(model)
            .calculate_all()
            .unwrap_err()
            .to_string();
        assert!(err.contains("ambiguous"), "{err}");
    }

    // Short names are read through the context's aliases rather than laid over
    // it for every formula (T1-SCALAR-SCALING); these hold either way.

    #[test]
    fn test_lambda_parameter_hides_a_short_name() {
        let result = calculate(&[
            ("a.x", Some(5.0), None),
            ("a.y", None, Some("=LAMBDA(x, x * 2)(3) + x")),
        ])
        .unwrap();
        assert!((value(&result, "a.y") - 11.0).abs() < 1e-9);
    }

    #[test]
    fn test_let_name_hides_a_short_name() {
        let result = calculate(&[
            ("a.x", Some(5.0), None),
            ("a.y", None, Some("=LET(x, 1, x + 1)")),
        ])
        .unwrap();
        assert!((value(&result, "a.y") - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_indirect_reads_a_short_name() {
        let result = calculate(&[
            ("a.x", Some(5.0), None),
            ("a.y", None, Some("=INDIRECT(\"x\") * 2")),
        ])
        .unwrap();
        assert!((value(&result, "a.y") - 10.0).abs() < 1e-9);
    }

    #[test]
    fn test_short_name_reads_a_value_calculated_earlier() {
        let result = calculate(&[
            ("a.x", None, Some("=2 + 3")),
            ("a.y", None, Some("=x * 2")),
            ("b.z", None, Some("=y + 1")),
        ])
        .unwrap();
        assert!((value(&result, "a.y") - 10.0).abs() < 1e-9);
        assert!((value(&result, "b.z") - 11.0).abs() < 1e-9);
    }

    #[test]
    fn test_column_hides_a_short_name_in_a_row_formula() {
        let mut model = model(&[("a.x", Some(100.0), None)]);
        let mut table = Table::new("t".to_string());
        table.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![1.0, 2.0]),
        ));
        table.add_row_formula("y".to_string(), "=x * 2".to_string());
        model.add_table(table);
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(
            result.tables["t"].columns["y"].values,
            ColumnValue::Number(vec![2.0, 4.0])
        );
    }

    #[test]
    fn test_row_formula_reads_a_short_name() {
        let mut model = model(&[("a.rate", Some(0.5), None)]);
        let mut table = Table::new("t".to_string());
        table.add_column(Column::new(
            "x".to_string(),
            ColumnValue::Number(vec![2.0, 4.0]),
        ));
        table.add_row_formula("y".to_string(), "=x * rate".to_string());
        model.add_table(table);
        let result = ArrayCalculator::new(model).calculate_all().unwrap();
        assert_eq!(
            result.tables["t"].columns["y"].values,
            ColumnValue::Number(vec![1.0, 2.0])
        );
    }

    mod scalar_scaling {
        //! `forge calculate` took time growing with the cube of the number of
        //! scalars: each formula laid every short name of the model over the
        //! context, and checked each against every name laid before it. 1,000
        //! scalars took 0.3 s, 4,000 took 14 s, and 26,600 (the generated cases of
        //! ADR-062) did not finish in 10 minutes. Roadmap T1-SCALAR-SCALING.
        //! The short names of each section were also worked out by going through
        //! every scalar, so a model of many small sections grew with the square.

        use crate::cli::calculate_core;
        use std::fmt::Write as _;
        use std::time::{Duration, Instant};

        fn model(n: usize, chained: bool) -> String {
            let mut text = String::from("_forge_version: \"5.4.0\"\nt:\n  c0: 1\n");
            for i in 1..n {
                let formula = if chained {
                    format!("=c{} + 1", i - 1)
                } else {
                    format!("=ROUND({i} * 1.5, 2)")
                };
                writeln!(text, "  c{i}: {{formula: \"{formula}\"}}").unwrap();
            }
            text
        }

        /// Under 2 s in a release build; a debug build is slower, so it gets ten
        /// times as long, which the old cubic time still exceeded many times over.
        fn limit() -> Duration {
            // 2 s, and 18 more in a debug build
            Duration::from_secs(2 + 18 * u64::from(cfg!(debug_assertions)))
        }

        fn time(n: usize, chained: bool) -> (Duration, Option<f64>) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("many.yaml");
            std::fs::write(&path, model(n, chained)).unwrap();
            let start = Instant::now();
            let result = calculate_core(&path, true, None).unwrap();
            (start.elapsed(), result.scalars[&format!("t.c{}", n - 1)])
        }

        #[test]
        fn ten_thousand_scalars_calculate_in_time() {
            let (took, last) = time(10_000, false);
            assert!(took < limit(), "10,000 scalars took {took:?}");
            assert_eq!(last, Some(14998.5));
        }

        #[test]
        fn ten_thousand_chained_scalars_calculate_in_time() {
            let (took, last) = time(10_000, true);
            assert!(took < limit(), "10,000 chained scalars took {took:?}");
            assert_eq!(last, Some(10000.0));
        }

        #[test]
        fn a_thousand_sections_of_ten_calculate_in_time() {
            let mut text = String::from("_forge_version: \"5.4.0\"\n");
            for section in 0..1000 {
                writeln!(text, "s{section}:\n  v0: {{formula: \"={section} + 0\"}}").unwrap();
                for j in 1..10 {
                    writeln!(text, "  v{j}: {{formula: \"=v{} + 1\"}}", j - 1).unwrap();
                }
            }
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("sections.yaml");
            std::fs::write(&path, text).unwrap();
            let start = Instant::now();
            let result = calculate_core(&path, true, None).unwrap();
            let took = start.elapsed();
            assert!(took < limit(), "1,000 sections took {took:?}");
            assert_eq!(result.scalars["s999.v9"], Some(1008.0));
            assert_eq!(result.scalars["s0.v9"], Some(9.0));
        }
    }
}
