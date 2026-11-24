//! The Agent Skill for modelling with Forge follows the Agent Skills
//! specification, and what it tells an agent to do happens (roadmap
//! A12-FORGE-SKILL).
//!
//! `skills/forge-modelling/SKILL.md` is read by agents that may know nothing
//! else of Forge, so a wrong line in it is a wrong step in every agent that
//! loads it. It is checked in three ways:
//!
//! - **Its format**, as the specification has it (agentskills.io/specification,
//!   read on 2026-11-24 from its source, `docs/specification.mdx` of
//!   agentskills/agentskills at 83021e2, with the reference validator
//!   `skills-ref` beside it): YAML frontmatter with only the six fields the
//!   specification defines; a `name` of 1 to 64 characters, `a-z`, `0-9` and
//!   hyphens, neither first nor last, never two in a row, and the name of its
//!   folder; a `description` of 1 to 1,024 characters and a `compatibility`
//!   of at most 500; `metadata` that maps names to strings; and, as it
//!   recommends, a body under 500 lines and about 5,000 tokens.
//! - **Its commands**, as [`crate::guide`] checks a lesson: its `$ forge`
//!   blocks run in a copy of `examples/`, the model it writes is written
//!   there first, and every line it shows is one the command prints.
//! - **Its MCP table**: each tool, with the arguments the table gives it, is
//!   called on `forge mcp`, in the order of the steps, on the skill's model.
//!
//! Its install line is checked by [`crate::install_script`].

use serde_json::Value;

/// The skill, by its path from the top
pub const SKILL: &str = "skills/forge-modelling/SKILL.md";

/// The frontmatter fields the specification defines
pub const FIELDS: [&str; 6] = [
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];

/// The longest body the specification recommends: under 500 lines, and
/// under about 5,000 tokens, counted as four characters a token.
pub const MOST_LINES: usize = 499;
/// See [`MOST_LINES`]
pub const MOST_CHARACTERS: usize = 20_000;

/// A skill's frontmatter, between its first two `---` lines, and its body.
/// None without frontmatter.
#[must_use]
pub fn split(doc: &str) -> Option<(&str, &str)> {
    let rest = doc.strip_prefix("---\n")?;
    let end = rest.find("\n---\n")?;
    Some((&rest[..end], &rest[end + 5..]))
}

