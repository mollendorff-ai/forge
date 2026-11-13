//! What calculating a model works out before it evaluates anything, kept so
//! that calculating the same model again (a simulation iteration, a
//! sensitivity step) does not work it out again.
//!
//! Everything here depends only on the model's names and formulas, never on
//! its values. Parsed formulas are keyed by their text. The calculation order,
//! the dependencies of each cell and how names resolve are keyed by a
//! fingerprint of every name and formula in the model, so a model with any
//! other name or formula gets its own entry and the same results as a fresh
//! calculation.

use super::parser::Expr;
use super::Cell;
use crate::types::ParsedModel;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Every formula cell in calculation order, with the formula cells it reads.
pub(super) type Order = Rc<Vec<(Cell, Vec<Cell>)>>;

/// The order and resolution of one model structure.
#[derive(Debug, Default)]
pub(super) struct Structure {
    /// Every formula cell in calculation order, with the formula cells it reads
    pub order: Option<Order>,
    /// For a section (`None` for row formulas): each short name and the full
    /// name it resolves to there
    pub short_names: HashMap<Option<String>, Rc<Vec<(String, String)>>>,
    /// (formula, section, table) whose names were checked and are not ambiguous
    pub unambiguous: HashSet<(String, Option<String>, Option<String>)>,
    /// What each cell of `order` worked out, by position
    pub steps: Vec<Step>,
}

/// What one formula cell worked out, kept by its position in the order so
/// that a later calculation finds it without looking up the formula's text.
#[derive(Debug, Default, Clone)]
pub(super) struct Step {
    /// The parsed formula
    pub ast: Option<Rc<Expr>>,
    /// Whether its names were checked and are not ambiguous
    pub unambiguous: bool,
}

/// Shared by every calculation of models with the same formulas.
#[derive(Debug, Default)]
pub struct CalcPlan {
    /// Parsed formulas by text (a syntax error is kept as its message)
    asts: RefCell<HashMap<String, Result<Rc<Expr>, String>>>,
    /// Order and name resolution, by model fingerprint
    structures: RefCell<HashMap<u64, Structure>>,
    /// Plans for the included files, by namespace
    includes: RefCell<HashMap<String, Rc<Self>>>,
}

impl CalcPlan {
    /// A plan to share between calculations of models with the same formulas.
    #[must_use]
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The parsed formula (without its `=`), parsing it the first time.
    pub(super) fn ast(
        &self,
        formula: &str,
        parse: impl FnOnce() -> Result<Expr, String>,
    ) -> Result<Rc<Expr>, String> {
        if let Some(found) = self.asts.borrow().get(formula) {
            return found.clone();
        }
        let parsed = parse().map(Rc::new);
        self.asts
            .borrow_mut()
            .insert(formula.to_string(), parsed.clone());
        parsed
    }

    /// The plan for an included file.
    pub(super) fn include(&self, namespace: &str) -> Rc<Self> {
        Rc::clone(
            self.includes
                .borrow_mut()
                .entry(namespace.to_string())
                .or_default(),
        )
    }

    /// Run `f` on the structure entry of a model fingerprint.
    pub(super) fn structure<R>(&self, fingerprint: u64, f: impl FnOnce(&mut Structure) -> R) -> R {
        f(self.structures.borrow_mut().entry(fingerprint).or_default())
    }
}

/// A hash of every name and formula of a model (not its values), including
/// which scalars have a formula: what the order and name resolution depend on.
pub(super) fn fingerprint(model: &ParsedModel) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (name, var) in &model.scalars {
        name.hash(&mut hasher);
        var.formula.hash(&mut hasher);
    }
    let mut tables: Vec<&String> = model.tables.keys().collect();
    tables.sort();
    for name in tables {
        let table = &model.tables[name];
        name.hash(&mut hasher);
        table
            .columns
            .keys()
            .collect::<std::collections::BTreeSet<_>>()
            .hash(&mut hasher);
        table
            .row_formulas
            .iter()
            .collect::<std::collections::BTreeMap<_, _>>()
            .hash(&mut hasher);
    }
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::super::ArrayCalculator;
    use super::CalcPlan;
    use crate::types::ParsedModel;

    fn model(text: &str) -> ParsedModel {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        std::fs::write(&path, text).unwrap();
        crate::parser::parse_model(&path).unwrap()
    }

    fn fresh(m: &ParsedModel) -> Result<String, String> {
        ArrayCalculator::new(m.clone())
            .calculate_all()
            .map(|r| format!("{:?} {:?}", r.scalars, r.tables))
            .map_err(|e| e.to_string())
    }

    fn planned(m: &ParsedModel, plan: &std::rc::Rc<CalcPlan>) -> Result<String, String> {
        ArrayCalculator::with_plan(m.clone(), std::rc::Rc::clone(plan))
            .calculate_all()
            .map(|r| format!("{:?} {:?}", r.scalars, r.tables))
            .map_err(|e| e.to_string())
    }

    const BASE: &str = "_forge_version: \"5.1.0\"\nt:\n  x: [1, 2, 3]\n  y: \"=x * rate\"\n  z: \"=y + t.x\"\na:\n  rate: 2\n  total:\n    formula: \"=SUM(t.z) + rate\"\nb:\n  rate: 5\n";

    /// One plan, many models: every result is what a fresh calculation
    /// gives, whether the model differs in values, in a formula, or in names.
    #[test]
    fn a_shared_plan_gives_the_results_of_a_fresh_calculation() {
        let plan = CalcPlan::new();
        let variants = [
            BASE.to_string(),
            BASE.replace("rate: 2", "rate: 7"),
            BASE.replace("=SUM(t.z) + rate", "=MAX(t.z) - rate"),
            BASE.replace("b:\n  rate: 5\n", ""),
            BASE.replace("=x * rate", "=x * a.rate"),
            BASE.to_string(),
        ];
        for text in &variants {
            let m = model(text);
            assert_eq!(planned(&m, &plan), fresh(&m), "{text}");
            assert_eq!(planned(&m, &plan), fresh(&m), "second use: {text}");
        }
        // `rate` in a table formula is ambiguous (a.rate, b.rate): the same
        // error every time, and the variant without b still calculates
        assert!(fresh(&model(BASE)).unwrap_err().contains("ambiguous"));
    }

    #[test]
    fn errors_are_the_same_with_a_plan() {
        let plan = CalcPlan::new();
        for text in [
            "_forge_version: \"5.1.0\"\na:\n  x:\n    formula: \"=y + 1\"\n  y:\n    formula: \"=x + 1\"\n",
            "_forge_version: \"5.1.0\"\na:\n  x:\n    formula: \"=nope * 2\"\n  y:\n    formula: \"=x + 1\"\n",
            "_forge_version: \"5.1.0\"\na:\n  x:\n    formula: \"=(1 + \"\n",
        ] {
            let m = model(text);
            let expected = fresh(&m);
            assert!(expected.is_err(), "{text}");
            assert_eq!(planned(&m, &plan), expected);
            assert_eq!(planned(&m, &plan), expected, "second use");
        }
    }
}
