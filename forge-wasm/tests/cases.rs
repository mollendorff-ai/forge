//! Every case of `tests/cases.json` gives the same answer from files in memory
//! as the command's core function from the same files on disk, and the
//! headline numbers of the examples come out of the browser's entry point.

use forge_wasm::{call, cases, run, run_on_disk};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn examples() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples")
}

/// A result without the parts that differ from one run to the next.
fn steady(mut value: Value) -> Value {
    match &mut value {
        Value::Object(map) => {
            map.remove("execution_time_ms");
            // A workbook records when it was made (docProps/core.xml); its
            // parts are compared in tests/node/compare.mjs
            map.remove("excel_base64");
            map.remove("byte_count");
            for v in map.values_mut() {
                *v = steady(v.take());
            }
        },
        Value::Array(items) => {
            for v in items.iter_mut() {
                *v = steady(v.take());
            }
        },
        _ => {},
    }
    value
}

/// The same answer: equal, except that numbers may differ in their last bits.
/// `forge bayesian` applies its evidence in hash-map order, which changes from
/// run to run, and with it the rounding of a probability
/// (0.7272727272727272 or ...273), on disk as in memory.
fn same(a: &Value, b: &Value, at: &str) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            assert!(
                (x - y).abs() <= 1e-12 * x.abs().max(y.abs()).max(1.0),
                "{at}: {x} in memory, {y} on disk"
            );
        },
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{at}: lengths differ");
            for (i, (x, y)) in x.iter().zip(y).enumerate() {
                same(x, y, &format!("{at}[{i}]"));
            }
        },
        (Value::Object(x), Value::Object(y)) => {
            let keys = |m: &serde_json::Map<String, Value>| m.keys().cloned().collect::<Vec<_>>();
            assert_eq!(keys(x), keys(y), "{at}: keys differ");
            for (k, v) in x {
                same(v, &y[k], &format!("{at}.{k}"));
            }
        },
        _ => assert_eq!(a, b, "{at}: memory and disk differ"),
    }
}

fn case(name: &str) -> Value {
    let case = cases::all()
        .into_iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("no case named {name}"));
    let tool = case["tool"].as_str().unwrap().to_string();
    run(&tool, &cases::request(&case, &examples())).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn number(value: &Value, pointer: &str) -> f64 {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("no number at {pointer}"))
}

fn close(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} is not {expected} (within {tolerance})"
    );
}

#[test]
fn every_case_answers_from_memory_as_from_disk() {
    for case in cases::all() {
        let name = case["name"].as_str().unwrap();
        let tool = case["tool"].as_str().unwrap();
        let request = cases::request(&case, &examples());
        let in_memory = run(tool, &request);
        assert!(in_memory.is_ok(), "{name}: {in_memory:?}");
        if case.get("files").is_none() {
            continue;
        }
        let dir = examples().join(case["dir"].as_str().unwrap());
        let on_disk = run_on_disk(tool, &request, &dir);
        same(&steady(in_memory.unwrap()), &steady(on_disk.unwrap()), name);
    }
}

// The numbers below are the ones examples/bagel/README.md and the main
// README give, and that tests/bagel_example.rs and tests/demo_examples.rs
// check against calculations made outside Forge.

#[test]
fn bagel_plan_profit_and_flour_shock() {
    close(
        number(&case("calculate bagel"), "/scalars/year.profit"),
        786.32,
        0.005,
    );
    close(
        number(
            &case("calculate bagel, flour shock"),
            "/scalars/year.profit",
        ),
        -16_943.27,
        0.005,
    );
}

#[test]
fn bagel_break_even_and_goal_seek() {
    close(number(&case("break-even price"), "/solution"), 15.97, 0.005);
    close(
        number(&case("goal seek staff hours"), "/solution"),
        24.27,
        0.005,
    );
}

#[test]
fn mortgage_extra_payment_for_80000_of_interest() {
    close(
        number(&case("goal seek mortgage extra"), "/solution"),
        397.044_208,
        1e-6,
    );
}

#[test]
fn bagel_simulation_with_seed_7() {
    let result = case("simulate bagel");
    let profit = &result["monte_carlo_results"]["outputs"]["year.profit"];
    let above_zero = profit["thresholds"]["> 0"]
        .as_f64()
        .expect("P(year.profit > 0)");
    close(above_zero, 0.3476, 0.000_05);
}

#[test]
fn bagel_decisions_and_options() {
    let tree = case("decision tree second shop");
    assert_eq!(tree["decision_policy"]["root"], "popup_first");
    close(number(&tree, "/root_expected_value"), 61_500.0, 0.005);
    let options = case("real options wait");
    assert_eq!(options["best_option"], "Wait up to a year");
}

#[test]
fn the_browser_entry_point_answers_in_json() {
    let request = cases::request(
        &cases::all()
            .into_iter()
            .find(|c| c["name"] == "calculate starter")
            .unwrap(),
        &examples(),
    );
    let answer: Value = serde_json::from_str(&call("calculate", &request.to_string())).unwrap();
    close(number(&answer, "/ok/scalars/plan.profit"), 1800.0, 1e-9);

    let missing: Value = serde_json::from_str(&call("calculate", r#"{"files": {}}"#)).unwrap();
    assert_eq!(
        missing["error"],
        "'main' names model.yaml, which is not in 'files'"
    );

    let unknown: Value = serde_json::from_str(&call("nope", r#"{"files": {}}"#)).unwrap();
    assert!(unknown["error"]
        .as_str()
        .unwrap()
        .starts_with("No tool named 'nope'"));
}

#[test]
fn model_lists_inputs_and_formulas_in_file_order() {
    let model = case("model bagel");
    let first = &model["scalars"][0];
    assert_eq!(first["path"], "assumptions.price_per_dozen");
    assert_eq!(first["value"], 16.0);
    assert!(first["formula"].is_null());
    let profit = model["scalars"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["path"] == "year.profit")
        .unwrap();
    assert_eq!(profit["formula"], "=SUM(months.profit)");
    let months = &model["tables"][0];
    assert_eq!(months["name"], "months");
    assert_eq!(months["columns"][0]["name"], "month");
    assert_eq!(months["columns"][0]["rows"], 12);
    assert_eq!(months["columns"][2]["name"], "dozens");
    assert!(months["columns"][2]["formula"].is_string());
    let costs = &model["includes"][0];
    assert_eq!(costs["as"], "costs");
    assert!(costs["scalars"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["path"] == "@costs.ingredients.flour_per_kg"));
}

#[test]
fn a_workbook_comes_back_through_import() {
    let exported = case("export bagel");
    assert!(exported["byte_count"].as_u64().unwrap() > 1000);
    let imported = run(
        "import",
        &serde_json::json!({"excel_base64": exported["excel_base64"], "name": "bagel.yaml"}),
    )
    .unwrap();
    assert_eq!(imported["main"], "bagel.yaml");
    let files = imported["files"].as_object().unwrap();
    assert!(
        files.contains_key("costs.yaml"),
        "files: {:?}",
        files.keys()
    );
    let calculated = run(
        "calculate",
        &serde_json::json!({"files": imported["files"], "main": "bagel.yaml"}),
    )
    .unwrap();
    close(number(&calculated, "/scalars/year.profit"), 786.32, 0.005);

    let refused = run(
        "import",
        &serde_json::json!({"excel_base64": "bm90IGEgd29ya2Jvb2s="}),
    );
    let message = refused.unwrap_err();
    assert!(message.contains("Failed to open Excel file"), "{message}");
}
