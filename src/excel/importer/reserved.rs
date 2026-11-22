//! A workbook exported from a model with a Bayesian network holds the
//! network's results as numbers (`bayesian_network.any_year.yes`), as it holds
//! a game's. A model's `bayesian_network` is its network, so a group of that
//! name would be read back as one: the results come back under
//! [`RENAMED`] instead, and every formula that reads them is rewritten.

use crate::bayesian::written::SECTION;
use crate::types::ParsedModel;

/// The group the network's results come back as.
pub(super) const RENAMED: &str = "network_results";

/// Whether `c` can be part of a name in a formula (as the calculator reads one).
fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.' || c == '@'
}

/// `formula` with each name under `bayesian_network.` moved under
/// [`RENAMED`], outside text in quotes.
fn rename_in(formula: &str) -> String {
    let from = format!("{SECTION}.");
    let mut out = String::with_capacity(formula.len());
    let mut quote: Option<char> = None;
    let mut previous: Option<char> = None;
    let mut rest = formula;
    while let Some(c) = rest.chars().next() {
        let starts_name = quote.is_none() && !previous.is_some_and(is_name_char);
        if starts_name && rest.starts_with(&from) {
            out.push_str(RENAMED);
            out.push('.');
            rest = &rest[from.len()..];
            previous = Some('.');
            continue;
        }
        quote = match quote {
            None if c == '"' || c == '\'' => Some(c),
            Some(q) if q == c => None,
            other => other,
        };
        out.push(c);
        previous = Some(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// Move the network's results under [`RENAMED`], and say so.
pub(super) fn rename_network_results(model: &mut ParsedModel, warnings: &mut Vec<String>) {
    let from = format!("{SECTION}.");
    if !model.scalars.keys().any(|name| name.starts_with(&from)) {
        return;
    }
    model.scalars = std::mem::take(&mut model.scalars)
        .into_iter()
        .map(|(name, mut var)| {
            let name = name
                .strip_prefix(&from)
                .map_or_else(|| name.clone(), |rest| format!("{RENAMED}.{rest}"));
            var.path.clone_from(&name);
            var.formula = var.formula.as_deref().map(rename_in);
            (name, var)
        })
        .collect();
    for table in model.tables.values_mut() {
        for formula in table.row_formulas.values_mut() {
            *formula = rename_in(formula);
        }
    }
    warnings.push(format!(
        "{SECTION}: the network's results, numbers as of the export, are imported as group \
         '{RENAMED}', since a model's {SECTION} is its network; the formulas that read them read \
         {RENAMED}"
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Table, Variable};

    #[test]
    fn a_name_under_the_network_is_moved_and_nothing_else() {
        for (formula, want) in [
            ("=bayesian_network.a.yes * 2", "=network_results.a.yes * 2"),
            (
                "=MAX(bayesian_network.a.yes, x.bayesian_network.b)",
                "=MAX(network_results.a.yes, x.bayesian_network.b)",
            ),
            (
                "=IF(my_bayesian_network.a > 0, \"bayesian_network.a\", 'é')",
                "=IF(my_bayesian_network.a > 0, \"bayesian_network.a\", 'é')",
            ),
            ("=@inc.bayesian_network.a.b", "=@inc.bayesian_network.a.b"),
        ] {
            assert_eq!(rename_in(formula), want, "{formula}");
        }
    }

    #[test]
    fn the_results_their_readers_and_a_warning() {
        let mut model = ParsedModel::new();
        let scalar = |name: &str, formula: Option<&str>| {
            Variable::new(name.to_string(), Some(1.0), formula.map(str::to_string))
        };
        model.add_scalar(
            "bayesian_network.q.yes".to_string(),
            scalar("bayesian_network.q.yes", None),
        );
        model.add_scalar(
            "loss.expected".to_string(),
            scalar("loss.expected", Some("=bayesian_network.q.yes * 10")),
        );
        let mut table = Table::new("t".to_string());
        table.add_row_formula("y".to_string(), "=bayesian_network.q.yes".to_string());
        model.add_table(table);
        let mut warnings = Vec::new();
        rename_network_results(&mut model, &mut warnings);
        assert_eq!(
            model.scalars.keys().collect::<Vec<_>>(),
            ["network_results.q.yes", "loss.expected"]
        );
        assert_eq!(
            model.scalars["network_results.q.yes"].path,
            "network_results.q.yes"
        );
        assert_eq!(
            model.scalars["loss.expected"].formula.as_deref(),
            Some("=network_results.q.yes * 10")
        );
        assert_eq!(
            model.tables["t"].row_formulas["y"],
            "=network_results.q.yes"
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("bayesian_network: the network's results"));
        // A model without them is left as it is, with no warning
        let mut plain = ParsedModel::new();
        plain.add_scalar("a".to_string(), scalar("a", None));
        rename_network_results(&mut plain, &mut warnings);
        assert_eq!(warnings.len(), 1);
    }
}
