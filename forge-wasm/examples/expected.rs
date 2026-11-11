//! Print the native results of every case in `tests/cases.json`, as one JSON
//! object by case name. CI compares the WebAssembly build's results with these.
//!
//! `cargo run -p forge-wasm --release --example expected > expected.json`
//!
//! With a directory as argument it also writes there what `forge export` writes
//! for each example Forge Studio exports (`bagel.xlsx`, `mortgage.xlsx`), with
//! the command line's own function, for the workbook comparisons.

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

    if let Some(dir) = std::env::args().nth(1) {
        let dir = Path::new(&dir);
        std::fs::create_dir_all(dir).expect("the directory for the workbooks");
        for (model, out) in [
            ("bagel/bagel.yaml", "bagel.xlsx"),
            ("mortgage/mortgage.yaml", "mortgage.xlsx"),
        ] {
            mollendorff_forge::cli::export_core(&examples.join(model), &dir.join(out))
                .unwrap_or_else(|e| panic!("forge export {model}: {e}"));
        }
    }
}
