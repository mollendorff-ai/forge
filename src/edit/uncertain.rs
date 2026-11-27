//! `make_uncertain` and `make_certain` (ADR-075, decision 2; ADR-076,
//! decision 4): an input becomes a distribution with its value kept as the
//! plan, and a distribution becomes an input again.
//!
//! The text is changed at the one entry, in the style the file already uses
//! for it (a short `price: 16`, a block with `value:` and `unit:`, a flow
//! mapping). Making an input uncertain in a model with no `monte_carlo:`
//! block adds one at the end of the main file, with the model's results as its
//! outputs. The model is then read back: valid at plan, with that formula and
//! nothing else changed.

use super::text::{encode_formula, parse, place, Style};
use super::{check, formula, js_number, value_text, Change, Op, Target};
use crate::monte_carlo::engine::plan_of;
use crate::monte_carlo::is_distribution;
use crate::types::ParsedModel;
use std::collections::HashMap;
use yaml_edit::{MappingEntry, YamlFile, YamlNode};

/// The distribution an input gets when none is asked for: a triangle from
/// 80% to 120% of the value, the value its mode (and its plan)
fn triangle(value: f64) -> Result<String, String> {
    if value == 0.0 {
        return Err("a range around 0 cannot be 80% to 120% of it: give `formula`".into());
    }
    let round = |x: f64| format!("{x:.10e}").parse::<f64>().unwrap_or(x);
    let (low, high) = (round(value * 0.8), round(value * 1.2));
    Ok(format!(
        "=MC.Triangular({}, {}, {})",
        js_number(low.min(high)),
        js_number(value),
        js_number(low.max(high))
    ))
}

/// The indentation of the line `at` is on
fn indent_at(text: &str, at: usize) -> &str {
    let line = text[..at].rfind('\n').map_or(0, |i| i + 1);
    &text[line..at]
}

/// The entry of the last of `names`, in the block that holds it
fn entry_of(text: &str, names: &[&str]) -> Result<(YamlFile, MappingEntry), String> {
    let file = parse(text)?;
    let (mapping, name) = place(&file, names)?;
    if mapping.is_flow_style() {
        return Err("its group is written on one line ({...}); edit changes a block".into());
    }
    let entry = mapping
        .find_entry_by_key(name)
        .ok_or_else(|| format!("{name} is not written where edit can find it"))?;
    Ok((file, entry))
}

/// An input written as `price: 16`, or as a mapping with `value:`, given the
/// distribution `formula` beside its value.
///
/// # Errors
///
/// Returns why, when the input is not written where edit can change it.
///
/// # Panics
///
/// Never: a mapping written as a block has a name.
pub(super) fn make_uncertain(text: &str, names: &[&str], formula: &str) -> Result<String, String> {
    let (_file, entry) = entry_of(text, names)?;
    let formula = encode_formula(formula, Style::Double, false);
    match entry.value_node() {
        Some(YamlNode::Scalar(value)) => {
            let range = value.byte_range();
            let (start, end) = (range.start as usize, range.end as usize);
            let indent = indent_at(text, entry.byte_range().start as usize);
            let inner = if indent.is_empty() {
                "  ".to_string()
            } else {
                format!("{indent}{indent}")
            };
            let line_end = text[end..].find('\n').map_or(text.len(), |i| end + i);
            Ok(format!(
                "{}\n{inner}value: {}{}\n{inner}formula: {formula}{}",
                text[..start].trim_end_matches([' ', '\t']),
                &text[start..end],
                &text[end..line_end],
                &text[line_end..]
            ))
        },
        Some(YamlNode::Mapping(inner)) if inner.is_flow_style() => {
            let close = text[..inner.byte_range().end as usize].trim_end().len() - 1;
            let open = text[..close].trim_end().len();
            Ok(format!(
                "{}, formula: {formula}{}",
                &text[..open],
                &text[close..]
            ))
        },
        Some(YamlNode::Mapping(inner)) => {
            let first = inner.entries().next().expect("a block mapping has a name");
            let last = inner.entries().last().expect("a block mapping has a name");
            let line = format!(
                "{}formula: {formula}",
                indent_at(text, first.byte_range().start as usize)
            );
            let at = last.byte_range().end as usize;
            let added = if text[..at].ends_with('\n') {
                line + "\n"
            } else {
                format!("\n{line}")
            };
            Ok(format!("{}{added}{}", &text[..at], &text[at..]))
        },
        _ => Err("it is not written as a number or a mapping with `value:`".into()),
    }
}

