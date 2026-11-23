//! What `forge export` and `forge import` carry over, said in their output:
//! the parts of a model that stay only in the YAML file, and whether every
//! formula of a workbook came in as a formula (roadmap T1-AGENT-FRICTION).

// Printing what was carried belongs to the command line (feature `cli`)
use std::path::Path;
#[cfg(feature = "cli")]
use {crate::cli::console::outln, crate::cli::Console};

/// Sections that run analyses from the YAML file. Excel has no counterpart,
/// so an export leaves them out (scenarios are reported by the exporter).
const ANALYSIS_SECTIONS: [&str; 6] = [
    "monte_carlo",
    "tornado",
    "decision_tree",
    "real_options",
    "bootstrap",
    "bayesian_network",
];

/// The parts of the model at `input` that an export does not write: analysis
/// sections, `better:` and comments. Empty when the file cannot be read (the
/// export itself reports that).
#[must_use]
pub(super) fn yaml_only(input: &Path) -> Vec<String> {
    let Ok(text) = crate::parser::read_model_text(input) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let sections: Vec<&str> = ANALYSIS_SECTIONS
        .into_iter()
        .filter(|s| {
            text.lines()
                .any(|l| l.strip_prefix(s).is_some_and(|r| r.starts_with(':')))
        })
        .collect();
    if !sections.is_empty() {
        out.push(format!(
            "{}: not exported; Excel has no equivalent, so run them from the YAML file",
            sections.join(", ")
        ));
    }
    let better = text
        .lines()
        .filter(|l| l.contains("better:") && !l.trim_start().starts_with('#'))
        .count();
    if better > 0 {
        out.push(format!(
            "better: (on {better} line(s)) not exported; forge variance reads it from the YAML file"
        ));
    }
    let comments = text.lines().filter(|l| has_comment(l)).count();
    if comments > 0 {
        out.push(format!(
            "comments ({comments} line(s) with #) not exported; they stay in the YAML file"
        ));
    }
    out
}

/// A YAML comment on this line: a `#` that starts the line, or follows a
/// space outside quotes.
fn has_comment(line: &str) -> bool {
    let mut quote = None;
    let mut previous = ' ';
    for c in line.chars() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, '#') if previous == ' ' || previous == '\t' => return true,
            _ => {},
        }
        previous = c;
    }
    false
}

/// How many formulas `model` has: its scalars' and its columns'.
pub(super) fn formulas_kept(model: &crate::types::ParsedModel) -> usize {
    model
        .scalars
        .values()
        .filter(|v| v.formula.is_some())
        .count()
        + model
            .tables
            .values()
            .map(|t| t.row_formulas.len())
            .sum::<usize>()
}

/// After `forge import`: the formulas brought in as values, or that every
/// formula (`kept` of them) was kept as a formula.
#[cfg(feature = "cli")]
pub(super) fn print_import_formulas(c: &mut Console<'_>, kept: usize, as_values: &[String]) {
    use colored::Colorize;
    if as_values.is_empty() {
        if kept > 0 {
            outln!(
                c,
                "{}",
                format!("✅ All {kept} formula(s) kept as formulas").green()
            );
            outln!(c);
        }
        return;
    }
    outln!(
        c,
        "{}",
        "⚠️  What the import changed or could not keep:".yellow()
    );
    for warning in as_values {
        outln!(c, "   - {warning}");
    }
    outln!(c);
}

#[cfg(test)]
mod tests {
    use super::{formulas_kept, has_comment, print_import_formulas, yaml_only};
    use crate::cli::console::capture;
    use crate::types::ParsedModel;

    /// A file that cannot be read has nothing to say: the export reports it.
    #[test]
    fn a_file_that_cannot_be_read_carries_nothing() {
        assert_eq!(
            yaml_only(std::path::Path::new("no-such-folder/m.yaml")),
            Vec::<String>::new()
        );
    }

    /// A model with no formula has none to say were kept.
    #[test]
    fn a_model_without_formulas_says_nothing_of_kept_formulas() {
        let ((), printed) =
            capture(|c| print_import_formulas(c, formulas_kept(&ParsedModel::new()), &[]));
        assert_eq!(printed.out, "");
    }

    #[test]
    fn a_hash_in_quotes_or_a_word_is_not_a_comment() {
        assert!(has_comment("# a note"));
        assert!(has_comment("price: 10  # in dollars"));
        assert!(!has_comment("label: \"item #3\""));
        assert!(!has_comment("color: '#ff0000'"));
        assert!(!has_comment("code: abc#def"));
    }
}
