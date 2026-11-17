//! What `forge export` and `forge import` carry over, said in their output:
//! the parts of a model that stay only in the YAML file, and whether every
//! formula of a workbook came in as a formula (roadmap T1-AGENT-FRICTION).

use crate::cli::console::outln;
use crate::cli::Console;
use std::path::Path;

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

/// After `forge import`: the formulas brought in as values, or that every
/// formula was kept as a formula.
#[cfg(feature = "cli")]
pub(super) fn print_import_formulas(
    c: &mut Console<'_>,
    model: &crate::types::ParsedModel,
    as_values: &[String],
) {
    use colored::Colorize;
    if as_values.is_empty() {
        let kept = model
            .scalars
            .values()
            .filter(|v| v.formula.is_some())
            .count()
            + model
                .tables
                .values()
                .map(|t| t.row_formulas.len())
                .sum::<usize>();
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
        "⚠️  Imported as values, because Forge cannot express these formulas:".yellow()
    );
    for warning in as_values {
        outln!(c, "   - {warning}");
    }
    outln!(c);
}

#[cfg(test)]
mod tests {
    use super::has_comment;

    #[test]
    fn a_hash_in_quotes_or_a_word_is_not_a_comment() {
        assert!(has_comment("# a note"));
        assert!(has_comment("price: 10  # in dollars"));
        assert!(!has_comment("label: \"item #3\""));
        assert!(!has_comment("color: '#ff0000'"));
        assert!(!has_comment("code: abc#def"));
    }
}
