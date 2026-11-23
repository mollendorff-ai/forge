//! The rules the guide gives an agent that works in a model repository hold
//! against `forge mcp` (roadmap A5-GUIDE-AI, ADR-078).
//!
//! `docs/guide/ai.md` has a template for a project's `AGENTS.md` or
//! `CLAUDE.md`: a numbered list of rules, and under it a table that says, for
//! each rule, what an agent that follows it sees from the server. This
//! module starts the server as a client does, with `--root` on a copy of
//! `examples/`, and makes those calls one at a time, in the order an agent
//! makes them, asserting what comes back:
//!
//! - the rules are numbered 1 to N, and so are the table's rows and the
//!   cases here, so a rule added without a case fails;
//! - each row names the tools its case calls, and the case calls them.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

/// What a tool answered: whether it failed, and its text.
#[derive(Debug)]
pub struct Answer {
    /// The tool's `isError`
    pub failed: bool,
    /// The one text the tool returns
    pub text: String,
}

impl Answer {
    /// The text read as JSON, as an agent reads a tool's answer.
    ///
    /// # Panics
    ///
    /// Panics if the text is not JSON.
    #[must_use]
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.text).expect("an answer in JSON")
    }
}

/// A client of `forge mcp --root DIR`, talking to it over its standard input
/// and output, a call at a time.
#[derive(Debug)]
pub struct Session {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
    last_id: u64,
    tools: Vec<String>,
}

impl Session {
    /// Start the server limited to `root` and say hello, as a client does.
    ///
    /// # Panics
    ///
    /// Panics if the server does not start or does not answer.
    #[must_use]
    pub fn start(root: &Path) -> Self {
        let mut child = Command::new(crate::forge_bin())
            .arg("mcp")
            .arg("--root")
            .arg(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("forge mcp starts");
        let stdin = child.stdin.take().expect("a pipe in");
        let out = BufReader::new(child.stdout.take().expect("a pipe out"));
        let mut session = Self {
            child,
            stdin,
            out,
            last_id: 0,
            tools: Vec::new(),
        };
        session.send(&json!({
            "jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "agent_rules", "version": "0"}
            }
        }));
        session.read();
        session.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        session
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").expect("the server reads");
        self.stdin.flush().expect("the server reads");
    }

    fn read(&mut self) -> Value {
        let mut line = String::new();
        self.out.read_line(&mut line).expect("the server answers");
        serde_json::from_str(&line).expect("an answer in JSON")
    }

    /// Call a tool and wait for its answer.
    ///
    /// # Panics
    ///
    /// Panics if the server answers another call, or answers without text.
    pub fn call(&mut self, tool: &str, arguments: &Value) -> Answer {
        self.last_id += 1;
        self.tools.push(tool.to_string());
        self.send(&json!({
            "jsonrpc": "2.0", "id": self.last_id, "method": "tools/call",
            "params": {"name": tool, "arguments": arguments}
        }));
        let reply = self.read();
        assert_eq!(reply["id"], self.last_id, "{reply}");
        let result = &reply["result"];
        Answer {
            failed: result["isError"] == true,
            text: result["content"][0]["text"]
                .as_str()
                .expect("a text answer")
                .to_string(),
        }
    }

