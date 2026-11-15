//! A scalar named on the command line: its full name (`assumptions.price`),
//! or the bare name (`price`) when only one scalar has it, as an agent writes
//! it after reading the model's scenarios or tornado sections.

use crate::error::{ForgeError, ForgeResult};
use crate::types::ParsedModel;

/// The scalar `name` stands for. A bare name that fits more than one scalar is
/// an error that lists them. A name that fits none is an error too, unless
/// `missing_ok` (an output can be something other than a scalar), when it is
/// returned as given.
///
/// # Errors
///
/// Returns an error if the name fits several scalars, or none and not `missing_ok`.
pub(super) fn scalar_name(
    model: &ParsedModel,
    name: &str,
    missing_ok: bool,
) -> ForgeResult<String> {
    if model.scalars.contains_key(name) {
        return Ok(name.to_string());
    }
    let suffix = format!(".{name}");
    let mut found: Vec<&String> = model
        .scalars
        .keys()
        .filter(|k| k.ends_with(&suffix))
        .collect();
    found.sort_unstable();
    match found.as_slice() {
        [one] => Ok((*one).clone()),
        [] if missing_ok => Ok(name.to_string()),
        [] => Err(ForgeError::Validation(format!(
            "Variable '{name}' not found. Available scalars: {:?}",
            model.scalars.keys().collect::<Vec<_>>()
        ))),
        many => Err(ForgeError::Validation(format!(
            "'{name}' could be any of {}; write the full name",
            many.iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::scalar_name;
    use crate::types::{ParsedModel, Variable};

    fn model(names: &[&str]) -> ParsedModel {
        let mut m = ParsedModel::new();
        for n in names {
            m.scalars.insert(
                (*n).to_string(),
                Variable::new((*n).to_string(), Some(1.0), None),
            );
        }
        m
    }

    #[test]
    fn full_bare_ambiguous_and_missing() {
        let m = model(&["a.price", "a.cost", "b.cost", "units"]);
        assert_eq!(scalar_name(&m, "a.price", false).unwrap(), "a.price");
        assert_eq!(scalar_name(&m, "price", false).unwrap(), "a.price");
        assert_eq!(scalar_name(&m, "units", false).unwrap(), "units");
        let e = scalar_name(&m, "cost", false).unwrap_err().to_string();
        assert!(e.contains("a.cost, b.cost"), "{e}");
        assert!(scalar_name(&m, "nope", false).is_err());
        assert_eq!(scalar_name(&m, "nope", true).unwrap(), "nope");
        // `rice` is not `price`: only a whole last part counts
        assert!(scalar_name(&m, "rice", false).is_err());
    }
}
