//! What a model states its results should be (`expect:`, model format 5.6.0,
//! ADR-074): at the top of the file for the file as written, and in each
//! scenario for that scenario. Each is a number, or a formula over numbers
//! only (`=2/11`): an expectation is stated, not worked out by the model.

use crate::core::array_calculator::evaluator::{evaluate_cell, EvalContext};
use crate::core::array_calculator::{parser, tokenizer};
use crate::error::{ForgeError, ForgeResult};
use crate::types::Expected;
use yaml_serde::Value;

/// The expectations written at `place` (`expect` or `scenarios.<name>.expect`),
/// in the file's order.
///
/// # Errors
///
/// Returns an error, with its place, if `value` is not a mapping, a name is
/// not text, or a value is neither a number nor a formula over numbers.
pub fn parse_expect(value: &Value, place: &str) -> ForgeResult<Vec<Expected>> {
    let Value::Mapping(map) = value else {
        return Err(ForgeError::Parse(format!(
            "'{place}' must be a mapping of names to what each should be, such as \
             `kinship.barrier: 0.5`"
        )));
    };
    map.iter()
        .map(|(name, value)| {
            let name = name.as_str().ok_or_else(|| {
                ForgeError::Parse(format!("'{place}' has a name that is not text"))
            })?;
            expected(name, value, &format!("{place}.{name}"))
        })
        .collect()
}

/// One expectation, `name: value`, at `place`.
fn expected(name: &str, value: &Value, place: &str) -> ForgeResult<Expected> {
    let (value, formula) = match value {
        Value::Number(n) => (n.as_f64().unwrap_or_default(), None),
        Value::String(text) if text.trim_start().starts_with('=') => {
            let text = text.trim();
            let value = constant(text)
                .map_err(|why| ForgeError::Parse(format!("'{place}' is {text}: {why}")))?;
            (value, Some(text.to_string()))
        },
        _ => {
            return Err(ForgeError::Parse(format!(
                "'{place}' must be a number, or a formula over numbers such as =2/11"
            )))
        },
    };
    Ok(Expected {
        name: name.to_string(),
        value,
        formula,
    })
}