/// What `name` breaks of the specification's rules for a skill in the
/// folder `folder`.
#[must_use]
pub fn name_errors(name: &str, folder: &str) -> Vec<String> {
    let mut errors = Vec::new();
    let length = name.chars().count();
    if !(1..=64).contains(&length) {
        errors.push(format!("name is {length} characters, not 1 to 64"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        errors.push(format!("name {name} has a character not in a-z, 0-9 or -"));
    }
    if name.starts_with('-') || name.ends_with('-') {
        errors.push(format!("name {name} starts or ends with a hyphen"));
    }
    if name.contains("--") {
        errors.push(format!("name {name} has two hyphens in a row"));
    }
    if name != folder {
        errors.push(format!("name {name} is not its folder's, {folder}"));
    }
    errors
}

/// Whether a field is text of 1 to `most` characters.
fn is_text(field: &Value, most: usize) -> bool {
    field
        .as_str()
        .is_some_and(|text| (1..=most).contains(&text.chars().count()))
}

/// What the skill `doc`, in the folder `folder`, breaks of the
/// specification.
#[must_use]
pub fn spec_errors(doc: &str, folder: &str) -> Vec<String> {
    let Some((front, body)) = split(doc) else {
        return vec!["no frontmatter between two --- lines".to_string()];
    };
    let Ok(Value::Object(fields)) = yaml_serde::from_str::<Value>(front) else {
        return vec!["the frontmatter is not a YAML mapping".to_string()];
    };
    let mut errors: Vec<String> = fields
        .keys()
        .filter(|key| !FIELDS.contains(&key.as_str()))
        .map(|key| format!("{key} is not a field of the specification"))
        .collect();
    match fields.get("name").and_then(Value::as_str) {
        Some(name) => errors.extend(name_errors(name, folder)),
        None => errors.push("no name".to_string()),
    }
    let limits = [
        ("description", 1024, true),
        ("compatibility", 500, false),
        ("license", usize::MAX, false),
        ("allowed-tools", usize::MAX, false),
    ];
    for (field, most, required) in limits {
        match fields.get(field) {
            None if !required => {},
            Some(value) if is_text(value, most) => {},
            _ => errors.push(format!("{field} is not text of 1 to {most} characters")),
        }
    }
    if let Some(metadata) = fields.get("metadata") {
        if !metadata
            .as_object()
            .is_some_and(|map| map.values().all(Value::is_string))
        {
            errors.push("metadata does not map names to strings".to_string());
        }
    }
    let lines = body.lines().count();
    if lines > MOST_LINES {
        errors.push(format!("the body is {lines} lines, not under 500"));
    }
    let characters = body.chars().count();
    if characters > MOST_CHARACTERS {
        errors.push(format!(
            "the body is {characters} characters, about {} tokens, not under 5,000",
            characters / 4
        ));
    }
    errors
}

/// The model a page has the reader write as `file`: its `yaml` block headed
/// `# file`, without the header. None if it has none.
#[must_use]
pub fn written_model(doc: &str, file: &str) -> Option<String> {
    let header = format!("# {file}");
    crate::guide::fenced_blocks(doc)
        .into_iter()
        .find(|(lang, block)| lang == "yaml" && block.first() == Some(&header))
        .map(|(_, block)| block[1..].join("\n") + "\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_rules::Session;
    use serde_json::json;

    const GOOD: &str = "---\nname: forge-modelling\ndescription: Models.\n---\n\nBody\n";

    #[test]
    fn the_frontmatter_is_between_the_first_two_rules() {
        assert_eq!(
            split(GOOD),
            Some(("name: forge-modelling\ndescription: Models.", "\nBody\n"))
        );
        assert_eq!(split("name: x\n---\n"), None);
        assert_eq!(split("---\nname: x\n"), None);
    }

    #[test]
    fn a_name_follows_the_specifications_rules() {
        assert_eq!(
            name_errors("pdf-processing", "pdf-processing"),
            Vec::<String>::new()
        );
        assert_eq!(name_errors("v2-tools", "v2-tools"), Vec::<String>::new());
        let one = |name: &str| {
            let errors = name_errors(name, name);
            assert_eq!(errors.len(), 1, "{name}: {errors:?}");
            errors[0].clone()
        };
        assert!(one("PDF-Processing").contains("a-z, 0-9 or -"));
        assert!(one("coût").contains("a-z, 0-9 or -"));
        assert!(one("-pdf").contains("starts or ends"));
        assert!(one("pdf-").contains("starts or ends"));
        assert!(one("pdf--processing").contains("two hyphens"));
        assert!(one("").contains("0 characters"));
        assert!(one(&"a".repeat(65)).contains("65 characters"));
        assert_eq!(
            name_errors("pdf", "pdfs"),
            ["name pdf is not its folder's, pdfs"]
        );
    }

    #[test]
    fn a_skill_with_only_the_required_fields_or_all_of_them_follows_it() {
        assert_eq!(spec_errors(GOOD, "forge-modelling"), Vec::<String>::new());
        let all = "---\nname: a\ndescription: d\nlicense: MIT\ncompatibility: c\n\
                   metadata:\n  author: me\n  version: \"1.0\"\nallowed-tools: Bash(forge:*) Read\n---\n";
        assert_eq!(spec_errors(all, "a"), Vec::<String>::new());
    }

    #[test]
    fn each_way_of_breaking_the_specification_is_named() {
        let errors =
            |front: &str, body: &str| spec_errors(&format!("---\n{front}\n---\n{body}"), "a");
        assert_eq!(
            spec_errors("no frontmatter", "a"),
            ["no frontmatter between two --- lines"]
        );
        assert_eq!(
            errors("- a list", ""),
            ["the frontmatter is not a YAML mapping"]
        );
        assert_eq!(
            errors("name: [", ""),
            ["the frontmatter is not a YAML mapping"]
        );
        assert_eq!(
            errors("description: d\nversion: 1", ""),
            ["version is not a field of the specification", "no name"]
        );
        assert_eq!(
            errors("name: a", ""),
            ["description is not text of 1 to 1024 characters"]
        );
        assert_eq!(
            errors(&format!("name: a\ndescription: {}", "d".repeat(1025)), ""),
            ["description is not text of 1 to 1024 characters"]
        );
        assert_eq!(
            errors("name: a\ndescription: 7", ""),
            ["description is not text of 1 to 1024 characters"]
        );
        assert_eq!(
            errors(
                &format!(
                    "name: a\ndescription: d\ncompatibility: {}",
                    "c".repeat(501)
                ),
                ""
            ),
            ["compatibility is not text of 1 to 500 characters"]
        );
        assert_eq!(
            errors("name: a\ndescription: d\nmetadata:\n  version: 1", ""),
            ["metadata does not map names to strings"]
        );
        assert_eq!(
            errors("name: a\ndescription: d\nmetadata: text", ""),
            ["metadata does not map names to strings"]
        );
        assert_eq!(
            errors("name: B\ndescription: d", ""),
            [
                "name B has a character not in a-z, 0-9 or -",
                "name B is not its folder's, a"
            ]
        );
        assert_eq!(
            errors("name: a\ndescription: d", &"line\n".repeat(500)),
            ["the body is 500 lines, not under 500"]
        );
        assert_eq!(
            errors("name: a\ndescription: d", &"line\n".repeat(499)),
            Vec::<String>::new()
        );
        assert_eq!(
            errors("name: a\ndescription: d", &"x".repeat(20_001)),
            ["the body is 20001 characters, about 5000 tokens, not under 5,000"]
        );
    }

    #[test]
    fn the_skill_follows_the_specification() {
        let folder = std::path::Path::new(SKILL)
            .parent()
            .and_then(|dir| dir.file_name())
            .and_then(|name| name.to_str())
            .expect("the skill's folder");
        assert_eq!(folder, "forge-modelling");
        assert_eq!(
            spec_errors(&crate::read(SKILL), folder),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_model_the_page_writes_is_found_by_its_file_name() {
        let doc = "```yaml\n# other.yaml\nx: 1\n```\n```text\n# shop.yaml\n```\n```yaml\n# shop.yaml\na: 1\nb: 2\n```\n";
        assert_eq!(
            written_model(doc, "shop.yaml"),
            Some("a: 1\nb: 2\n".to_string())
        );
        assert_eq!(written_model(doc, "none.yaml"), None);
    }

    #[test]
    fn the_skills_commands_print_what_it_shows() {
        // Its commands, files and links, as a lesson's
        let page = [SKILL.to_string()];
        assert_eq!(
            crate::guide::wrong_pages_of(&page, &crate::read),
            Vec::<String>::new()
        );
        // And how much of it that is: write the model, calculate, validate,
        // edit, see it stale, calculate, set a table's column (a dry run),
        // audit
        let doc = crate::read(SKILL);
        let (_dir, path) = crate::guide::page_folder();
        let mut run = |args: &[String]| crate::forge_in(&path, args);
        let ran = crate::guide::check_commands(SKILL, &doc, &mut run, &path);
        assert!(ran.is_ok(), "{ran:?}");
        let (checked, _) = ran.unwrap_or_default();
        assert_eq!(checked.written, 1);
        assert_eq!(checked.commands, 7);
        assert_eq!(checked.files, 1);
        assert!(checked.lines >= 20, "{checked:?}");
    }

    /// A tool's answer as JSON, after checking it did not fail.
    fn answer(s: &mut Session, tool: &str, arguments: &Value) -> Value {
        let answer = s.call(tool, arguments);
        assert!(!answer.failed, "{tool}: {}", answer.text);
        answer.json()
    }

    #[test]
    fn the_skills_mcp_table_is_calls_the_server_answers() {
        let doc = crate::read(SKILL);
        for cell in [
            "`forge_examples` with `name: \"starter\"`",
            "`forge_calculate` with `file_path`, `dry_run: true`",
            "`forge_calculate` with `file_path` |",
            "`forge_validate` with `file_path`: read `passed`",
            "`forge_edit` with `file_path`, `changes: [{\"op\": \"set_value\", \"path\": \"a.b\", \"value\": 1}]`",
            "`forge_audit` with `file_path`, `variable: \"a.b\"`",
            "`forge_simulate` with `file_path`",
        ] {
            assert!(doc.contains(cell), "SKILL.md's MCP table has {cell}");
        }
        let (dir, path) = crate::guide::page_folder();
        let model = written_model(&doc, "shop.yaml").expect("the skill writes shop.yaml");
        let shop = dir.path().join("shop.yaml");
        std::fs::write(&shop, &model).unwrap();
        let file = json!({"file_path": "shop.yaml"});
        let mut s = Session::start(&path);

        let starter = answer(&mut s, "forge_examples", &json!({"name": "starter"}));
        assert!(starter["content"]
            .as_str()
            .unwrap()
            .contains("_forge_version: \"5.1.0\""));
        answer(
            &mut s,
            "forge_calculate",
            &json!({"file_path": "shop.yaml", "dry_run": true}),
        );
        assert_eq!(std::fs::read_to_string(&shop).unwrap(), model);
        // Not calculated yet: the verdict fails, naming the three formulas
        let before = answer(&mut s, "forge_validate", &file);
        assert_eq!(before["passed"], false, "{before}");
        assert_eq!(
            before["uncalculated"],
            json!(["year.revenue", "year.costs", "year.profit"])
        );
        answer(&mut s, "forge_calculate", &file);
        assert_eq!(answer(&mut s, "forge_validate", &file)["passed"], true);
        let change = json!({"op": "set_value", "path": "inputs.monthly_costs", "value": 3500});
        answer(
            &mut s,
            "forge_edit",
            &json!({"file_path": "shop.yaml", "changes": [change]}),
        );
        assert_eq!(answer(&mut s, "forge_validate", &file)["passed"], false);
        answer(&mut s, "forge_calculate", &file);
        let audit = s.call(
            "forge_audit",
            &json!({"file_path": "shop.yaml", "variable": "year.profit"}),
        );
        assert!(
            !audit.failed && audit.text.contains("63600"),
            "{}",
            audit.text
        );
        answer(
            &mut s,
            "forge_simulate",
            &json!({"file_path": "examples/bagel/bagel_simulate.yaml"}),
        );
        assert_eq!(
            s.tools_called(),
            [
                "forge_examples",
                "forge_calculate",
                "forge_validate",
                "forge_calculate",
                "forge_validate",
                "forge_edit",
                "forge_validate",
                "forge_calculate",
                "forge_audit",
                "forge_simulate",
            ]
        );
    }
}