/// A distribution made an input: its `formula:` taken out, and `value:` set
/// to `value` when that is given, or put where the formula was when there is
/// none. What is left is written short (`price: 16`) when it is `value:`
/// alone.
///
/// # Errors
///
/// Returns why, when the distribution is not written where edit can change
/// it, or has no `value:` and none was given.
pub(super) fn make_certain(
    text: &str,
    names: &[&str],
    value: Option<&str>,
) -> Result<String, String> {
    let (file, entry) = entry_of(text, names)?;
    let missing = || "it has no `formula:` written where edit can find it".to_string();
    let Some(YamlNode::Mapping(inner)) = entry.value_node() else {
        return Err(missing());
    };
    let Some(YamlNode::Scalar(formula)) = inner.get("formula") else {
        return Err(missing());
    };
    let slot = match inner.get("value") {
        Some(YamlNode::Scalar(s)) => Some(s),
        _ => None,
    };
    let kept = match (slot, value) {
        (None, Some(v)) => {
            // The formula's place takes the value
            inner.rename_key("formula", "value");
            formula.set_value(v);
            formula
        },
        (Some(scalar), given) if given.is_some() || !scalar.is_null() => {
            if let Some(v) = given {
                // `value:` with nothing after it: the space goes in too
                let spaced = if scalar.value().is_empty() {
                    format!(" {v}")
                } else {
                    v.to_string()
                };
                scalar.set_value(&spaced);
            }
            inner.remove("formula");
            scalar
        },
        _ => return Err("it has no value to keep: give `value`".into()),
    };
    // `price:` with `value: 16` under it and nothing else is written `price: 16`
    let written = file.to_string();
    if inner.is_flow_style() || inner.len() != 1 {
        return Ok(written);
    }
    let key = entry
        .key_node()
        .expect("an entry found by its name has a key");
    let key_end = key.byte_range().end as usize;
    let start = kept.byte_range().start as usize;
    let collapsed = written[key_end..start]
        .strip_prefix(':')
        .and_then(|rest| rest.trim_start().strip_prefix("value:"))
        .filter(|rest| rest.trim().is_empty());
    Ok(collapsed.map_or_else(
        || written.clone(),
        |_| {
            let value = written[start..].trim_start_matches([' ', '\t']);
            format!("{}: {value}", &written[..key_end])
        },
    ))
}

/// The model's results: its scalars with a formula that is not a
/// distribution, in the file's order. A result that another formula reads is
/// one too: a year's profit is read by its margin, and is what a simulation
/// is asked about first (roadmap E-EDIT-UNCERTAIN)
fn results(model: &ParsedModel) -> Vec<String> {
    model
        .scalars
        .iter()
        .filter(|(_, var)| var.formula.as_deref().is_some_and(|f| !is_distribution(f)))
        .map(|(name, _)| name.clone())
        .collect()
}

/// `text` with a `monte_carlo:` block at its end, asking about `outputs`
fn with_simulation(text: &str, outputs: &[String]) -> String {
    let mut block = String::from(
        "\nmonte_carlo:\n  iterations: 5000\n  seed: 7\n  sampling: latin_hypercube\n",
    );
    if !outputs.is_empty() {
        block.push_str("  outputs:\n");
        for output in outputs {
            block.push_str("    - variable: ");
            block.push_str(output);
            block.push('\n');
        }
    }
    let end = if text.ends_with('\n') { "" } else { "\n" };
    format!("{text}{end}{block}")
}

/// What `make_uncertain` or `make_certain` must leave Forge reading
pub(super) struct Expect {
    /// The scalar's formula, `None` when it has none
    pub(super) formula: Option<String>,
    /// Its value, when it is known
    pub(super) value: Option<f64>,
    /// Whether the model gets a `monte_carlo:` block
    pub(super) block: bool,
}