/// The number a formula over numbers gives, as a scalar's formula would give
/// it. A formula that reads a name is refused: the model would check itself.
fn constant(text: &str) -> Result<f64, String> {
    let body = text.trim_start_matches('=').trim();
    let tokens = tokenizer::tokenize(body).map_err(|e| format!("syntax error: {}", e.message))?;
    let read = tokens
        .iter()
        .enumerate()
        .find_map(|(at, token)| match token {
            tokenizer::Token::Identifier(name)
                if tokens.get(at + 1) != Some(&tokenizer::Token::OpenParen) =>
            {
                Some(name.clone())
            },
            _ => None,
        });
    if let Some(name) = read {
        return Err(format!(
            "it reads {name}, and an expectation reads no name: it is stated, not worked out \
             by the model"
        ));
    }
    let ast = parser::parse(tokens).map_err(|e| format!("syntax error: {}", e.message))?;
    let value = evaluate_cell(&ast, body, &EvalContext::new()).map_err(|e| e.message)?;
    value
        .as_number()
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("it gives {value:?}, not a number"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-15, "{a} is not {b}");
    }

    fn read(yaml: &str) -> ForgeResult<Vec<Expected>> {
        parse_expect(
            &yaml_serde::from_str(yaml).unwrap(),
            "scenarios.costly.expect",
        )
    }

    #[test]
    fn numbers_and_formulas_over_numbers_are_read_in_the_files_order() {
        let got =
            read("kinship.threshold: 0.375\nkinship.barrier: =2/11\nequilibria: 3\n").unwrap();
        let names: Vec<&str> = got.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            ["kinship.threshold", "kinship.barrier", "equilibria"]
        );
        same(got[0].value, 0.375);
        assert_eq!(got[0].formula, None);
        same(got[1].value, 2.0 / 11.0);
        assert_eq!(got[1].formula.as_deref(), Some("=2/11"));
        same(got[2].value, 3.0);
    }

    #[test]
    fn a_formula_may_call_a_function_of_numbers() {
        let got = read("x: \"=-19/15\"\ny: =SQRT(16) + 1\n").unwrap();
        same(got[0].value, -19.0 / 15.0);
        same(got[1].value, 5.0);
    }

    #[test]
    fn a_formula_that_reads_a_name_is_refused_with_its_place() {
        let err = read("kinship.threshold: =(T - c) / 2\n")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("'scenarios.costly.expect.kinship.threshold' is =(T - c) / 2: it reads T"),
            "{err}"
        );
        assert!(err.contains("stated, not worked out by the model"), "{err}");
    }

    #[test]
    fn what_is_neither_a_number_nor_a_formula_is_refused() {
        for yaml in ["x: high\n", "x: [1, 2]\n", "x: true\n"] {
            let err = read(yaml).unwrap_err().to_string();
            assert!(
                err.contains(
                    "'scenarios.costly.expect.x' must be a number, or a formula over numbers"
                ),
                "{yaml}: {err}"
            );
        }
    }

    #[test]
    fn a_formula_that_does_not_parse_or_gives_no_number_is_refused() {
        let err = read("x: =2 +\n").unwrap_err().to_string();
        assert!(
            err.contains("'scenarios.costly.expect.x' is =2 +: syntax error"),
            "{err}"
        );
        let err = read("x: =1 / 0\n").unwrap_err().to_string();
        assert!(
            err.contains("'scenarios.costly.expect.x' is =1 / 0:"),
            "{err}"
        );
        let err = read("x: =\"text\"\n").unwrap_err().to_string();
        assert!(err.contains("not a number"), "{err}");
        let err = read("x: =2 $ 3\n").unwrap_err().to_string();
        assert!(err.contains("syntax error"), "{err}");
    }

    fn model(text: &str) -> ForgeResult<crate::types::ParsedModel> {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.yaml");
        std::fs::write(&path, text).unwrap();
        crate::parser::parse_model(&path)
    }

    #[test]
    fn a_model_of_5_6_0_states_what_it_and_each_scenario_should_give() {
        let m = model(
            "_forge_version: \"5.6.0\"\nw: 0.6\nout:\n  twice: {formula: \"=w * 2\"}\n\
             expect:\n  out.twice: 1.2\n  w: =3/5\n\
             scenarios:\n  short:\n    scalars: {w: 0.4}\n    expect: {out.twice: 0.8}\n",
        )
        .unwrap();
        let base: Vec<(&str, f64)> = m
            .expect
            .iter()
            .map(|e| (e.name.as_str(), e.value))
            .collect();
        assert_eq!(base, [("out.twice", 1.2), ("w", 0.6)]);
        assert!(!m.scalars.contains_key("expect.out.twice"));
        let short = &m.scenarios["short"];
        same(short.overrides["w"], 0.4);
        assert_eq!(short.expect[0].name, "out.twice");
        same(short.expect[0].value, 0.8);
    }

    #[test]
    fn a_wrong_expectation_in_a_model_is_refused_with_its_place() {
        let err = model(
            "_forge_version: \"5.6.0\"\nw: 0.6\n\
             scenarios:\n  short:\n    scalars: {w: 0.4}\n    expect: {w: =w}\n",
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("'scenarios.short.expect.w' is =w: it reads w"),
            "{err}"
        );
        let err = model("_forge_version: \"5.6.0\"\nw: 0.6\nexpect: {w: =w * 2}\n")
            .unwrap_err()
            .to_string();
        assert!(err.contains("'expect.w' is =w * 2: it reads w"), "{err}");
    }

    #[test]
    fn before_5_6_0_a_scenarios_expect_is_refused_and_a_group_named_expect_is_a_group() {
        let err = model(
            "_forge_version: \"5.5.0\"\nw: 0.6\n\
             scenarios:\n  short:\n    scalars: {w: 0.4}\n    expect: {w: 0.4}\n",
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains(
                "'scenarios.short.expect' states what the scenario should give, which model \
                 format 5.6.0 adds (ADR-074). Set `_forge_version: \"5.6.0\"` (this file says 5.5.0)"
            ),
            "{err}"
        );
        let m = model("_forge_version: \"5.5.0\"\nexpect:\n  growth: 0.05\n").unwrap();
        assert_eq!(m.expect, Vec::new());
        assert_eq!(m.scalars["expect.growth"].value, Some(0.05));
        // A scenario of the short form, and no scenarios at all, are as before
        model("_forge_version: \"5.5.0\"\nw: 0.6\nscenarios:\n  short: {w: 0.4}\n").unwrap();
    }

    #[test]
    fn expect_must_be_a_mapping_with_names_of_text() {
        let err = read("[1, 2]\n").unwrap_err().to_string();
        assert!(
            err.contains("'scenarios.costly.expect' must be a mapping"),
            "{err}"
        );
        let err = read("1: 2\n").unwrap_err().to_string();
        assert!(err.contains("has a name that is not text"), "{err}");
    }
}
