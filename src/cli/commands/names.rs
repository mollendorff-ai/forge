//! A scalar named on the command line: its full name (`assumptions.price`),
//! or the bare name (`price`) when only one scalar has it, as an agent writes
//! it after reading the model's scenarios or tornado sections. A scalar of a
//! file the model includes is written as a scenario override writes it,
//! `@costs.flour_per_kg` for the file included `as: costs` (roadmap
//! T1-VARY-INCLUDED).

use crate::error::{ForgeError, ForgeResult};
use crate::types::{ParsedModel, Variable};

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
    lookup(model, name, missing_ok).map_err(ForgeError::Validation)
}

/// The scalar `name` stands for, as [`scalar_name`] finds it, for a command
/// that is not on the command line (a tornado's inputs).
///
/// # Errors
///
/// Returns the reason when the name fits several scalars or none.
pub fn input_name(model: &ParsedModel, name: &str) -> Result<String, String> {
    lookup(model, name, false)
}

/// The scalar a full name from [`scalar_name`] is, in the model or in the file
/// it includes; `None` when there is no such scalar.
pub fn scalar<'a>(model: &'a ParsedModel, name: &str) -> Option<&'a Variable> {
    match name.strip_prefix('@').and_then(|rest| rest.split_once('.')) {
        Some((alias, inner)) => scalar(&model.resolved_includes.get(alias)?.model, inner),
        None => model.scalars.get(name),
    }
}

/// The full name an expectation's `name` stands for (ADR-074), found as a
/// scenario's override is; a name that fits nothing is returned as given, and
/// the check then finds no result by it.
///
/// # Errors
///
/// Returns the reason when a bare name fits several scalars.
pub(super) fn expected_name(model: &ParsedModel, name: &str) -> Result<String, String> {
    lookup(model, name, true)
}

fn lookup(model: &ParsedModel, name: &str, missing_ok: bool) -> Result<String, String> {
    if let Some(rest) = name.strip_prefix('@') {
        return included(model, name, rest);
    }
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
        [] => Err(format!(
            "Variable '{name}' not found. Available scalars: {:?}",
            model.scalars.keys().collect::<Vec<_>>()
        )),
        many => Err(format!(
            "'{name}' could be any of {}; write the full name",
            many.iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// `@alias.inner`: `inner` found in the file included as `alias`, as any name
/// is found in the model (an `@` again for a file that file includes).
fn included(model: &ParsedModel, name: &str, rest: &str) -> Result<String, String> {
    let (alias, inner) = rest
        .split_once('.')
        .ok_or_else(|| format!("'{name}' needs a name after the include, as in @costs.flour"))?;
    let Some(file) = model.resolved_includes.get(alias) else {
        let mut aliases: Vec<&str> = model.resolved_includes.keys().map(String::as_str).collect();
        aliases.sort_unstable();
        let listed = if aliases.is_empty() {
            "it includes no file".to_string()
        } else {
            format!("it includes {}", aliases.join(", "))
        };
        return Err(format!(
            "'{name}': the model includes no file as '{alias}'; {listed}"
        ));
    };
    let inner = lookup(&file.model, inner, false)
        .map_err(|e| format!("{e} (in the file included as '{alias}')"))?;
    Ok(format!("@{alias}.{inner}"))
}

#[cfg(test)]
mod tests {
    use super::{input_name, scalar, scalar_name};
    use crate::types::{Include, ParsedModel, ResolvedInclude, Variable};

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

    /// `inner` included in `outer` as `alias`
    fn include(outer: &mut ParsedModel, alias: &str, inner: ParsedModel) {
        outer.resolved_includes.insert(
            alias.to_string(),
            ResolvedInclude {
                include: Include::new(format!("{alias}.yaml"), alias.to_string()),
                resolved_path: std::path::PathBuf::from(format!("{alias}.yaml")),
                model: inner,
            },
        );
    }

    /// Roadmap T1-VARY-INCLUDED: `@alias.name` is found in the file included
    /// as `alias`, in full or bare, and through a file that file includes.
    #[test]
    fn a_name_in_an_included_file() {
        let mut market = model(&["m.rate", "m.spread"]);
        market.scalars.get_mut("m.rate").unwrap().value = Some(0.05);
        let mut rates = model(&["fx.usd"]);
        include(&mut rates, "market", market);
        let mut m = model(&["a.price"]);
        include(&mut m, "costs", model(&["c.flour", "c.wage"]));
        include(&mut m, "rates", rates);
        assert_eq!(
            scalar_name(&m, "@costs.c.flour", false).unwrap(),
            "@costs.c.flour"
        );
        assert_eq!(input_name(&m, "@costs.wage").unwrap(), "@costs.c.wage");
        let nested = input_name(&m, "@rates.@market.rate").unwrap();
        assert_eq!(nested, "@rates.@market.m.rate");
        assert_eq!(scalar(&m, &nested).unwrap().value, Some(0.05));
        assert_eq!(scalar(&m, "a.price").unwrap().value, Some(1.0));
        for missing in ["@nope.c.flour", "@costs.c.salt", "@costs"] {
            assert!(scalar(&m, missing).is_none(), "{missing}");
        }
    }

    #[test]
    fn a_name_in_an_included_file_that_is_not_there() {
        let mut m = model(&["a.price"]);
        include(&mut m, "costs", model(&["c.flour"]));
        include(&mut m, "rates", model(&["r.rate"]));
        let cases = [
            ("@costs", "needs a name after the include"),
            (
                "@labour.c.wage",
                "no file as 'labour'; it includes costs, rates",
            ),
            ("@costs.c.salt", "(in the file included as 'costs')"),
        ];
        for (name, want) in cases {
            let err = input_name(&m, name).unwrap_err();
            assert!(err.contains(want), "{name}: {err}");
        }
        // An @ name is never taken as given, even for an output
        let err = scalar_name(&model(&["a.price"]), "@costs.c.flour", true).unwrap_err();
        assert!(err.to_string().contains("it includes no file"), "{err}");
    }
}