    /// The tools called so far, in order.
    #[must_use]
    pub fn tools_called(&self) -> &[String] {
        &self.tools
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

/// The rules of the template in `doc`: the numbers of the numbered lines of
/// its first `markdown` block.
#[must_use]
pub fn rule_numbers(doc: &str) -> Vec<usize> {
    crate::guide::fenced_blocks(doc)
        .into_iter()
        .find(|(lang, _)| lang == "markdown")
        .map(|(_, block)| block)
        .unwrap_or_default()
        .iter()
        .filter_map(|line| line.split_once(". "))
        .filter_map(|(number, _)| number.parse().ok())
        .collect()
}

/// The table under the template: for each row, the rule it is about, and the
/// text of the row.
#[must_use]
pub fn table_rows(doc: &str) -> Vec<(usize, String)> {
    doc.lines()
        .filter(|line| line.starts_with("| "))
        .filter_map(|line| {
            let cell = line
                .trim_start_matches("| ")
                .split(" |")
                .next()
                .unwrap_or_default();
            cell.parse().ok().map(|number| (number, line.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A copy of `examples/` for an agent to work in, and a session on it.
    fn agent() -> (tempfile::TempDir, Session) {
        let (dir, path) = crate::guide::page_folder();
        let session = Session::start(&path);
        (dir, session)
    }

    fn read(dir: &tempfile::TempDir, file: &str) -> String {
        std::fs::read_to_string(dir.path().join(file)).expect("a file the agent worked on")
    }

    /// Rule 1: start from the starter model, and `_forge_version` is the
    /// model's format.
    fn start_from_the_starter(_: &tempfile::TempDir, s: &mut Session) {
        let starter = s.call("forge_examples", &json!({"name": "starter"}));
        assert!(!starter.failed, "{}", starter.text);
        let content = starter.json()["content"]
            .as_str()
            .expect("the model's text")
            .to_string();
        assert!(content.contains("_forge_version: \"5.1.0\""));
        let calculated = s.call(
            "forge_calculate",
            &json!({"content": content, "dry_run": true}),
        );
        assert!(!calculated.failed, "{}", calculated.text);
    }

    /// Rule 2: a formula is `{formula: "=..."}` in a group of scalars and a
    /// bare string in a table.
    fn two_forms_of_a_formula(_: &tempfile::TempDir, s: &mut Session) {
        let group = "_forge_version: \"5.1.0\"\nplan:\n  price: 16\n  rev: \"=price * 2\"\n";
        let refused = s.call("forge_calculate", &json!({"content": group}));
        assert!(refused.failed);
        assert!(
            refused
                .text
                .contains("write it as `rev: {formula: \"=price * 2\"}`"),
            "{}",
            refused.text
        );
        let table = "_forge_version: \"5.1.0\"\nsales:\n  price: [16, 17]\n  rev: \"=price * 2\"\n";
        let accepted = s.call("forge_calculate", &json!({"content": table}));
        assert!(!accepted.failed, "{}", accepted.text);
    }

    /// Rule 3: change a model with `forge_edit`, which keeps the rest of the
    /// file and refuses a name that is not there.
    fn change_with_edit(dir: &tempfile::TempDir, s: &mut Session) {
        let before = read(dir, "examples/starter.yaml");
        let misspelt = s.call(
            "forge_edit",
            &json!({"file_path": "examples/starter.yaml",
                    "changes": [{"op": "set_value", "path": "plan.prise", "value": 45}]}),
        );
        assert!(misspelt.failed);
        assert!(
            misspelt.text.contains("it is not in the model"),
            "{}",
            misspelt.text
        );
        assert_eq!(read(dir, "examples/starter.yaml"), before);
        let edited = s.call(
            "forge_edit",
            &json!({"file_path": "examples/starter.yaml",
                    "changes": [{"op": "set_value", "path": "plan.price", "value": 45}]}),
        );
        assert!(!edited.failed, "{}", edited.text);
        assert_eq!(edited.json()["written"], true);
        assert_eq!(
            read(dir, "examples/starter.yaml"),
            before.replacen("  price: 40", "  price: 45", 1)
        );
    }

    /// Rule 4: `forge_validate` finds a result that an edit left stale.
    fn validate_finds_stale_results(dir: &tempfile::TempDir, s: &mut Session) {
        let file = json!({"file_path": "examples/starter.yaml"});
        assert!(!s.call("forge_calculate", &file).failed);
        let edit = json!({"file_path": "examples/starter.yaml",
            "changes": [{"op": "set_value", "path": "plan.price", "value": 45}]});
        assert!(!s.call("forge_edit", &edit).failed);
        let stale = s.call("forge_validate", &file).json();
        assert_eq!(stale["scalars_valid"], false);
        assert_eq!(stale["mismatches"][0]["name"], "plan.revenue");
        assert_eq!(stale["mismatches"][0]["expected_value"], json!(5400.0));
        assert!(read(dir, "examples/starter.yaml").contains("value: 4800"));
    }

    /// Rule 5: results are written by `forge_calculate`, after which
    /// `forge_validate` passes.
    fn calculate_writes_the_results(dir: &tempfile::TempDir, s: &mut Session) {
        let file = json!({"file_path": "examples/starter.yaml"});
        let edit = json!({"file_path": "examples/starter.yaml",
            "changes": [{"op": "set_value", "path": "plan.price", "value": 45}]});
        assert!(!s.call("forge_edit", &edit).failed);
        assert!(!s.call("forge_calculate", &file).failed);
        let passed = s.call("forge_validate", &file).json();
        assert_eq!(passed["scalars_valid"], true);
        assert_eq!(passed["mismatches"], json!([]));
        assert_eq!(passed["unstored"], 0);
        assert!(read(dir, "examples/starter.yaml").contains("value: 5400"));
    }

    /// Rule 6: a model with `MC.` distributions is simulated; `calculate`
    /// refuses it and says so, and takes `plan` to calculate it at plan.
    fn uncertain_models_are_simulated(_: &tempfile::TempDir, s: &mut Session) {
        let file = "examples/bagel/bagel_simulate.yaml";
        let refused = s.call("forge_calculate", &json!({"file_path": file}));
        assert!(refused.failed);
        assert!(
            refused.text.contains("Run `forge simulate`"),
            "{}",
            refused.text
        );
        let plan = s
            .call(
                "forge_calculate",
                &json!({"file_path": file, "plan": true, "dry_run": true}),
            )
            .json();
        assert_eq!(
            plan["at_plan"]["assumptions.opening_dozens_per_day"],
            json!(55.0)
        );
        let simulated = s.call(
            "forge_simulate",
            &json!({"file_path": file, "iterations": 200}),
        );
        assert!(!simulated.failed, "{}", simulated.text);
        assert_eq!(simulated.json()["monte_carlo_results"]["iterations"], 200);
    }

    /// Rule 7: a result is checked another way, here by what `forge_audit`
    /// says it is: the price times the units.
    fn check_a_result_another_way(_: &tempfile::TempDir, s: &mut Session) {
        let audit = s
            .call(
                "forge_audit",
                &json!({"file_path": "examples/starter.yaml", "variable": "plan.revenue"}),
            )
            .json();
        assert_eq!(audit["formula"], "=plan.price * plan.units_per_month");
        assert_eq!(audit["calculated_value"], json!(4800.0));
    }

    /// Rule 8: files stay inside the folder the server names.
    fn files_stay_inside_the_folder(_: &tempfile::TempDir, s: &mut Session) {
        let outside = crate::root().join("examples/starter.yaml");
        let refused = s.call(
            "forge_calculate",
            &json!({"file_path": outside, "dry_run": true}),
        );
        assert!(refused.failed);
        assert!(
            refused.text.contains("the folder this server may use"),
            "{}",
            refused.text
        );
    }

    /// Rule 9: the answer holds the whole file, to hand back.
    fn hand_back_the_file(_: &tempfile::TempDir, s: &mut Session) {
        let model = "_forge_version: \"5.1.0\"\nplan:\n  price: 16\n";
        let edited = s
            .call(
                "forge_edit",
                &json!({"content": model,
                        "changes": [{"op": "set_value", "path": "plan.price", "value": 17}]}),
            )
            .json();
        assert_eq!(
            edited["files"]["model.yaml"],
            "_forge_version: \"5.1.0\"\nplan:\n  price: 17\n"
        );
        let content =
            "_forge_version: \"5.1.0\"\nplan:\n  price: 16\n  rev: {formula: \"=price * 2\"}\n";
        let calculated = s
            .call(
                "forge_calculate",
                &json!({"content": content, "dry_run": false}),
            )
            .json();
        assert_eq!(
            calculated["calculated_content"],
            "_forge_version: \"5.1.0\"\nplan:\n  price: 16\n  rev: {formula: \"=price * 2\", value: 32}\n"
        );
    }

    type Case = fn(&tempfile::TempDir, &mut Session);

    /// The cases, rule 1 first.
    const CASES: [Case; 9] = [
        start_from_the_starter,
        two_forms_of_a_formula,
        change_with_edit,
        validate_finds_stale_results,
        calculate_writes_the_results,
        uncertain_models_are_simulated,
        check_a_result_another_way,
        files_stay_inside_the_folder,
        hand_back_the_file,
    ];

    #[test]
    fn the_templates_rules_are_numbered_from_one_and_each_has_a_row_and_a_case() {
        let doc = crate::read("docs/guide/ai.md");
        let rules = rule_numbers(&doc);
        let wanted: Vec<usize> = (1..=CASES.len()).collect();
        assert_eq!(rules, wanted, "the template's rules and the cases here");
        let rows = table_rows(&doc);
        assert_eq!(
            rows.iter().map(|(n, _)| *n).collect::<Vec<_>>(),
            wanted,
            "the table under the template has a row for each rule"
        );
    }

    #[test]
    fn each_rule_holds_against_the_server_and_its_row_names_the_tools_called() {
        let doc = crate::read("docs/guide/ai.md");
        let rows = table_rows(&doc);
        for (case, (number, row)) in CASES.iter().zip(&rows) {
            let (dir, mut session) = agent();
            case(&dir, &mut session);
            let mut tools = session.tools_called().to_vec();
            tools.dedup();
            assert!(!tools.is_empty(), "rule {number} calls no tool");
            for tool in tools {
                assert!(
                    row.contains(&format!("`{tool}`")),
                    "rule {number}: its row does not name `{tool}`:\n{row}"
                );
            }
        }
    }

    #[test]
    fn rules_and_rows_are_read_from_the_page() {
        let doc = "```markdown\n# Rules\n\n1. First.\n2. Second. See 1. Also.\nA line.\n```\n\n\
                   | Rule | What |\n| --- | --- |\n| 1 | `forge_a` |\n| 2 | `forge_b` |\n| x | no |\n";
        assert_eq!(rule_numbers(doc), [1, 2]);
        assert_eq!(
            table_rows(doc),
            [
                (1, "| 1 | `forge_a` |".to_string()),
                (2, "| 2 | `forge_b` |".to_string())
            ]
        );
        assert_eq!(rule_numbers("no block"), Vec::<usize>::new());
    }

    #[test]
    fn an_answer_is_read_as_json() {
        let answer = Answer {
            failed: false,
            text: "{\"a\": 1}".to_string(),
        };
        assert_eq!(answer.json()["a"], 1);
    }
}
