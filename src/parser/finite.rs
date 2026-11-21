//! Every number a model holds is finite.
//!
//! YAML reads `.nan`, `.inf` and `-.inf` as numbers (a number past the largest
//! double, `1e400`, it reads as text). No Excel cell holds any of them: a
//! model that did gave answers its own workbook did not (MEDIAN counted a NaN
//! as a value above the others, where the exported text cell was skipped), and
//! wrote cells Excel cannot read. So they are refused where they are written,
//! naming the place, before anything reads the model.

use crate::error::{ForgeError, ForgeResult};
use yaml_serde::Value;

/// Refuse a number that is not finite anywhere in `yaml`.
///
/// A `value:` beside a `formula:` is not checked: it is the result a
/// calculation wrote, which is calculated again, and one an older Forge wrote
/// as `.nan` must not stop the file from opening (the scalar reader drops it).
///
/// # Errors
///
/// Returns the first number that is not finite, with where it is.
pub fn check_finite(yaml: &Value) -> ForgeResult<()> {
    check(yaml, "")
}

fn check(value: &Value, path: &str) -> ForgeResult<()> {
    match value {
        Value::Number(n) => n
            .as_f64()
            .filter(|f| !f.is_finite())
            .map_or(Ok(()), |f| Err(refused(path, f))),
        Value::Sequence(items) => (1..)
            .zip(items)
            .try_for_each(|(row, item)| check(item, &format!("{path} row {row}"))),
        Value::Mapping(map) => {
            let calculated = map.contains_key("formula");
            map.iter().try_for_each(|(key, item)| {
                let key = key.as_str().unwrap_or_default();
                if calculated && key == "value" {
                    return Ok(());
                }
                let inner = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                check(item, &inner)
            })
        },
        Value::Tagged(tagged) => check(&tagged.value, path),
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
    }
}

fn refused(path: &str, number: f64) -> ForgeError {
    let written = if number.is_nan() {
        ".nan"
    } else if number > 0.0 {
        "infinity"
    } else {
        "minus infinity"
    };
    ForgeError::Parse(format!(
        "'{path}' is {written}, which is not a number a model can hold: no Excel cell holds \
         NaN or infinity. Write a finite number"
    ))
}

#[cfg(test)]
mod tests {
    use super::check_finite;

    fn refused(yaml: &str) -> String {
        let yaml: yaml_serde::Value = yaml_serde::from_str(yaml).unwrap();
        check_finite(&yaml).unwrap_err().to_string()
    }

    fn accepted(yaml: &str) {
        let yaml: yaml_serde::Value = yaml_serde::from_str(yaml).unwrap();
        assert!(check_finite(&yaml).is_ok(), "{yaml:?}");
    }

    /// NaN, infinity and minus infinity, in a column, a scalar, a scenario and
    /// under a tag, each named by its place
    #[test]
    fn what_is_not_finite_is_refused_where_it_is() {
        for (yaml, place, written) in [
            ("t:\n  x: [3, .nan, 2]\n", "t.x row 2", ".nan"),
            ("a:\n  value: .inf\n", "a.value", "infinity"),
            ("a: -.inf\n", "a", "minus infinity"),
            ("out:\n  nanres: .nan\n", "out.nanres", ".nan"),
            (
                "scenarios:\n  low:\n    price: .nan\n",
                "scenarios.low.price",
                ".nan",
            ),
            ("t:\n  x: !t [1, .nan]\n", "t.x row 2", ".nan"),
        ] {
            let error = refused(yaml);
            assert!(
                error.contains(&format!("'{place}' is {written}, which is not a number")),
                "{yaml}: {error}"
            );
        }
    }

    /// Finite numbers, and everything else a model holds, are accepted; so is
    /// a formula's stored result, which is calculated again
    #[test]
    fn finite_numbers_and_calculated_values_are_accepted() {
        accepted("t:\n  x: [1, 2.5, -3]\n  y: [a, b, ~]\n  z: [true, false, null]\n");
        accepted("a:\n  value: 1.7976931348623157e308\n");
        accepted("out:\n  m:\n    formula: =1 + 1\n    value: .nan\n");
        accepted("1: 2\n");
    }

    /// The round trip. A model with NaN in it is refused by `forge calculate`
    /// and by `forge export`. A formula that would give NaN (infinity minus
    /// infinity, from a literal past the largest double) is refused by
    /// `forge calculate`, which so writes no `.nan` into the model, and is left
    /// out of the workbook by `forge export`, with its reason, where it was
    /// written as `inf - inf` with a cached `NaN` that Excel cannot read
    #[test]
    fn neither_the_model_nor_a_workbook_gets_a_number_that_is_not_finite() {
        use calamine::{open_workbook, Data, Reader, Xlsx};
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("m.yaml");
        let path = model.display().to_string();
        let xlsx = dir.path().join("m.xlsx");
        let workbook = xlsx.display().to_string();

        let text = "_forge_version: \"5.0.0\"\nt:\n  x: [3, .nan, 2, 1]\n";
        std::fs::write(&model, text).unwrap();
        for args in [
            vec!["calculate", path.as_str()],
            vec!["export", &path, &workbook],
        ] {
            let ran = crate::cli::forge(&args);
            assert_eq!(ran.code, 1, "{args:?}");
            assert!(
                ran.err.contains("'t.x row 2' is .nan"),
                "{args:?}: {}",
                ran.err
            );
        }
        assert!(!xlsx.exists());

        let text = "_forge_version: \"5.0.0\"\nout:\n  m:\n    formula: =1E309 - 1E309\n";
        let refused = "#NUM!: 1E309 is larger than any number a cell holds";
        std::fs::write(&model, text).unwrap();
        let ran = crate::cli::forge(&["calculate", &path]);
        assert_eq!(ran.code, 1);
        assert!(ran.err.contains(refused), "{}", ran.err);
        assert_eq!(std::fs::read_to_string(&model).unwrap(), text);
        let ran = crate::cli::forge(&["export", &path, &workbook]);
        assert_eq!(ran.code, 0, "{}", ran.err);
        let printed = ran.printed();
        assert!(printed.contains("out.m: left empty"), "{printed}");
        let mut book: Xlsx<_> = open_workbook(&xlsx).unwrap();
        for sheet in book.sheet_names() {
            let values = book.worksheet_range(&sheet).unwrap();
            assert!(
                values.cells().all(|(_, _, cell)| match cell {
                    Data::Float(n) => n.is_finite(),
                    other => !other.to_string().contains("inf"),
                }),
                "{sheet}"
            );
            let formulas = book.worksheet_formula(&sheet).unwrap();
            assert!(
                formulas.cells().all(|(_, _, f)| !f.contains("inf")),
                "{sheet}"
            );
        }
    }
}