/// The changes of `make_uncertain` or `make_certain`, in `files`, kept only
/// when Forge reads the model back as asked.
pub(super) fn apply(
    files: &mut HashMap<String, String>,
    main: &str,
    before: &ParsedModel,
    target: &Target<'_>,
    change: &Change,
) -> Result<(), String> {
    let var = target
        .model
        .scalars
        .get(&target.name)
        .ok_or("it is not an input or a distribution of the model")?;
    let names: Vec<&str> = target.name.split('.').collect();
    let at = |why: String| format!("{why} ({})", target.file);
    let text = &files[&target.file];
    let (new, expect) = if change.op == Op::MakeUncertain {
        if let Some(f) = &var.formula {
            return Err(if is_distribution(f) {
                "it is a distribution already".into()
            } else {
                "it is a formula, not an input".into()
            });
        }
        if var.is_date {
            return Err("it is a date, which cannot be uncertain".into());
        }
        let value = var.value.ok_or("it has no value")?;
        let distribution = match change.formula.as_deref() {
            Some(given) => {
                let given = formula(Some(given))?;
                if !is_distribution(given) {
                    return Err(format!(
                        "{given} is not a distribution: =MC.Triangular(min, mode, max)"
                    ));
                }
                given.to_string()
            },
            None => triangle(value)?,
        };
        let new = make_uncertain(text, &names, &distribution).map_err(at)?;
        let block = !before.analyses.iter().any(|a| a == "monte_carlo");
        let expect = Expect {
            formula: Some(distribution),
            value: Some(value),
            block,
        };
        (new, expect)
    } else {
        if !var.formula.as_deref().is_some_and(is_distribution) {
            return Err("it is not a distribution".into());
        }
        let given = change.value.as_ref();
        let written = given.map(|v| value_text(Some(v))).transpose()?;
        let plan = match (var.value, &written) {
            (None, None) => plan_of(target.model)?
                .get(&target.name)
                .map(|p| p.value)
                .filter(|v| v.is_finite()),
            _ => None,
        };
        let value = match given {
            Some(crate::edit::Value::Number(n)) => Some(*n),
            Some(crate::edit::Value::Text(_) | crate::edit::Value::List(_)) => None,
            None => plan.or(var.value),
        };
        let new = make_certain(
            text,
            &names,
            written.or_else(|| plan.map(js_number)).as_deref(),
        )
        .map_err(at)?;
        let expect = Expect {
            formula: None,
            value,
            block: false,
        };
        (new, expect)
    };
    let mut changed = files.clone();
    changed.insert(target.file.clone(), new);
    if expect.block {
        let main_text = &changed[main];
        let with = with_simulation(main_text, &results(before));
        changed.insert(main.to_string(), with);
    }
    let after = check::read(&changed, main)
        .and_then(|after| check::valid(&after).map(|()| after))
        .map_err(|e| format!("Forge refuses the model with this change: {e}"))?;
    check::only_uncertainty(before, &after, target, &expect)?;
    *files = changed;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{edit_files, Change, Value};
    use super::*;
    use std::collections::BTreeMap;

    const MODEL: &str = "_forge_version: \"5.4.0\"
_includes:
  - file: costs.yaml
    as: costs
rate: 0.05   # a rate
year:
  price: 16   # dollars
  units: 100
  tax:
    value: 0.15
    unit: ratio
  fee: {value: 25, unit: CAD}
  start: 2026-01-31
  revenue:
    formula: \"=price * units\"
  net:
    formula: \"=revenue - fee - @costs.rent\"
months:
  month: [1, 2, 3]
  sales: =month * price
four:
    deep: 1
    other: 2
";
    const COSTS: &str = "_forge_version: \"5.4.0\"\nrent: 900   # a month\nspare: 1\n";
    /// The simulation asks about every result, year.revenue too though
    /// year.net reads it, as a year's profit is read by its margin
    /// (roadmap E-EDIT-UNCERTAIN)
    const BLOCK: &str = "\nmonte_carlo:\n  iterations: 5000\n  seed: 7\n  sampling: latin_hypercube\n  outputs:\n    - variable: year.revenue\n    - variable: year.net\n";

    fn files() -> HashMap<String, String> {
        HashMap::from([
            ("model.yaml".to_string(), MODEL.to_string()),
            ("costs.yaml".to_string(), COSTS.to_string()),
        ])
    }

    fn change(op: Op, path: &str) -> Change {
        Change {
            op,
            path: path.into(),
            value: None,
            formula: None,
            unit: None,
            after: None,
            new_name: None,
            plays: None,
        }
    }

    fn uncertain(path: &str) -> Change {
        change(Op::MakeUncertain, path)
    }

    fn certain(path: &str) -> Change {
        change(Op::MakeCertain, path)
    }

    fn run(
        files: &HashMap<String, String>,
        change: Change,
    ) -> Result<BTreeMap<String, String>, String> {
        edit_files(files, "model.yaml", &[change]).map_err(|e| e.to_string())
    }

    /// `change` gives `want` for the file it changes (the main file, with
    /// the simulation block when `block`), and `back` gives the file as it
    /// was, but for that block
    fn made_and_unmade(forward: Change, back: Change, from: &str, to: &str) {
        let made = run(&files(), forward).unwrap();
        let want = format!("{}{BLOCK}", MODEL.replacen(from, to, 1));
        assert_eq!(made.keys().collect::<Vec<_>>(), ["model.yaml"]);
        assert_eq!(made["model.yaml"], want);
        let mut now = files();
        now.extend(made);
        let again = run(&now, back).unwrap();
        assert_eq!(again["model.yaml"], format!("{MODEL}{BLOCK}"));
    }

    #[test]
    fn each_way_an_input_is_written_becomes_a_distribution_and_back() {
        // Short, in a group, its comment kept on the value
        made_and_unmade(
            uncertain("year.price"),
            certain("year.price"),
            "  price: 16   # dollars\n",
            "  price:\n    value: 16   # dollars\n    formula: \"=MC.Triangular(12.8, 16, 19.2)\"\n",
        );
        // Short, at the top level
        made_and_unmade(
            uncertain("rate"),
            certain("rate"),
            "rate: 0.05   # a rate\n",
            "rate:\n  value: 0.05   # a rate\n  formula: \"=MC.Triangular(0.04, 0.05, 0.06)\"\n",
        );
        // A block with its unit
        made_and_unmade(
            uncertain("year.tax"),
            certain("year.tax"),
            "    unit: ratio\n",
            "    unit: ratio\n    formula: \"=MC.Triangular(0.12, 0.15, 0.18)\"\n",
        );
        // A flow mapping
        made_and_unmade(
            uncertain("year.fee"),
            certain("year.fee"),
            "{value: 25, unit: CAD}",
            "{value: 25, unit: CAD, formula: \"=MC.Triangular(20, 25, 30)\"}",
        );
        // A group indented by 4 indents what is under the name by 4 more
        made_and_unmade(
            uncertain("four.deep"),
            certain("four.deep"),
            "    deep: 1\n",
            "    deep:\n        value: 1\n        formula: \"=MC.Triangular(0.8, 1, 1.2)\"\n",
        );
    }

    #[test]
    fn a_distribution_is_given_and_a_simulation_already_there_stays_as_it_is() {
        let given = Change {
            formula: Some("=MC.PERT(10, 16, 30)".into()),
            ..uncertain("year.price")
        };
        let with_block = format!("{MODEL}monte_carlo:\n  iterations: 100\n");
        let mut files = files();
        files.insert("model.yaml".into(), with_block.clone());
        let made = run(&files, given).unwrap();
        assert_eq!(
            made["model.yaml"],
            with_block.replacen(
                "  price: 16   # dollars\n",
                "  price:\n    value: 16   # dollars\n    formula: \"=MC.PERT(10, 16, 30)\"\n",
                1
            )
        );
        // A file with no last line break gets one before the block
        let bare = "_forge_version: \"5.1.0\"\nx: 2\ny:\n  formula: \"=x * 2\"";
        let one = HashMap::from([("m.yaml".to_string(), bare.to_string())]);
        let made = edit_files(&one, "m.yaml", &[uncertain("x")]).unwrap();
        assert_eq!(
            made["m.yaml"],
            "_forge_version: \"5.1.0\"\nx:\n  value: 2\n  formula: \"=MC.Triangular(1.6, 2, 2.4)\"\ny:\n  formula: \"=x * 2\"\n\nmonte_carlo:\n  iterations: 5000\n  seed: 7\n  sampling: latin_hypercube\n  outputs:\n    - variable: y\n"
        );
        // A model with no result has a simulation with no outputs
        let none = "_forge_version: \"5.1.0\"\nx: -2";
        let one = HashMap::from([("m.yaml".to_string(), none.to_string())]);
        let made = edit_files(&one, "m.yaml", &[uncertain("x")]).unwrap();
        assert_eq!(
            made["m.yaml"],
            "_forge_version: \"5.1.0\"\nx:\n  value: -2\n  formula: \"=MC.Triangular(-2.4, -2, -1.6)\"\n\nmonte_carlo:\n  iterations: 5000\n  seed: 7\n  sampling: latin_hypercube\n"
        );
    }

    #[test]
    fn a_value_of_an_included_file_is_changed_there_and_the_simulation_is_asked_in_the_main_file() {
        let made = run(&files(), uncertain("@costs.rent")).unwrap();
        assert_eq!(
            made["costs.yaml"],
            COSTS.replace(
                "rent: 900   # a month\n",
                "rent:\n  value: 900   # a month\n  formula: \"=MC.Triangular(720, 900, 1080)\"\n"
            )
        );
        assert_eq!(made["model.yaml"], format!("{MODEL}{BLOCK}"));
        let mut now = files();
        now.extend(made);
        let back = run(&now, certain("@costs.rent")).unwrap();
        assert_eq!(back["costs.yaml"], COSTS);
    }

    #[test]
    fn a_distribution_made_certain_keeps_its_value_or_takes_a_figure() {
        let head = "_forge_version: \"5.1.0\"\n";
        let one = |body: &str, change: Change| {
            let files = HashMap::from([("m.yaml".to_string(), format!("{head}{body}"))]);
            edit_files(&files, "m.yaml", &[change])
                .map(|made| made["m.yaml"].replacen(head, "", 1))
                .map_err(|e| e.to_string())
        };
        // No value: the plan figure takes the formula's place
        assert_eq!(
            one(
                "g:\n  growth:\n    formula: \"=MC.Normal(0.15, 0.05)\"   # c\n  base: 2\n",
                certain("g.growth")
            ),
            Ok("g:\n  growth: 0.15   # c\n  base: 2\n".to_string())
        );
        assert_eq!(
            one(
                "g:\n  u:\n    formula: \"=MC.Uniform(1, 3)\"\n    unit: x\n",
                certain("g.u")
            ),
            Ok("g:\n  u:\n    value: 2\n    unit: x\n".to_string())
        );
        // A value beside it is kept, or replaced when one is given
        let flow = "v: {value: 1.0, formula: \"=MC.PERT(0.5, 1.0, 1.3)\", unit: x}\n";
        assert_eq!(
            one(flow, certain("v")),
            Ok("v: {value: 1.0, unit: x}\n".to_string())
        );
        let given = Change {
            value: Some(Value::Number(1.1)),
            ..certain("v")
        };
        assert_eq!(
            one(flow, given),
            Ok("v: {value: 1.1, unit: x}\n".to_string())
        );
        let text = Change {
            value: Some(Value::Text("1e-1".into())),
            ..certain("v")
        };
        assert_eq!(
            one(flow, text),
            Ok("v: {value: 1e-1, unit: x}\n".to_string())
        );
        let word = Change {
            value: Some(Value::Text("1 # two".into())),
            ..certain("v")
        };
        assert_eq!(
            one(flow, word),
            Err("Validation error: make_certain v: \"1 # two\" is not written as a number or a date (2026-01-31). No file was changed".to_string())
        );
        // `value:` with nothing after it
        assert_eq!(
            one(
                "w:\n  value:\n  formula: \"=MC.Uniform(1, 3)\"\n",
                certain("w")
            ),
            Ok("w: 2\n".to_string())
        );
        // What is left is not `value:` alone: it stays a block
        assert_eq!(
            one(
                "w:\n  # why\n  value: 1\n  formula: \"=MC.Uniform(1, 3)\"\n",
                certain("w")
            ),
            Ok("w:\n  # why\n  value: 1\n".to_string())
        );
        assert_eq!(
            one(
                "w:   # c\n  value: 1\n  formula: \"=MC.Uniform(1, 3)\"\n",
                certain("w")
            ),
            Ok("w:   # c\n  value: 1\n".to_string())
        );
    }

    #[test]
    fn a_change_that_does_not_fit_what_the_name_is_refused_with_the_reason() {
        let refused = |c: Change, why: &str| {
            let message = run(&files(), c).unwrap_err();
            assert!(message.contains(why), "{message}");
            assert!(message.ends_with(". No file was changed"), "{message}");
        };
        refused(
            uncertain("nothing"),
            "it is not an input or a distribution of the model",
        );
        refused(
            certain("year.nothing"),
            "it is not an input or a distribution of the model",
        );
        refused(uncertain("year.revenue"), "it is a formula, not an input");
        refused(
            uncertain("year.start"),
            "it is a date, which cannot be uncertain",
        );
        refused(certain("year.price"), "it is not a distribution");
        refused(certain("year.revenue"), "it is not a distribution");
        refused(
            Change {
                formula: Some("=1 + 2".into()),
                ..uncertain("year.price")
            },
            "=1 + 2 is not a distribution: =MC.Triangular(min, mode, max)",
        );
        refused(
            Change {
                formula: Some("MC.Normal(1, 2)".into()),
                ..uncertain("year.price")
            },
            "MC.Normal(1, 2) is not a formula: a formula starts with =",
        );
        refused(
            Change {
                formula: Some("=MC.Normal(1)".into()),
                ..uncertain("year.price")
            },
            "Forge refuses the model with this change",
        );
        refused(
            Change {
                value: Some(Value::Number(1.0)),
                ..uncertain("year.price")
            },
            "make_uncertain does not take `value`",
        );
        refused(
            Change {
                formula: Some("=MC.Normal(1, 2)".into()),
                ..certain("year.price")
            },
            "make_certain does not take `formula`",
        );
        let zero = HashMap::from([(
            "m.yaml".to_string(),
            "_forge_version: \"5.1.0\"\nx: 0\n".to_string(),
        )]);
        let message = edit_files(&zero, "m.yaml", &[uncertain("x")])
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("a range around 0 cannot be 80% to 120% of it: give `formula`"),
            "{message}"
        );
    }

    #[test]
    fn a_distribution_written_where_edit_cannot_change_it_is_refused() {
        let head = "_forge_version: \"5.1.0\"\n";
        let refused = |body: &str, c: Change, why: &str| {
            let files = HashMap::from([("m.yaml".to_string(), format!("{head}{body}"))]);
            let message = edit_files(&files, "m.yaml", &[c]).unwrap_err().to_string();
            assert!(message.contains(why), "{message}");
        };
        refused(
            "g: {a: 1, b: 2}\n",
            uncertain("g.a"),
            "its group is written on one line ({...}); edit changes a block",
        );
        // No value, and none given (the plan figure is given by the change,
        // not by the text)
        let direct = |text: &str, value: Option<&str>| make_certain(text, &["y"], value);
        let missing = "it has no `formula:` written where edit can find it".to_string();
        let no_value = Err("it has no value to keep: give `value`".to_string());
        assert_eq!(
            direct("y:\n  formula: \"=MC.Uniform(1, 3)\"\n", None),
            no_value
        );
        assert_eq!(direct("y: 5\n", None), Err(missing.clone()));
        assert_eq!(direct("y:\n  value: 1\n", None), Err(missing.clone()));
        assert_eq!(
            direct("y:\n  formula:\n    a: 1\n", Some("2")),
            Err(missing)
        );
        assert_eq!(
            direct("y:\n  value:\n  formula: \"=MC.Uniform(1, 3)\"\n", None),
            no_value
        );
        assert_eq!(
            make_uncertain("y: [1]\n", &["y"], "=MC.Uniform(1, 3)"),
            Err("it is not written as a number or a mapping with `value:`".to_string())
        );
        assert_eq!(
            make_uncertain("y: 1\n", &["z"], "=MC.Uniform(1, 3)"),
            Err("z is not written where edit can find it".to_string())
        );
        // A distribution is not made one again, and an input needs a value
        refused(
            "d:\n  value: 1\n  formula: \"=MC.Uniform(1, 3)\"\n",
            uncertain("d"),
            "it is a distribution already",
        );
        refused(
            "n:\n  value:\n  unit: x\n",
            uncertain("n"),
            "it has no value",
        );
        // A group written on one line cannot be made certain either
        refused(
            "g: {a: {value: 1, formula: \"=MC.Uniform(1, 3)\"}}\n",
            certain("g.a"),
            "its group is written on one line ({...}); edit changes a block (m.yaml)",
        );
        // The plan's figure is asked for when there is no value, and a
        // distribution the plan refuses stops it
        refused(
            "d:\n  formula: \"=MC.Normal(1, -3)\"\n",
            certain("d"),
            "make_certain d: d: Normal distribution stdev must be > 0",
        );
        // An anchor on the input would make its alias a distribution too
        refused(
            "x: &a 2\ny: *a\n",
            uncertain("x"),
            "Forge reads the new text as more than this change, so it was not made",
        );
        // Text that is not YAML, or has not the group, is refused by the
        // functions themselves
        let broken = "its YAML cannot be read (Unclosed flow sequence. Expected: ']' to close sequence. Found: end of input. Context: in flow sequence. Suggestion: Add ']' to close the array, or check for missing commas between elements)";
        assert_eq!(make_certain("y: [", &["y"], None), Err(broken.to_string()));
        assert_eq!(
            make_uncertain("y: [", &["y"], "=MC.Uniform(1, 3)"),
            Err(broken.to_string())
        );
        assert_eq!(
            make_certain("a: 1\n", &["z", "b"], None),
            Err("z is not written as a group of names".to_string())
        );
        // A last input written as a block, with no line break after it
        assert_eq!(
            make_uncertain("x:\n  value: 2", &["x"], "=MC.Uniform(1, 3)"),
            Ok("x:\n  value: 2\n  formula: \"=MC.Uniform(1, 3)\"".to_string())
        );
        // A table's column is not an input
        refused(
            "t:\n  a: [1, 2]\n  b: \"=MC.Uniform(1, 3)\"\n",
            certain("t.b"),
            "it is not an input or a distribution of the model",
        );
    }

    #[test]
    fn a_model_read_back_as_more_than_the_change_is_refused() {
        let model = |text: &str| {
            check::read(
                &HashMap::from([("m.yaml".to_string(), text.to_string())]),
                "m.yaml",
            )
            .unwrap()
        };
        let before = model("_forge_version: \"5.1.0\"\nx: 1\ny: 2\n");
        let target = super::super::locate(&before, "m.yaml", "x").unwrap();
        let expect = Expect {
            formula: Some("=MC.Uniform(1, 3)".into()),
            value: Some(1.0),
            block: false,
        };
        let after = model(
            "_forge_version: \"5.1.0\"\nx: {value: 1, formula: \"=MC.Uniform(1, 3)\"}\ny: 2\n",
        );
        assert_eq!(
            check::only_uncertainty(&before, &after, &target, &expect),
            Ok(())
        );
        let more = model(
            "_forge_version: \"5.1.0\"\nx: {value: 1, formula: \"=MC.Uniform(1, 3)\"}\ny: 3\n",
        );
        assert_eq!(
            check::only_uncertainty(&before, &more, &target, &expect),
            Err(
                "Forge reads the new text as more than this change, so it was not made".to_string()
            )
        );
        let other = model(
            "_forge_version: \"5.1.0\"\nx: {value: 1, formula: \"=MC.Uniform(1, 4)\"}\ny: 2\n",
        );
        assert_eq!(
            check::only_uncertainty(&before, &other, &target, &expect),
            Err(
                "Forge reads x in the new text differently from what was asked, so it was not made"
                    .to_string()
            )
        );
        let no_block = Expect {
            block: true,
            ..expect
        };
        assert!(check::only_uncertainty(&before, &after, &target, &no_block).is_err());
    }

    /// Every input of every example, and of every file one includes, made
    /// uncertain and certain again: Forge reads back the model it read
    /// (with the simulation block the first change added), and the file is
    /// the one it was, byte for byte, but for that block
    #[test]
    fn every_input_of_every_example_is_made_uncertain_and_certain_again() {
        let sets = super::super::tests::example_sets();
        let results: Vec<(usize, usize, Vec<String>)> = std::thread::scope(|scope| {
            // Every folder's thread starts before the first is waited for
            let mut runs = Vec::new();
            for chunk in sets.chunks(sets.len().div_ceil(4)) {
                runs.push(scope.spawn(move || chunk.iter().map(round_trip_in).collect::<Vec<_>>()));
            }
            runs.into_iter()
                .flat_map(|run| run.join().unwrap())
                .collect()
        });
        let made: usize = results.iter().map(|r| r.0).sum();
        let refused: usize = results.iter().map(|r| r.1).sum();
        let failures: Vec<&String> = results.iter().flat_map(|r| &r.2).collect();
        assert_eq!(failures, Vec::<&String>::new());
        assert!(made > 470, "{made} made");
        assert_eq!(refused, 20);
    }

    /// `change` made to `main` of `files`, or why not
    fn attempt(
        files: &HashMap<String, String>,
        main: &str,
        change: Change,
    ) -> Result<BTreeMap<String, String>, String> {
        edit_files(files, main, &[change]).map_err(|e| e.to_string())
    }

    /// One folder of examples: how many inputs were made uncertain and
    /// certain, how many Forge refused (an input two includes deep), and
    /// what failed, which includes a file that did not come back
    fn round_trip_in(
        (dir, files): &(std::path::PathBuf, HashMap<String, String>),
    ) -> (usize, usize, Vec<String>) {
        let (mut made, mut refused, mut failures) = (0, 0, Vec::new());
        let mut mains: Vec<&String> = files.keys().filter(|k| !k.contains('/')).collect();
        mains.sort();
        for main in mains {
            let model = check::read(files, main).unwrap();
            let mut names: Vec<String> = model
                .scalars
                .iter()
                .filter(|(_, v)| {
                    v.formula.is_none() && !v.is_date && v.value.is_some_and(|n| n != 0.0)
                })
                .map(|(name, _)| name.clone())
                .collect();
            for (alias, included) in &model.resolved_includes {
                names.extend(
                    included
                        .model
                        .scalars
                        .iter()
                        .filter(|(_, v)| {
                            v.formula.is_none() && !v.is_date && v.value.is_some_and(|n| n != 0.0)
                        })
                        .map(|(name, _)| format!("@{alias}.{name}")),
                );
            }
            for path in names {
                let place = format!("{}/{main} {path}", dir.display());
                let first = attempt(files, main, uncertain(&path));
                // An input of a file that an included file includes too: that
                // file calculates it without a plan figure
                let plan_refusal = first.as_ref().err().is_some_and(|e| {
                    e.contains("included file '") && e.contains("Unknown function: MC.")
                });
                let note = format!("{place}: {:?}", first.as_ref().err());
                failures.extend((first.is_err() && !plan_refusal).then_some(note));
                refused += usize::from(plan_refusal);
                if first.is_err() {
                    continue;
                }
                let mut now = files.clone();
                now.extend(first.unwrap_or_default());
                let back = attempt(&now, main, certain(&path));
                let note = format!("{place} back: {:?}", back.as_ref().err());
                failures.extend(back.is_err().then_some(note));
                made += usize::from(back.is_ok());
                now.extend(back.unwrap_or_default());
                let block_less = |name: &String, text: &String| {
                    let own = &files[name];
                    text == own
                        || text
                            .strip_prefix(own.as_str())
                            .is_some_and(|rest| rest.starts_with("\nmonte_carlo:"))
                };
                let same = now.iter().all(|(name, text)| block_less(name, text));
                failures.extend((!same).then_some(format!("{place} back: not the file it was")));
            }
        }
        (made, refused, failures)
    }
}
