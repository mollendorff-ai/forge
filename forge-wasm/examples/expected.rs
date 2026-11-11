//! Print the native results of every case in `tests/cases.json`, as one JSON
//! object by case name. CI compares the WebAssembly build's results with these.
//!
//! `cargo run -p forge-wasm --release --example expected > expected.json`

use forge_wasm::{cases, run};
use serde_json::{Map, Value};
use std::path::Path;

fn main() {
    let examples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    let mut results = Map::new();
    for case in cases::all() {
        let name = case["name"].as_str().unwrap_or_default().to_string();
        let tool = case["tool"].as_str().unwrap_or_default();
        let answer = match run(tool, &cases::request(&case, &examples)) {
            Ok(result) => serde_json::json!({ "ok": result }),
            Err(message) => serde_json::json!({ "error": message }),
        };
        results.insert(name, answer);
    }
    println!("{}", Value::Object(results));
}
